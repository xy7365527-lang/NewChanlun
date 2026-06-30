# LCB(μ) 选择器 vs 裸 μ 选择器 L2 实证（acc-lcb-l2-vs-naive，task #74）

- **认识论等级**：L2（真实数据 walk-forward，可证伪，携信息增量）
- **goal**：g-20260630T155255Z-f65436f2 判决工位
- **跑法**：`cargo test --release --lib theta_v0::backtest::l3_delta_r_alpha::lcb_vs_naive_l2 -- --ignored --nocapture`
- **变量**：唯一变量 = `config.risk.chi_z_alpha`（0=裸 μ baseline vs 1.645=95% LCB）；同 μ 表 / 同 split / 同 θ=0 / 同 16K test 窗 / 8 品种

## 1. 结论

**两维度分离裁定（纪律1，660 正交）**：

| 维度 | 结果 | 裁定 |
|------|------|------|
| ① 过拟合控制 | 源(b)=39 ≫ 源(a)=9；BRN ΔR 由负转正，CL/OKLO 大幅改善 | **LCB 有真 alpha 价值**（高方差类正确收缩拒绝） |
| ② 统计功效 | 裸 μ n_L3=4 p=0.3125；LCB n_L3=1 p=0.5 | **两 selector 均 inconclusive**（功效不足 n_L3<5） |

**核心张力（两维度正交的物质化）**：LCB 在维度①改善过拟合的**代价**是维度②功效更弱——LCB 退化品种数(7) > 裸 μ(4)，把 L3 有效池从 n=4 压到 n=1。过拟合控制（拒更多高方差类）与功效（保留更多有效品种入 L3 池）拉锯。**LCB 单独不解决 inconclusive**。

## 2. 逐品种实证数据

```
symbol  test    μ类   χ0单   χL单   拒源a  拒源b   ΔR(裸μ)     ΔR(LCB)
BTC     16000   9     0      0     0      0      1.220e-5    1.220e-5
ES      16000   8     0      0     1      0      3.737e-6    3.737e-6
CL      16000   7     8667   0     2      15     1.721e-5    1.995e-5
GC      16000   9     349    349   0      0      3.482e-6    3.482e-6
BRN     16000   10    7936   0     0      10    -1.323e-6    1.415e-5  ← 负转正
DX      16000   9     0      0     4      0      1.044e-5    1.044e-5
QQQ     16000   5     0      0     0      0      8.274e-6    8.274e-6
OKLO    16000   7     3518   0     2      14     1.387e-6    9.558e-6  ← 大幅改善
```

## 3. 纪律2 两源归因（χ 空仓差异分解，跨品种）

- **源 (a) n<2 无 LCB 证据被拒** = **9**（样本饥饿，**非**过拟合控制）：n=1 类裸 μ mean 有定义但 mu_lcb=None，treat_empty=false ⟹ 滤。诚实但非 alpha 价值。
- **源 (b) n≥2 高方差 LCB<θ 被拒** = **39**（★LCB 真 alpha 价值）：裸 μ>θ 放行但 LCB=mean−1.645·std/√n<θ。裸 μ 在这些类过拟合估计噪声，LCB 正确收缩拒绝（p25 §12）。

**判据满足**：源(b)=39 > 0 ⟹ LCB 确有过拟合控制的 alpha 价值（非纯样本饥饿）。CL/BRN/OKLO 三品种的 χ 单数从 8667/7936/3518 全压到 0，正是源(b) 主导（CL 源b=15, BRN=10, OKLO=14）。

## 4. 定义依据

- LCB(μ) = mean − z_α·std/√n（`mu_estimator.rs:214` `mu_lcb`，n<2→None，严格alpha.pdf p25 §12 防高维 z 过拟合）。
- χ_t(γ)=1 ⟺ LCB(μ)>θ ∧ RiskOK ∧ ConflictOK（`selector.rs:121` `chi_open_gate_lcb`，alpha2 §13）。
- walk-forward μ 表因果纯净（train 前半估 μ，test 后半冻结，`l3_delta_r_alpha.rs:119` `build_walk_forward_mu`，codex Q1 无 test 泄漏）。
- ΔR = nav0·(E^χ − E^0) 逐 bar 绝对增量配对差（§10，`delta_r_stats` delta-r-audit P0 修复口径）。

## 5. 边界条件（结论翻转条件）

- **维度① 翻转**：若源(b)=0 ∧ 源(a)>0 ⟹ LCB 与裸 μ 差异纯样本饥饿，无 alpha 价值。实测源(b)=39≠0 ⟹ 不翻转。
- **维度② 翻转**：n_L3≥5 且符号检验 p<0.05 且池均值>0 ⟹ 系统性 alpha 成立。实测 n_L3≤4 ⟹ 功效不足，无法否证亦无法确认。扩大池（更多品种）或延长窗（>16K）可能翻转，但撞 O(n²) 性能墙（见影响声明）。
- **有效域 caveat**：16K bar 截断（O(n²) 全窗不可行）+ 8 品种 + OOS 内 split（非独立 train 历史）+ 保守欠对冲 pi（ρ 漂移剪枝）。

## 6. 下游推论

- LCB 改善单品种 ΔR（BRN 负转正、CL/OKLO 上移）= 维度①生效；但把有效品种从 L3 池剔除 = 维度②受损。**净效应在当前样本量下不可判**（功效不足）。
- 若 goal 要求"LCB 解决 inconclusive"——**否定性结果**：LCB 单独不足。需更大池/更长窗，受 O(n²) 性能墙阻塞（功效维度 blocked，非 LCB 失败）。
- 子集单调性诊断：LCB 退化数(7) ≥ 裸 μ(4)，与过滤层子集关系一致（LCB 放行集 ⊆ 裸 μ 放行集），runner 级联未引入非单调。

## 7. 谱系引用

- **660号（genealogist 正交警示）**：LCB 控过拟合 ⊥ inconclusive 根因（功效不足）——本实证证实正交（维度①好但维度②未改善，且 LCB 因更保守反使 n_L3↓）。
- **lcb-selector 两源归因**：χ 空仓两源（a 饥饿 / b 过拟合控制）分离——本实证实测 a=9, b=39。
- **231号（formalization-validity-domain）**：L2 否定性结果（功效维度 inconclusive）缩小有效域边界，比确认更有价值。

## 8. 影响声明

- **新增**：`rust/src/theta_v0/backtest/l3_delta_r_alpha.rs::lcb_vs_naive_l2` 测试（复用既有 harness/helper，零生产代码改动）。
- **未改动**：mu_estimator/selector/runner/config（工具上轮已 commit）。
- **性能墙**：扩大池/窗以提升维度②功效撞 O(n²)（train+test 各 16K 逐 bar 重分类，8 品种 ×2 z_alpha ≈12s）——更长窗或更多品种线性叠加但单窗 O(n²) 不可约（见 project_engine_on2_full_audit / project_bsp_event_stream_osb_wall）。
