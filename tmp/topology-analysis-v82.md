# Block-Topology 冷读分析报告（v82-swarm）

## 1. 基本统计

| 指标 | 值 |
|------|-----|
| meta.json 声明区块数 | 259 |
| blocks/ 目录实际文件数 | 259+（截断，见下文孤儿分析） |
| relations.jsonl 声明关系数 | 1072（meta.json），实际行数 1088 |
| 谱系编号范围 | 001 ~ 228（含子编号 005a/b, 019a-d, 020a, 030a, 073a/b） |
| 缺失编号 | 100, 101, 142, 144, 146, 148-152（谱系编号不连续，这些编号在 id_mapping 中不存在） |

## 2. 关系类型分布

| 关系类型 | 出现次数 | 占比 |
|----------|---------|------|
| depends_on | ~550 | 50.6% |
| related | ~457 | 42.1% |
| tensions_with | 31 | 2.9% |
| negated_by | 13 | 1.2% |
| negates | 13 | 1.2% |
| records | 14 | 1.3% |
| splits | 5 | 0.5% |
| residue_of | 4 | 0.4% |

**观察**：depends_on 和 related 合计占比 92.7%，构成图的主干。辩证关系（tensions_with + negated_by + negates）合计 57 条（5.2%），这是谱系辩证演化的结构痕迹。

## 3. 区块格式不一致（重复结构模式）

### 3.1 两种区块格式共存

**格式 A（迁移格式）**：来自 `source: "migration"`，包含完整的 content 包裹结构：
```json
{
  "id": "<sha256>",
  "type": "event",
  "timestamp": "...",
  "source": "migration",
  "content": {
    "id": "001",
    "title": "...",
    "status": "已结算",
    "type": "矛盾记录",
    "depends_on": [],
    "related": [],
    "negates": [],
    "negated_by": [],
    "source_file": "..."
  },
  "refs": [],
  "git_ref": ""
}
```

**格式 B（consensus-ceremony 格式）**：来自 `source: "cc"`，扁平结构：
```json
{
  "source": "cc",
  "block_type": "meta-rule",
  "title": "...",
  "genealogy_id": 228,
  "depends_on": [227, 226, 36],
  "timestamp": "...",
  "id": "<sha256>"
}
```

**格式 C（精简格式）**：仅含 id、number、source_file、type：
```json
{
  "id": "<sha256>",
  "number": 218,
  "source_file": "...",
  "type": "语法记录",
  "source": "cc"
}
```

**诊断**：三种格式共存意味着写入路径不统一。格式 A 有完整的关系信息嵌入（content.depends_on, content.related 等），而格式 B/C 的关系完全依赖 relations.jsonl。这是**剩余积累**——迁移格式保留了关系的双重存储（区块内 + relations.jsonl），增加了不一致风险。

### 3.2 关系记录格式不一致

relations.jsonl 中至少存在四种关系记录格式：

1. **完整格式**（lines 1-943）：`{"from": "<sha256>", "to": "<sha256>", "relation": "depends_on", "order": 1, "created_by": "...", "timestamp": "..."}`
2. **简化格式**（lines 1029+）：`{"from": "<short_hash>", "to": "<genealogy_id>", "type": "depends_on"}`——用 `type` 替代 `relation`，用谱系编号替代 SHA256
3. **残缺格式**（lines 959-960）：`{"target": "<sha256>", "relation": "depends_on", "order": 1}`——缺少 `from` 字段
4. **混合格式**（lines 1070-1082）：`{"type": "depends_on", "target_genealogy_id": "137"}`——仅有 target，缺少 from
5. **数组目标格式**（line 1083-1084）：`{"from": "<sha256>", "to_genealogy": [226, 36, 76], "relation": "depends_on"}`

**诊断**：5 种关系格式共存是严重的**剩余积累**。尤其是格式 3 和 4（残缺格式），缺少 from 字段意味着这些关系在图遍历中是悬空的——它们指向目标但没有起点，无法被任何图算法正确处理。

## 4. 奇点候选

### 4.1 高入度节点（超级被依赖者）

从 relations 数据分析，以下区块被大量依赖：

| 区块 | 谱系号 | 入度估计 | 角色 |
|------|--------|---------|------|
| `650f4e37...` | 020 | 15+ depends_on 指入 | 阻断等待——蜂群的门控节点 |
| `3cc72fade...` | 016 | 10+ depends_on 指入 | 四分法前身 |
| `311db38e...` | 042 | 8+ depends_on 指入 | |
| `5286ce2b...` | 030a | 7+ depends_on 指入 | |
| `d0622b2a...` | 041 | 7+ depends_on 指入 | |
| `8c5cf4e1...` | 029 | 6+ depends_on 指入 | |
| `3b5b801b...` | 069 | 6+ depends_on 指入 | RTAS 蜂群定义 |
| `f9407ddb...` | 178 | 5+ depends_on 指入 | |

**奇点 #1**：020号（阻断等待）是整个 DAG 中最关键的中间节点之一。大量后续谱系通过 020 → 015 → 014 链路建立依赖。如果 020 的定义发生变更，爆炸半径极大。

**奇点 #2**：069号（RTAS蜂群定义）是元层到操作层的关键枢纽，既被多个元规则依赖，又与 093 号存在 tensions_with 关系（line 918）。

### 4.2 tensions_with 集群（张力热区）

| 张力对 | 谱系号 | 状态 |
|--------|--------|------|
| 007 ↔ 008 ↔ 009 | 笔定义三角张力 | tensions_with（line 911-913） |
| 012 ↔ 013 ↔ 019d | 谱系张力 | tensions_with（line 914-915） |
| 062 ↔ 065, 064 ↔ 065, 068 ↔ 065 | ceremony 与内存张力 | tensions_with，valid_until: 068（line 916-917, 925） |
| 069 ↔ 093 | RTAS 与 ceremony 张力 | tensions_with（line 918） |
| 062 ↔ 069, 062 ↔ 094, 062 ↔ 104 | ceremony 链张力 | tensions_with，已 settled（line 922-924） |
| 089 ↔ 168, 089 ↔ 169 | 扬弃与操作张力 | tensions_with（line 929-930） |
| 020a ↔ 068 | 阻断等待与创世 Gap | tensions_with（line 926） |
| 053 ↔ 167 | | tensions_with（line 928） |
| 105 ↔ 090, 105 ↔ 095 | | tensions_with（line 920-921） |
| 139 ↔ 166 | | tensions_with（line 927） |

**观察**：007/008/009 的三角张力是最早期的（笔定义相关），至今保留在 tensions_with 中而未被 negated_by 解消——可能是**已解决但未更新关系状态**，或确实仍处于张力中。

### 4.3 孤立区块候选

**残缺关系的孤儿节点**：lines 959-960 和 1070-1082 包含约 12 条缺少 `from` 字段的关系，这些关系的来源区块不可追溯。它们引用的 target 包括：
- 137号、143号、218号、224号、225号、057号、173号

这些是**影子依赖**——被声明但无法在图中定位起点的依赖关系。

### 4.4 meta.json block_count 与实际不一致

meta.json 声明 `block_count: 259`，`relation_count: 1072`。但 relations.jsonl 实际有 1088 行。差异 = 16 行，正好对应后期追加的关系（lines 1073-1088）。meta.json 的计数未被同步更新。

## 5. 剩余积累总结

| 编号 | 类型 | 描述 | 严重度 |
|------|------|------|--------|
| R1 | 格式碎片 | 区块存在 3 种不同的 JSON schema | 中 |
| R2 | 格式碎片 | relations.jsonl 存在 5 种关系格式 | 高 |
| R3 | 悬空关系 | 12+ 条关系缺少 from 字段 | 高 |
| R4 | 双重存储 | 迁移格式区块内嵌关系与 relations.jsonl 可能不一致 | 中 |
| R5 | 计数漂移 | meta.json 的 relation_count (1072) < 实际行数 (1088) | 低 |
| R6 | ID 体系混用 | 部分关系用 SHA256 全长、部分用 8 字符短 hash、部分用谱系编号字符串、部分用数字 | 高 |

## 6. 重复结构模式

### 6.1 否定链的重复出现

谱系中存在一个反复出现的模式：**X 提出 → X 被 tensions_with → X 被 negated_by → 新 Y 替代 X**。这不是冗余，而是辩证否定的结构痕迹。具体实例：

- 032（缠论定义审计）→ 088（被否定）→ 173（替代）
- 086 → 087（被否定：补丁思维第一次否定）
- 062/065 的张力 → 068（解消）→ 069（新结构）
- 154 → 156（被否定）
- 158 → 162（被否定）

### 6.2 "严格性"相关区块的扇出

090号（严格性语法规则）的下游扇出极广：被 136、137、218、225 等多个后续谱系直接引用。这表明 090 号是蜂群语法的**根公理之一**，与 069 号（RTAS）、005b 号（对象否定对象）构成三个支柱。

## 7. 关键发现摘要

1. **relations.jsonl 的格式碎片是最紧迫的结构问题**：5 种格式共存 + 12 条悬空关系 = 图遍历不可靠。任何依赖 relations.jsonl 做自动化分析的工具都需要处理格式兼容问题。

2. **020号和069号是 DAG 的两个关键奇点**：020号是元层门控，069号是操作层枢纽。它们的入度最高，修改爆炸半径最大。

3. **早期张力（007/008/009）可能需要状态审计**：这些 tensions_with 关系是否已被后续谱系解消但未在 relations.jsonl 中标记为 settled？如果是，则存在张力状态的维护缺口。

4. **meta.json 与实际数据的漂移**：relation_count 差 16 条。虽然影响低，但信号意义明确——写入路径在追加关系时未更新 meta.json。

5. **区块格式三种并存本身不是问题，但双重关系存储是问题**：迁移格式区块内的 depends_on/related 与 relations.jsonl 之间的一致性没有保证机制。如果两者不一致，以哪个为准？

---

*生成时间：2026-02-27*
*工位：topology-analyst*
*数据来源：.chanlun/block-topology/（meta.json + blocks/ + relations.jsonl）*
