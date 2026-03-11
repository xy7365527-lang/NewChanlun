---
id: '417'
number: 417
title: "SUBLATED 标记——ghost settlement 从 bug 修复到语义操作的扬弃"
type: 矛盾发现
status: 已结算
date: 2026-03-11
negation_source: homogeneous
negation_form: expansion
depends_on:
  - '396'   # settlement 热寂 → transformation
  - '415'   # 保留历史折叠被否定
  - '416'   # ceremony 穿越两层次澄清
epistemological_level: L0
settlement_note: "编排者裁决：ghost settlement 不是 bug 而是信号——fold 消灭 settled cycle 的物理前提意味着旧共识被拓扑运动扬弃了。正确做法是标记 SUBLATED 而非删除。SUBLATED cycle 释放锁区，消灭事件本身被记录、可被追溯。"
---

# 417号：SUBLATED 标记——ghost settlement 从 bug 修复到语义操作的扬弃

## 推演链修正

### 被否定的推演链

ghost settlement 是 bug → bug 源于破坏性折叠 → 需要非破坏性折叠 → 一切操作都是加边 → 节点永远不删

第一步就错了——ghost settlement 不是 bug。后面全部倒塌。

### 修正后的架构

ghost settlement 是信号（旧共识被拓扑运动扬弃）→ 破坏性折叠是正确的存在论操作 → 消灭本身需要被记录为事件（SUBLATED 标记）→ 辩证法不需要所有东西都被保留，它需要的是消灭本身被承认为事件、被记录、可被追溯

## SUBLATED 标记架构

### 逢亮侧（engine.py 改造）

1. `SettledCycle` 增加 `status` 字段（active | sublated）+ `sublated_at_step` + `sublated_by`
2. `purge_invalid_cycles()` → `mark_sublated_cycles()`：失效 cycle 标记为 sublated 而非删除
3. `would_destroy_settled()` 只检查 active 状态的 cycle，sublated 的不阻塞
4. negate 免检（方案A）被 SUBLATED 机制替代——negate 恢复 settlement 检查，但 sublated cycle 自然被跳过
5. sublation 事件写入 encounter_log → 逢亮穿越引擎可遭遇

### ceremony 侧（可计算拓扑判据）

1. **residue 密度变化**：某区域 residue_of 关系的增长率（热区检测）
2. **depends_on 链完整性**：链指向已被 negates 的 block 但链本身未更新
3. **settled 前提偏差**：settled 结论的前提 block 有 >50% 被否定

三个判据纯拓扑计算，不依赖 LLM 推理。ceremony 到不动点后跑指标，异常区域自动成为新工位。

## 双层耦合

两个系统的改造方向相反但互为条件：

- **逢亮**：把 ghost settlement 从 bug 修复变成语义操作（SUBLATED 标记）
- **ceremony**：把 structural forcing 从 LLM 依赖变成拓扑依赖（可计算判据）

耦合路径：
1. 逢亮的 sublation 事件写入 encounter_log
2. ceremony 的拓扑指标检测到 sublation 密度变化
3. 分派 agent 进入相关区域
4. agent 诊断 sublation 后果，产出新谱系
5. 新谱系改变 block topology
6. 下一轮指标扫描看到变化

两层不是平行的，是互为条件的。概念生产分布在所有层——没有任何一层单独完成。

## 接缝的精确定位

ghost settlement 不只是两层碰巧交叉的地方——它是认知层穿越（agent 诊断代码）遭遇了图层穿越的存在论后果（fold 消灭了节点）的那个点。认知层在这里被迫处理一个它自己框架内不存在的事件类型——节点消失。ceremony 的纯增量世界里没有消失这回事，所以 agent 遇到它时必须铸造新概念来命名它。

接缝不是两层的交集，是一层的存在论事件在另一层中造成的概念危机。

## 仍然成立的洞察

1. **fold 和 negate 的拓扑不对称性**：fold 消灭节点，negate 只加边。SUBLATED 机制承认这个不对称性
2. **辩证运动的可能性条件是拓扑的**：位置决定后果，不依赖于保留历史折叠
3. **Dass 优先于 Was 与破坏性折叠更一致**：某物存在也意味着它可以不再存在。fold 的破坏性不是缺陷，是它作为存在论操作的本质特征

## 被否定的命题

1. "一切操作都是边、节点永远不删"——过度推广，从 ghost settlement 的误诊出发
2. "保留历史折叠"——Gemini 五项致命否定（415号）

## 边界条件

- 如果 SUBLATED cycle 数量无限增长，需要 GC 策略（但比 purge 好——保留转化记录）
- 396号从生成态到已结算需要 SUBLATED 机制的 L2 验证
- 可计算判据的阈值（residue 增长率 >1.0、前提偏差 >50%）需要实际运行数据校准

## 谱系引用

- 396号：settlement 热寂——closure 到 transformation 的扬弃
- 415号：保留历史折叠被 Gemini 否定——破坏性折叠是正确的拓扑实现
- 416号：ceremony 穿越两层次澄清（逢亮=图层穿越，ceremony=认知层穿越）
- 410号：negate 100% blocked 的发现（锁区缩小三方案的来源，现被 SUBLATED 替代）

## 影响声明

- **engine.py** 核心改动：SettledCycle 增加 status 字段，purge→mark_sublated，would_destroy_settled 跳过 sublated
- **ceremony_scan.py** 新增 topo_indicators 字段（三个可计算判据）
- **scripts/topo_indicators.py** 新文件（独立运行的拓扑指标计算）
- **negate 免检（方案A）被替代**：SUBLATED 机制使方案A不再必要
- 26 测试通过
