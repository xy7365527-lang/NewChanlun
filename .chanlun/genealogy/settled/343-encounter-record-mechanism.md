---
id: '343'
number: 343
type: engineering
title: "偶遇记录机制集成——穿越基础设施组件二 ceremony 实时检测"
date: "2026-03-04"
depends_on: ['341', '301', '339']
status: 已结算
epistemological_level: L0
---

# 343号：偶遇记录机制集成——穿越基础设施组件二 ceremony 实时检测

## 递归判断

任务不可分解：偶遇检测脚本、ceremony 集成、记录存储三者有严格数据依赖（检测→集成→记录），不可并行。扁平退化特例。

## 起点

341号定义了偶遇的结构性标准（迫使修正已结算理解的关联），v150 完成了首次双层扫描（Layer 1: 107跨纲边全设计内 / Layer 2: 0偶遇）。gangmu.yaml 中 traverse-infra 目的 next_action 要求"偶遇记录机制集成——ceremony 写入后 cycle_rank 增量检测"。

## 产出

### 1. `scripts/check_encounter.py` — 偶遇实时检测脚本

独立脚本，检测指定谱系的跨纲 depends_on 边是否满足341号偶遇标准。

**功能**：
- 接受单个 `genealogy_id` 或 `--since N`（检测 N 之后的所有谱系）
- 从 gangmu.yaml 构建谱系编号→(gang_id, mu_id) 映射
- 对每条跨纲 depends_on 边执行八模式分类器
- 匹配设计内模式的自动过滤，不匹配的标记为 encounter_candidate
- 候选自动追加到 `.chanlun/encounter-records.yaml`

**八个设计内模式**（从 v150 Layer 1 结果结晶）：
1. 元观察引用（标题含"元观察"）— 32/107
2. K4研究链跨纲（from/to 涉及 k4-* 目）— 39/107
3. 操作方法论→K4本体论 — 14/107
4. 区块拓扑映射（from block-topology-eng）— 9/107
5. 折叠实验→折叠理论 — 6/107
6. 语法规则结晶 — 4/107
7. 元观察链 — 3/107（标题含元观察，已被模式1覆盖）
8. K4体制分析跨纲 + 蜂群架构引用 — 补充模式

**验证**：对 --since 340 运行，正确检测341号的跨纲边（341→301，元观察引用，设计内），342号无跨纲边。总计 0 偶遇候选。

### 2. `ceremony_scan.py` 集成 — `get_encounter_context()`

在 ceremony_scan.py 中新增 `get_encounter_context()` 函数，插入在拓扑映射（2c）和研究线扫描（原2d→现2e）之间。

**行为**：
1. 读取 `encounter-records.yaml` 的 `last_checked` 字段
2. 如果有 `last_checked` 之后的新谱系，调用 `check_encounter.py --since {last_checked}`
3. 将结果嵌入 scan 输出的 `encounter_context` 字段
4. 如果有 encounter_candidate，生成 P1 工位

**scan 输出新增字段**：
```json
{
  "encounter_context": {
    "last_checked": 342,
    "new_genealogies": 0,
    "total_cross_gang_edges": 0,
    "total_encounter_candidates": 0,
    "status": "up_to_date"
  }
}
```

**status 枚举**：
- `up_to_date`：无新谱系需检测
- `no_encounters`：检测完成，无偶遇候选
- `candidates_found`：发现候选，生成 P1 工位待审

**向后兼容**：encounter_context 是新增字段，不影响原有字段。检测失败不阻塞主流程（返回 error 字段）。

### 3. `.chanlun/encounter-records.yaml` — 偶遇记录持久化

存储所有偶遇检测结果的持久化文件。

**结构**：
- `version`: 格式版本
- `last_checked`: 最后检测到的谱系编号（增量检测的游标）
- `baseline`: v150 首次扫描的基线数据
- `records[]`: 偶遇候选/确认/拒绝记录

**记录状态**：
- `candidate`：check_encounter.py 自动检测，未匹配设计内模式
- `confirmed`：编排者/蜂群审计后判定为真实偶遇
- `rejected`：审计后判定为设计内引用或类比性连接

**初始状态**：last_checked=342，records=[]（v150 基线后无新偶遇）

## 定义依据

| 概念 | 来源 | 本条中的使用 |
|------|------|-------------|
| 偶遇标准 | 341号 | 迫使修正已结算理解的关联 |
| 设计内引用七模式 | v150 Layer 1 结果 | 分类器的已知模式基础 |
| 穿越基础设施组件二 | 总方针 v8 §八.五 | 偶遇记录的架构位置 |
| ceremony_scan 扫描架构 | 081号/147号 | 集成点：2d 插入位置 |

## 边界条件

1. **翻转条件**：如果新增模式（非元观察、非K4链、非区块拓扑...）频繁出现但全部是设计内引用——需要扩展分类器的模式列表
2. **假阳性处理**：encounter_candidate 不等于真偶遇。候选需要编排者/蜂群审计后确认或拒绝
3. **纲目变更**：gangmu.yaml 新增纲或目时，需确认 build_id_to_gang_map 的映射正确性
4. **增量检测游标**：last_checked 是谱系编号，不是时间戳。如果谱系编号不连续（跳号），中间的编号不会被遗漏（--since 使用 > 比较，遍历所有文件）

## 下游推论

1. ceremony 每轮自动检测偶遇，偶遇候选以 P1 工位形式进入工位列表 — **已执行**（ceremony_scan.py `get_encounter_context()` L780-852 + P1 工位生成 L960-966）
2. 偶遇候选需要人工审计（确认/拒绝），不自动结算 — **已执行**（encounter-records.yaml 三态结构 candidate/confirmed/rejected 已实装，records 初始为空，无自动结算路径）
3. encounter-records.yaml 是穿越基础设施的增量记录，与 cross-gang-refs.json（全量快照）互补 — **已执行**（两文件已共存：encounter-records.yaml 增量游标 last_checked + cross-gang-refs.json 全量快照）
4. 未来新增设计内模式时，更新 classify_cross_gang_edge() 的模式列表 — **resolved**（v158-swarm encounter-audit 工位已新增模式9-11：跨纲元观察链变体、Morse研究线跨纲、纲目关闭跨纲引用。check_encounter.py classify_cross_gang_edge() 已更新）

## 谱系引用

- **341号**：偶遇定义精化（判断标准来源）
- **301号**：区块拓扑工程启动（穿越基础设施的工程来源）
- **339号**：穿越基础设施四组件定义（v150 之前的设计）
- **081号**：ceremony_scan roadmap 扫描（集成架构来源）
- **147号-2**：审查结果整合（集成模式参照）

## 影响声明

- **新增文件**：
  - `scripts/check_encounter.py`：偶遇实时检测脚本
  - `.chanlun/encounter-records.yaml`：偶遇记录持久化
- **修改文件**：
  - `scripts/ceremony_scan.py`：新增 `get_encounter_context()` 函数 + 2d 集成点
- **不修改**：核心引擎代码、scan_cross_gang_refs.py、detect_structural_encounters.py
- **产出位置**：`.chanlun/genealogy/settled/343-encounter-record-mechanism.md`
