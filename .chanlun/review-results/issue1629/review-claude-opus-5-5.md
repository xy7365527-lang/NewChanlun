# #1629 独立审查（claude-opus-5-5，非作者，只读）

- 审查对象：`codex/1629-endpoint-prune`（HEAD `6b270b145d`），相对 #1628 head `2d95fd36f8` 的 `rust/` diff
  （evidence.rs +258/-32、stroke.rs +50、issue1588_parse_mem_probe.rs +17；另有 `.chanlun/review-results/issue1629/bench.sh`）。
- 方法：逐分支人工推导不变量与复杂度，交叉核对下游消费者；**本机未跑 cargo**（按指令），测试读数采信远端
  `cargo test --release --lib theta_v0::parser` 190/0 及给出的成本读数。

## 结论：**APPROVE**

无 HIGH。核心算法（split / union / join）在所有分支上不变量成立，输出元素序列与 #1628 逐位相同，
成本锁能抓住 #1628 式回退。下面的 MEDIUM / LOW 是锁的覆盖缺口和文档措辞问题，不阻塞合入，建议本 PR
或后续顺手修。

---

## 1. 正确性核验（结论：成立）

**join / balance（evidence.rs:124-166，#1628 原实现）对任意高度差都正确。**
设 `h(L) ≥ h(R)+2`，`L = Concat(a,b)`，则 `h(b) ≥ h(L)-2 ≥ h(R)`，所以 `h(join(b,R)) ∈ [h(b), h(b)+1]`。
又 `|h(a)-h(b)| ≤ 1`，得 `h(join(b,R)) - h(a) ∈ [-1, 2]`，只会右重且最多差 2。`balance` 右重分支
（:137-147）的单旋（`h(rb) ≥ h(ra)`）与双旋（`ra = Concat(c,d)`，`h(c),h(d) ∈ {h(a)-1, h(a)}`）
逐一代入后各节点差都 ≤ 1，左重对称。归纳下去，join 对任意两棵 AVL 输出 AVL，高度字段由 `branch`
统一计算（:118-121），不会失真。递归深度 = 高度差，O(log n)。

**split_node（:229-247）。**
- 整棵 ≤ key 或整棵 > key 时原指针返回；单叶必落其中一支，所以 :237 的 `unreachable!` 确实到不了。
- `node_max(left) ≤ key` ⟹ left 整体在低侧，`lo ⊆ right`，`join_opt(left, lo)` 满足「左 < 右」；
  另一支对称。
- 输出两侧有序、无交、并集等于输入，平衡性由 join 保证。

**union_nodes（:259-292）。**
- `ptr_eq` 时集合相同，返回 a 正确。
- b 是叶走 `insert_node(a,x)`，`None`（已含）时返回 a；a 是叶走 `insert_node(b,x)`，返回 b 或新节点。
- 值域不相交时 join，严格 `<`，边界相等不会误判。
- 一般分支：`lo` 全部 ≤ `max(left)`，`right` 与 `hi` 全部 > `max(left)`，所以 `new_left` 全部 < `new_right`。
  两侧值域不交，不会产生重复；`lo ∪ hi = b`，不会丢元素。
- **「左右都没变就返回原 a」不丢元素。** 归纳可证 `ptr_eq(union_nodes(x,y), x)` ⟺ `y ⊆ x`：
  - 返回 x 只有三条路：ptr 相等、insert 返回 `None`、递归两侧都没变。
  - join/branch 总是新建 Rc，不会碰巧等于 x。
  - a 为叶时返回的是 b 或新节点；如果 b 的指针就是 a，第一行已经命中。
  所以 :288 判两侧都没变时 `lo ⊆ left` 且 `hi ⊆ right`，返回 a 精确。
- 终止性：每次递归 a 严格下降一层，深度 ≤ h(a)。

**调用方。** stroke.rs:196/:549 的 `expect` 都受前置守卫保护（:166 全部共享、:541 两侧共享），不会 panic。
`confirmation_sources` 两侧都为空时，`union` 的 `(None,_)` 分支返回空，正确。

**测试独立性。** `coords_ref_union_matches_set_semantics_and_keeps_inputs`（evidence.rs:754）的对照物是
BTreeSet，不是实现镜像。它逐节点核验高度字段和平衡度，核验输入未被改写，覆盖单元素（`round%7==0`
时 range=1）、部分重叠、不相交（offset 150/300）、同指针、空集、同谱系新旧版本双向，以及
「旧 ⊆ 新时返回原指针」。完整事实对拍 `shared_full_facts_match_flat_reference_for_every_prefix`
（stroke.rs:1602）以 #1597 前的扁平构造为参照，比对 PartialEq、`{:?}`、`{:#?}`、JSON 四种输出，
是本 PR 等价性的独立证据，不是镜像。

## 2. 与 #1628 逐位等价（结论：成立）

- 新旧实现输出的都是升序去重集合 `base ∪ others`，内容相同。`SourceCoords` 的 Debug、Serialize、
  PartialEq 都只看元素序列（evidence.rs:595-647）。PartialEq 的 `ptr_eq` 短路（:618）是「指针相同 ⟹
  内容相同」，只会更快，不改结果。`SourceCoords` 未实现 Hash。
- 下游 `content_producer.rs` 的 `array_from_node`（:2535-2604）遍历的是 **IncrFactsState 包含证据树**
  （`coords_array`、`ref_array_over`、`dir.coords.root_node()`，:740/:801/:2300）。这些树由
  inclusion.rs 构造，本 PR 未改动。它按物理节点做缓存，键是 `Rc::as_ptr`，并持有强句柄，但缓存的值
  是 `sequence_concat` 得到的规范 M 序列；:5961 起的 skewed 对拍测试锁定了形状无关性。
- 新笔端点、确认证书的 `source_coords`（本 PR 改动的树）在 `content_bi.rs` 中一律经 `to_vec()` 或
  `iter()` 消费（:1094/:1123/:1260/:1676/:1750），不读树形状。
- 结论：树形状变化不进入任何内容身份。

## 3. 复杂度（结论：在完整事实路径上与前缀解耦，polylog；文档界偏乐观）

- **完整事实路径上的输入都同属一个持久谱系。** `process_inclusion_with_facts`（inclusion.rs:612）
  只在坐标严格递增时走 IncrFactsState，否则回落扁平（`shared_endpoint_sources` 返回 `None`）。
  证据链满足 `E_{g+1} = concat(concat(E_g, tail_g), leaf)`（inclusion.rs:1103-1110）。两处调用都以
  max 较大的一侧（较新、超集）为 `a`，这正是剪枝有效的方向。
  - 窗口内 `acc = E_{mid+2}`，并入各祖先版本时都走「⊆ 返回原 a」。
  - 组成员已含在 `E_{mid+2}` 中时，`insert` 不分配。
  - 我没找到在完整事实路径上能让两棵树大面积不共享的输入形态。
- **退化形态：两棵树内容重叠但不共享指针。** 例如两条独立谱系，或同一集合按不同插入顺序建出的树。
  此时 `T(n) = 2T(n/2) + O(log² n)`，等于 Θ(n)·polylog，比 #1628 的线性归并慢一个 polylog 因子。
  完整事实路径到不了这种形态；但 `CoordsRef::union` 是 `pub`，并经 inclusion.rs:978 公开再导出，
  外部调用方可能触发（见 L-3）。
- **栈深度无风险。** AVL 高度 ≤ 1.44·log₂(4.6M) ≈ 32。union 递归深度 ≤ h(a)，每帧内 split、insert、
  join 的递归各自 ≤ 32，叠加不超过约 100 帧，帧很小。Rc 链的 Drop 递归深度也就是树高。
- **insert_node 每层调用 node_max，O(log² n)，可以接受。** 这是 #1628 的遗留。每次约 h²/2 ≈ 250 次
  指针跳转，组成员总插入次数 O(n)，总量是 O(n log² n) 量级的常数项，远低于 Θ(n²)。后续可在 Concat
  节点缓存 min/max 降到 O(log n)。不过 `CoordNode` 是公开枚举，content_producer 用
  `Concat(left, right, _)` 做模式匹配，加字段会破坏下游，不建议在本 PR 做。

## 4. 发现

### MEDIUM

**M-1 成本锁计的是函数入口次数，大头工作没有计入；读数不能直接当「每端点步数」用。**
- 位置：evidence.rs:196-206、:229-231、:259-260、:295-296
- 计数方式：`bump_ops` 只在 `split_node`、`union_nodes`、`insert_node` 入口各计 1。
- 没计入的工作：
  - 每层 `node_max`/`node_min` 的 O(h) 下行：split 每层调用 3 次，union 每层调用 5 次；
  - `join`/`balance` 的递归与分配。
- 影响：真实指针跳转数大约是读数（148-232、152-217）乘以 10-30 倍。仍是 polylog，结论「与前缀解耦」
  不受影响，但 PR 描述把它称作「树操作步数」会让人误读绝对成本。≥1M 和 4.6M 前缀的墙钟与 RSS
  收据目前没有出现在 diff 里，`bench.sh` 只有脚本，没有日志。
- 建议：
  - 把注释和 PR 文案改成「递归调用次数」；
  - 或在 `node_max`/`node_min`/`join` 里也调用 `bump_ops`；
  - 附上 `bench.sh big/full` 的收据。

**M-2 「不逐元素遍历」锁只覆盖 `SourceCoords::iter`，其他展开口没有探针。**
- 位置：evidence.rs:571 只有这一处 `bump_iter`；`CoordsRef::materialize`（:384-401）、
  `CoordsRef::count`（:415-431）、`SourceCoords::len/to_vec/contains` 都不计数。
- 能抓住的：#1628 式回退（`base_view.iter()` 归并），以及 `confirmation_sources` 改回
  `union_into(base, other.iter())`，两者都会让 `iters > 0`。
- 抓不住的：今后在笔事实构造里引入 `.len()`、`.to_vec()`、`materialize()`。这类 Θ(i)-per-endpoint
  回退 `iters == 0` 照样成立，`ops` 也不变，两把锁都放行。
  - 例如有人加一句 `debug_assert_eq!(src.len(), …)`：远端跑的是 `--release`，debug_assert 不生效，
    但同类的非 debug 代码会漏掉。
- 建议：在 `materialize` 和 `count` 的循环里也调用 `bump_iter()`，零成本，仅测试构建。

### LOW

**L-1 `shared_endpoint_sources` 的文档注释过期。**
- 位置：stroke.rs:149-155
- 现状：仍写着「其余各集与 B 做有序归并……归并逐元素核对」。
- 建议：按 :192-193 的新注释改写，说明证据树走 `CoordsRef::union`，组成员逐个持久插入。

**L-2 「零分配」措辞不准。**
- 位置：evidence.rs:254、:373-374
- 问题：`b ⊆ a` 时最终返回原 a，不留存新节点；但 `split_node` 沿切分路径的 `join_opt`（:242/:245）
  仍会新建临时节点，随后丢弃。
- 另外，「返回原句柄」对**所有** `other ⊆ self` 成立，不需要「结构共享」这个前提；结构共享影响的只是成本。
- 建议：改为「不新建留存节点（返回原句柄）」，并把成立条件改成 `other ⊆ self`。

**L-3 `union_nodes` 文档的复杂度界偏乐观，公开 API 没有说明退化形态。**
- 位置：evidence.rs:256-257、:373-375
- 问题一：「差异点数 × O(log² n)，与集合大小无关」没有证明。每个差异接缝上，union 递归 O(h) 层，
  每层 split 还要 O(h²) 的 `node_max`，按指针跳转算更接近 O(log³ n)/接缝。「与集合大小无关」实为 polylog。
- 问题二：`CoordsRef::union` 是 pub 并经 inclusion.rs:978 再导出。对不共享指针的大树，它约为
  Θ(|b|)·polylog，比有序归并更慢，文档里应该写明。
- 建议：改为「在同一持久谱系（共享子树）的输入上为 polylog；不共享时退化为 Θ(min(|a|,|b|))·polylog」。

**L-4 并集成本锁只测了「近版本、单一方向」。**
- 位置：evidence.rs:827-851，`older` 只比 `tree` 早 20 次追加（:834-835）。
- 缺口：`confirmation_sources` 的实际输入是相隔整笔若干组的两个版本。按推导，成本主要受接缝数
  O(h) 限制，与相隔组数无关，但目前没有锁住。
- 建议：再加一档 `older = tree@n/2`，并断言 ops 同样只按 polylog 增长。
- 反向 `union(old, new)` 不在生产调用路径上（两处调用都以 max 较大者为 a），可以不锁，但最好在
  `CoordsRef::union` 文档里写明「`self` 取较新、较大的一侧才有剪枝收益」。

**L-5 `CoordNode` 不变量文档列出的构造口不全。**
- 位置：evidence.rs:102-104
- 现状：写着「构造口只有 concat 与 append_greater」。实际还有 #1597 的 `insert` 和本 PR 的 `union`。
- 建议：顺手更新。

**L-6 stroke 成本锁的阈值与夹具单一。**
- 位置：stroke.rs:1574-1597
- 现状：单一 LCG 种子，阈值 3.0×，实测 1.43×。真实 1m 行情的组尺寸、笔跨度分布可能不同。阈值本身
  合理：log² 的理论增幅约 1.96×，线性回退是 16×。
- 建议：如果远端已有 BTC 前缀的端点数与 ops 读数，把它们写进 PR 作为补充证据即可，不必入测试。

## 5. 其他核对

- **cost_probe 零开销、无行为差异。** `cost_probe` 模块是 `#[cfg(test)]`（:179）；`bump_*` 体内的语句
  也带 `#[cfg(test)]`，非测试构建下是空的 `#[inline(always)]` 函数。计数器是 thread_local `Cell`，
  只增不读，不参与任何分支。libtest 每个测试一个线程，计数互不污染；`build_stroke_facts` 内部没有派生
  线程，计数完整。
- **issue1588_parse_mem_probe.rs 的改动只影响 bin。** `ISSUE1629_LAYER_ONLY` 只跳过摘要，不改构造，
  测得的墙钟仍含笔事实构造，符合意图。其余是 rustfmt 噪音。
- **reward hacking：未发现。** 两个对照物（BTreeSet 和扁平参照构造）都独立于实现。成本锁的唯一软肋
  是 M-1/M-2 所述的计数口径，不是镜像实现。
