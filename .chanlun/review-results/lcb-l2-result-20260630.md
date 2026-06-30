# L2 实证：LCB 选择器 vs 裸μ 选择器（acc-lcb-l2-vs-naive，task #74/#78）

> **2026-06-30 重判（task #78）**：codex 异质审（`codex-lcb-l2-audit-20260630-1218.md`）判原结论 (B) 缺陷，
> 4 攻击点全 SUCCEEDS。本文档为**修复 + 诚实重判**版本，**撤回**原"LCB 有真 alpha 价值"结论。

- **测试**：`rust/src/theta_v0/backtest/l3_delta_r_alpha.rs::lcb_vs_naive_l2`
- **认识论等级**：**L2**（真实数据 8 品种单标的对比，z_alpha 是唯一变量；产出了**否定性结果**）
- **n / 窗口 / z_alpha**：8 品种（BTC/ES/CL/GC/BRN/DX/QQQ/OKLO），OOS=(2023-01-01, 2025-06-30)（OKLO §2.4 特例），
  截断 MAX_BARS=32000 → train 16000 / **test 16000** bar；θ=0；**z_naive=0（裸μ）vs z_lcb=1.645（95% 单边 LCB）**
- **唯一变量**：`config.risk.chi_z_alpha`（同 walk-forward μ 表 / 同 split / 同 θ / 同 test 窗）

## 逐品种结果（修复后，含 src_b 分桶 + 退化弃权标记）

| symbol | μ类 | 裸μ单 | LCB单 | 拒源(a)饥饿 | 拒源(b)总 | src_b鲁棒(n≥10) | src_b低dof(n<10) | LCB退化? | ΔR(裸μ) | ΔR(LCB) |
|--------|----:|------:|------:|----:|----:|----:|----:|:--:|--------:|--------:|
| BTC  |  9 | 0    | 0 | 0 | 0  | 0  | 0 | 弃权 | 1.220e-5 | 1.220e-5 |
| ES   |  8 | 0    | 0 | 1 | 0  | 0  | 0 | 弃权 | 3.737e-6 | 3.737e-6 |
| CL   |  7 | 8667 | 0 | 2 | 15 | 14 | 1 | **弃权** | 1.721e-5 | 1.995e-5 |
| GC   |  9 | 349  | 349 | 0 | 0 | 0 | 0 | 否 | 3.482e-6 | 3.482e-6 |
| BRN  | 10 | 7936 | 0 | 0 | 10 | 10 | 0 | **弃权** | −1.323e-6 | +1.415e-5 |
| DX   |  9 | 0    | 0 | 4 | 0  | 0  | 0 | 弃权 | 1.044e-5 | 1.044e-5 |
| QQQ  |  5 | 0    | 0 | 0 | 0  | 0  | 0 | 弃权 | 8.274e-6 | 8.274e-6 |
| OKLO |  7 | 3518 | 0 | 2 | 14 | 14 | 0 | **弃权** | 1.387e-6 | 9.558e-6 |

（"弃权" = LCB.n_orders==0 且 base.n_orders>0：χ 全滤空仓，ΔR=flat-selector vs always-open base 非 alpha）

**关键聚合**：
- 源(a)=9，源(b)总=39（CL15+BRN10+OKLO14）
- **源(b) 鲁棒(n≥10)=38，低dof(n<10)=1** ← codex 攻击点3（n=2 伪高方差）在本批数据上**基本不成立**（仅 CL 1 个低dof）
- **源(b) 鲁棒 ∧ 非退化品种 = 0** ← codex 攻击点1+4 致命：全部 38 个鲁棒 src_b 落在 CL/BRN/OKLO，而这三个全是 LCB 退化品种

---

## 结果包六要素

### 1. 结论（撤回原"alpha 价值"，改为否定性结论）

**LCB 选择器无 demonstrated alpha 价值**（`tot_src_b_robust_nondegen = 0`）。

剥离两层伪装后真相：
- src_b=39 全部集中在 CL/BRN/OKLO，而这三个品种在 LCB 下 χ.n_orders→0（**全滤空仓 = 停止交易**）。
- 停止交易 = 平凡零过拟合（按构造：从不交易则从不过拟合）。把"χ→0"算作"LCB 正确收缩高方差类"
  是把**退化**用相反价值符号重贴标签为**alpha**（codex 攻击点1+4）。
- BRN 的 ΔR「负转正」（−1.3e-6→+1.4e-5）**不是 alpha 改善**：LCB.n_orders=0 ⟹ delta_r_stats 比较
  flat-selector vs always-open base，下跌窗里"空仓跑赢 always-long"是**弃权 PnL**，非信号质量发现（161 号）。

**稳健陈述（codex 异质审 (B) 结论的可成立内核）**：LCB 比裸μ 更保守，把 3 个品种（CL/BRN/OKLO）从
"有单"压成"全滤空仓"，这降低了表观过拟合度量、同时摧毁了统计功效（L3 池 4→1）。这是同一收紧机制的两面，
不是"LCB 发现并修剪了真过拟合"。

### 2. 定义依据

- 准入量定义（selector.rs:188-200）：`chi_t(est.mu_lcb(&z, z_alpha), θ, ...)`，`mu_lcb=mean−z_alpha·std/√n`
  （mu_estimator.rs:214，n<2 返 None）。
- ΔR 口径（delta_r_stats，l3_delta_r_alpha.rs:239）：归一化绝对权益逐 bar 增量配对差 = §10 ΔR/nav0。
  **退化品种（n_orders=0）的 ΔR = flat 权益 vs always-open base 权益的增量差**——这是弃权对照，非 alpha。
- src_b 分桶依据（mu_estimator.rs:221 `count`）：est.count(z) = 该 z 在 train μ 表的样本量 n。

### 3. 边界条件（结论翻转的条件）

- **否定结论翻转**：若在**更长窗 / 更大品种池**下，存在某品种满足 (LCB.n_orders>0 ∧ 存在 n≥10 高方差类被 LCB
  拒) ⟹ `tot_src_b_robust_nondegen>0` ⟹ LCB 获得**有限**（窄）过拟合控制价值。当前 16K×8 品种数据下为 0。
- **退化判定翻转**：退化由 runner.rs RiskOK/ConflictOK/持仓树级联调制，非纯过滤层单调。若某品种 LCB 收紧后
  仍有 n_orders>0（非退化）且其 src_b 鲁棒 ⟹ 计入 alpha 候选。
- **低dof 分桶阈值**：n≥10 为鲁棒阈（约 9 自由度方差估计可信）。若改阈值（如 n≥30）src_b_robust 会更小——
  当前 38/39 已 n≥10，阈值敏感性低（codex 攻击点3 在本数据上钝化）。

### 4. 下游推论

- **LCB 不应作为 chi_z_alpha 生产默认的 alpha 理由**：原"保留 1.645 作生产默认候选"的推论**撤回**——
  本 L2 未给出 LCB 的 demonstrated alpha 证据，唯一可观测效应是"更保守地压品种空仓"。
- **覆盖≠盈利（645/653/654）再次兑现**：LCB 改变交易集（CL/BRN/OKLO 全滤）≠ alpha；空仓躲亏 ≠ 选择 alpha。
- **维度①②非正交**（codex 攻击点2）：收紧 LCB → 拒更多类（降过拟合度量）**同时** → 推品种向 n_orders=0
  退化 → 排除 L3 池（降功效）。同一 rejection mass 两投影，不是两个独立维度。原"两维度正交"声明**删除**。

### 5. 谱系引用

- **161 号（务实/补丁思维否定）**：本修复的核心——"停止交易（χ→0 空仓）不可粉饰为改善"。原结论把退化
  弃权 PnL 表述为"LCB 真 alpha 价值"撞 161 号，是声明膨胀（090 号）。
- **660（χ_t L3 inconclusive 伪否证 + 根因分离）**：原引用 660 的"正交"框架被 codex 攻击点2 否证——
  过拟合维度与功效维度**非正交**而是同源。保留 660 的"inconclusive 是功效不足非 LCB 失败"内核，
  **撤回**"两维度正交"的形式化。
- **231（formalization-validity-domain）**：否定性结果（`tot_src_b_robust_nondegen=0`）照实报，标 L2 +
  有效域 caveat（16K 截断 + 8 品种 + OOS 内 split）。**否定性结果缩小了 LCB alpha 价值的有效域边界**——
  比原确认性结果更有信息增量（231 号"否定性结果比确认性结果更有价值"）。

### 6. 影响声明

- **改动文件**：`rust/src/theta_v0/backtest/l3_delta_r_alpha.rs` 的 `lcb_vs_naive_l2`（测试代码，非 frozen 生产路径）：
  - (a) src_b 按 train-class n 分桶（n≥10 鲁棒 vs n<10 低dof），剥离低自由度伪装。
  - (b) 退化品种（LCB.n_orders=0）ΔR 标 `[LCB弃权PnL]`，新增 `tot_src_b_robust_nondegen`（排除退化）作唯一 alpha 判据。
  - (c) 删除"两维度正交"声明，改为"同一 rejection mass 两投影"（codex 攻击点2）。
- **未碰** selector/estimator/runner 生产代码（已真封）。
- **测试计数不降**：`cargo test --lib` → **1268 passed / 0 failed**（baseline 不降）；
  `cargo test --lib theta_v0::backtest::l3_delta_r_alpha` → 7 passed / 4 ignored。
- **重跑数据**：`cargo test --release --lib ...::lcb_vs_naive_l2 -- --ignored --nocapture` → 1 passed（12.0s）。

---

## 诚实重判（防再次 090 膨胀）

| codex 攻击点 | 修复 | 修复后数据 | 判定 |
|------|------|------|------|
| 1+4（退化弃权伪装为 alpha） | 排除退化品种，`tot_src_b_robust_nondegen` | =0（全部 src_b 在退化品种） | **致命，结论翻转** |
| 3（n=2 低自由度伪高方差） | src_b 按 n 分桶 | 38 鲁棒/1 低dof（n≥10 充足） | 本数据上钝化（非主因） |
| 2（两维度正交虚假） | 删正交声明 | — | 接受，已删 |

**最终裁定**：原结论"LCB 在 3/8 品种有显著过拟合控制 + BRN 翻正是 alpha"**被否证**。
诚实结论：**LCB 无 demonstrated alpha 价值**（src_b_robust_nondegen=0）——LCB 唯一可观测效应是更保守地
把 CL/BRN/OKLO 压向空仓（弃权），而退化弃权 PnL 不是 alpha（161 号）。

功效维度独立 inconclusive（n_L3<5）不变——这与过拟合维度**同源**（非正交，codex 攻击点2），均是
LCB 收紧 rejection mass 的投影；根因是 16K 短窗 + 8 品种功效不足（撞 O(n²) 全窗墙，诚实报 **blocked**）。
