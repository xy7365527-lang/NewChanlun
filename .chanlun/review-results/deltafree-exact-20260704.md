# δ-free 聚合基逐笔精确重算（Task #186，final-alpha §3.1 下游待办落地）

- **工位**：swarm/ws-dfexact | **task #186** | **加固对象**：`final-alpha-20260704.md` §3.1（第144-152行）δ-free 聚合基三态
- **认识论**：L2（BTC 单标的全历史 walk-forward OOS 逐笔精确重算）。结果照实（161），加固不翻终局。
- **口径链**：`DELTAFREE_DUMP=/tmp/finalpha/deltafree_pertrade.tsv` 门控 wverify_full 落盘 2256 笔（= WV_FULL residuals，逐字节一致）→ 离线 `deltafree_exact_recompute` 精确重算。

---

## 0. 结论（一行）

**§3.1 唯一 LCB>0 桶 `L0 bsp3 σ+1` 在精确口径（真 n_eff + δ-free perm_p）下维持**——mean+162.50 lcb+89.90 n_eff=248.0 perm_p=0.000 → Validated。**但它仍是 beta 漂移伪结构**（报告桶两 δ 方向同号 +162，657/oddeven 签名），非方向性 alpha。**终局 INCONCLUSIVE 不翻转**——加固而非改判。

---

## 1. §3.1 正态近似 vs 精确口径差分（唯一 LCB>0 桶）

| 量 | §3.1 正态近似 | 精确重算 | 差 |
|---|---|---|---|
| N | 248 | 248 | — |
| mean(Y) | +162.50 | +162.4967 | ~0 |
| std | 695.0（由 LCB 反推） | 694.95（Welford 直算） | ~0 |
| LCB | +89.90 | +89.9035 | ~0 |
| **n_eff** | **无**（§3.1 明文不可算） | **248.00**（Geyer IPS，无自相关缩水） | 精确口径补齐 |
| **δ-free perm_p** | **无**（§3.1 明文不可算） | **0.000**（B22 同批置换，只读出侧池化） | 精确口径补齐 |
| powered? | 正态近似未算功效门 | n_eff=248 ≥ (1.645·4.277)²≈49.5 ✓ powered | — |
| state | LCB>0（正态近似上界） | **Validated**（powered ∧ μ̂>0 ∧ perm_p<0.05 ∧ LCB>0） | 三态判据全项满足 |

**§3.1 曾声明的口径限制（第150行）已消解**：正态近似"不能精确重建 n_eff 与 δ-free perm_p"——本重算两项均补齐。该桶 n_eff=N=248（Geyer 配对 IPS 首配对即非正，无自相关缩水 ⟹ 有效样本不打折），δ-free perm_p=0.000（层内 δ 置换 200 次无一使池化均值 ≥ 观测）。§3.1 的"至多与此一致，不会更多桶 LCB>0"预判被证实：精确口径下仍仅此一桶 LCB>0。

---

## 2. beta 漂移判据（精确口径下维持）

§3.1 的 beta 判据核心是**报告桶两 δ 方向同号不抵消**。精确重算的报告桶行（`/tmp/wv_full_rows.md`）：

| 报告桶 | n | n_eff | mean(Y) | LCB | perm_p | state |
|---|---|---|---|---|---|---|
| L0 bsp3 **δ+1** σ+1 | 156 | 156.00 | **+162.25** | +81.18 | 0.000 | Validated |
| L0 bsp3 **δ−1** σ+1 | 92 | 58.06 | **+162.91** | +23.01 | 0.000 | Inconclusive（仅因 n_eff=58 欠功效） |

- **两 δ 方向同为 +162**（+162.25 / +162.91）⟹ δ-free 池化不抵消 ⟹ δ-free mean 仍 +162.50。
- **决定性 beta 判据**（final-alpha §3.1 铁律）：若是方向性 alpha，买/卖池化必抵消（异号）；同号不抵消 = 该 (level,bsp,σ^H) 结构格的**纯 beta 暴露**（level×持有窗 beta 相位，与交易方向无关），非可交易买卖边际。精确 δ-free perm_p=0.000 只确认"δ 池化残差极端"，不改变"同号 ⟹ beta"的判读——perm 检验的是 δ 标签解释力，同号意味着解释力来自共同 beta 而非方向切换。

---

## 3. 全 22 桶精确三态（verdict=Pass V=1/F=0/I=21）

原始表见 `/tmp/deltafree_exact.md`（12 列全量）。摘要：

- **唯一 Validated**：`L0 bsp3 σ+1`（上述 beta 桶）。
- **其余 21 桶全 Inconclusive**：无一 powered-Falsified。精确 n_eff 揭示多桶欠功效（如 L0 bsp1 σ+0 n=13 n_eff=6.31、L1 bsp3 σ+0 n=15 n_eff=8.74——自相关缩水），CV 普遍极高（L1 bsp2 σ+1 CV=310）⟹ 功效门未过 ⟹ Inconclusive 而非判纯 beta（667/231：LCB≤0 ⊬ μ≤0）。
- **与 §3.1 正态近似的桶数差异照实**：§3.1 报告"1 个 UCB≤0（L0 bsp1 σ+1 纯 beta）+ 其余 21 欠功效"。精确口径下 L0 bsp1 σ+1（N=8 n_eff=3.61 UCB−5.35）因 **n_eff=3.61 远低于功效门** 判 **Inconclusive**（不是 Falsified）——精确 n_eff（3.61）比正态近似隐含的 N=8 更严格地暴露欠功效，故不落 161 纯 beta。这是精确口径**更保守**的方向（欠功效不误判为纯 beta），与 667 一致。

---

## 4. 与 final-alpha 五路证据的关系

本重算是 final-alpha §3.1 δ-free 主判据分量的**精确化加固**，五路证据（报告桶 beta 签名 + co-primary β 不显著 + full-z 零 Validated + L3 池化翻 Falsified + δ-free 聚合基重算）中的第五路从"正态近似上界"升级为"精确 n_eff+perm_p"。**五路一致 INCONCLUSIVE 不变**：δ-free 主判据下唯一 LCB>0 桶是纯 beta（同号不抵消），acceptance 仍 INCONCLUSIVE。

---

## 5. 结果包六要素

1. **结论**：δ-free 精确重算维持 §3.1——唯一 LCB>0 桶 `L0 bsp3 σ+1`（精确 n_eff=248 perm_p=0.000 Validated）是 beta 漂移伪结构（两 δ 同号 +162），非方向性 alpha。终局 INCONCLUSIVE 不翻。
2. **定义依据**：可交易判据=LCB_OOS>0 ∧ powered ∧ perm_p<0.05（663/§12）；δ-free 主判据聚合基 (level,bsp_class,parent_dir)（prereg §3.2）；beta 漂移=两 δ 同号残差（657/oddeven）；n_eff=Geyer 配对 IPS（665）；powered=n_eff≥(z_α·CV)²（667）。
3. **边界条件（翻转）**：(a) 若某 δ-free 基 perm_p<0.05 ∧ LCB>0 ∧ powered ∧ **两 δ 异号**（真方向切换）→ 方向性 alpha，翻 Pass；本轮无此桶。(b) 若样本升数量级（跨标的池化 + 657 归一化新 prereg）→ 提功效重估。
4. **下游推论**：final-alpha §3.1 第150行"逐笔留存精确重算列为下游待办"已闭合——per-trade dump 门控落盘（不进判定路径 bit-exact）+ 离线精确 n_eff/perm_p 落地。B30②④仍后置（无 Validated 真 alpha 候选）。
5. **谱系引用**：657 + memory `oddeven_mu_identity`（δ 同号=beta）；663/665/667（判据/n_eff/功效门）；231/formalization-validity-domain（正态近似=更宽松上界，精确口径消解口径限制）；090/161（bit-exact + 否定性照实）；135（冻结先于跑数）。
6. **影响声明**：新增 `perm_test::stratified_delta_perm_p_deltafree`（δ-free 池化 perm_p，B22 三处置换同批口径不动、只读出侧）+ `wverify_run::dump_deltafree_pertrade`（env 门控诊断落盘）+ `deltafree_exact_recompute`（离线精确重算测试）+ `load_deltafree_dump`（f64 bit round-trip 还原）+ 自检 `deltafree_dump_roundtrip_and_pooling`。**不改任何判定路径**（dump 门控关时 bit-exact，全 1490 lib 测试绿）。产物落 `/tmp/finalpha/deltafree_pertrade.tsv`（dump）+ `/tmp/deltafree_exact.md`（22 桶表）+ 本文件。
