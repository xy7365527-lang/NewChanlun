# 049 — 统一编排协议：语义事件总线

**类型**: 选择
**状态**: 已结算
**日期**: 2026-02-20
**negation_source**: heterogeneous（Gemini 编排者代理 decide + 人类编排者指导）
**negation_form**: synthesis（048号 Stop-Guard + 041号 Gemini 路由 + 037号递归蜂群 → 统一编排协议）
**前置**: 048-universal-stop-guard, 041-orchestrator-proxy, 037-recursive-swarm-fractal, 042-hook-network-pattern, 043-self-growth-loop
**关联**: 016-runtime-enforcement-layer, 035-math-tools-crystallization

## 现象

系统中存在多个独立的自动化机制（hook 网络、Gemini 路由、递归蜂群、谱系检查），但它们之间没有统一的编排层。Lead 需要手动判断何时拉蜂群、何时调用 Gemini、何时检查谱系。这导致"等待"模式反复出现。

## Gemini 决策

建立语义事件总线（Semantic Event Bus），将所有路由逻辑统一为基于意图的语义路由。

### 路由表

| 事件源 | 触发条件 | 路由目标 | 动作 | 优先级 |
|--------|---------|---------|------|--------|
| FileSystem | spec/theorems/* 变更 | Gemini | Math Verify | High |
| FileSystem | docs/chan_spec.md 变更 | Genealogist | Lineage Check | High |
| FileSystem | 新文件不在 Manifest | Manifest Guard | Block commit | High |
| Annotation | @proof-required | Gemini | Math Prove | Low (Async) |
| Process | Task Queue >= 2 独立任务 | Swarm Manager | Spawn 递归蜂群 | High |
| Process | Session Start | Meta Lead | Ceremony | Critical |
| Process | Session End | Pattern Detector | 模式提取 (043号) | Low (Async) |
| Tool | Gemini API Error | System | Fallback 到本地规则 | Critical |
| Tool | Build Failure 3x | Build Resolver | 自动修复 | Medium |

### 数学问题路由

三重触发机制：
1. 位置触发：spec/theorems/ 目录下文件变更
2. 语义标签触发：代码/文档中 @proof-required 或 @math-verify 标签
3. 提交意图触发：commit message 包含 [MATH] 前缀

### 孤岛判定

真孤岛（需连接）：新文件未注册 Manifest、定义变更 vs 谱系检查、Spec 修改 vs 代码同步、PR 创建后流程
有意边界（不连接）：部署后验证

## 推导链

1. 048号证明了 Stop-Guard 可以统一检查多个条件
2. 042号证明了 hook 网络可以覆盖多个事件
3. 但 hook 是 point-to-point 的，缺乏全局视图
4. 语义事件总线 = hook 网络的升级：从散点连接到总线架构
5. 缠论系统是递归的，开发过程也应是递归的——编排者需要全局视图

## 边界条件

- 递归深度限制：路由产生循环时第 3 层强制熔断
- 成本控制：Gemini 数学验证仅在明确标识时触发，不全量扫描
- Meta-Observer 监控 Token 消耗，防止误报风暴

## 被否定的方案

- 继续逐场景加 hook：O(N) 复杂度，无全局视图
- 纯 CLAUDE.md 规则：016号已证明文本层不足

## 待实现

1. [ ] 创建 spec/theorems/ 目录
2. [ ] 实现路由表的 runtime 版本（dispatch-spec.yaml 扩展或独立配置）
3. [ ] topology-guard 扩展：新文件 manifest 注册检查
4. [ ] chan_spec.md 变更监听
5. [ ] @proof-required 标签扫描

## 溯源

[新缠论]（工程架构，非旧缠论原有概念，符合同构性原则）
