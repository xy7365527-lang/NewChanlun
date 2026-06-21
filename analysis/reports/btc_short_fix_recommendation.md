# 如何修好 BTC 做空：诊断 + 修复方向

## 0. 一句话回答用户

**做空吃掉了约 873pp 的总收益，其中 660pp（75.6%）是 displacement（多头暴露被 F-entry-short 翻空挤掉，多头利润从 907k 坍缩到 247k），只有 213pp（24.4%）是空头直接亏损——所以杀手是"root 在回调处被误判翻空"，不是"做空动作本身亏钱"。**

---

## 1. 核心论断确认/修正（独立逐笔重算，全部 bit-match）

我用 `.venv/bin/python` 从四份真实 trades 逐笔重算（NAV 基=100000），结果与验证 JSON 逐位吻合：

| 论断 | 判决 | 我的重算 | 修正 |
|------|------|---------|------|
| **A：displacement 占主导** | ✅ **成立** | disp=660,119.4(+660.1pp) / 空头直接亏损增量=−212,816(−212.8pp) / 恒等式 872.9 = 660.1 + 212.8 逐位闭合 / 占比 75.6% | 无 |
| **B：H¹ 短差"本身赚钱"** | ❌ **不成立（符号对但实质错）** | long-only swing-short = +18,281（n=4577）但 **win%=49.8%、median=−0.0000、删top1%翻 −449,886、bootstrap P(sum>0)=0.514**（≈掷硬币，boot_std≈12×点估计） | **改口径**：短差在 root 做多时是**净零**（两股 ±3.4M 巨流相抵的残差），不是"赚钱"。"做空能赚"的正确证据不是这 +18k，而是 Claim C |
| **C：做空非结构性不可盈利，杀手是 root 整仓翻空/强平** | ✅ **成立** | long-only 含 4586 笔空头、99.6% 多头同量级名义额，净 view 仅 −5.7k 却交付 +901.7%；T-and 两笔整核心强平 liq_long(−132.5k)+liq_short(−79.5k)=−212k=总亏损 211%，同期 swing 书净 +111.7k | **微调**：T-and 爆仓是 liq_long+liq_short **两笔合计**，单归因 liq_short(−79k) 会低估 long-core 失血 |
| **D：可加性成立（单核心引擎）** | ✅ **成立但需限定 L 等级** | 5/5 版本 view 总和 = mobile_realized_pnl / final_nav，误差 <0.001%（lo diff −0.45、nf diff +0.16、and diff≈0） | **标注**：这是 **L0 定义性恒等式**（NAV 被定义为逐笔已实现 P&L 的纯累加台账，weight=1.0/deferred=0/partial=False，无复利无 MtM），不是独立路径对账（非 L2）。信息增量近零 |

**新发现（强化 Claim D 诊断）**：nf 空头亏损 **98.6%（−215,522 / n=2096）集中在 held>500k bar 的"僵尸空头"**，按层分解 ladder3(n=3294, avg_held=885,350 bar, −124.5k) + ladder5(n=8, avg_held=1.7M bar, −84.5k)。僵尸空头入场中位价 $23,986 → 平仓中位价 $24,381（被牛市抬着走，exit>entry 系统性失血）。**非僵尸短差仅 −2,967（近平）**——失血 100% 来自"长持逆势空头"，不是短差机制本身。

---

## 2. view-account 会计限制（诚实声明）

**逐笔 P&L 是视图账（view-account），只刻画分布，仅在单核心引擎上可加到 NAV。** 验证：
- `long-only/nf/structural/and` 均为 single-core flat-realized round-trip（weight=1.0），故 `sum(逐笔)≡equity[-1]−CAP` 是**定义性恒等式（L0）**，非独立证据。
- **陷阱**：因为 NAV 同源于这套 P&L 公式，对账无法独立验证多空标签的"经济正确性"，只能验证分区穷尽+符号自洽。
- **不可加边界**：一旦引入多核心共享仓位池、分数 sizing、浮盈 MtM 或复利（如 longonly/nf 的多声部共享现金池场景），逐笔 view 和不再可加到 NAV——必须用 by_ladder 聚合 + mobile_realized 交叉印证。
- **可实现性陷阱**：T-and NAV<0(−359)，可加性算术成立但 view 账无保证金地板，该已实现亏损在真实杠杆账户不可兑现。

---

## 3. 推荐修复方案

### 主轴 = B（非对称 regime 门控 F-entry）

**机制定位**：失效环节在 `rust/src/fugue_v3/operate.rs` 的 **F 段 line 426-444**（已核实），具体是 line 430 的 `Direction::Down => (Polarity::Short, obs.sell_source(), ...)` —— 单层 `sell_source@s` 在 secular bull 里是**次级别顶背驰=平多窗口**，被误当翻空信号，导致 root 整核心翻空 → core_polarity=Short → E/D 分派镜像 → ladder3 僵尸空头被批量开出。

**具体改向**（仅作用 Short 侧，Long 侧字面不动）：

```rust
// mod.rs：新增 L0 阈值
pub const SHORT_REGIME_MIN_LEVELS: usize = 2;

// operate.rs：纯读 h0 的辅助函数（emergent_ceiling 已在 line 40 暴露）
fn short_regime_confirmed(s: usize, h0: &dyn MorphologyAxis) -> bool {
    let ceiling = h0.emergent_ceiling();
    let mut consec = 0;
    for k in s..ceiling {
        if h0.direction(k) == Some(Direction::Down) { consec += 1 } else { break }
    }
    consec >= SHORT_REGIME_MIN_LEVELS   // 从入场势源层 s 起连续 Down ≥ N_GATE
}

// F 段 line 443-444 入场守卫：把 short_blocked 替换为 regime_ok
let regime_ok = match polarity {
    Polarity::Long  => true,                              // 不收紧多头，避免新踏空
    Polarity::Short => short_regime_confirmed(s, h0),
};
if bsp_fire && aligned && self.free > 0.0 && regime_ok { ... }
// force_long_only 保留为 regime_ok 的特例（long_only 时 Short 恒 false），不删受控开关
```

**为什么消除 displacement**：回调处（单层 sell_source、上层仍 Up）`short_regime_confirmed` 返回 false → F 段不翻空 → root 维持 Long → +811k 多头摆动书继续在上涨段挂载（自然复现 long-only 的 +901.7% 结构）。

**为什么做空仍能赚（不是关做空）**：只有 s..ceiling 多级别贯通 Down 才整仓做空——此时进入真下跌段，entry>exit，逐笔 short=shares×(entry−exit)>0 是结构性盈利。ES2022/CL 油链下跌侧（用户实际押注金油比下降=做多油，但系统跨 regime 用）保留做空赚钱能力。

### 副轴 = D 的 A′ 离场纪律（焊进 B，正交不冲突）

B 控"空头何时生"，A′ 控"空头何时死"。在 A 段（line 258 子仓强平循环）**之前**插入 A′：对每个 `direction==Short` 的层，缠论次级别底（`obs.nf_buy(k)`）或方向翻 Up（`h0.direction(k)==Some(Up)`）任一成立即 `reduce_at` 全平 cover（reason=`short_stop`），先于 2× 被动强平。用 `touched[k]` 与 A/D/E 互斥（prove_no_double_act 不破）。这直接截断残余僵尸空头（−215k 失血通道的兜底）。

### 不纳入 C（删 F-entry-short + overlay cap）

C 用"root 恒 Long"放弃熊段整核心做空能力，有效域被声明为 ⊂上行 regime，抵触跨 regime 部署需求（谱系 539）；且 OVERLAY_SHORT_CAP 是与 N_GATE 功能重叠的新自由参数；删 force_long_only 旗标丧失受控对照能力。

---

## 4. 可证伪回测（四步，全用 t_engine/fugue_v3 在 BTC 全程 25M bar 重跑）

| 步骤 | 改动/开关 | 预期（机制成立） | 证伪条件 |
|------|-----------|-----------------|---------|
| **基线** | FUGUE_LONG_ONLY=1 / 默认 NF | +901.7%(L+907k,S≈−5.7k) / +28.8%(L+247k,S−218.5k) | displacement 上下界 |
| **1 验 B 复位 disp** | 新增 `FUGUE_SHORT_REGIME_MIN=2`（0=退回NF） | strat 从 28.8% 回升向 901.7%（落区间 300%~901.7%）；long_pnl 247k→向 907k；short_pnl −218.5k→收敛近 0（僵尸不再诞生） | long_pnl 不回升⇒推翻全部诊断；short_pnl 不收敛⇒推翻 core_polarity=Short 镜像论 |
| **2 验 short 激活率非零** | 同上，dump per-trade | short-root 在 BTC 2018/2022 熊段激活（episode>0）且段内 short_pnl 为正；扫 N_GATE∈{2,3} | 激活率→0⇒B 暗中退化成 long-only（伪做空），需降 N_GATE |
| **3 焊 A′** | 加 `FUGUE_SHORT_STOP=1` | short_pnl 从近 0 转小正（残余僵尸截断）；多头 P&L 不变（A′ 只动空头层） | 加 A′ 后 strat 下降⇒whipsaw 摩擦>僵尸节省（震荡标的鞭打），需收紧 dir_flip |
| **4 L3 跨 regime** | B+A′ 在 8 标的全跑 | 强牛{BTC,OKLO,QQQ,ES,BRN}受益最大；下跌{CL,DX}验证 short-root 吃下跌、A′ 无鞭打净亏 | 任一 regime 否证⇒退回白名单声明，**不可由 BTC 单标的 L2 改善外推全 regime** |

---

## 5. 结果包六要素

**结论**：BTC 做空失血的 75.6%（660pp）是 displacement（root 被回调误翻空挤掉多头），24.4%（213pp）是空头直接亏损，其中 98.6% 集中在 held>500k bar 的僵尸空头。修复=主轴 B（F 段 Short 入场加多级别贯通 Down 门控，operate.rs line 443-444）+ 副轴 A′（缠论次级别底即 cover 离场纪律），**保留做空、不关做空**。

**定义依据**：(1) 缠论第 49/64 课——单层 sell_source 在 Up regime 是次级别顶背驰=走势结束=平多窗口，非翻空信号；多级别贯通下跌才是 regime 翻转（operate.rs line 410-418 注释已记录此区分）。(2) 第 64 课买卖点对称——空头的"底"=多头的"顶"，缠论买点=平空点（A′ 离场判据）。输入数据特征：nf 版 ladder3 僵尸空头 avg_held=885,350 bar、入场中位 $23,986→平仓 $24,381，满足"次级别顶背驰被误当翻空"的定义条件。

**边界条件（结论翻转）**：(1) 若某标的全样本无任何 s..ceiling 连续 ≥2 级 Down 窗口，B 退化为 long-only，short 腿恒零——做空有效域为空（与谱系 539 一致）。(2) 若步骤1 long_pnl 不回升，则 displacement 非 root-flip 造成，全部诊断推翻。(3) SHORT_REGIME_MIN_LEVELS=2 是 **L0 假设**，未经 L2/L3，低波动标的上 ceiling 难涨到 s+2 会令 short-root 永不激活（伪做空）。(4) A′ 的 dir_flip 用 `direction(k)==Up` 是破前高的 **L0 代理**（MorphologyAxis 未暴露真破极值），震荡 regime 有 whipsaw 风险。

**下游推论**：若修复成立，则 (1) BTC strat 回升至接近 901.7%，长头复位为主项 + 真下跌段空头为正的次项；(2) "清仓/翻空频率是 regime 函数"被再次确认（与 nrf v4 正域收敛一致），B+A′ 不可声称全 regime 有效，须按 L3 白名单部署；(3) E/D 段 H¹ 短差通道一字不改（它在 root 做多下已净零，方向对齐即转正），修复只动 F 段翻空入口。

**谱系引用**：直接相关——`project_t14_t5_root_flip_necessity`/539号（根翻空有效域⊂非上行 regime，本方案=对 539 的工程回应：不取消根做空，补 regime 门+离场纪律）、`project_t_short_close_level_mismatch`（空头晚建早平/平空级别错配，A′ 滞后税同构）、`project_unn_btc_spawn_throwback`（E spawn 强牛过度做空 −54k，与 ladder3 僵尸同源）。本方案未改任何缠论定义，仅补"空头离场=缠论买点"这一既有对称性的实装缺口。

**影响声明**：改动 `rust/src/fugue_v3/operate.rs` F 段（line 426-444，Short 分支入场守卫）+ A 段前插入 A′ 段 + `mod.rs` 新增常数 `SHORT_REGIME_MIN_LEVELS`；新增 env 开关 `FUGUE_SHORT_REGIME_MIN`/`FUGUE_SHORT_STOP`；保留 `force_long_only` 受控对照旗标。不改 E/D 段 H¹ sink/recover、不改 core_polarity 下游分派、不改信号层、不改 MorphologyAxis trait（全部读现有暴露方法）。守恒/NAV 中性证明（prove_conservation/prove_nav_neutral/prove_no_double_act）不受影响（reduce_at 是既有一减一加级别转移）。

**认识论等级**：诊断（Claim A/C/D 重算）= **L2**（BTC 单标的真实数据，可证伪，已逐位复核）；Claim B 重算 = **L2**（bootstrap P=0.514 否证"短差盈利"）；可加性恒等式 = **L0**（定义性，信息增量近零）；修复方案有效性 = **L0 结构论证 + L1 待验**（未重跑引擎，部署需走第 1-4 步 L2→L3）。**不可在 BTC L2 改善后声称全 regime 有效（formalization-validity-domain 规则）。**