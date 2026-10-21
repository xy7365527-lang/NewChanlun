# #1597 非作者独立审查（只读）

- 被审：`codex/1597-coords-shared` @ `0f747b938c`；审查面 `git diff e7f309be83 0f747b938c`（10 文件，+718/−62）
- 方法：逐行读源码与测试；未编译、未运行 cargo（本机禁令），测试结论以作者远端运行为准，本报告只判断测试**能锁住什么**。
- 行号均指 `0f747b938c`。

## 结论：APPROVE（附 2 条 MEDIUM 建议，均不阻塞正确性）

没有发现会让共享导出与扁平参照分叉的缺陷。逐位等价有独立的逐字 Debug 对拍兜底，退化域不自递归，AVL 插入与有序归并在不变量上正确。主要不足在于留存形状锁对「回落扁平」盲视，以及票面验收要求的真实数据探针读数不在审查包内。

## 1. 逐位等价

**Debug / `{:#?}`**：`SourceCoords` 的 Debug（evidence.rs:467）是 `f.debug_list().entries(self.iter()).finish()`；`Vec<usize>` 的 Debug 经切片走的也是 `debug_list().entries(iter())`，条目类型都是 `&usize`，美化模式由同一个 `DebugList` 处理。两者按构造相同，空集都是 `[]`。

**serde**：evidence.rs:474 `serialize_seq(Some(len))` 后逐个 `serialize_element`。serde 对切片/Vec 的实现是 `collect_seq`，切片迭代器 size_hint 精确，默认 `collect_seq` 走的也是 `serialize_seq(Some(len))` 再逐元素序列化。serde_json（含 `to_value`，tb02b.rs 在用）没有覆盖 `collect_seq`，输出相同。

**PartialEq 的 `ptr_eq` 捷径**（evidence.rs:486）：`CoordNode` 不可变，同根 Rc 必然是同一逻辑序列，捷径只会返回 true，不会让不相等的值判等，是安全的。空树（root 为 None）落到逐元素比较，Shared 与 Flat 之间按元素序列比较，和 Vec 语义一致（顺序敏感、不去重）。

**`insert_node` / `join`**（evidence.rs:179、:246）：
- 有序性：`coord <= node_max(left)` 时插左子树，左段最大值不变，仍小于右段最小值；否则插右子树，新坐标大于左段最大值。等于已有坐标时在叶上返回 `None`，句柄原样返回，所以结果仍是升序去重。
- 平衡：原节点满足 AVL，单侧插入后高度差至多 2；`join` 在差为 2 时下降一层，`join(b, right)` 的高度差 ≤1，直接 `branch`，然后 `balance` 做单旋或双旋。`balance` 里两处 `unreachable!` 的前提（高度 ≥2 必为 Concat）成立。
- 持久性：只沿插入路径新建节点，未修改的子树经 `clone()` 共享，旧版本不变；测试里每 500 个元素保存一次旧版本快照并回核。
- 递归深度：`insert_node` 的深度等于树高，O(log n)；每层 `join` 至多递归一层；`node_max` 用循环实现。单次插入 O(log² n) 时间，新建 O(log n) 个节点。Drop 递归深度同样受树高约束。

**`union_into`**（evidence.rs:526）：基底 B 是升序去重的。对 others 的每个 x，游标跳过所有小于 x 的元素，再判断游标处是否等于 x。只要 x 比上一个元素小（进入新的一段，或输入非升序）就从头重走游标，所以每个 x 都对 B 做了一次精确的成员判断。段内重复元素相邻，游标不会越过它；`missing` 中的重复项在 `insert` 时命中 `None`。因此结果精确等于 `sort(B ++ others).dedup()`，对多段或非升序输入都成立。

**端点与确认来源**：`shared_endpoint_sources`（stroke.rs:157）只要有任一证据是 Flat 就返回 `None`，回落 `sorted()`；组成员不参与这个形态判定，但并集对它们是精确的，不受影响。`confirmation_sources`（stroke.rs:530）在任一侧是 Flat 时回落扁平构造。增量路径（`expand_sources=false`）得到的是 Flat 空集，与原来的空 Vec 逐位相同。

## 2. 退化域

- `process_inclusion_with_facts`（inclusion.rs:612）的严格递增判据与 `IncrFactsState::append` 的降级判据（`incoming <= previous`）同义，非单调输入直接进入 `_flat`，不会构造 IncrFactsState，`debug_assert` 与此一致。空输入和单根输入满足 `windows(2).all`，走共享路径，被 `n < 12` 的前缀覆盖。
- `step_fact`（:1421）、`group_fact`（:1453）、`materialize_groups`（:1506）、`materialize_facts`（:1540）在 degraded 时全部改调 `process_inclusion_with_facts_flat`，从而消除了「公开入口 → IncrFactsState → 退化 → 公开入口」的潜在互递归。`shared_step`/`shared_group` 返回 Err，行为不变。
- `parser/mod.rs:243` 的 degraded 分支仍调用公开入口；由于输入非单调，它会直接进入 `_flat`，不递归。

## 3. 留存复杂度

- **步骤**：`step.coords` 来自 #1394 的 `concat(evidence.coords, group_tail).append_greater(incoming)`，每步新建 O(log n) 个节点。现在 `step_fact` 和 `expand_evidence` 直接包装这些句柄（O(1)），不再 `materialize`，于是 `steps[*].source_coords` 与 `direction_evidence` 都共享同一批节点。等待步骤用的 `raw_prefix` 是共享的追加链。
- **组证据**：同一个 `EvidenceNode.coords` 被组与步骤共享。
- **端点**：方向证据链是单调超集，新证据等于前组成员 ∪ 旧证据 ∪ incoming，所以窗口内最大值最大的那份证据覆盖其余证据和前两组成员，缺的只是末组锚点之后的成员。每组在至多一个窗口里充当这种未覆盖的末组，ΣK = O(n)，每次插入新建 O(log n) 个节点。
- **确认证书**：以 f 为基底，a 基本被 f 的证据链覆盖，K 很小。`pairs`/`strokes` 克隆 `Endpoint` 时 `SourceCoords` 是 O(1) 克隆。
- 合计 O(n log n)。注意，O(n log n) 这个界依赖上面的证据链形状，正确性则不依赖。形状一旦不成立，结果仍然精确，只是留存会退回更差的量级。

**仍逐步展开的字段（遗漏面）**：`InclusionStepFact.high_sources/low_sources` 仍是逐步 `Vec` 克隆（inclusion.rs:1441 `v.as_ref().clone()`，扁平参照同样如此）。它的规模是组内同极值根数 m，不随前缀增长，但一段长时间的平盘（连续同价 K 归入同一组）会给出组内 Θ(m²)。`groups[*].members` 合计 O(n)，`extreme_roots` 有界。见 LOW-3。

## 4. 测试是否真正锁住

- `assert_per_prefix_bit_exact`（inclusion.rs:2076）：参照是基本未改的扁平实现（只加了 `.into()`），公开入口、`materialize_facts`、`step_fact`、`group_fact` 都在每个前缀上与它对拍，并额外比较紧凑与美化 Debug 字符串。即使 `PartialEq` 写错（比如恒真），Debug 对拍也会抓到，两边走的是不同的遍历代码（切片迭代 vs 树栈迭代），所以不是同义反复。degraded 分支比较的是同一函数自己，这一点与修前相同，不新增缺口。
- `shared_full_facts_match_flat_reference_for_every_prefix`（stroke.rs:1568）：900 根合成数据，stride 1 和 5 两组，覆盖包含层与新笔事实的 PartialEq、两种 Debug 与 JSON，并断言步骤坐标与端点坐标为共享形态。这是有效的等价锁。
- `coords_ref_insert_and_union_into_match_set_semantics`（evidence.rs:590）：3000 次随机插入与 BTreeSet 对照，回核旧版本、树深 ≤30（AVL 上界约 17）；并集用例含非升序、重复、越界元素，并检查基底不变。有效。
- `shared_full_facts_retain_subquadratic_nodes`（stroke.rs:1609）：**对回落扁平盲视**，见 MEDIUM-1。
- 未见奖励黑客迹象：参照实现没有被改写去迁就新实现，断言没有被放宽，修复也没有借助测试专用分支。

## 5. 公开合同影响

- 受影响调用方：`s_session_v2/content_bi.rs`（:1091、:1120、:1257、:1673、:1747）、`tb02a_facts.rs`（:20、:501、:742）、`tb02b.rs`（:169、:188、:257）、`content_producer.rs`/`s_session_v3/object_records.rs`（测试）、`s_structure_session.rs:3285`、探针。全部只是把借用 `&Vec` 改为 `.to_vec()` 后再传 `&[usize]`，或者用 `extend(&SourceCoords)`（`&usize` 迭代，`Vec<usize>: Extend<&usize>`）。值相同，原行为保持。`serde_json::to_value(ep)` 输出相同（见 §1）。其余仅调用入口函数的地方（`s_structure_session.rs:3134/:6511`、`content_bi.rs:2141`、`tb02c.rs:411/496/571`、`generation.rs:490/2078`）不读这些字段，或读取后结果不变，类型检查由编译保证。
- `!Send`：`InclusionFacts`、`StrokeFacts`、`GroupParseEvidence`、`Endpoint` 原本是 Send + Sync，现在因为含 `Rc` 都变成 `!Send`。`ParseLayer` 与 `IncrFactsState` 本来就含 Rc。我在 s_session v2/v3 与 s_structure_session 中找到的 `thread::spawn` 都只传 IO/客户端对象，没有发现跨线程传递这些事实的地方。这是编译期约束，作者远端编译通过即可证明当前没有破坏，但它是合同收窄，见 LOW-1。

## 6. 探针

默认 `ISSUE1588_WINDOW=50000`，`tail[5..]`、`[..n]`、`ThetaConfig::default()` 与 #1588 口径逐位相同；`layer`/`facts` 的 Debug 摘要输入不变（Debug 逐位相同），所以摘要值可以和票面的 `52e595c5…0236` / `21d820a0…65df7` 直接对比。变化有三处：
- facts 行多了 `alloc_total_mb` 字段；
- 摘要行改成 `digest layer=… stroke_facts=… facts=…`；
- `cap` 列只统计 Flat 形态（:151），所以「坐标 Vec 容量占 retained」那一列与 #1588 表不可比。`elems` 仍是逻辑元素数，和票面「来源坐标元素数合计」可比。

仓内没有脚本解析这些行。新增的 `layer` 模式和 `ISSUE1588_SKIP_FACTS_DIGEST` 不影响默认行为。

## 发现

### HIGH

无。

### MEDIUM

**M-1 留存形状锁对回落扁平盲视**（stroke.rs:1613、:1663）。`walk` 遇到非共享坐标直接 `return`，扁平元素既不计入 `seen` 也不计入任何留存量。如果确认证书（`confirmation_sources`）或组证据回退成扁平 Θ(i) 展开，`seen.len()` 只会更小，断言照样通过。doc 注释说「回到逐步展开…即失败」，这只对步骤坐标和端点坐标成立，因为另一个测试断言了它们是共享形态；对确认证书和组证据不成立。建议在 `walk` 里对 `as_shared().is_none()` 的集合把 `c.len()` 计入留存上限（或者直接断言所有被访问的集合都是共享形态或空集），这样这个锁才能覆盖所有 Θ(n²) 字段。

**M-2 票面前提「调用方不在生产热路径」不再成立，且验收读数缺席**。`endpoints_from`（stroke.rs:217，`expand_sources=true` 在 :231）同时服务生产 S 路径 `parse_layer_with_group_evidence`（`s_session_v3/generation.rs:490`），所以生产端点来源现在也走 `union_into`，下游用 `.to_vec()` 展开。正确性由逐前缀对拍覆盖。性能上，`union_into` 每段 O(|B|) 游标、下游每个端点要多次 `to_vec`（content_bi 中每个端点、配对、笔各一次），常数因子和原来的「拼接 + 排序」同阶，但没有实测。另外，票面验收要求的真实数据读数（探针 `facts` 模式 2k→20k 的 retained 近线性，layer/facts 摘要与 `52e595c5…0236` / `21d820a0…65df7` 一致）不在审查包内。建议合入前附上探针的 2k/5k/10k/20k 读数和一次生产 S 回放的耗时对比。

### LOW

- **L-1** `SourceCoords` 基于 `Rc`，使 `InclusionFacts`/`StrokeFacts`/`GroupParseEvidence` 由 Send 变为 `!Send`（evidence.rs `SourceRepr`）。当前没有消费方受影响，建议在类型文档里写明，或者等将来真有跨线程需求时改用 `Arc`。
- **L-2** `union_into` 的 doc（evidence.rs:524）写「非升序时退化为逐元素插入」，与实现不符：实现是在每次下降时从头重走基底游标，最坏 O(|others|·|B|)。结果仍然精确，建议改写措辞。
- **L-3** `high_sources/low_sources` 仍逐步克隆（inclusion.rs:1441），长平盘组内 Θ(m²)。它与前缀长度无关，不在本票 Θ(n²) 主因内，可以另记。
- **L-4** 没有直接把 `SourceCoords` 和真实 `Vec<usize>` 的 Debug/JSON 逐字对拍；现有测试比较的是 Shared 与 Flat 两种 `SourceCoords`。两者按构造相同（§1），补一个三行单测成本很低。
- **L-5** `SourceCoords::len()` 在共享形态下是 O(len)（evidence.rs:367），Serialize 时会先计数再遍历，走两遍；探针的 `coords()` 也为每个集合遍历一次。可以接受，但不宜在循环里调用。
- **L-6** stroke.rs:175 `.map(|r| Rc::as_ptr(r))` 可写成 `.map(Rc::as_ptr)`（clippy `redundant_closure`；CI 不跑 clippy，不阻塞）。
