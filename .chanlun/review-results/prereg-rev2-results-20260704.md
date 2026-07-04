# prereg-rev2 结果包：ForceState 分层键 + μ_R co-primary 双门（a1+a6 联合，L2 BTC OOS）

- **工位**：swarm/ws-prereg2 | **Task #7** | goal g-20260704T2030Z-deepresearch-impl
- **冻结基**：`26ebe90b29`（`prereg-rev2-20260704.md` 独立冻结 commit，#135 跑数先于本 commit ⟹ fail——本跑数在其后 HEAD 上执行 ✓）
- **认识论**：**L2**（BTC 单标的 461万 bar walk-forward OOS，5 窗 test_start≥OOS_START，residuals=2256）。代码留工作区未 commit（除 prereg 冻结）。

---

## 0. 结论（一行）

A1（ForceState 进 δ-free 主裁决聚合基）**ForceState⊥δ 检验 PASS**，δ-free 主裁决从 22 桶升 **25 桶**（force_state 细分一类买卖点桶），在线↔离线 **逐字节一致**。A6（μ_R=E[Y/d] co-primary）产出双 estimand 表：raw μ 唯一 Validated 桶（L0 bsp3 σ+1，已知 beta 漂移伪结构）**在 μ_R 除 d 后退为 Inconclusive**（perm_p 0.000→0.085、LCB +89.9→−0.16）。**无任一桶双门同时 VALIDATED ⟹ fail 条件 5 未触发，终局 INCONCLUSIVE 不翻**（161 否定性照实）。

---

## 1. A1 结果：ForceState 进主裁决聚合基

### 1.1 ForceState⊥δ 前置检验（prereg §1.2）— **PASS**

```
ForceState⊥δ 检验结论：PASS(每态 δ 两向非空，有交换自由度 ⟹ 合法进主裁决聚合基)
```

Some(force_state) 记录仅出现在一类买卖点桶（type1=38 笔，force_some=38，A6 #159 透传真值）——Dom-/Inc 各态 δ 两向均非空。**非 i_class×δ 共线**（force_state 由 A/C 段绝对量算，mirror-invariant，置换 δ 恒定）。检验落 `/tmp/wv_full_forcestate_ortho.md`。

### 1.2 δ-free 主裁决（含 force_state 第 8 维）— 25 桶

`records=2256 δ-free基桶=25 verdict=Pass V=1/F=0/I=24`。唯一 Validated：`L0 bsp3 σ+1 force=None`（n_eff=248, mean +162.50, LCB +89.90, perm_p 0.000）——与终局/dfonline-a2 的唯一 Validated **同一桶**（force_state=None，即多数非一类交易），是已知 **beta 漂移伪结构**（δ 同号不抵消，deltafree-exact §2），A1 未改变其性质。

force_state 细分只对一类桶（L0 bsp1 分裂为 Dom-/Inc/None）生效，全部 Inconclusive（n=2..11 结构性欠功效）。**桶数 22→25 与旧表不可逐桶比较**（#135）。

### 1.3 在线↔离线 bit-exact（问题F 收口口径保持）

`deltafree_exact_recompute`（离线 dump 复现器，含 A1 force_state + A6 d 两新列 round-trip）与在线主裁决表 **逐字节一致**（`diff` 干净，25 桶同 verdict V=1/F=0/I=24）。dump round-trip 自检 `deltafree_dump_roundtrip_and_pooling` 绿。

---

## 2. A6 结果：μ_R=E[Y/d] co-primary 双门

### 2.1 样本与口径

`raw_n=2256 → μ_R_n=2184`（剔除：d 不可得 NAN=72，d<D_MIN=0）。72 笔 d 不可得 = structural_stop 返 None（该方向无止损可定，非交易点）——诚实剔除不兜底（231）。D_MIN=tick_size 无一笔触发（无近零止损距离）。μ_R 与 raw μ **不可比**（除 d + 样本集不同，#135）。

### 2.2 μ_R δ-free 主裁决 — 25 桶

`δ-free基桶=25 verdict=Inconclusive V=0/F=1/I=24`。
- **无 Validated 桶**：raw μ 的唯一 Validated（L0 bsp3 σ+1）在 μ_R 下 `mean +0.296 LCB −0.162 perm_p 0.085` = Inconclusive。风险归一化后 beta 漂移伪结构的「超额」被止损距离摊平。
- **唯一 Falsified**：`L0 bsp2 σ-1 None`（n=346, powered, UCB −0.13≤0）——μ_R 视角有功效地判该结构格纯 beta（161 照实）。
- μ_R 侧唯一 LCB>0 桶 `L2 bsp2 σ-1 None`（mean +0.78 LCB +0.02 n_eff=23 perm_p 0.800）——perm_p 不显著 ⟹ Inconclusive，非 Validated。

### 2.3 双门 acceptance（Holm 全族校正，prereg §3）

假设族 = {raw μ, μ_R} × 25 桶 × {β 路径}。**无任一 estimand 任一桶 VALIDATED**（raw μ 的 Validated 是 force_state=None 的已知 beta 伪结构，μ_R 无 Validated）。Holm 校正前已无候选 ⟹ 校正后必无。**acceptance = INCONCLUSIVE**（存在 INCONCLUSIVE、无 VALIDATED、β 路径无新显著）——不 PASS 不 161，与终局一致。

---

## 3. 结果包六要素

1. **结论**：见 §0。A1 PASS 且不翻结构，A6 双门无 VALIDATED，终局 INCONCLUSIVE 保持。
2. **定义依据**：A1 键 = `(level,bsp_class,parent_dir,force_state)`（prereg-rev2 §1，dfonline-a2 §4 两注入点 `deltafree_verdict` series + `stratified_delta_perm_p_deltafree` base_map 同步）；ForceState⊥δ = force_state 绝对量算 mirror-invariant（perm_test.rs:211）+ 实证每态 δ 两向非空（§1.1）。A6 μ_R = codex-ruling-696 选项 B 的残差框架落点 E[Y/d]（非字面 E[X/d]——X δ-baked 进 perm 虚假显著，perm_test.rs:506 铁律）；d=|entry_px−structural_stop|（risk.rs:74-105，ex-ante）；D_MIN=tick_size 防护（prereg §2.3）。三态判据 = final-prereg §3.2（μ̂>0∧powered∧LCB>0∧perm_p<0.05）。
3. **边界条件（翻转）**：(a) 若某标的/窗 ForceState⊥δ FAIL（任一态 δ 单向）⟹ A1 退出主裁决基（本轮 BTC PASS）；(b) 含 force_state 主桶 perm_p→1.000 ⟹ 共线证伪（本轮无——唯一显著桶 force=None perm_p 0.000）；(c) 若双门同桶 VALIDATED 出现 ⟹ 翻转终局上浮（本轮无）；(d) X 改 position-sized ⟹ μ_R 降诊断（§2.5）。
4. **下游推论**：A1/A6 确认**不翻**终局 INCONCLUSIVE——force_state 细分未暴露隐藏 alpha，风险归一化（μ_R）反而抹掉唯一 raw μ Validated（进一步坐实其为 beta 伪结构，非可交易 alpha）。M1 里程碑维持「无 confirmed 正 alpha」。B30②④ 生产化/收缩无触发（无 VALIDATED 候选）。
5. **谱系引用**：prereg-rev2-20260704（本轮冻结基 26ebe90b29）；dfonline-a2（问题F 收口 + A1 接口）；codex-ruling-696（A6 选项 B）；final-prereg-20260704（终局，INCONCLUSIVE 不翻确认）；memory `iclass_delta_collinearity_perm_degeneracy`（方向性维共线自毁——ForceState⊥δ 检验的防护对象）+ `oddeven_mu_identity`（δ 同号=beta，唯一 Validated 桶的性质）；#82（残差口径 δ-free 置换）；657/663/665/667；231/formalization-validity-domain（L2 标注 + 诚实剔除）；090/161（bit-exact + 否定性照实）；135（冻结先于跑数 + 不可比）。
6. **影响声明**：代码改动（工作区，未 commit——prereg 冻结 commit 除外）：`mu_estimator.rs`（ResidualTrade+d）、`runner.rs`（LedgerOpen/TypedTrade+entry_stop_dist、candidate_stop_dist 助手、schema→3、5 push 点透传）、`l3_delta_r_alpha.rs`（d 填充）、`perm_test.rs`（deltafree base_map 键+DeltaFreeKey 类型）、`wverify_run.rs`（deltafree_verdict series 键+force_state 列、μ_R co-primary 块、ForceState⊥δ 检验、dump/load+force_state+d 两列、force_state_code/decode/label 助手）。**未改**：records 生产判定量、perm 分层键（恒 (ℓ,h,time,σ^H)）、decontam、raw μ 现口径（bit-exact——离线复现验证）。测试 `cargo test --release --lib theta_v0` = **914 passed / 0 failed**（不劣化）。产物：`/tmp/wv_full_deltafree.md`（raw μ 25 桶 + ortho 检验）、`/tmp/wv_full_mu_r.md`（μ_R 25 桶）、`/tmp/wv_full_forcestate_ortho.md`、`/tmp/deltafree_exact.md`（离线复现，bit-exact）。
