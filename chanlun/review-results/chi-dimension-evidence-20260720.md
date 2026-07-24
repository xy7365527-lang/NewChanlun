# wf61：χ 量纲三方案（绝对额/收益率/归一化）教义与数据证据收集

- **工位**：research 证据收集（不代裁——供编排者拍板；分支 `kimi-nest-mainline-20260717`，worktree `/tmp/kimi-nest-mainline`）
- **日期**：2026-07-20
- **票据**：wayfinder #61
- **纪律**：090（照实否定合格，不代裁）；v3 硬禁令（本文全部数据演算是 χ 门自身定义 LCB = mean − z_α·std/√n 的确定性求值，非概率推断；无回测、无胜率/夏普、无样本外外推）；只读调研——未改 `rust/` 一行，未 git mutation，主仓只读。
- **输入**：`买卖点alpha2.pdf`、`严格alpha.pdf`（pdftotext 提取）；`chanlun/review-results/gap1-chi-dimension-ruling-material-20260719.md`（下称裁定材料）§2；`/tmp/m8_win/wf7/trades.jsonl`（n=504）、`/tmp/m8_win/wf8/trades.jsonl`（n=518）。

## 1. 教义证据：两份 PDF 的 X_γ / alpha 定义（§12-§14 + 净值 alpha）

### 1.1 买卖点alpha2.pdf（pdftotext 提取，页码按 PDF 页脚）

- **§12「三类买卖点的严格 alpha 判定」（p.22/38）**：证书 `γ=(c,ℓ,δ,I_γ,t)`，退出时刻 `τ_γ = inf{u>t : 出现该声部的出场证书或风险退出}`，交易收益
  **X_γ = δ(P_τγ − P_t) − C_{t:τγ}**（提取文本 line 2147），其中买点 δ=+1、卖点 δ=−1。条件期望 `μ_{ℓ,δ,I} = E[X_γ | ℓ(γ)=ℓ, δ(γ)=δ, I_γ=I]`（line 2173），严格 alpha 条件 = `μ_{ℓ,δ,I} > 0`。
  - **量纲照实读**：X_γ 是**每单位价格差**（δ=±1 无qty、无÷价格、无÷时长）——PDF 在 §12-§14 全段**未规定任何量纲/归一化条款**（与裁定材料 §0 结论一致，本文独立复核确认）。
- **§13「严格"解决方案"」（p.22-23/38）**：目标改为「只交易具有正边际条件期望的买卖点证书」，选择函数 `χ_t(γ)=1 ⟺ μ(γ)>θ ∧ RiskOK(γ,x_t) ∧ ConflictOK(γ,A_t)`（line 2239），θ≥0 是成本和风险门槛——χ 门的教义出处，**未涉 X_γ 量纲**。
- **§14「正收益定理」（p.23/38）**：若所有被交易证书 `E[X_γ|F_t] ≥ η > 0`，则 `E[R] ≥ η·E[|T|] > 0`（line 2274/2281/2290）；并自承该假设「不能从缠论语法推出，只能由统计检验、模型假设或外部经济假设给出」（line 2319 后正文）。
- 另：裁定材料 §0 指出 alpha2.pdf 节号多义（合订本）——另有「§13 精细分类优势定理」为 15 维 MuClass 桶键落点；两义并存，均不涉量纲。本文复核：同一提取文本中确有「13. 严格"解决方案"」（line 2218）一处节号；精细分类优势定理段未在本文展开复核，沿用裁定材料标注。

### 1.2 严格alpha.pdf（净值 alpha 定义）

- **§1「先定义什么叫 alpha 范式」（p.8-9/27）**：净值递推 `W^π_{t+1} = W^π_t + N^π_t(P_{t+1}−P_t) − C^π_t`（line 769-771）；相对 baseline 的 alpha `α(π) = E[W^π_T − W^{π0}_T]`（line 782-784）；单期增量 `ΔR_{t+1} = ΔN_t(P_{t+1}−P_t) − ΔC_t`（line 790）；alpha 范式本质 = `E[ΔR] > 0`（line 804）。
  - **量纲照实读**：此处 `N^π_t` 是**净头寸**（含仓位量），ΔR 是货币额——即净值 alpha 教义写法的量纲是「头寸×价格差」的**绝对货币额**。但 N_t 是策略仓位决策（Θ_risk 层对象），不是 X_γ 的固有成分；且该 PDF 同样**没有任何条款要求 X_γ 必须带 qty 或必须归一化**。
- 两 PDF 合计结论（与裁定材料 §0 一致，独立复核成立）：**教义定义了 X_γ 的代数结构（δ·价格差−成本，per-certificate，全程持有），把量纲（绝对额/速率/相对收益）留为 extra-教义设计决策** ⟹ 量纲裁定属编排者裁定域，三方案都不「违教义」，区别在教义距离与机制后果。

## 2. 裁定材料 §2 机制推导要点 + 一处 090 更正

### 2.1 要点复述（裁定材料 §1-§2，:22-72）

- 判定代数：`LCB>0 ⟺ √n/CV > z_α`（CV=std/mean，裁定材料 :27）——量纲裁定的真实对象是「把与结构无关的方差源移出 CV」，不是把数字变小（P1 常数缩放不变性，:32）；逐笔异尺度因子 c_i 的截面方差**加性注入** std、抬高 CV（P2，:33）。
- 方案 ①（绝对额）：CV 含 qty/价格水平/时长三个非结构方差源（:35-38）；gate-filter F1 已否证其实测形态（罚项淹没 mean）。
- 方案 ②（%/bar）：移三源最彻底，但 D-a 改估计对象（per-certificate 收益→每 bar 速率，:55）、D-b 成本摊薄不对称（短持仓桶每 bar 成本占比放大，:56）。
- 方案 ③（÷入场名义）：移 qty/价格水平两源，估计对象不变；与 §12 字面距离最近；θ=0 语义与严格alpha 净值口径对齐；无新增失真（:62-72）。

### 2.2 090 更正：裁定材料「现状 = qty·δ·ΔP_net」的措辞与调用点不符

裁定材料 §0（:20）与 §2-①（:44）称实装 X_γ 含 qty 乘子（「qty 乘子是实装层引入的」）。照实核验全部 `MuObservation` 喂入点：

- **ledger 路径（主口径）**：`l3_delta_r_alpha.rs:266-269`——注释自承「X_γ = δ(P_exit−P_entry) − C（**qty=1 名义单位**，μ 是单位边际收益的类条件均值）」，调用 `marginal_return(entry_px, exit_px, 1.0, fee_rate, delta)`。**现状 ledger 口径 = 费扣 per-unit 绝对额，不含 qty**。
- **bin 路径**：`rust/src/bin/pi_bsp_timing.rs:370` 与 `:463`——`marginal_return(..., voice.qty, ...)`，**含真实 sizing qty**（该路径的 P2 qty 注入成立）。
- `marginal_return` 本身（`mu_estimator.rs:364-372`）是通用 qty 形参；其 docstring 的「返回绝对收益」指未归一化，不隐含 qty≠1。

更正后果：裁定材料 §2-① 的「qty sizing 噪声注入 CV」批判对 **ledger 现状口径不成立**（qty=1 无截面方差），只对 pi_bsp_timing bin 口径成立；① 的剩余非结构方差源 = 价格水平漂移 + 持仓时长两项。方案 ③ 相对 ①（ledger 口径）的机制收益相应减为「移价格水平源 + θ=0 跨期可比 + 防未来 qty 喂入」；对 ② 的 D-a/D-b 批判不受影响。本文 §3 数据同时给出两种 ① 变体（费扣 per-unit / ×units 代理）使两口径各自的后果可见。

## 3. 静态数据证据：wf7/wf8 三口径按桶 LCB 符号分布

### 3.1 方法与口径（纯确定性求值）

- 逐行解析 trades.jsonl；分桶键 = dir / level / level×dir（gate-filter §3.1 边际投影同构）；桶统计 = mean、样本 std（与 `mu_estimator.rs:380-404` Welford 数值等价）、LCB = mean − 1.645·std/√n（`mu_estimator.rs:468-472`；z_α=1.645、θ=0，同 gate-filter §3.1）；n=1 桶 LCB=None（不参与拒/放计数）。
- 五口径：
  - **S1 绝对额费前**：X = `pnl_raw_unlevered` = δ(exit−entry)（dump 口径，`runner.rs:2940-2943`；与 gate-filter 投影同口径）；
  - **S1f 绝对额费扣 per-unit**：long `exit(1−f)−entry(1+f)`、short `entry(1−f)−exit(1+f)`，f=3×10⁻⁴（1bp comm + 2bp slip，`reference-theta-v0.md` §Θ_exec:51；`runner.rs:327-328`）——**= ledger 现状口径**（`l3_delta_r_alpha.rs:269` qty=1）；
  - **S1q 绝对额费扣 ×units**：S1f×`units`（pi_bsp_timing bin 口径代理；照实：dump `units` 与 `voice.qty` 是否同值未证实，标「代理」）；
  - **S2 收益率 %/bar**：S1f ÷ entry_px ÷ max(exit_bar−entry_bar,1)（方案 ②，qty=1 下 ÷入场名义 ÷持仓 bar）；
  - **S3 归一化 ÷入场名义**：S1f ÷ entry_px（方案 ③，费扣后持仓期相对收益）。
- **交叉验证**：S1 在 wf8 复算 L0 桶 mean=+1.10408/std=444.229/LCB=−33.9733、L3 桶 mean=+477.646/std=1511.19/LCB=−40.7003、Short 占比 234/518=45.17%——与 gate-filter 报告 §3.1（L0 mean+1.10/std444→LCB−33.97；L3 +477.65/1511→−40.70；dir 拒 45.2%）逐位一致，方法学锚定成立。

### 3.2 LCB 符号分布总表（REJ = LCB≤0 @ θ=0, z=1.645）

| 口径 | wf7 dir | wf7 level | wf7 level×dir | wf8 dir | wf8 level | wf8 level×dir | 合并 dir | 合并 level | 合并 level×dir |
|---|---|---|---|---|---|---|---|---|---|
| S1 绝对额费前 | REJ 2/2 | REJ 4/4¹ | REJ 7/7¹ | **REJ 1/2**（Long PASS +31.79） | REJ 4/4 | REJ 5/6（L0×Long PASS） | REJ 1/2 | REJ 5/5 | REJ 7/8 |
| S1f 费扣（现状 ledger） | REJ 2/2 | REJ 4/4¹ | REJ 7/7¹ | REJ 1/2（Long PASS +10.25） | REJ 4/4 | **REJ 6/6**（费扣翻转 L0×Long：+19.84→−1.91） | REJ 1/2 | REJ 5/5 | REJ 8/8 |
| S1q ×units（bin 代理） | REJ 2/2 | REJ 4/4¹ | REJ 7/7¹ | **REJ 2/2**（Long 翻 REJ −4516） | REJ 4/4 | REJ 6/6 | REJ 2/2 | REJ 5/5 | REJ 8/8 |
| S2 %/bar（方案②） | REJ 2/2 | REJ 4/4¹ | REJ 7/7¹ | **REJ 2/2** | REJ 4/4 | REJ 6/6 | REJ 2/2 | REJ 5/5 | REJ 8/8 |
| S3 ÷入场名义（方案③） | REJ 2/2 | REJ 4/4¹ | REJ 7/7¹ | REJ 1/2（Long PASS +1.72e-4） | REJ 4/4 | REJ 6/6 | REJ 1/2（Long PASS +8.43e-5） | REJ 5/5 | REJ 8/8 |

¹ wf7 另有 L3 桶 n=1（LCB=None，不计）。

### 3.3 逐方案关键读数（全部确定性求值，非推断）

**方案 ①（绝对额）**：
- S1f（现状）wf8：dir Long n=284 mean=+62.34 std=533.64 CV=8.56 LCB=+10.25 PASS；Short n=234 mean=−91.67 LCB=−152.32 REJ。level 键全 REJ 含 **L3 n=23 mean=+458.34 std=1510.93 CV=3.30 LCB=−59.92**——gate-filter F1（罚项淹没 mean、拒掉盈利引擎）在费扣口径下原样复现。
- S1q（qty 注入演示）wf8：dir Long CV 8.56→**13.4**，LCB +10.25→**−4516.26 翻 REJ**——P2 机制（逐笔尺度因子截面方差加性注入 CV）的实测演示：同一批交易，仅乘逐笔 `units`，唯一 PASS 桶消失，χ 退化为全拒。
- wf7 下 ① 全键全 REJ（含 mean=+244.72 的 L1 桶：CV=3.51，n=29，√n/CV=1.54<1.645）。

**方案 ②（%/bar）**：
- **两窗口、三键、全部桶 REJ，无一 PASS**——② 下 χ 退化为全拒门，比 ① 更彻底。
- D-a（估计对象改变）实测坐实：**wf8 L3 mean = −1.557e-6（微负）**——S3 下 +1.524%、S1f 下 +458.34 的 L3 盈利引擎，÷bars_held 后 mean 符号被抹成负；wf7 L1×Short mean 由 S3 的 +0.929% 变为 −3.41e-6。长持仓正收益桶被 ÷bars 系统性压向零/负，与结构优劣无关。
- D-b（成本摊薄不对称）可见：费扣 2f·entry 为每笔固定比例成本，÷bars 后短持仓桶每 bar 成本占比更高；wf8 L1×Short mean=−8.21e-5/bar 为全表最负。
- ② 的机制批判（裁定材料 :54-56）从「方向推导」升级为「实测坐实」。

**方案 ③（÷入场名义）**：
- dir 键：wf8 Long PASS（mean=+1.852e-3，LCB=+1.72e-4，√n/CV=1.81）、合并 PASS（LCB=+8.43e-5）、**wf7 REJ**（mean=+4.91e-4，LCB=−1.06e-3）——唯一放行桶跨窗口不稳定，照实标注；Short 两窗口 mean 恒负（wf7 −1.38e-3 / wf8 −2.46e-3），拒绝由真实负 mean 承载。
- level 键：**L3 仍未翻正**——wf8 L3 mean=+1.524% std=5.234% CV=3.43 LCB=−2.71e-3 REJ。照实：**③ 在本静态样本未降 L3 的 CV**（S1f 3.30→S3 3.43，罚项/mean 比 1.131→1.178 略升）——单窗口内价格水平方差小，③ 移出的方差源有限，逐笔除法的非线性使 CV 微升；③ 的跨期可比性收益在单窗口静态核算中**不可见**，只能由机制推导与多窗口语义支持，不得 claim 数据已证 CV 下降。
- 合并（跨窗口、价格水平 23756 vs ~26000-28598 并存）下 ③ 的 dir CV=12.8 仍高于 S1 的 8.01——**本样本未观测到 ③ 的 CV 缩减效应**，此点与裁定材料 §2-③「CV 下降方向确定」的机制推导存在张力，照实上报（可能原因：per-unit 口径已无 qty 源，价格水平源在 8xx bar 窗口内方差小；÷entry_px 改变的是逐笔权重而非纯常数缩放）。

### 3.4 数据证据边界（090 声明=能力）

- 静态核算是**全样本一次性** LCB 求值，非 walk-forward 因果 μ（真实 χ 门喂入是时序累积的，冷启动/自锁行为不在本核算范围——裁定材料 §4-4）。
- 桶键为 1-2 维边际投影；真实 MuClass 15 维键下 n 更小、None 更多（gate-filter F2），三方案的 REJ 计数都是**下界**。
- n 仅 504/518；所有「PASS/REJ」是 θ=0、z=1.645 判定点的确定性读数，不构成任何样本外效果预测。

## 4. 三方案证据总表（教义锚 + 机制推导 + 静态数据证据）

| 维度 | ① 绝对额（现状） | ② %/bar | ③ ÷入场名义 |
|---|---|---|---|
| 教义锚 | alpha2 §12 X_γ=δ(P_τγ−P_t)−C 为每单位价格差（line 2147）；严格alpha §1 净值 ΔR=ΔN·ΔP−ΔC 为货币额（line 790）——两可，但 ledger 实装取 qty=1（l3_delta_r_alpha.rs:266-269）与 §12 字面最近 | **无教义支持**：§12 的 X_γ 是 per-certificate 全程收益，PDF 无持仓速率概念（裁定材料 :58） | §12 每单位价格差 ÷ 入场价的可比化推进（裁定材料 :67）；θ=0 语义与严格alpha §1 净值比例解释对齐（:68） |
| 机制推导 | CV 含价格水平+时长两非结构源（qty 源经 §2.2 更正后仅 bin 口径成立）；罚项淹没 mean（裁定材料 :46） | 移三源最彻底，但 D-a 改估计对象 + D-b 短持仓成本放大（:54-56） | 移价格水平源，估计对象不变，无新增失真；qty 留在 Θ_risk 层（:64-69，经 §2.2 更正） |
| 静态数据（本文 §3） | S1f 复现 F1：level 键 100% REJ 含 L3（+458.34→LCB −59.92）；dir 仅 Long PASS 且 wf7 不稳定；S1q 演示 qty 注入即全拒 | **全键全窗口 100% REJ**；D-a 坐实：L3 mean 被抹成 −1.56e-6；退化为全拒门，劣于 ① | dir 键与 ① 同判定（wf8/合并 Long PASS、Short REJ 由负 mean 承载）；level 键 L3 未翻正；**本样本未观测到 CV 缩减**（照实，:§3.3-③） |
| 退化形态 | 方向门（gate-filter F1） | **全拒门**（比 ① 更退化） | 方向门（同 ①），但拒绝归因净化为 mean 符号+结构 CV |
| INCONCLUSIVE 关系（裁定材料 :49/:59/:71） | χ 把 INCONCLUSIVE 性质当拒单依据（231 同构） | 提功效但混 D-b artifact | 提功效路径，无新 artifact；UCB/powered 门控缺失不随量纲解决 |

## 5. 推荐排序依据（材料，非裁定）

**数据证据排序：③ ≳ ① > ②**；**综合（教义+机制+数据）排序：③ > ① > ②**（供编排者拍板，本文不代裁）。

理由链：
1. **② 末位（数据坐实，新证据）**：② 在 wf7/wf8/合并三键下 100% REJ（连 ① 唯一放行的 dir Long 桶也被拒），且 D-a 失真把 L3 盈利引擎的 mean 符号抹成微负——② 不是「缓解退化」而是「退化为全拒门」。裁定材料排 ②>① 的依据是机制上「多移一个时长源」，但实测显示时长源的移除以估计对象被毁为代价；数据证据把 ② 排到 ① 之后。
2. **③>①（依据在教义与稳健性，不在本样本判定翻转——照实）**：静态数据上 ③ 与 ①（S1f）在 θ=0 的桶判定**完全相同**（dir 1/2、level 全 REJ），③ 未在本样本展示判定改善；③ 的优势是（a）θ=0 语义跨期可比、与严格alpha §1 净值口径对齐；（b）对 qty 喂入免疫——若未来 χ 喂入切到 pi_bsp_timing 式 qty 口径，① 退化为全拒（S1q 实测），③ 的逐笔 ÷qty·entry_px 天然吸收该方差源；（c）sizing 留在 Θ_risk 的层级净化（严格alpha §4 μ(z,a) 的 a 不含仓位）。即 ③>① 是**教义对齐 + 机制稳健性**排序，不是数据判定差排序。
3. **① 的保留理由**只剩工程惯性（bit-exact、与 metrics.rs 消费侧同口径），且 gate-filter F1（拒 L3 盈利引擎）在费扣现状口径下原样复现——维持 ① = 维持「χ 实为方向门」的声明-能力不符（090 禁）。
4. 与裁定材料 §3 排序（③>②>①）的差异仅在 ①② 次序：裁定材料按机制推导排，本文补上数据证据后 ② 落到末位；③ 首位两者一致。

**不代裁声明**：本证据表不替代裁定；③ 的「CV 下降」机制收益在本静态样本未被观测到（§3.3-③），若编排者裁定 ③，其预期收益应落在「θ=0 语义可比 + qty 免疫 + 层级净化」上，不应落在「本样本已证过滤率下降」上；量纲裁定生效后的首批门生效观测仍属【待实测】（chi=Some 的 walk-forward 运行，裁定材料 §4-5）。

## 附：锚点清单与复算方法

- **教义锚**：`买卖点alpha2.pdf` §12（p.22/38，X_γ/τ_γ/μ 定义，pdftotext line 2147/2173）、§13 严格解决方案（p.22-23/38，χ_t 定义，line 2239）、§14 正收益定理（p.23/38，line 2274/2281/2319）；`严格alpha.pdf` §1（p.8-9/27，净值递推/alpha/ΔR 定义，line 769/782/790/804）、§4（μ(z,a)，经裁定材料 :40 转引）。
- **代码锚**：`mu_estimator.rs:364-372`（marginal_return 通用 qty）、`:468-472`（mu_lcb）；`l3_delta_r_alpha.rs:266-269`（ledger 喂入 qty=1.0——**现状口径**）；`rust/src/bin/pi_bsp_timing.rs:370`/`:463`（voice.qty 喂入）；`metrics.rs:579-590`（trade_abs_pnl 双边费公式）；`runner.rs:2940-2943`（dump pnl_raw_unlevered=δ(exit−entry) 费前）、`:327-328`（fee_rate=(comm+slip+tax)/1e4）；`reference-theta-v0.md` §Θ_exec:51（1bp+2bp）。
- **文档锚**：`chanlun/review-results/gap1-chi-dimension-ruling-material-20260719.md` §0-§3；`chanlun/review-results/l3-econ-gate-filter-rate-20260719.md` §3.1 F1/F2（交叉验证基准）。
- **复算**：Python3 stdlib（json/math/collections），逐行解析 `/tmp/m8_win/wf{7,8}/trades.jsonl`，defaultdict 分桶 + 全量样本 std，LCB=mean−1.645·std/√n 逐桶求值；输入只读，无写入；脚本 `/tmp/chi_dim_static.py`（临时工位文件）。
- **未做（v3/090）**：无概率/统计推断作决策基础；无胜率/夏普/p 值；无参数寻优；无样本外外推；无策略回测结论；不代裁。
