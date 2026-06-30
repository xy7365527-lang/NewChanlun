"""D′ 开口① writer：events.jsonl 的唯一合法写者（严格 schema 校验）。

写路径退化（630 开口①）的根因：actor 一直手搓叙事事件 append，关键结构化事件退化
——GOAL_SET 丢 acceptance/base_head、DECOMPOSE 丢 sub_goals[]、SUPERSEDE 丢
old_goal_id/new_goal_id（全退化成 {event,sub_goal_id,artifact,ts} 叙事三元组），且
无 schema 校验 writer。reducer（goal_reducer.py）只能算出 goal id 不崩，但退化数据无
结构化 acceptance/sub_goals → 永远 ready_workstations=[]、acceptance=[]。

本 writer 修写侧：append 前按 SCHEMA.md 强制校验，退化/非法事件 raise ValueError
（no-patch：不容退化 schema 通过；输入验证在系统边界 fail fast）。EVIDENCE 本就是
叙事 schema（sub_goal_id + artifact），合法——叙事归 EVIDENCE，结构归
GOAL_SET/DECOMPOSE/SUPERSEDE/CHECK_PASS/BLOCKED/CLOSED。

校验是纯函数 validate_event；IO（盖时间戳 + append）隔离在 append_event。
SCHEMA 外的事件类型（CEREMONY/ESCALATE/ROUTING/MILESTONE/DECISION/ADJUDICATION 等手搓
治理/叙事类型）被拒绝——它们不在 SCHEMA.md 的 8 种之内，归 EVIDENCE(artifact 自由文本)+
escalation 文件（人读），writer 不让退化事件类型继续扩散。

650 提升协议（编排者 2026-06-30 裁决=B+A2 审计部分）：
- GOAL_RESUME 纳入 schema（纯审计事件 goal_id+note+ts，禁带 base_head——base_head 不可变）。
- CHECK_PASS 必带来源（method=auto→command+verifier / manual→judge+rationale+evidence_ids），
  禁裸写（EVIDENCE=材料，CHECK_PASS=裁决）。
- EVIDENCE 可带 evidence_id（稳定身份），CHECK_PASS.evidence_ids 指向之（机器溯源）。
- GOAL_SET/CHECK_PASS 可带 acceptance_id（acceptance 稳定身份，优于 check 文本匹配）。

依据：.chanlun/goals/SCHEMA.md（必填字段表 + 来源说明）+ spec §2/§6/§9（goal 无验收标准
则拒绝 GOAL_SET）+ goal_reducer.py（reader 侧，本 writer 是其上游）+ codex 议题二 verdict=B。
"""
import datetime
import hashlib
import json
import os
import subprocess

# 每种事件类型的必填字段（不含自动盖的 ts）。顺序即写入顺序（可读性，非语义）。
# EVIDENCE 是叙事 schema（artifact 自由文本）；其余是结构 schema。
_REQUIRED_FIELDS = {
    "GOAL_SET": ("goal_id", "description", "acceptance", "base_head"),
    "GOAL_RESUME": ("goal_id", "note"),  # 纯审计事件（650）：禁带 base_head
    "DECOMPOSE": ("goal_id", "sub_goals"),
    "EVIDENCE": ("sub_goal_id", "artifact"),
    "CHECK_PASS": ("sub_goal_id", "check"),  # + 来源（method=auto/manual，见 _validate_check_pass）
    "BLOCKED": ("sub_goal_id", "blocker"),
    "SUPERSEDE": ("old_goal_id", "new_goal_id"),
    "CLOSED": ("goal_id",),
    # GOAL_AMEND（codex 严格解法 2026-06-30）：寻址层修正——给历史 GOAL_SET 的无 id
    # acceptance 补绑稳定 id，goal_id/base_head 不变（非 SUPERSEDE，不制造假 goal 更替）。
    "GOAL_AMEND": ("goal_id", "amendment_kind", "reason", "acceptance_vector_hash", "bindings"),
}
# 每种事件类型允许出现的全部字段（必填 + 可选）。多余字段拒绝（防退化模式渗入：
# 例如 GOAL_SET 误带 sub_goal_id 是退化写法的指纹）。ts 全局允许（自动或调用方提供）。
# 可选字段（650 提升协议，向后兼容，历史事件可无）：
#  - GOAL_SET：acceptance 稳定身份在 acceptance[].id（嵌套，非顶层；codex MAJOR-1 修正——
#    顶层 acceptance_id 是 reducer 读不到的废字段）。校验在 _validate_acceptance。
#  - GOAL_RESUME：禁带 base_head（base_head 不可变，RESUME 不重锚——契约硬约束）
#  - EVIDENCE：evidence_id（稳定身份，供 CHECK_PASS.evidence_ids 机器溯源）
#  - CHECK_PASS：acceptance_id（匹配 GOAL_SET.acceptance[].id）+ method +
#    auto(command,verifier) / manual(judge,rationale,evidence_ids)
_ALLOWED_FIELDS = {k: set(v) for k, v in _REQUIRED_FIELDS.items()}
_ALLOWED_FIELDS["EVIDENCE"] |= {"evidence_id"}
_ALLOWED_FIELDS["CHECK_PASS"] |= {
    "acceptance_id", "method", "command", "verifier", "judge", "rationale", "evidence_ids",
}
# GOAL_AMEND 可选审计字段（codex：definition_event_id 当前 events 的 GOAL_SET 无 event_id
# 字段，故可选；recorded_head 记「补 id 发生在现在」的 HEAD，与不可变 base_head 分离）。
_ALLOWED_FIELDS["GOAL_AMEND"] |= {"definition_event_id", "definition_base_head", "recorded_head"}


_ACCEPTANCE_ALLOWED = {"check", "falsifiable", "id"}


def acceptance_hash(acc: dict) -> str:
    """acceptance 项去 id 后的 canonical sha256（codex slot_uid 的 acceptance_hash 分量）。

    去 id：hash 只覆盖 check+falsifiable（语义内容），不含 id——否则补 id 会改 hash，
    GOAL_AMEND 的 ordinal+hash 命中失败。canonical：sort_keys 消除字段序差异。
    """
    canonical = {k: acc[k] for k in ("check", "falsifiable") if k in acc}
    blob = json.dumps(canonical, ensure_ascii=False, sort_keys=True)
    return "sha256:" + hashlib.sha256(blob.encode("utf-8")).hexdigest()


def acceptance_vector_hash(acceptance: list[dict]) -> str:
    """整个 acceptance 数组（每项去 id）的 canonical sha256（GOAL_AMEND 防篡改锚）。

    锁定「amend 针对的 GOAL_SET acceptance 向量」——若 GOAL_SET 被改写，hash 不符，
    amend 拒绝（防对错误 goal 版本补 id）。
    """
    blob = json.dumps([acceptance_hash(a) for a in acceptance], ensure_ascii=False)
    return "sha256:" + hashlib.sha256(blob.encode("utf-8")).hexdigest()


def _validate_acceptance(acceptance: object) -> None:
    """GOAL_SET 的 acceptance：非空 list，每项含 check（非空字符串）+ falsifiable=True。

    可选 id（稳定身份，CHECK_PASS.acceptance_id 用以稳定匹配；codex MAJOR-1：稳定身份在
    acceptance[].id 而非顶层）。多余字段拒绝（防退化指纹渗入嵌套）。

    依据 SCHEMA.md「acceptance 每项必须 falsifiable=true」+ spec §9「goal 无验收标准则
    拒绝 GOAL_SET（验收必须可证伪，否则永动空转）」。
    """
    if not isinstance(acceptance, list) or not acceptance:
        raise ValueError("GOAL_SET.acceptance 必须是非空 list")
    seen_ids: set[str] = set()
    for i, acc in enumerate(acceptance):
        if not isinstance(acc, dict):
            raise ValueError(f"acceptance[{i}] 必须是 dict")
        extra = set(acc) - _ACCEPTANCE_ALLOWED
        if extra:
            raise ValueError(f"acceptance[{i}] 含多余字段 {sorted(extra)}（allowed={sorted(_ACCEPTANCE_ALLOWED)}）")
        check = acc.get("check")
        if not isinstance(check, str) or not check.strip():
            raise ValueError(f"acceptance[{i}] 缺少非空 check 字段")
        if acc.get("falsifiable") is not True:
            raise ValueError(
                f"acceptance[{i}] 必须 falsifiable=true（不可证伪 → goal 永动空转）: {check}"
            )
        if "id" in acc:
            if not isinstance(acc["id"], str) or not acc["id"].strip():
                raise ValueError(f"acceptance[{i}].id 必须是非空字符串")
            # 同一 GOAL_SET 内唯一（codex v2 MAJOR：重复 id 会让一个 (gid,acceptance_id) 同时
            # 闭合多个 acceptance）。
            if acc["id"] in seen_ids:
                raise ValueError(f"acceptance[].id 重复 {acc['id']!r}（同一 GOAL_SET 内须唯一）")
            seen_ids.add(acc["id"])


def _validate_sub_goals(sub_goals: object) -> None:
    """DECOMPOSE 的 sub_goals：非空 list，每项含 id + desc + blocked_by(list)。

    依据 SCHEMA.md「sub_goals[]（{id, desc, blocked_by[]}）」。blocked_by 必须是 list
    （reducer 行67 对 blocked_by 逐项查 passed，非 list 会静默错算）。
    """
    if not isinstance(sub_goals, list) or not sub_goals:
        raise ValueError("DECOMPOSE.sub_goals 必须是非空 list")
    for i, sg in enumerate(sub_goals):
        if not isinstance(sg, dict):
            raise ValueError(f"sub_goals[{i}] 必须是 dict")
        if not sg.get("id"):
            raise ValueError(f"sub_goals[{i}] 缺少 id 字段")
        if not sg.get("desc"):
            raise ValueError(f"sub_goals[{i}] 缺少 desc 字段")
        if not isinstance(sg.get("blocked_by"), list):
            raise ValueError(f"sub_goals[{i}].blocked_by 必须是 list")


def _nonempty_str(v: object) -> bool:
    return isinstance(v, str) and bool(v.strip())


# method 适用/禁用字段（codex MINOR-2：method=auto 禁带 manual 专属字段，反之亦然——
# 否则字段串味，来源语义混淆）。
_CHECK_PASS_AUTO_ONLY = {"command", "verifier"}
_CHECK_PASS_MANUAL_ONLY = {"judge", "rationale", "evidence_ids"}


def _validate_check_pass(fields: dict) -> None:
    """CHECK_PASS 必带来源（650：EVIDENCE=材料，CHECK_PASS=裁决，禁裸写）。

    method=auto → command(可重跑,非空 str) + verifier(验证器标识,非空 str) 必填，禁 manual 专属字段。
    method=manual → judge + rationale(非空 str) + 非空 evidence_ids(非空 str 的 list，指向
    EVIDENCE.evidence_id) 必填，禁 auto 专属字段。
    引用完整性（evidence_ids 真指向已存在 EVIDENCE）在 append_event 层校验（需历史 events）。
    依据 SCHEMA.md「CHECK_PASS 来源」+ codex 议题二验收闭合（裸 CHECK_PASS=声明膨胀）。
    """
    method = fields.get("method")
    if method == "auto":
        misused = _CHECK_PASS_MANUAL_ONLY & set(fields)
        if misused:
            raise ValueError(f"CHECK_PASS method=auto 不应带 manual 专属字段 {sorted(misused)}")
        if not _nonempty_str(fields.get("command")):
            raise ValueError("CHECK_PASS method=auto 缺少非空 command（可重跑命令）")
        if not _nonempty_str(fields.get("verifier")):
            raise ValueError("CHECK_PASS method=auto 缺少非空 verifier（验证器标识）")
    elif method == "manual":
        misused = _CHECK_PASS_AUTO_ONLY & set(fields)
        if misused:
            raise ValueError(f"CHECK_PASS method=manual 不应带 auto 专属字段 {sorted(misused)}")
        if not _nonempty_str(fields.get("judge")):
            raise ValueError("CHECK_PASS method=manual 缺少非空 judge（裁决者）")
        if not _nonempty_str(fields.get("rationale")):
            raise ValueError("CHECK_PASS method=manual 缺少非空 rationale（裁决理由）")
        eids = fields.get("evidence_ids")
        if not isinstance(eids, list) or not eids:
            raise ValueError("CHECK_PASS method=manual 缺少非空 evidence_ids（指向 EVIDENCE.evidence_id）")
        if not all(_nonempty_str(x) for x in eids):
            raise ValueError("CHECK_PASS.evidence_ids 每项必须是非空字符串")
    else:
        raise ValueError(
            f"CHECK_PASS 必带来源 method=auto|manual（禁裸写），收到 method={method!r}"
        )
    if "acceptance_id" in fields and not _nonempty_str(fields["acceptance_id"]):
        raise ValueError("CHECK_PASS.acceptance_id 必须是非空字符串")


def _validate_amend_bindings(fields: dict) -> None:
    """GOAL_AMEND 无状态结构校验（codex 严格解法）。跨事件校验（hash 匹配历史 GOAL_SET、
    ordinal 命中 slot、acceptance_id goal 内唯一、幂等）需历史 → 在 append_event 层。

    amendment_kind 仅 ACCEPTANCE_ID_BINDING；bindings 非空 list，每项 {ordinal:int>=0,
    acceptance_hash:非空 str, acceptance_id:非空 str}。ordinal/acceptance_id amend 内唯一。
    """
    if fields["amendment_kind"] != "ACCEPTANCE_ID_BINDING":
        raise ValueError(
            f"GOAL_AMEND.amendment_kind 仅支持 ACCEPTANCE_ID_BINDING，收到 {fields['amendment_kind']!r}"
        )
    if not _nonempty_str(fields["acceptance_vector_hash"]):
        raise ValueError("GOAL_AMEND.acceptance_vector_hash 必须是非空字符串")
    bindings = fields["bindings"]
    if not isinstance(bindings, list) or not bindings:
        raise ValueError("GOAL_AMEND.bindings 必须是非空 list")
    seen_ord: set[int] = set()
    seen_id: set[str] = set()
    for i, b in enumerate(bindings):
        if not isinstance(b, dict):
            raise ValueError(f"bindings[{i}] 必须是 dict")
        if not isinstance(b.get("ordinal"), int) or isinstance(b.get("ordinal"), bool) or b["ordinal"] < 0:
            raise ValueError(f"bindings[{i}].ordinal 必须是非负 int")
        if not _nonempty_str(b.get("acceptance_hash")):
            raise ValueError(f"bindings[{i}].acceptance_hash 必须是非空字符串")
        if not _nonempty_str(b.get("acceptance_id")):
            raise ValueError(f"bindings[{i}].acceptance_id 必须是非空字符串")
        extra = set(b) - {"ordinal", "acceptance_hash", "acceptance_id"}
        if extra:
            raise ValueError(f"bindings[{i}] 含多余字段 {sorted(extra)}")
        if b["ordinal"] in seen_ord:
            raise ValueError(f"bindings ordinal 重复 {b['ordinal']}（amend 内须唯一）")
        if b["acceptance_id"] in seen_id:
            raise ValueError(f"bindings acceptance_id 重复 {b['acceptance_id']!r}（amend 内须唯一）")
        seen_ord.add(b["ordinal"])
        seen_id.add(b["acceptance_id"])


def validate_event(event_type: str, fields: dict) -> dict:
    """纯函数：校验 event_type + fields，合法则返回规范化事件 dict，非法 raise ValueError。

    返回的 dict 以 event 字段开头，含校验后的全部字段（含调用方传入的 ts，若有）。
    无 IO、无副作用——可独立单测，也供 append_event 复用。
    """
    if event_type not in _REQUIRED_FIELDS:
        raise ValueError(
            f"未知 event 类型 {event_type!r}（仅接受 SCHEMA.md 定义的 9 种："
            f"{', '.join(sorted(_REQUIRED_FIELDS))}）"
        )

    # GOAL_RESUME 带 base_head 的契约语义错误优先于泛化的「多余字段」（codex MINOR-1：
    # 否则定制错误信息被多余字段检测抢先，base_head 不可变的契约语义不显式）。
    if event_type == "GOAL_RESUME" and "base_head" in fields:
        raise ValueError(
            "GOAL_RESUME 禁带 base_head（base_head 不可变，RESUME 是纯审计事件不重锚——"
            "移基线须显式 GOAL_REBASE）"
        )

    # 多余字段检测（ts 全局允许）。退化写法的指纹（如 GOAL_SET 带 sub_goal_id）在此拦截。
    allowed = _ALLOWED_FIELDS[event_type] | {"ts"}
    extra = set(fields) - allowed
    if extra:
        raise ValueError(
            f"{event_type} 含未知/多余字段 {sorted(extra)}（unexpected，allowed={sorted(allowed)}）"
        )

    # ts 类型校验：必须是非空字符串（ISO 时间戳）。否则非字符串值会绕过下面的必填校验
    # （ts 不在 _REQUIRED_FIELDS）与多余字段检测（ts 合法），污染 events.jsonl 让下游
    # 按字符串消费 ts 时类型错乱。
    if "ts" in fields and (not isinstance(fields["ts"], str) or not fields["ts"].strip()):
        raise ValueError(f"ts 必须是非空字符串（ISO 时间戳），收到 {type(fields['ts']).__name__}")

    # 必填字段存在性
    for field in _REQUIRED_FIELDS[event_type]:
        if field not in fields or fields[field] in (None, ""):
            raise ValueError(f"{event_type} 缺少必填字段 {field}")

    # 类型化子校验
    if event_type == "GOAL_SET":
        _validate_acceptance(fields["acceptance"])
    elif event_type == "DECOMPOSE":
        _validate_sub_goals(fields["sub_goals"])
    elif event_type == "CHECK_PASS":
        _validate_check_pass(fields)
    elif event_type == "EVIDENCE":
        if "evidence_id" in fields and not _nonempty_str(fields["evidence_id"]):
            raise ValueError("EVIDENCE.evidence_id 必须是非空字符串")
    elif event_type == "GOAL_AMEND":
        _validate_amend_bindings(fields)

    # 规范化：event 字段置首，必填字段按 SCHEMA 顺序，可选字段（已通过多余字段检测）按序
    # 附后，ts 置尾。可选字段（acceptance_id/evidence_id/method/command/... 650 提升协议）
    # 不能丢——否则 CHECK_PASS 来源、稳定 id 被规范化抹掉。
    out = {"event": event_type}
    for field in _REQUIRED_FIELDS[event_type]:
        out[field] = fields[field]
    optional = (_ALLOWED_FIELDS[event_type] - set(_REQUIRED_FIELDS[event_type]))
    for field in sorted(optional):
        if field in fields:
            out[field] = fields[field]
    if "ts" in fields:
        out["ts"] = fields["ts"]
    return out


def append_event(event_type: str, *, ev_path: str | None = None, **fields) -> dict:
    """校验后将事件 append 到 events.jsonl。返回写入的规范化事件 dict。

    ts 处理：调用方显式提供 ts → 保留（补历史事件用）；未提供 → 自动盖 UTC ISO 时间戳。
    校验失败 raise ValueError，文件不被触碰（append-only：不写半截、不污染）。

    ev_path 可注入（测试用临时文件）；None 时用默认真实路径
    .chanlun/goals/events.jsonl（相对本脚本定位，不依赖 cwd）。
    """
    if ev_path is None:
        ev_path = os.path.join(
            os.path.dirname(os.path.abspath(__file__)),
            "..", ".chanlun", "goals", "events.jsonl",
        )
    if "ts" not in fields:
        fields["ts"] = datetime.datetime.now(datetime.timezone.utc).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        )

    event = validate_event(event_type, fields)  # 无状态字段校验在 IO 之前——失败则文件不被触碰

    # 跨事件引用完整性校验（codex CRITICAL：evidence_ids 须真指向已存在 EVIDENCE，否则
    # 「机器溯源」是空壳；evidence_id 须唯一，否则溯源歧义）。需读历史 events——故归
    # append_event 而非无状态 validate_event。读历史用 reader 宽容口径（坏行跳过，不崩）。
    _check_reference_integrity(event, ev_path)

    os.makedirs(os.path.dirname(os.path.abspath(ev_path)), exist_ok=True)
    with open(ev_path, "a", encoding="utf-8") as f:
        f.write(json.dumps(event, ensure_ascii=False) + "\n")
    return event


def _load_events(ev_path: str) -> list[dict]:
    """宽容读历史 events.jsonl（reader 口径：坏行/缺文件跳过，不崩）。"""
    out: list[dict] = []
    if not os.path.exists(ev_path):
        return out
    with open(ev_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                out.append(json.loads(line))
            except json.JSONDecodeError:
                continue  # 历史坏行宽容跳过（reader 有效域）
    return out


def _existing_evidence_ids(ev_path: str) -> set[str]:
    """历史所有 EVIDENCE.evidence_id。"""
    return {e["evidence_id"] for e in _load_events(ev_path)
            if e.get("event") == "EVIDENCE" and e.get("evidence_id")}


def _current_goal_set(events: list[dict], goal_id: str) -> dict | None:
    """历史中该 goal_id 最后一个未被 SUPERSEDE 的 GOAL_SET（reader 口径与 reducer 一致）。"""
    superseded = {e.get("old_goal_id") or e.get("sub_goal_id")
                  for e in events if e.get("event") == "SUPERSEDE"}
    found = None
    for e in events:
        if e.get("event") == "GOAL_SET":
            gid = e.get("goal_id") or e.get("sub_goal_id")
            if gid == goal_id and gid not in superseded:
                found = e
    return found


def _check_amend_against_history(event: dict, events: list[dict]) -> None:
    """GOAL_AMEND 跨事件校验（codex 严格解法 reducer 改动点4）。

    - 目标 GOAL_SET 存在且有结构化 acceptance。
    - acceptance_vector_hash 等于目标 GOAL_SET acceptance（去 id）的 canonical hash（防对
      错误 goal 版本补 id）。
    - 每个 binding 的 (ordinal, acceptance_hash) 命中目标唯一 slot。
    - 幂等：若目标 acceptance[ordinal] 已绑同名 id → 该 binding no-op（允许）；绑不同 id →
      invalid（同 slot 不可改绑）。
    - acceptance_id 在目标 goal 内唯一（跨已有 + 本 amend）。
    """
    goal_id = event["goal_id"]
    gs = _current_goal_set(events, goal_id)
    if gs is None:
        raise ValueError(f"GOAL_AMEND 目标 goal_id {goal_id!r} 无对应 GOAL_SET")
    acceptance = gs.get("acceptance")
    if not isinstance(acceptance, list) or not acceptance:
        raise ValueError(f"GOAL_AMEND 目标 GOAL_SET {goal_id!r} 无结构化 acceptance（退化事件不可 amend）")

    if event["acceptance_vector_hash"] != acceptance_vector_hash(acceptance):
        raise ValueError(
            "GOAL_AMEND.acceptance_vector_hash 与目标 GOAL_SET 不符"
            "（GOAL_SET 已变更或 hash 算错，拒绝对错误版本补 id）"
        )

    # aid → 它已绑定的 ordinal（GOAL_SET 自带 + 历史 amend 绑定）。
    # goal 内唯一 = 同一 aid 不得绑到不同 slot。同 aid 同 ordinal 的重放是幂等 no-op，
    # 不是冲突（否则重放同一 amendment 会误报"已使用"——前一次相同 amend 把 aid 记进了历史）。
    id_to_ordinal: dict[str, int] = {a["id"]: i for i, a in enumerate(acceptance) if a.get("id")}
    for prev in events:
        if prev.get("event") == "GOAL_AMEND" and prev.get("goal_id") == goal_id:
            for b in prev.get("bindings", []):
                id_to_ordinal.setdefault(b["acceptance_id"], b["ordinal"])

    for b in event["bindings"]:
        ordinal, ahash, aid = b["ordinal"], b["acceptance_hash"], b["acceptance_id"]
        if ordinal >= len(acceptance):
            raise ValueError(f"binding ordinal {ordinal} 越界（acceptance 仅 {len(acceptance)} 项）")
        if acceptance_hash(acceptance[ordinal]) != ahash:
            raise ValueError(
                f"binding ordinal {ordinal} 的 acceptance_hash 不匹配目标 slot（ordinal/hash 错位）"
            )
        bound = acceptance[ordinal].get("id")
        if bound is not None:
            if bound == aid:
                continue  # 幂等 no-op：GOAL_SET slot 已绑同名 id
            raise ValueError(
                f"binding ordinal {ordinal} 已绑 id {bound!r}，不可改绑为 {aid!r}（slot 身份不可变）"
            )
        prev_ordinal = id_to_ordinal.get(aid)
        if prev_ordinal is not None and prev_ordinal != ordinal:
            raise ValueError(f"acceptance_id {aid!r} 已在 goal {goal_id!r} 内使用（须唯一）")
        id_to_ordinal[aid] = ordinal


def _check_reference_integrity(event: dict, ev_path: str) -> None:
    """跨事件引用完整性（codex CRITICAL）。失败 raise ValueError（写入前，文件不被触碰）。

    - 新 EVIDENCE.evidence_id 须唯一（不与历史已存在的 evidence_id 撞）。
    - manual CHECK_PASS.evidence_ids 每项须指向历史已存在的 EVIDENCE.evidence_id。
    - GOAL_AMEND 校验（hash 匹配 + ordinal/hash 命中 slot + acceptance_id 唯一 + 幂等）。
    """
    events = _load_events(ev_path)
    existing = {e["evidence_id"] for e in events
                if e.get("event") == "EVIDENCE" and e.get("evidence_id")}
    if event["event"] == "EVIDENCE" and event.get("evidence_id"):
        if event["evidence_id"] in existing:
            raise ValueError(
                f"EVIDENCE.evidence_id {event['evidence_id']!r} 已存在（须唯一，溯源歧义防护）"
            )
    elif event["event"] == "CHECK_PASS" and event.get("method") == "manual":
        missing = [eid for eid in event["evidence_ids"] if eid not in existing]
        if missing:
            raise ValueError(
                f"CHECK_PASS.evidence_ids 指向不存在的 EVIDENCE.evidence_id {missing}"
                f"（机器溯源须真指向既有证据，禁指向空气）"
            )
    elif event["event"] == "GOAL_AMEND":
        _check_amend_against_history(event, events)


def _slug_goal_id(description: str) -> str:
    """从描述生成稳定+唯一的 goal_id（中文描述无法 ASCII slug，用 g-<时间戳>-<内容hash8>）。

    时间戳保唯一（同描述多次 SET 不撞），hash8 锚定描述内容（同描述同 hash 段，可溯源）。
    """
    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    h = hashlib.sha256(description.encode("utf-8")).hexdigest()[:8]
    return f"g-{ts}-{h}"


def _cli_goal_set(description: str, ev_path: str | None = None) -> dict:
    """CLI GOAL_SET：自动生成 goal_id + base_head + falsifiable acceptance 骨架，append 后返回事件。

    acceptance 骨架结构合法（falsifiable=true + 非空 check + 稳定 id），内容是占位——
    Lead 在 command 后审查质量并按需 GOAL_AMEND/SUPERSEDE 补全。骨架不是假数据：
    它声明一个真实可证伪的占位验收项（"待 Lead 补全具体可证伪标准"本身可被"已补全"否证）。
    """
    base_head = subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=True
    ).stdout.strip()
    goal_id = _slug_goal_id(description)
    acceptance = [{
        "id": "a0-skeleton",
        "check": f"待 Lead 补全可证伪验收标准（目标：{description}）",
        "falsifiable": True,
    }]
    return append_event(
        "GOAL_SET", ev_path=ev_path,
        goal_id=goal_id, description=description,
        acceptance=acceptance, base_head=base_head,
    )


def _main() -> None:
    import argparse
    parser = argparse.ArgumentParser(description="events.jsonl CLI writer（/goal command 入口）")
    sub = parser.add_subparsers(dest="cmd", required=True)
    p_set = sub.add_parser("GOAL_SET", help="自动建 GOAL_SET（goal_id+base_head+falsifiable骨架）")
    p_set.add_argument("--description", required=True, help="目标描述")
    p_set.add_argument("--ev-path", default=None, help="events.jsonl 路径（测试注入用）")
    args = parser.parse_args()
    if args.cmd == "GOAL_SET":
        event = _cli_goal_set(args.description, ev_path=args.ev_path)
        print(f"goal_id:   {event['goal_id']}")
        print(f"base_head: {event['base_head']}")
        print("acceptance 骨架（falsifiable，待 Lead 补质量）:")
        for a in event["acceptance"]:
            print(f"  - [{a['id']}] {a['check']}")


if __name__ == "__main__":
    _main()
