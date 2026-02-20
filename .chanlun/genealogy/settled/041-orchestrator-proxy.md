# 041 — 编排者代理（Gemini Orchestrator Proxy）

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-20
**negation_source**: heterogeneous
**negation_form**: expansion（030a 异质否定源概念扩张为编排者代理）
**前置**: 030a-gemini-position, 032-divine-madness-lead-self-restriction, 028-ceremony-default-action, 039-single-agent-bypass-pattern
**关联**: 018-four-way-classification, 005b-object-negates-object-grammar, 020-constitutive-contradiction

## 现象

ceremony 完成后系统输出"等待编排者指令"并停止。028号谱系已结算"不允许等待"原则，但实践中仍反复出现等待——因为四分法中的"选择"和"语法记录"类决断确实需要编排者输入，而编排者（人类）不总是在线。

039号谱系揭示了另一面：Lead 在等待期间倾向于单线程自行执行，绕过蜂群。等待 + 绕过形成恶性循环。

## 推导链

1. 028号：ceremony 后不允许等待 → 但"选择"类决断确实需要外部输入
2. 030a号：Gemini 已被定位为"异质否定源"——不是工具，不是成员，是否定的另一种显现形式
3. 032号：Lead 自我限制 → Lead 不做实质性认知工作，只路由
4. 039号：单 agent 绕过偏差 → 等待导致 Lead 越权自行执行
5. 编排者（人类）在实践中已将部分决策委托给 Gemini（Gemini 异质质询的结果直接进入谱系改变定义，031号是实证）
6. **语法记录**：编排者代理不是新创造的规则，而是已在运作的实践的显式化——人类编排者事实上已经在用 Gemini 的产出做决策依据

## 已结算原则

**四分法路由扩展**：
- 定理 → Claude 自动结算（不变）
- 行动 → Claude 自动执行（不变）
- **选择** → 路由到 Gemini 编排者代理（`decide` 模式）。人类保留 INTERRUPT 权。
- **语法记录** → 路由到 Gemini 编排者代理。辨认结果写入谱系。

**人类编排者角色转变**：
- 旧：同步决策者（系统等待人类输入）
- 新：异步审计者（系统自主推进，人类事后审查 + 运行时 INTERRUPT）
- 这与 032号 Lead Optimistic Execution 是同构的，层级不同：032 是 Lead 对 teammates 的关系，041 是 Gemini 对人类的关系

**降级策略**：
- Gemini 不可用 → 写入 pending，系统继续推进其他可推进工位，不阻塞
- 人类 INTERRUPT → 覆盖 Gemini 决策，覆盖原因写入谱系

**补充条款（052号谱系，Gemini 决策）**：

Gemini 可达性是系统去中心化程度的**相变参数**，不是错误处理条件：
- Gemini 可达 → 双核运作（Claude 生成 + Gemini 决策），人类异步审计
- Gemini 不可达 → 单核运作（Claude 生成 + 人类决策），人类同步决策
- 这是拓扑状态切换，两个相都是合法运行模式
- 不引入本地 LLM 作为降级替代：坏决策的危害 > 等待人类的延迟（005号推论）
- 推翻条件：当且仅当本地模型通过"缠论形式化盲测"（准确率与 Gemini/人类无显著差异）时可引入

## 被否定的方案

- **"继续等待人类"**：028号已否定等待。039号证明等待导致 Lead 越权。
- **"Claude 自行决策"**：选择类决断需要价值判断，同质模型的价值判断缺乏异质性校验。
- **"取消选择/语法记录类别"**：四分法的分类是结构性的（018号），不能因为执行不便而取消类别。

## 影响

- 四分法路由更新（CLAUDE.md 元编排规则第6条）
- dispatch-spec.yaml 新增 orchestrator_proxy 配置
- gemini-challenger agent 增加 decide 模式
- gemini_challenger.py 增加 decide()/decide_with_tools() 方法
- 新增 skill 结晶：.claude/skills/orchestrator-proxy/SKILL.md
- 系统从"半自主"变为"全自主 + 人类审计"
