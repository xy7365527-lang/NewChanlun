# 引擎架构重构研究：在保持逐bar流式的前提下消除 O(N²)

> 状态：研究报告（不含代码改动）
> 日期：2026-06-08
> 认识论等级总览：全文性能预期为 **L0**（算法复杂度推导）。落地后需 **L1**（管线逐位等价）+ **L2**（真实数据 wall-clock）验证。任何"已消除 O(N²)"的表述都是设计断言，不是实测结论。

---

## 0. 执行摘要

### 0.1 推荐结论（先于论证）

**不存在单一银弹方案。** 三个剩余 O(N²) 瓶颈的障碍本质不同，强行用一种架构改造覆盖三者会引入与障碍不匹配的复杂度（违反 no-patch-mentality 的"严格 = 直面问题的本质形式"）。

推荐采用**分而治之的"稳定前缀检查点（Stable Prefix Checkpoint, SPC）统一化"** 作为主干，对三个瓶颈施加各自匹配的解：

| 瓶颈 | 障碍本质 | 推荐解 | 架构归类 |
|------|---------|--------|---------|
| bi changed-case 重建 | 稳定前缀未变却被全量拷贝 | 结构共享（前缀段缓存 + 尾部追加），即受控 CoW | 方案1 的退化形式 |
| BSP `_find_move_for_seg` | 单调结构上用了线性扫描 | 二分 + 末尾游标缓存（SPC 投影到 Move 层） | 方案4 + 算法修正 |
| `_scan_zhongshu` | 把 confirmed 误当 stable，前缀非单调 | 锚定 SegmentEngine 已证明的稳定前缀做 resume | 方案4 的正确实现 |

**被否决的方案及理由**：
- **方案2（pyrsistent 全局持久化结构）**：遍历开销（每访问 O(log₃₂N)）落在 `segments_from_strokes_v1` 的逐 cursor 热路径上，与本系统"密集顺序遍历"的访问模式相克。
- **方案3（纯事件溯源替换快照接口）**：事件流已存在（DomainEvent 双轨已在跑），但批量回测/可视化/FSM 需要随机访问语义，全量切换要求重写所有消费者，信息增量为零而成本极高——典型的成本收益不划算且非必须（瓶颈不在快照传递，在每层全量重算）。

**语言重写（方案五，§4.6）的定位**：换高性能语言（Rust/C++/Cython/Mojo）是**正交于算法重构的独立维度**——它降低常数因子（10-50×），不改变复杂度阶数。首选 **Rust + PyO3** 重写闭合引擎子图、Python 保留外围。它与 SPC 可叠加：既可作为 SPC 后仍不达标的"放大器"，也可在 Rust 内直接实现 SPC（一次拿降阶+降常数）。**Cython 是纯常数优化，不消除 O(N²)，须与 SPC 叠加；Mojo 因生态不成熟 + SIMD 重排威胁逐位等价而否决。** 走哪条路径由 §6 P0 的 L2 profiling 裁决。

### 0.2 一句话诊断

> 当前的"O(N²) 残余"不是快照可变性问题，而是**三处"在已知单调的结构上做了非增量操作"**。修复方向是把已经在 SegmentEngine 中验证过的"稳定前缀单调不变量"推广到 zhongshu 与 BSP，并把 bi 的前缀拷贝改为结构共享。

---

## 1. 当前架构精确画像

### 1.1 五层流式管线

`orchestrator/recursive.py:190-234` 的 `process_bar` 每 bar 顺序驱动五层 + 递归栈：

```
Bar
 → BiEngine.process_bar()              → BiEngineSnapshot.strokes
 → SegmentEngine.process_snapshot()    → SegmentSnapshot.segments
 → ZhongshuEngine.process_segment_…()  → ZhongshuSnapshot.zhongshus
 → MoveEngine.process_zhongshu_…()     → MoveSnapshot.moves
 → BSPEngine.process_snapshots()       → BuySellPointSnapshot.buysellpoints
 → RecursiveStack.process_level1_…()   → list[RecursiveLevelSnapshot]（level ≥ 2）
```

返回 `RecursiveOrchestratorSnapshot`（`recursive.py:38-54`），聚合五层快照 + 递归层快照 + `all_events` + `lstar`。

### 1.2 快照语义（关键事实，反直觉）

| 维度 | 实际情况 | 证据 |
|------|---------|------|
| 元素可变性 | **全部 frozen** | `Stroke`/`Segment`/`Zhongshu`/`Move` 均 `@dataclass(frozen=True, slots=True)`（`a_stroke.py:42`、`a_zhongshu_v1.py:35` 等） |
| 容器可变性 | **可变 list** | 各 Snapshot 的 `strokes`/`segments`/… 字段是普通 `list[...]`，Snapshot 本身 `@dataclass` 未 frozen |
| 拷贝策略 | **条件零拷贝** | 结果不变 → 返回缓存引用（`bi_engine.py:449-462`、`segment_engine.py:200-203`、`zhongshu_engine.py:86-93`）；结果变化 → 构建新 list |
| 消费者行为 | **全部纯读** | segments_from_strokes_v1（`a_segment_v1.py:592`）、zhongshu_from_segments（`a_zhongshu_v1.py:177`）等只读字段，不 mutate 容器 |

**结论**：当前"immutable snapshot"的表述在**元素层成立**（frozen），在**容器层不成立**（可变 list）。但由于"返回缓存引用 + 整体替换、从不原地 mutate"的纪律（`segment_engine.py:201` 注释明示），实际语义等价于不可变——只是该不变量靠注释和审查维持，不靠类型系统强制。

### 1.3 已有的隐式缓存层（fix1-6 的产物）

系统**没有显式 dirty flag**，而是用"输入指纹短路 + 检查点 resume"两类机制：

| 引擎 | 短路键（O(1) 检测输入未变） | resume 机制 |
|------|------|------|
| SegmentEngine | `_last_tail_key`（末两笔） | **有**：`_cp_segments` + `_cp_stable_count`（稳定前缀检查点） |
| ZhongshuEngine | `_last_seg_key`（倒数第二段身份） | **无**：每次变化全量 `zhongshu_from_segments` |
| MoveEngine | `_last_zs_key` | 无 |
| BSPEngine | `_last_input_key` | 无 |
| RecursiveStack / Level | `_last_input_key` / `_last_move_key` | 无（仅短路缓存） |

**关键观察**：SegmentEngine 已经独立解决了"confirmed 前缀可能回退"的难题（见 §3.3），这套机制是 zhongshu/BSP 增量化的现成模板。

---

## 2. 核心矛盾的再诊断

### 2.1 任务描述的前提

> "当前架构每 bar 返回完整快照（immutable snapshot），下游消费者可能保留旧快照引用。增量优化需要避免重建完整快照，但不知道谁在保留旧引用。"

### 2.2 证据对前提的修正

调查结果对该前提做三点修正（这是后续方案选择的基础）：

1. **"不知道谁保留引用" → 可完整枚举。** 跨 bar 持有快照引用的消费者是有限且可列举的：
   - `RecursiveStack._cached_snapshots`（短路缓存，`recursive_stack.py:44`）
   - `RecursiveLevelEngine._prev_zhongshus / _prev_moves`（diff 基准，`recursive_level_engine.py:55-56`）
   - `SegmentEngine._cp_segments`（resume 检查点，`segment_engine.py:173`）
   - `ph_layer.should_stop_recursion(curr, prev_moves)`（跨 bar 比较，`ph_layer.py:115-137`）
   - `ReplaySession.event_log`（全历史快照，`replay.py:70`）
   - `gateway._live_snapshots[symbol]`（最新快照缓存）

2. **持有者全部纯读。** 上述消费者无一原地 mutate 容器。"跨 bar 引用风险"是**理论风险**（可变 list 允许 mutate），不是**实际风险**（无人 mutate）。

3. **元素 frozen，容器可变。** 真正的脆弱点是容器层的 `list` 没有类型级不可变保证，而非元素。

### 2.3 真正的根因

把"O(N²) 残余"归因为"快照可变性 / 引用安全性未知"是**错误定位**。逐个解剖三个瓶颈（§3）后，真正的根因是三处独立的算法-结构失配：

> **在已知具有单调不变量的结构上，执行了非增量的全量操作。**

- bi：稳定前缀单调（confirmed 不回退）→ 却每次 `list(cp[:base_len])` 全量拷贝。
- BSP：`Move.seg_start` 严格单调 → 却用无序线性扫描 `_find_move_for_seg`。
- zhongshu：SegmentEngine 已证明"稳定前缀单调" → 却把"全部 confirmed"（含会回退的尾部）当输入全量重扫。

这个再诊断直接决定了"为什么方案2/3 是过度设计"：它们解决的是"如何安全共享可变快照"，而真正要解决的是"如何在单调前缀上 resume"。后者是局部的、可枚举的，不需要全局数据结构革命。

---

## 3. 三个瓶颈的精确解剖

### 3.1 瓶颈一：bi changed-case 重建（~8.5s / 640K bars）

**位置**：`bi_engine.py:464-467`

```python
# 结果变化 → 构建完整列表（O(N)，仅 27.5% 的调用）
result_strokes = list(cp[:base_len])   # ← 拷贝整个确认前缀（~6000 笔）
result_strokes.extend(tail)
return result_strokes
```

- `cp = self._cp_strokes`（确认笔的持久前缀）跨 bar 保留，前缀部分**单调不变**。
- 每个 changed-case bar 拷贝 `cp[:base_len]`，base_len ≈ 6000，累积 Σ O(i) = **O(N²)**。
- 代码已部分优化：尾部用 `tail = [cp[-1]]` 隔离，72.5% 调用走零拷贝快路径（`bi_engine.py:449-462`）。剩余 27.5% 仍全量拷贝前缀。

**障碍本质**：纯结构问题——**前缀没变却被拷贝**。无任何不变量障碍（前缀确认笔单调不变，已被代码注释 `bi_engine.py:375-382` 确认）。

**"跨 bar mutate 风险"的真相**：`_extend_prev_stroke`（`a_stroke.py:150-161`）会原地替换 `strokes[-1]`。若直接返回共享前缀的 list 给下游，且某 bar 在尾部 mutate，理论上污染已发出的快照。但 §2.2 已证：(a) 替换的总是**尾部**元素（不在稳定前缀内）；(b) 下游纯读。所以风险可用"前缀只读共享 + 尾部 copy-on-write"消解，无需全局 CoW。

### 3.2 瓶颈二：BSP `_find_move_for_seg`（~13s / 640K bars）

**位置**：`a_buysellpoint_v1.py:128-137`

```python
def _find_move_for_seg(moves: list[Move], seg_idx: int) -> Move | None:
    for m in moves:                                  # ← 无序线性扫描 O(N_moves)
        if m.seg_start <= seg_idx <= m.seg_end:
            return m
    return None
```

- 每 bar 在 Type2/Type3 检测中被调用 O(N_segments) 次，每次 O(N_moves) → per-bar O(N_moves × N_segments)，累积超线性。
- **关键单调性**：`Move.seg_start` 严格单调递增（由 Zhongshu.seg_start 单调性保证，`a_move_v1.py:159-162`）。
- 仅最后一个 Move 半稳定（`settled=False`，`seg_end` 可随新中枢闭合而变，但单调非递减）。

**障碍本质**：纯算法问题——**单调有序数组上用了线性查找**。`seg_start` 单调 → 二分可达 O(log N_moves)；末尾查询用游标缓存可达 O(1) 摊还。无架构障碍。

**注意（严格性）**：当前 `_find_move_for_seg` 仅用于 BSP 的 `settled` 字段赋值（confirmed-fix 后已与 confirmed 判定分离，`a_buysellpoint_v1.py:130` 注释）。其优化价值取决于 `settled` 字段的实际消费频率——若 Move 列表通常 < 100，该瓶颈的绝对收益可能小于 §3.1/§3.3。**这是一个需要 L2 profiling 复核的优先级问题**，不应在未测量前断言其为主导项。

### 3.3 瓶颈三：`_scan_zhongshu`（~7s / 640K bars）

**位置**：`a_zhongshu_v1.py:123-168`（外层循环）+ `_extend_zhongshu:91-111`（内层扫描）

```python
while i + 2 < n:                                      # 外层 O(N)
    ...
    seg_end_idx, j, gg, dd = _extend_zhongshu(confirmed, i, n, zd, zg)  # 内层 O(N)
    ...
    if settled:
        i = max(break_seg_idx - 2, seg_end_idx)       # ← 续进锚点回退 2 段
    else:
        break
```

- 每 bar `ZhongshuEngine.process_segment_snapshot` 全量调用 `zhongshu_from_segments(segs)`（`zhongshu_engine.py:96`），无 resume。
- 内外层叠加在长中枢形态下退化为 **O(N²)**。

**障碍本质**：这是三者中**唯一真正的不变量问题**。调查报告将其表述为"confirmed segment 非单调，resume 不安全"。精确化如下：

**误解来源**：`zhongshu_from_segments` 过滤 `confirmed AND settled`（`a_zhongshu_v1.py:177`）。直觉上"confirmed 段应单调增长"，故可缓存"已扫描到第 k 段"的检查点。但 diff 层（`segment_state.py:145-175`）会发 `SegmentInvalidateV1`，**使先前 confirmed 的尾部段回退**——confirmed 前缀会收缩。若 zhongshu 检查点锚定在"全部 confirmed 段"的边界，回退后该检查点指向已删除的段，resume 索引越界/错配。

**但这个障碍 SegmentEngine 已经解出。** `segment_engine.py:116-180` 的 `_update_checkpoint` 区分了两个概念：

- **confirmed**：`seg.confirmed == True`（非最后一段）——**会回退**。
- **stable**：confirmed **且** `break_evidence` 的扫描窗口已完全展开（`gap_type=="second"` 时 `trigger_k + 1 + MARGIN <= n_strokes`）——**一旦稳定永远稳定**（`segment_engine.py:123-128` 证明：稳定条件随 n_strokes 单调，只增不减）。

`_cp_stable_count` 标记的"稳定前缀"是 confirmed 前缀的**单调子集**。SegmentEngine 在其上 resume 是安全的，已落地（`segment_engine.py:164-177`，标注 L0 逐位等价）。

**zhongshu 的严格解（非 workaround）**：zhongshu resume 失败不是因为"无法 resume"，而是因为它把检查点错误地锚定在"全部 confirmed"而非"stable 前缀"。把检查点锚定到 SegmentEngine 已计算的 `_cp_stable_count` 边界（且 zhongshu 自身续进锚点 `max(break_seg_idx-2, seg_end_idx)` 的回退区也落在该前缀内），则：

1. stable 前缀内的段不回退 → 其上已扫出的中枢不变。
2. 续进回退 2 段仍在 stable 前缀内 → 锚点不指向易变区。
3. 非单调只发生在 stable 前缀之外的尾部 → 不污染缓存中枢。

**这是 no-workaround 要求的解**：不绕过非单调，而是精确定位单调的子集（stable 前缀），在其上 resume；非单调的尾部每 bar 重扫（尾部长度 O(1)~O(常数)，不累积成 O(N²)）。

---

## 4. 四种方案逐一评估

### 4.1 方案1：Copy-on-Write (CoW) 快照

**机制**：快照共享底层数据，仅被修改部分拷贝。前缀共享 → 消费者持旧引用安全。

| 维度 | 评估 |
|------|------|
| 可行性 | 中。Python 无原生 CoW 数组，需自定义"前缀段（不可变）+ 尾部（可变）"的复合 list-like 类。 |
| 性能预期（L0） | 对 **bi 瓶颈直接命中**：前缀共享消除 `list(cp[:base_len])` → O(N²) 降为 O(N)。对 zhongshu/BSP **无效**（它们的障碍是扫描/不变量，不是拷贝）。 |
| 改造成本 | 局部。`BiEngineSnapshot.strokes` 改为复合结构，下游若只做顺序遍历 + 索引访问可透明兼容（需实现 `__getitem__`/`__len__`/`__iter__`）。 |
| 风险 | 自定义容器的 `__getitem__` 每次访问有分支开销，可能拖慢密集遍历。需 benchmark。 |

**结论**：是 bi 瓶颈的**正确解的一种形式**，但其完整形态（通用 CoW）对本系统是过度设计。退化为"前缀只读共享 + 尾部追加"即足够（见 §5.1）。

### 4.2 方案2：持久化数据结构（pyrsistent PVector）

**机制**：结构共享的不可变向量，修改返回新版本，大部分节点共享。

| 维度 | 评估 |
|------|------|
| 可行性 | 高（库成熟，PVector 有 C 加速）。 |
| 性能预期（L0） | 前缀共享天然消除拷贝类 O(N²)。**但每次元素访问 O(log₃₂ N)**。 |
| 改造成本 | 高。所有 `list[...]` 字段替换为 PVector，所有 `strokes[cursor]` 访问点语义不变但常数变大。 |
| 致命问题 | `segments_from_strokes_v1`（`a_segment_v1.py:592`）、`_scan_zhongshu`（`a_zhongshu_v1.py:140-166`）是**密集顺序遍历**的热路径。把 O(1) 数组访问换成 O(log N) 节点遍历，会在最热的代码上引入常数惩罚——可能抵消甚至超过省下的拷贝。 |

**结论**：访问模式与数据结构相克。本系统的读访问（密集顺序遍历）远多于写（尾部追加），PVector 优化的是"频繁随机写 + 共享",恰好不是本系统的 profile。**否决**。

### 4.3 方案3：事件溯源（Event Sourcing）

**机制**：不返回完整快照，返回事件流；消费者 replay 重建所需状态。

| 维度 | 评估 |
|------|------|
| 可行性 | 中。**事件流已存在**——每层已产生 `DomainEvent`（Candidate/Confirm/Settle/Invalidate），快照 + 事件双轨已在跑（`zhongshu_engine.py:98-104` 等）。 |
| 性能预期（L0） | 对实盘增量极优（只处理 delta）。对**批量回测无收益甚至负收益**：回测需随机访问历史状态，replay 重建 = 把"读"换成"重算"。 |
| 改造成本 | 极高。需重写所有依赖完整快照随机访问的消费者：`ReplaySession.seek`（`replay.py:88-90`，已是 reset + 重跑）、`a_level_fsm_adapter`（`level_views_from_recursive_snapshot`）、`gateway`、可视化脚本、PH 层。 |
| 信息增量 | **零**。事件已经存在，把消费者从"读快照"改成"读事件 replay"不产生新能力，只换接口。 |

**结论**：违反"成本收益消解禁止"的反面——这里是**高成本零信息增量**。瓶颈不在快照传递（§5 证明传递已是零拷贝引用），而在每层全量重算。事件溯源不触碰重算瓶颈。**否决作为主干**；但事件流作为**实盘路径的既有能力**应保留（它已经是双轨的一轨）。

### 4.4 方案4：分层快照 + 脏标记（系统化的 SPC）

**机制**：每层维护"稳定前缀检查点"，只重算 stable 边界之后的尾部，stable 前缀 resume。

| 维度 | 评估 |
|------|------|
| 可行性 | 高。**SegmentEngine 已是该机制的实证存在**（`_cp_stable_count`）。其余层是同构推广。 |
| 性能预期（L0） | 对 **zhongshu 直接命中**（§3.3 的严格解）。对 BSP 命中（SPC 投影到 Move 层 + 二分）。对 bi 部分命中（bi 的 SPC 即 `_cp_strokes` 前缀，配合结构共享）。 |
| 改造成本 | 中。需为 zhongshu/BSP 各实现一个锚定上游 stable 前缀的检查点。依赖图清晰（线性管线，每层只依赖直接上游的 stable 前缀）。 |
| 风险 | 检查点不变量的正确性证明（需逐位等价测试，对标 SegmentEngine 的 L0 标注）。 |

**结论**：与系统现有架构同构，是三瓶颈中两个（zhongshu、BSP）的**正确解**，且与 §2.3 的根因诊断（"单调前缀上做非增量操作"）精确对应。**作为主干推荐**。

### 4.5 方案适用矩阵

| | bi 拷贝 | BSP 扫描 | zhongshu 重扫 | 全局成本 | 信息增量 |
|---|:---:|:---:|:---:|:---:|:---:|
| 方案1 CoW | ✅ 命中 | ❌ | ❌ | 局部 | — |
| 方案2 持久结构 | ✅ | ⚠️ 遍历变慢 | ⚠️ 遍历变慢 | 高 | 负 |
| 方案3 事件溯源 | ❌ | ❌ | ❌ | 极高 | 零 |
| 方案4 SPC | ⚠️ 需配结构共享 | ✅ 命中 | ✅ 命中 | 中 | 正 |
| **推荐组合** | 方案1退化 | 方案4+二分 | 方案4 | 中 | 正 |

> 方案1-4 全部在 Python runtime 内。方案五（§4.6）是**正交维度**——换 runtime/语言，故不在本矩阵的列内（它横切全部三个瓶颈，改变的是常数因子而非复杂度阶数）。

### 4.6 方案五：用高性能语言重写引擎核心

**前提澄清（最关键，决定整个方案的定位）**：

> **语言重写与算法重构（方案1-4）正交，不是二选一。**
> - 算法重构（SPC）：改变**复杂度阶数**，O(N²) → O(N)。
> - 语言重写：改变**常数因子**，解释器开销 + 指针数组 cache miss → native + 连续内存，典型 10-50×。
> - 二者**叠加**：O(N²)×大常数 → 既可降阶（SPC）也可降常数（Rust）。

这引出一个真实的工程权衡，**必须由 L2 profiling 裁决，不可在测量前断言**：

- **路径 A（先算法后语言）**：在 Python 内做 SPC 拿到 O(N) → 若仍不达 30min 目标，再 Rust 重写降常数。
- **路径 B（直接语言重写）**：在 Rust 内 O(N²) 的常数极小，**可能在目标数据规模（640K bars）内"够用"**，无需 SPC 的复杂度改造即达标。
- **路径 C（语言内实现 SPC）**：直接在 Rust 里实现 SPC 算法，同时拿降阶 + 降常数。

哪条路径最优取决于"640K bars 上 Rust 的 O(N²) 常数"是否落在目标时间预算内——这是 L2 问题，本报告不臆测。

#### 4.6.1 逐位等价的硬约束（所有语言共同）

`bit-exact` 是用户的关键要求，也是跨语言移植最硬的约束。证据（§grep）：引擎全程 `np.float64`（IEEE 754 double），比较用 `abs(a-b) < 1e-9`（`bi_engine.py:460,525`）和 `round(x, 8)`（segment 检查点 key）。逐位等价要求：

1. **浮点类型一致**：目标语言用 IEEE 754 binary64（Rust `f64` / C++ `double` / Cython C `double` / Mojo `Float64`）——全部满足。
2. **关闭运算重排**：禁止 `-ffast-math`（C++）、禁止自动 SIMD 向量化改变约简顺序（Mojo 高危）。`max(a,b,c)`、`min(...)`、累加（如 `seg_high = highs[i0:i1+1].max()`）的关联顺序必须逐字复刻。
3. **`round(x, 8)` 语义**：Python 的 `round` 用 banker's rounding（round-half-to-even）。目标语言需复刻该舍入模式，不能用 C 默认的 round-half-away-from-zero。
4. **NumPy 约简顺序**：`a_stroke.py:248-249` 用 `np.ndarray.max()/min()` 在切片上做约简。NumPy 的成对约简（pairwise）顺序需在目标语言复刻，否则大数组约简的舍入误差可能在某些边界 case 翻转 `< 1e-9` 比较 → 分型/笔端点偏移 → 全链发散。**这是逐位等价的最大陷阱**，必须有逐位等价测试守护（对标 SegmentEngine `_update_checkpoint` 的 L0 逐位等价标注）。

#### 4.6.2 数据结构在各语言的自然表达

frozen dataclass + `slots=True` 的设计意图（紧凑内存）在 native 语言里天然实现为连续 struct 数组（vs Python 的指针数组 → cache 友好）。映射示例（`Stroke`）：

| Python | Rust | C++ | Cython | Mojo |
|--------|------|-----|--------|------|
| `@dataclass(frozen=True, slots=True)` | `#[derive(Clone, Copy)] struct` | `struct`（值语义） | `cdef class` / `cdef struct` | `@value struct` |
| `direction: Literal["up","down"]` | `enum Dir { Up, Down }` | `enum class Dir` | `int` 标志 | `enum`（演进中） |
| `i0,i1: int` | `i64` / `usize` | `int64_t` | `cdef long` | `Int64` |
| `high,low,p0,p1: float` | `f64` | `double` | `cdef double` | `Float64` |
| `confirmed: bool` | `bool` | `bool` | `cdef bint` | `Bool` |
| `list[Stroke]` | `Vec<Stroke>` | `std::vector<Stroke>` | `cdef Stroke[]` / memoryview | `List[Stroke]` |

bi/segment/zhongshu/move 全是同构的"frozen 记录 + 单向管线"，**无复杂共享可变状态**——这是对 Rust 借用检查器最友好的形态（管线每层拥有自己的 Vec，跨层传引用只读）。

#### 4.6.3 四语言逐项对比

**Rust + PyO3（推荐为语言重写首选）**

| 维度 | 评估 |
|------|------|
| 适用性 | 极高。frozen 记录 → `struct`，单向管线 → 所有权天然清晰，无 GC 暂停（实盘低延迟友好）。`Vec<Stroke>` 连续内存 cache 友好。 |
| 互操作 | 高。PyO3 `#[pyclass]` 暴露引擎，maturin 构建 wheel。Python 回测喂 bar、读快照/事件。FFI 边界清晰。 |
| 逐位等价 | 安全。`f64` = IEEE754 double，默认**不**做 fast-math/重排。需手工复刻 NumPy 约简顺序与 banker's rounding（可控）。 |
| 开发成本 | 高。所有权/借用学习曲线，但本引擎的值语义管线规避了大部分借用难点。 |
| 维护 | 中。独立 toolchain，但内存安全无 UB，重构有编译器护栏。 |
| 提速预期（L0 估计） | 10-50×（解释器消除 + cache 友好 + 边界检查可选关闭）。叠加 SPC 后复杂度阶数也降。 |

**C++ + pybind11**

| 维度 | 评估 |
|------|------|
| 适用性 | 高。struct + `std::vector` 自然，性能上限最高（无边界检查、激进优化）。 |
| 互操作 | 高。pybind11 成熟，STL 容器自动转换。 |
| 逐位等价 | **高危**。必须显式 `-fno-fast-math`，禁止 `-Ofast`；`-O2` 默认安全但需审计自动向量化对约简顺序的影响。 |
| 开发成本 | 中高。手动内存管理，悬垂指针/UB 风险（值语义 + RAII + 智能指针可大幅规避）。 |
| 维护 | 低-中。UB 调试成本高，无编译器内存护栏。 |
| 提速预期（L0 估计） | 10-50×，可能略高于 Rust（无边界检查）。 |

**Cython**

| 维度 | 评估 |
|------|------|
| 适用性 | 中。最小改动路径——现有 Python 逐步加 `cdef` 类型注解。但 dataclass / 复杂控制流加速有限，真正提速需 `cdef class` + 静态类型 + `@cython.boundscheck(False)`。 |
| 互操作 | 极高。本就是 Python 扩展，零 FFI 边界设计成本，回测脚本无感。 |
| 逐位等价 | 极易。沿用 CPython float 语义（或 C double），无重排风险，舍入与 Python 一致。 |
| 开发成本 | 低。与 Python 共生，最低学习成本，可增量迁移（先热点函数）。 |
| 维护 | 低。与现有代码同仓同语言谱系。 |
| 提速预期（L0 估计） | 2-10×（取决于类型注解深度）。**关键：Cython 是常数优化，不改复杂度——O(N²) 在 Cython 里仍是 O(N²)，640K bars 上仍会被 N 增长吃掉。** 必须与 SPC 算法改造叠加才能根治。 |

**Mojo**

| 维度 | 评估 |
|------|------|
| 适用性 | 理论高（Python 超集 + native 编译），实际**生态风险高**。2026 年 Mojo 仍在演进，dataclass/dict 等高层特性支持不完整，向后兼容无保证。 |
| 互操作 | 理论无缝（可 import Python），但**反向暴露给 Python 回测的成熟度不足**。 |
| 逐位等价 | **最高危**。Mojo 的卖点是自动 SIMD 向量化，默认可能重排浮点约简顺序 → 破坏 `< 1e-9` 边界等价。需显式禁止向量化，丧失其主要优势。 |
| 开发成本 | 中（语法近 Python），但生态不全导致隐性成本高。 |
| 维护 | **高（赌注语言）**。语言不稳定，生产依赖风险大。 |
| 提速预期（L0 估计） | 宣称极高，但本引擎是**标量分支逻辑**（非数值密集 kernel），SIMD 收益有限，可能不及宣传。 |

#### 4.6.4 移植边界与 FFI 设计

**移植闭合子图，不做部分移植**：bi→segment→zhongshu→move→bsp + 递归栈（RecursiveStack level≥2）+ 事件 diff + PH persistence 应**整体**移植为一个 native 模块。部分移植（只移 bi）会在每层 FFI 边界产生 Python↔native 的对象转换开销，可能抵消收益。

**保持 Python 的外围**（用户要求）：回测 FSM（`cost_reduction_fsm`）、K4 选股管线、分析脚本、TradingView/IBKR 数据接入、可视化——这些非热点、迭代频繁、生态依赖重（pandas/numpy/matplotlib），留在 Python。

**FFI 边界设计（决定实际收益）**：
- 引擎在 native 侧持有状态。Python 每 bar 调用 `engine.process_bar(bar) -> handle`。
- **快照惰性拉取**：快照对象大，每 bar 跨界序列化整个快照会成为新瓶颈。回测场景批量喂 bar，只在检查点/结束/信号触发时拉快照。事件流（小、增量）可每 bar 跨界。
- 这与 §4.3 的发现一致：事件双轨天然适配 native 引擎——native 侧产生事件，Python 侧消费事件做策略；只在需要完整状态时才拉快照。

#### 4.6.5 语言重写小结

| 语言 | 逐位等价风险 | 提速(L0) | 改复杂度? | 互操作成本 | 维护风险 | 综合 |
|------|:---:|:---:|:---:|:---:|:---:|------|
| **Rust+PyO3** | 低 | 10-50× | 否(需配SPC) | 中 | 低 | **首选** |
| C++ pybind11 | 高 | 10-50×+ | 否 | 中 | 中(UB) | 次选 |
| Cython | 极低 | 2-10× | **否(纯常数)** | 极低 | 低 | 过渡/止血 |
| Mojo | 极高 | 不确定 | 否 | 不成熟 | 高 | 否决(生态赌注) |

**判断**：
- 若追求"最小风险快速止血"且接受常数级收益 → **Cython** 给热点函数加类型，但**必须明确它不改 O(N²)**，须叠加 SPC。
- 若追求"根治 + 实盘低延迟 + 长期维护安全" → **Rust+PyO3** 重写闭合引擎子图（路径 C：Rust 内实现 SPC，一次拿降阶 + 降常数）。
- C++ 仅在已有 C++ 团队/依赖时考虑（逐位等价审计成本 + UB 风险不划算）。
- Mojo 在其生态成熟（dataclass 完整 + 稳定 ABI + 可控向量化）前不纳入生产路径。

---

## 5. 推荐方案：SPC 统一化 + 受控结构共享（分而治之）

> **算法主干 + 语言放大器的双层结构。** §5.1-5.4 是算法主干（降复杂度阶，runtime 无关）。§5.5 是语言放大器（降常数，正交叠加）。两层独立有效、可叠加施加。施工顺序由 §6 路线图的 P0 profiling 裁决。

### 5.1 bi 瓶颈 → 前缀只读共享 + 尾部 CoW

**设计**：`BiEngineSnapshot.strokes` 返回的复合视图 = `(immutable_prefix, mutable_tail)`。
- `immutable_prefix`：指向 `_cp_strokes[:base_len]` 的**只读共享**（不拷贝）。
- `mutable_tail`：当前 bar 的尾部笔（短，O(1)）。
- 下游遍历透明（实现 `__getitem__`/`__len__`/`__iter__`）。
- `_extend_prev_stroke` 的尾部替换发生在 `mutable_tail`，不触碰已共享的前缀。

**消除**：`list(cp[:base_len])` 的 O(N) 前缀拷贝 → O(1) 引用 + O(尾部) 追加。

**不变量依据**：bi 确认前缀单调不变（`bi_engine.py:375-382` 已证）。

**认识论等级**：设计为 L0（前缀单调推导）。落地需 L1 逐位等价测试（对标 SegmentEngine `_update_checkpoint` 的 L0 标注做法）。

### 5.2 zhongshu 瓶颈 → 锚定 segment stable 前缀的 resume

**设计**：ZhongshuEngine 增加检查点：
- 从 SegmentEngine 暴露 `stable_count`（当前稳定 confirmed 段数）。
- zhongshu 缓存"扫描到的最后一个**完全落在 stable 前缀内的已 settled 中枢**"的续进锚点 `i_resume`（取该中枢的 `max(break_seg-2, seg_end)`）。
- 每 bar：从 `i_resume` 续扫尾部；stable 前缀内的中枢直接复用缓存。

**消除**：`_scan_zhongshu` 从 O(N²) 全量重扫降为 O(stable 前缀复用 + 尾部重扫)，尾部长度不随 N 累积。

**必须验证的实现约束（严格性，不可假设）**：
1. `zhongshu_from_segments` 过滤 `confirmed AND kind=="settled"`；SegmentEngine 的 stable 前缀过滤 `confirmed AND break_evidence 窗口展开`。**两个过滤集合的对齐关系必须证明**——stable 前缀内的段是否一定满足 `kind=="settled"`？若不一定，需取两者交集作为 zhongshu 检查点锚定基准。**此点未经验证前不得假设成立**（否则是声明膨胀）。
2. 续进锚点回退 2 段（`break_seg_idx - 2`）必须证明落在 stable 前缀内，否则锚点指向易变区。

**认识论等级**：设计为 L0（依赖 SegmentEngine 已证的 stable 单调不变量）。但上述两个约束属于**待验证的 L0 前提**——在测试证明前，本方案的有效性是**假设性的**。

### 5.3 BSP 瓶颈 → 二分 + 末尾游标缓存

**设计**：
- `_find_move_for_seg` 利用 `Move.seg_start` 严格单调 → 二分查找 O(log N_moves)。
- 末尾未 settled Move 的 `seg_end` 可变但单调非递减 → 缓存上次查询的 `(seg_idx, move)`，O(1) 验证命中。

**消除**：per-call O(N_moves) → O(log N_moves) 或 O(1) 摊还。

**优先级保留**：§3.2 已指出——该瓶颈的绝对收益取决于 Move 列表规模与 `settled` 字段消费频率，**需 L2 profiling 确认其是否为主导项**。若 profiling 显示 Move 列表恒小（< 100），此项可降级为低优先级。**不在测量前断言其收益排序。**

### 5.4 容器不可变性的类型级加固（横切，可选）

当前"不可变"靠注释 + 审查维持（§1.2）。可选加固：快照容器字段改为 `tuple`（元素已 frozen）。
- 收益：类型级强制只读，消除 §2.2 的理论 mutate 风险。
- 成本：尾部追加需构造新 tuple（与结构共享方案协同时，仅尾部，O(尾部)）。
- **判断**：这是"声明与实际一致"的加固（把"约定只读"升级为"强制只读"），但非性能必需。建议与 §5.1 的复合视图一起做（复合视图本就需要自定义容器，顺带 freeze 容器层）。

### 5.5 语言放大器定位（正交于 5.1-5.4）

- **首选 Rust + PyO3**：把 bi→segment→zhongshu→move→bsp + 递归栈整体移植为闭合 native 模块，Python 保留外围（回测 FSM / K4 管线 / 分析 / 数据接入 / 可视化）。
- **施加方式取决于 P0 profiling**（§4.6 路径 A/B/C）：
  - 若 SPC（5.1-5.3）后 Python 已达 30min 目标 → 语言重写**非必需**，仅在实盘低延迟需求出现时再做。
  - 若 SPC 后仍不达标 → Rust 重写降常数（路径 A），或直接 Rust 内实现 SPC（路径 C，一次拿降阶+降常数）。
- **Cython 作为可选止血**：若需快速常数收益且不立即上 Rust，可给 §3 三个热点函数加 `cdef` 类型——但**必须叠加 SPC**，因 Cython 不改 O(N²) 阶数。
- **逐位等价是所有语言路径的准入门槛**（§4.6.1）：移植任何函数前先建立逐位等价测试（对全历史快照逐字段比对 native vs Python），无测试不得合并。

> 局部依赖原则（275号）：每阶段只依赖直接上游的 stable 前缀，无全局排序。

| 阶段 | 内容 | 依赖 | 验证门槛 |
|------|------|------|---------|
| P0 | **L2 profiling 复核**：在 640K bars 真实数据上测量三瓶颈的实际 wall-clock 占比，确认优先级排序 | 无 | L2 实测数据（否则后续优先级是臆测） |
| P1 | zhongshu SPC：先验证 §5.2 的两个实现约束（过滤集对齐 + 回退区） | SegmentEngine `stable_count` 暴露 | L1 逐位等价（对全历史 zhongshu 列表逐字段比对 resume vs 全量） |
| P2 | bi 前缀共享 + 尾部 CoW | 无（独立） | L1 逐位等价 + 下游遍历兼容性测试 |
| P3 | BSP 二分 + 游标缓存 | P0 确认其为主导项 | L1 逐位等价 |
| P4 | 容器 tuple 加固（可选） | P1/P2 完成 | 回归测试全绿 |
| **P5（可选轨道，正交）** | **语言放大器**：Rust+PyO3 重写闭合引擎子图（或 Cython 止血热点） | P0 裁决是否需要；若走路径 C 则可与 P1-P3 并行（Rust 内实现 SPC） | **逐位等价测试**（native vs Python 全历史快照逐字段比对）+ L2 wall-clock |

**轨道关系**：P1-P3 是算法主干（Python 内降阶），P5 是语言放大器（降常数）。二者正交：
- 路径 A = 先 P1-P3，不达标再 P5（降风险，增量验证）。
- 路径 C = P5 直接在 Rust 内实现 SPC，吸收 P1-P3（一次到位，但风险集中）。
- P0 的 L2 数据决定走哪条——Rust 的 O(N²) 常数是否在 640K bars 内"够用"是关键裁决点。

**反模式警告**：
- 不得在 P0 之前按本报告的"~8.5s/~13s/~7s"估值排序施工——这些是任务描述给出的估值，本报告未独立测量。P0 的 L2 profiling 是后续一切优先级的前提。
- 不得把"换语言"当作"修算法"的替代品来回避 §5.2 的 zhongshu 不变量分析——Cython/Rust 降常数不消除 O(N²) 阶（除非在 native 内显式实现 SPC，即路径 C）。把"用 Rust 重写"当成"不用想清楚 confirmed≠stable"的借口 = 补丁思维换皮。

---

## 7. 结果包六要素

### 1. 结论
三个剩余 O(N²) 瓶颈的障碍本质不同，应分而治之：bi 用前缀只读共享 + 尾部 CoW（方案1 退化形式），zhongshu 用锚定 segment stable 前缀的 resume（方案4），BSP 用二分 + 游标缓存（方案4 + 算法修正）。否决方案2（访问模式相克）与方案3 作为主干（高成本零信息增量）。主干机制是把 SegmentEngine 已验证的"稳定前缀检查点"推广到 zhongshu/BSP。

### 2. 定义依据
- **稳定前缀单调不变量**：`segment_engine.py:116-180` `_update_checkpoint`——区分 confirmed（会回退）与 stable（一旦稳定永远稳定，`:123-128` 证明稳定条件随 n_strokes 单调）。
- **zhongshu 过滤定义**：`a_zhongshu_v1.py:177` `confirmed AND kind=="settled"`。
- **Move.seg_start 单调**：`a_move_v1.py:159-162`（仅处理 settled 中枢，seg_start 由 Zhongshu.seg_start 单调性保证）。
- **bi 前缀单调**：`bi_engine.py:375-382`（确认前缀不被本循环触及）。

### 3. 边界条件（结论翻转条件）
- 若 §5.2 的实现约束（stable 前缀段 ⊆ `kind=="settled"` 段，且回退 2 段落在 stable 前缀内）**经测试证伪** → zhongshu SPC 方案失效，需退回全量重扫或重新设计锚点。
- 若 P0 profiling 显示 Move 列表恒小（< 100）→ BSP 二分收益微小，§5.3 降级为低优先级。
- 若下游存在未枚举的快照容器 mutate（§2.2 的"全部纯读"被证伪）→ 前缀共享方案需加防护，方案2/方案4 的引用复用假设动摇。
- 若实盘路径成为主导用例（非批量回测）→ 方案3 事件溯源的成本收益反转，应重新评估。
- 若 P0 profiling 显示 Rust 的 O(N²) 常数在 640K bars 内已达 30min 目标 → 语言重写路径 B 成立，SPC 算法改造（5.2/5.3）可降级为非必需（但实现复杂度的根因仍未消除，超出 640K 规模会再次暴露）。
- 若逐位等价测试证明 NumPy 约简顺序/banker's rounding 无法在目标语言复刻（§4.6.1 陷阱）→ 该语言路径失效，须退回 Python 或接受非逐位等价（违反用户硬约束，需上浮）。

### 4. 下游推论
- 若推荐成立，则"快照可变性 / 引用安全"不是性能改造的主线（§2.3），后续优化应聚焦"单调前缀上的增量化"而非"数据结构革命"。
- SegmentEngine 的 `_cp_stable_count` 从单点优化升格为**可复用的架构模式**（SPC），应抽象为各层共享的检查点协议。
- 事件流（DomainEvent 双轨）的存在使实盘增量路径与批量回测随机访问路径**天然分离**——两条路径应保持双轨，不应为统一接口牺牲任一方。
- 若走语言重写（§4.6），事件双轨天然适配 native 引擎：native 侧产生增量事件（小、每 bar 跨 FFI），Python 侧消费事件做策略，仅在需要完整状态时惰性拉快照——FFI 边界设计应围绕"事件常跨界、快照惰性拉"。
- frozen dataclass + `slots=True` 的既有设计意图（紧凑内存）在 native 语言中升格为连续 struct 数组（cache 友好），说明原作者的数据建模已为 native 移植预留了结构友好性——移植不需要重新设计数据模型，只需逐字复刻语义。

### 5. 谱系引用
- 本报告涉及性能-架构层，非缠论概念定义层，无直接的缠论概念分离谱系。
- 相关工程谱系：SegmentEngine `_update_checkpoint` 注释自标 "L0（单调不变量推导，逐位等价）"——本报告的 SPC 推广沿用该认识论标注范式。
- 与 `project_engine_incremental_optimization`（记忆）一致：已修 6 个 O(N²) 点 bit-exact，剩余 bi-rebuild/BSP/zhongshu-scan，本报告为这三者提供设计路径。
- 与 `project_ph_perf_streaming_oN2`（记忆）一致：真瓶颈 = 流式每 bar 全量重算 O(N²)，本报告诊断三处全量重算的具体障碍。

### 6. 影响声明
- 本产出为研究报告，**未改动任何代码**，仅写入 `docs/architecture/engine_architecture_research.md`。
- 若按算法主干（§5.1-5.4）施工，将影响：`bi_engine.py`（快照容器结构）、`zhongshu_engine.py`（新增检查点）、`segment_engine.py`（暴露 stable_count）、`a_buysellpoint_v1.py`（`_find_move_for_seg` 算法）、及所有遍历这些快照的下游消费者（需兼容复合视图/tuple）。
- 若按语言放大器（§5.5）施工，将新增：独立的 Rust crate（或 Cython `.pyx` 模块）+ PyO3/maturin 构建链 + CI 中的 native 编译步骤 + 逐位等价测试套件；`orchestrator/recursive.py` 入口改为调用 native 引擎；外围（回测 FSM / K4 / 分析 / 数据 / 可视化）不变。引入新 toolchain 依赖（Rust/C++ 编译器、maturin）。
- 不影响缠论定义、不改变任何快照的语义内容（要求逐位等价），仅改变计算路径与（可选的）执行 runtime。

---

## 8. 认识论等级声明（formalization-validity-domain 规则）

| 主张 | 等级 | 说明 |
|------|------|------|
| 三瓶颈复杂度为 O(N²) | L0 | 算法结构推导（外层×内层、累积拷贝） |
| 推荐方案"消除 O(N²)" | **L0（设计断言）** | 复杂度推导，**未实测**。落地后需 L1（逐位等价）+ L2（wall-clock） |
| "~8.5s/~13s/~7s" 瓶颈占比 | **未独立验证** | 引自任务描述，本报告未测量。P0 需 L2 复核 |
| zhongshu SPC 的两个实现约束 | **待验证 L0 前提** | 依赖未证明的集合对齐关系，测试前为假设 |
| 方案2 遍历惩罚 | L0 | PVector O(log₃₂N) 访问的结构性质 |
| 语言重写提速 10-50×（Rust/C++） | **L0 估计** | 行业经验区间，**未在本引擎实测**。实际倍数取决于 cache 行为/分支预测/FFI 开销，需 L2 |
| Cython 提速 2-10×、不改阶数 | L0 | 常数优化的结构性质（Cython 不改算法复杂度） |
| Mojo 提速"宣称极高" | **未验证（厂商宣称）** | 本引擎为标量分支逻辑，SIMD 收益存疑，无独立证据 |
| 逐位等价可达性（IEEE754 double） | L0（必要条件） | 类型层成立；但 NumPy 约简顺序/banker's rounding 的可复刻性是**待验证 L1 前提** |

**有效域声明**：本报告的结论是**设计层有效**（L0 推导一致）。其**经验有效域**（真实数据上确实消除 O(N²) 且无回归、语言重写确实提速且逐位等价）尚未建立——需 P0-P5 的 L1/L2 验证逐步确认。在 L2 验证前，不得声称任何方案"已消除瓶颈"或"提速 N 倍"；在逐位等价测试通过前，不得声称任何 native 移植"逻辑等价于 Python 版"。
