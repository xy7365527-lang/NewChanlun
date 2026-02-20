# 过程监督粒度细化设计

**状态**: 设计稿
**日期**: 2026-02-20
**前置**: 055-double-helix-architecture, 042-hook-network-pattern, 049-unified-orchestration-protocol
**关联**: 028-flow-continuity, 043-self-growth-loop, 037-recursive-swarm-fractal

## 1. 问题陈述

055号谱系要求"过程监督"——每步奖励，非仅最终结果。当前 hook 网络是**事件级**（工具调用前后触发），而非**步骤级**（语义操作的每个阶段触发验证）。

差距不在 hook 数量，在于粒度：一个 `git commit` 是一个事件，但 commit 之前的"定义变更→谱系更新→质量检查→Gemini 验证"是四个步骤。当前只有最后一步（commit）被拦截验证。

## 2. 当前粒度 vs 目标粒度

| 语义操作 | 当前覆盖 | 当前粒度 | 目标粒度 | 差距 |
|----------|---------|---------|---------|------|
| **git commit** | `double-helix-verify.sh` (Pre) + `flow-continuity-guard.sh` (Post) + `crystallization-guard.sh` (Post) | 事件级：commit 整体放行/拦截 | 步骤级：staged diff 按文件类型分流验证 | 定义文件/谱系文件/代码文件应有不同验证路径 |
| **Task spawn** | `ceremony-guard.sh` (Pre) | 事件级：检查结构工位是否就绪 | 步骤级：spawn 前验证任务描述与 dispatch-spec 一致性 | 缺少任务描述 vs spec 的语义匹配 |
| **Task complete** | `recursive-guard.sh` (Post) | 事件级：检查函数行数 | 步骤级：完成时验证产出是否满足结果包六要素 | 只检查代码质量，不检查概念完整性 |
| **定义文件写入** | 无 | 无覆盖 | 步骤级：写入前验证版本递增 + 谱系引用 | **完全缺失** |
| **谱系文件写入** | 无 | 无覆盖 | 步骤级：写入前验证强制字段 + 前置/关联链接有效性 | **完全缺失** |
| **Session 结束** | `ceremony-completion-guard.sh` (Stop) + `post-session-pattern-detect.sh` (Stop) | 事件级：检查是否有未完成工作 | 当前已足够 | 无差距 |
| **Session 开始** | `session-start-ceremony.sh` (SessionStart) | 事件级：热/冷启动 | 当前已足够 | 无差距 |
| **Context 压缩** | `precompact-save.sh` (PreCompact) | 事件级：保存快照 | 当前已足够 | 无差距 |

## 3. 需要新增的 hook

只列出真正有差距的点。Session 生命周期（Start/Stop/PreCompact）已经足够，不需要细化。

### P0: 定义文件写入守卫（definition-write-guard.sh）

- **hook 类型**: PreToolUse(Write) + PreToolUse(Edit)
- **触发条件**: 目标路径匹配 `.chanlun/definitions/*.md`
- **验证步骤**:
  1. 版本号是否递增（不允许降级或不变）
  2. `**状态**` 字段是否合法（生成态/已结算）
  3. 如果从生成态→已结算，检查是否有对应的 settled 谱系
- **失败动作**: block + 注入具体缺失项
- **熔断**: 连续 block 3 次同一文件 → 放行 + 警告
- **理由**: 定义是系统的根基。当前定义文件可以被任意写入，没有任何运行时约束。这是最大的监督盲区。

### P1: 谱系文件写入守卫（genealogy-write-guard.sh）

- **hook 类型**: PreToolUse(Write) + PreToolUse(Edit)
- **触发条件**: 目标路径匹配 `.chanlun/genealogy/**/*.md`
- **验证步骤**:
  1. 强制字段存在性：类型、状态、日期、前置、关联
  2. `前置` 中引用的谱系编号是否存在于 `settled/` 或 `pending/`
  3. 编号是否与文件名一致（如 `055-xxx.md` 的内容中应有 055 相关标识）
- **失败动作**: block + 列出缺失字段
- **熔断**: 同上
- **理由**: 谱系是发现引擎（012号）。强制字段不完整的谱系无法被下游质询，等于无效产出。

### P2: commit 分流验证（double-helix-v2 增强）

在现有 `double-helix-verify.sh` 基础上增强，不新建 hook。

- **当前**: 将整个 staged diff 发给 Gemini 做统一验证
- **增强**: 按文件类型分流，构建不同的验证 prompt
  - `definitions/*.md` 变更 → 验证 prompt 聚焦"定义一致性 + 版本递增"
  - `genealogy/**/*.md` 变更 → 验证 prompt 聚焦"推导链完整性 + 前置有效性"
  - `src/**` 代码变更 → 验证 prompt 聚焦"与 spec 一致性 + 054号同一存在论"
  - 混合变更 → 分段验证，合并结果
- **理由**: 统一 prompt 导致 Gemini 验证精度低（500行 diff 中定义变更可能被淹没）。分流后每类变更获得针对性验证。

### P3: Task 完成时结果包检查（result-package-guard.sh）

- **hook 类型**: PostToolUse(TaskUpdate)
- **触发条件**: status == "completed" 且任务描述包含概念关键词（定义/谱系/定理/原则）
- **验证步骤**:
  1. 扫描 task 的最终输出（SendMessage 内容）
  2. 检查是否包含结果包六要素的关键标记词
  3. 纯技术任务（关键词：重构/格式/构建/lint）只检查简化版三要素
- **失败动作**: 注入 systemMessage 要求补全（不 block，因为 task 已完成）
- **熔断**: 无需（只是提醒，不阻断）
- **理由**: 结果包格式是强制的（result-package.md），但当前没有运行时检查。quality-guard 工位做人工检查，但覆盖率不稳定。

## 4. 不需要细化的点

| 操作 | 理由 |
|------|------|
| SessionStart | `session-start-ceremony.sh` 已实现热/冷启动自动化，粒度足够 |
| PreCompact | `precompact-save.sh` 已实现状态快照，粒度足够 |
| Stop | `ceremony-completion-guard.sh` 已实现多维扫描（ceremony/蜂群/谱系/@proof），粒度足够 |
| pattern-detect | `post-session-pattern-detect.sh` 已实现跨 session 合并，粒度足够 |
| topology-guard | 拓扑守卫已覆盖工位扩张/收缩信号，粒度足够 |
| git push | 不需要额外守卫——commit 已被双螺旋拦截，push 是 commit 的下游 |

## 5. 与双螺旋 hook 的关系

```
                    写入时守卫（P0/P1）          提交时守卫（P2）
                    ┌─────────────┐            ┌─────────────┐
                    │ definition- │            │ double-helix│
Write/Edit ──────→ │ write-guard │            │ -verify v2  │
(definitions/)      │ genealogy-  │  ──commit──→│ (分流验证)   │──→ allow/block
                    │ write-guard │            │             │
                    └─────────────┘            └─────────────┘
                    即时、本地、轻量              批量、Gemini、重量

                    完成时守卫（P3）
                    ┌─────────────┐
TaskUpdate ───────→ │ result-pkg  │
(completed)         │ -guard      │──→ systemMessage（补全提醒）
                    └─────────────┘
```

P0/P1 是**前置过滤器**：在文件写入时就拦截明显的格式/字段错误，不需要等到 commit。
P2 是**批量验证器**：在 commit 时做语义级交叉验证（需要 Gemini）。
P3 是**产出检查器**：在任务完成时检查概念完整性。

三层形成递进关系：写入时 → 提交时 → 完成时。每层只检查前层未覆盖的内容。

## 6. 实现优先级

| 优先级 | hook | 工作量 | 理由 |
|--------|------|--------|------|
| **P0** | definition-write-guard.sh | 小（纯本地字段检查） | 定义是根基，当前完全无守卫 |
| **P1** | genealogy-write-guard.sh | 小（纯本地字段检查） | 谱系是发现引擎，强制字段无运行时保障 |
| **P2** | double-helix-verify.sh 增强 | 中（修改现有 hook，分流逻辑） | 提升 Gemini 验证精度 |
| **P3** | result-package-guard.sh | 小（PostToolUse 注入） | 结果包完整性，非阻断性 |

P0 和 P1 可并行实现（无依赖）。P2 依赖 P0/P1 先就位（否则分流验证会重复检查已被前置过滤的内容）。P3 独立于前三者。

## 7. settings.json 变更预览

```json
{
  "PreToolUse": [
    // 现有
    { "matcher": "Task", "hooks": [{ "command": "ceremony-guard.sh" }] },
    { "matcher": "Bash", "hooks": [{ "command": "double-helix-verify.sh" }] },
    // 新增 P0+P1
    { "matcher": "Write", "hooks": [{ "command": "definition-write-guard.sh" }] },
    { "matcher": "Edit",  "hooks": [{ "command": "definition-write-guard.sh" }] },
    { "matcher": "Write", "hooks": [{ "command": "genealogy-write-guard.sh" }] },
    { "matcher": "Edit",  "hooks": [{ "command": "genealogy-write-guard.sh" }] }
  ],
  "PostToolUse": [
    // 现有
    { "matcher": "TaskUpdate", "hooks": [{ "command": "recursive-guard.sh" }] },
    { "matcher": "Bash", "hooks": [{ "command": "flow-continuity-guard.sh" }] },
    { "matcher": "Bash", "hooks": [{ "command": "crystallization-guard.sh" }] },
    { "matcher": "Bash", "hooks": [{ "command": "topology-guard.sh" }] },
    // 新增 P3
    { "matcher": "TaskUpdate", "hooks": [{ "command": "result-package-guard.sh" }] }
  ]
}
```

## 8. 边界条件

- 如果 Write/Edit hook 的性能开销导致写入延迟 > 500ms，降级为异步检查（PostToolUse 而非 PreToolUse）
- 如果 Gemini 分流验证的 token 消耗超过单次验证的 2 倍，回退到统一 prompt
- 如果 definition-write-guard 的误报率 > 10%（合法写入被 block），放宽字段检查规则
- P0/P1 的 hook 脚本内**不调用 Gemini**（纯本地检查），避免写入路径上的网络延迟

## 9. 影响声明

- 新增 2 个 hook 脚本（P0: definition-write-guard.sh, P1: genealogy-write-guard.sh）
- 新增 1 个 hook 脚本（P3: result-package-guard.sh）
- 修改 1 个现有 hook（P2: double-helix-verify.sh 增加分流逻辑）
- 修改 settings.json 的 hooks 配置（新增 PreToolUse[Write], PreToolUse[Edit], PostToolUse[TaskUpdate] 条目）
- 不影响现有 hook 的行为（P0/P1/P3 是新增，P2 是增强）
