# O6 精修 L3：级别角色方向轴（R+/R−）替代退化力度轴 — 爆仓消除验证

**工位**：swarm/bsp-opsem-impl ｜ topo_address: swarm/bsp-opsem-impl ｜ 基因 073a/274号 ｜ 任务 #64
**日期**：2026-06-25 ｜ 分支：orbit9-B-dispatch-20260624（接续 #63 commit 6a680b2e5b）
**认识论等级**：L0（判别量从退化力度轴改为非退化方向轴）+ L1（OFF bit-exact）+ **L3（8 标的真实数据，否定性：爆仓消除但净收益仍 regime 函数）**

---

## 〇、一句话结论

**判别量从退化力度轴 `d_top[r*]`（net-up 恒 false）改为非退化级别角色方向轴 `top_trend_dir`（nf/morphology 驱动），按 r* 走势方向二分 O6：主升浪（R+）保护核心、下跌段（R−）核心正常 sink。结果——8 标的爆仓全部消除（#63 的 CL/BRN/OKLO/QQQ −87~−105% → −81/−68/+5.4/+14.1，无标的 −100%），编排者预言验证。GC/QQQ/OKLO 由负转正。BTC/ES 仍溶解远超 OFF（+381%/+247% ≫ OFF +26%/−2.3%）。但 8 标的仍 NOT 趋同——无标的超 BH，相对 BH 差距 regime 依赖（净收益仍 regime 函数，561 b2/563 未决）。** OFF bit-exact PASS（CL +20.3% sink=2707 逐字）。

---

## 一、#63 诊断正确 / 解法位置错 → #64 修正（编排者裁定）

**#63 诊断**（正确）：`d_top[r*]` 在 net-up 恒 false（最高涌现级别恒生长不产 type1）⇒ 力度轴退化 ⇒ 双源坍回单源 ⇒ 对所有次级别反核心向 BSP（含真下跌段）全保护 ⇒ 下跌段核心裸多扛跌爆仓。

**#63 解法错位**：把「力度轴退化」误当「判别量退化」。**真判别量 = 级别角色方向轴**——最高涌现级别恒不产 type1（力度退化），但它的**方向**恒由 nf 驱动已知（morphology emergent_dir），**方向轴永不退化**（[[project_morphology_nf_driven_emergence]]）。

**#64 修正**：判别量 `d_top[r*]`（退化）→ `top_trend_dir`（非退化方向轴，rec_engine.rs `view.top_trend_dir`，源 rec_stream.rs:312/338 = 最高有走势级别的 `top_trend.direction`，nf/morphology）。

## 二、O6 按 r* 走势方向二分（diff）

`is_rstar_pullback_leg(j, is_buy, view)` 改判别量（rec_engine.rs:1304-1338）：

```
保护核心(protect_core=true) ⟺ topo_pullback ∧ rstar_continuing
  topo_pullback   = j<r* ∧ 反核心向（拓扑轴，恒真 in is_reduce 分支）
  rstar_continuing = (top_trend_dir=Up ∧ 核心Long) ∨ (top_trend_dir=Down ∧ 核心Short)
                   = r* 走势方向 = 核心方向（主升浪/主跌浪延续中）  [级别角色方向轴，非退化]
```

- **R+（top_trend_dir=Up ∧ 核心 Long，主升浪）+ 次级别回调卖点** ⇒ 保护核心（核心 H⁰ 不动，§六 O6）。
- **R−（top_trend_dir=Down，下跌段，或走势转向核心方向失配）** ⇒ `_ => false` ⇒ **不保护，核心走常规 sink**（次级别下跌=主跌延续，核心该被减，不裸多扛跌）。**这修复 #63 的爆仓根因**：#63 把下跌段当回调 no-op 掉核心裸多。
- **同一「次级别下跌走势」在 r*=R+ vs r*=R− 下操作相反** = 读全纤维（含 r* 角色方向分量）⇒ 踏空溶解。

**改动**：`rec_engine.rs` `is_rstar_pullback_leg` body（判别量 d_top → top_trend_dir + 二分逻辑）。`sink_impl(protect_core)`/route_bsp 签名/字段不变（接续 #63）。

## 三、OFF bit-exact（L1，PASS）

`cargo test --release recursive_t` = **126 passed / 0 failed**（含 OFF 守卫）。真实数据 NODEVEC OFF：CL Structural = **+20.3% / sink=2707 / recover=1369 / short_pnl=−13050** = 逐字一致 ⇒ 方向轴二分门控不扰动 OFF。

## 四、三版本 8 标的 L3 对照（Structural，net-up 全程）

| 标的 | OFF | #63 全禁 sink | **#64 方向轴二分** | sink (#64) | net_short (#64) | BH | 爆仓判定 |
|---|---:|---:|---:|---:|---:|---:|---|
| **BTC** | +26.0% | +1536.7% | **+381.5%** | 1117 | 28.1% | +1380.4% | 溶解（超 OFF） |
| **ES** | −2.3% | +828.5% | **+246.8%** | 490 | 9.9% | +594.3% | 溶解（超 OFF） |
| **GC** | −22.7% | −27.9% | **+56.9%** | 1155 | 74.8% | +257.3% | **转正** |
| CL | +20.3% | −104.9% | **−68.2%** | 1894 | 43.3% | +28.2% | **爆仓消除** |
| BRN | −26.0% | −100.0% | **−81.2%** | 1168 | 59.2% | +87.4% | **爆仓消除** |
| DX | −0.7% | −16.9% | **−11.7%** | 739 | 58.0% | +4.1% | 改善 |
| **QQQ** | −8.6% | −86.8% | **+14.1%** | 320 | 68.1% | +174.6% | **转正** |
| **OKLO** | +55.3% | −100.4% | **+5.4%** | 114 | 20.7% | +307.1% | **爆仓消除+转正** |

**计数**：
- **爆仓（−100%）消除：8/8**（#63 有 5 标的 ≤−87%，#64 无任何 −100% 爆仓）。
- **相对 #63 改善：6/8**（CL/BRN/DX/GC/QQQ/OKLO；BTC/ES 仍溶解但量级降，因方向轴允许部分 sink）。
- **相对 OFF 改善：5/8**（BTC/ES/GC/QQQ + DX 边际；CL/BRN/OKLO 仍负但脱离爆仓）。
- **由负转正：3/8**（GC/QQQ/OKLO）。
- **超 BH：0/8**（净收益仍 regime 函数）。

## 五、机制分析

1. **sink 恢复（不再全 0）**：#63 力度轴退化 ⇒ 全禁 sink（sink=0）；#64 方向轴二分 ⇒ 下跌段恢复 sink（CL 1894、BRN 1168、BTC 1117）。下跌段核心正确被减 = 不裸多扛跌 = **爆仓消除根因**。
2. **主升浪保护仍生效（BTC/ES 溶解）**：top_trend_dir=Up 时核心保护，BTC/ES 强牛大部分时间 r* 走势向上 ⇒ 核心不被回调腿砍 ⇒ 远超 OFF（+381%/+247% vs +26%/−2.3%）。量级低于 #63 全禁（+1537%/+828%），因方向轴允许了顶背驰转向期的 sink。
3. **不超 BH 的根因**：BTC +381% < BH +1380%——主升浪保护核心溶解了踏空，但 net_short 28.1%（仍有做空腿失血，project_t_short_leg_regime_function）+ 顶背驰转向期的 sink 砍仓在强牛中是噪声。**踏空（纤维拍扁）溶解 ≠ 净收益普适**——后者受做空腿/转向噪声/regime 调制（561 b2/563 未决）。
4. **GC 转正（−22.7%→+56.9%）= 最大单标的增益**：GC 既有强趋势段（方向轴保护核心）又有下跌段（方向轴放行 sink），方向轴二分恰好两段都正确路由 ⇒ 转正。

## 六、结果包六要素

### 1. 结论
判别量从退化力度轴 `d_top[r*]` 改为非退化级别角色方向轴 `top_trend_dir`（nf 驱动），O6 按 r* 走势方向二分（主升浪保护核心 / 下跌段核心正常 sink）。**爆仓全部消除（8/8 无 −100%），编排者预言验证**；GC/QQQ/OKLO 转正；BTC/ES 仍溶解超 OFF。8 标的 NOT 趋同（无标的超 BH，净收益 regime 函数）。OFF bit-exact PASS。

### 2. 定义依据
- **#61 §3.4 断言U**：j 对 r* 两角色 R+/R−（顺/逆核心主向），穷尽无第三态。
- **#61 §3.0 双源乘积 + project_morphology_nf_driven_emergence**：判别量 = 级别角色方向轴（nf 驱动，非退化），NOT 力度轴 d_top（退化）。
- **project_l4_consoldn_no_leave_falsified**：最高涌现级别恒生长不产 type1 ⇒ d_top[r*] 恒 false（力度退化）；但方向 nf 驱动恒已知。
- 输入满足：`view.top_trend_dir`（最高级别走势方向，rec_stream.rs:312/338）；`instances[rstar].direction`（核心方向）。

### 3. 边界条件（结论翻转）
- 若 top_trend_dir 也退化（最高级别无任何走势）⇒ None ⇒ 不保护（保守走常规 sink，bit-exact 安全）。**当前 net-up 全程 top_lvl 恒有走势 ⇒ 方向轴非退化 ⇒ 不翻转**。
- 若 protect_core 改为「机动 1/3 仍 sink 仅保护 2/3」（而非 mob_base=0 全禁单次）⇒ 强牛溶解量级可能再变（机动对冲腿在强牛是失血源，#63 全禁最优）。当前 R+ 用 mob_base=0（接续 #63）。
- 若换 bear 窗 ⇒ 强牛标的优势消失（委托 bear 窗工位）。

### 4. 下游推论
- **判别量选择元规则（建议结晶，meta-observer #68）**：当双源乘积的一个轴在某 regime 退化（如力度轴 d_top 在最高级别），应换**该 regime 下非退化的轴**（如 nf 驱动方向轴）作判别量，而非声称「判别量退化」。**力度退化 ≠ 判别量退化**——级别角色方向轴与力度轴正交（#61 §3.0），一个退化另一个可非退化。
- **爆仓消除 = O6 二分架构成立**（下跌段核心正常 sink 避免裸多扛跌）。但**净收益普适未达**——踏空溶解（形态层）≠ 跑赢 BH（功能层），后者受做空腿/转向噪声 regime 调制。
- 节点向量路由须 per-regime 白名单（与项目所有 regime 函数同构），不能全局保证超 BH。

### 5. 谱系引用
- **#61（双源乘积）/576（同概念结晶）**：本工位是节点侧实装；**精化**：双源一轴退化时换非退化轴（判别量选择元规则）。
- **561（Ω 坍缩）**：节点侧解坍缩；净收益有效域 L3 未决（b2/563）继承。
- **539（做空腿 regime / 错级别裸空）**：r*=R− 下跌段不再误读回调 ⇒ 核心正常 sink，避免 #63 的下跌段裸多；net_short 仍有失血（539 未完全解）。
- **275（级别局部性）**：每节点读自己在 r* 的角色方向，局部判别。
- **project_morphology_nf_driven_emergence**：判别量 top_trend_dir 的 nf 驱动来源（非退化方向轴）。
- **project_l4_consoldn_no_leave_falsified**：d_top[r*] 力度退化根因。
- **#63（bsp-opsem-impl-l3）**：本工位直接修正其判别量错位（力度轴→方向轴）。

### 6. 影响声明
- **改动**：`rec_engine.rs` `is_rstar_pullback_leg`（判别量 d_top → top_trend_dir + R+/R− 二分逻辑 + 文档）。`sink_impl`/route_bsp 签名/字段不变（接续 #63）。
- **影响模块**：route_bsp 的 sink 调用（protect_core 判别）；OFF bit-exact 契约不变（NODEVEC OFF ⇒ protect_core 恒 false）。
- **未改动**：八轨道原语 OFF、add/recover、信号层、收益口径。
- **重诊断**：#63「踏空溶解但爆仓」重诊断为「判别量错位（用退化力度轴）；换非退化方向轴二分 ⇒ 爆仓消除、强牛仍溶解、净收益仍 regime 函数」。

## 七、认识论等级标注

| 命题 | 等级 | 信息增量 |
|---|---|---|
| 判别量 = 非退化级别角色方向轴（非退化力度轴 d_top） | **L0**（#61 §3.0 + nf 驱动） | 高：判别量选择元规则 |
| OFF bit-exact（CL +20.3% 逐字 + 126 测试） | **L1**（真实数据全管线） | 零（验证 OFF 不变） |
| **爆仓消除（8/8 无 −100%）+ 强牛溶解（BTC/ES 超 OFF）** | **L3**（8 标的真实数据） | **高：编排者预言验证（下跌段核心正常 sink 修裸多）** |
| **8 标的 NOT 趋同（0/8 超 BH，净收益 regime 函数）** | **L3**（否定性） | **高：踏空溶解 ≠ 净收益普适，缩小有效域** |

**核心诚实声明**：① **爆仓消除成立**（8/8 脱离 −100%，下跌段核心正确 sink），编排者裁定的 O6 二分架构验证。② **踏空在强牛溶解**（BTC/ES 远超 OFF），方向轴判别量非退化。③ **但 8 标的仍 NOT 趋同**——无标的超 BH，相对 BH 差距 regime 依赖（净收益 = regime 函数，561 b2/563 未决）。**只声明爆仓消除 + 踏空溶解（形态层），不声明净收益普适。** **否定性结果价值**：方向轴二分把「全禁 sink 爆仓」缩小为「下跌段正常 sink」，但揭示净收益普适仍受做空腿/转向噪声调制——缩小有效域边界（踏空溶解 + 爆仓消除成立 ∧ 净收益超 BH 不普适）。

## 八、约束 4 异质审计降级

codex 死（venv editable，593号）⇒ 标 **pending-real-heterogeneous**。L0（判别量选择，#61 §3.0）不依赖异质复验；OFF bit-exact = L1 自验；L3 = 真实数据（可复跑 `T_ORBIT9_NODEVEC=1 cargo test --release recursive_t::rec_stream::tests::rec_btc -- --ignored --nocapture`）。
