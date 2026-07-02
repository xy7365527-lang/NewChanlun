# 严格检验管线 s2 实装结果包（task #82，B 路线残差减法+分层补全）

**工位**：swarm/ws-residual | 文件域：mu_estimator.rs / perm_test.rs / l3_delta_r_alpha.rs / wverify_run.rs（无并发）
**源规格**：`docs/formal-chain/alpha分离.pdf`（§1/§4.1/§4.2）+ `alpha检验.pdf`（§6/§7-§8）逐条落注释
**认识论**：实装逻辑 **L0/L1**（残差减法/分层置换/删尾/H2 是确定性变换，验证管线正确性，零 L2 信息增量）；
施加于真实 BTC 全量的 `wverify_full`（#[ignore]）才是 **L2**——本工位不跑 L2，只实装管线。

---

## 1. 结论（四件套逐项状态）

| # | 件套 | 状态 | 落点 |
|---|------|------|------|
| 1 | 残差减法 `Y_i=δ(H−B̂)−C` + 残差上分层置换 | ✅ 实装 | `ResidualTrade`（mu_estimator.rs）+ `build_mu_from_bars`（l3）+ `stratified_delta_perm_p` 重写（perm_test.rs）+ wverify 桶/perm 全改 Y 口径 |
| 2 | 分层键补全 `(ℓ, h桶, time block, σ^H)` + h 管道接入 | ✅ 实装 | `h_bucket()` + `time_block`（build_mu_from_bars entry/exit bar 差）；perm 分层键改为 4 维 |
| 3 | 删尾稳健（删前 3 赢家重估符号） | ✅ 实装 | `drop_top_k_mean`（perm_test.rs）+ wverify 逐桶报告「删尾mean/翻转」列 |
| 4 | H2 方向不对称回归 `β=μ_sell−μ_buy` | ✅ 实装 | `direction_asymmetry_beta_pvalue`（perm_test.rs，block bootstrap 单边 p）+ wverify 逐级别 H2 报告 |

**护航**：既有 perm_test（12/12，含 D1 跨进程复现 `perm_xproc_reproducible`）+ mu_estimator（27/27）全绿；
`cargo build --lib` / `cargo test --lib --no-run` 干净编译（含 #[ignore] l3/wverify）。

## 2. 定义依据（原文 → 实装）

- **B̂_i（alpha分离.pdf §1/§4.1，p1-2/p5）**：`B̂_i=Σ_{s=t_i}^{τ_i−1}ĝ_s`，`ĝ_s` 只用 train/过去信息。
  实装取 §4.1 离散强式 `B̂=ĝ·(exit−entry)`，`ĝ=(P_entry−P_first)/(entry−first)` **因果扩张窗漂移**
  （只用 ≤entry_bar 价，F_entry-可测，无前视）。`Y_i=δ(H−B̂)−C`，`H=P_out−P_in`（δ-free），
  `C=fee·(P_in+P_out)`（与 marginal_return bit-exact 分解 X=δ·H−C）。
- **分层置换（§4.2，p5-6）**：分层键 `s(i)=(ℓ,h bucket,time block,σ_higher)`——**bsp_class 不入分层**
  （§4.2 明列四维，bsp 是被检验的 Z 维非混杂）；层内置换 δ，残差统计 `Y=δ·resid_base−C`（δ-free 基
  重新赋 δ，非重排 δ-baked X_γ）。输出桶键仍 `(ℓ,bsp,δ,σ^H)` 保留逐桶三态口径。
- **删尾（alpha检验.pdf §6，p4）**：删前 k=3 最大赢家后均值 + 符号翻转标志（尾部依赖诊断）。
- **H2（alpha检验.pdf §7-§8/§13，p5-6/p9）**：`X_i=α+βD_i+ε`（D=1 卖/0 买）⟹ `β̂=mean(sell)−mean(buy)`，
  block bootstrap 单边 p（H0:β≤0，事件聚集不用 iid SE）。输入用残差 Y（§1 口径）。

## 3. 边界条件（结论翻转条件）

- **残差检验 vs 选择器口径分离**：本工位只把**验证管线**（wverify/perm）改为残差 Y；`MuEstimator`
  的 μ 表**仍喂 X_γ**（选择器 χ 门语义不变——残差化选择器是独立 gap #9 LCB 门控，不在本 4 件套）。
  若后续裁定选择器也须残差 μ，则 `est.observe` 需改喂 Y（触及 L3/runner/selector 测试，另立工位）。
- **B̂ 扩张窗 ceiling**：`ĝ` 是 slice 内扩张窗均值漂移；若窗内 regime 切换，改滚动窗/因子模型
  （§4.1 允许，`ponytail:` 注释标了升级路径）。entry=slice 首可交易 bar ⟹ 无过去 ⟹ B̂=0（不外推）。
- **time_block 窗间碰撞**：wverify 逐窗传 `win.i·WF_TIME_STRIDE=10000` 偏移隔开；窗内块=entry/43200。
- **残差置换鉴别力**（新单测 `residual_permutation_finds_no_structure_when_r_constant`）：当 δ-free 基 r
  与真实 δ 无关（买卖同 r）⟹ perm_p≈1（正确判无结构）——这是残差口径**区别于旧 X_γ 置换的关键**
  （X_γ 置换即使 r 无结构也会因重排 δ-baked 值制造虚假显著）。

## 4. 下游推论

- wverify_full（L2）重跑后主桶 mean/lcb/ucb/perm_p 全变**残差** Y 口径——若原 PASS 主桶
  （L0/type3/δ+1/σ^H=0）的 perm_p 在残差+四维分层下不再显著，则 B 组报告 §1.1「beta 未分离」
  被 L2 坐实（否定性结果，缩小有效域）。本工位不预判翻转（L0 只实装管线）。
- H2 逐级别 β 报告落 `/tmp/wv_full_h2_asymmetry.md`；原报告称 level0「卖优于买」更稳，L2 重跑可证。

## 5. 谱系引用

- **231（形式化有效域）**：残差减法/分层置换是 L0/L1 管线实装（零信息增量），L2 由 wverify_full 产出。
- **665/666/667**：δ 置换/perm_p/n_eff 门槛口径保留；本次将置换从 X_γ 口径改为 §4.2 残差口径。
- **#51（G-A2 h 缺口）**：本工位接通 h 桶管道（l3 entry/exit bar 差 → ResidualTrade），闭合该诚实缺口。
- B 组报告 `dlpdf-b-bsp-alpha-20260702.md` §3 缺口 1/2/4/5 的管线侧实装（缺口 3 跨标的 L3 属数据侧，非本域）。

## 6. 影响声明

- **改动文件**：mu_estimator.rs（+ResidualTrade/h_bucket/2 单测，MuObservation 及既有 API 未动）；
  perm_test.rs（stratified_delta_perm_p 残差重写 + drop_top_k_mean + direction_asymmetry + 测试更新）；
  l3_delta_r_alpha.rs（build_mu_from_bars 签名 +time_block_base/返回 records + 残差计算；
  build_walk_forward_mu 内部适配，对外 `(est,usize)` 签名不变 ⟹ L3 各 #[ignore] 测试不受影响）；
  wverify_run.rs（消费 records，桶/perm/报告全改 Y 口径 + H2/删尾）。
- **不改**：selector/runner/pooling/econ_positive 的 μ 语义（est 仍喂 X_γ）；prereg 冻结常量；无 git。
