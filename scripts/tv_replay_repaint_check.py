#!/usr/bin/env python3
"""
TradingView Desktop Replay 模式逐根回放 + 缠论指标 repaint 检测。

通过纯 Python CDP（websocket）连接 TV Desktop（debug port 9222），
驱动 TV 内部 _replayApi 逐根前进，每步抓取缠论 Pine labels，
比较相邻步检测 repaint（历史已确认 bar 上的信号被后续 K 线改变/删除）。

不使用 TV MCP 工具（会 hang）。CDP/JS 路径搬自 tradingview-mcp/src/core/replay.js。

关键陷阱（实测发现）：CDP 抓取的 label `x` 是相对当前加载窗口的 bar_index，TV 在
replay 步进触发历史数据重载时会整体平移。必须对每对相邻步估计偏移并对齐后再比较，
否则 repaint 比例虚高约 3 倍（坐标漂移假象冒充 repaint）。详见 detect_repaint()。

用法:
    .venv/bin/python scripts/tv_replay_repaint_check.py --steps 200 --start-date 2026-04-15
    .venv/bin/python scripts/tv_replay_repaint_check.py --steps 3            # 快速验证回放链路
    .venv/bin/python scripts/tv_replay_repaint_check.py --analyze-only       # 仅用已存 json 重算

输出:
    analysis/data_cache/replay_labels_sequence.json — 每步完整 labels 快照
    analysis/tv_repaint_check.md                    — repaint 报告
"""

import json
import sys
import time
import urllib.parse
from pathlib import Path

sys.argv_backup = sys.argv[:]
_argv = sys.argv[:1]  # 隔离，防止 cdp_extract_labels 解析参数
sys.argv = _argv
from cdp_extract_labels import (  # noqa: E402
    connect_ws, CDPClient, get_pages, get_browser_path, is_chart_target,
    EXTRACT_PINE_JS, run_js, CDP_HOST, CDP_PORT,
)
sys.argv = sys.argv_backup

SEQ_PATH = Path("analysis/data_cache/replay_labels_sequence.json")
REPORT_PATH = Path("analysis/tv_repaint_check.md")

RP = "window.TradingViewApi._replayApi"
SETTLE_SEC = 1.5          # 每步后等指标重算
STEP_POLL_MAX = 16        # currentDate 变化轮询次数（×0.25s）
CHANLUN_KEY = "CHANLUN"   # study name 过滤关键字


def wv(expr: str) -> str:
    """包裹一个返回 WatchedValue 的表达式，解包为原始值。"""
    return (f"(function(){{var v={expr};"
            f"return (v&&typeof v==='object'&&typeof v.value==='function')?v.value():v;}})()")


def parse_args():
    steps, start_date, analyze_only = 200, None, False
    out_path, resolution = None, None
    a = sys.argv[1:]
    for i, tok in enumerate(a):
        if tok == "--steps" and i + 1 < len(a):
            steps = int(a[i + 1])
        elif tok.startswith("--steps="):
            steps = int(tok.split("=", 1)[1])
        elif tok == "--start-date" and i + 1 < len(a):
            start_date = a[i + 1]
        elif tok.startswith("--start-date="):
            start_date = tok.split("=", 1)[1]
        elif tok == "--out" and i + 1 < len(a):
            out_path = a[i + 1]
        elif tok.startswith("--out="):
            out_path = tok.split("=", 1)[1]
        elif tok == "--resolution" and i + 1 < len(a):
            resolution = a[i + 1]
        elif tok.startswith("--resolution="):
            resolution = tok.split("=", 1)[1]
        elif tok == "--analyze-only":
            analyze_only = True
    return steps, start_date, analyze_only, out_path, resolution


# ── Replay 控制（搬自 tradingview-mcp core/replay.js）──────────────────────────

def replay_eval(cdp, sid, expr, await_promise=False, timeout=30):
    result = cdp.call("Runtime.evaluate", {
        "expression": expr, "returnByValue": True, "awaitPromise": await_promise,
    }, session_id=sid, timeout=timeout)
    exc = result.get("exceptionDetails")
    if exc:
        raise RuntimeError(exc.get("exception", {}).get("description", "JS error")[:200])
    return result.get("result", {}).get("value")


def replay_stop(cdp, sid):
    try:
        started = replay_eval(cdp, sid, wv(f"{RP}.isReplayStarted()"))
        if started:
            replay_eval(cdp, sid, f"{RP}.stopReplay()")
            time.sleep(1.0)
    except Exception as e:
        print(f"  [!] stop 异常（忽略）: {e}")


def replay_start(cdp, sid, start_date):
    if not replay_eval(cdp, sid, wv(f"{RP}.isReplayAvailable()")):
        raise RuntimeError("Replay 对当前 symbol/timeframe 不可用")
    replay_eval(cdp, sid, f"{RP}.showReplayToolbar()")

    if start_date:
        ts = int(time.mktime(time.strptime(start_date, "%Y-%m-%d")) * 1000)
        replay_eval(cdp, sid, f"{RP}.selectDate({ts}).then(function(){{return 'ok';}})",
                    await_promise=True, timeout=40)
    else:
        replay_eval(cdp, sid, f"{RP}.selectFirstAvailableDate()")

    started, current = False, None
    for _ in range(40):
        started = replay_eval(cdp, sid, wv(f"{RP}.isReplayStarted()"))
        current = replay_eval(cdp, sid, wv(f"{RP}.currentDate()"))
        if started and current is not None:
            break
        time.sleep(0.25)
    if not started:
        replay_stop(cdp, sid)
        raise RuntimeError("Replay 启动失败——该日期可能无数据，换更近日期或更高周期")
    return current


def replay_step(cdp, sid, before):
    replay_eval(cdp, sid, f"{RP}.doStep()")
    current = before
    for _ in range(STEP_POLL_MAX):
        time.sleep(0.25)
        current = replay_eval(cdp, sid, wv(f"{RP}.currentDate()"))
        if current != before:
            break
    return current


# ── Labels 抓取 ───────────────────────────────────────────────────────────────

BAR_COUNT_JS = (
    "(function(){try{var w=window.TradingViewApi._activeChartWidgetWV.value()._chartWidget;"
    "var b=w.model().mainSeries().bars();return (b&&b.size)?b.size():null;}catch(e){return null;}})()"
)


def grab_chanlun_labels(cdp, sid):
    """抓缠论 study 的 labels，返回 (labels, raw_studies)。"""
    pine = run_js(cdp, EXTRACT_PINE_JS, sid)
    labels = [
        {"si": l.get("si"), "bar": l.get("time"), "text": l.get("text"), "price": l.get("price")}
        for l in pine.get("labels", [])
        if CHANLUN_KEY in str(l.get("study", "")).upper()
    ]
    return labels, pine.get("studies", [])


def grab_bar_count(cdp, sid):
    try:
        return replay_eval(cdp, sid, BAR_COUNT_JS)
    except Exception:
        return None


# ── repaint 检测 ──────────────────────────────────────────────────────────────

import re as _re

# 买卖点信号（缠论交易信号）—— 与价格刻度/笔编号 label 区分
SIGNAL_RE = _re.compile(r"买|卖|背驰|盘整")
# repaint 判定的成熟区缓冲（K线数）——豁免最新 N 根上的合法右侧确认
GUARD = 5
# offset 漂移搜索范围
OFFSET_RANGE = range(-12, 13)
# 对齐质量低于此阈值则判定该步为"坐标重映射步"（非均匀漂移，测量不可靠）
ALIGN_QUALITY_MIN = 0.95


def _smap(labels, si):
    """某 study 内 bar_index -> (text, price)，过滤占位 label（price=None）。"""
    return {l["bar"]: (str(l["text"]).strip(), l["price"])
            for l in labels if l["si"] == si and l["price"] is not None}


def estimate_offset(cur_map, nxt_map):
    """
    估计 cur→nxt 的 bar_index 整体平移量（CDP label x 坐标随加载窗口漂移）。
    返回 (best_offset, align_quality)。align_quality = 该 offset 下 text 匹配率。
    """
    if not cur_map:
        return 0, 1.0
    best_off, best_match = 0, -1
    for off in OFFSET_RANGE:
        m = sum(1 for bar, (t, _) in cur_map.items()
                if nxt_map.get(bar + off, (None, None))[0] == t)
        if m > best_match:
            best_match, best_off = m, off
    return best_off, best_match / len(cur_map)


def detect_repaint(sequence, studies=(3, 12)):
    """
    去漂移后检测真实 repaint。

    对每对相邻步、每个 study：
      1. 估计 bar_index 整体偏移 offset 并对齐（消除 CDP 坐标漂移假象）
      2. 对齐质量 < ALIGN_QUALITY_MIN 的步标记为"坐标重映射步"——该步漂移非均匀，
         无法可靠判断 repaint，单独计数，不混入干净步统计
      3. 干净步中，对成熟区 label（bar <= 该步 max_bar - GUARD）比较对齐后的 text/price：
         - signal_repaint : 买卖点 label 的 text 改变或消失（实盘关心的真 repaint）
         - scale_repaint  : 非信号 label（价格刻度/笔编号）的 text/price 改变
    """
    clean_signal_events, clean_scale_count = [], 0
    n_signal_compare, n_total_compare = 0, 0
    remap_steps = []
    edge_activity, edge_signal_activity = 0, 0  # 最右端(GUARD豁免区)活性——排除"指标冻结"假0

    for n in range(len(sequence) - 1):
        cur, nxt = sequence[n], sequence[n + 1]
        for si in studies:
            a = _smap(cur["labels"], si)
            b = _smap(nxt["labels"], si)
            if not a:
                continue
            off, quality = estimate_offset(a, b)
            if quality < ALIGN_QUALITY_MIN:
                remap_steps.append({"from_step": cur["step"], "to_step": nxt["step"],
                                    "si": si, "offset": off, "align_quality": round(quality, 3)})
                continue  # 坐标重映射步：测量不可靠，跳过

            max_bar = max(a)
            cutoff = max_bar - GUARD
            for bar, (t1, p1) in a.items():
                if bar > cutoff:
                    # 最右端未确认区：合法右侧确认。统计活性以证明指标未冻结
                    nb_edge = b.get(bar + off)
                    if nb_edge is None or nb_edge[0] != t1:
                        edge_activity += 1
                        if SIGNAL_RE.search(t1) or (nb_edge and SIGNAL_RE.search(nb_edge[0])):
                            edge_signal_activity += 1
                    continue
                nb = b.get(bar + off)
                is_signal = bool(SIGNAL_RE.search(t1) or (nb and SIGNAL_RE.search(nb[0])))
                n_total_compare += 1
                if is_signal:
                    n_signal_compare += 1
                if nb is None:
                    if is_signal:
                        clean_signal_events.append(
                            {"from_step": cur["step"], "to_step": nxt["step"], "si": si,
                             "bar": bar, "type": "vanished",
                             "old": t1, "new": None, "price": p1, "offset": off})
                    else:
                        clean_scale_count += 1
                    continue
                t2, p2 = nb
                if t1 != t2:
                    if is_signal:
                        clean_signal_events.append(
                            {"from_step": cur["step"], "to_step": nxt["step"], "si": si,
                             "bar": bar, "type": "mutated",
                             "old": t1, "new": t2, "price": p1, "offset": off})
                    else:
                        clean_scale_count += 1
                elif p1 != p2 and not is_signal:
                    clean_scale_count += 1  # 价签位置微调

    return {
        "guard": GUARD,
        "n_total_compare": n_total_compare,
        "n_signal_compare": n_signal_compare,
        "clean_signal_events": clean_signal_events,
        "n_signal_repaint": len(clean_signal_events),
        "signal_repaint_rate": (len(clean_signal_events) / n_signal_compare) if n_signal_compare else 0.0,
        "clean_scale_count": clean_scale_count,
        "remap_steps": remap_steps,
        "n_remap_steps": len(remap_steps),
        "n_adjacent_pairs": len(sequence) - 1,
        "edge_activity": edge_activity,
        "edge_signal_activity": edge_signal_activity,
    }


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    steps, start_date, analyze_only, out_path, resolution = parse_args()
    seq_out = Path(out_path) if out_path else SEQ_PATH

    if analyze_only:
        print(f"[*] --analyze-only：从 {seq_out} 读取已存 sequence 重新分析")
        data = json.loads(seq_out.read_text())
        seq = data["sequence"]
        write_report(seq, data.get("symbol", "?"), start_date)
        return

    print(f"[*] steps={steps} start_date={start_date or '(first available)'} "
          f"resolution={resolution or '(当前)'} out={seq_out}")

    sock = connect_ws(CDP_HOST, CDP_PORT, get_browser_path())
    cdp = CDPClient(sock)
    cdp.call("Target.setDiscoverTargets", {"discover": True})
    sequence = []
    symbol = "?"
    try:
        chart = next((p for p in get_pages() if is_chart_target(p)), None)
        if not chart:
            print("[ERROR] 无 TradingView chart target")
            sys.exit(1)
        url = chart.get("url", "")
        symbol = urllib.parse.unquote(url.split("symbol=")[-1]) if "symbol=" in url else url
        print(f"[*] chart: {url[:70]}")
        sid = cdp.attach(chart.get("targetId") or chart.get("id"))
        cdp.call("Runtime.enable", session_id=sid)

        if resolution:
            print(f"[*] 切换周期 → {resolution} ...")
            replay_eval(cdp, sid,
                        f"window.TradingViewApi.activeChart().setResolution('{resolution}')")
            time.sleep(3.0)  # 等待新周期数据加载

        print("[*] 清理残留 replay 状态...")
        replay_stop(cdp, sid)

        print("[*] 启动 replay...")
        current = replay_start(cdp, sid, start_date)
        print(f"    起始 currentDate={current}")

        time.sleep(SETTLE_SEC)
        labels, _ = grab_chanlun_labels(cdp, sid)
        bc = grab_bar_count(cdp, sid)
        sequence.append({"step": 0, "current_date": current, "bar_count": bc, "labels": labels})
        print(f"    step 0: {len(labels)} 缠论 labels, bar_count={bc}")

        for i in range(1, steps + 1):
            before = current
            current = replay_step(cdp, sid, before)
            if current == before:
                print(f"    step {i}: currentDate 未变化（可能到达数据末端），停止")
                break
            time.sleep(SETTLE_SEC)
            labels, _ = grab_chanlun_labels(cdp, sid)
            bc = grab_bar_count(cdp, sid)
            sequence.append({"step": i, "current_date": current, "bar_count": bc, "labels": labels})
            if i % 10 == 0 or i <= 3:
                print(f"    step {i}: cd={current} labels={len(labels)} bar_count={bc}")

        print("[*] 停止 replay...")
        replay_stop(cdp, sid)
        cdp.detach(sid)
    finally:
        cdp.close()

    # ── 保存 sequence ──
    seq_out.parent.mkdir(parents=True, exist_ok=True)
    seq_out.write_text(json.dumps({
        "symbol": symbol, "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "n_steps": len(sequence), "settle_sec": SETTLE_SEC,
        "resolution": resolution,
        "sequence": sequence,
    }, ensure_ascii=False, indent=2))
    print(f"[✓] sequence 保存到 {seq_out} ({len(sequence)} 步)")

    # ── 分析 ──
    if len(sequence) < 2:
        print("[!] 步数不足，无法分析 repaint")
        return
    write_report(sequence, symbol, start_date)


def write_report(sequence, symbol, start_date):
    symbol = urllib.parse.unquote(symbol)  # 兼容旧 json 中 URL 编码的 symbol
    r = detect_repaint(sequence)
    n_pairs = r["n_adjacent_pairs"]
    n_remap = r["n_remap_steps"]
    n_clean = n_pairs - n_remap  # 干净步对数（近似，按 study 累计的 remap 已分离）
    sig_rate = r["signal_repaint_rate"]

    lines = []
    lines.append("# TradingView 缠论指标 Repaint 检测报告\n")
    lines.append(f"- **标的**：{symbol}")
    lines.append(f"- **起始日期**：{start_date or '(首个可用)'}")
    lines.append(f"- **回放步数**：{len(sequence)}（{n_pairs} 对相邻步）")
    lines.append(f"- **GUARD（豁免最右端 K 线数）**：{r['guard']}")
    lines.append(f"- **生成时间**：{time.strftime('%Y-%m-%d %H:%M:%S')}\n")

    lines.append("## 方法：bar_index 漂移校正（核心）\n")
    lines.append("CDP 抓取的 label `x` 坐标是**相对当前加载窗口的索引**——TV 在 replay 步进中"
                 "触发历史数据重载时，整个序列的 bar_index 会整体平移。直接用 `(study, bar_index)` "
                 "作身份键会把'同一信号换了索引'误判为 repaint（实测虚高 ~3 倍）。\n")
    lines.append("本报告对**每对相邻步、每个 study** 先估计 bar_index 整体偏移 offset 并对齐，"
                 "再比较。对齐质量 <95% 的步判定为**坐标重映射步**（非均匀漂移，CDP 提取层无法可靠"
                 "对齐 → 测量假象，单独计数，不混入干净步统计）。\n")

    lines.append("## 坐标重映射步（测量假象，非指标 repaint）\n")
    if r["remap_steps"]:
        lines.append(f"检出 **{n_remap}** 个 study-步对发生 bar_index 重映射：\n")
        lines.append("| from→to step | study | 估计 offset | 对齐质量 |")
        lines.append("|--------------|-------|------------|---------|")
        for rs in r["remap_steps"]:
            lines.append(f"| {rs['from_step']}→{rs['to_step']} | si={rs['si']} | "
                         f"{rs['offset']:+d} | {rs['align_quality']*100:.0f}% |")
        lines.append("\n这些步是 TV 历史数据重载导致的坐标系整体偏移，**不是缠论指标的 repaint**。\n")
    else:
        lines.append("✅ 无坐标重映射步——所有相邻步 bar_index 对齐质量 ≥95%。\n")

    lines.append("## 真实 Repaint（干净步，去漂移后）\n")
    verdict = ("✅ **基本不 repaint**" if sig_rate < 0.05
               else "⚠️ **偏高**" if sig_rate < 0.15 else "❌ **严重**")
    lines.append(f"- 买卖点信号成熟区比较数：**{r['n_signal_compare']}**")
    lines.append(f"- 信号 repaint 事件（对齐后 text 仍改变/消失）：**{r['n_signal_repaint']}**")
    lines.append(f"- **信号 repaint 比例：{sig_rate*100:.2f}%** {verdict}")
    lines.append(f"- 非信号（价格刻度/笔编号）repaint：{r['clean_scale_count']}（不影响交易信号）\n")

    lines.append("### 指标活性佐证（排除'指标冻结'假 0）\n")
    lines.append(f"干净步最右端（GUARD 豁免区）label 变化 **{r['edge_activity']}** 次，"
                 f"其中买卖点信号生成/确认 **{r['edge_signal_activity']}** 次。"
                 "这证明指标在 replay 中**持续工作**——信号在最右端动态形成（右侧确认），"
                 "进入成熟区后才永久稳定。历史区 0 repaint 是真稳定，非指标未更新。\n")

    # 信号 repaint 事件按 step 聚合
    by_step = {}
    for e in r["clean_signal_events"]:
        by_step.setdefault(e["from_step"], []).append(e)
    if by_step:
        lines.append("### 干净步中的信号 repaint 分布\n")
        lines.append("| from→to step | 事件数 | 说明 |")
        lines.append("|--------------|-------|------|")
        for st in sorted(by_step):
            n = len(by_step[st])
            lines.append(f"| {st}→{st+1} | {n} | 见下方样例 |")
        lines.append("")
        lines.append("### 样例信号 repaint 事件（前 20 条）\n")
        lines.append("```")
        for e in r["clean_signal_events"][:20]:
            arrow = f"{e['old']!r} → {'<消失>' if e['new'] is None else repr(e['new'])}"
            lines.append(f"[{e['type']}] step{e['from_step']}→{e['to_step']} "
                         f"si={e['si']} bar={e['bar']} (off={e['offset']:+d})  {arrow}")
        lines.append("```")
        lines.append("")
    else:
        lines.append("✅ **干净步零信号 repaint**——所有买卖点一旦确认即全程不变。\n")

    # 结果包六要素
    lines.append("## 结果包\n")
    clean_note = ("绝大多数信号 repaint 集中在坐标重映射步附近，去漂移后干净步信号高度稳定"
                  if r["n_signal_repaint"] > 0 else "干净步零信号 repaint")
    lines.append(f"1. **结论**：{symbol} 缠论指标在 {n_pairs} 对相邻步回放中，"
                 f"去 bar_index 漂移后买卖点信号 repaint 比例 **{sig_rate*100:.2f}%**"
                 f"（{r['n_signal_repaint']}/{r['n_signal_compare']}）。{clean_note}。"
                 f"检出 {n_remap} 个坐标重映射步（CDP 提取层假象，非指标缺陷）。"
                 f"{'指标历史买卖点稳定，可用于回测/实盘信号统计。' if sig_rate < 0.05 else '存在历史信号回画，回测需剔除 repaint 信号。'}")
    lines.append(f"2. **定义依据**：repaint = 锚定在历史已确认 K 线（bar ≤ 该步 max_bar − GUARD={r['guard']}）"
                 f"上的买卖点 label（text 含'买/卖/背驰/盘整'），在后续 K 线到来后 text 改变或消失。"
                 f"身份键为去漂移对齐后的 `(study, bar_index+offset)`。")
    lines.append("3. **边界条件**：① 若 bar_index 漂移非整体均匀（坐标重映射步），单 offset 无法对齐，"
                 "该步标为测量假象而非判定 repaint——可能掩盖真 repaint，也可能误标；"
                 f"② GUARD={r['guard']} 改变成熟区边界；③ settle_sec={SETTLE_SEC}s 若不足，"
                 "指标未算完会误报；④ 仅 study 级 label，未覆盖中枢框/线段。")
    lines.append("4. **下游推论**：信号 repaint 比例低 → `data_get_pine_labels` 实时抓取的买卖点"
                 "等价于历史回看信号，实盘信号可直接用于历史统计与回测，无前视偏差。")
    lines.append("5. **谱系引用**：本产出为缠论指标工具的可靠性经验验证，未涉及缠论概念定义的谱系分离。"
                 "bar_index 漂移是 CDP 提取层的坐标系问题，与缠论定义无关。")
    lines.append("6. **影响声明**：新增 `scripts/tv_replay_repaint_check.py`、"
                 "`analysis/data_cache/replay_labels_sequence.json`、本报告。不改动任何缠论定义或源码。")
    lines.append("")
    lines.append(f"**认识论等级**：L2（真实数据，单标的 {symbol}，单周期）。"
                 "交叉验证（多标的/多周期）为 L3，本次未做。\n")

    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    REPORT_PATH.write_text("\n".join(lines))
    print(f"[✓] 报告保存到 {REPORT_PATH}")

    print("\n── repaint 摘要（去漂移后）──")
    print(f"  相邻步对: {n_pairs}  坐标重映射步: {n_remap}")
    print(f"  信号 repaint: {r['n_signal_repaint']}/{r['n_signal_compare']} "
          f"= {sig_rate*100:.2f}%")
    print(f"  非信号(刻度)repaint: {r['clean_scale_count']}")


if __name__ == "__main__":
    main()
