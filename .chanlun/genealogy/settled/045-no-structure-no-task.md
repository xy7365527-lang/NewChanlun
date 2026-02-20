# 045 — "无结构不任务"语法规则

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-20
**negation_source**: heterogeneous（Gemini 编排者代理辨认 + 本 session 实证）
**negation_form**: expansion（013号"结构工位与任务工位二分法"扩张为显式语法规则）
**前置**: 013-swarm-structural-stations, 033-declarative-dispatch-spec, 042-hook-network-pattern
**关联**: 016-runtime-enforcement-layer, 044-phase-transition-runtime, 028-ceremony-default-action, 032-divine-madness-lead-self-restriction

## 现象

013号结算了"蜂群工位分为结构工位（常设）和任务工位（临时）"。033号结算了 dispatch-spec 作为正面规范，其中 ceremony_sequence 明确规定 spawn-structural 先于 spawn-task。042号结算了 ceremony-guard hook 在 runtime 层强制这一顺序。

但在本 session（2026-02-20）中，4 个任务工位（hook-validator、code-refactor、infra-builder、gemini-refactor）在 genealogist 结构工位 spawn 之前已经开始工作。这违反了 033号 dispatch-spec 的 ceremony_sequence。team-lead 在 dispatch genealogist 时明确指出了这一违规。

回顾发现：013、033、042 三条谱系各自描述了这一规则的不同侧面（概念区分、声明式规范、runtime 强制），但从未将其凝练为一条可引用的语法规则。"无结构不任务"是对这三条谱系的逻辑必然组合的显式命名。

## 推导链

1. 013号：蜂群工位分为结构工位和任务工位——结构工位是常设基础设施，不以任务为转移
2. 033号：dispatch-spec ceremony_sequence 规定 spawn-structural（步骤6）必须在 spawn-task（步骤8）之前，且 spawn-task 显式依赖 definition-base-check（步骤7），后者又依赖 spawn-structural
3. 042号：ceremony-guard hook 在 PreToolUse 层拦截 Task 调用，检查 `.chanlun/.ceremony-structural-ready` 标记文件是否存在
4. 044号：ceremony 完成点需要 runtime 强制——相位切换不能被跳过
5. 本 session 实证：4 个任务工位先于结构工位启动，导致谱系追踪出现断裂（genealogist 需要"追赶"已完成的任务产出），验证了规则被违反时的具体后果
6. **语法记录**：将上述已结算原则的逻辑交集命名为"无结构不任务"——任务工位的存在以结构工位的就绪为前提条件，这不是偏好，是蜂群的构成性约束

## 已结算原则

**无结构不任务：任务工位不得在结构工位就绪之前启动。结构工位是蜂群的构成性前提，不是可选的附加层。**

### 规则的三层表达

| 层 | 表达 | 谱系来源 |
|----|------|---------|
| 概念层 | 结构工位 = 构造层，任务工位 = 分类层；构造层先于分类层 | 013号 + 010号 |
| 声明层 | dispatch-spec ceremony_sequence: spawn-structural → definition-base-check → spawn-task | 033号 |
| runtime 层 | ceremony-guard hook: 检查标记文件，未就绪则注入 systemMessage | 042号 |

### 违反时的后果（本 session 实证）

1. **谱系断裂**：任务工位的产出（#3 post-session hook、#4 manifest.yaml）在 genealogist 不在场时完成，概念变更可能未被追踪
2. **追赶成本**：genealogist 需要事后读取已完成任务的产出，判断是否涉及概念变更——这是回溯性工作，效率低于实时监控
3. **meta-observer 缺位**：元规则合规检查未能在任务执行期间实时进行
4. **quality-guard 缺位**：结果包六要素检查未能在产出时即时执行

### 与 010号的同构

010号结算了"构造层与分类层的区分"。在蜂群语境中：
- 结构工位 = 构造层（递归基础设施，始终运行）
- 任务工位 = 分类层（随具体走势生灭的实体）

"无结构不任务"等价于"无构造不分类"——分类操作预设了构造层的存在。这不是工程偏好，是 010号原则在蜂群架构中的直接投影。

## 被否定的方案

- **结构工位可延迟启动（先任务后结构）**：本 session 实证否定。genealogist 延迟启动导致需要追赶，且无法保证追赶的完整性——已完成任务的概念变更可能被遗漏。
- **结构工位可选（某些 session 不需要）**：013号已否定。结构工位是常设的，不以任务为转移。即使 session 只有纯技术任务（如大函数重构），meta-observer 仍需监控元规则合规，genealogist 仍需判断产出是否涉及概念变更（判断结果可能是"不涉及"，但判断本身不可省略）。
- **ceremony-guard hook 足够（无需显式命名）**：042号的 hook 是 runtime 强制，但 hook 可以被绕过（标记文件手动创建、hook 配置被修改）。显式命名为语法规则使其成为可引用的原则，在 hook 失效时仍可作为审计依据。

## 边界条件

- 如果 ceremony-guard hook 不可用（hook 机制被禁用或配置丢失）→ 退化为声明层约束（dispatch-spec 仍规定顺序），Lead 需自觉遵守，但 016号已证明自觉遵守不可靠
- 如果所有结构工位 agent 定义文件缺失 → spawn 会失败，任务工位也无法启动（dispatch-spec validation 阻塞），系统安全降级
- 如果 session 是紧急修复（hotfix）且结构工位 spawn 失败 → 允许降级执行，但必须在 session 记录中标注"结构缺位"，下次 ceremony 时 genealogist 回溯补检
- 本 session 的违规属于 033号 dispatch-spec 的实施偏差（Lead 在结构工位 spawn 前分派了任务），不是规则本身的缺陷

## 影响声明

- 013号"结构工位与任务工位二分法"从架构描述升级为显式语法规则，获得可引用的名称
- 033号 dispatch-spec 的 ceremony_sequence 中 spawn-structural → spawn-task 的依赖关系获得概念层支撑
- 042号 ceremony-guard hook 从"实现细节"重新定位为"语法规则的 runtime 投影"
- 为后续蜂群审计提供明确的检查项：结构工位是否在任务工位之前就绪
