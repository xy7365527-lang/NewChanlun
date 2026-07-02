# 全 PDF × 全结果包实装缺口盘点（goal g-abfb9eaa 先行项 a0 / task #45）

- **来源**：编排者澄清「问题是我们并没有完全实装」；收口=全实装后回测 alpha
- **范围**：`docs/pdfs/` 5 份等价关系 + 断点检测/最小验证框架 + `docs/chanlun/external-consults/level-sigma-complete-classification`（alpha 统计框架）× `.chanlun/review-results/` 结果包
- **认识论**：本盘点=**L1**（源码逐行三态判定 + 结果包承诺反查），零 alpha 证据
- **环境**：只读，零 git
- **基线**：以 `pdf-conformance-audit-20260702.md`（概念一致/偏离层）为 PDF 主张提取基础，本文件补**代码实装三态**（未实装 / 部分实装+缺什么 / 已实装+锚点）

---

## 0. 阻塞 alpha 重测口径子清单（置顶 · a3 必做项）

「阻塞」判定依 652/275 局部依赖：a3 全量 alpha 重测（task #13）的**输入**是否直接消费该缺口的输出。

| 缺口 | 为何阻塞 | 硬/软 |
|------|---------|-------|
| **G-A1 桶键省略 σ^H** | 重测按 `z=(ℓ,bsp_class,δ)` 认证；PDF 要 `z=(ℓ,δ,σ^H)=(ℓ,q)`。不加 σ^H ⟹ 重测结论**不对 PDF 的 (ℓ,q) alpha 发言**。编排者「全实装」标准下=硬阻塞；诚实 Inconclusive 收口标准下=软阻塞（可用 231 有效域标注绕过） | 硬（全实装口径）/ 软（Inconclusive 口径） |
| **G-A2 分层缺 σ^H+time block+持有桶 h** | 与 G-A1 同根；且缺 time block ⟹ 置换不保时间自相关 | 硬（同 G-A1） |
| **G-A4 LCB_OOS walk-forward 未接入判据** | 当前 LCB 是**单 OOS 窗 in-sample 正态近似 SE**（wverify_run.rs:47-49），非 train/test 分离的样本外下界。prereg 窗常量已定义但未被消费 ⟹ 「LCB_OOS>0」严格声明不成立 | 硬（若要严格 OOS 充要）/ 软（若接受 in-sample LCB + Holdout 隔离） |
| **G-A6 prereg §2 文本全局 omnibus vs 实跑逐桶（D3）** | 纯文档订正，实跑已逐桶（对）。收口前订正 prereg §2 文本口径即可，行动类 | 软（文档，非代码） |

**其余缺口（G-A3/A5、G-B2/B3/B4、G-C1/C3/C4）均非阻塞**——后续泳道，理由见各条「阻塞性」列。

---

## 1. 总表（缺口 ID · PDF 锚点 · 三态 · 代码锚点 · 阻塞性 · 路径）

### 簇 A：alpha 统计管线（level-sigma PDF p8-11 三判据）

| ID | PDF 锚点 | 三态 | 代码锚点 | 缺什么 | 阻塞 | 路径 |
|----|---------|------|---------|--------|------|------|
| G-A1 | p3-4/p8/p11 `z=(ℓ,δ,σ^H)` | **部分** | `perm_test.rs:19` BucketKey=(ℓ,bsp_class,δ)；`wverify_run.rs:22-26,28` 投影 `c.bsp_class()`；`mu_estimator.rs:69` MuClass 含 parent_dir 但投影丢弃 | σ^H（父声部方向）维；bsp_class 是结构先验，与 σ^H 正交，非等价替代 | **阻塞** | perm_test BucketKey 加第4维 σ_p；wverify 投影改用 `c.parent_dir`（MuClass 已携，仅投影层改） |
| G-A2 | p9 分层 S=(ℓ,σ^H,h,time block) | **部分** | `perm_test.rs:69,86` Stratum=(ℓ,bsp_class) | σ^H + time block + 持有桶 h | **阻塞** | Stratum 扩维；time block 需成交时间分桶 |
| G-A3 | p9 分层内置换保自相关 | **部分** | `perm_test.rs:104-107` 层内 fisher_yates 打乱 δ（iid） | 无 time block ⟹ iid 置换不保时间自相关 | 非阻塞（n_eff 已在 powered 门保守校正自相关） | 随 G-A2 time block 落地 |
| G-A4 | p10 LCB_OOS + walk-forward split | **部分** | `wverify_run.rs:17` OOS 窗+Holdout 隔离 ✓；`:47-49` LCB=mean±z·se（**in-sample 正态近似，se 用 raw n**）；`prereg_windows.rs:35-95` WfWindow train/test/holdout 常量已定义**但未被 wverify 消费** | walk-forward 逐窗 train 估 μ / test 算 LCB 的样本外下界 | **阻塞**（严格 OOS 充要口径） | wverify 按 WfWindow 逐窗：train 段估 μ，test 段算 OOS LCB |
| G-A5 | p9 time-block 精神 | **部分** | `metrics.rs:609` block_bootstrap_total（TRADE_BLOCK_LEN=5, n=1000）✓ 但用于**策略级 CAGR/Sharpe 显著性**（`metrics.rs:305-376`），非 per-bucket μ | per-bucket μ 的 LCB 用正态近似（decontam/wverify），未用 block bootstrap | 非阻塞（PDF 未强制 per-bucket bootstrap；可选更严） | 可选：per-bucket LCB 改 block bootstrap |
| G-A6 | p9 禁全局 omnibus（D3） | **文档缺口** | `wverify_run.rs:46-60` 实跑逐桶 ✓；prereg §2 文本写全局 Σ_bucket\|μ̂\| | prereg §2 文本口径 | 软（订正文档） | 更新 acc-alpha-estimand-prereg §2 到逐桶口径 |
| G-A7 | p9 n_eff 自相关校正 | **已实装** | `decontam.rs:66` effective_n（Geyer 配对 IPS，τ=1+2Σρk，clamp n_eff≤n）；`wverify_run.rs:53` 接入 | — | — | — |
| G-A8 | p10 三态充要+powered | **已实装** | `decontam.rs:115` powered（n_eff≥(z·CV)²）；`:123` classify_bucket；`:146` global_verdict | — | — | — |
| G-A9 | p9-10 per-(ℓ,q) 逐桶置换（非全局） | **已实装（结构）** | `perm_test.rs:81-134` 层内置换（非全局 T）✓ | q=bsp_class 非 σ^H（见 G-A1）；结构对、维度错 | 随 G-A1 | — |

### 簇 B：等价关系与不变量 5 份 PDF（637/673/换序/双模拟）

| ID | PDF 锚点 | 三态 | 代码锚点 | 缺什么 | 阻塞 | 路径 |
|----|---------|------|---------|--------|------|------|
| G-B1 | PDF1 EQ-Closure 单标准冻结 | **已实装** | `center.rs:86-101` 单塔 B（全三段 min3/max3）；A 仅派生见证 `third_spans_core`（637-ab-differential §0-1，task #11） | — | — | — |
| G-B2 | PDF4 行为商 A/B 全重组同判检验 | **部分** | `637-ab-differential-20260702.md` §1-3：L0 结构差分 machine-checked（存在性/外缘/level/type1-2 恒等，差分仅 type3+点位窄带）+ 定性裁 B | L2 数值频率（真实标的第三段收窄占比 + type3 信号 Δ）未跑批（§4 缺 harness） | 非阻塞（裁 B 已定，频率只缩放不改定性） | §4 最小 harness：中枢扫描点累加 narrow 占比 + type3 重放 Δ |
| G-B3 | PDF2 自然变换换序律 I∘f=f'∘I | **未实装** | grep commut/naturality/natural_trans 零命中 | 独立的跨尺度换序律形式检验 | 非阻塞（换序威胁已被 637-ab 双模拟侧证具体化；概念判据无独立代码检验） | 语法记录：换序律由 A/B 差分具体化，无需独立形式检验（除非要通用不变性守卫） |
| G-B4 | PDF5 双模拟不变性 Obs-bisimulation | **部分（概念侧应用）** | grep bisimul 零命中；但 `637-ab-differential §3` 用双模拟判据（Obs 可区分⟹裁 B）做了具体应用 | 通用「任意画法差异⟹策略输出一致」不变性守卫 | 非阻塞 | 语法记录：作为通用守卫未实装；单例应用已完成 |
| G-B5 | PDF4 673 按 bsp 类型分叉 | **已实装** | `cand_predicate.rs:107` DivCand^δ Type1 谓词；接口三分拆 task #12；Type2/3 走结构完成非背驰，per-delta 判型分流 | — | — | — |

### 簇 C：断点检测 + 最小验证框架 PDF + 区间套 + 力度

| ID | PDF 锚点 | 三态 | 代码锚点 | 缺什么 | 阻塞 | 路径 |
|----|---------|------|---------|--------|------|------|
| G-C1 | pp.1-2 CPD 基准（ruptures/PELT/BOCPD）+ 对比指标（Jaccard/变点误差/中心位移 L2） | **未实装** | 全 theta_v0 零命中 | CPD 工具与对比指标 | 非阻塞 · 语法记录 | **不实装是正确的**——PDF 明确定位 CPD 为基准工具（有效域<New-Chan 定义域），原文权威>工具基准 |
| G-C2 | 区间套下沉 + 小转大终止 | **已实装** | `econ_positive.rs::descend_type1_anchor_depth` 真递归下沉；descend=None⟹小转大门拒（task #24/#41）；`l2-depth-distribution-20260702.md`：小转大 91.85%、可锚 8.15%、depth d=1 主导 95.8% | — | 非阻塞 | 95% 退化是 **L2 数据事实非缺口**；depth≥2 真递归稀有（5/120）=观测事实 |
| G-C3 | 力度 Step3 Lex 词典序 | **部分** | `divergence.rs:423` weak_theta Lex（DIF 主▷面积次）+ 多 proxy（dif_peak/振幅/速度 `:335-388`）已实装为纯函数 | 端到端未接通生产：buy1 仍用 `segments_diverge`（MACD 面积单判据）；selector Weak_Θ 门需 Candidate 携 A/C 段力度但 assemble_gamma 作用域只有 BspPoint（`:290-295` 诚实 gap） | 非阻塞（原语可测；"哪个 mode 有 alpha"是 L2/L3 未测） | 段力度透传进 gamma 组装（独立大工位，改 Candidate/gamma 管线） |
| G-C4 | 力度=走势力度非仅 MACD（第17课） | **部分 · 诚实 gap** | `divergence.rs:253-266` 诚实标注：MACD 面积是**唯一**背驰判据；振幅/速度原语已实装未接通 buy1 | 独立走势力度判据（振幅/速度/量能）接通生产 | 非阻塞（编排者裁定：识别标注，不顺手实装） | ForceMeasure 增非-MACD strength 实例，MACD 降 proxy 之一 |
| G-C5 | pp.3-8 Ledger 禁语义回补 | **已实装** | task #34 移除 hwm_gain 棘轮对 free/stage_progression 承重；codex R3 裁决 C' 落地；acc-GAP3 恢复 FALSIFIED | — | — | — |

---

## 2. 分簇详情

### 簇 A · alpha 统计管线（阻塞根所在）

**核心事实链**：MuClass 六维（`mu_estimator.rs:69` level/delta/i_class/**parent_dir**/short_swing/position）**携带** parent_dir σ_p（≈上级/父声部方向 σ^H）。但两处投影把它丢弃：
1. `wverify_run.rs:22-26` 逐笔投影 `(c.level, c.bsp_class(), c.delta, x)`——`bsp_class()` 把 i_class 折叠为主类号（1/2/3），parent_dir/short_swing/position 全丢。
2. `perm_test.rs:19` BucketKey=(u32,u8,i8) 三维，`stratified_delta_perm_p` 分层键 (ℓ,bsp_class)。

因此 PDF 要求的 `σ^H` 维在 alpha 判定管线**端到端缺失**——不是没这个量（MuClass 有），是投影到统计层时被压掉。**修复代价小**（投影层加一维），但会**稀释每桶样本**（更少功效，663 pending 独立方向）。

**判据③ 现状**（wverify_run.rs）：μ 在**整个 OOS 窗**（2023-01…2025-06）估计，LCB=mean±1.645·(std/√n)——**in-sample 正态近似 SE，用 raw n**（非 n_eff）。Holdout（2025-07+）已隔离（:17），walk-forward 窗常量已在 `prereg_windows.rs` 冻结（anchored+rolling 12 窗+holdout），**但 wverify 未消费它们**。所以「LCB_OOS」当前是「单窗 in-sample LCB + Holdout 隔离」，非「walk-forward train/test 分离的样本外下界」。

**已实装且正确的部分**（勿重做）：n_eff 自相关校正（Geyer 配对 IPS，decontam.rs:66，对 VALIDATED 保守）、三态判据 + powered 门（decontam.rs:115-154）、per-桶（非全局）置换（perm_test.rs 层内）、N_PERM=200/种子冻结、block bootstrap（metrics.rs:609，但服务策略级 Sharpe，非 per-bucket μ）。

### 簇 B · 等价关系 5 PDF

637（G-B1/B2）：**生产信号路径只有一座塔 B**（637-ab-differential §0 全库扫描坐实，A 已不在任何中枢构造路径，仅 `third_spans_core` 派生谓词见证）。A/B 差分是**反事实**且被 machine-checked 局限在 type3 阈值/止损 + 点位三态两条窄带；存在性/外缘/level/type1-2 逐位恒等。PDF4/5 判据回答：A≠B 在 Obs 可区分 ⟹ 不可折叠双塔 ⟹ 裁 B（一级权威第18课 + PDF 双重支持）。**唯一残余** = L2 频率未跑批（G-B2，非阻塞，只缩放差分面）。

换序律/双模拟（G-B3/B4）：无独立形式检验代码，但威胁面已被 637-ab 的双模拟具体应用覆盖。作为**通用不变性守卫**未实装——语法记录，非缺陷（当前无消费者需要通用守卫）。

673（G-B5）：DivCand^δ Type1 四条件谓词（方向/Comparable/Extreme/Weak）已实装（cand_predicate.rs），接口三分拆 task #12 完成。Type2/3 不走背驰谓词，per-delta 判型分流。

### 簇 C · 断点检测 + 区间套 + 力度

CPD（G-C1）：零实装=**正确**。PDF 自身把 ruptures/PELT/BOCPD 定位为「基准工具」，缠论断点定义（分型/笔/线段/中枢）是内生的，工具基准有效域 < New-Chan 定义域。实装 CPD 反而是引入外部非权威判据。

区间套/小转大（G-C2）：真递归下沉 + 小转大门拒**已实装**（task #24/#41）。`l2-depth-distribution` L2 实测：level1-4 Type2/3 信号 91.85% 无次级别一类背驰锚（小转大剔除），可锚域仅 8.15%，其中 depth d=1 占 95.8%，max depth=3。**95% 退化是市场数据事实，非实装缺口**——绝大多数 Type2/3 是「小级别转大级别」式转折，本就无本级背驰段可套（第43课）。跨级真递归稀有但存在（5/120）。

力度（G-C3/C4）：weak_theta 的 4 mode（MacdArea/Dif/PriceAmplitude/**Lex**）+ 多 proxy 原语（DIF 峰值/价格振幅/速度）**已实装为可测纯函数**（divergence.rs:298-437）。但**端到端未接通**：buy1 判据冻结为 MACD 面积（segments_diverge），selector Weak_Θ 力度门要接通需 Candidate 携段力度 feature，而 assemble_gamma 作用域只有 BspPoint（算不了段振幅/速度）——divergence.rs:290-295 诚实标注 gap。更根本（G-C4）：MACD 面积是当前**唯一**背驰判据，与第17课「MACD 只是辅助，走势力度才是判据」相悖；振幅/速度原语在手但未接通。编排者已裁定「识别标注，不顺手实装」。

---

## 3. 结果包侧承诺反查（待实装/后续/deferred 兑现核对）

| 结果包承诺 | 出处 | 兑现状态 |
|-----------|------|---------|
| 「perm_p 生产者留空，回测入口产逐笔后在跑批层实装」 | decontam.rs:28-29 | **已兑现** perm_test.rs 实装，wverify_run.rs:37 接入 |
| 「W-VERIFY 收口须携 σ^H 有效域缺口标注 + 订正 prereg §2」 | pdf-conformance-audit R2/D3 | **未兑现**（G-A1/G-A6）——task #13 收口待落 |
| 「A/B 差分 L2 频率最小 harness 留给回测工位」 | 637-ab-differential §4 | **未兑现**（G-B2，非阻塞） |
| 「力度原语端到端接通 selector 力度门留待独立工位」 | divergence.rs:290-295 | **未兑现**（G-C3，非阻塞） |
| 「gap3 hwm_gain 棘轮移除承重（codex R3 裁决 C'）」 | codex-r3-ruling / 组3 偏离 | **已兑现** task #34 |
| 「小转大专用通道（区别区间套）输入域=1353 条剔除信号」 | l2-depth-distribution §4.2 | **部分**——小转大门拒已实装（C2），专用后续处理通道未建（非阻塞，后续泳道） |

---

## 4. 结果包六要素

1. **结论**：全 PDF × 结果包实装盘点得 **16 条缺口项**，其中**已实装 6**（G-A7/A8/A9结构、G-B1/B5、G-C2/C5）、**部分实装 8**（G-A1/A2/A3/A4/A5、G-B2/B4、G-C3/C4）、**未实装 2**（G-B3、G-C1，均语法记录非缺陷）、**文档缺口 1**（G-A6）。**阻塞 alpha 重测 3+1 条**：G-A1/A2（σ^H 桶键/分层）、G-A4（LCB_OOS walk-forward 未接入）、G-A6（prereg 文本订正，软）。建议实装顺序：**先 G-A6（文档，最轻）→ G-A1+G-A2（同根，投影层加 σ^H 维）→ G-A4（walk-forward 逐窗 LCB）→ 再跑 a3 全量重测**。

2. **定义依据**：level-sigma PDF p8-11 三判据（z=(ℓ,q)/per-(ℓ,q) 置换/LCB_OOS）；5 份等价关系 PDF（EQ-Closure/换序律/行为商/双模拟/本体链）；断点检测+最小验证框架 pp.1-8；逐条对照 `mu_estimator.rs`/`perm_test.rs`/`decontam.rs`/`wverify_run.rs`/`prereg_windows.rs`/`center.rs`/`cand_predicate.rs`/`divergence.rs` 与结果包锚点。

3. **边界条件（结论翻转）**：(a) 若接受「诚实 Inconclusive + 231 有效域标注」口径，G-A1/A2 从硬阻塞降为软（重测可不加 σ^H，但须声明「σ^H 未纳入，PDF (ℓ,q) alpha 未认证」）；(b) 若接受「单 OOS 窗 in-sample LCB + Holdout 隔离」为充分 OOS，G-A4 降为非阻塞；(c) 若某标的第三段从不收窄核心，G-B2 差分退空、A=B 经验坐实。**编排者「完全实装」标准下 (a)(b) 不成立 ⟹ G-A1/A2/A4 硬阻塞。**

4. **下游推论**：task #13（a3 全量重测）blockedBy G-A1/A2/A4/A6；建议为 G-A1+A2 建单一实装工位（同根，投影层改动）、G-A4 建 walk-forward LCB 工位、G-A6 并入 #13 收口。非阻塞项（G-B2/C3/C4）归后续泳道，不进 a3 门。G-B3/C1 无需实装（语法记录）。

5. **谱系引用**：231（形式化有效域——σ^H estimand 缺口 = 有效域<定义域）；663 pending（σ^H 加维稀释功效独立方向）；606（区间套有效域=Type1）；637/673 pending；576=C（gap3 双账本边界）；本盘点不引入新概念分离，坐实既有偏离的**代码侧实装度**。

6. **影响声明**：本文件为只读盘点矩阵，零代码/零 git。产出=a3 重测的前置 blockedBy 清单（G-A1/A2/A4/A6）+ 非阻塞后续泳道清单 + 「不实装即正确」的语法记录标注（G-B3/C1）。无中断 #1（所有缺口均可分层/可自决/或已有裁决）。
