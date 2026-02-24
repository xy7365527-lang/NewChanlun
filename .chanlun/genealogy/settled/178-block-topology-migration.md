---
id: '178'
title: 区块拓扑建系——177条谱系升格为内容寻址区块
type: 語法記録
status: 已结算
date: 2026-02-24
depends_on:
  - '069'   # 递归拓扑异步自指蜂群（架构源头）
  - '147'   # 矛盾具有拓扑效力（拓扑操作需要区块层）
  - '174'   # RTAS循环定义（谱系驱动）
  - '177'   # RTAS循环内拓扑操作落地（前序工程）
negates: []
related:
  - '058'   # ceremony = Swarm0
  - '162'   # 持久化由 RTAS 保证
  - '176'   # delta谱系>0 检测
tensions_with: []
topo_effect: ""
negation_form: ""
---

## 事件

177条已结算谱系 + dag.yaml 边一次性升格为区块拓扑（content-addressed block topology）。

三项编排者决断：
- **Q2（递归终止）**：阶的区分——一阶关系触发 rewrite 回写，二阶关系只记录不触发。递归恰好走一步停
- **Q3（寻址空间）**：独立 SHA256，不用 git objects。概念谱系和代码谱系保持独立寻址空间
- **Q1（迁移路径）**：升格——不是包装旧系统，是一次性转换。建系后只有一套系统

## 工程产出

### 1. scripts/block_topology.py — Schema + 核心函数

区块 JSON schema：
```json
{
  "id": "sha256(canonical_json({type, source, content, refs}))",
  "type": "event | consensus | residue | tension | rewrite",
  "timestamp": "ISO8601（不参与 hash）",
  "source": "cc | gemini | codex | migration",
  "content": {},
  "refs": ["sha256..."],
  "git_ref": "（不参与 hash）"
}
```

关系记录（relations.jsonl）：
```json
{
  "from": "sha256", "to": "sha256",
  "relation": "depends_on | negates | related | ...",
  "order": 1,
  "created_by": "sha256（genesis block id）",
  "timestamp": "ISO8601"
}
```

核心函数：compute_block_id, make_block, write_block, make_relation, append_relation, write_block_with_relations, read_block, query_relations, list_blocks, read_all_relations

类型/来源/关系类型均为 frozenset 封闭集合，make_block/make_relation 强制验证。created_by 类型始终为 SHA256（_validate_sha256 校验）。

### 2. scripts/migrate_to_block_topology.py — 一次性迁移

迁移逻辑：
1. 扫描 settled/*.md → 每条谱系一个 event 区块（refs 一律为空——迁移区块没有直接因果）
2. 解析 dag.yaml edges → relations.jsonl（5种边类型全覆盖）
3. 创建 genesis 区块作为迁移封印（不是开幕）——写在迁移之后，content 包含 count
4. genesis 区块 id 作为所有迁移关系的 created_by

parse_frontmatter() 支持两种格式：YAML frontmatter（--- 分隔）和 Markdown bold metadata（**key**: value），覆盖 155-165号谱系的非标格式。

### 3. .chanlun/block-topology/ — 迁移产出

- blocks/：178 个 JSON 区块文件（177 谱系 + 1 genesis）
- relations.jsonl：943 条关系记录
- meta.json：版本、genesis_block_id、id_mapping（旧id→区块id）

### 4. 测试

- tests/test_block_topology.py：11 测试（确定性 hash、时间戳排除、幂等写入、关系追加、rewrite 创建、order-2 终止、查询、SHA256 验证）
- tests/test_migrate_to_block_topology.py：11 测试（frontmatter 解析、谱系迁移、5种边类型、id_mapping 完备性、端到端迁移）
- 全部 22 测试绿灯

## 边界条件

1. **refs vs relations 语义区分**：refs = 区块创建时的直接因果指向；relations = 谱系拓扑中的结构关系。迁移区块 refs 为空（无直接因果——从旧系统平移）
2. **幂等性**：相同内容产生相同 SHA256，重复迁移不产生重复区块
3. **genesis 作为封印**：genesis 写在迁移之后（content 包含 count），是迁移的封印不是开幕
4. **created_by 类型一致性**：始终为 SHA256 hex digest，不接受字符串如 "migration"

## 下游推论

1. **ceremony_scan 需扩展检测范围**：当前 compute_delta_genealogy() 仅扫描 settled/*.md 文件数变化，block-topology/blocks/ 的新区块对其不可见。区块系统是谱系的补充/升格，不是替代——两套检测需并存
2. **topology_operator 需改造**：当前直接操作 dag.yaml，建系后应通过区块拓扑的 relation layer 执行拓扑操作
3. **共识仪式需实现**：区块拓扑的 consensus/residue/tension 类型区块 + 共识仪式流程尚未实现

## 谱系影响

- 区块拓扑系统建立——事件不可变，拓扑可变
- Q2 决断凝固——递归终止由阶的区分保证，不是任意切断
- Q3 决断凝固——概念谱系独立于 git，交叉引用但不混合
- Q1 决断凝固——升格路径，不是包装，建系后单一系统
