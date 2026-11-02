# #1585 非作者独审（claude-fable-5-1，只读）

- 审查对象：PR #1664，分支 `sandcastle/issue-1585`（`430591db60` 实装 + `74fbffeb0c` 评审工蜂修正），base `d07fc20438`。
- 路由：Claude Code CLI 经 CLIProxyLocal（本机代理端口 8318），模型 `claude-fable-5-1`；实装与评审工蜂均为 `claude-opus-5-5`，本审查非作者。
- 工具限制：Read/Grep/Glob 与只读 git；未跑 cargo。审查提示原文：`prompt.md`。日期：2026-10-21。
- 附注（harvest owner）：M2 所说的 X-38 编号来自票 #1585 标题本身（「[impl] X-38 非 classifier 入口……」），与 `.chanlun/definitions/xianduan.md` §15 的 X-38（#1600 tangency）冲突，属票面编号问题。

---

结论：PASS

（两条 MEDIUM 建议合前顺手修，均不改变行为；无 HIGH。）

## 总评

五处路径都处理了，口径分两类且与 #1581 一致：三处「构造」类（`cascade_dual`、`cand_delta_tower_with_series`、`cp_recall_upper_bound_audit`）整层拒读，与 `classify_incremental` 入口（pipeline.rs:554）同一判定字段；两处「离线统计」类（segment_gn 的幅比/缺口分布、capture_oos 的 λ 定参与理论分母）读可读前缀并在报告首行标出「段解析无效」。capture_oos 逐 bar 增量在标记 bar 拒读段账本、持仓打污染标记剔出全部聚合、标记区间上报。对照臂只产 `println!` 内存报告，无任何文件/账本写入。

## HIGH

无。

## MEDIUM

**M1. S2/S3 的负控锁的是闸函数，不是接线**（`rust/src/theta_v0/backtest/capture_oos.rs:514-531`、`rust/src/theta_v0/backtest/segment_gn.rs:133-154`）
`tb1585_s3_refuses_segment_confirmation_under_marker` 验的是 `readable_confirmed_len` / `segments_valid_prefix` / `parse_invalid_note` 这三个函数本身。把主回测里的接线去掉——即 capture_oos.rs:351 的 `match readable_confirmed_len(&layer)` 还原为 `let c = layer.segments_confirmed_len;`，或 capture_oos.rs:203 / segment_gn.rs:68 的 `segments_valid_prefix()` 还原为 `&layer.segments`——两条负控仍绿。原因是 S2/S3 主体是 `#[ignore]` 的真数据测试，闸的落点在其内联循环里。按票面「注入标记 → 输出带标记/拒读」字面能过，按本次审查标准 4「把处理去掉会红」不满足。建议：把逐 bar 的「闸 + 开平仓决策」抽成一个接受 `&ParseLayer` 的函数（或至少把 `fin_segs`/`segs` 的取数抽成函数），让负控直接喂 marked 层调它。wverify/cand_delta/cp_recall 三处没有这个问题——去掉早返回后 `tb1585_cascade_dual_refuses_parse_invalid_layer` 与 `tb1585_diag_entries_refuse_parse_invalid_layer` 会红（marked 克隆层的 segments 照样能构造出中枢/事件/审计行）。

**M2. 注释里的清单号 X-38 与 xianduan.md §15 冲突**（所有新增注释，如 `parser/mod.rs:140`、`capture_oos.rs:39`、`wverify_run.rs:3063`、`cand_delta.rs:152`、`cp_recall_audit.rs:349`、`segment_gn.rs:20`、`tb1585_tests.rs:1`）
`.chanlun/definitions/xianduan.md:524` 里 X-38 已是 #1600 的「相切边界 `tangency` 字段」条目（承接票 #1604/#1626），#1581 才是 X-37。本 PR 没改 xianduan.md，也没登记新的 X-行，却把 #1585 自称 X-38，读者顺着注释查清单会落到无关条目。建议去掉 X-号或在 §15 新增一行后改引。未核实票面是否自称 X-38（本审查不调 gh）。

## LOW

- **L1. 另一处直读段账本的 bin 未纳入**：`rust/src/bin/p89_dual_core_audit.rs:147` `layer.segments.iter().map(segment_to_unit)` 直接构造 L0 中枢做双核对账。遇标记时它会在 `tower.len() != 6`（:130）先报「守恒失败」——不静默，但诊断口径误导（是拒读不是守恒破缺）。其余 grep 命中（`backtest/tick.rs:432` seam 测试、`l3_delta_r_alpha.rs:2577`、`incremental.rs`、`p123_fast_replay.rs:1951/2049`、`p856`/`issue1588` bin）都只是计数/日志或走 classifier；`lib.rs` 的 `.segments()` 是 `RecursiveOrchestrator`，非 `ParseLayer`。票面只点了五处，故不升级。
- **L2. 见证锁的数据身份断言在拒读断言之前**（`wverify_run.rs:3357`、`:3442` 对比 `:3253`）：`layer.segments.len()` 对 GOLDEN 的断言先跑，标记若改变段数会先报「数据换版」而非「段解析无效」。顺序对调即可。
- **L3. 积压确认的透明度**（`capture_oos.rs:370-422`）：标记消失后首个 bar 一次性处理 `hw..c`，这些交易 `parse_invalid=false` 且报告里不可辨。建议加一个「标记后追赶确认」计数进报告行。判定见下文第 3 点——这不是静默使用失败区间结构，所以只是 LOW。
- **L4. 评审工蜂提交缺编号声明行**（`74fbffeb0c`）：`.sandcastle/CODING_STANDARDS.md:13` 要求「编号声明行必备」；实装提交有，修正提交没有，格式也非 `<type>(<scope>): #N——`。
- **L5.** `capture_oos.rs:523-527` 用 `.map(|b| incr.append(*b)).last()` 驱动有副作用的增量，clippy `double_ended_iterator_last` 可能建议 `next_back()`——照改会只喂最后一根 bar。改成 `for` 循环更稳。CI 目前没有 clippy 闸，故 LOW。

## 五个重点的逐项判定

1. **覆盖面**：五处都处理。额外直读点只有 L1 的 `p89_dual_core_audit`（失败响亮但口径误导）与 tick.rs 的 seam 测试（固定 fixture，无决策）。
2. **`segments_valid_prefix` 语义**（`parser/mod.rs:144-151`）：标记 = 失败段的 `seg_start` 笔号（segment.rs:1016/1418 `get_or_insert(start)`）。门失败后段「跳过触发、段继续」，失败段可能在更晚触发处发射入 `segments`，其 `start_index == strokes[from].start_index`；`partition_point(start_index < cut)` 恰好把它和其后全部排除，前面的段全部保留（段起点随笔序严格递增）。流首段在生产口径门恒过（segment.rs:438-440），测试口径 `FIRST_START_AT_SEED` 下起点固定为 seed，不存在「失败时起点 A、发射时起点 B」的错位。笔号越界取 `cut=0` 空前缀，fail-closed。与 #1581 同一字段、不另起判定。负控 `tb1585_s2` 用 `segments[k]` 起点反查笔号，锁住了「不含失败段本身」这个边界（改 `<` 为 `<=` 即红）。
3. **capture_oos 逐 bar**：标记 bar 走 `continue`（:364），跳过 `retreats`/`hw`/开平仓；止损、窗界强平、挂起出场在闸之前（:272-349），只读价格与日期，不读段结构；`last_tradable_close` 在分支内补更新（:361-363）。关于「标记消失后首 bar 一次性处理积压确认」：我核了 `IncrSegments::append`——标记对保留前缀持久（segment.rs:1319/1369 `filter(p < resume_seg_start)`），只有失败段起点落在重扫区（即失败段尚未封入 confirmed 前缀）时标记才可能因尾笔改写而消失；因此在任一无标记 bar 上，`segments_confirmed_len` 之内的段全部早于失败起点，积压的 `hw..c` 是有效前缀上被延迟处理的确认，读的是当下这根无标记 bar 的账本。**不构成静默使用失败区间结构**；代价只是入场滞后，这正是拒读的本意。反之若失败段被封入前缀，标记永久、后续全拒读，持仓直到止损/窗界并打污染。
4. **负控/正控**：wverify、cand_delta、cp_recall 三处负控真锁（见 M1 说明）；S2/S3 两处锁的是闸函数而非接线（M1）。正控五处都有（无标记时 c>0 / 有中枢 / 有事件与审计行 / 整本账可读）。
5. **范围/编号/票号**：改动面 9 个文件全在票面路径 + parser 一个只读 helper + 两处测试可见性放宽（`pub(crate) mod tb1581_tests`、`pub(super) mod tests`，均 `cfg(test)`），未越界。实装提交有编号声明；修正提交缺（L4）。注释票号 #1585/#1581 正确，X-38 错指（M2）。

## 看过什么 / 未核实什么

**看过**：`git diff d07fc20438...HEAD` 全文与两条提交信息；`parser/mod.rs` 的 `ParseLayer` 字段、四个装配点与 `ParseLayerIncr::append`；`parser/segment.rs` 全量主循环门失败分支（:990-1049）、`IncrSegments::append` 的标记持久化/过滤/重扫（:1254-1453）、`first_segment_start` 与 `EndGate`；`classifier/pipeline.rs:539-575` 的 #1581 入口拒读；`capture_oos.rs` 整段因果重放循环；`wverify_run.rs` 的 `cascade_dual`、`c327_run_witness`、`assert_cascade_faithful` 与两把见证锁；`cand_delta.rs` 三个入口的汇聚；全仓 `.segments\b|segments_confirmed_len` 177 处命中逐条分类；`.chanlun/definitions/xianduan.md` §15 X-35～X-38；`.sandcastle/CODING_STANDARDS.md` 提交规范；`tb1581_tests.rs` 的既有负控写法。

**未核实**：未跑 `cargo build/test/clippy/fmt`（只读约束）——新增测试能否编译、六个 seed 在 4000 bar 下生产口径是否确无标记（依赖 `tb1581_probe_marker_in_production` 这把 `#[ignore]` 探针的历史读数）、`tb1581_bars(0x1581_0000, 4_000)` 是否必有中枢/事件以支撑正控，均未实证；未读 #1585 票面原文（不调 gh），X-38 是否票面自称无法确认；未看 Python/Lean 侧是否有段账本直读（票面限 Rust 路径）。
