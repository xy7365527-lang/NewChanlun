# L3 econ 门过滤率离线探针（χ / Nest-Xzd 接入的机制影响）

- **工位**：只读探针（分支 `kimi-nest-mainline-20260717`，worktree `/tmp/kimi-nest-mainline`）
- **数据源**：`/tmp/m8_opsem_p3fold/trades.jsonl`（518 笔 opened 订单；字段拆分见 `m8-opsem-trades-breakdown-20260719.md`）
- **日期**：2026-07-19
- **纪律**：090（照实否定合格）；v3 硬禁令（不引入概率推断作决策基础、不回测策略、不假设 EMH）；纯只读——未改 `rust/`，未 git mutation，主仓未写。
- **方法边界（先声明，后论证）**：trades.jsonl 只 dump **opened** 交易（`rust/src/theta_v0/backtest/runner.rs:1602` `dump.mark_entry(i)` 在开仓路径），**不含 Γ_t 全候选集**。因此本探针能算「若 χ 作用于已开仓候选的 z 桶，哪些桶会被拒」（机制投影），**不能**测候选侧的原始拒绝率（候选级 dump 不存在）。所有「过滤率」数字是**静态全样本 μ 的边际投影**——真实接入是 walk-forward 因果 μ，逐 bar 演化，离线不可复现；凡涉幅度一律标【待实测】。LCB 的 mean/std/√n 演算是 χ 门自身定义（`selector.rs:323`）的确定性求值，非新增统计推断。

## 1. 现状代码锚（三处事实）

**① χ≡1 全覆盖现状**：`runner.rs:707`（overlay 入口 `None, // χ≡1 全覆盖`）；fill loop 分支 `runner.rs:1353-1361`——`Some(ctx)` 走 `filter_gamma_with_admission`，`None => step_gamma.clone()`（原候选集，bit-exact 不变）。χ 端到端入口 `runner.rs:511-526 run_theta_v0_pi_chi` 由 `config.risk.chi_theta` 驱动，本 dump 运行未启用。

**② χ 门定义**：`selector.rs:357-388 filter_gamma_with_admission`——每个方向候选构 z（`z_of_candidate`，`selector.rs:267-270`），准入量 = `est.mu_lcb(&z, z_alpha)`（`shrink_tau_sq=None` 路径），`chi_t(admission, θ, ...)` 判 `LCB > θ`（严格大于，`selector.rs:124`）。LCB = `mean − z_alpha·std/√n`（`mu_estimator.rs:468-472`）；**n<2 或空类 ⟹ None ⟹ 由 `treat_empty_as_pass` 裁决**（`selector.rs:325-330`：false=无证据不交易，true=默认放行）。μ 观测值 X_γ = 绝对 unlevered PnL（`mu_estimator.rs:364-372 marginal_return` 复用 `trade_abs_pnl`）——与 trades.jsonl `pnl_raw_unlevered` 同量纲（差异仅费扣口径，照实标注：下文投影用 `pnl_raw_unlevered` 原值，未加费）。z 桶键 = `MuClass` 15 维（`mu_estimator.rs:92-137`：level/δ/i_class/parent_dir/short_swing/position/horizontal/force_state/σ_higher/cand_channel/nest_depth/origin_level/risk_mode/t_stage/eta_bucket）。

**③ Nest/Xzd dump-only 现状**：`runner.rs:1309-1310`（G3 注释：「π 路径候选不经 Nest/Xzd 准入门（cand_channel/nest_depth None 诚实口径）」）；`ext_i` 装配 `runner.rs:1322-1327` 只填 risk_mode/t_stage/eta_bucket 三维，nest 三维恒 None；`runner.rs:1613` nest_depth 仅进 opsem 快照；`runner.rs:2313-2316`（R5-1 铁律：「**不进 entry_z/MuClass/μ 桶键**」）。

## 2. 090 照实否定：m8 报告 D5 的解读不成立

m8 报告 D5 称「χ 门确实过滤了 γ=0 的候选（确认项）」。**否定**：本 dump 运行于 `chi=None`（§1①），`step_gamma_trade = step_gamma.clone()`（`runner.rs:1360`），字段 `gamma_count_chi_filtered`（`runner.rs:2428`）记录的是**未过滤的 |Γ_t|**（`runner.rs:1622` + `runner.rs:2274` 注释「χ 过滤后」在 χ≡1 下恒等于原集）。「γ=0 零笔」是「开仓需 ≥1 候选」的**同义反复**——不开仓的 bar 根本不进 trades.jsonl（§0 方法边界）。χ≡1 下不存在任何过滤动作可供确认。D5 应降级为「字段语义核实：该字段当前 = |Γ_t|，非门生效证据」。

## 3. ④ χ 接入（chi=Some）预期过滤率——机制投影

### 3.1 静态全样本边际投影（z_alpha=1.645，θ=0，逐桶 LCB 确定性求值）

| 桶键（边际投影） | 放行桶 | 被拒订单占比 | 被拒桶 pnl 和 | 备注 |
|---|---|---:|---:|---|
| `dir` | Long（LCB=+31.79） | 234/518 = **45.2%** | −16464.99 | 拒掉全部 Short；留下的 Long pnl 和 +23826.69 |
| `level × dir` | (0,Long)（LCB=+19.84） | 293/518 = **56.6%** | −4237.60 | 留下 +11599.30；**L3-Long（+10985.85）也被拒**（LCB=−40.70） |
| `level` | 无 | 518/518 = **100%** | +7361.70（全部） | L0 mean +1.10/std 444 → LCB −33.97；L3 mean +477.65/std 1511 → LCB −40.70 |
| `level × bsp_class_min` | 无（(3,3) n=1 → None 分支） | **100%** | 全部 | (3,3) 的 1 笔落 treat_empty 裁决 |
| `nest_depth` | 无 | **100%** | 全部 | depth=0 mean +53.13/std 643 → LCB −20.41 |
| `gamma_count_chi_filtered` | γ=3（LCB=+6.47） | 489/518 = 94.4% | +5061.42 | 投影键，非 z 维，仅佐证量纲问题 |

（演算脚本：Python3 逐行解析 JSONL + Welford 等价全量 std；输入只读。）

### 3.2 机制发现（三条，均可在代码层定位原因）

- **F1（量纲主导）**：X_γ 为绝对额 unlevered PnL（`mu_estimator.rs:360`「返回绝对收益，非归一化」），该口径下**每笔 std（266~1511）普遍 ≫ mean（1.10~477.65）**。LCB = mean − 1.645·std/√n 在 n∈[2,434] 的边际桶上几乎恒 <0——**z_alpha 罚项淹没 mean 信号**。χ 门在该量纲下退化为「放行唯一高 mean/低 std 比的桶」（本样本恰好是 Long 单边桶，即 m8 D2 方向偏差的载体），不是 μ 鉴别。L3——全部盈利的 149% 来源——在 level 键与 level×dir 键下 LCB 均 <0，**裸接入 χ(θ=0, z=1.645) 会把盈利引擎一并拒掉**。
- **F2（稀疏性单调放大）**：边际投影（1-2 维）已 100% 拒绝；真实 z 是 15 维全互斥键（`mu_estimator.rs:92-137`），每桶 n 只会更小 ⟹ std/√n 更大 + n<2/空类更多 ⟹ **实际过滤率 ≥ 边际投影**。在 `treat_empty_as_pass=false` 下，walk-forward 冷启动期全表 None ⟹ **空仓**；`=true` 下冷启动全放行，过滤只在桶攒到 n≥2 后逐个「点亮」。**过滤率由 (z_alpha, treat_empty_as_pass, 量纲, θ) 四个参数主导，而非由 μ 的结构信号主导**——518 笔对 15 维键，稀疏是结构性的，不是数据量再大一点就能绕过。
- **F3（对 `gamma_count_chi_filtered` 分布的推演）**：chi=Some 后该字段 = |Γ_t^trade|（`runner.rs:1622` 同源）。机制方向：被拒 z 桶候选所在 bar 的计数下降；被拒桶占比高的 bar 出现 γ=0 ⟹ 不开仓 ⟹ 从 trades.jsonl 消失（分布整体左移、总单数下降）。本样本分布 {1:335, 2:147, 3:29, 4:2, 5:5} 是 χ≡1 下的 |Γ_t|。γ∈{4,5} 高 gamma 子集（7 笔，pnl 和 −1549.45）是否被拒**取决于其 z 桶的 walk-forward μ**，本 dump 无候选级数据，【待实测】。**预期过滤率点估不可离线给出**——walk-forward μ 逐 bar 演化，静态投影只给出边界：dir 键 45.2% / level 键 100%，真实值落在由 F2 放大的区间内。

### 3.3 ④结论

χ 接入的过滤率**不能被当前设计参数良性控制**：在绝对额量纲 + z_alpha=1.645 + θ=0 下，机制上必然（a）过滤率 ≥45% 且大概率趋近全拒，（b）被拒集同时包含最大亏损源（Short，−16465）与最大盈利源（L3，+10986）——**对 net_r 的方向不定**：拒 Short 得 +16465，拒 L3 失 10986，二者同源于「LCB 罚项淹没 mean」这一量纲事实而非结构鉴别。n_orders 方向**确定下降**（χ 只减不增候选）；net_r 方向**不定**，【待实测】。

## 4. ⑤ Nest/Xzd 准入接入预期过滤率

两种接法语义不同，分开论证：

**⑤a 进 μ 桶键（cand_channel/nest_depth/origin_level 从 None → Some）**：
- 机制：z 维数 +3 ⟹ 每桶 n 进一步稀释 ⟹ F2 的稀疏性放大加剧。**在 z_alpha>0 ∧ treat_empty=false 下过滤率单调不减**——这是纯维数效应，与 nest 的经济价值无关。
- nest 维本身的边际证据：depth 分桶 pnl 和 {0:+10997.24(n=207), 1:−4754.86(n=146), 2:+268.30(n=101), 3:−686.94(n=44), 4:+1537.96(n=20)}；全部 5 桶 LCB<0（F1 同因）。depth=1 看似负桶，但 depth×dir 交叉显示其亏损 **−7121.66 来自 Short、+2366.80 来自 Long**，depth×level 交叉显示 −2768.63 来自 L1——**depth=1 的负贡献与 m8 D2（方向）/D8（L1）共线，不是独立亏损信号**。故「nest_depth 进桶键能鉴别」在当前数据上无独立依据；确定的只有稀疏性代价。
- 该维度是否值得保留的判据已存在：`mu_estimator.rs:694 oos_gated_drop`（§11 OOS-value-gated 删维骨架）——**接入前应先过 OOS 删维评估**，【待实测】。

**⑤b 作为候选侧准入门（nest_confirmed / Xzd 门在 Γ_t 构造前拒候选）**：
- **照实声明能力边界**：trades.jsonl 只含 opened 交易（§0），**候选侧拒绝率本数据不可测**。nest_confirmed=518/518 True（m8 D7）只证明「opened 子集上该门常开」，对被拒候选一无所知——测它需要 Γ_t 候选级 dump（当前不存在，`runner.rs:1602` 只在 opened 路径 mark）。这是接入前的 **L0 前置缺口**。
- 若门语义是「无 nest 证书不开仓」：本样本 518 笔全部持证书 ⟹ 对 opened 子集过滤率 0%；对 Γ_t 全候选的过滤率未知【待实测，需候选级 dump】。

## 5. 对 n_orders 与 net_r 的机制影响方向（不预测幅度）

| 接入 | n_orders 方向 | net_r 方向 | 依据 |
|---|---|---|---|
| χ（当前参数量纲） | **下降**（χ 只滤不加，`runner.rs:1353-1361`；treat_empty=false 下冷启动空仓） | **不定**——拒 Short 亏损（+16465 效应）与拒 L3 盈利（−10986 效应）同源并发，净符号取决于 walk-forward μ 落到哪些桶【待实测】 | §3 F1/F2 |
| Nest 三维进桶键 | **下降**（稀疏性单调放大，纯维数效应） | **不定且偏险**——nest 维无独立鉴别证据（§4⑤a 共线），增加的只是 LCB 罚项与 None 桶 | §4⑤a |
| Nest/Xzd 候选侧准入门 | 方向下降（门只拒不放），**幅度不可测**（无候选级数据） | 不可测 | §4⑤b |

共性机制：任何接入都**先降 n_orders、后谈 net_r**；n_orders 下降本身改变 μ 训练样本流（被拒桶不再产生观测 ⟹ 桶永远停在 None/稀疏态 ⟹ treat_empty=false 下永久拒绝）——**χ 接入是自锁反馈**，冷启动参数选择即长期行为选择。

## 6. 对 L0/L3 接入的决策支持

1. **量纲裁定是 χ 接入的前置阻塞项**（非参数调优）：绝对额 X_γ + z_alpha=1.645 在 518 笔量级退化为方向门（§3 F1），且拒掉 L3 盈利引擎。接入前需裁定：X_γ 归一化口径（÷nav_base，`mu_estimator.rs:360` 已留调用方归一化口）、θ 的量纲匹配。未裁定前接入 = 声明与能力不符（090 禁）。
2. **(z_alpha, treat_empty_as_pass) 组合必须先定语义**：false+15 维 z+518 笔 = 结构性空仓（§3 F2）；true = 冷启动期 χ 名存实亡。两者都是合法语义（`selector.rs:108-111`），但产出完全不同的系统，需在 L0 裁定，不能留给默认值。
3. **Nest 三维进桶键：建议先过 `oos_gated_drop`（`mu_estimator.rs:694`）再决定**。当前证据：depth=1 负贡献与 dir/level 共线，无独立鉴别依据；稀疏代价确定、收益未定【待实测】。
4. **Nest/Xzd 候选侧准入门：先补 Γ_t 候选级 dump（L0 工位），再谈过滤率**。opened-only 的 trades.jsonl 在原理上测不了准入门（§4⑤b），任何绕过此缺口的「过滤率」数字都是伪造。
5. **m8 D5 需更正**（§2）：当前 dump 无 χ 门生效证据；「χ 门工作正常」的确认项不成立，降级为字段语义核实。
6. 所有幅度结论（χ 真实过滤率、net_r 净效应、Nest 准入拒绝率）**一律待 walk-forward 实测**（L2/L3 工位，`runner.rs:960-962` 诚实声明：in-sample μ 过滤 = 泄漏，只证 L1 选择器逻辑）。本探针只交付机制方向与边界，不交付点估。

## 附：复现与锚点

- 演算：Python3 逐行解析 `/tmp/m8_opsem_p3fold/trades.jsonl`，`defaultdict` 分桶 + 全量 std（与 `mu_estimator.rs:380-404` Welford 数值等价），LCB=mean−1.645·std/√n 逐桶求值；输入只读，无写入。
- 代码锚：`rust/src/theta_v0/backtest/runner.rs:707`（chi=None）、`:1353-1361`（χ 分支）、`:1309-1310`（Nest None 口径）、`:1322-1327`（ext_i 三维）、`:1602`/`:1613`（opsem dump/nest_depth）、`:2274`/`:2313-2316`/`:2428`（dump-only 铁律与字段）、`:511-526`（chi 入口）；`selector.rs:357-388`（filter_gamma）、`:323-330`（LCB/None 语义）、`:116-128`（chi_t）；`mu_estimator.rs:364-372`（X_γ 绝对额）、`:468-472`（mu_lcb）、`:92-137`（MuClass 15 维）、`:694`（oos_gated_drop）。
- 文档锚：`chanlun/review-results/m8-opsem-trades-breakdown-20260719.md` §0/§①/§②b/§⑦/§9 D2/D5/D7/D8。
- 未做（v3）：无胜率/夏普等推断；无参数寻优；无策略回测结论；无样本外外推。
