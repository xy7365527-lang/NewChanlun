# ADR 0035：成本停止 policy 实例、缺项状态与两层进阶证据门

**日期**：2026-10-14
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核），走 #1354（G-029）
**状态**：已采纳。本 ADR 只定记录形状、输入来源门、读法、缺项状态与进阶判据形态；不填费率、滑点、延迟、持有成本、θ 或任何数值。教义正本：`.chanlun/definitions/qujiantao.md` §4（成本门 `[旧缠论]` `035:30`，塔底 `[新缠论:选择]`，参数不入正本）。

## 背景
#907 后裁：ROUNDTRIP 配对；第一层由「延伸前算出上限」收窄为「当前 L0 余量 + 延伸后立即复测」；中位数与下尾读法给出相反结论的张力移交 #853，ADR 0022 裁定三已答「用置信下界」。RESIDUAL G-029：历史 BTC/1m、两重叠前缀窗、无置信区间、误差只有代理下界、未计持有成本，不能作当前证据。SPEC C04.4/C04.5、A-ST-037-01/02、A-FG-011-01。

## 裁定
### 一、记录形状与读法
`tower_base_ref` 的值域与 `zhongshu.md` §2.1.1 一致；当前 `L0 = segment`，机读记录为 `tower-base/1`（#1626）。`kline3`／`trade3` 只在相应真实构件与数据 profile 具备时可签，缺位时显式拒绝，不以线段输入冒充。

政策 id 族 `#1354/cost-stop-policy/v1`。记录 ⟨policy_id, version, profile_id, symbol, granularity, tower_base_ref, inputs{fee(Known, 费率簿引用含 source_url/retrieved_at/tier/role), slippage(Known|ProxyLowerBound|Missing), latency_error(同), holding(funding/borrow 引用 | Missing)}, amplitude{pairing=ROUNDTRIP, per_level[{level, median, lower_bound_95, n, max_month_cluster, windows[]}], probe_run_ref}, threshold{form=ratio_lb_ge_theta, theta 整数对(用户签署), authorizer, authorized_at}, evidence_gate(继承 ADR 0022 裁定六/七), valid_through, supersedes⟩。比值 `r(ℓ) = A_ℓ / (2·fee + slippage + latency_error [+ holding])`，A_ℓ 取 ROUNDTRIP 波幅单边 95% 下界（按月 block bootstrap）。通过 ⟺ `lb(r) ≥ θ` ∧ 证据门；`InsufficientEvidence` 与 `Fail` 分开。θ 由用户签署；系统拒绝 θ < 1（约束，非默认值）。全部定点整数。

### 二、缺项是状态不是 0
任一输入 Missing ⟹ `AwaitingPolicy{cost_input}`，成本门 `NotEvaluable`；此时停止判据只剩塔底（阶段一塔底 = L0，现役塔不受影响），阶段二任何延伸不得开启。ProxyLowerBound 代入只得 `UpperBoundOnly`：可判失败，不可判通过。holding Missing 只影响持有时间长于 funding 周期的级别列，记录须写明起始级别。

### 三、第一层 = L0 余量对象 + 复测协议
`L0Margin{policy_version, lb_r, theta, status}`。复测协议：延伸前冻结版本与塔底；延伸后立即以同一探针血统测新级；`lb(r) ≥ θ` ∧ 证据门 ⟹ Enabled，否则 Disabled 且塔底回退、延伸记录保留；逐级重复。当前在案材料（#907）按本记录评估为 `InsufficientEvidence`（中位数、窗重叠、滑点代理、无区间），可作 probe_run_ref，不可作通过证据。

### 四、第二层 = 序关系判据
`DescentBottomEvidence{run_ref, classification_set(#852), per_class{count, share_lb_95, share_ub_95}, windows[], base_ref}`；「主因已转为到底」⟺ `share_lb_95(reached_bottom) > share_ub_95(divergence_not_judged)` ∧ 证据门。前置：S4-a/S4-b 修完。当前状态 NotStarted；阶段二不启用，义务完整保留。

### 五、有效域与回放
绑定 (profile_id, symbol, granularity, fee tier, tower_base_ref)；AsKnown 按请求时刻生效版本，缺版本即 NotEvaluable 不回填；跨域 Unsupported 不回落旧数；TB-08 只把输入来源升为 Known，不碰 θ 与波幅证据；experiment 证据不得被 production profile 直接引用。

## 被否方案
中位数读法；θ 固定为 1；AMP_EXEC 配对；缺项代 0；代理下界充 Known；第二层用占比阈值；引用 #852 旧 d≥2=0 读数。

## 受影响代码清单
见 `.chanlun/review-results/issue1323-grilling-20261013/1354-AFFECTED-CODE.md`（基线 main 3da0b2c8）。

## 下游票
#1441 TB-02-S；#1382/#1400/#1412 TB-08；#1366 TB-07（回放版本）。

## 溯源标签
成本门语义 `[旧缠论]`（`035:30`、`060:42`）；ROUNDTRIP `[新缠论:推论]`（#907）；其余 `[新缠论:选择]`。
