# #1630 非作者独审（claude-fable-5-1，只读）

- 审查对象：PR #1659，分支 `sandcastle/issue-1630` HEAD `b34edde595`，base origin/main `02b5b571eb`。
- 路由：Claude Code CLI 经 CLIProxyLocal（本机代理端口 8318），模型 `claude-fable-5-1`；作者为 Sandcastle 实装工蜂，非本审查者。
- 工具限制：Read/Grep/Glob 与只读 git；未跑 cargo。
- 审查提示原文：`prompt.md`（同目录）。
- 日期：2026-10-21。

---

结论：PASS

## 总评

本 PR 不改生产代码，只在探针里加了一个逐 bar 段续扫跨度读数（`rescan_span`）并附定位报告。我对照 `segment.rs` 核实了 `rescan_span` 与库里 `IncrSegments::append` 的实际续扫起点一致（`keep ≥ 1` 时精确相等，其余三种情况为高估，报告已如实声明为上界代理），无越界/panic 风险，不影响原有摘要输出。报告的定位链（旧分支复现 → 与 main 首个可跑探针的提交对照 → parser 差异唯一为 #1582）在 git 层面逐条核对成立，读数内部算术自洽，结论措辞没有夸大，不确定处都打了「推断」「精确的阶没有测定」标签。交付纪律合规。以下只有 LOW 级建议，不阻塞。

## HIGH

无。

## MEDIUM

无。

## LOW

1. **`rust/src/bin/issue1588_parse_mem_probe.rs:13` 与 `:357`**：`spans` 在 `window()` 之后、循环之前 `with_capacity(bars.len())`，全程存活，所以它不仅计入 `alloc_total`（注释已说明），也抬高 `peak_mb` 4 B/bar（2M 根时约 8 MB，占 4,879.6 MB 的 0.2%）。注释只提了 `alloc_total`，建议补一句 peak 也含在内，免得日后拿 peak 与修前对比时多出一个解释不了的常数。
2. **`.chanlun/review-results/issue1630/REPORT.md:11`**：结论 1 写「main 自 #1596 合入（439be77560）起就是线性的」，真实的消除点是更早的 24c2fa068f（#1582，2026-10-20 已在 main）；439be77560 只是探针首次存在、能跑读数的最早 main 提交。结论 3 已澄清，但结论 1 单看会让人误以为 #1596 修的。建议改成「自探针可用的最早 main 提交 439be77560 起即线性」。
3. **`REPORT.md:35`**：标题「二分」实际上不是二分——旧分支与 439be77560 之间 9 个提交没有逐个跑，而是靠「parser 目录差异 = #1582 的差异」推定。这个推定我核过是成立的（见下），逻辑有效，但称「对照 + 差异归因」更准确；main 侧那几行是线性复核，不是二分步骤。
4. **`REPORT.md:60`**：对拍段落说 10k/20k 的末 bar 摘要（`ee7ee036…`/`eb8fa282…`）与 `da537bfbcb` 未改探针的读数逐位相同，但二分表只列了 49,995 一档的修前摘要（`7a3586de…`），10k/20k 的修前摘要没有落在表里，只在正文断言。建议把三档修前摘要都入表，便于日后复核。
5. **`REPORT.md:3`**：编号声明里 #1628、#1633 除声明行外全文未再出现，无害，可删。

## 逐项核实记录

**1. diff**：`git diff 02b5b571eb...HEAD` 只含两文件，一个提交 b34edde595；提交说明已读。

**2. `rescan_span` 与库续扫起点一致性**（`segment.rs:1254-1453`，`mod.rs:387-436`）：
- 库里 `keep = end_indices.partition_point(end_idx < confirmed_bound)`，truncate 到 `keep`，`resume_seg_start = end_indices[keep-1] + 1`，返回态 `confirmed_len = keep`，`ParseLayer.segments_confirmed_len` 直接取它（`mod.rs:427`），`segments` 是同一个 Rc（`mod.rs:413`）。探针用 `segments[keep-1].end_index` 反查 `strokes.partition_point(end_index <= end)`；因 `make_segment` 的 `end_index = strokes[s1].end_index`（`segment.rs:868`）且笔的 `end_index` 严格递增（相邻笔共端点），结果恰为 `s1 + 1 = resume_seg_start`。库自己的 `from_checkpoint`（`segment.rs:1212-1228`）也用同一匹配假设。
- 高估的三种情况与报告/注释所述一致：同输入早退（`segment.rs:1261`，实际不扫）、`keep == 0`（`segment.rs:1339` 从 `find_overlap_start` 起）、`n < 3`。`resume_seg_start >= n` 分支探针给 0，与实际一致。`can_reuse_segments=false`（`mod.rs:404`）走 `empty().append`，探针给 n，仍是上界。
- 越界：`segments_confirmed_len ≤ segments.len()` 在 append 各返回路径均成立（truncate 到 keep 后只 push），`partition_point ≤ len` 故减法不下溢；`print_spans` 的 `spans[n-1]` 在 n=0 时不可达（`run_replay:370` 先 `expect("n ≥ 1")`，既有行为）。
- 原有输出：`print_spans` 在原两行 println 之后追加一行，`alloc_total`/`peak` 读取位置未动；49,995 档修前 223.6 → 修后 223.8 MB 的 0.2 MB 差正好等于 4 B × 49,995，与注释自洽。

**3. 定位链**（命令均在 worktree 内执行）：
- `git log -1` 解析到报告引用的全部提交：24c2fa068f（#1582 合入，10-20）、439be77560（#1596，10-21）、d4e6a20230（#1597/#1628）、084ebd700f（#1581/#1584）、6c6acd5fe0（#1625）、1bc3534014（分支尖，`fix(#1588)` nit 提交）以及四个临时插桩提交 bdc93fcb35/e24d109235/51617d0b0e/5969582f86 均可达。
- `git merge-base --is-ancestor`：24c2fa068f 不是 1bc3534014 的祖先、是 439be77560 的祖先——「旧分支不含 #1582」成立。
- `git log 1bc3534014..439be77560 -- src/theta_v0/parser` 只有 24c2fa068f 与 439be77560 两条，与报告结论 3 一致。
- `git diff --shortstat` 两次结果完全相同（8 files, +2501, −241）：`1bc3534014→439be77560` 的 parser 差异与 `24c2fa068f~1→24c2fa068f` 的 parser 差异同尺寸，支持「#1596 squash 的 parser 内容 = 分支内容 + #1582」。
- 区间内 parser 之外的 rust 改动（s_session bins、`l3_delta_r_alpha.rs`、tb1490 fixtures）都不在探针回放路径上，`config.rs` 未动，无混淆变量。
- `git show 24c2fa068f` 的 `feature_seq.rs` 片段确认：修前 `OpenAtPrefixEnd` 走 `skipped_secondkind = true; continue`（跳过后继续判后续候选），修后与预算耗尽合并为 `unresolved_blocked = true; return None`（封段）。报告对旧语义「未决候选钉住 euf、每 bar 从早起点重扫」的描述与此吻合。
- 报告内算术抽查：×4.7、Σ跨度 3.17 倍、phase5 5.36 倍、45.9/77.5 B/笔、各档 KB/bar、均值×n≈Σ 全部复算一致。

**4. 交付纪律**：提交说明无 fix/close/resolve #号；有编号声明且与引用一致（#1630/#1634 Issue，#1582/#1596 PR）；REPORT 编号声明与正文用法一致（#1581/#1597/#1629 均按 Issue 用）；新增函数注释带 #1630。

## 未核实

- 没有跑 cargo，所以报告里所有读数（分配量、跨度统计、末 bar 摘要逐位相同、线段数 152 vs 524、重建次数为 0）只做了内部一致性检查，未独立复现；远端日志 `/tmp/nc-remote/...` 不在本地。
- 临时插桩提交的内容（分阶段归账是否正确）未审，报告也声明不合入。
- 「#1582 之后为何跨度维持在几十笔」只核了语义改动方向，没有从代码推导跨度上界；报告自己也明说这是数据决定的界而非结构界。
