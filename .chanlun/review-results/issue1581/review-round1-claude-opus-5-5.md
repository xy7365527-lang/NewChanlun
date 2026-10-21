# #1581 非作者独审第 1 轮（归档入仓）

- 原件：外盘 \`/Volumes/AgentStorage/issue1581-review/result-r1.md\`（负责人 /root/owner_1581 发起，claude-opus-5-5 经 CLIProxyLocal）；第 1 轮审 \`c9b8ee96c2\`，第 2 轮审 \`2163a39728\`
- 入仓：主控合入代理 /root/merge_train，2026-10-21，原文未改

---

**结论：PASS_WITH_FIXES。** 实现本身是对的，而且是 fail-closed：段端门失败后，下游在标记期间确实什么都不构造，也不会悄悄放过。两个问题需要处理：一是生产路径上的标记接线没有回归锁；二是作者自己的探针（seed 4）在 #1493 分支上找到了一个真实的生产门失败，这件事本身属于 #1493 的问题。本次只做静态阅读和读日志，没有运行 cargo。

## 1. 三处读取是否正确、是否 fail-closed

**四条装配路径用的是同一个判定。**
- 两条 facts 路径取 `SegmentFacts.parse_invalid_from.seg_start`。
- 生产增量路径取 `IncrSegments::parse_invalid_from()`。
- 批量路径用 `IncrSegments::empty().append(...)` 从头扫一遍取标记。

三者最后都落到 `end_gate(...) == Invalid` 时的 `get_or_insert(start)`，也就是扫描中第一次失败的那个段起点（`segment.rs:1015-1019`、`:1410-1411`）。

**批量路径不会出现"段"和"标记"对不上的情况。**
- 从空状态 append 时，续扫起点等于 `find_overlap_start(strokes,0)`，和全量核的 `seed` 相同。
- 首段起点都用 `first_segment_start(seed, ...)`，主循环逐行镜像，两边都不带 second store。
- 所以段取自 `divide_segments_with_tail`、标记取自增量核，在 #1493 声明的"增量核与全量核逐位相同"前提下等价。
- 代价是批量入口多扫一遍段，代码注释已经说明。

**尾部处理正确。**
- `build_tail` 在标记非空时整体不构造 `PendingSegment`；笔层、分型层的尾部照常。
- `tail.rs` 新增的单测有正控（不带标记时确实有 `PendingSegment`），能证伪。
- 连带效果：`active_segment_frontier`（`nest_lifecycle/pan.rs:127`）只认 tail 里的 `PendingSegment`，标记期间会返回 None，L1 活窗跟着 fail-closed。

**PartialEq 纳入标记不会误伤现有对拍。**
- 增量和全量的标记来自同一个判定。如果对拍因此变红，暴露的是增量核的真实分歧，这正是想要的效果。
- 静态阅读看到一处属于 #1493 核心、不归本 PR 的小边界：`append` 的三个提前返回分支（`segment.rs:1302/1322/1334`）保留了上一轮的标记，没有按续扫起点过滤。只有笔数回缩时才可能留下过期标记。登记为观察项，不要求本 PR 处理。

## 2. classifier 入口整层拒读

**做法合理。**
- `cache.clear()` 只清可以重新推导的塔缓存，候选事件簿保留（`tower_cache.rs:355` 的注释和代码一致）。
- 返回值只读事件簿，本 bar 不写新事件。
- 下一个无标记的 bar 会从清空的缓存全量重建，seed 4 的逐 bar 对拍证实了这一点（见第 3 节）。
- 整层拒读比"截到失败点之前"更严。作者给的理由是：各级末端判定以账本末端为"当下"，截断后会把失败前的结构当成当下输出。这个理由成立，也符合"下游无产出"的验收口径。

**`classify` 也覆盖到了。** 全量入口就是空缓存加 `classify_incremental`，所以 nautilus 和 strategy 走的 `append_bar` / `classify` 都会被拦住。

**仍然绕过标记的下游**（都在票面"三处"之外，不阻断本 PR，但应另开跟进票，或在 §15 登记）：
- `backtest/capture_oos.rs:327-370`：直接按 `layer.segments_confirmed_len` 和 `layer.segments[j]` 开仓、平仓，完全不经过 classifier。这是最需要处理的一处。
- `backtest/wverify_run.rs:3062` 的 `cascade_dual`：直接从 `l0.segments` 复算中枢级联。
- `classifier/diag/cp_recall_audit.rs:354`、`diag/cand_delta.rs:163`：诊断用途。
- `backtest/segment_gn.rs`：统计测试。

## 3. 测试能不能证伪

**能证伪的部分：**
- `tb1581_classifier_refuses_marked_layer_with_warm_cache` 有正控（不带标记时有产出），删掉入口拒读就会变红。
- tail 单测删掉过滤会变红。
- `tb1581_injected...` 能锁住 facts 路径的标记写入，也能锁住"批量增量核标记 == facts 标记"。

**锁不住的部分：**
- **生产增量路径 `parse_layer_from_compact_facts` 和批量路径 `parse_layer_from_inclusion_state` 的标记写入，没有任何非 ignored 测试锁。** 把这两处改回 `None`，所有常驻测试照样全绿。注入测试只走了 `parse_layer_from_strokes`，也就是 facts 路径。
- `tb1581_injected` 里 `classify(&bad)` 的三条断言没有正控，用这 15 笔的输入，classifier 本来就可能产出空结果，断言可能是空转。
- `cache.clear()` 本身没有被单独锁住。

**seed 4 的差异完全由标记解释。** 我逐行比对了两份逐 bar 摘要：
- 差异恰好是 bar 1999–2035 共 37 bar，layer 和 out 两个摘要都不同。
- 这 37 bar 改后 `levels=0`，out 摘要等于空输出。
- bar 1998 和 bar 2036 起两边逐位相同，说明清空缓存后重建是逐位一致的。

**一个缺口：** 汇总摘要在 bar 1999 还混入了批量和 facts 层，但 `layer_text` 不含新字段。所以"批量标记 == 增量标记 == Some(100)"没有任何断言证据，只有一个已被删掉的旧版探针曾打印过。

## 4. #799/#804（同一判断宽严两档）

- 标记只来自一个判定（`end_gate`），四条装配路径共用，没有违反 #804。
- 宽严两档的风险只存在于第 2 节列出的那些绕过者：classifier 拒读，而 `capture_oos` 照常在同一份无效段账本上交易，效果上就是同一判断出现了宽严两档。建议跟进票统一处理。

## 5. 要升级给 #1493 的事（不是本 PR 的缺陷）

裁定 4b 原文写的是："任一生产路径出现 `segments_parse_invalid_from.is_some()` 即视为 #1493 验收失败并阻断发布"，并认为这条路径"无已知可达输入"。

本 PR 的探针在 #1493 分支上找到了一个可达输入：随机游走 `tb1581_bars(0x1581_0004, 4000)`，在生产增量路径上于 bar 1999–2035 出现 seg_start=100 的真实门失败。这不是注入的，而且是首段之外的门失败。

这个反例必须回报到 #1493。按裁定，它阻断 #1493 的发布；而本 PR 基于 #1493 分支，合入节奏也会受影响。

```json
{"verdict":"PASS_WITH_FIXES","must_fix":["生产增量路径 parse_layer_from_compact_facts 与批量路径 parse_layer_from_inclusion_state 的标记写入没有非 ignored 测试锁：加一个常驻测试（例如复用 seed 4 前 2000 根，或用 FIRST_START_AT_SEED 驱动 ParseLayerIncr），断言增量/批量/facts 三路标记相等且非空、尾部无 PendingSegment、classifier 产出为空","把 seed 4 随机游走在生产增量路径上的真实段端门失败（bar 1999–2035，seg_start=100，首段之外）作为反例回报 #1493：按裁定 4b 这是 #1493 验收失败、阻断发布，本 PR 基于该分支，合入须等 #1493 处理"],"should_fix":["tb1581_injected 里 classify(&bad) 的断言补正控（同一输入去掉标记时 classifier 有非空产出），否则可能空转","另开跟进票或在 §15 登记仍直接读 l0.segments、绕过标记的下游：backtest/capture_oos.rs（按段确认直接开平仓，最需要处理）、wverify_run.rs cascade_dual、diag/cp_recall_audit、diag/cand_delta；否则 classifier 拒读、capture_oos 照常交易，构成同一判断宽严两档（#799）","对拍摘要或逐 bar 测试加断言：增量标记与 bar 1999 处的批量/facts 标记相等（目前 layer_text 不含新字段，这一点无证据）","classify_incremental 文档的「## 边界」节补一句标记拒读语义","（#1493 核心，不归本 PR）IncrSegments::append 三个提前返回分支保留上一轮标记而不按续扫起点过滤，笔数回缩时可能留下过期标记，登记给 #1493"]}
```
