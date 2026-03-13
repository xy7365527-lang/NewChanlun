# Codex diagnose — 2026-03-13 06:09:55 UTC

## 元数据

- **mode**: diagnose
- **subject**: 复合能指结晶实装可行性——穿越路径反复相邻节点对拼接为新能指节点写入S_net和区块拓扑
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-13 06:09:55 UTC
- **context-file**: C:/Users/hanju/AppData/Local/Temp/codex-diagnose-ctx.md

## Prompt

## 诊断目标

复合能指结晶实装可行性——穿越路径反复相邻节点对拼接为新能指节点写入S_net和区块拓扑

## 上下文

# Codex Diagnose Context: 复合能指结晶实装可行性

## 任务描述

编排者提议：S_net 新增结晶操作——当逢亮的穿越在环路闭合中反复经过同一组相邻节点时，
将这组节点拼接为新的复合能指（compound signifier），作为新节点写入 S_net，
同时作为 immutable block 写入区块拓扑物质层。

诊断目标：评估实装可行性，发现实现中的根本矛盾和风险点。

---

## 现有架构概览

### 1. `_is_locally_crystallized`（daemon.py:942）

```python
def _is_locally_crystallized(self, window: int = 50) -> bool:
    """Check if beta_1 has been stable over the last `window` steps."""
    if len(self._beta_1_history) < window:
        return False
    recent = self._beta_1_history[-window:]
    return all(v == recent[0] for v in recent)
```

判据：beta_1（第一 Betti 数）在最近 50 步内保持不变。
这是概念层环路结构的宏观稳定性——不是物质层路径的重复模式。

### 2. `_compute_f`（traversal.py:165）

```python
def _compute_f(self, v: str, w: str) -> int:
    """Compute f(v,w) = (c-1) + n_loop for terrain annotation."""
    # lower_link: 与 v,w 相邻的其他顶点集合（概念层）
    # c: lower_link 的连通分量数
    # n_loop: v-w 之间的概念层边数
    return (c - 1) + n_loop
```

f=0 → 两顶点拓扑等价（fold 候选）
f=-1 → 无共享邻居（拓扑孤立）
f>0 → 拓扑张力（negate 候选）

### 3. `Graph.add_vertex` / `Graph.add_edge`（engine.py:224, 239）

Graph 是不可变的，add_vertex/add_edge 返回新 Graph 实例。
这个接口已经存在，支持新节点写入。

### 4. `SNet.add_signifier`（signifier_net.py:236）

```python
def add_signifier(self, sig: Signifier) -> "SNet":
    """添加能指。若 id 已存在则覆盖。"""
    new_sigs = dict(self._signifiers)
    new_sigs[sig.id] = sig
    return SNet(new_sigs, self._edges, self._morphemes)
```

Signifier 数据结构：
```python
@dataclass(frozen=True, slots=True)
class Signifier:
    id: str                           # 规范形式
    surface_forms: tuple[str, ...] = ()  # 表层变体
    source: str = "k_active_projection"  # 来源
    lang: str = ""
    domain: str = ""
    # !! 没有 passage 字段 !! （能指层无语料内容）
```

### 5. `SNetActivation.check_articulation_feedback`（snet_activation.py:520）

`ConceptCreationSuggestion` 机制：
- 触发条件：共振激活一个**无 concept_ref** 的孤立 signifier，且该 signifier 与有 concept_ref 的 signifier 有组合轴连接
- 产出：ConceptCreationSuggestion，记录 orphan_signifier + anchor_concept
- 这是"概念创建建议"，不是自动创建——等待 operator 裁决

### 6. 区块拓扑写入（block_topology_persistence.py）

现有 block 类型：
- `vertex` — K_active 概念节点
- `edge` — 概念边
- `merge` — fold 合并
- `settlement` — 环路结算
- `articulation` — 物质层→概念层涌现（ARTICULATED 边）
- `event` — 穿越事件（traversal_association 等）

articulation block 的核心字段：
```
event_type: "articulation"
source_vid, target_vid: 概念节点 ID
score: 拓扑不一致得分
imbalance_type: "A密B疏" 或 "B密A疏"
```

### 7. `inject_settlement_memory_node`（encounter_log.py:233）

新节点创建模式（settlement memory）：
```python
vertex = Vertex(id=vid, status=VertexStatus.ACTIVE, content=content, created_at=step)
result = graph.add_vertex(vertex)
# 加 REFERENCE 边连接到涉及的概念
```

### 8. `_step` 中的结晶跳转逻辑（daemon.py:1119）

```python
if self._is_locally_crystallized():
    self._crystallization_count += 1
    # ... 保存结算历史 ...
    # ... 间隙检测 ...
    self._inject_cross_domain_incremental()
    target = self._find_most_unstable_region()  # 跳到最不稳定区域
    self._beta_1_history.clear()  # 清除历史，下一轮重新计时
```

### 9. 穿越路径（visit_history）

daemon.py 中 `self.engine.visit_history` 是穿越的顶点序列（概念层顶点 ID 列表）。
S_net 层面没有独立的"能指路径"——穿越路径通过 concept → signifier 映射间接对应。

---

## 五个核心问题（需要诊断）

### Q1: 环路闭合检测的粒度差异

`_is_locally_crystallized()` 检测的是 beta_1 宏观稳定性（概念层环路总数不变）。
复合能指结晶需要的是"物质层路径中反复出现的相邻节点对"——这是完全不同的粒度。

问题：能否复用 settlement 环路检测？还是需要新的检测机制？

### Q2: 新 Signifier 的数据完整性

Signifier 没有 passage 字段。复合能指的"内容"是什么？
如果是 "signifier_A+signifier_B" 的字符串拼接，与 concept_creation_suggestion 的 orphan_signifier 有何区别？

### Q3: 共现边的继承规则

新复合节点出现后，与其他能指的组合轴边如何建立？
继承 A 和 B 各自的边的并集？还是需要重新计算 PMI？
后者依赖原始语料，在运行时不可用。

### Q4: concept_creation_suggestion 的关系

ConceptCreationSuggestion：孤立 signifier + 有概念节点的 signifier 有组合轴连接 → 建议创建概念
复合能指结晶：两个（可能都有 concept_ref 的）signifier 的路径重复 → 拼接为新 signifier

这两个机制在触发条件和语义层级上都不同，不是同一件事的两个名字。

### Q5: block 类型的选择

新复合能指应该写入什么类型的 block？
现有 ARTICULATION block 是"物质层积累→概念层连接"。
复合能指结晶是"物质层路径→能指层新节点"，语义层级不同。

---

## 被审查的核心实现设想（伪代码）

```python
# 设想的实现（未写入代码）
def _detect_compound_signifier_candidate(path: list[str]) -> tuple[str, str] | None:
    """从穿越路径中检测反复相邻的能指对."""
    # 通过 concept → signifier 映射，将 visit_history 转换为能指序列
    sig_path = [concept_to_sig.get(vid) for vid in path if concept_to_sig.get(vid)]
    # 滑动窗口统计相邻对出现次数
    pair_counts: dict[tuple[str,str], int] = {}
    for i in range(len(sig_path) - 1):
        pair = (sig_path[i], sig_path[i+1])
        pair_counts[pair] = pair_counts.get(pair, 0) + 1
    # 触发阈值？拓扑判据？
    ...

def _crystallize_compound_signifier(pair: tuple[str, str], s_net: SNet, step: int) -> SNet:
    """将能指对合并为新复合能指."""
    new_id = f"{pair[0]}+{pair[1]}"
    new_sig = Signifier(id=new_id, source="crystallization", ...)
    new_s_net = s_net.add_signifier(new_sig)
    # 边继承：取并集
    # ...
    return new_s_net
```

---

## 诊断要求

请诊断：
1. 环路闭合检测的粒度问题——beta_1 稳定性是否足够，还是必须引入独立的路径重复计数？
2. "拓扑闭合判据（不是频率阈值）"的具体含义——什么拓扑不变量可以用？f 值？连通分量？
3. 新节点写入 S_net 的语义完整性——没有 passage 的能指节点对穿越有什么意义？
4. concept_creation_suggestion vs 复合能指结晶的根本区别，以及是否存在实现冲突
5. block 类型设计——是新增类型还是复用现有类型？
6. 编排者提案中是否有根本性的架构矛盾需要上浮？

输出格式：
- 每个问题的诊断结论（可行/需要额外工作/根本矛盾）
- 如果可行：具体代码修改点
- 发现的根本矛盾或需要决策的选择点


## 诊断要求

请执行严格根因诊断：
1. 失败现象：观测到什么？
2. 直接原因：哪行代码导致了失败？
3. 根本原因：为什么会写出那行代码？是设计错误还是实现疏忽？
4. 分类判定：实现错误 or 定义冲突？
5. 修复方案（仅限实现错误）：精确的修复步骤

如果是定义冲突，输出：
- **冲突方**：A 说什么 vs B 说什么
- **不可弥合理由**：为什么不能同时满足两边


## Response

下面给出严格诊断报告（把“提案与现架构不一致”视为失败对象）。

## 总览结论

1. **环路闭合检测粒度**：**需要额外工作**（不能复用 `_is_locally_crystallized` 充当该功能）
2. **“拓扑闭合判据”定义**：**需要额外工作**（当前实现只有频次计数设想，缺闭合拓扑判据）
3. **新 Signifier 语义完整性**：**根本矛盾（定义冲突）**（若要求承载 passage/立即影响穿越）
4. **与 ConceptCreationSuggestion 关系**：**可行，但有实现错误风险**
5. **block 类型选择**：**复用 articulation 属定义冲突**；采用新事件语义则可行
6. **需上浮的架构矛盾**：存在 2 个定义冲突 + 2 个实现错误

---

## 1) 环路闭合检测粒度问题

### 失败现象
“反复相邻节点对”可能已出现，但系统不会稳定检测；或在无相邻对重复时误触发结晶流程。

### 直接原因
`daemon.py:942` 的 `_is_locally_crystallized()` 仅检查 `beta_1_history` 常值：
```python
recent = self._beta_1_history[-window:]
return all(v == recent[0] for v in recent)
```
它完全不读取 `visit_history` 的相邻对模式。

### 根本原因
把“概念层宏观环路稳定（β1）”误当成“物质层路径局部重复闭合”的判据。

### 分类判定
**实现错误（机制误复用）**

### 修复方案
- 在 daemon 新增独立检测器（不能替代 `_is_locally_crystallized`，应并行）：
  - 输入：`engine.visit_history`（窗口化）
  - 映射：concept→signifier 序列
  - 输出：compound candidate + 证据
- 在 `_step` 中于跳转清理前执行该检测，不依赖 β1 常值作为充分条件。

---

## 2) “拓扑闭合判据（不是频率阈值）”应是什么

### 失败现象
伪代码只做 `pair_counts`，会把“高频但非闭合路径”误判为结晶候选。

### 直接原因
伪代码核心仅统计：
```python
pair_counts[pair] += 1
```
没有任何“闭合”拓扑条件。

### 根本原因
规范里未给出“闭合”的拓扑不变量定义，导致实现退化为频次规则。

### 分类判定
**实现错误（规范缺失导致实现不可判定）**

### 修复方案
给出可执行拓扑判据（非阈值主导）：
- 在窗口内构建 signifier 转移图 `G_w`
- 候选边 `(a,b)` 必须满足：
  1) `b` 可达 `a`（即 `(a,b)` 位于有向环内，SCC 条件）
  2) 候选所在连通子图满足 `β1_sub > 0`（有真实回路）
- 频次仅作“证据强度”而非闭合定义本身。

---

## 3) 新 Signifier 语义完整性（无 passage）

### 失败现象
即便写入了复合 signifier，新节点也不天然进入穿越主循环；“路径内容”也无法存入 Signifier。

### 直接原因
- `Signifier` 结构无 `passage` 字段（你已明确）
- 穿越历史是概念顶点 `visit_history`，非 signifier 路径主状态

### 根本原因
当前定义中：**Signifier 是形式节点，不是语料容器；穿越主引擎在概念层运行。**

### 分类判定
**定义冲突**

- **冲突方**
  - A：`Signifier` 仅形式信息（id/surface/source...），不承载 passage；穿越在概念层
  - B：复合能指应由路径内容结晶，并写入后直接参与穿越语义
- **不可弥合理由**
  - 不改定义时，Signifier 无法承载所要求的“路径内容语义”，也不会直接成为 traversal 顶点

---

## 4) ConceptCreationSuggestion vs 复合能指结晶

### 失败现象
复合节点若无 `concept_ref` 且连到有概念节点，可能被 `check_articulation_feedback` 连续触发建议，造成建议洪泛。

### 直接原因
`snet_activation.py:520` 机制按“orphan + 组合轴连接 anchor”触发；复合节点天然可能满足。

### 根本原因
两个机制语义不同，但当前未做 source/provenance 隔离。

### 分类判定
**实现错误（机制耦合缺隔离）**

### 修复方案
- 在 `check_articulation_feedback` 增加来源门控：`source=="crystallization"` 默认不进入建议，或进入单独待审队列
- 对同一 `(A,B)` 去重 + 冷却窗口
- 操作员批准后再开放进入 concept creation 建议链。

---

## 5) block 类型选择

### 失败现象
若复用 `articulation` block，会出现字段语义不匹配（`source_vid/target_vid/imbalance` 都是概念层语义）。

### 直接原因
`articulation` 被定义为“物质层→概念层连接事件”，不是“能指层新节点结晶”。

### 根本原因
事件语义层级不同。

### 分类判定
**定义冲突（仅针对“复用 articulation”这一路径）**

- **冲突方**
  - A：`articulation` = 概念节点间的涌现连接
  - B：复合能指结晶 = 新 signifier 生成事件
- **不可弥合理由**
  - 同一 block 语义与字段约束无法同时表达两类事件而不失真

---

## 6) 需要上浮的根本矛盾/风险点

### 6.1 ID 覆盖破坏“结晶不可变记录”
- **失败现象**：新复合 id 冲突时旧 signifier 被静默覆盖
- **直接原因**：`signifier_net.py:236` `new_sigs[sig.id] = sig`
- **根本原因**：`add_signifier` 是 upsert 语义，不是 append-only
- **分类**：实现错误
- **修复方案**：结晶路径改用“禁止覆盖”接口（冲突即拒绝或版本化 id）

### 6.2 concept→signifier 映射基数未定义
- **失败现象**：伪代码假设单值映射，可能产生歧义或信息丢失
- **直接原因**：`concept_to_sig.get(vid)` 单值读取
- **根本原因**：架构未定义该映射是 1:1 / 1:N / N:1
- **分类**：定义冲突
- **冲突方**：A 要确定性路径结晶；B 映射基数未定
- **不可弥合理由**：无基数定义就无确定的能指路径

---

## 仅实现错误项的可执行改动点（可落地）

1. `daemon.py`
   - 新增 `_detect_compound_from_visit_history(window)`（独立于 `_is_locally_crystallized`）
   - 在 `_step` 中于 `clear()/jump` 前调用

2. `snet_activation.py`
   - `check_articulation_feedback` 增加 `source/provenance` 门控与去重冷却

3. `signifier_net.py`
   - 新增 strict add（禁止同 id 覆盖）供结晶路径使用

4. `block_topology_persistence.py`
   - 不复用 articulation 语义；为结晶事件单列事件语义（至少 event_type 区分，字段独立）

---

如果你要，我可以下一步直接给出**最小改动 patch 设计**（函数签名级别），只覆盖“实现错误”部分，完全不触碰定义冲突部分。
