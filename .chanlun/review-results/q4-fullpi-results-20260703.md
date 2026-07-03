# q4 π^full 回测 — 结果包（goal g-full-spec-pi 最后验收项）

- **工位** swarm/ws-q4pi | task #135 | **冻结 commit = 388a9ebc16**（prereg-q4-fullpi-20260703.md）→ harness commit d0dc985689 → 跑数（顺序满足铁律：冻结先于跑数，fail 条件① 未触发；冻结后未改 prereg 文本，fail 条件② 未触发）
- **认识论**：R1/R2/R3/R6 = **L2**（BTC 真实全历史 461 万 bar walk-forward OOS + CL 对照）；R4 = **L3**（7 品种 σ̂ 归一化池化）。判据/三态逻辑 L0；harness 日期助手自检 L1。
- **原始产出**：/tmp/q4_r{1,2,3,4,6}_*.log、/tmp/q4_fullpi_policy.md、/tmp/wv_full_rows.md、/tmp/wv_fullz_rows.md、/tmp/wv_uclass_rows.md、/tmp/wv_xsym_pooled.md、/tmp/q4_r6_deadgate_full.log

---

## 1. 结论（三态判定）

**π^full（§16 十环链全接通）三态判定 = INCONCLUSIVE（全局），V=0 全口径——无 confirmed 正 alpha。** 与全战役至今一致；否定性结果照实，残差口径 `Y_i=δ(H−B̂)−C` 未动摇。局部 powered FALSIFIED 桶存在（负 alpha 在若干桶被确认，见 §3）。

| 口径 | 残差 n | 桶 | V | F | I | 全局 |
|------|-------|----|---|---|---|------|
| R1 4元组（BTC walk-forward OOS，**新 typed exit 口径**） | 2209 | 25 | 0 | 1 | 24 | **INCONCLUSIVE** |
| R3 full-z 并列 | 2209 | 40 | 0 | 3 | 37 | INCONCLUSIVE |
| R3 UClass 并列 | 2209 | 13 | 0 | 2 | 11 | INCONCLUSIVE |
| R4 池化（7 品种 σ̂ 归一化，L3） | 8596 | 42 | 0 | 11 | 31 | **INCONCLUSIVE** |

**★本轮最重要的否定性发现：旧口径主桶正点估计 = 出场口径伪影。** 旧 τ^reverse 口径主桶 (L0,type3,买,σ0)：n=1597–1613，mean +178~+185，LCB +102~+108（p3/f3 屡次存活）。**新 typed exit 口径同桶：n=1002，mean(Y) = −0.37，LCB = −42.6**——正点估计完全消失。这是 G4 边界条件⑤的正式对照结论：τ^typed ≢ τ^reverse，且旧主桶的正 μ̂ 不是 alpha，是"下一个任意反向信号"出场规则的口径伪影。旧口径全部 alpha 数字（W-VERIFY PASS 主桶/econ-663 μ̂ 表/奇偶交替）就此在新口径下失效确认。

## 2. 定义依据

- **判据**：主判据 LCB_OOS(μ(z,a))>0（§12 口径，z_α=1.645，非 p<0.05）；三态 = decontam `classify_bucket`（VALIDATED ⟺ powered ∧ LCB>0 ∧ perm_p<0.05；FALSIFIED ⟺ powered ∧ UCB<0；否则 INCONCLUSIVE）+ 逐桶功效门 `n_eff≥(1.645·CV)²`（663/665/667）。输入满足条件：全部残差经 `typed_ledger_from_bars`（生产 π fill loop）→ `build_mu_from_bars`，treatment-on-the-treated 口径（PDF §9，τ^reverse 已删除）。
- **π^full**：G1–G7+GAP3+hl13+margin 十环全在 HEAD（冻结基 a043abba51 后继）；Arm1 显式 `enforce_gross_cap=true` + margin=Some(CME-simple)（prereg ①④）。
- **桶键**：聚合基 δ-free `(level, bsp_class(), parent_dir)`，σ_higher/force 不进分层（prereg ③，三处同批不动）；分层键 `(ℓ,h_bucket,time_block,σ^H)`。

## 3. 跑数读出（逐项对 prereg）

### 3.1 R1 μ 新口径（BTC，5 个 OOS 窗）

- 残差 2209（旧口径 3468，**−36%**）。**prereg ② 的"两个数量级下降"外推未兑现**——16K 半窗冒烟（18 腿）的密度不代表全窗（每窗实测 362–537 腿），照实订正：下降是实质的但为同数量级。μ 类数（6 月 train 窗）21–33 类。
- **hl13 效应可见**：桶覆盖 L0–L5 六个级别（旧口径主要 L0）——level≥1 的 2364 三类候选真实进入生产开腿（R5 兑现：L1 桶 n=33–79，L2–L5 小样本）。
- 主桶 (L0,type3,买,σ0)：n=1002，n_eff=1002，mean −0.37，LCB −42.6，UCB +41.8，**perm_p=0.350（非退化）**——prereg fail 条件③ 未触发，δ-共线检查过。state=INCONCLUSIVE（mean≈0 致 CV 爆炸，未 powered——这是"点估计归零"的表现而非样本不足）。
- 唯一 FALSIFIED：**(L4,2类,卖,σ0) n=3**，mean −20699，UCB −2953（powered 小桶，弱证据）。

### 3.2 R2 π^full 四臂 policy（BTC 6 窗 + CL 6 窗，walk-forward + p3 可比单折）

**χ teap=false（spec 语义主臂 Arm1）下 π^full 在 12 窗中 9 窗零订单**——train 段无任何 LCB>0 的 μ 类 ⟹ χ 全拒 ⟹ 空仓。这不是 bug 而是 §12 语义的忠实执行：「无正边际收益证据时不交易」+「无 confirmed alpha」⟹ 不交易。3 个开仓窗（train 段偶现 LCB>0 类）OOS 全负：

| 窗 | Arm0 无χ | Arm1 π^full | 读出 |
|----|----------|-------------|------|
| BTC wf11* | −86.1M / 32329 单 | −52.2M / 15467 单 | χ 减损 39% 仍深负 |
| CL wf12 | −48.3K / 19511 单 | −4.5K / 8664 单 | χ 减损 91% 仍负 |
| CL wf14* | −64.8K / 31442 单 | −66.7K / 48327 单 | **χ 后更差且订单反增**（χ 剪枝改变净头寸轨迹致再平衡订单增加——路径效应，非单调过滤） |

聚合（wf 窗）：BTC Arm0 −214.1M/137394 单 vs Arm1 −52.2M/15467 单；CL Arm0 −315.2K/149959 单 vs Arm1 −71.2K/56991 单。**train 段 LCB>0 不在 OOS 保持——与 R1 的 V=0 自洽。**

**G7+margin 在实测轨迹上零 binding：Arm1 ≡ Arm3 逐位相同（全部 12 窗 ΔΣpnl=0、Δorders=0）**——毛头寸 cap 从未触发缩放、CME-simple margin 从未把 RiskMode 推离 Normal 至改变订单流。π^full 的 §11 一致性分量合法接通（enforce_gross_cap=true 真跑）但在当前 sizing/nav 口径下不 binding。**有效域**：此结论限于 CME-simple 单段口径（prereg ④ 声明），非 SPAN/实盘保证金。

**Arm2（teap=true 敏感臂）**：接近无χ基线（空类全放），大量交易且净负（BTC 聚合 −157.8M）。Arm1−Arm2 差分坐实 prereg ② 的预言：新口径下 `treat_empty_as_pass` 是 χ 行为的主宰参数（false≈全拒/true≈全放）。

**Arm0 漂移归因（差分 d）**：BTC p3fold −14.97M/34647 单 vs p3 档案 −15.68M/33067 单；CL −61.4K/28691 vs 档案 −54.5K/20483。HEAD 无χ路径相对 p3 档案**已分叉**，归因 = 上游变更集合（hl13 +2364 信号为已知最大源；#115 force 热路由/#124 G5 统一/GAP3 realize 同批）。**P2/P3/P4 逐 bar 触发计数未插桩**（prereg ⑤「可得则报」——本轮不可得，照实）；间接证据：Arm1≡Arm3 表明 TW→订单流通道未在 χ 臂产生差异。

### 3.3 R3/R4（full-z/UClass 并列 + L3 池化）

- full-z 40 桶 V0/F3/I37（p3 旧口径 52 桶 V0/F9/I43）；UClass 13 桶 V0/F2/I11。
- 池化（σ̂ 归一化）：8596 残差、42 桶 V0/F11/I31。**大样本 FALSIFIED**：(L0,type3,卖,σ0) 池化桶 n=3452，mean −1.36，UCB −0.51（powered）——**卖向主桶负 alpha 在池化口径被确认**（否定性结果，缩小有效域）。
- f3 档案对照（差分 e）：旧口径 BTC 28 桶 V0/F5/I23 / 池化 29 桶 V0/F5/I24（14678 残差）→ 新口径如上。V=0 不变；F/I 分布随口径与样本重构。

### 3.4 R6 XZD C3 高级别 Type3 消费重新评估

- 默认 300K 窗：死门基线重封断言通过（lvl≥2 routed=75/sub_bsp_type3_total=3150）。
- **全历史 461 万 bar（210s）**：Xzd routed=3701，C2 成立=1401，**旧 C3（same_side_same_center）成立=0**——全历史仍全级恒 0，「旧 C3 死门」维持；lvl1 新判据（新中枢+突破）routed=2626，new_center_exists=0，breakout_ok=0——**命中恒 0 的终局不变量在全历史口径维持**（codex #55/#56 健康）。小转大门通过 399/11939（3.34%）。**prereg fail 条件④ 未触发，无行为变化上浮。**

## 4. 边界条件（结论翻转）

1. **INCONCLUSIVE→VALIDATED**：任一桶同时 powered ∧ LCB>0 ∧ perm_p<0.05。当前最近的桶都差两项以上（主桶点估计≈0）。提功效路径（池化/更长窗）已在本轮试过——池化产出的是更多 FALSIFIED 而非 VALIDATED。
2. **"主桶正点估计=口径伪影"翻转**：若未来证明 typed exit 实装错误（出场时点不符 PDF §9 五枚举语义）则口径对照失效——现有 1453 测试+G4 端到端见证反证此可能。
3. **"G7/margin 零 binding"翻转**：更高杠杆 sizing、更小 nav₀、或真实分段 MM 快照（更高 MM）下可 binding——本结论有效域=CME-simple 单段+现行 sizing。
4. **χ 空仓翻转**：若换更粗桶键（提每类 n）或降 z_α，χ 可放行更多——那是新 estimand，须新 prereg，不在本冻结内。
5. **Arm0 漂移归因精化**：P2/P3/P4 逐 bar 插桩后可分离 TW 通道贡献——现归因为变更集合层面。

## 5. 下游推论

- **goal g-full-spec-pi 验收闭合**：π^full 十环链全接通、全历史 L2 跑批完成、判据兑现——**工程验收成立，alpha 验收=诚实 INCONCLUSIVE**。M1 里程碑语义：完整策略已实装并被完整口径回测，当前 estimand 下无正收益边缘；χ 门在 spec 语义下正确表达「无证据不交易」。
- **受影响旧结论（g4-impl §4 清单）本轮全部重估落地**：
  1. `project_wverify_alpha_retest_pass`（旧主桶 PASS）——**新口径下主桶点估计消失，该结论正式作废**（非仅"待重估"）；
  2. `project_l3_cross_symbol_btc_idiosyncratic`——新口径池化 V=0/F=11，正号桶消失，卖向主桶 FALSIFIED；
  3. `project_oddeven_mu_identity`/econ-663 μ̂ 表——旧口径数字全部失效确认（其"口径特有放大"预判被本轮坐实到根：整个正 μ̂ 都是口径伪影）；
  4. perm_p 数字——新口径主桶 0.350（非退化，桶键结构性免疫维持）。
- **χ treat_empty_as_pass 成为一等策略参数**：新口径下它主宰 π 行为（全拒/全放二态）。若未来要"有限证据下部分交易"，需要新的空类语义设计（如 UClass 降维查询 fallback）——新 estimand，新 prereg。
- **G7/margin**：机器正确、约束不 binding——把「毛约束/保证金约束改变了结论」的任何声明堵死在有效域外（当前 sizing 下它们是休眠守卫）。
- **hl13**：level≥1 候选真实进入生产开腿与 μ 表（L1–L5 桶出现），但全部 INCONCLUSIVE/小样本——高级别买卖点当前无 alpha 证据。

## 6. 谱系引用

- prereg-q4-fullpi-20260703（冻结 388a9ebc16）：七项全部逐条兑现，零偏离（除 ② 样本量外推按实测订正——prereg 自身预留"以跑数为准"）。
- g4-impl（#134）边界条件⑤：本轮=其正式对照，τ^typed≢τ^reverse 且旧正点估计翻转为伪影。
- codex-q1-spec-rulings G4 终裁（"为生产永远不会入场的候选估 μ = 训练/生产分布错配"）——本轮 L2 证据坐实该裁定的全部分量。
- p3（fullz-policy）/f3（f3-fugue）：差分档案；其 V=0 结论在新口径下维持，其主桶正点估计被否证。
- 663/665/667/675/231/090；i_class×δ 共线（memory）——主桶 perm_p=0.350 非退化再次坐实桶键免疫。
- dx-deadgate-20260703（#137）：R6 基线来源；本轮升级到全历史口径确认。

## 7. 影响声明

- **代码改动**：`rust/src/theta_v0/backtest/wverify_run.rs` +169 行（q4_fullpi_policy 四臂 harness + 日期助手 + L1 自检，全部 `#[ignore]`/tests 增量，生产逻辑零改动，lib 1453 全绿零回归）。
- **产出**：本结果包 + prereg（388a9ebc16）+ harness（d0dc985689）+ /tmp 全套跑批产物。
- **未改**：残差减法/decontam/perm_test/桶键/生产 π 链/既有冻结文件/prereg 文本（冻结后零改动）。
- **memory**：`project_wverify_alpha_retest_pass` 更新为作废；新增 q4 判决记忆。
