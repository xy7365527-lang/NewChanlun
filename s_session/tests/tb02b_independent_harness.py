#!/usr/bin/env python3
"""#1392：17 条独立 oracle trace 的正式采集驱动（dry-prepare / prepare / run）。

本驱动只做三件事：
1. 把冻结的 TestOnly 受控合成原始 OHLC 按 `source seq = rawOrdinal` 展开成正式消息、
   固定 clock 与全新运行目录；两个 arm 用同一业务身份与同一份消息/时钟字节，
   只有端口、DB、control dir 与短 Unix socket 路径独立。
2. 纠错 trace 先跑修订前原前缀，再对同一坐标同一 event_id 发 revision=2 修订，
   并记录 prefix→实际 cut 映射（旧输入不改，不能一开始就只跑修订后 OHLC）。
3. 逐 trace 起两个全新 S/Q/浏览器 runtime 采集公共候选与持久原件，保全退出码。

本驱动不构造 expected、不实现第二套结构判据、不宣布业务验收；最终独立对照由父级会话完成。
计分口径：只报采集覆盖与重放一致性，不报任何结构结论。
"""

from __future__ import annotations

import argparse
import errno
import hashlib
import json
import os
import signal
import socket
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, wait, FIRST_COMPLETED
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from s_service_control import canonical, process_identity
from tb01c_compare import first_difference, read_records

TICKET = 1392
PLAN_SCHEMA = "tb02b-independent-plan/1"
DRY_SCHEMA = "tb02b-independent-dry-prepare/1"
MAP_SCHEMA = "tb02b-prefix-cut-map/1"
PROVENANCE_SCHEMA = "tb02b-independent-provenance/1"
ORACLE_SCHEMA = "newchanlun.issue1392.signed-source-oracle.v1"
ORACLE_SHA256 = "0450031d1201f64e1cfcd5f347a2597070e6c61f31f9376434382ac09245ac56"
SOCKET_BYTE_LIMIT = 100
MAX_CONCURRENCY = 2
CONFIG_KEEP = ("schema_revision", "session_generation", "source_epoch", "writer_epoch", "query_epoch",
               "delivery_retain_generations", "startup_timeout_ms", "binary", "python", "catalog", "profile",
               "browser", "query_resource_config", "economic_query_config", "s_resources")


def sha256_bytes(raw):
    return hashlib.sha256(raw).hexdigest()


def sha256_file(path):
    return sha256_bytes(Path(path).read_bytes())


def load_json(path):
    return json.loads(Path(path).read_text())


def write_json(path, value):
    """独占创建：计划与原件目录不允许被历史运行覆盖。"""
    with Path(path).open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def write_json_over(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def operation_id(index):
    return "op-" + str(index).zfill(4)


def slug(trace_id):
    return trace_id.lower().replace("_", "-")


# ---------------------------------------------------------------------------
# 模板：运行环境/二进制/节点/身份/消息形状/时钟全部取自已真实通过的独立 A 计划，
# 不硬编码任何用户目录，也不在脚本里复制历史材料。
# ---------------------------------------------------------------------------

def command_option(command, name):
    if name not in command:
        raise ValueError("模板运行命令缺少 " + name)
    return command[command.index(name) + 1]


def read_template(template_plan_path):
    plan_path = Path(template_plan_path).resolve()
    plan = load_json(plan_path)
    runs = plan.get("runs") or []
    if len(runs) < 2:
        raise ValueError("模板计划必须含 a/b 两个 arm 的运行")
    commands = {run["arm"]: run["command"] for run in runs}
    config_path = Path(command_option(commands["a"], "--config")).resolve()
    config = load_json(config_path)
    if config.get("schema_revision") != "s-launcher/2":
        raise ValueError("模板启动配置必须为 s-launcher/2")
    messages_path = Path(command_option(commands["a"], "--messages")).resolve()
    first = json.loads(messages_path.read_bytes().splitlines()[0].decode("utf-8"))
    clock = load_json(config["clock_plan"])
    events = sorted(clock["events"].items())
    if len(events) != 7 or [key for key, _ in events] != [operation_id(i) for i in range(7)]:
        raise ValueError("模板 clock 事件名/条数与独立 A 不符")
    accept = [int(value["accept_ns"]) for _, value in events]
    step = accept[1] - accept[0]
    if step <= 0 or any(accept[i] - accept[0] != i * step for i in range(len(accept))):
        raise ValueError("模板 clock 不是等步长固定调度")
    attempts = [value["attempts"][0] for _, value in events]
    begin = {int(item["begin_ns"]) - accept[i] for i, item in enumerate(attempts)}
    commit = {int(item["commit_ns"]) - accept[i] for i, item in enumerate(attempts)}
    if len(begin) != 1 or len(commit) != 1 or min(begin) <= 0 or min(commit) <= min(begin):
        raise ValueError("模板 clock 的 begin/commit 偏移不唯一或越界")
    identity = {"session_id": config["session_id"], "source_namespace": config["source_namespace"],
                "source_epoch": config["source_epoch"], "control_producer": config["producer_id"],
                "input_producer": first["producer_id"], "session_generation": first["session_generation"]}
    shape = {"schema_revision": first["payload"]["raw_input"]["schema_revision"],
             "instrument": first["payload"]["raw_input"]["instrument"],
             "profile": first["payload"]["raw_input"]["profile"],
             "received_at": first["payload"]["raw_input"]["events"][0]["received_at"],
             "volume": first["payload"]["raw_input"]["events"][0]["volume"],
             "raw_revision": first["payload"]["raw_input"]["events"][0]["revision"],
             "event_prefix": first["payload"]["raw_input"]["events"][0]["event_id"].rsplit("-", 1)[0]}
    for index in range(7):
        message = json.loads(messages_path.read_bytes().splitlines()[index].decode("utf-8"))
        event = message["payload"]["raw_input"]["events"][0]
        if message["message_id"] != operation_id(index) or event["seq"] != str(index) \
                or event["timestamp"] != str(index * 60) or event["event_id"] != shape["event_prefix"] + "-" + str(index):
            raise ValueError("模板消息坐标规则不是 source seq=rawOrdinal/每根一跳")
    clock_shape = {"schema_revision": clock["schema_revision"], "origin_utc": clock["origin_utc"],
                   "unit": clock["unit"], "step_ns": str(step), "begin_offset_ns": str(min(begin)),
                   "commit_offset_ns": str(min(commit)), "origin_ns": str(accept[0])}
    node = command_option(commands["a"], "--node")
    playwright = command_option(commands["a"], "--playwright-module")
    return {"plan": plan, "plan_path": plan_path, "plan_sha256": sha256_file(plan_path), "config": config,
            "config_path": config_path, "identity": identity, "shape": shape, "clock_shape": clock_shape,
            "node": node, "playwright_module": playwright, "environment": plan.get("environment") or {},
            "input_sha256": runs[0].get("input_sha256")}


# ---------------------------------------------------------------------------
# 输入：oracle.traces[*].rawOHLC → 正式消息 + clock + prefix→cut 映射
# ---------------------------------------------------------------------------

def load_oracle(path, expected_sha256=ORACLE_SHA256):
    raw = Path(path).read_bytes()
    digest = sha256_bytes(raw)
    if expected_sha256 and digest != expected_sha256:
        raise ValueError("oracle 原件 SHA-256 与声明不符：" + digest)
    oracle = json.loads(raw.decode("utf-8"))
    if oracle.get("schema") != ORACLE_SCHEMA:
        raise ValueError("oracle schema 不是本票冻结版本：" + str(oracle.get("schema")))
    if int(oracle.get("issue", 0)) != TICKET:
        raise ValueError("oracle 票号不是 #" + str(TICKET))
    return oracle, digest


def correction_specs(oracle):
    """纠错输入只从 oracle.correctionsAndHistory 读：修订坐标、修订前后极值都由原件给出。"""
    specs = {}
    for item in oracle.get("correctionsAndHistory") or []:
        changed = item.get("changedInput") or {}
        specs[item["trace"]] = {"before_trace": item["beforeTrace"], "ordinal": int(changed["rawOrdinal"]),
                                "before_hl": [str(v) for v in changed["beforeHL"]],
                                "after_hl": [str(v) for v in changed["afterHL"]]}
    return specs


def raw_event(shape, bar, revision):
    return {"event_id": shape["event_prefix"] + "-" + str(bar["rawOrdinal"]), "revision": revision,
            "seq": str(bar["rawOrdinal"]), "received_at": shape["received_at"],
            "raw_text": canonical(bar).decode("utf-8"), "timestamp": str(bar["rawOrdinal"] * 60),
            "volume": shape["volume"],
            **{key: str(bar[key]) for key in ("open", "high", "low", "close")}}


def ingest_message(identity, shape, index, bar, revision):
    operation = operation_id(index)
    payload = {"op": "ingest", "target_session_id": identity["session_id"],
               "target_session_generation": identity["session_generation"], "writer_epoch": "1",
               "clock_event_id": operation,
               "raw_input": {"schema_revision": shape["schema_revision"], "session_id": identity["session_id"],
                             "source_namespace": identity["source_namespace"],
                             "source_epoch": identity["source_epoch"], "instrument": shape["instrument"],
                             "profile": shape["profile"], "events": [raw_event(shape, bar, revision)]}}
    return {"schema_revision": "s-session/2", "session_id": identity["session_id"],
            "session_generation": identity["session_generation"],
            "source_namespace": identity["source_namespace"], "source_epoch": identity["source_epoch"],
            "message_id": operation, "producer_id": identity["input_producer"], "producer_epoch": "1",
            "causal_refs": [], "payload_hash": sha256_bytes(canonical(payload)), "payload": payload}


def build_messages(identity, shape, bars, revision, indices=None):
    """普通 trace：逐根一条 ingest（一次接纳一次 advance）。

    纠错 trace：先跑修订前原前缀（revision=1），再对同一 event_id/seq 追加 revision=2 修订消息；
    旧消息字节不改，修订不是替换。
    """
    positions = list(range(len(bars))) if indices is None else list(indices)
    messages = [ingest_message(identity, shape, index, bars[index], shape["raw_revision"]) for index in positions]
    if revision is not None:
        messages.append(ingest_message(identity, shape, len(messages), revision["bar"], "2"))
    return messages


def build_clock(clock_shape, session_id, count):
    origin = int(clock_shape["origin_ns"])
    step = int(clock_shape["step_ns"])
    begin_offset = int(clock_shape["begin_offset_ns"])
    commit_offset = int(clock_shape["commit_offset_ns"])
    events = {}
    for index in range(count):
        base = origin + index * step
        events[operation_id(index)] = {"accept_ns": str(base),
                                       "attempts": [{"begin_ns": str(base + begin_offset),
                                                     "commit_ns": str(base + commit_offset)}]}
    return {"schema_revision": clock_shape["schema_revision"], "clock_plan_id": session_id,
            "origin_utc": clock_shape["origin_utc"], "unit": clock_shape["unit"], "events": events}


def messages_bytes(messages):
    return b"".join(canonical(message) + b"\n" for message in messages)


def clock_bytes(clock):
    return json.dumps(clock, ensure_ascii=False).encode("utf-8") + b"\n"


def identity_for(trace_id, template_identity):
    name = slug(trace_id)
    return {"session_id": "tb1392-ind-" + name, "source_namespace": "testonly.ohlc.ind-" + name,
            "source_epoch": template_identity["source_epoch"],
            "control_producer": "tb1392-ind-control-" + name,
            "input_producer": "tb1392-ind-input-" + name,
            "session_generation": template_identity["session_generation"]}


def trace_inputs(oracle, trace_id, specs, identity, shape):
    by_id = {trace["id"]: trace for trace in oracle["traces"]}
    trace = by_id[trace_id]
    spec = specs.get(trace_id)
    if spec is None:
        bars, revision = list(trace["rawOHLC"]), None
        return {"trace": trace, "bars": bars, "messages": build_messages(identity, shape, bars, None),
                "revision": None, "base_trace": trace_id}
    before = by_id[spec["before_trace"]]
    bars = list(before["rawOHLC"])
    ordinal = spec["ordinal"]
    original = bars[ordinal]
    corrected = trace["rawOHLC"][ordinal]
    messages = build_messages(identity, shape, bars, {"ordinal": ordinal, "bar": corrected, "original": original})
    return {"trace": trace, "bars": bars, "messages": messages,
            "revision": {"ordinal": ordinal, "message_index": len(bars), "original": original,
                         "corrected": corrected, "base_trace": spec["before_trace"],
                         "before_hl": spec["before_hl"], "after_hl": spec["after_hl"]},
            "base_trace": spec["before_trace"]}


def prefix_to_cut(declared, extra_messages):
    return [{"prefix": int(value), "cut": int(value) + extra_messages} for value in declared]


# ---------------------------------------------------------------------------
# 计划构建 + dry-prepare 定点断言
# ---------------------------------------------------------------------------

def check(report, name, ok, detail=None):
    report["checks"].append({"name": name, "ok": bool(ok), "detail": detail})
    if not ok:
        raise ValueError("dry-prepare 断言失败：" + name + ("" if detail is None else "（" + str(detail) + "）"))


def parse_lifecycle(values):
    result = {}
    for item in values or []:
        trace, separator, asof = item.partition("=")
        if not separator or not asof.isdigit() or not trace:
            raise ValueError("生命周期断言须为 TRACE=ASOF_代际 形式：" + item)
        result[trace] = int(asof)
    return result


def build_plan(args, template, oracle, oracle_sha):
    output = Path(args.output).resolve()
    if output.exists():
        raise ValueError("输出目录已存在，拒绝覆盖历史运行：" + str(output))
    report = {"schema": DRY_SCHEMA, "oracle_path": str(Path(args.oracle).resolve()), "oracle_sha256": oracle_sha,
              "oracle_schema": oracle["schema"], "template_plan": str(template["plan_path"]),
              "template_plan_sha256": template["plan_sha256"], "checks": [], "counts": {}}
    traces = oracle["traces"]
    by_id = {trace["id"]: trace for trace in traces}
    specs = correction_specs(oracle)
    lifecycle = parse_lifecycle(args.lifecycle)
    selection = list(args.trace or [trace["id"] for trace in traces])
    unknown = [trace for trace in selection if trace not in by_id]
    if unknown:
        raise ValueError("选择里有 oracle 之外的 trace：" + ",".join(unknown))
    for trace_id in specs:
        if trace_id not in by_id:
            raise ValueError("纠错票点名了 oracle 之外的 trace：" + trace_id)
    check(report, "oracle.traces=17", len(traces) == 17, len(traces))
    obligations = sum(len(trace.get("expectedPrefixCuts") or []) for trace in traces)
    check(report, "oracle.declared_prefix_obligations=82", obligations == 82, obligations)
    check(report, "corrections=2", len(specs) == 2, sorted(specs))
    check(report, "oracle.runtimeExecuted=false", oracle.get("runtimeExecuted") is False,
          oracle.get("runtimeExecuted"))
    report["counts"].update({"traces": len(traces), "declared_prefix_obligations": obligations,
                             "corrections": len(specs), "selected_traces": len(selection)})

    # 模板字节奇偶校验：用模板身份重建独立 A 的 7 条消息，必须逐字节等于已真实通过的输入。
    parity_identity = dict(template["identity"])
    parity_shape = dict(template["shape"])
    parity_bars = by_id["A"]["rawOHLC"]
    parity_messages = build_messages(parity_identity, parity_shape, parity_bars, None)
    parity_sha = sha256_bytes(messages_bytes(parity_messages))
    check(report, "template.parity_A_messages", parity_sha == template["input_sha256"],
          {"built": parity_sha, "template": template["input_sha256"]})
    parity_clock = build_clock(template["clock_shape"], parity_identity["session_id"], len(parity_bars))
    parity_clock_sha = sha256_bytes(clock_bytes(parity_clock))
    template_clock_path = Path(template["config"]["clock_plan"])
    if template_clock_path.exists():
        check(report, "template.parity_A_clock", parity_clock_sha == sha256_file(template_clock_path),
              {"built": parity_clock_sha, "template": sha256_file(template_clock_path)})
    else:
        report["checks"].append({"name": "template.parity_A_clock", "ok": None,
                                 "detail": "模板 clock.json 不在原位，未做字节奇偶校验（已用解析形状重建）"})
    report["counts"]["template_parity_messages_sha256"] = parity_sha

    plan = {"schema": PLAN_SCHEMA, "ticket": TICKET, "status": "PREPARED_NOT_RUN",
            "oracle_path": str(Path(args.oracle).resolve()), "oracle_sha256": oracle_sha,
            "oracle_schema": oracle["schema"], "input_kind": by_id[selection[0]]["inputKind"],
            "source": "oracle.traces[*].rawOHLC；source seq=rawOrdinal；TestOnly 受控合成输入",
            "template_plan": str(template["plan_path"]), "template_plan_sha256": template["plan_sha256"],
            "template_identity": template["identity"], "template_shape": template["shape"],
            "clock_shape": template["clock_shape"], "environment": template["environment"],
            "node": template["node"], "playwright_module": template["playwright_module"],
            "python": template["plan"]["runs"][0]["command"][0], "port_base": str(args.port_base),
            "socket_dir": str(Path(args.socket_dir).resolve()),
            "initial_as_known": not args.skip_initial_as_known,
            "browser_lifecycle_assertions": {trace: str(asof) for trace, asof in lifecycle.items()},
            "browser_lifecycle_selection_note":
                "默认不声明生命周期断言：oracle 的确认义务是 REQUIRED_NOT_SUPPLIED（未指定唯一确认算法），"
                "在本驱动里断言 CONFIRMED 等于替产品把未知默认成 true。父级若按证书核确认，"
                "用 --lifecycle TRACE=ASOF 对该 trace 显式声明（普通 CC 目录/来源/页面检查始终执行）。",
            "scope": "只采集：17 条独立 trace 的 82 个声明 prefix 义务 + 正式消息重投/旧 cursor gap/"
                     "错版本拒绝/后续重访/清理。expected 对照与业务验收由父级会话完成。",
            "runs": [], "traces": {}}
    if lifecycle and set(lifecycle) - set(selection):
        raise ValueError("生命周期断言点到未选择的 trace：" + ",".join(sorted(set(lifecycle) - set(selection))))

    port_next = args.port_base
    for trace_id in selection:
        built = trace_inputs(oracle, trace_id, specs, identity_for(trace_id, template["identity"]),
                             template["shape"])
        declared = [entry["prefixCount"] for entry in built["trace"].get("expectedPrefixCuts") or []]
        extra = len(built["messages"]) - len(built["bars"])
        mapping = prefix_to_cut(declared, extra)
        trace_report = {"declared_prefixes": declared, "message_count": len(built["messages"]),
                        "prefix_to_cut": mapping, "base_trace": built["base_trace"], "revision": None}

        if built["revision"] is None:
            check(report, trace_id + ".bar_count", len(built["bars"]) == len(built["messages"]), len(built["bars"]))
            check(report, trace_id + ".declared_prefix<=bars",
                  all(0 <= value <= len(built["bars"]) for value in declared),
                  {"declared": declared, "bars": len(built["bars"])})
            check(report, trace_id + ".prefix==cut", all(item["prefix"] == item["cut"] for item in mapping), mapping)
        else:
            revision = built["revision"]
            ordinal = revision["ordinal"]
            base_bars = list(by_id[revision["base_trace"]]["rawOHLC"])
            corrected = revision["corrected"]
            before_event = built["messages"][ordinal]["payload"]["raw_input"]["events"][0]
            revised_event = built["messages"][revision["message_index"]]["payload"]["raw_input"]["events"][0]
            corrected_payload = json.loads(canonical(corrected).decode("utf-8"))
            check(report, trace_id + ".pre_revision_bar_unchanged",
                  before_event["raw_text"] == canonical(base_bars[ordinal]).decode("utf-8")
                  and before_event["open"] == str(base_bars[ordinal]["open"])
                  and before_event["high"] == str(base_bars[ordinal]["high"])
                  and before_event["low"] == str(base_bars[ordinal]["low"])
                  and before_event["close"] == str(base_bars[ordinal]["close"]),
                  {"ordinal": ordinal})
            check(report, trace_id + ".pre_revision_matches_before_hl",
                  [before_event["high"], before_event["low"]] == revision["before_hl"],
                  {"event": [before_event["high"], before_event["low"]], "oracle": revision["before_hl"]})
            check(report, trace_id + ".revision_same_identity",
                  revised_event["event_id"] == before_event["event_id"]
                  and revised_event["seq"] == before_event["seq"] == str(ordinal)
                  and revised_event["revision"] == "2" and before_event["revision"] == "1",
                  {"event_id": revised_event["event_id"], "seq": revised_event["seq"]})
            check(report, trace_id + ".revision_is_corrected_bar",
                  revised_event["raw_text"] == canonical(corrected).decode("utf-8")
                  and [revised_event["high"], revised_event["low"]] == revision["after_hl"]
                  and canonical(corrected_payload) == canonical(corrected),
                  {"high": revised_event["high"], "low": revised_event["low"]})
            check(report, trace_id + ".revision_not_replacement",
                  revised_event["raw_text"] != before_event["raw_text"]
                  and revision["message_index"] == len(base_bars)
                  and built["messages"][revision["message_index"]]["message_id"] != built["messages"][ordinal]["message_id"],
                  {"message_index": revision["message_index"], "bars": len(base_bars)})
            check(report, trace_id + ".other_bars_match_before_trace",
                  all(canonical(item) == canonical(base_bars[index])
                      for index, item in enumerate(built["trace"]["rawOHLC"]) if index != ordinal),
                  {"bars": len(built["trace"]["rawOHLC"])})
            check(report, trace_id + ".declared_prefix==base_bars",
                  declared == [len(base_bars)], {"declared": declared, "base_bars": len(base_bars)})
            check(report, trace_id + ".prefix9/14->cut10/15",
                  mapping == [{"prefix": len(base_bars), "cut": len(built["messages"])}], mapping)
            trace_report["revision"] = {"ordinal": ordinal, "event_id": revised_event["event_id"],
                                        "seq": revised_event["seq"], "revision": "2",
                                        "message_index": revision["message_index"],
                                        "base_trace": revision["base_trace"],
                                        "before_hl": revision["before_hl"], "after_hl": revision["after_hl"]}

        clock = build_clock(template["clock_shape"], identity_for(trace_id, template["identity"])["session_id"],
                            len(built["messages"]))
        check(report, trace_id + ".clock_events", len(clock["events"]) == len(built["messages"]),
              len(clock["events"]))
        check(report, trace_id + ".clock_linear",
              all(int(clock["events"][operation_id(i)]["accept_ns"])
                  == int(clock["events"]["op-0000"]["accept_ns"]) + i * int(template["clock_shape"]["step_ns"])
                  for i in range(len(built["messages"]))), None)
        plan["traces"][trace_id] = {"declared_prefixes": declared, "prefix_to_cut": mapping,
                                    "message_count": len(built["messages"]), "base_trace": built["base_trace"],
                                    "revision": trace_report["revision"], "input_kind": built["trace"]["inputKind"]}
        for arm in ("a", "b"):
            port = port_next
            port_next += 1
            run_dir = output / (slug(trace_id) + "-" + arm)
            socket_file = str(Path(args.socket_dir) / ("tb1392-" + sha256_bytes(str(run_dir).encode("utf-8"))[:16] + ".sock"))
            if len(os.fsencode(socket_file)) > SOCKET_BYTE_LIMIT:
                raise ValueError("Unix socket 路径超过 " + str(SOCKET_BYTE_LIMIT) + " 字节：" + socket_file)
            command = [plan["python"], str(ROOT / "tests/tb02b_runtime.py"),
                       "--config", str(run_dir / "config.json"), "--messages", str(run_dir / "messages.jsonl"),
                       "--output", str(run_dir / "evidence"), "--node", template["node"],
                       "--playwright-module", template["playwright_module"],
                       "--prefix-map", str(run_dir / "prefix-map.json"),
                       "--provenance", str(run_dir / "provenance.json")]
            if not args.skip_initial_as_known:
                command.append("--initial-as-known")
            if trace_id in lifecycle:
                command += ["--browser-lifecycle-asof", str(lifecycle[trace_id])]
            plan["runs"].append({"trace": trace_id, "arm": arm, "directory": str(run_dir), "port": str(port),
                                 "socket": socket_file, "db": str(run_dir / "session.sqlite"),
                                 "command": command, "message_count": len(built["messages"]),
                                 "declared_prefixes": declared,
                                 "prefix_to_cut": mapping,
                                 "input_sha256": sha256_bytes(messages_bytes(built["messages"])),
                                 "clock_sha256": sha256_bytes(clock_bytes(clock)),
                                 "initial_as_known": not args.skip_initial_as_known,
                                 "browser_lifecycle_asof": str(lifecycle[trace_id]) if trace_id in lifecycle else None,
                                 "messages": built["messages"], "bars": built["bars"], "clock": clock,
                                 "identity": identity_for(trace_id, template["identity"])})
    # 两个 arm 的输入/时钟字节必须完全一致；config 只允许端口/DB/socket/clock_plan 不同。
    for trace_id in selection:
        pair = [run for run in plan["runs"] if run["trace"] == trace_id]
        check(report, trace_id + ".arms=2", len(pair) == 2, len(pair))
        check(report, trace_id + ".arms_messages_identical",
              pair[0]["input_sha256"] == pair[1]["input_sha256"], pair[0]["input_sha256"])
        check(report, trace_id + ".arms_clock_identical",
              pair[0]["clock_sha256"] == pair[1]["clock_sha256"], pair[0]["clock_sha256"])
        check(report, trace_id + ".arms_identity_identical",
              canonical(pair[0]["identity"]) == canonical(pair[1]["identity"]), None)
    check(report, "runs=2*traces", len(plan["runs"]) == 2 * len(selection), len(plan["runs"]))
    report["counts"]["runs"] = len(plan["runs"])
    report["counts"]["selected_declared_prefixes"] = sum(
        len(plan["traces"][trace]["declared_prefixes"]) for trace in selection)

    # 端口与 socket 先检查；命中占用即拒绝，不覆盖历史运行。
    occupied, unverifiable = [], []
    for run in plan["runs"]:
        state, detail = port_state(int(run["port"]))
        if state is False:
            occupied.append({"port": run["port"], "detail": detail})
        elif state is None:
            unverifiable.append({"port": run["port"], "detail": detail})
        if os.path.lexists(run["socket"]):
            occupied.append({"socket": run["socket"], "detail": "路径已存在"})
    check(report, "ports_not_occupied_and_sockets_new", not occupied, occupied)
    report["checks"].append({"name": "ports_bind_verifiable", "ok": not unverifiable,
                             "detail": {"unverifiable": len(unverifiable),
                                        "sample": unverifiable[:2] if unverifiable else None,
                                        "note": "本环境不允许绑定 loopback 端口时只记不可验证；"
                                                "正式启动的真实退出码仍然权威"}})
    report["counts"]["ports_unverifiable"] = len(unverifiable)
    return plan, report


def port_state(port):
    """返回 (state, detail)：True 空闲 / False 已被占用 / None 本环境不允许绑定（不据此判占用）。

    真占用是 EADDRINUSE；受限沙箱会返回 EPERM 一类错误，这类只能记「不可验证」，
    由实际启动的正式 launcher 报真实退出码——不把环境无能力当成端口占用，也不把占用当空闲。
    """
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        try:
            probe.bind(("127.0.0.1", port))
        except OSError as exc:
            if exc.errno == errno.EADDRINUSE:
                return False, "occupied: " + str(exc)
            return None, "unverifiable: errno=" + str(exc.errno) + " " + str(exc)
    return True, ""


# ---------------------------------------------------------------------------
# prepare / dry-prepare
# ---------------------------------------------------------------------------

def materialize(plan, dry_report):
    output = Path(plan["runs"][0]["directory"]).parent
    output.mkdir(parents=True, exist_ok=False)
    kept_runs = []
    for run in plan["runs"]:
        run_dir = Path(run["directory"])
        run_dir.mkdir()
        config = dict((key, plan["template_config"][key]) for key in CONFIG_KEEP if key in plan["template_config"])
        config.update({"schema_revision": plan["template_config"]["schema_revision"],
                       "session_id": run["identity"]["session_id"],
                       "session_generation": run["identity"]["session_generation"],
                       "source_namespace": run["identity"]["source_namespace"],
                       "source_epoch": run["identity"]["source_epoch"],
                       "producer_id": run["identity"]["control_producer"], "producer_epoch": "1",
                       "writer_epoch": "1", "query_epoch": "1", "port": run["port"],
                       "db": run["db"], "socket": run["socket"],
                       "clock_plan": str(run_dir / "clock.json")})
        write_json(run_dir / "config.json", config)
        (run_dir / "messages.jsonl").write_bytes(messages_bytes(run["messages"]))
        (run_dir / "clock.json").write_bytes(clock_bytes(run["clock"]))
        write_json(run_dir / "prefix-map.json",
                   {"schema": MAP_SCHEMA, "ticket": TICKET, "trace": run["trace"], "arm": run["arm"],
                    "declared_prefixes": run["declared_prefixes"], "prefix_to_cut": run["prefix_to_cut"],
                    "message_count": run["message_count"], "note":
                    "prefixCount 是 oracle 声明的前缀义务；cut 是本次正式输入的持久代际。纠错 trace 的补充修订"
                    "消息使 cut=prefix+1，且旧前缀按 revision=1 原样先跑过。"})
        write_json(run_dir / "provenance.json",
                   {"schema": PROVENANCE_SCHEMA, "ticket": TICKET, "trace": run["trace"], "arm": run["arm"],
                    "input_kind": plan["traces"][run["trace"]]["input_kind"],
                    "source": plan["source"], "oracle_path": plan["oracle_path"],
                    "oracle_sha256": plan["oracle_sha256"], "oracle_schema": plan["oracle_schema"],
                    "template_plan": plan["template_plan"], "template_plan_sha256": plan["template_plan_sha256"],
                    "session_id": run["identity"]["session_id"],
                    "source_namespace": run["identity"]["source_namespace"],
                    "message_count": run["message_count"], "declared_prefixes": run["declared_prefixes"],
                    "prefix_to_cut": run["prefix_to_cut"], "revision": plan["traces"][run["trace"]]["revision"],
                    "clock": {"origin_utc": plan["clock_shape"]["origin_utc"], "unit": plan["clock_shape"]["unit"],
                              "step_ns": plan["clock_shape"]["step_ns"],
                              "begin_offset_ns": plan["clock_shape"]["begin_offset_ns"],
                              "commit_offset_ns": plan["clock_shape"]["commit_offset_ns"]},
                    "chromium_environment": {key: value for key, value in (plan["environment"] or {}).items()
                                             if key.startswith("TB02B_")},
                    "arms_share_identity_and_input": True})
        kept_runs.append({key: value for key, value in run.items()
                          if key not in ("messages", "bars", "clock", "identity")}
                         | {"config_sha256": sha256_file(run_dir / "config.json"),
                            "messages_sha256": sha256_file(run_dir / "messages.jsonl"),
                            "clock_file_sha256": sha256_file(run_dir / "clock.json")})
        kept_runs[-1]["prefix_map_sha256"] = sha256_file(run_dir / "prefix-map.json")
        kept_runs[-1]["provenance_sha256"] = sha256_file(run_dir / "provenance.json")
    plan_out = {key: value for key, value in plan.items() if key not in ("template_shape",)}
    plan_out["template_config"] = plan["template_config"]
    plan_out["runs"] = kept_runs
    plan_out["dry_prepare_counts"] = dry_report["counts"]
    write_json(output / "RUN-PLAN.json", plan_out)
    write_json(output / "DRY-PREPARE.json", dry_report)
    write_json(output / "PREFIX-MAP.json",
               {"schema": MAP_SCHEMA, "ticket": TICKET, "oracle_sha256": plan["oracle_sha256"],
                "counts": {"traces": len(plan["traces"]),
                           "declared_prefix_obligations": sum(len(value["declared_prefixes"])
                                                              for value in plan["traces"].values()),
                           "runs": len(plan["runs"])},
                "traces": plan["traces"]})
    return output, plan_out


def prepare(args):
    template = read_template(args.template_plan)
    oracle, oracle_sha = load_oracle(args.oracle, args.oracle_sha256)
    plan, report = build_plan(args, template, oracle, oracle_sha)
    plan["template_config"] = template["config"]
    report["counts"]["declared_prefix_obligations_all_traces"] = 82
    output, plan_out = materialize(plan, report)
    return {"output": str(output), "counts": report["counts"], "checks": len(report["checks"]),
            "dry_prepare": str(output / "DRY-PREPARE.json"), "plan": str(output / "RUN-PLAN.json")}


def dry_prepare(args):
    template = read_template(args.template_plan)
    oracle, oracle_sha = load_oracle(args.oracle, args.oracle_sha256)
    plan, report = build_plan(args, template, oracle, oracle_sha)
    return {"counts": report["counts"], "checks": len(report["checks"]),
            "failed": [item for item in report["checks"] if item["ok"] is False]}


# ---------------------------------------------------------------------------
# run：并发不超过 2 个实际 runtime，失败保留原件与退出码，不把失败算通过
# ---------------------------------------------------------------------------

def pick_runs(plan, trace, arm):
    runs = plan["runs"]
    if trace:
        known = {run["trace"] for run in runs}
        unknown = [value for value in trace if value not in known]
        if unknown:
            raise ValueError("计划里没有这些 trace：" + ",".join(unknown))
        runs = [run for run in runs if run["trace"] in set(trace)]
    if arm:
        runs = [run for run in runs if run["arm"] in set(arm)]
    if not runs:
        raise ValueError("选择没有匹配任何运行")
    return runs


def stop_services(run, directory):
    receipts = []
    for service in ("q", "s"):
        argv = [run["command"][0], str(ROOT / "s_service_control.py"), "stop-" + service,
                "--config", str(directory / "config.json"), "--state-dir", str(directory / "evidence/control")]
        try:
            result = subprocess.run(argv, capture_output=True, timeout=60)
            (directory / ("timeout-stop-" + service + ".stdout")).write_bytes(result.stdout)
            (directory / ("timeout-stop-" + service + ".stderr")).write_bytes(result.stderr)
            receipts.append({"service": service, "exit": result.returncode})
        except Exception as exc:
            receipts.append({"service": service, "error": type(exc).__name__ + ": " + str(exc)})
    return receipts


RUN_FILES = {"config.json": "config_sha256", "messages.jsonl": "messages_sha256",
             "clock.json": "clock_file_sha256", "prefix-map.json": "prefix_map_sha256",
             "provenance.json": "provenance_sha256"}


def validate_run_files(run):
    directory = Path(run["directory"])
    for name, key in RUN_FILES.items():
        if sha256_file(directory / name) != run[key]:
            raise ValueError("准备后输入漂移：" + str(directory / name))
    for option, name in (("--config", "config.json"), ("--messages", "messages.jsonl"),
                         ("--output", "evidence"), ("--prefix-map", "prefix-map.json"),
                         ("--provenance", "provenance.json")):
        if Path(command_option(run["command"], option)) != directory / name:
            raise ValueError("运行命令脱离具名输入：" + option)


def valid_arm(run, plan_sha):
    try:
        validate_run_files(run)
        directory = Path(run["directory"])
        exit_receipt = load_json(directory / "EXIT.json")
        result = load_json(directory / "evidence/RESULT.json")
        if exit_receipt.get("exit") != 0 or exit_receipt.get("plan_sha256") != plan_sha:
            return False
        if result.get("status") != "CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER" or result["cleanup"]["errors"]:
            return False
        for name in ("prefix-map.json", "provenance.json"):
            if sha256_file(directory / "evidence" / name) != run[RUN_FILES[name]]:
                return False
        return not run["initial_as_known"] or result["initial_as_known"]["captured"] is True
    except (OSError, ValueError, KeyError, TypeError):
        return False


def cancel_runtime(process, run, directory):
    """先让 runtime 执行 finally；硬停止只针对本次登记、身份仍匹配的进程。"""
    forced = False
    errors = []
    try:
        try:
            process.terminate()
            process.wait(timeout=60)
        except ProcessLookupError:
            pass
        except Exception as exc:
            forced = True
            errors.append({"stage": "runtime-terminate", "error": type(exc).__name__ + ": " + str(exc)})
        # 即使 runtime 已退出，也核查独立浏览器组；身份不匹配或无法核实时不杀未知进程。
        try:
            browser_path = directory / "evidence/browser-process.json"
            if browser_path.exists():
                browser = load_json(browser_path)
                identity = browser["identity"]
                if identity is not None and process_identity(browser["pid"]) == identity:
                    os.killpg(browser["pid"], signal.SIGKILL)
                else:
                    errors.append({"stage": "browser", "error": "浏览器身份未确认或进程已退出"})
        except ProcessLookupError:
            pass
        except Exception as exc:
            errors.append({"stage": "browser", "error": type(exc).__name__ + ": " + str(exc)})
    finally:
        # runtime/helper 共享本次新建进程组；浏览器观测失败不能阻断其清理和 S/Q 停止。
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        except Exception as exc:
            errors.append({"stage": "runtime-group", "error": type(exc).__name__ + ": " + str(exc)})
        try:
            process.wait(timeout=5)
        except Exception as exc:
            errors.append({"stage": "runtime-wait", "error": type(exc).__name__ + ": " + str(exc)})
        services = stop_services(run, directory)
    return {"forced": forced, "errors": errors, "services": services}


def execute_plan(args):
    plan_path = Path(args.plan).resolve()
    plan = load_json(plan_path)
    plan_sha = sha256_file(plan_path)
    if plan.get("schema") != PLAN_SCHEMA:
        raise ValueError("计划 schema 不是 " + PLAN_SCHEMA)
    if args.concurrency > MAX_CONCURRENCY:
        raise ValueError("实际 runtime 并发不得超过 " + str(MAX_CONCURRENCY))
    output = plan_path.parent
    runs = pick_runs(plan, args.trace, args.arm)
    port_notes = []
    for run in runs:
        validate_run_files(run)
        if Path(run["directory"]).joinpath("evidence").exists():
            raise ValueError("该运行已有 evidence 目录，拒绝覆盖：" + run["directory"])
        state, detail = port_state(int(run["port"]))
        if state is False:
            raise ValueError("端口已占用：" + run["port"] + " " + detail)
        if state is None:
            port_notes.append({"trace": run["trace"], "arm": run["arm"], "port": run["port"], "detail": detail})
        if os.path.lexists(run["socket"]):
            raise ValueError("socket 路径已存在：" + run["socket"])
    environment = dict(os.environ)
    environment.update(plan.get("environment") or {})
    outcomes, failures, skipped = [], [], []

    def launch(run):
        directory = Path(run["directory"])
        environment_now = dict(environment)
        try:
            with (directory / "driver.stdout").open("xb") as stdout, (directory / "driver.stderr").open("xb") as stderr:
                validate_run_files(run)
                process = subprocess.Popen(run["command"], stdout=stdout, stderr=stderr, env=environment_now,
                                           start_new_session=True)
                try:
                    process.wait(timeout=args.timeout_seconds)
                except subprocess.TimeoutExpired:
                    return {"trace": run["trace"], "arm": run["arm"], "exit": None, "error": "timeout",
                            "plan_sha256": plan_sha, "stop": cancel_runtime(process, run, directory)}
            validate_run_files(run)
            return {"trace": run["trace"], "arm": run["arm"], "exit": process.returncode, "plan_sha256": plan_sha}
        except Exception as exc:
            return {"trace": run["trace"], "arm": run["arm"], "exit": None,
                    "error": type(exc).__name__ + ": " + str(exc)}

    # 并发上限 2 个实际 runtime；默认失败即停止调度后续运行，已启动的运行照旧收口。
    queue = list(runs)
    pending = {}
    with ThreadPoolExecutor(max_workers=args.concurrency) as pool:
        while queue or pending:
            while queue and len(pending) < args.concurrency:
                run = queue.pop(0)
                pending[pool.submit(launch, run)] = run
            if not pending:
                break
            finished, _ = wait(list(pending), return_when=FIRST_COMPLETED)
            for future in finished:
                run = pending.pop(future)
                outcome = future.result()
                write_json_over(Path(run["directory"]) / "EXIT.json", outcome)
                outcomes.append(outcome)
                if outcome["exit"] != 0:
                    failures.append(outcome)
                    if not args.keep_going:
                        skipped.extend({"trace": item["trace"], "arm": item["arm"]} for item in queue)
                        queue = []
                        break

    comparisons = []
    for trace in sorted({run["trace"] for run in runs}):
        pair = [run for run in plan["runs"] if run["trace"] == trace]
        if len(pair) != 2:
            continue
        cores = [Path(run["directory"]) / "evidence/semantic-core.jsonl" for run in pair]
        if not all(core.exists() for core in cores) or not all(valid_arm(run, plan_sha) for run in pair):
            comparisons.append({"trace": trace, "status": "not_compared", "reason": "缺少完成且来源匹配的两 arm 原件"})
            continue
        left_raw, right_raw = cores[0].read_bytes(), cores[1].read_bytes()
        record = {"trace": trace, "status": "equal" if left_raw == right_raw else "different",
                  "semantic_core_sha256": [sha256_bytes(left_raw), sha256_bytes(right_raw)],
                  "records": [sum(1 for _ in read_records(core)) for core in cores],
                  "scope": "两 arm 全新进程重放一致性（采集完整性），不是业务验收"}
        if left_raw != right_raw:
            record["difference"] = first_difference(list(read_records(cores[0])), list(read_records(cores[1])))
        write_json_over(output / (slug(trace) + "-COMPARE.json"), record)
        comparisons.append(record)

    obligations = sum(len(plan["traces"][trace]["prefix_to_cut"]) for trace in {run["trace"] for run in runs})
    selected_traces = {run["trace"] for run in runs}
    compared = {item["trace"] for item in comparisons if item["status"] == "equal"}
    missing_prefixes = [{"trace": trace, "prefix": 0} for trace in selected_traces
                        if 0 in plan["traces"][trace]["declared_prefixes"]
                        and any(not run["initial_as_known"] for run in runs if run["trace"] == trace)]
    complete = not failures and compared == selected_traces and not skipped and not missing_prefixes
    result = {"schema": "tb02b-independent-run-result/1", "ticket": TICKET,
              "status": "COLLECTED_REQUIRES_INDEPENDENT_COMPARISON" if complete
                        else "INCOMPLETE_REQUIRES_PARENT_ATTENTION",
              "runs": outcomes, "failed": failures, "not_run": skipped, "arm_replay": comparisons,
              "missing_required_prefixes": missing_prefixes,
              "port_check_unverifiable": port_notes,
              "counts": {"runs": len(runs), "passed": len([item for item in outcomes if item["exit"] == 0]),
                         "failed": len(failures), "selected_traces": len(selected_traces),
                         "selected_declared_prefixes": obligations},
              "independent_comparison": "未执行：公共事实与 oracle prefix 投影的逐条独立对照由父级会话完成。",
              "note": "本结果只说明采集与重放一致性；任何一条失败都不计为通过。"}
    write_json_over(output / "RESULT.json", result)
    return result


def add_plan_options(parser):
    parser.add_argument("--oracle", type=Path, required=True)
    parser.add_argument("--oracle-sha256", default=ORACLE_SHA256)
    parser.add_argument("--template-plan", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True,
                        help="本次运行根目录（必须不存在；dry-prepare 只按该布局做定点断言，不落盘）")
    parser.add_argument("--trace", action="append", default=[])
    parser.add_argument("--port-base", type=int, default=25200)
    parser.add_argument("--socket-dir", default="/private/tmp")
    parser.add_argument("--skip-initial-as-known", action="store_true",
                        help="仅用于产品不支持 AsKnown0 时的分诊复采；默认必须取得初态")
    parser.add_argument("--lifecycle", action="append", default=[],
                        help="TRACE=ASOF_代际：由计划选择显式浏览器生命周期断言（默认不声明）")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="action", required=True)
    dry = subparsers.add_parser("dry-prepare", help="只核计划：17/82、修订身份、两 arm 输入字节一致")
    add_plan_options(dry)
    for name, help_text in (("prepare", "落成运行目录与 RUN-PLAN.json（含 dry-prepare 断言）"),):
        subparser = subparsers.add_parser(name, help=help_text)
        add_plan_options(subparser)
    runner = subparsers.add_parser("run", help="执行计划里的运行（并发<=2）")
    runner.add_argument("--plan", type=Path, required=True)
    runner.add_argument("--trace", action="append", default=[])
    runner.add_argument("--arm", action="append", default=[], choices=("a", "b"))
    runner.add_argument("--concurrency", type=int, default=MAX_CONCURRENCY)
    runner.add_argument("--keep-going", action="store_true")
    runner.add_argument("--timeout-seconds", type=int, default=1800)
    args = parser.parse_args()
    if args.action == "dry-prepare":
        result = dry_prepare(args)
    elif args.action == "prepare":
        result = prepare(args)
    else:
        result = execute_plan(args)
    print(json.dumps(result, ensure_ascii=False))
    if args.action != "run":
        return 0
    return 0 if result["status"] == "COLLECTED_REQUIRES_INDEPENDENT_COMPARISON" else 1


if __name__ == "__main__":
    raise SystemExit(main())
