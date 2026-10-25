# ADR 0035：成本停止 policy 实例、缺项状态与两层进阶证据门

**日期**：2026-10-14
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核），走 #1354（G-029）
**状态**：已采纳。本 ADR 只定记录形状、输入来源门、读法、缺项状态与进阶判据形态；不填费率、滑点、延迟、持有成本、θ 或任何数值。教义正本：`.chanlun/definitions/qujiantao.md` §4（成本门 `[旧缠论]` `035:30`，塔底 `[新缠论:选择]`，参数不入正本）。

## 背景
#907 后裁：ROUNDTRIP 配对；第一层由「延伸前算出上限」收窄为「当前 L0 余量 + 延伸后立即复测」；中位数与下尾读法给出相反结论的张力移交 #853，ADR 0022 裁定三已答「用置信下界」。RESIDUAL G-029：历史 BTC/1m、两重叠前缀窗、无置信区间、误差只有代理下界、未计持有成本，不能作当前证据。SPEC C04.4/C04.5、A-ST-037-01/02、A-FG-011-01。

## 裁定
### 一、记录形状与读法
政策 id 族 `#1354/cost-stop-policy/v1`。记录 ⟨policy_id, version, profile_id, symbol, granularity, tower_base_ref, inputs{fee(Known, 费率簿引用含 source_url/retrieved_at/tier/role), slippage(Known|ProxyLowerBound|Missing), latency_error(同), holding(funding/borrow 引用 | Missing)}, amplitude{pairing=ROUNDTRIP, per_level[{level, median, lower_bound_95, n, max_month_cluster, windows[]}], probe_run_ref}, threshold{form=ratio_lb_ge_theta, theta 整数对(用户签署), authorizer, authorized_at}, evidence_gate(继承 ADR 0022 裁定六/七), valid_through, supersedes⟩。比值 `r(ℓ) = A_ℓ / (2·fee + slippage + latency_error [+ holding])`，A_ℓ 取 ROUNDTRIP 波幅单边 95% 下界（按月 block bootstrap）。通过 ⟺ `lb(r) ≥ θ` ∧ 证据门；`InsufficientEvidence` 与 `Fail` 分开。θ 由用户签署；系统拒绝 θ < 1（约束，非默认值）。全部定点整数。

ADR 0035 `amplitude.pairing` 保持单值 ROUNDTRIP，不恢复 AMP_EXEC。依据 [#907 resolution 评论5194145006](https://github.com/xy7365527-lang/NewChanlun/issues/907#issuecomment-5194145006) 的三步：成本每趟往返支付；与其可比的波幅是同一趟往返能吃到的幅度；相邻同向点不对应完整交易。标签 `[新缠论:推论]`，排除类别为「违已裁推论」，不以拟合好坏作选择理由。该评论的否证口逐字为：“若能论证「成本也可以按单边逐点计而不按往返计」，则 AMP_EXEC 回到候选。”逐字原件见 #907 该 resolution。若将来满足否证口，仍按现行裁定变更流程处理；后一句是流程要求，不冒称 #907 原话。

### 二、缺项是状态不是 0
任一输入 Missing ⟹ `AwaitingPolicy{cost_input}`，成本门 `NotEvaluable`；此时停止判据只剩塔底（阶段一塔底 = L0，现役塔不受影响），阶段二任何延伸不得开启。ProxyLowerBound 代入只得 `UpperBoundOnly`：可判失败，不可判通过。holding Missing 只影响持有时间长于 funding 周期的级别列，记录须写明起始级别。

### 三、第一层 = L0 余量对象 + 复测协议
`L0Margin{policy_version, lb_r, theta, status}`。复测协议：延伸前冻结版本与塔底；延伸后立即以同一探针血统测新级；`lb(r) ≥ θ` ∧ 证据门 ⟹ Enabled，否则 Disabled 且塔底回退、延伸记录保留；逐级重复。当前在案材料（#907）按本记录评估为 `InsufficientEvidence`（中位数、窗重叠、滑点代理、无区间），可作 probe_run_ref，不可作通过证据。

### 四、第二层判据与逐层逐窗证据（#1598）

> 2026-10-26 用户批准 #1598：本节将合法选择落为具名版本字段；未签数值与未标定能力不因方法批准自动生效。

`second_layer.rule`。可选值 = {`order_relation`（默认，出处：ADR 0035 裁定四:21，序关系判据，无需签数），`share_threshold{theta2}`（待标定，前置 S4-a/S4-b 修完）}。两值语义差异：(1) `order_relation` = 「主因已转为到底」⟺ `share_lb_95(reached_bottom) > share_ub_95(divergence_not_judged)` ∧ 证据门（ADR 0035:21），判据形式完全由读数区间序关系决定、不需用户签数值阈值；(2) `share_threshold{theta2}` = 「逐层、逐窗的已到底占比单边置信下界 ≥ θ₂」∧ 下述具名证据门，θ₂ 为签署门字段（默认 AwaitingPolicy{theta2}，推荐值进签署单、用户批复后生效），须在读数前预登记版本；引用已授权的#1677/evidence-gate/v1不要求重签，改六字段才另签v2。两值共同前置：S4-a/S4-b 修完（到底分类器与延伸实例证据收集）。代价：`share_threshold` 多签一个数（θ₂ 整数对）、多一次签署版本预登记流程。占比阈值为 ADR 0041 允许的选择，不再列入被否方案。

Q6 的既裁部分：引用 `#1677/evidence-gate/v1`（origin/main 的 ADR0022 修订§1–4、§6，原件见 ADR 0022），v1 六值为 confidence=95/100、block_unit=CalendarMonthUTC、min_events=30、min_blocks=12、min_windows=3、min_coverage=1/2。方法仍为时间块 bootstrap；下界用分布的 (1−confidence) 分位；固定规则为单边下界而非点估计、互不重叠窗口、每窗各自过线、不以合计替代。窗口表及版本早于 run_started_at 冻结，覆盖按预登记数据集首末 bar 时间区间计。v1已获授权，不能误写其缺用户重签；改六值须另签v2。

Q6 新增方法（用户 2026-10-26 已批准；θ₂ 数值仍待签）：`share_threshold` 的比较量为每个预登记分层、每窗 `reached_bottom` 占比的单边置信下界，非点估计。分层固定为 `(profile_id, symbol, granularity, base_ref, evaluated_level)`；不跨级/跨标的/跨profile合并。每层每窗的分母为该层该窗全部合格且已分类的下钻判定事件（由 classification_set 具名版本确定，每事件唯一计数），分子为其中 reached_bottom 事件。缺失/无法归类事件单列，存在此类事件时该层窗为 InsufficientEvidence，不静默剔除后过门。

具体算法：按所引 block_unit 将该窗事件分块，保留块内全部事件及依赖；从该窗 m 个非空块有放回抽 m 块，每次取合并事件的分子总数/分母总数，不平均各块比例。method_id=`descent-share/block-bootstrap/1`、resamples=10000、seed=0；PRNG固定PCG-XSH-RR 64/32（stream=1），块索引用拒绝取模避免偏差。以上算法标识/次数/随机种子和实现版本均在读数前记录；它们属于Q6已批准方法，不加入或改写evidence_gate六字段。每次重采样按抽样块计重复，得B个精确有理占比；升序排列，取第 ceil((1−confidence)×B) 项（1-based）为下界，不作插值。固定种子仅保证可复现，不替代区间有效性；在#1441实现时以独立定义oracle和同seed双跑检验方法。

每层每窗分别须 events≥min_events、非空块数≥min_blocks；每层所报窗口数≥min_windows，窗口互不重叠、合计时长覆盖≥min_coverage数据集区间。有效窗各自比较 `share_lb(reached_bottom) ≥ θ₂`，所有窗全过才给该层通过；所有被宣称通过的层逐层满足，不能以合计或其他层补偿。门不足为InsufficientEvidence，不是Fail；完整门成立但任一窗下界<θ₂为Fail。θ₂保持AwaitingPolicy，未签不运行share_threshold。默认 `order_relation` 及其已裁比较式不改；两规则共同前置S4-a/S4-b。本次用户批准完整方法及政策形式，不在这里代签θ₂。

原记录 `DescentBottomEvidence{run_ref, classification_set(#852), per_class{count, share_lb_95, share_ub_95}, windows[], base_ref}` 保留；新增事件分母只用于 share_threshold，不改既有 `(重,S稳定点)` 操作统计。当前 S4-a/S4-b 证据仍未齐，阶段二不启用。

### 五、有效域与回放
绑定 (profile_id, symbol, granularity, fee tier, tower_base_ref)；AsKnown 按请求时刻生效版本，缺版本即 NotEvaluable 不回填；跨域 Unsupported 不回落旧数；TB-08 只把输入来源升为 Known，不碰 θ 与波幅证据；experiment 证据不得被 production profile 直接引用。

## 被否方案
中位数读法；θ 固定为 1；AMP_EXEC 配对；缺项代 0；代理下界充 Known；未签 theta2 即启用占比阈值；引用 #852 旧 d≥2=0 读数。

## 受影响代码清单
见 `.chanlun/review-results/issue1323-grilling-20261013/1354-AFFECTED-CODE.md`（基线 main 3da0b2c8）。

## 下游票
#1441 TB-02-S；#1382/#1400/#1412 TB-08；#1366 TB-07（回放版本）。

## 溯源标签
成本门语义 `[旧缠论]`（`035:30`、`060:42`）；ROUNDTRIP `[新缠论:推论]`（#907）；其余 `[新缠论:选择]`。

## #1598 批复与受影响清单

本次修订沿用的 ADR 旧行号以获批草稿固定基线 `13ed4d45c8ff85175819ca987690328ab2a247a4` 为准；当前条款按标题定位。

用户 2026-10-26 批复 [resolution](https://github.com/xy7365527-lang/NewChanlun/issues/1598#issuecomment-6403277308)。本次字段修订为 `[新缠论:选择]`，ROUNDTRIP 沿用已裁推论。受影响文件、固定基线与候选承接见 [清单](../../.chanlun/review-results/issue1598-policy-20261026/affected-code.md)；政策批准不代表下游已接单或实现。
