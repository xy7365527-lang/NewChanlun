# phi_L 自我进化机制设计文档

版本：v0.1（设计草稿）
日期：2026-03-07
状态：生成态（等待穿越验证）

---

## 问题陈述

phi_L 当前实现的核心缺陷：**signifier 合并错误**。

相同词形（signifier）= 相同哈希 = 相同顶点 ID。

结果：不同理论传统中的同名概念被强制合并为一个顶点，导致拓扑结构失真。

示例：
- `c_<sha("consciousness")>` 同时承载黑格尔的意识（自我认识的绝对精神阶段）和马克思的意识（被生产关系决定的社会意识）
- 两者在原始理论空间中有完全不同的邻域结构，合并后邻域交叉，cycle 检测产生虚假 settlement

当前的"粗粒度修复"（`_disambiguated.add("*")` — 全局标记所有概念需要区分）是补丁思维，不能解决问题。

---

## 设计目标

1. **精细区分规则**：不是"所有概念都区分"，而是"概念 X 在语境 A 和 B 中必须区分"
2. **规则来自穿越积累**：区分规则不是外部给定的，而是从 settlement_history 中提取
3. **带语境的顶点 ID**：phi_L 在提取概念时查询规则，决定是否附加语境前缀
4. **渐进过渡**：不需要重建整个图，规则在穿越中逐步生效
5. **bootstrapping**：第一个 phi_L 是外部给定的（当前实现），后续通过穿越积累自我修订

---

## 1. 从 settlement_history 提取区分规则

### 1.1 何时产生区分规则

settlement_history 中的每个 `settle` 记录包含一个 cycle（顶点对的环路）。当一个 cycle 的两个顶点满足以下条件时，产生区分规则：

**条件**：
- 两个顶点共享相同的 signifier（词形相同，当前哈希相同）
- 但它们的来源文本属于不同的理论语境（不同来源文件、不同作者、不同时期）
- 且这个 cycle 在跨语境边（来源 A 的顶点 → 来源 B 的顶点）上 settle

**信号**：跨语境的 NEGATION 边频繁出现在 settlement cycle 中 = 系统在告诉自己"这两个位置上的概念是对立的，不应该是同一个节点"。

### 1.2 区分规则提取算法

```
对每个 settlement 记录：
  1. 取 cycle 中所有边
  2. 找出跨语境边（source_vertex 和 target_vertex 来自不同来源文件）
  3. 对跨语境边上的顶点对：
     a. 如果两个顶点 content 相同（signifier 相同）
     b. 提取来源语境（来源文件名 / 作者 / 理论域标记）
     c. 生成区分规则：
        {"signifier": "consciousness", "contexts": ["hegel", "marx"], "rule_type": "require_separation"}
  4. 将规则追加到 distinction_rules.jsonl
```

### 1.3 区分规则格式

```json
// distinction_rules.jsonl — append-only，每行一条规则
{
  "rule_id": "dr_<sha(signifier+contexts)[:8]>",
  "created_at": 1741234567.89,
  "created_from_settlement_step": 142,
  "signifier": "consciousness",
  "contexts": ["hegel", "marx"],
  "rule_type": "require_separation",
  "evidence": {
    "cycle_edges": [["c_abc123", "c_abc123"], ["c_abc123", "c_def456"]],
    "edge_types": ["NEGATION", "DEPENDENCY"],
    "settlement_count": 3
  },
  "confidence": 0.85,
  "status": "active"
}
```

**字段说明**：
- `rule_type`: 当前只有 `require_separation`，后续可扩展 `require_merge`（发现错误分离）
- `confidence`: 同一 signifier 跨语境 settle 的次数 / 总 settlement 次数（频率信号）
- `status`: `active`（生效）/ `pending`（积累中，confidence 未过阈值）/ `superseded`（被更新版本覆盖）

---

## 2. 带语境的顶点 ID 生成

### 2.1 语境检测

phi_L 在处理文本时，需要知道当前文本属于哪个"理论语境"。语境信息来源（优先级递减）：

1. **显式传入**：调用方传入 `source_context` 参数（如 `"hegel"`, `"marx"`, `"chanlun"`）
2. **文件名推断**：从输入文件名提取（`experiment_hegel.py` → `"hegel"`）
3. **内容信号**：文本中出现的特征词汇（高频出现"Aufhebung"→黑格尔语境；高频出现"生产关系"→马克思语境）
4. **默认**：`"default"`（无语境区分，退化为当前行为）

### 2.2 查询区分规则

```python
def _get_vertex_id(label: str, source_context: str, distinction_rules: dict) -> str:
    """
    查询区分规则，决定是否为 label 附加语境前缀。

    distinction_rules: {signifier -> list[Rule]}
    """
    base_hash = sha256(label.encode())[:12]

    if label not in distinction_rules:
        # 无规则：退化为当前行为（全局唯一 ID）
        return f"c_{base_hash}"

    rules = distinction_rules[label]
    active_rules = [r for r in rules if r["status"] == "active"]

    if not active_rules:
        return f"c_{base_hash}"

    # 检查当前 source_context 是否在任何规则的 contexts 中
    for rule in active_rules:
        if source_context in rule["contexts"]:
            # 需要区分：生成带语境的 ID
            ctx_hash = sha256(f"{label}::{source_context}".encode())[:12]
            return f"c_{ctx_hash}__ctx_{source_context}"

    # source_context 不在任何规则的 contexts 中：使用默认 ID
    return f"c_{base_hash}"
```

### 2.3 ID 生成规则

| 情况 | 顶点 ID 格式 | 示例 |
|------|-------------|------|
| 无区分规则 | `c_<sha(label)[:12]>` | `c_a3f9b21c4e7d` |
| 有规则，语境已知 | `c_<sha(label::ctx)[:12]>__ctx_<ctx>` | `c_7f2a1d3e5b9c__ctx_hegel` |
| 有规则，语境未知 | `c_<sha(label)[:12]>` | `c_a3f9b21c4e7d`（退化） |

**关键性质**：相同 label + 相同 ctx = 相同 ID（跨文档合并正确）；相同 label + 不同 ctx = 不同 ID（语境内合并，跨语境分离）。

---

## 3. phi_L 调用界面（更新后）

```python
def phi_L(
    text: str,
    source_context: str | None = None,       # 新增：来源语境标记
    rules_path: str | Path | None = None,    # 新增：区分规则文件路径
) -> Graph:
    """
    完整管线：text -> typed directed simplicial complex。

    source_context: 当前文本的理论语境（如 "hegel", "marx", "chanlun"）。
                    None 时尝试从 rules_path 加载的默认语境，否则退化为当前行为。
    rules_path: distinction_rules.jsonl 路径。None 时从默认 checkpoint 目录加载。
    """
    # 1. 加载区分规则（新增）
    distinction_rules = _load_distinction_rules(rules_path)

    # 2. 语言检测 + 预处理（不变）
    lang = detect_language(text)
    trees = preprocess_zh(text) if lang == "zh" else preprocess(text)

    # 3. 顶点提取（修改：传入规则 + 语境）
    vertices, token_to_vertex = extract_vertices(
        trees,
        source_context=source_context or "default",
        distinction_rules=distinction_rules,
    )

    # 4. 边提取（不变）
    ...
```

---

## 4. 进化路径：粗粒度 → 细粒度

### 阶段 0（当前实现）

- 顶点 ID = `c_<sha(label)[:12]>`
- settlement 检测到含 "signifier" 关键词的裁决 → 全局标记 `_disambiguated.add("*")`
- 缺陷：全局标记没有操作意义（没有语境信息，无法生成不同的 ID）

### 阶段 1（本文档设计）

- 引入 `distinction_rules.jsonl`
- phi_L 调用时传入 `source_context`
- 区分规则从 settlement_history 中手动提取（bootstrap 阶段）或通过提取算法自动产生
- 顶点 ID 带语境后缀

### 阶段 2（穿越积累后）

- 区分规则提取完全自动化：每次 settlement 后调用提取算法，更新 `distinction_rules.jsonl`
- `confidence` 字段生效：低频 settle（confidence < 0.3）保持 `pending`，不改变 ID 生成行为
- 高频 settle（confidence > 0.7）激活规则

### 阶段 3（规则自我修正）

- 区分规则可以被否定：如果一条区分规则产生的新顶点（带语境 ID）反而没有在 settlement 中出现，说明这条规则可能是误判
- 引入 `require_merge` 规则类型：发现两个带语境 ID 的顶点其实应该合并
- 规则版本化：每条规则有 `version` 和 `superseded_by` 字段

---

## 5. Bootstrapping

### 第一个 phi_L 是外部给定的

当前的 phi_L 实现（Stanza + 哈希 + 固定边类型词表）是系统的原始 phi_L——外部给定，不来自穿越积累。它的局限性（signifier 合并问题）已知且可接受，因为：

1. 它能运行——能产生图，能开始穿越
2. 它的错误是可检测的——错误的合并会产生异常的 cycle 结构，settlement 会发现它们
3. 它的错误是自我揭示的——settlement_history 中的跨语境 NEGATION cycle 就是错误合并的证据

### 后续 phi_L 通过穿越积累自我修订

穿越积累的 settlement_history → 提取区分规则 → 更新 distinction_rules.jsonl → phi_L 下次调用时行为改变

这是一个闭环：**phi_L 的输出（图结构）→ 穿越 → settlement → 区分规则 → phi_L 的输入规则（改变下次输出）**

不需要外部监督——系统自己发现自己的区分错误，并修正下次的提取行为。

### Bootstrapping 的初始区分规则

为加速 bootstrapping，可以手动提供一批初始区分规则（基于已知的理论领域划分）：

```json
// checkpoint/default_distinctions.jsonl（手动种子，不来自穿越）
{"signifier": "consciousness", "contexts": ["hegel", "marx", "freud"], "rule_type": "require_separation", "status": "active", "confidence": 1.0, "source": "manual_seed"}
{"signifier": "dialectic", "contexts": ["hegel", "marx"], "rule_type": "require_separation", "status": "active", "confidence": 1.0, "source": "manual_seed"}
{"signifier": "subject", "contexts": ["hegel", "lacan", "kant"], "rule_type": "require_separation", "status": "active", "confidence": 1.0, "source": "manual_seed"}
{"signifier": "negation", "contexts": ["hegel", "lacan", "logic"], "rule_type": "require_separation", "status": "active", "confidence": 1.0, "source": "manual_seed"}
```

手动种子的 `confidence` 设为 1.0（最高信心），作为初始先验。穿越积累后如果发现手动种子有误，可以被自动规则以更新版本覆盖。

---

## 6. 与现有系统的接口

### 6.1 区分规则文件位置

```
~/.swarm/checkpoint/
  default_encounters.jsonl      # 已有：encounter_log
  default_settlements.jsonl     # 已有：settlement_history
  default_state.json            # 已有：mutable state
  default_distinctions.jsonl    # 新增：区分规则（phi_L 读取）
  default_distinctions_seeds.jsonl  # 新增：手动种子（优先级最高）
```

### 6.2 daemon 集成点

`daemon.py` 在每次 settlement 后：

1. 调用区分规则提取器（`extract_distinction_rules(settlement_record, k_active)`）
2. 将新规则追加到 `default_distinctions.jsonl`
3. phi_L 下次被调用时（通过 `auto_feed.py` 或 `feeds/`）自动读取更新后的规则

### 6.3 feed 处理时的语境注入

`auto_feed.py` / `feeds/` 处理器在调用 `phi_L()` 时，从文件名或元数据提取 `source_context`：

```python
# 示例：feeds/hegel_phenomenology.txt 处理
ctx = extract_context_from_filename("hegel_phenomenology.txt")  # -> "hegel"
graph = phi_L(text, source_context=ctx, rules_path=checkpoint_dir / "default_distinctions.jsonl")
```

---

## 7. 边界条件与局限性

### 边界条件（会导致机制失效的情况）

1. **语境未知**：如果调用方不传 `source_context` 且无法从文件名推断，退化为无区分行为
2. **语境标签不一致**：同一理论域在不同 feed 中用不同标签（"Hegel", "hegel", "G.W.F. Hegel"），规则不会匹配
3. **规则冲突**：两条规则对同一 signifier 有不同的 contexts 集合，取并集处理（保守策略）
4. **早期穿越的误合并顶点**：已经合并的顶点（早于规则生效前处理的文本）不会自动分裂——规则只影响后续 phi_L 调用，不修改已有图结构

### 局限性（不打算在当前版本解决的）

1. **图不可变性**：分离规则不能修复已合并的历史顶点。解决方案需要图的重放（expensive），留到阶段 3
2. **多义词的细粒度区分**：当前只支持"按理论域区分"，不支持"按段落内语义区分"
3. **规则的传递性**：如果 A 和 B 需要区分，B 和 C 需要区分，A 和 C 是否需要区分？当前不处理传递性

---

## 8. 开放问题（留给后续 session）

1. 区分规则的 `confidence` 阈值设多少？（当前提案：active > 0.7，pending 0.3-0.7，ignored < 0.3）
2. 手动种子和自动提取规则的优先级如何协调？（当前提案：手动种子置信度 1.0，自动规则可以用更高频率的证据超过它）
3. `require_merge` 规则类型的触发条件是什么？（当前提案：同一 signifier 的两个带语境顶点频繁出现在同一 settlement cycle 的同侧）
4. 区分规则本身是否需要 settlement 机制？（phi_L 进化规则本身是否需要被"结算"？）
