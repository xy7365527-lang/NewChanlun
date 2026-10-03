# Stage21：原重切输出与来源、外缘、时间证书接通

日期：2026-10-03，#1467，`RETILE-PACKET-FIDELITY-v1`。接续[开工卡](Stage21WorkCard.md)。本轮完成的是给定成员列表的重切数据输出，不是完整F₂。

## 结果

已定义并检查实际Packet生成器。它把原Retile分类得到的Cell、原成员、角色、位置跨度、真实外缘和可用时刻放在同一份输出中。一般定理保证其Cell序列逐项等于原Retile结果，全部成员和余段完整恢复输入，跨度满足Stage20的Cover，每份外缘满足ExactHull，可用时刻不早于输入触发参数或任一成员确认。

候选/保留角色由实际Local判据决定，不再由示例手填。旧NoCore块保留为原成员，非空待组装余段也确实进入输出。Stage14的三个局部候选仍不能构成严格上级，未被该数据接线改判为成功。

## 生成规则与四项一般保证

声明见[RetilePacketSpec.lean](RetilePacketSpec.lean)，证明见[RetilePacketProof.lean](RetilePacketProof.lean)。每个完整三成员块调用原`Retile.classify`；成功取candidate，失败取retained。余下1或2成员生成active Packet，空余段不生成Packet。

这里active指**本次重切尚不足三成员的待组装余段**。成员本身可能已确认，它不是底层DC构造器那个尚未确认的活动单元。candidate只表示通过原局部中枢谓词，不赋予完整走势或可上送身份。

| 一般目标 | 已证明的内容 |
|---|---|
| FIDELITY | 输出成员拼回原列表；位置从start连续覆盖到start+输入长度；提取的Cell列表等于原Retile输出；每Packet跨度长度等于其成员数，且成员非空 |
| PACKET | availableAt不早于触发参数和所有成员knownAt；存在实际外缘并满足ExactHull |
| ROLE | 完整块candidate当且仅当原Retile.Local成立；retained当且仅当该谓词不成立 |
| WITNESS | 接入旧合法DC/正量见证，检查NoCore保留、非空余段、准确范围及相切上级仍拒绝 |

FIDELITY与Stage20覆盖定理合用，保证所给源位置完整、保序且不重复。范围直接从Packet源成员的端价计算，未读取Stage19有0哨兵问题的Formal.Move.interval。

完整块保留原Cell中的knownAt，等于原stamp；余段时间为`max(触发参数,成员knownAt最大值)`。这是可用时间下界保证，不证明该参数就是实际首次触发，更不把保守的发布时间当作最早可知时刻。

## 具名见证

| 输入 | 生成结果 | 检查 |
|---|---|---|
| Stage8/13九成员失败样例 | candidate、candidate、retained；各3成员 | 全九成员原顺序恢复，三块availableAt均为10；第三块没有被丢弃或升格 |
| Stage14九成员相切样例 | 三个candidate | 外缘分别为[10040,10060]、[10030,10055]、[10030,10040]；原严格上级判定仍拒绝 |
| 旧合法历史的前五成员 | 一个3成员candidate与一个2成员待组装余段 | 余段真实出现在输出中，全部五成员恢复 |

五成员例检验通用组装函数处理非空余段的能力，不宣称五成员已满足九段触发条件。一般函数的定义域是给定记录列表，触发资格由调用者提供。

在任意人工记录上，Local失败也可能来自方向，不一概称NoCore；只有旧DC连续三单元的有效域可以使用Stage13的等价关系，把失败解释为没有严格核心。

## 证据与修订

精确根`RetilePacket.exact_root`类型为`FIDELITY_TARGET ∧ PACKET_TARGET ∧ ROLE_TARGET ∧ WITNESS_TARGET`。run `fc8e984887ab406f93e42575368bf071`，Lean4.31、exit0、machine/exact_root通过，公理仅propext/Classical.choice/Quot.sound，无sorryAx、自定公理、不安全或未知依赖。独立语义审查not_reviewed。

初始v1声明有无效的`SourceHull.Interval`导入别名，导致展开失败。v1a仅删除这个未使用的别名，定义与目标正文未改；[原始快照](RetilePacketSpec-v1-elaboration-failed.txt)和v1锁保留，当前源由[当前锁](retile-packet-spec-v1a.sha256)绑定。此后修复都在证明体，涉及展开和参数推断；诊断不当作数学反例。

完整通过证据保存在`/Users/silencehan/Documents/Codex/research-evidence/issue1467/retile-packet-v1a-checked/`，失败记录在`retile-packet-v1a/`。实际字节数、命令、freshness及公理见[机器摘要](stage21-lean-verification-summary.json)，源码/原文继承绑定见[源指纹](source-sha256-stage21.json)，复现和审查入口见[待审包](Stage21PendingReview.md)。没有修改formal或生产源、下载数据、重跑市场回放或进行确认实验。

## 仍缺的语义

一般定理保证给定记录的忠实输出，不自行保证记录来自真实行情、各成员已完成且同级、触发实际发生或角色具有完整走势身份。具名样例继续借旧Whole、visible与确认关系提供相应前提；推广仍须这些桥。

局部重切的数据输出在本轮已接齐。下一数学问题必须回到成功离开/回抽后的成员归属和走势完成条件，以及同级连接如何生成；不再以添加同类元数据引理替代该缺口。完整生命周期、上级生成关系的语义唯一性、递归闭合、独立审查及四臂增量价值均保持开放。两条F₁、原P1–P4和活动goal范围不变。
