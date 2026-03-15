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

**收敛判断**：这不是新发现的范式（set 交集是基础数据结构操作），但在本项目中被系统性应用于解决同一类问题（大白名单 × 小文本的匹配）。如果后续继续出现类似场景，可考虑提取为通用匹配工具。

## 观察3：pickle 兼容性缺口

不可变对象 + pickle 持久化的组合存在固有风险：每次给 Graph 添加新的缓存属性（如 `_edge_keys`），旧 pickle 恢复的对象会缺少该属性。`__getattr__` 懒初始化是后补方案。

**风险评估**：此问题只在**跨版本恢复**时出现。如果部署总是 clean start，不会触发。但双实例耦合 + 持久化 + 热更新的场景下，跨版本恢复是常态。

## 观察4：重复材料入口

cc-cedict 同时作为辞典（dict_cedict.jsonl）和语料（corpora/cc-cedict/）存在，触发两次摄入。暴露了摄入管道缺少去重层——同一材料可从不同入口进入 SNet。已通过删除 corpora 侧修复，但根本问题（无全局去重）仍在。

## 自环检查

- 与 455 号**部分收敛**：性能诊断方法论相同（py-spy → 定位热路径 → 结构性修复），但发现性质不同（本体论错误 vs 规模退化）
- **新维度**：观察1（规模跃升退化）和观察2（反转搜索范式）是 455 号未覆盖的

## 结论

无需上浮。所有观察属于定理类（已结算原则的逻辑推论）或行动类（已执行的修复）。语法记录候选（反转搜索范式）尚未达到结晶阈值——需要更多 session 的重复出现确认。
