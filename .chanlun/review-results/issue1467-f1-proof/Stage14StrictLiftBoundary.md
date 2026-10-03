# Stage14：三个合法子窗仍不足以自动产生严格上级核心

日期：2026-10-03，#1467，命题 `DC-ALL-CHILDREN-LIFT-v1`。承接 [开工卡](Stage14WorkCard.md)。名分：当前DC候选与研究吸收/重切规则内的限定否定结果，不是缠论教义裁定或所有订单递归的不可能定理。

## 结论

**即使九个DC单元都属于合法完整分解，按已有回入规则进入待重切，全部成员与初始核心闭区间相交，固定重切的三个子窗也全部通过局部判据，仍不能保证三个完整外缘形成严格非空的上级核心。**

本轮给出集成反例：三个外缘的共同交集只有单点10040。它不是 Stage8 那种某个子窗没有核心的失败；本例三个子窗都有严格核心，后续一层才失败。因此“所有子窗通过 → 自动封装上送并保证上级成立”的充分性假设被否定。

触发重切、子窗局部成立、上级几何成立、完整走势完成仍须分别证明。没有据此放宽单点规则、删除相切输入或修改原研究目标。

## 同一份合法输入贯穿全部步骤

固定 bid=10000、ask=10200、卖量10000，由正买量通过旧精确投影得到：

`10040,10060,10040,10055,10030,10040,10035,10040,10030,10040,10036`，δ=2。

同一Lean根证明：

- 整个十一观察满足原 `DCSpec.Whole`，有九条已确认单元及活动尾部；
- 每个观察都来自正数量及精确量化，未借负量、浮点误差或零波动单元；
- 初始seed核心为 `[10040,10055]`；三次下行离开后回抽触及10040，按旧 `CenterFrame.Absorb` 完成 `3→5→7→9`，最终进入 `retilePending`；
- 最终成员恰等于观察10的真实可见列表；全部成员与母核心按闭区间口径相交；
- 旧 `Retile` 固定三段处理后无尾部、三个子窗全部通过，数值如下。

| 子窗 | 自身严格核心 ZD/ZG | 自身完整外缘 DD/GG | 原跨度端价 |
|---|---|---|---|
| 0–2 | [10040,10055] | [10040,10060] | 10040→10055，向上 |
| 3–5 | [10035,10040] | [10030,10055] | 10055→10035，向下 |
| 6–8 | [10035,10040] | [10030,10040] | 10035→10040，向上 |

三个跨度的净端点方向严格交替，失败不依赖方向标错。计算上级使用的是**完整外缘**，不是误用子核心：

`上级ZD = max(10040,10030,10030) = 10040`；

`上级ZG = min(10060,10055,10040) = 10040`。

这仅是单点交集。`origin_rejection`进一步对**任何具有上述三个范围的 Segment 载体**证明实际 `Origin.CenterConfirmedComplete` 不成立，与其方向字段或源索引如何填写无关。没有把 `CenterFull` 载体的弱 `valid` 字段当作严格判据。

## 缺的具体条件

同时证明一个一般区间关系：三段各自非退化时，严格三重交集等价于**三对**两两严格相交。

在反例里，相邻两对分别严格相交于 `[10040,10055]` 和 `[10030,10040]`；首末外缘却只在10040相切。只检查相邻关系，或只数三个局部成功对象，都不足以越过这一几何条件。

本轮没有把“全部底层单元必须严格触碰母核心”加成准入门。原延伸口径是闭区间相交，本例正处在该有效域内；把它删掉会改变待研究候选的定义域。

## 与原规则及已有反例的关系

中枢正本 Z-2要求严格 `ZD<ZG`；§4.1延伸允许闭区间相交；S-4触发计数与Z-4触发后的重算原本就是两项。当前研究得到的是这些条件下**某个自动上送推断不成立**，并不将其宣称为已裁规则互相矛盾。

Stage8说明九段触发后不一定有三个合法子窗；本轮补充：即便三个子窗全部合法，也还必须检查下一层。本轮仍未证明DC局部对象拥有原义完整次级走势资格，所以也不将反例扩大成原文理论不可能或全部F₁候选不可能。

对当前路线的处置是具体的：停止把“九段＋三个局部成功”当作完成证书或闭合证明。若继续研究封装，必须保留独立上级判定、未形成上级时的状态与成员来源，以及真正的完成条件。重新打开这条被否定的充分性路线，需要新的有依据前提或不同候选版本，不能重跑同一规则直到通过。

## 形式化范围与验收

冻结的 `AutomaticStrictLift`量化正δ、旧Whole、待重切吸收关系、真实可见成员、闭区间接触、空尾部和三个局部成功；结论要求它们的完整外缘产生严格上级核心。见证满足这些前提而否定结论，所以根直接证明 `¬ AutomaticStrictLift`。

根 `LiftBoundary.exact_root` 的精确类型为：

`INTERSECTION_TARGET ∧ WITNESS_TARGET ∧ ORIGIN_REJECTION_TARGET ∧ ¬ AutomaticStrictLift`。

最终run `70afcb63cad748cebe219c90e28e4091`，Lean4.31 精确检查exit0，machine/exact_root均通过；实际假定仅标准 `propext/Classical.choice/Quot.sound`，无sorryAx、自定假定或不安全依赖。声明锁未变。初次算术检查需要展开记录投影和closedTouches，修复只在证明体；该编译诊断本身没有被当作数学反例。

摘要：[stage14-lean-verification-summary.json](stage14-lean-verification-summary.json)；源绑定：[source-sha256-stage14.json](source-sha256-stage14.json)。完整证据在 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/`。独立读回/比较包已准备，未派发，semantic仍为not_reviewed。

工作树根目录复现，使用新输出目录：

```sh
LEAN_PATH="$PWD/formal/.lake/build/lib/lean" python3 \
  .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project "$PWD/.chanlun/review-results/issue1467-f1-proof" \
  --target-file LiftBoundaryProof.lean --declaration LiftBoundary.exact_root \
  --expected-type 'LiftBoundary.INTERSECTION_TARGET ∧ LiftBoundary.WITNESS_TARGET ∧ LiftBoundary.ORIGIN_REJECTION_TARGET ∧ ¬ LiftBoundary.AutomaticStrictLift' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-reproduction-2
```

没有修改旧候选、formal或生产源，没有新取数或市场实验。P1的完整递归资格、P2的完整实时接线、P3增量价值、P4采用判断及独立审查均保持开放。
