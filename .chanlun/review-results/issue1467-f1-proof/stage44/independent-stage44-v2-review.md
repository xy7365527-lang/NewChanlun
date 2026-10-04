# #1467 Stage44 v2 限定修复复核

日期：2026-10-04。绑定 #1467/#1468。本审查者未参与作者稿和前审，现场完整读回两版订正、前审报告及当前汇总，并计算文件哈希。来源根为 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/`。下文 S2、S1、R、T 分别指 `stage44/ScopeCorrections-v2.md`、`stage44/ScopeCorrections-v1.md`、`stage44/independent-stage44-corrections-review.md`、`Stage44BaseCompletionAndB.md`。

## 结论

限定通过。S2:13、17 已补齐前审 R:29–37 指出的反例前提缺口。当前 BASE-CUT+ 可以作为继续实例化与检验的反例骨架，不再遗漏该历史上的 Above 为真条件。这只解除题面缺项，不证明 BASE-CUT+，不反驳 MAXΩ，也不证明 Ω 已冻结、合格 B 存在或 RT 已获资格。

当前汇总与所提供的旧汇总字节相比，唯一差异是 T:7 的订正链接从 `ScopeCorrections-v1.md` 改为 `ScopeCorrections-v2.md`。全文读回未发现借此提升结论的表述。

## 反例前件与时间边界

S1:11 写出候选资格及提前结束，但没有断言 `Above_B(K30,K20)`；S1:15 要求同一 Above 关系，不能替代该关系在具体反例上成立。此缺口经本审实际读回确认，不以 R 的结论代替判断。

S2:13 现在明确要求具名 `B∈C`、`h₀∈H_B`、B 在声明证明域上满足同一 `GoodΩ`，以及 h₀ 使用 MAXΩ 固定的同一有序核心目录。K20 是 U 的末自有核心，K30 是紧随其后的同级核心，并且 `Above_B(K30,K20)=true`。S2:17 又要求两边的 C、Ω、历史域、目录及 Above 解释相同，且资格、指定完成与 Above 为真的前提全部成立。历史域、目录、相邻关系和具体真值因此都已进入条件。

在四个平台各三事件的指定历史里，K30 的首个源事件为 e7。若上述条件确实有见证，则 MAXΩ 对此 B、h₀、U 的实例要求不得在 e7 前结束；BASE-CUT+ 给出的 `end=6` 与之冲突。`confirm=12` 是确认时间，不改变结束位置 6。S2:12 明写 MAX 比较结束边界，T:31–34 也将结束6与确认12分列，没有把确认滞后误作反例失效的理由。

S2:13 保留“标量排序只能在点外缘身份成立后替代 Above 证据”，S2:19 的负控制也准确。现场执行两个微型算术断言：`30>20` 为真而 `26>35` 为假；`6<7<12`。因此外缘为 `[19,35]`、`[26,34]` 时，标量上升不保证严格外缘分离。这只是检查所述逻辑区别，没有给这些区间补上合法历史或 GoodΩ 身份，也不是新的语义试验。

S2:17 的“只有……才……”仍是前提限定。上面的反例推导依赖 S2:13 的全部条件实际成立；不能将补齐条件本身读成已经构造了满足条件的见证。

## 范围是否保持

S2:5、15 明确说 Ω 尚未冻结，B0–B7 只是部分谓词待实例化的责任表。GoodΩ 必须独立定义，不能以正常返回或把 MAX 写入 Ω 充当资格。S2:11、17 将研究许可与资格分开，即使未来存在反例 B，也不能据此推出 `GoodΩ(RT)`。这些限制足以阻止本次修复被读成资格通过。

T:17、23 继续声明完整 Ω、GoodΩ(RT) 和 MAXΩ 真值未决；T:38、42 将有限候选与父层几何同原义完成区分；T:48、52、58 保留固定文件完整性、模型消费、四臂实验及生产采用的限制；T:60 未宣布 P1–P4 完成或关闭研究图。T:7 的“验证新关系”在这些限定下指 RT 定义内结果，没有被新链接扩大为原义资格。

本轮只读上述文件并作字节差异核查及两个微型逻辑检查。没有重跑1559次枚举、27项数据检查、192事件重放或 fixture 检查，没有重新认证其历史结果，没有重审全部原文、RT 实现、B 实现或历史理论。没有修改仓库、原稿或证据文件，没有提交、推送或外发；唯一新增文件为本仓外报告。

## 字节核对与 SHA-256

以下哈希现场计算。两份目标文件与派发的完整哈希一致。旧汇总只出现一次 `ScopeCorrections-v1.md`；将该字节串替换为 `ScopeCorrections-v2.md` 后，结果与当前 T 逐字节相同，包括其余文字、空白及换行。

| 文件 | SHA-256 |
|---|---|
| S2：stage44/ScopeCorrections-v2.md | `6d6efc677c2b0aad9a759f2ea5c35c2a5a626897568fc0c6a276668ff6868aa6` |
| T：Stage44BaseCompletionAndB.md | `df996cb667b7de83232bc93dd0471236b21f174be80d3d6493a5c0025f85c831` |
| S1：stage44/ScopeCorrections-v1.md | `3b3c31ea645671b2887d917babb41eb11aa1ddce76899bf6e8d5e988cab90380` |
| R：stage44/independent-stage44-corrections-review.md | `95e04da62ea454fee22a5d034296b1af0338258abb901b2eb272754eaf91b132` |
| 仓外旧汇总：/Users/silencehan/Documents/Codex/research-evidence/issue1467/base-completion-v1/stage44-summary-before-v2.md | `25ef99e79943f364507a7ecb1af1ba18674c4257bae379a55aed7517bd74ad0c` |

结论仅适用于这些字节版本。报告自身哈希另随交付消息提供。
