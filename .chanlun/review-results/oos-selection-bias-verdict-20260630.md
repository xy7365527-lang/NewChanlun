# level0卖OOS阳性结果——选择偏差判定(PDF§6权威 + codex CLI环境问题记录)

## codex异质审计的两次失败(环境问题,非判定)
1. 第一次(#100 agent): codex exec带 `service_tier="fast"` -c `mcp_servers={}` 卡死(raw停124行,15:10无更新)。根因: service_tier="fast"排队 + MCP auth errors(OpenRouter→原装切换副作用)。
2. 第二次(Lead直接CLI,编排者"codex要用CLI换调用方式"): 去掉service_tier/mcp_servers后codex CLI**响应了**(基本测试返回OK),但在项目目录跑OOS审计时**注入了codex自己的记忆系统**(.codex/memories/MEMORY.md + .agents/skills加载失败),输出全是codex历史任务记忆([Task1][Task2]completeness-first等),未回答5问审计。
   根因: codex CLI在NewChanlun项目目录有记忆/skill自动加载,污染审计会话。修法: 在干净目录(非项目根)跑codex,或 -c 禁记忆加载(待查codex flag)。

## 审计目的已由第7份PDF《过拟合》§6权威达成(替代codex)
编排者拿level0卖阳性结果问ChatGPT,§6精确裁定(比codex本地审计更权威):

**选择偏差判定(codex Q2核心,PDF§6条件一)**:
> "如果你先看全样本,再发现z=level0卖是赢家,然后说'我要验证它',那么它仍然带有选择偏差。
>  若它是从很多类别中事后挑出来的,就要做多重比较修正或重新留出完全未见过的数据验证。
>  正确做法: 先在train中决定level0卖是否为可交易类别,再在OOS中验证。
>  如果level0卖是你事前理论指定的唯一主检验类别,则n=349已具备一定检验能力。"
> **"如果它是事后赢家,就不能直接把+34,684当真alpha。"**

⟹ 当前"全样本挑level0卖→切分验"**仍有选择偏差**(BIAS,非完全FATAL): 缓解因素是(level,δ)低基数+可论证level0卖是理论先验(顶背驰卖=缠论核心买卖点,非数据挖掘挑的)。但严格修复必须预注册→train挑类→OOS验。

**有效样本(PDF§6条件二)**: 必须用neff非原始n(收益重叠/时间相关neff=nraw/(1+2Σρk)),neff可能≪349。n=349 dmin≈0.133,切半n≈175 dmin≈0.188。

**正确验收(PDF§11)**: 预注册level0卖→train定规则→多窗口walk-forward(每train只用过去估μ/阈值,OOS只执行预定规则)→事件收益Xm=δ(Pτm−Ptm)−Cm→block bootstrap/HAC/cluster-by-time置信区间→报μ_OOS/se/LCB/p。OOS下仍显著正才可说初步alpha。

## 结论
- level0卖holdout +7.87e3阳性**不可直接当真alpha**(选择偏差未除,PDF§6明确)。
- 已用ACCEPTANCE_REPLACE把econpositive-OOS升级到PDF§11标准(预注册+walk-forward+neff+block bootstrap)。
- codex异质审计因CLI环境问题(记忆污染)未独立完成,但PDF§6(ChatGPT权威咨询)达成同等审计目的且更完整。
- 边界: codex CLI在项目目录的记忆加载问题待修(影响后续codex异质审计)。
