# g3-neutral-bitexact｜q_Θ v1 (g2) Neutral 套端到端 OOS bit-exact 验证

**工位**：swarm/ws-g3neutral ｜ topo_address: swarm/ws-g3neutral ｜ 基因 073a/274号
**日期**：2026-07-05 ｜ 分支 gap3-rework-codex9-fix ｜ parent_callback: main
**认识论等级**（231/formalization-validity-domain 强制）：**L2**（真实 BTC OOS 跑批双提交 diff，可否证——任何漂移即实装 bug）。

---

## 〇、验收结论（PASS）

**Neutral 套（ThetaDirPreset::Neutral = default）端到端 OOS 输出与 v0 基线（af8910d062）逐字节一致——9/9 产物 bit-exact。**

g2（q_Θ v0→v1 σ_higher 分级权重，commit 0d732723bb）在 Neutral default 口径下**零漂移**：
- signal 层（`wverify_full`，5 窗 walk-forward，2256 residuals）：8 产物全 bit-exact；
- execution 层（`m8_e2e_all_systems_oos`，三系统同开三窗）：报告 bit-exact（n_orders/net_r/MaxDD/treasury 全不变）。

⟹ Follow/Adversary 套的任何变化（待 η 冻结后另跑）将**仅来自 σ_higher 分级权重**，非实装 bug。

---

## 一、双提交 diff 方法（为何 L2 可否证）

g2 单元报告（`qthetav1-impl-20260705.md`）已在叶函数级证明 `dir_weight≡1.0 ∀(role×depth)` under Neutral。
本工位做**端到端经验证**——同一 BTC OOS 跑批在两个 commit 上独立执行，逐产物 md5 diff：

| 侧 | commit | worktree | g2 状态 | 代码性质 |
|---|---|---|---|---|
| **v0 基线** | af8910d062 | `/tmp/g3-v0-wt`（detached） | **无**（`dir_weight`/`ThetaDirPreset` 计数=0） | signal-full 首轮 OOS 基线 |
| **v1** | b7819c3f22 (HEAD) | 主仓工作区 | **有**（g2 @ 0d732723bb） | 含 g2 + R5(5db6362093, env-gated) |

两 commit 间**唯一 theta 代码改动** = g2（coverage.rs `dir_weight`/`leg_target` + config.rs `ThetaDirPreset`）+ R5（env-gated 语义插桩，生产 bit-exact）+ g2 的 runner.rs OpsemDump YAGNI 重构（`OPSEM_DUMP_DIR` env-gated 诊断路径，非生产）。

**bit-exact 等价关系**：HEAD 生产 config（Neutral default，无 R5 env）≡ af8910d062 ⟺ g2 Neutral 是真 no-op 且 R5 env-gate 不泄漏。9/9 产物 md5 一致 = 二者同时经验确认（无法单独剥离 R5，但 R5 commit 自述 env-gated + 本 diff 全绿 = R5 不泄漏的间接佐证）。

**复现**：
```bash
# v1（HEAD Neutral，主仓）
cd rust && cargo test --release --lib theta_v0::backtest::wverify_run::wverify_full -- --ignored --nocapture
cd rust && cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
# v0（af8910d062 worktree，data_cache 符号链接回主仓——314M 数据 gitignored）
git worktree add --detach /tmp/g3-v0-wt af8910d062   # GIT_LFS_SKIP_SMUDGE=1（.chanlun LFS 不影响 rust 构建）
ln -s <main>/analysis/data_cache /tmp/g3-v0-wt/analysis/data_cache
cd /tmp/g3-v0-wt/rust && cargo test --release --lib theta_v0::backtest::wverify_run::wverify_full -- --ignored --nocapture
# diff /tmp/wv_full_* 与 /tmp/m8_e2e_all_systems_oos.md
```

---

## 二、9/9 产物 bit-exact 总表（md5 双提交一致）

| 产物 | 层 | 内容 | md5（v0=v1） | 状态 |
|---|---|---|---|---|
| `wv_full_zdecision.tsv` | signal | 2256 笔 Z_decision 逐笔 dump（resid_base/cost/d 均 `f64::to_bits` 十六进制，round-trip 精确） | `ee8580a8718abcf9841afb46230e6425` | **BIT-EXACT** |
| `wv_full_rows.md` | signal | 25 桶 raw μ 描述性表（n/n_eff/mean/lcb/ucb/cv/perm_p/state） | `29fb2fef99667f33995986c9e1cc6322` | **BIT-EXACT** |
| `wv_full_deltafree.md` | signal | δ-free 主裁决（Z_decision=(ℓ,bsp,parent_dir,force_state)，**acceptance gate**） | `fb2a1c820f47f06de37e570de9c53835` | **BIT-EXACT** |
| `wv_full_mu_r.md` | signal | μ_R=E[Y/d] co-primary 双门 | `e46a49bccde4545824aaa893e070108a` | **BIT-EXACT** |
| `wv_full_h2_asymmetry.md` | signal | H2 方向不对称 β block-bootstrap | `5050b222959efffbedff1eb72a087224` | **BIT-EXACT** |
| `wv_full_forcestate_ortho.md` | signal | ForceState⊥δ 前置检验 | `18fde7c9b06643ff1ce572ea72f99a90` | **BIT-EXACT** |
| `wv_full_timeblocks.md` | signal | 逐窗 time-block 残差均值 | `ad12706845790f5cf5585adc55c5437c` | **BIT-EXACT** |
| `wv_full_exittype.md` | signal | ExitType 5 变体诊断切片 | `67192929323607f8e14c83eb5a831093` | **BIT-EXACT** |
| `m8_e2e_all_systems_oos.md` | execution+treasury | 三系统同开三窗（n_orders/net_r/MaxDD/声部/Stage/Q_T/W_T/η_T/R/LCB） | `b5e66395291f6e800cd47ebc8f24d8e6` | **BIT-EXACT** |

**zdecision.tsv 行数**：v0=2257（1 header + 2256 trades）= v1=2257。订单数（residuals）bit-exact。

**execution 三窗关键值（v1=HEAD Neutral，= v0，= e2e-d2 基线）**：

| 窗 | n_orders | net_r(execR) | MaxDD | 终Stage |
|---|---|---|---|---|
| p3fold | 35692 | −15293070 | 0.9383 | I(降成本) |
| wf7 | 30917 | −21050235 | 0.8991 | I(降成本) |
| wf8 | 28373 | −23974747 | 0.8696 | I(降成本) |

三窗 net_r 与 e2e-d2 / maxfull-e2e-round1 报告转引值逐位一致 ⟹ HEAD-Neutral 复现已发布 v0 基线。

---

## 三、定义依据

- **Neutral 豁免定理**（prereg-rev4 §6.2 / g2 单元报告）：`ThetaDirPreset::Neutral` ⟹ `Θ_dir_neutral[ℓ][sign]≡1.0 ∀(ℓ,sign)` ⟹ `dir_weight≡1.0 ∀(role,depth)`（ShortDiff/Ambient 经 CASE 1/2 豁免；FollowParent 经 CASE 3 查 Neutral 表得 1.0）⟹ `s_e = base × w_depth × 1.0 = v0 s_e`。
- **端到端传导链**（g2 报告 §四）：`leg_target`(s_e 不变) ⟹ `net_target_units`(p̃ 不变) ⟹ `pi_theta_position`(p* 不变) ⟹ `schedule_order`(订单不变) ⟹ `typed_ledger`(records 不变) ⟹ μ 桶表/residuals 不变；overlay 臂（`run_theta_v0_pi_overlay`）同源 ⟹ trade_pnls/net_r/treasury 不变。
- **输入特征满足定义的条件**：BTC OOS（btc_1m_full.json，314M，5 窗 walk-forward test_start≥OOS_START）经生产 π fill loop（`typed_ledger_from_bars` / `run_theta_v0_pi_overlay`）产 2256 residuals + 三窗 execution 真值；两 commit 同数据同 config（ThetaConfig::default()，v0 无 theta_dir 字段 ≡ v1 theta_dir=Neutral）⟹ 唯一变量 = g2 代码。
- **zdecision.tsv bit-exact 的精度含义**：`resid_base.to_bits()`/`cost.to_bits()`/`d.to_bits()` 十六进制 = IEEE-754 双精逐位相等（非格式化十进制近似）。md5 一致 ⟹ 2256 笔 × 3 浮点分量 = 6768 浮点位模式全等。

---

## 四、边界条件（结论翻转）

- **(a) 翻转 = 实装 bug**：若任一产物 md5 不一致 ⟹ g2 Neutral 不是真 no-op（单元测试漏 role/depth 组合，或 R5 env-gate 泄漏生产）。**本轮 9/9 一致 ⟹ 不翻转**。
- **(b) Follow/Adversary 套不保留 bit-exact**（定理 2，非 bug）：切 Follow/Adversary ⟹ s_e 改 ⟹ p̃/p*/订单/ledger/μ 桶全改——这是 σ_higher 权重的预期信号，**须 η_adv(ℓ)/η_same(ℓ) 数值跑数前冻结（135号）**，不在本工位。
- **(c) R5 单独剥离不可行**：两 commit 间 R5 与 g2 同时存在；本 diff 全绿是「g2 Neutral no-op ∧ R5 不泄漏」的**联合**证据，无法单独证 R5。R5 commit 自述「env gated 生产 bit-exact」+ 本联合证据 = 充分（R5 独立验证不在 g3 范畴）。
- **(d) 数据/窗口不变性**：两 commit 共享同一 btc_1m_full.json（符号链接）+ 同 PREREG_WINDOWS 冻结 ⟹ 窗口选择非翻转源。

---

## 五、下游推论

- **对 g3 Follow/Adversary**：Neutral 端到端零漂移已坐实 ⟹ Follow/Adversary 跑批（待 η 冻结）的任何输出变化**可纯净归因到 σ_higher 分级权重**（w_dir ≠ 1 的腿），无实装 bug 噪声。这是 σ_higher estimand 域分离（231号）的实装前提。
- **对 Π_max-full OOS 基线**：HEAD-Neutral 三窗 net_r（−15293070/−21050235/−23974747）= e2e-d2/maxfull-e2e-round1 已发布值 ⟹ g2 合入主仓**不破坏 M8 已闭合基线**（default config 不变 ⟹ 所有已发布 OOS 数值仍可复现）。
- **对 g2 单元证明**：单元级 `w_dir_neutral_is_identity_for_all_18_roles`（叶函数）+ 本工位端到端 OOS（整链）= **两级证据闭合**（L1 叶 + L2 链）。Neutral no-op 在单元与集成两层均经验证。

---

## 六、谱系引用

- **g2 实装**：commit 0d732723bb；报告 `qthetav1-impl-20260705.md`（单元级 18 角色 × depth Neutral identity 证明）。
- **g1 冻结**：prereg-rev4（958f722674 / 54ec3b6384）——§6.2 Neutral bit-exact 定理、§4.1 dir_weight 三 CASE、§4.3 ThetaDirPreset 三套预注册。
- **v0 基线**：af8910d062（signal-full 首轮 OOS 报告 `signalfull-oos-round1-20260704.md` 跑批 HEAD bd01afc1b8）；execution 基线 `e2e-d2-20260704.md` / `maxfull-e2e-round1-20260704.md`。
- **规则**：231/formalization-validity-domain（L2 标注 + 有效域）；090（bit-exact 严格性）；135（η 冻结先于 Follow/Adversary 跑数）；161（否定性/零漂移照实合格）。
- **memory**：`project_m0m8_mainline_closed`（M8 基线 net_r 三窗值，本工位复现）；`project_formal_chain_downloads_authority`。
- **发生史无新概念分离**：本工位是 g2 实装的端到端经验证，未产生新谱系条目。

---

## 七、影响声明

- **新增文件**：`.chanlun/review-results/g3-neutral-bitexact-20260705.md`（本报告）。
- **零代码改动、零 commit**（任务约束）——纯验证工位，g2 代码已在 0d732723bb 合入主仓。
- **临时 worktree**：`/tmp/g3-v0-wt`（af8910d062 detached，含 analysis/data_cache 符号链接）。验证完成，可 `git worktree remove /tmp/g3-v0-wt --force` 清理（不触及主仓工作区）。
- **测试**：`wverify_full`（v0 49.69s / v1 51.56s，2 passed each）+ `m8_e2e_all_systems_oos`（v0 29.63s / v1 29.62s，1 passed each）全绿；1630/1635 filtered（单测隔离）。
- **引用但未修改**：g2 代码（coverage.rs/config.rs/runner.rs）、wverify_run.rs 测试、btc_1m_full.json 数据。

---

**g3 Neutral 端到端 bit-exact 验证结束**。q_Θ v1（g2）在 Neutral default 下端到端 OOS == v0 基线（af8910d062）9/9 产物逐字节一致。Follow/Adversary 套待 η 冻结后另跑（135号）。ready-for-shutdown。
