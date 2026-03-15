---
id: '456'
number: 456
title: "元观察——v245-swarm（规模跃升后的隐性退化：正确代码在 263K signifiers 下的 O(N) 属性复制退化 + 反转搜索范式结晶）"
type: meta-rule
status: 已结算
date: 2026-03-15
source: meta-observer（二阶观察，v245-swarm session 触发）
depends_on:
  - '455'   # v243-swarm 元观察（编排者从性能识别本体论错误）
  - '090'   # 严格性语法规则
epistemological_level: L0
negation_form: none
negation_source: ""
topo_effect: ""
tensions_with: []
rule_version_baseline:
  claude_md_commit: "09bc3999062d55d369dde9ebedd37c9efe4ce0e3"
  rules_dir_mtime: "2026-03-14 23:27:42 +0000"
---

# 456号：元观察——v245-swarm

## 递归判断

任务不可分解：meta-observer 是单一观察角色。扁平退化特例。

## 规则版本基线

- `claude_md_commit`: `09bc3999062d55d369dde9ebedd37c9efe4ce0e3`（与 455 号相同，规则未变）
- `rules_dir_mtime`: `2026-03-14 23:27:42 +0000`

## 观察1：规模跃升后的隐性退化

**模式**：代码在小 SNet（几千 signifiers）下正确运行，在大 SNet（263K signifiers）下因 O(N) 公共属性复制而退化——`SNet.signifiers` 属性每次调用 `dict(self._signifiers)` 复制 263K 条目的字典。被内部代码在循环中频繁调用时，产生 O(N×P) 隐性瓶颈（P=调用次数）。

**关键特征**：这不是 bug。代码在任何规模下都**逻辑正确**。退化来自**正确的防御性设计**（不可变数据结构的公共接口返回副本）在规模变化后的隐性成本。

**与 455 号的关系**：455 号观察到"编排者从性能问题识别本体论错误"。本 session 是该模式的变体——从性能诊断中发现的不是本体论错误，而是**工程设计在规模跃升后的退化点**。两者共享"py-spy 驱动的自下而上诊断"方法。

**影响范围**：7 个文件的 `.signifiers` 调用全部改为 `._signifiers` 直接访问。这打破了封装，但在不可变对象中内部直接访问是安全的——外部调用者仍通过公共属性获得副本保护。

## 观察2：反转搜索范式

**语法记录候选**：当两个集合大小不对称时（如 200 个 content vs 263K 个 signifiers），从小集合出发到大集合做 O(1) set 查找，总操作数 = O(M×W) 而非 O(N×M)。

本 session 中此范式被应用 5 次：
1. ASCII 单词术语 — text_words(~50) ∩ ascii_single(~140K) = O(50)
2. CJK 预筛 — 段落前 200 字符检测 CJK，跳过 107K 中文术语
3. 多词首词分组 — text_words ∩ multi_first_words = O(|text_words|)
4. `_expand_mappings` — content_words(~50) ∩ unmapped_single(~140K) = O(50)
5. `_expand_mappings` 多词 — content_words ∩ multi_first_words = O(|content_words|)

**收敛判断**：本 session 出现 5 次，编排者裁定显式化为规则。不是偶然的优化技巧——是处理不对称集合的通用范式。

## 观察3：pickle 兼容性缺口 → 编排者裁定：放弃 pickle

不可变对象 + pickle 持久化的组合存在固有风险：每次给 Graph 添加新的缓存属性（如 `_edge_keys`），旧 pickle 恢复的对象会缺少该属性。`__getattr__` 懒初始化是后补方案——是在错误的持久化选择上打补丁。

### 编排者洞察：Dass/Was 区分

pickle 序列化的是对象的完整状态——包括不可变数据（Dass）和可变缓存（Was）。这和 events immutable 原则冲突：你应该持久化的是 Dass（这些边存在、这些超边发生过），不是 Was（对象当前的计算状态）。加载时从 Dass 重建 Was。Was 的定义随代码演化而变，Dass 不变。

与 ghost settlement 同构——操作改变了结构的前提，但记录层没跟上。区别是 ghost settlement 发生在运行时的图操作中，pickle 兼容性发生在持久化和恢复的边界上。根本原因相同：不可变对象不是真正不可变的。它的公共接口是 frozen 的，但内部缓存在运行时被添加。

### 编排者裁定

**放弃 pickle。** Graph 和 SNet 的 persist 从 pickle dump 改为导出边和顶点的 JSONL。load 从 JSONL 读入事件，调用正常构造流程重建。缓存在构造过程中自然生成。版本兼容问题消失。

优先级：不高。下次碰到 pickle 兼容问题时执行，不再打 `__getattr__` 补丁。旧 pickle 文件做一次性迁移。

## 观察4：重复材料入口

cc-cedict 同时作为辞典（dict_cedict.jsonl）和语料（corpora/cc-cedict/）存在，触发两次摄入。暴露了摄入管道缺少去重层——同一材料可从不同入口进入 SNet。已通过删除 corpora 侧修复，但根本问题（无全局去重）仍在。

## 自环检查

- 与 455 号**部分收敛**：性能诊断方法论相同（py-spy → 定位热路径 → 结构性修复），但发现性质不同（本体论错误 vs 规模退化）
- **新维度**：观察1（规模跃升退化）和观察2（反转搜索范式）是 455 号未覆盖的

## 结论

1. **反转搜索范式**：编排者裁定显式化——写入 meta-rule（本条即是）
2. **pickle → JSONL**：编排者裁定放弃 pickle，下次碰到兼容问题时执行迁移
3. 其余观察属于定理类或行动类，不需上浮
