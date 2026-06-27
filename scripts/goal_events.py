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
SCHEMA 外的事件类型（CEREMONY/ESCALATE/ROUTING/MILESTONE 等手搓叙事类型）被拒绝
——它们不在 SCHEMA.md 的 7 种之内，writer 不让退化事件类型继续扩散。

依据：.chanlun/goals/SCHEMA.md（必填字段表）+ spec §2/§6/§9（goal 无验收标准则拒绝
GOAL_SET）+ goal_reducer.py（reader 侧，本 writer 是其上游）。
"""
import datetime
import json
import os

# 每种事件类型的必填字段（不含自动盖的 ts）。顺序即写入顺序（可读性，非语义）。
# EVIDENCE 是叙事 schema（artifact 自由文本）；其余是结构 schema。
_REQUIRED_FIELDS = {
    "GOAL_SET": ("goal_id", "description", "acceptance", "base_head"),
    "DECOMPOSE": ("goal_id", "sub_goals"),
    "EVIDENCE": ("sub_goal_id", "artifact"),
    "CHECK_PASS": ("sub_goal_id", "check"),
    "BLOCKED": ("sub_goal_id", "blocker"),
    "SUPERSEDE": ("old_goal_id", "new_goal_id"),
    "CLOSED": ("goal_id",),
}
# 每种事件类型允许出现的全部字段（必填 + 可选）。多余字段拒绝（防退化模式渗入：
# 例如 GOAL_SET 误带 sub_goal_id 是退化写法的指纹）。ts 全局允许（自动或调用方提供）。
_ALLOWED_FIELDS = {k: set(v) for k, v in _REQUIRED_FIELDS.items()}


def _validate_acceptance(acceptance: object) -> None:
    """GOAL_SET 的 acceptance：非空 list，每项含 check（非空字符串）+ falsifiable=True。

    依据 SCHEMA.md「acceptance 每项必须 falsifiable=true」+ spec §9「goal 无验收标准则
    拒绝 GOAL_SET（验收必须可证伪，否则永动空转）」。
    """
    if not isinstance(acceptance, list) or not acceptance:
        raise ValueError("GOAL_SET.acceptance 必须是非空 list")
    for i, acc in enumerate(acceptance):
        if not isinstance(acc, dict):
            raise ValueError(f"acceptance[{i}] 必须是 dict")
        check = acc.get("check")
        if not isinstance(check, str) or not check.strip():
            raise ValueError(f"acceptance[{i}] 缺少非空 check 字段")
        if acc.get("falsifiable") is not True:
            raise ValueError(
                f"acceptance[{i}] 必须 falsifiable=true（不可证伪 → goal 永动空转）: {check}"
            )


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


def validate_event(event_type: str, fields: dict) -> dict:
    """纯函数：校验 event_type + fields，合法则返回规范化事件 dict，非法 raise ValueError。

    返回的 dict 以 event 字段开头，含校验后的全部字段（含调用方传入的 ts，若有）。
    无 IO、无副作用——可独立单测，也供 append_event 复用。
    """
    if event_type not in _REQUIRED_FIELDS:
        raise ValueError(
            f"未知 event 类型 {event_type!r}（仅接受 SCHEMA.md 定义的 7 种："
            f"{', '.join(sorted(_REQUIRED_FIELDS))}）"
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

    # 规范化：event 字段置首，其余按 SCHEMA 字段顺序，ts 置尾（若提供）。
    out = {"event": event_type}
    for field in _REQUIRED_FIELDS[event_type]:
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

    event = validate_event(event_type, fields)  # 校验在 IO 之前——失败则文件不被触碰

    os.makedirs(os.path.dirname(os.path.abspath(ev_path)), exist_ok=True)
    with open(ev_path, "a", encoding="utf-8") as f:
        f.write(json.dumps(event, ensure_ascii=False) + "\n")
    return event
