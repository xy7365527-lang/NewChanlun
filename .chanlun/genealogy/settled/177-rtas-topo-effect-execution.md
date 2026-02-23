---
id: '177'
title: RTAS循环内谱系驱动拓扑操作——158号第一步工程落地
type: 語法記録
status: 已结算
date: 2026-02-23
depends_on:
  - '158'   # 三步战略（矛盾→拓扑效力）
  - '174'   # RTAS循环定义（谱系驱动）
  - '141'   # retrospective topo_effect 标注方案
  - '147'   # 矛盾具有拓扑效力
negates: []
related:
  - '040'   # negation_form 双字段 schema
  - '170'   # 承重点保护
  - '176'   # Δ谱系>0 检测
tensions_with: []
topo_effect: ""
negation_form: ""
---

## 事件

158号三步战略第一步"让矛盾具有拓扑效力"的工程落地。

对审结论（Codex + Gemini Round 1 收敛）否定了在 PostToolUse hook 中直接执行拓扑操作的原方案：
- Codex 否定：PostToolUse 是 advisory，不应有副作用
- Gemini 否定：hook 执行是"脚本驱动拓扑"不是"谱系驱动拓扑"；RTAS 循环（174号）才是谱系的实现

收敛方案：拓扑操作在 RTAS 循环内执行。

## 工程产出

### 1. ceremony_scan.py — detect_pending_topo_effects()

扫描 settled/*.md，提取含结构化 topo_effect（type:target:scope）但无 topo_executed_at 的文件。输出为 P0 工位到 workstations。

### 2. topology_operator.py — auto_execute_from_file()

RTAS 循环调用入口。流程：
1. 读取谱系文件 frontmatter
2. 检查 topo_executed_at（已执行则跳过——幂等）
3. 验证结构化 topo_effect
4. 检查目标节点承重性（compute_load_bearing_score）
5. 承重点 → 拒绝自动执行，返回 warning
6. 非承重点 → 执行拓扑操作（修改 dag.yaml）
7. 回写 topo_executed_at 到谱系文件 frontmatter

### 3. 执行状态记录在谱系文件本身（编排者修正1）

topo_executed_at 写入谱系文件 frontmatter，不写入 dag.yaml 节点。单一数据源，谱系携带自身拓扑后果的完整记录。

### 4. genealogy-write-guard.sh — Guard 4 advisory

检测新写入谱系文件的结构化 topo_effect，输出 advisory 提示。不执行任何副作用。

### 5. 测试

- test_topology_operator.py：+9 测试（正常执行、承重点、幂等、已执行跳过、无效 topo_effect、DAG 更新验证）
- test_ceremony_scan.py：新建，7 测试（结构化检出、已执行跳过、描述性跳过、空目录、混合场景、三种类型覆盖）
- 全部 51 测试绿灯

## 执行路径

```
谱系写入(带topo_effect) → ceremony_scan.detect_pending_topo_effects() 检测
→ Lead 在 RTAS 循环中调用 topology_operator.auto_execute_from_file()
→ dag.yaml 更新 + 谱系文件回写 topo_executed_at
→ 下次 ceremony_scan 不再检出（幂等）
```

## 边界条件

1. **承重点拒绝自动执行**：compute_load_bearing_score >= threshold 时返回 warning，需人工决策
2. **非结构化 topo_effect 跳过**：描述性文本（如"引入 topo_effect retrospective 标注方案"）不是可执行操作
3. **dag.yaml 不存在时跳过**：返回 executed=False

## 开放矛盾

### 开放矛盾1：negation_form 标注判据未形式化（保持开放）

040号+147号规定了 waiting→freeze / expansion→split / separation→sever 映射。但"这个否定是 waiting 还是 expansion"的分类决策由 LLM 隐式做出，判据未形式化。141号结算的是标注时机（retrospective），不是标注判据。这是真正的开放矛盾，不是当前能裁定的。

### ~~开放矛盾2~~（已裁定）：158号第三步撤回

### ~~开放矛盾3~~（已裁定）：158号第三步撤回

**编排者裁定**：开放矛盾2和3是同一条——genus 0 的系统不能通过连续变形变成 genus 1，原表述预设不可能的拓扑操作。158号第三步撤回。递归总方针中"取消编排者位置"和"亏格自然生成"表述一并修正。

## 下游推论

1. ~~**递归总方针.md 需修正**~~：已执行——158号第三步标记撤回 + "亏格自然生成"改为"拓扑手术"
2. **现有谱系回标**：已有 topo_effect 的谱系（如141号）是描述性文本，不会被自动执行——这是正确行为，不需要回标
3. **RTAS 循环集成**：ceremony_scan 现在输出 pending_topo_effects 工位，Lead 在 RTAS 循环中执行 auto_execute_from_file()
4. **158号从三步战略变为两步战略**：第一步（矛盾具有拓扑效力）+ 第二步（异步自指遭遇不可能性），不存在第三步

## 谱系影响

- 158号第一步落地完成（矛盾→拓扑效力自动化）
- 174号忠实实现（拓扑操作在 RTAS 循环内，是"谱系驱动"不是"脚本驱动"）
- 141号结论1实现（retrospective topo_effect 作为输入）
