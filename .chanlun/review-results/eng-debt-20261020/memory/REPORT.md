# #1588 memory 分片：解析入口内存二次增长（#1493 (E)）

- 分支：`eng-debt-1588/memory`（基于 origin/main `1d70db893f`）
- PR：#1596；转票：#1597
- 远端：windows-5090 WSL Ubuntu，`~/.local/bin/nc-remote`，release 构建，日志在 WSL `~/nc-remote/logs/`
- 数据：`btc_1m_full.json`（sha256 `16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`，探针启动时校验）。窗口与 #1493 探针相同：`all[len-50000..][5..]` 取前 n 根。
- 探针：`rust/src/bin/issue1588_parse_mem_probe.rs`（`--features backtest_bin`）。计数全局分配器报告 `alloc_total`（窗口内累计请求字节）、`peak`（窗口内存活峰值）、`retained`（调用后仍存活）。Debug 输出流式写入 sha256 作逐位摘要。每个 (mode, n) 一个进程。

复现：

```bash
cd rust && cargo build --release --locked --features backtest_bin --bin issue1588_parse_mem_probe
P=$CARGO_TARGET_DIR/release/issue1588_parse_mem_probe; D=<path>/btc_1m_full.json
for n in 2000 5000 10000 20000; do $P $D facts $n; done
for m in replay-drop replay-hold classifier; do for n in 2000 5000 10000 20000 49995; do $P $D $m $n; done; done
$P $D classifier 2000 1; $P $D classifier 20000 50; $P $D classifier 49995 50
```

| 运行 | 提交 | 日志 |
|---|---|---|
| 修前曲线 | `da78044035`（只加探针） | `20261020-145238-da78044035.log` |
| 修后曲线 + incremental/parser 测试 | `7775135c0f` | `20261020-150150-7775135c0f.log` |
| 反证（撤回 `last_l0 = None` 一行，临时分支，未推送） | `0265c224cd` | `20261020-151858-0265c224cd.log` |
| 收缩容量后 facts 曲线 + fmt/check/测试复核 | `c262c34b73` | `20261020-152224-c262c34b73.log` |

---

## 条目 1（E-b）：跨 bar 持有 ParseLayer，导致逐 bar 写时整表拷贝

**现象**：`IncrementalClassifier::classify_at` 逐 bar 回放的累计分配随 n 二次增长。49,995 根累计分配 127,944 MB，而存活峰值只有约 106 MB。

**根因**：`ParseLayer` 的 `merged_bars/fractals/strokes/segments` 是 `Rc<Vec<_>>`，与增量状态共享同一块缓冲：
- `IncrFactsState.merged`：`inclusion.rs:993/1058` 的 `Rc::make_mut`
- `IncrFractals.fractals_rc`：`fractal.rs:183`
- `IncrStrokes.strokes`：`stroke.rs:636`
- `IncrSegments.segments_rc`：`segment.rs:1104`

`make_mut` 在 strong_count > 1 时深拷贝整表。`classify_at` 的写法是 `let l0_i = self.parser_incr.append(..); …; self.last_l0 = Some(l0_i);`，`append` 执行期间上一 bar 的 `last_l0` 仍存活，所以每根 bar 四张表各整表复制一次：单 bar O(n)，累计 O(n²)。`fractal.rs:181` 的注释已写明「调用方须丢弃上一轮 ParseLayer 保 O(1)」，生产调用方没有遵守。`s_structure_session.rs`（main :3113 `layer = Some(incr.append(bar))`）和 4 个诊断 bin（`l0 = parser.append(bar)`）也是同一模式：赋值的旧值在右侧求值期间仍存活。

**测量（修前 → 修后，`classifier` 模式）**

| n | alloc_total 修前 | alloc_total 修后 | 耗时 修前 → 修后 | peak 修前 / 修后 |
|---|---|---|---|---|
| 2,000 | 244.1 MB | 28.2 MB | 0.10 → 0.05 s | 4.4 / 4.2 MB |
| 5,000 | 1,470.4 MB | 109.4 MB | 0.21 → 0.10 s | 11.2 / 10.6 MB |
| 10,000 | 5,144.8 MB | 278.6 MB | 0.56 → 0.28 s | 22.7 / 21.5 MB |
| 20,000 | 20,511.6 MB | 937.6 MB | 1.49 → 0.46 s | 46.0 / 43.6 MB |
| 49,995 | 127,944.2 MB | 4,554.0 MB | 6.21 → 1.95 s | 106.2 / 99.4 MB |

纯解析对照（`ParseLayerIncr` 回放，修前修后数字相同，因为探针自己控制持有方式）：

| n | replay-hold alloc_total | replay-drop alloc_total |
|---|---|---|
| 2,000 | 224.2 MB | 8.4 MB |
| 5,000 | 1,386.0 MB | 25.1 MB |
| 10,000 | 4,919.2 MB | 52.9 MB |
| 20,000 | 20,000.4 MB | 426.5 MB |
| 49,995 | 125,408.6 MB | 2,018.4 MB |

修前 `classifier` ≈ `replay-hold` + 塔开销。修后 `classifier` 的 alloc_total 回到 `replay-drop` 量级加塔开销。RSS（VmHWM）在这几种模式下约 1.29 GB，几乎都是加载 JSON 的基线，不能反映差异，所以用分配器计数。

**处置：修**（提交 `bcb153d79e`）：
- `backtest/incremental.rs::classify_at`：`append` 前先 `self.last_l0 = None`。
- `bin/s_structure_session.rs`：`drop(layer.take())` 后再 append。
- 4 个诊断 bin（`p_issue668_bsp_bridge_battery`、`p_issue668_bsp_key_truth`、`issue550_event_battery`、`p126_d3_descending_clock`）：`drop(l0)` 后再 append。
- `parser/mod.rs::ParseLayerIncr::append` 文档补持有合同。
- 回归测试 `backtest::incremental::tests::classify_at_keeps_shared_parse_tables_in_place`：逐 bar 断言 `merged_bars/strokes` 的 Rc 地址不变，`fractals` 在 merged ≥ 3 后不变；另有一个持有反例臂，断言 `ParseLayerIncr` 持有上一轮 layer 时 merged 至少换块一次。

**逐位对拍**（同一输入，修前 `da78044035` 对修后 `7775135c0f`）：

| 检查 | 修前 | 修后 |
|---|---|---|
| classifier 2k 逐 bar classification 滚动摘要 | `e8d69ceab365…5dfacb5` | `e8d69ceab365…5dfacb5` |
| classifier 20k 每 50 bar 滚动摘要 | `2a3cd44fda9f…8e2f1` | `2a3cd44fda9f…8e2f1` |
| classifier 49,995 每 50 bar 滚动摘要（修前侧 = 反证提交） | `00658642444d…742e75d2` | `00658642444d…742e75d2` |
| classifier 末 bar ClassifyOutput（2k/5k/10k/20k/49,995） | `a09ab131…` / `a3ade875…` / `3ad8259e…` / `3ad8259e…` / `e99e438f…` | 同左 |
| classifier 末 bar last_l0（同上 n） | `51f9d33d…` / `9c26b466…` / `2d1a5023…` / `2b142ac1…` / `0dc6f8f8…` | 同左 |
| replay-hold 对 replay-drop，5k 逐 bar layer 滚动摘要 | `7fe1cda47227…cd073` | `7fe1cda47227…cd073` |

（10k 与 20k 末 bar 的 ClassifyOutput 摘要相同，修前修后都是这样。这一段新增 bar 没有改变段与塔，L0 摘要不同。）

**测试**（`7775135c0f` / `c262c34b73`）：`cargo test --lib theta_v0::backtest::incremental` 9 passed / 0 failed / 24 ignored（含新测试）；`cargo test --lib theta_v0::parser` 166 passed / 0 failed / 9 ignored；`cargo fmt --all -- --check` ok；`cargo check --all-targets --features backtest_bin` 与 `cargo check --features s_session --bin s_structure_session` 通过。

**反证**（`0265c224cd`，只撤回 `self.last_l0 = None;` 一行）：新测试失败，`assertion left == right failed: bar 1: merged_bars 被整表拷贝`；探针 `classifier 49995` 的 alloc_total 回到 127,944.6 MB，摘要不变。反证提交**未推送**。复现方式：撤回 `incremental.rs:101` 的 `self.last_l0 = None;` 一行，再跑 `cargo test --lib theta_v0::backtest::incremental::tests::classify_at_keeps_shared_parse_tables_in_place` 与 `issue1588_parse_mem_probe <json> classifier 49995 50`。该日志的命令行有一段中文字符串转义出错（`bash: … command not found`），尾行 `EXIT=0` 也与 `test result: FAILED` 不符（grep 管道吞了退出码），不过正文的测试结果与探针输出仍能支持结论。非作者评审（`reviews/1596-claude-fable-5.md`）用本地临时 ref `refs/review/1596-counter` 独立重做了反证，日志 `/tmp/nc-remote/20261020-173355-9755108096.log`：新测试 FAILED（`bar 1: merged_bars 被整表拷贝`），`classifier 5000` 回到 1,470.4 MB，`classifier 20000` 回到 20,511.7 MB，摘要与修后相同。

**修后残余**：`replay-drop` 的 alloc_total 仍略超线性（20k 426.5 MB → 49,995 2,018.4 MB）。这是增量段状态在不可复用时重建（`parse_layer_from_compact_facts` 中 `can_reuse_segments=false` 分支）和笔尾部重扫的累计分配，存活峰值线性（约 2 KB/bar）。它不是持有问题，本分片不处理，列为遗留观察。

## 条目 2（E-a）：完整事实入口的输出本身是 Θ(n²)

**现象**：`parse_layer_with_inclusion_facts` 单次调用的留存随 n 约 ×4／翻倍。20k 修前 retained 9,530.9 MB（WSL VmHWM 9.45 GB，与 #1493 记录的 9.3 GB 吻合）。

**根因**：`InclusionFacts` 中展开的来源坐标 `Vec<usize>`。探针分项计数显示，坐标 Vec 容量占 retained 的 95.9%～99.7%。
- `inclusion.rs::process_inclusion_with_facts`（:603 起）：新建组时方向证据 = 上一组成员 ∪ 旧证据 ∪ incoming，第 i 步的 `direction_evidence.source_coords` 与 `source_coords` 都为 Θ(i)，每步各存一份展开副本，Σi = Θ(n²)。组的方向/确认证据同理。
- `stroke.rs::endpoint_at`（expand_sources=true，仅完整事实路径）：每个端点拼接相邻三组的成员和证据坐标，Θ(i)；端点数 Θ(n)。`pairs/strokes` 按值克隆端点。

20k 分项（元素数）：steps.source/high/low 200.1 M；steps.direction_evidence 200.0 M；groups 证据 240.7 M；stroke_facts.endpoints 54.2 M（**容量 433.6 M**）；pairs 108.4 M；strokes 46.7 M（容量 62.2 M）。

**可隔离部分，处置：修**（提交 `c262c34b73`）。端点坐标拼接后 `sort_unstable + dedup`，去重后约剩 1/8，容量不回收。`stroke.rs::sorted` 加 `shrink_to_fit()`，只改容量。

| n | retained 修前 | retained 修后 | layer 摘要（修前=修后） | facts 摘要（修前=修后） |
|---|---|---|---|---|
| 2,000 | 107.8 MB | 72.9 MB | `e67faacc…0615` | `7eb9541d…76d6` |
| 5,000 | 654.7 MB | 439.2 MB | `98fa48d6…1160` | `8d178804…bbb2` |
| 10,000 | 2,303.7 MB | 1,592.5 MB | `108e003e…3468` | `e52218bd…75ac` |
| 20,000 | 9,530.9 MB | 6,518.1 MB | `52e595c5…0236` | `21d820a0…65df7` |

**主体部分，处置：转票 #1597**。理由：消掉 Θ(n²) 必须改 `InclusionFacts/InclusionStepFact/InclusionGroupFact/InclusionDirectionEvidence/stroke::Endpoint/Confirmation` 公开字段的类型或语义。消费方位于 S 会话写者（`tb02a_facts.rs`、`content_producer.rs`、`s_structure_session.rs`、`s_session_v2/*`、`s_session_v3/generation.rs`），是 #1488 的在途区域，parser/ 一带又有 #1493/#1582 在途。完整事实入口的调用方都是测试或 oracle（`s_structure_session.rs:3128` oracle 开关、:6505 测试；`content_bi.rs:2141` 测试；`tb02c.rs:398/483/558` 测试；`generation.rs:2078` oracle），不在生产回放热路径上。#1394 已有共享表示（`evidence.rs::CoordsRef`、`IncrFactsState`、`parse_layer_with_group_evidence`），#1597 的建议方案是复用它，或把剩余调用方迁到共享路径。查重：`gh issue list --search "解析入口内存二次增长"` 与 `--search "InclusionFacts source_coords 内存"` 均无重复（只命中 #1588 母票）。

## 共享 target 污染窗口复跑

**问题**：2026-10-20 12:00–15:38，WSL 上的 `nc-remote` 让所有调用共用同一个 `CARGO_TARGET_DIR`，并发作业可能互相覆盖二进制。主控已把 `nc-remote` 改成按作业私有 `NC_TARGET` 加 `flock`，原脚本备份在 `~/.local/bin/nc-remote.bak-20261020`。本分片上表四份日志（14:52–15:22）都落在这个窗口里，测量结果理论上可能来自别的分支构建的二进制。

**复跑**：主控用 `NC_TARGET=eng1588` 私有 target 串行复跑同一组命令：
- 修前 `da78044035`（ref `eng1588-mem-pre`）：`/tmp/nc-remote/20261020-172803-da78044035.log`
- 修后 `e52cbb0171`（ref `eng1588-mem-post`）：`/tmp/nc-remote/20261020-173051-e52cbb0171.log`

两份日志头都是 `target=/home/hanju/nc-remote/eng1588`。

**结论**：测量数字不受污染窗口影响，本报告的表格无需更正。
- 主控核对：31 组 alloc/peak 数字与本报告表格逐项一致；全部摘要修前修后逐位相同。
- 作者另行逐行 diff（去掉 `secs=` 后比较）：
  - 两份复跑日志各 21 组：facts 2k/5k/10k/20k；replay-drop、replay-hold、classifier 各 2k/5k/10k/20k/49,995；classifier 2k 逐 bar；classifier 20k 每 50 bar。
  - 修前复跑与 `145238-da78044035` 的 alloc_total、peak、retained、计数和摘要逐字相同。修后复跑的非 facts 组与 `150150-7775135c0f` 逐字相同，facts 组与 `152224-c262c34b73` 逐字相同。
  - 修前复跑与修后复跑的全部 layer/rolling 摘要逐字相同。
- 复跑未包含两项，仍以窗口内原日志为准：
  - replay-hold/replay-drop 5k 逐 bar 摘要（`7fe1cda4…`）。
  - classifier 49,995 每 50 bar 摘要（`00658642…`）。修前一侧只有反证提交 `0265c224cd` 的日志，修后一侧有 `7775135c0f`、`c262c34b73` 两份日志，三份都在窗口内。

## 与在途分支的关系

按 `memory-coordination.md` 的指示：未碰 #1582 改动的 `segment.rs/feature_seq.rs` 和 parser 测试文件；`IncrSegments::append` 未改，修复全部在持有方/调用方一侧。`parser/mod.rs` 只加文档；`stroke.rs` 改 1 行（不在 #1582 列表内）。`s_structure_session.rs` 在 main :3113 加 2 行，抽查 #1488/#1527/#1419 在途分支的 hunk，无重叠。
