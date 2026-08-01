## Question

真人在场的票（grilling / prototype）**开新会话时输什么**——要不要固化成模板？固化的话，模板管到哪一步为止？

**为什么现在才够锐**：这片原本是 [#767](https://github.com/xy7365527-lang/NewChanlun/issues/767) 迷雾段的一条（「HITL 票的开场 prompt 要不要固化」），注明「依赖并行纪律那张票」。两个依赖已清：

- [并行吞吐：HITL 瓶颈与 batch 化 #776](https://github.com/xy7365527-lang/NewChanlun/issues/776) 定了 grilling 票的默认入口 = `/batch-grill-me` 节奏 + `/domain-modeling` 落文档并调；
- [并行纪律：多会话同写一张图的冲突口径 #772](https://github.com/xy7365527-lang/NewChanlun/issues/772) 定了会话开头必须做「已关子票 ↔ 正文决策行」对账、认领是会话首写、写正文要压窗口 + 写前重比。

于是「一个走图会话的开头该发生什么」现在是**一串已裁定的具体动作**，不再是模糊意向——可以判断要不要把它固化了。

**对照**：Matt 用 handoff skill 自动写好 prompt 起子代理；本仓真人在场的票没法这么干，人得自己开会话、自己输第一句。

**待裁定**：

1. 要不要固化。（不固化 = 每次靠 `/wayfinder <图链接>` 这一句，其余全靠 skill 正文和本仓正本里的纪律自动生效；固化 = 另给一段可粘贴的开场文本。）
2. 若固化，管到哪：只管「加载图 + 对账 + 认领」这段机械开头，还是连「本票该怎么烤」的上下文一起带？
3. 载体：写进正本 `docs/agents/wayfinder-routing.md` 当一段可粘贴文本，还是做成本仓自己的 skill / slash 命令？（注意 [#767](https://github.com/xy7365527-lang/NewChanlun/issues/767) 已定前提：**不改 `~/.claude/skills/*` 本体**；本仓内新增载体不在此禁令内，但要论证是否值得。）
4. 无人值守票要不要同款模板——还是照 [#767](https://github.com/xy7365527-lang/NewChanlun/issues/767) Notes 已定的「本体在当前会话派子代理」走，不需要开场模板。
