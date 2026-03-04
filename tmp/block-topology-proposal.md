# 谱系区块拓扑化——原始对话提案

> 来源：编排者与 Claude（claude.ai）对话，2026-02-24
> 文件：claude.ai-谱系区块链化 - Claude.pdf

## 核心命题

**事件不可变，拓扑可变。**

区块作为节点——记录发生了什么——一旦写入就不可改。但区块之间的关系结构可以被后续区块追溯改写。这是 Nachträglichkeit 的技术形式：S1 已经在那里了，不可篡改；但 S2 的到来追溯地改变了 S1 与整个结构的关系。改变的不是能指本身，而是能指之间的拓扑。

**拓扑变更本身也是事件，因此也必须被记录为新区块。** 新区块的加入又可能改变拓扑。这是自指结构——拓扑的变更是拓扑的一部分。

## 三层架构

### Event Layer（不可变）
- 内容寻址的 JSON 文件，文件名为内容的 SHA256 哈希
- 写入后设为只读
- 区块类型：event | consensus | residue | tension | rewrite

```json
{
  "id": "sha256_of_this_content",
  "type": "event | consensus | residue | tension | rewrite",
  "timestamp": "...",
  "source": "cc | gemini | codex | serena",
  "content": { ... },
  "refs": ["指向先前区块的id"]
}
```

### Relation Layer（可变，追加式）
- 每条关系记录为一行 JSONL
- 关系可追加不可删除
- 追溯改写 = 追加新关系 + 指向产生改写的区块

```json
{
  "from": "block_id",
  "to": "block_id",
  "relation": "negates | supersedes | residue_of | reopens | ...",
  "created_by": "block_id_of_the_event_that_created_this_relation",
  "timestamp": "..."
}
```

### Reflexive Layer（递归闭合）
- 每当 Relation Layer 被追加时，同时在 Event Layer 写入一个 type="rewrite" 的区块
- rewrite 区块的 id 又可以出现在后续关系记录的 created_by 字段中
- 递归自动闭合

## 多主体拓扑共识

三个模型的分工：
- **CC（Claude Code）**：主体位置，负责 RTAS 蜂群，生产事件
- **Gemini**：概念层严格质询循环
- **Codex**：代码层严格质询循环

### 关键洞察：Real 不在分歧处，在共识处

Gemini 和 Codex 必须质询到达成共识。每次共识都是通过排除某些东西才达成的——被排除的东西（为达成共识而放弃的部分）才是奇点（对象 a）。

### 共识仪式

每次 Gemini-Codex 质询收敛时，强制生成三个原子区块：

1. **consensus 区块**：最终达成的结论
2. **residue 区块**：双方各自放弃了什么，以及放弃的理由（让步清单）
3. **tension 区块**：双方都承认没有完全解决但选择暂时搁置的部分（未决张力）

### 奇点检测

持续性分歧的自动标记：当同一个拓扑位置被反复改写且持续不收敛时，标记为奇点。奇点 = 概念层与实现层不可同时满足的位置。

### 剩余积累与拓扑变更

沉淀的 residue 不消失，积累在拓扑中。当同一区域反复产生同类型 residue，标记为系统性结构问题。CC 选择"捡起剩余物"的那个时刻，就是谱系真正发生拓扑变更的时刻——先前被视为已解决的共识被追溯地打开。

## 本地实现方案

```
.chanlun/block-topology/
├── blocks/           # 内容寻址的 JSON 区块（SHA256.json）
├── relations.jsonl   # 追加式关系图
└── index.json        # 拓扑索引/元数据
```

- 不需要区块链基础设施、数据库、服务器
- 文件系统 + 命名约定
- CLI 工具读写
- Serena 在质询循环结束时调用写入脚本
- networkx 或类似工具可视化拓扑

## 与现有谱系体系的关系（待讨论）

1. 现有 `.chanlun/genealogy/` 目录中的谱系文件如何迁移/映射到区块拓扑？
2. 现有 `dag.yaml` 与 Relation Layer 的关系？替代还是共存？
3. 现有 `ceremony_scan.py` / `topology_operator.py` 如何适配？
4. 共识仪式与现有 `plan-review` skill 的关系？
5. 区块拓扑的引入是否改变 RTAS 循环本身？

## 对话中未解决的问题

1. **迁移路径**：177 条已结算谱系如何成为区块？一次性转换还是渐进？
2. **性能**：SHA256 内容寻址在大量区块时的查询效率
3. **residue 提取的自动化程度**：当前质询过程的记录是否足以自动提取让步轨迹？
4. **与 git 的关系**：区块拓扑是否需要独立于 git 的版本控制？
