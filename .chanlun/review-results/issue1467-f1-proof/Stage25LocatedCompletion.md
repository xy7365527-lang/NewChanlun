# Stage25：程序确认标记与独立完成边界相符

日期：2026-10-04，#1467，`LOCATE-COMPLETED-v1`。接续[开工卡](Stage25WorkCard.md)，关闭Stage15/23保留的locate到Completed确认精化缺口。

## 已证明的结论

对任意模型事件前缀，原group/locate产生的已确认边界恰好满足原独立Completed定义：程序不误确认，也不漏掉定义内的完成边界。firstKnown等于start+1；knownAt非空时恰为finish+1；knownAt为空当且仅当该块结束于当前输入末尾。

进一步证明：两个历史只要截至t的模型事件相同，无论之后后缀和总长度如何不同，程序中knownAt≤t的已确认边界及确认序号一致。没有修改group、locate或Completed来适应证明。

## 独立语义与程序之间的桥

[LocatedSpec.lean](LocatedSpec.lean)的Span不调用程序。它要求：起止非空且不超出已见前缀；起点是流开始或变号位置；内部事件同号；终点是当前EOF或下一次变号位置。

[证明](LocatedProof.lean)先用既有Runs的源恢复、块内同号和相邻异号性质，把真实事件索引接到每个程序输出；再证明：

| 目标 | 内容 |
|---|---|
| PROGRAM | 每个输出满足Span与元数据关系；每个已见事件位置均被某输出区间覆盖 |
| MATCH | `Completed p n s e`当且仅当程序中存在start=s、finish=e、knownAt=some(e+1)的输出 |
| CAUSAL | 截止共同已见前缀，已确认边界不受不同后缀影响 |
| WITNESS | 首次反号前后、EOF尾块和分叉后缀的具体检查 |

反向证明依靠位置覆盖和独立极大同号区间的唯一性：若定义内完成区间与覆盖其起点的程序区间边界不同，就会同时要求相同位置同号与异号，发生矛盾。因此不是把“等于程序输出”重新命名成Completed。

本轮MATCH给出完成边界的等价存在性，通用CAUSAL针对边界与确认序号。没有把它扩大成所有真实行情对象、价格读出与发布记录的逐字段精化；那些还需结合源载荷、Stage24报价前缀和实际时钟。

## 确认时序与见证

沿Stage23六事件模型，程序knownAt为`[2,3,4,6,None]`。第一块在仅见第一个事件时未完成；第二个反号事件到达后才满足Completed。末块在第六事件处仍是EOF活动尾，不能因文件结束而确认。

另取两种不同后缀，一种继续正压力，另一种转负。对八事件全历史回算后，只保留knownAt≤6的程序输出，两份完整Located列表相同。新反号到达后才允许确认原活动块；例中另一后缀的[start=5,finish=6]块在序号7确认。

通用证明覆盖不同总长度，不仅这两个同长例。每个确认都必须由已见反号后继支持，EOF没有确认权。

## 结论的层次

Completed在这里是**事件符号块**的独立定义，不是完整缠论走势的完成判据。Stage23已表明一个确认符号块仍可能没有几何。几何可用、同级资格、真正走势完成及上递归准入仍须各自证书。

start/finish/knownAt使用模型事件位置与计数。真实消息的接收时间、快照迟到和断档恢复未由本轮自动验证；同一逻辑前缀何时真正可见仍是下一P2义务。暂不可知状态也不能换成“真实无报价”。

## 检查记录与接续

精确根`LocatedProof.exact_root : PROGRAM_TARGET ∧ MATCH_TARGET ∧ CAUSAL_TARGET ∧ WITNESS_TARGET`通过，run `e5f7bf9c2561494385ce50efad73b4cd`。Lean4.31、exit0，公理仅propext/Quot.sound，无sorryAx、自定公理、未知或不安全依赖。声明锁未变；首轮修复只涉及列表结合形式和当前工具链支持的证明写法，没有改变命题。

完整证据位于`/Users/silencehan/Documents/Codex/research-evidence/issue1467/located-v1-check2/`，失败记录保存在`located-v1/`。命令、freshness与体积见[机器摘要](stage25-lean-verification-summary.json)，输入绑定见[源指纹](source-sha256-stage25.json)，复现与审查入口见[待审包](Stage25PendingReview.md)。独立语义审查not_reviewed。

下一步应将这些模型边界接到真实可知时钟与恢复路径，而不是继续把符号确认当作完整Move。两条F₁、完整F₂与增量价值仍保持原验收，研究票及goal开放；没有修改formal/生产、取新数据或运行确认实验。
