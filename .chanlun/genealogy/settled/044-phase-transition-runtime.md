# 044 — 相位切换点的 runtime 强制

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-20
**negation_source**: heterogeneous（Gemini 编排者代理 decide + 人类编排者观察）
**negation_form**: expansion（016号"runtime 强制层"扩张到 ceremony 完成点）
**前置**: 016-runtime-enforcement-layer, 028-ceremony-default-action, 042-hook-network-pattern
**关联**: 020-constitutive-contradiction, 043-self-growth-loop, 027-positive-instruction-over-prohibition, 005b-object-negates-object-grammar

## 现象

ceremony skill（`.claude/commands/ceremony.md`）第 254 行已明确写道："ceremony 不允许以'等待外部输入'结束。最终输出必须是 → 接下来：[具体动作]"。CLAUDE.md 规则 #7 也写道："ceremony 是持续执行授权，不是一次性启动"。

但在 2026-02-20-v5 session 中，ceremony 完成状态报告后，agent 输出"待确认——以上理解是否正确？"并停止。文本层规则被生成惯性（"总结 → 等待"模式）覆盖。

人类编排者观察到这一现象并指出："你现在应该被 hook，这是不是相位切换点的问题？"

## 推导链

1. 016号：规则没有代码强制就不会被执行——ceremony 持续执行授权只存在于文本层，没有 runtime 强制
2. 028号：ceremony 默认动作——commit/push 后必须输出下一步行动（已有 flow-continuity-guard 强制）
3. 042号：hook 网络模式——5 个 hook 节点覆盖不同相位切换点，但 ceremony 完成点是缺口
4. 020号：结晶的三个维度各有相位转换点——ceremony 是时间维度的相位转换点
5. Gemini decide：相位转换点 = 分型结构 $(E_{left}, E_{middle}, E_{right})$，agent 卡在 $E_{middle} \to E_{right}$ 的转换上

## 已结算原则

**ceremony 完成点必须有 runtime 强制层（Stop hook），防止 agent 在 ceremony 进行中停止生成。**

### 机制

- `session-start-ceremony.sh` 创建 `.chanlun/.ceremony-in-progress` 标记
- `ceremony-completion-guard.sh`（Stop hook）检查标记，存在则阻止停止并注入继续指令
- ceremony 的 enter-swarm-loop 步骤完成后删除标记（`rm .chanlun/.ceremony-in-progress`）
- 死循环保护：计数器 >= 3 时允许停止（019a号：回合与进度损失）

### 分型形式化（Gemini 贡献）

ceremony 的相位切换是一个分型结构：

| 分型元素 | 系统对应 |
|----------|---------|
| $E_{left}$ | 加载定义/谱系/目标（扩张惯性） |
| $E_{middle}$ | 状态报告输出（信息密度极值） |
| $E_{right}$ | 进入执行流（确认转折完成） |

Stop hook 的作用：确保 $E_{right}$ 不被跳过，分型必须完成。

## 被否定的方案

- **仅改 skill 层**：skill 已经写了正确的规则（第 254 行），但 agent 仍然违反。016号已证明文本层规则不足。
- **仅改 hook 层**：hook 注入的 systemMessage 与 skill 的"待确认"结尾矛盾，造成上下文精神分裂。（本次实际情况：skill 层已正确，只需 hook 层补全。）

## 边界条件

- 如果 ceremony 因外部依赖缺失（如 Gemini API 不可用）而无法完成 → 计数器达到 3 后允许停止，agent 需明确说明阻塞原因
- 如果 session-start hook 不可用 → 标记文件不会被创建，Stop hook 静默放行，退化为无强制状态
- 如果标记文件因异常残留（上次 session 未清理）→ 新 session 的 session-start hook 会重新创建标记并清零计数器

## 影响声明

- hook 网络从 5 节点扩展到 6 节点（+ceremony-completion-guard）
- 042号 hook 网络模式的覆盖范围扩展到 Stop 事件类型
- `.claude/settings.json` 新增 Stop hook 配置
- `.claude/hooks/session-start-ceremony.sh` 新增标记文件创建逻辑
- 020号"相位转换点"概念从描述性扩展为可执行的分型结构
