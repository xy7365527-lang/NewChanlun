---
id: "072"
title: "Hook 强制层的双重前提——执行保障与阻断语义"
status: "已结算"
type: "语法记录"
date: "2026-02-21"
depends_on: ["013", "042", "071"]
related: ["016", "033", "070"]
negated_by: []
negates: []
---

# 072 — Hook 强制层的双重前提——执行保障与阻断语义

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-21
**前置**: 013-swarm-structural-stations, 042-hook-network-pattern, 071-text-instruction-fragility-scan
**关联**: 016-runtime-enforcement-layer, 033-declarative-dispatch-spec, 070-genesis-gap-engineering-instance

**negation_form**: expansion（042号 hook 网络概念的边界扩张）
**negation_source**: homogeneous（v11 蜂群实证暴露）

---

## 背景

v11 蜂群启动时违反了 013号谱系（结构工位必须先于任务工位 spawn）。编排者指出后修正。根因分析揭示 ceremony-guard hook 的强制力存在两层缺陷。

## 根因分析

**根因 A（设计层）**：ceremony-guard.sh 是 warning-only hook。代码第 143 行始终返回 `continue: true`，即使检测到结构工位未就绪，也只注入 systemMessage 警告，不阻断 Task 调用。这使得它本质上仍是 F2 型脆弱指令（071号分类），而非真正的 runtime 强制。

**根因 B（平台层）**：dispatch-spec.yaml 第 146 行明确规定"子 teammates 使用 bypassPermissions 模式"。bypassPermissions 可能导致 hook 不被执行，使得 ceremony-guard 对递归 spawn 的子蜂群完全失效。

## 结论

Hook 强制层要真正强制，必须同时满足两个前提：

1. **执行保障**：hook 被执行（不被 bypassPermissions 等平台机制绕过）
2. **阻断语义**：hook 输出 block 语义（`continue: false`），而非 warning 语义（`continue: true` + systemMessage）

缺少任一前提，hook 退化为纯文本指令，回到 016号的原始问题（知道规则 ≠ 执行规则）。

### 推论：结构工位的 hook+agent 混合模式

013号建立了结构工位概念（纯 agent 模型）。042号添加了 hook 强制层。两者之间存在一条此前未显式化的语法规则——结构工位的保障分为两层：

| 层 | 机制 | 职责 | 可 hook 化？ |
|----|------|------|-------------|
| 强制层 | hook | spawn 顺序、格式完整性、字段校验 | 是（必须 blocking） |
| 认知层 | agent | 谱系写入、张力检查、质量审查 | 否（需理解语义） |

ceremony-guard 的失败暴露了这条隐含规则：强制层必须同时满足执行保障和阻断语义。认知层不可替代，必须由 agent 执行。

## 定义依据

- 042号：hook 网络模式——"每条关键规则对应一个 hook，hook 读取 dispatch-spec 执行强制"
- 013号：结构工位必须先于任务工位 spawn（架构规则第 1 条）
- 071号：ceremony-guard 被标记为"✅ 已加固"（第 52 行）
- dispatch-spec.yaml 第 146 行：子 teammates 使用 bypassPermissions

## 边界条件

- 如果 Claude Code 平台保证 bypassPermissions 不跳过 hook 执行 → 根因 B 消失，只需修复根因 A（warning→block）
- 如果所有 hook 都已是 blocking 语义 → 本记录退化为仅关于 bypassPermissions 的平台约束记录
- 如果未来 agent 系统支持 per-agent 工具限制 → 认知层也可获得物理约束（类似 032号 Lead 权限剥夺）

## 下游推论

1. ceremony-guard.sh 需从 warning-only 升级为 blocking（`continue: false`）
2. 071号的 ceremony-guard 评级需从"✅ 已加固"降级为"⚠️ 部分加固"
3. 所有现有 hook 需审查：哪些是 blocking，哪些是 warning-only？warning-only 的 hook 不应被计入"已加固"
4. bypassPermissions 与 hook 网络的兼容性需要平台层确认
5. 结构工位保障的双层模型（hook 强制层 + agent 认知层）应写入 013号或 dispatch-spec 作为显式架构
6. **Gemini 3.1 Pro 架构决策**：dispatch-spec 需重构为 dispatch-dag，结构工位作为拓扑支配节点（Dominator Nodes）硬编码入 DAG 网络——结构工位不再是"先 spawn"的顺序约束，而是 DAG 中的支配节点，所有任务工位的路径必须经过结构工位。这将 013号的顺序约束提升为拓扑约束。（trace: orchestrator-proxy/decide | structural-station-architecture | 2026-02-21T00:30）

## 推导链

1. 013号：结构工位必须先于任务工位（语法规则）
2. 042号：hook 网络强制规则执行（语法记录）
3. 071号：ceremony-guard 被评为"已加固"（系统性扫描）
4. v11 实证：ceremony-guard 未能阻止结构工位跳过
5. 根因 A：ceremony-guard 是 warning-only（`continue: true`）
6. 根因 B：bypassPermissions 可能跳过 hook 执行
7. 本记录：hook 强制层需要双重前提（执行保障 + 阻断语义）
8. Gemini 决策：从顺序约束升级为拓扑约束（dispatch-spec → dispatch-dag）

## 谱系引用

- 042→071 演化链：hook 网络从建立到系统性扫描
- 070号：创世 Gap 工程实例（同构——文本指令对抗生成惯性失败）
- 016号：知道规则 ≠ 执行规则（hook 退化后回到的原始问题）
- 068号：缠论空间是偏序集/有向图（dispatch-dag 的理论基础）

## 影响声明

- 修正 071号对 ceremony-guard 的评级（✅→⚠️）
- 为 hook 网络引入"blocking vs warning-only"的显式区分
- 为结构工位引入"强制层 + 认知层"双层模型
- 触发下一轮 L2 重构方向：dispatch-spec → dispatch-dag（Gemini 决策）
- 不修改任何现有代码文件（代码修复由任务工位执行）
