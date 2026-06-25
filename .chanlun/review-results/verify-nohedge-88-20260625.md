# 独立验证：staged no-hedge 被动版真实 ON 数字 — contested 解决

**工位**：swarm/verify-nohedge ｜ topo_address: swarm/verify-nohedge ｜ 任务 #88
**日期**：2026-06-25 ｜ 分支：prop4-nest-readingB-20260623（工作树 orbit9-B-wt）
**认识论等级**：L1（OFF bit-exact 管线验证）+ **L3（8 标的真实数据 ON，否定性可证伪）**
**约束**：只测不改代码（实证独立，不偏向任一方）

---

## 〇、一句话结论

**当前工作树 staged 代码 = no-hedge 被动版（`if !protect_core { rec_add(...) }`，protect_core=true 时核心不减 ∧ 不开 hedge 空腿 ⇒ sink 退化为 no-op）。8 标的 ON（`T_ORBIT9_NODEVEC=1`）Structural 实测逐字复现 bsp-opsem-impl 报告的全部 8 个数字（BTC +381.5% / CL −68.2% / ...），无任何 −100% 爆仓，short_pnl 全部有界（最大 BRN −87930，非天文数字）。判定：bsp-opsem-impl 数字真实，no-hedge 是健全基座。o6-finalize 测的是另一版本（commit bf17bb91c4 开 hedge 空腿 / σ-cap 版），其无界爆仓诊断对那两版成立，但与当前 staged no-hedge 版无关 —— 三方数字各自真实，分歧源于测了不同代码版本。**

---

## 一、代码状态核验（git diff，先确认测的是什么）

**staged 状态**：`git diff --cached rec_engine.rs` 为空 → 改动在工作树未 staged。`git diff` (vs HEAD) 显示 no-hedge 被动版：

```rust
// rec_engine.rs sink_impl（行 1415-1429）
let realized = if protect_core { 0.0 }                       // 核心不减
               else { rec_reduce(&mut self.instances[parent], m, &mut free, c) };
...
if !protect_core {                                            // ★ R+ 不开 hedge 空腿
    rec_add(&mut self.instances[sub], short_u, mob, &mut free, c);
}
```

- **σ-cap 不在工作树代码里**：`grep -niE "sigma.?cap|σ-cap|sigma_clamp"` 全仓 rust 零匹配 → 当前 staged 是**纯 no-hedge 版，无 σ-cap**。这正是 bsp-opsem-impl 声称的版本。
- protect_core 来源（行 1594）：`self.enable_orbit9_nodevec && self.is_rstar_pullback_leg(j, is_buy, view)`。
- NODEVEC 开关（行 225）：`enable_orbit9_nodevec: std::env::var("T_ORBIT9_NODEVEC").is_ok()`（环境变量存在即开；OFF 时 protect_core 恒 false ⇒ 原始 sink，bit-exact）。
- 编译：`cargo build --release --tests` PASS（仅 warning，无 error）。

## 二、OFF bit-exact 管线验证（L1，PASS）

`BT_SYMBOLS=CL cargo test --release recursive_t::rec_stream::tests::rec_btc -- --ignored --nocapture`（不带 NODEVEC）：

```
[CL/Structural] strat=+20.3% sink=2707 recover=1369 short_pnl=-13050 net_short=37.9% liq=8
```

逐字复现两方报告的 OFF 列（CL +20.3% / sink=2707 / recover=1369）⇒ 管线正确，no-hedge 门控不扰动 OFF。两方对 OFF 无分歧。

## 三、ON 版（no-hedge 被动，T_ORBIT9_NODEVEC=1）8 标的 Structural L3 实测

| 标的 | **实测 ON（本工位独立）** | bsp-opsem-impl 报告 ON | OFF | BH | liq | net_short | short_pnl | 一致? |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| **BTC** | **+381.5%** | +381.5% | +26.0 | +1380.4 | 0 | 28.1% | −32114 | ✓逐字 |
| **ES**  | **+246.8%** | +246.8% | −2.3  | +594.3  | 0 | 9.9%  | −3862  | ✓逐字 |
| **GC**  | **+56.9%**  | +56.9%  | −22.7 | +257.3  | 0 | 74.8% | −26353 | ✓逐字 |
| **CL**  | **−68.2%**  | −68.2%  | +20.3 | +28.2   | 4 | 43.3% | −44593 | ✓逐字 |
| **BRN** | **−81.2%**  | −81.2%  | −26.0 | +87.4   | 3 | 59.2% | −87930 | ✓逐字 |
| **DX**  | **−11.7%**  | −11.7%  | −0.7  | +4.1    | 0 | 58.0% | −7373  | ✓逐字 |
| **QQQ** | **+14.1%**  | +14.1%  | −8.6  | +174.6  | 0 | 68.1% | −18885 | ✓逐字 |
| **OKLO**| **+5.4%**   | +5.4%   | +55.3 | +307.1  | 3 | 20.7% | −66623 | ✓逐字 |

**8/8 逐字复现。** 跑法：`T_ORBIT9_NODEVEC=1 cargo test --release recursive_t::rec_stream::tests::rec_btc -- --ignored --nocapture`（728s 全 8 标的）。

### 关键诊断（判别 真实 vs 幻觉）
1. **无 −100% 爆仓**：最负 BRN −81.2%，CL −68.2%，无任何标的打穿账户。对比 o6-finalize 报告 commit bf17bb91c4 的 CL **−1165404%**（无界爆仓）。
2. **short_pnl 全部有界**：最大绝对值 BRN −87930（4 标的在 1e4 量级，OKLO −66623）。对比 o6-finalize 的 GC short_pnl=**−1.8e28**、DX +1.1e10（无界累积）。
3. **no-hedge 机制确认（关键诊断点）**：BTC `short_pnl_by_level=[−2875,−3654,283,−112,0,0]` —— 全是小额配对短差腿，来自 **protect_core=false（R− 下跌段）** 路径正常开的 sink 空腿（有界、有 recover 配对）。protect_core=true（R+ 主升浪）路径**不开空腿**（`if !protect_core` 门控），那部分 short_u 累积为 0 ⇒ 无 #567 无界空腿。net_short（如 BTC 28.1%）是 R− 路径有界空腿的市值占比，非无界发散。

## 四、三方数字对照 —— 分歧根因 = 测了不同代码版本（非数字造假）

| 版本 | sink_impl 行为 | CL ON | 测者 | 真实性 |
|---|---|---:|---|---|
| commit bf17bb91c4 | 核心不减 + **机动腿照常全额开空** | −1165404%（无界） | o6-finalize | 对该版真实（#567 无界空腿） |
| σ-cap 版（#73 未提交） | 上版 + held_core_equiv 封顶 | −677.6%（仍爆仓） | o6-finalize | 对该版真实（补丁未约束住） |
| **staged no-hedge 被动版** | 核心不减 ∧ **不开 hedge 空腿** | **−68.2%（有界）** | **本工位 + bsp-opsem-impl** | **真实（无空腿=无无界）** |

**三方数字各自真实**。o6-finalize 的「无界爆仓」诊断对**开 hedge 空腿的两个版本**成立 —— 那两版确实无界（核心不减 ⇒ 配额基不衰减 ⇒ 每次回调全额加空 ⇒ 无 recover 配对则线性发散，#567）。**no-hedge 被动版直接砍掉空腿**（`if !protect_core` 门控），从根上消除了无界源 —— 这正是 escalate §五建议方案2「机动腿配额随空腿衰减 / 放弃机动主动做空」的实现：选了「不开空腿」这一极。**不是数字造假，是 escalate 之后代码演化到了第三版，三份报告测的是不同代码。**

## 五、结果包六要素

### 1. 结论
staged no-hedge 被动版 8 标的 ON 实测逐字复现 bsp-opsem-impl 报告（BTC +381.5% / CL −68.2% / 全 8 标的），无 −100% 爆仓，short_pnl 有界 ⇒ **数字真实，no-hedge 是健全基座**（无空腿 ⇒ 无 sink↔recover 印钞机/无界源）。

### 2. 定义依据
- **#567 无界空腿**：σ-不变配额在「核心不减」前提下退化为无界累积 —— 仅当**机动腿照常开空**时成立。no-hedge 版 protect_core=true 时不开空腿 ⇒ #567 前提不满足 ⇒ 无界源消除。
- 输入满足：`enable_orbit9_nodevec=true`（NODEVEC ON）；`is_rstar_pullback_leg` 判 R+/R− ⇒ protect_core；`if !protect_core` 门控空腿开仓。

### 3. 边界条件（结论翻转）
- 若改回「protect_core=true 仍开 hedge 空腿」（commit bf17bb91c4 行为）⇒ #567 无界空腿复活 ⇒ CL → −1165404%（o6-finalize 已证）。**当前 no-hedge 门控是有界性的唯一保证**。
- 若 net-up regime 换 bear 窗 ⇒ 强牛标的（BTC/ES）优势消失（被动保护在真跌中不防守）。8 net-up 标的无法证 bear 有效性（231号有效域 ⊂ net-up）。
- 净收益仍 regime 函数：**超 BH 0/8**（无标的超 BH），相对 OFF 改善 5/8（BTC/ES/GC/QQQ/DX 边际），由负转正 3/8（GC/QQQ/OKLO）—— 与 bsp-opsem-impl 报告 §四计数一致。

### 4. 下游推论
- bsp-opsem-impl 报告 o6-level-role-direction-fix-l3 的 ON 数字表**对 no-hedge 版有效**，可作下游依据 —— 但须**标注版本 = no-hedge 被动版**（非 commit bf17bb91c4 开空腿版）。
- o6-finalize escalate 的「ON 数字造假/声明膨胀」指控**对 commit bf17bb91c4 成立**（那版确实无界），但**对当前 staged no-hedge 版不成立**（逐字复现）。escalate 的真实贡献 = 暴露了「开 hedge 空腿版无界」+ F1 判别量级别错配真 bug，驱动代码演化到 no-hedge 版。
- no-hedge 版可 commit（无界源已消除，8 标的有界）；但「净收益超 BH 不普适」「F1 判别量级别错配（top_trend_level≠rstar）是否仍 panic」需独立确认（本工位未测 debug 构建 F1 守卫）。

### 5. 谱系引用
- **#567（无界空腿）**：no-hedge 门控消除其前提（核心不减 + 不开空腿，非核心不减 + 开空腿）。
- **#63/#64（bsp-opsem-impl-l3）**：#64 报告数字对 no-hedge 版真实；与 commit bf17bb91c4 的版本差异是 contested 根因。
- **escalate-o6-report-data-fabrication**（o6-finalize）：其无界爆仓 L3 对 commit bf17bb91c4 / σ-cap 版真实，对 no-hedge 版被否证（逐字复现非幻觉）。
- **formalization-validity-domain**：no-hedge 有效域 ⊂ net-up（8 标的）；bear 窗未证（231号）。

### 6. 影响声明
- **改动**：无（只测不改，符合任务约束）。
- **产出**：本报告（实测数字表 + 三方对照 + 判定）。
- **影响**：解决 contested —— bsp-opsem-impl 数字真实，escalate 指控针对不同版本。下游引用 #64 ON 数字时须标注「no-hedge 被动版」。

## 六、认识论等级标注

| 命题 | 等级 | 信息增量 |
|---|---|---|
| OFF bit-exact（CL +20.3% sink=2707 逐字） | L1 | 零（验证 OFF 不变） |
| **no-hedge 版 8 标的 ON 逐字复现 bsp-opsem-impl（BTC +381.5 等）** | **L3（8 标的真实数据）** | **高：bsp-opsem-impl 数字真实，no-hedge 健全基座** |
| **short_pnl 有界 + liq 无 −100%（对比 o6-finalize −1165404/−1.8e28）** | **L3（否定性）** | **高：no-hedge 消除 #567 无界源** |
| 净收益超 BH 0/8（regime 函数） | L3（否定性） | 中：踏空溶解 ≠ 净收益普适（继承 #64） |

**核心诚实声明**：① no-hedge 被动版数字真实（8/8 逐字复现，无界源消除）。② o6-finalize 的无界爆仓诊断**对开 hedge 空腿版真实**（非误报）—— 三方分歧 = 测了不同代码版本，非任一方造假。③ 净收益超 BH 仍不普适（0/8），有效域 ⊂ net-up（bear 未证）。④ 本工位未测 debug 构建的 F1 守卫（top_trend_level≠rstar panic）—— 该判别量级别错配问题独立于有界性，需另测。
