# Stage19：现有递归资格关系不能单独保证来源和区间忠实

日期：2026-10-03，#1467，`FORMAL-COMPOSE-FAITHFULNESS-v1`。本轮审计当前研究树的`formal/Formal/RecursiveConstruction.lean`，不修改正式定义或生产实现。接续[开工卡](Stage19WorkCard.md)。

## 结论

两个继承风险已取得精确Lean反例：

1. 现有`MovesComposedFrom`接受重叠窗口见证。起点严格递增和总数量约束，不能保证具体原单元不被重复消费。该类反例可缩为六个原单元、两个上级对象，并证明六是现有关系中至少两个上级对象所允许的最小输入长度。
2. 现有`Move.interval`把正价复合走势的下界扩到0。三个子对象自身中枢的外缘都正确，区间读出仍产生错误范围；这些范围足以使下一层`WindowOverlap`及`WellFormed`通过，而同一源的真实外缘被实际Origin严格谓词拒绝。

这些结果限制的是两个具名形式化接口的语义保证。Lean检查的是实际写下的定义；现有定理在这些定义内仍可成立。本轮没有证明生产Rust会重复消费或使用这个0下界，也没有否定全部形式化结果。

## 连接问题为什么需要这次审计

本轮重新核查正本博文：第18课36行正文给出至少三段次级走势的构成约束；第49课138行是带“缠中说禅”作者标记的答疑，正文界为78行，相关语句无行首或行内注者文字。第18课50–52行讨论趋势中连接两个中枢的更低级走势，第49课134/138行讨论扩展构造中的次级别成员，不能仅凭都叫“连接”就赋予相同资格。第18课24/50行有行内编注，本轮不把那些注释作作者依据。

Stage6已证单成员包装不能满足现有WellFormed。本轮继续问：即使取得WellFormed和MovesComposedFrom，来源与几何是否足以支持递归资格？答案仍是否定的，缺口具体如下。

## 来源反例与最短性

`RecursiveConstruction.lean:200`的注释宣称窗口不重叠，实际关系`:210`只要求：上级非空、各上级WellFormed、数量`3×上级数≤下级数`、窗口起点严格递增、每窗在界内且确实封装其切片。它没有要求前窗结束不晚于后窗起点。

[ComposeAuditSpec.lean](ComposeAuditSpec.lean)使用Stage12已证明的完整正量DC路径，逐单元把方向与真实端价外缘映射到该正式Move载体；底层观察规则没有变化。9条原单元上取起点0、1、2，窗口各长3。每个上级都调用原`windowCenters`和原单窗口良构引理，源位置消费序列却为：

```text
[0,1,2, 1,2,3, 2,3,4]
```

位置1/2/3被重复消费，5/6/7/8不在这些窗口里，现有MovesComposedFrom仍成立。这里的遗漏是**相对于所给封装窗口没有消费**，不是原始历史文件丢失；原关系没有保存残留归属的输出字段。

独立补充声明[ComposeMinimalSpec.lean](ComposeMinimalSpec.lean)保持原两项声明不动，将同一合法历史的前六条已确认单元作为输入，只取起点0、1两个窗口：

| 项目 | 最短见证 |
|---|---|
| 原单元数 / 上级数 | 6 / 2 |
| 实际消费位置 | [0,1,2,1,2,3] |
| 重复位置 | 1、2 |
| 未被窗口消费的位置 | 4、5 |
| 原MovesComposedFrom / 每个UpperMoveSound | 均成立 |

一般下界定理从原关系本身的数量约束推出：对任意上下级列表和级号，只要上级数至少2且MovesComposedFrom成立，输入长度至少6。因此此六单元见证在“至少两个上级、检验该关系是否禁止重叠”这一类内最短，不宣称其他反例类型的全局最短性。

**没有据此打回canonicalWindows算法。**同一根另外检查9条输入的规范窗口起点确为`[0,3,6]`。规范算法可以具有强于其返回关系的性质；本反例否定的是只拿该关系就断言所给窗口不重叠的做法，也不把一个重叠见证自动升级为对所有其他可能见证的结论。

## 区间反例与下一层虚假通过

正式定义`:48–53`对复合Move取`centers.dd.foldr min 0`和`centers.gg.foldr max 0`。因此这些正价外缘即使非空，下界仍取到0。使用Stage14的完整合法DC/正量见证，按固定三段形成三个WellFormed对象：

| 子对象 | 由自己成员派生的中枢外缘 | 正式Move.interval输出 |
|---|---|---|
| 0 | [10040,10060] | [0,10060] |
| 1 | [10030,10055] | [0,10055] |
| 2 | [10030,10040] | [0,10040] |

真实外缘共同交集只为10040单点，原Origin严格谓词拒绝；经过这个区间读出后，正式WindowOverlap通过，并生成核心`[0,10040]`的下一层对象。该下一层对象还满足原WellFormed，而0并非这条正价源路径的实际下界。旧核心重算、方向与DC合法性反例证据均进入根，没有用手填中心表或虚构无效原单元作为前提。

这说明“中枢由子成员派生”和“每个节点WellFormed”不足以保证跨层读出保持真实价格范围。此处是实际正式函数的数值反例；未实现对全部Rust消费者的精化或运行时调查。

## 复用处置及受影响入口

| 入口 | 本轮处理 |
|---|---|
| `RecursiveConstruction.lean:210` MovesComposedFrom | 保留其实际非空、层级、数量及窗口见证保证；暂停把它单独作为不重复消费或源完整归属证明 |
| 同文件`:48` Move.interval，`:245` WindowOverlap | 暂停继承其复合对象价格区间作为真实外缘；局部基本单元的直接区间不因此全部否定 |
| 同文件`:417` window_wellFormed | 定理的实际结论保留；本轮已展示它不能替代跨层区间忠实性 |
| `Formal/RStarNonSpecial.lean:81` GeneratedStep | 实际引用MovesComposedFrom，相关来源保证待审；未证明其全部其他前提下也能触发本见证 |
| `Foundation/ChanlunInstantiation.lean:136`规范compose流程 | 使用规范窗口，应区分算法自身性质与弱关系；本轮不声称规范窗口重叠 |
| `Formal/DivergenceNesting.lean:431` currentExtreme | 读到同类0哨兵表达式，只登记后续线索；本轮没有构造该入口的反例或运行其消费者 |

Stage6的单成员否定、Stage14的严格上级反例、Stage18在明示不重叠来源关系下的重分组否定均保留。Stage18没有使用MovesComposedFrom来偷偷假定不重叠，故这次发现不推翻它。

后续候选资格证书须分别证明：窗口结束与下个起点的实际关系；已消费、连接、NoCore与活动尾部的完整去向；非空成员真实外缘的精确极值；同级与完成依据。完成这两项修正仍不等于完整F₂，尤其不能将同级连接资格由一个level字段替代。当前研究不去修整套旧正式引擎，相关问题先作为继承限制落本票。

## 证据与复现

两份声明各在证明前冻结，锁均未变。主根`ComposeAudit.exact_root : OVERLAP_TARGET ∧ RANGE_TARGET`在run `e0fff3aa5da843c3a33719390e0b6d82`通过；最短性根`ComposeMinimal.exact_root : WITNESS_TARGET ∧ LOWER_BOUND_TARGET`在run `6ba44fc8a81949c486cb4c8aab1743a5`通过。Lean4.31、exit0，传递公理仅propext/Quot.sound，无sorryAx、自定公理、未知或不安全依赖。独立语义审查仍为not_reviewed。

主根首轮失败为证明展开、空列表成员化简和StrictlyIncreasing的Decidable展开问题；修复只改证明体，未改有效域、目标或正式源。最短性补充单独冻结，不替换或削弱主根。正式模块只运行已有固定工具链的具名构建，完成4 jobs；`formal/`源码没有变更，不需fixture漂移检查。

完整运行保存在仓外`/Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-audit-v1-checked/`和`compose-minimal-v1/`；失败记录另在`compose-audit-v1/`，两个小项目源与仓内逐字一致。[摘要](stage19-lean-verification-summary.json)记录命令、退出码、公理、freshness与实际字节数，[源绑定](source-sha256-stage19.json)记录当前输入。复现命令及独立审查问题见[Stage19PendingReview.md](Stage19PendingReview.md)。本轮不重跑市场样本、不下载、不作确认实验或生产替换，研究票与goal保持开放。
