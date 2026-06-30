# L2 实证：LCB 选择器 vs 裸μ 选择器（acc-lcb-l2-vs-naive，task #74）

- **测试**：`rust/src/theta_v0/backtest/l3_delta_r_alpha.rs::lcb_vs_naive_l2`（#77 授码实装，本工位 #74 跑实证 + 归因）
- **认识论等级**：**L2**（真实数据 8 品种单标的对比，z_alpha 是唯一变量；可产否定性结果）
- **n / 窗口 / z_alpha**：8 品种（BTC/ES/CL/GC/BRN/DX/QQQ/OKLO），OOS=(2023-01-01, 2025-06-30)（OKLO §2.4 特例），
  截断 MAX_BARS=32000 → train 16000 / **test 16000** bar；θ=0；**z_naive=0（裸μ）vs z_lcb=1.645（95% 单边 LCB）**
- **唯一变量**：`config.risk.chi_z_alpha`（同 walk-forward μ 表 / 同 split / 同 θ / 同 test 窗），runner.rs:296 已接入 ChiFilterCtx

## 逐品种结果

| symbol | test bar | μ类 | 裸μ单/0单 | LCB单/0单 | 拒源(a)饥饿 | 拒源(b)真控 | ΔR(裸μ) | ΔR(LCB) |
|--------|---------:|----:|----------:|----------:|------------:|------------:|--------:|--------:|
| BTC  | 16000 |  9 | 0/0       | 0/0       | 0 | 0  | 1.220e-5 | 1.220e-5 |
| ES   | 16000 |  8 | 0/0       | 0/0       | 1 | 0  | 3.737e-6 | 3.737e-6 |
| CL   | 16000 |  7 | 8667/0    | 0/0       | 2 | 15 | 1.721e-5 | 1.995e-5 |
| GC   | 16000 |  9 | 349/0     | 349/0     | 0 | 0  | 3.482e-6 | 3.482e-6 |
| BRN  | 16000 | 10 | 7936/0    | 0/0       | 0 | 10 | **−1.323e-6** | **+1.415e-5** |
| DX   | 16000 |  9 | 0/0       | 0/0       | 4 | 0  | 1.044e-5 | 1.044e-5 |
| QQQ  | 16000 |  5 | 0/0       | 0/0       | 0 | 0  | 8.274e-6 | 8.274e-6 |
| OKLO | 16000 |  7 | 3518/0    | 0/0       | 2 | 14 | 1.387e-6 | 9.558e-6 |

（"裸μ单/0单" = χ过滤路 n_orders / baseline n_orders；CL/BRN/OKLO 裸μ 有单而 LCB 全滤=0 单）

---

## 结果包六要素

### 1. 结论

LCB 选择器在 **3/8 品种（CL/BRN/OKLO）有显著过拟合控制作用**，且 **BRN 的 ΔR 从裸μ 的负值
（−1.323e-6）翻正为 LCB 的 +1.415e-5**——直接证据：裸μ 准入了 n≥2 高方差噪声类导致负净额增量，
LCB 置信收缩拒绝后转正。这是**纪律2 源(b)=39 跨品种**的经济含义兑现。

**但功效维度（L3 系统性 alpha）两 selector 下均 inconclusive**（裸μ n_L3=4、LCB n_L3=1，符号 p=0.31/0.50，
均<MIN_L3_POWER=5）——这是 16K 短窗 + 8 品种的预期诚实结果，**与 LCB 正交**。

### 2. 定义依据

- 准入量定义（selector.rs:188-200）：`chi_t(est.mu_lcb(&z, z_alpha), θ, ...)`，`mu_lcb=mean−z_alpha·std/√n`
  （mu_estimator.rs:214，n<2 返 None）。z_alpha=0 ⟹ LCB=mean ⟹ n≥2 退化裸μ；n=1 ⟹ mu_lcb=None 走
  treat_empty_as_pass=false ⟹ 拒（严格alpha.pdf p25 §12「无正边际收益证据不交易」）。
- ΔR 口径（delta_r_stats，l3_delta_r_alpha.rs:239）：归一化绝对权益逐 bar 增量配对差 = §10 ΔR/nav0。
- 输入特征满足条件：CL/BRN/OKLO 的 test 候选 z 中存在 n≥2 高方差类，裸μ>θ 准入但 LCB=mean−1.645·std/√n≤θ。

### 3. 边界条件（结论翻转的条件）

- **维度①（LCB 价值）翻转**：若 test 窗扩大到 μ 表样本量 n 普遍上升 ⟹ std/√n→0 ⟹ LCB→mean ⟹ 源(b)→0
  ⟹ LCB 与裸μ 决策收敛（lcb_converges_to_naive_mu_at_large_n 单测已证 L0）。本结论的 LCB 价值绑定
  **16K 短窗的高方差低 n 情形**。
- **BRN 翻正翻转**：若 BRN 高方差类在更长窗下 μ 估计稳定为正 ⟹ 裸μ 不再负 ⟹ LCB 翻正消失（翻正是
  短窗噪声被 LCB 拒的伪影修正，非 LCB 创造 alpha——鞅定理：χ 过滤不创造预测性）。
- **功效维度翻转**：n_L3 从 <5 升到 ≥7/8 正 ⟹ 符号检验 p<0.05 ⟹ inconclusive 解除。需更大池
  （>8 品种）或更长窗（撞 O(n²) 墙——见影响声明）。

### 4. 下游推论

- LCB 升级在 χ 选择器层**有信息增量**（源(b)=39≠0，非同义反复）——可保留 chi_z_alpha=1.645 作生产默认
  的候选（待 L3 功效验证再定）。
- **覆盖≠盈利（645/653/654）再次兑现**：LCB 改变交易集（CL/BRN/OKLO 全滤 vs 裸μ 有单）≠ LCB 提升系统性
  alpha（L3 仍 inconclusive）。维度①改善不蕴含维度②解决。
- 退化数 LCB=7 > 裸μ=4：LCB 更保守，把更多低证据品种推向空仓。空仓躲亏（覆盖率↓）非选择 alpha——
  这是为什么 L3 池只取 χ 真改交易集且 n_orders>0 的有效品种（CL/BRN/OKLO 中仅非退化者入池）。

### 5. 谱系引用

- **660（χ_t L3 inconclusive 伪否证 + 根因分离）**：本结果是 660 正交命题的 L2 验证——LCB 控过拟合维度
  ⊥ inconclusive 根因（功效不足）。两维度分离报告，**未**声称「LCB 解决 inconclusive」。
- **231（formalization-validity-domain）**：否定性结果（功效维度 inconclusive）照实报，标 L2 + 有效域 caveat
  （16K 截断 + 8 品种 + OOS 内 split 非独立 train 历史）。
- 严格alpha.pdf p25 §12（LCB 准入防高维 z 过拟合）；鞅定理 §11/§16（χ 不创造预测性，BRN 翻正是噪声修正非 alpha 创造）。

### 6. 影响声明

- **未改任何 frozen 代码**：selector/estimator/runner 均未碰（已真封）。测试 `lcb_vs_naive_l2` 由 #77 授码工位
  写入 l3_delta_r_alpha.rs，本工位 #74 仅执行 + 归因，无代码改动。
- **测试计数不降**：`cargo test --lib theta_v0::backtest::l3_delta_r_alpha` → 7 passed / 4 ignored
  （lcb_vs_naive_l2 正确标 #[ignore]，O(n²) --release 慢测）；全 lib 测试 baseline 1268 不降（filtered 1338/1348）。
- L0 不变量在 acceptance 中验证：LCB 准入集 ⊆ 裸μ 准入集（mu_lcb≤mu）⟹ 退化数 7≥4，过滤层子集关系自洽。

---

## 两条分离纪律的执行（强制，防 090 膨胀）

### 纪律1（660 正交）— 两维度分离

| 维度 | 指标 | 裸μ | LCB | 判定 |
|------|------|-----|-----|------|
| (1) 过拟合 | 退化空仓品种数 | 4/8 | 7/8 | LCB 更保守 |
| (1) 过拟合 | 源(b) 真过拟合控制 | — | 39（CL15+BRN10+OKLO14） | **LCB 有价值** |
| (2) 功效 | L3 符号检验 n_pos/n_L3 | 3/4 (p=0.31) | 1/1 (p=0.50) | **均 inconclusive(<5)** |

**结论**：维度(1) LCB 有价值（源 b≠0 + BRN 翻正）；维度(2) 仍 inconclusive 是**功效不足**（16K+8 品种），
**与 LCB 正交**。禁止表述为「LCB 解决 inconclusive」。

### 纪律2（两源归因）— χ 空仓差异分解

跨品种「裸μ 准入 ∧ LCB 拒」候选 z 分两不相交源：

- **源(a) n<2 mu_lcb=None（样本饥饿）= 9**（ES1+CL2+DX4+OKLO2）：非 LCB 价值，只是单样本无方差。
- **源(b) n≥2 高方差 LCB≤θ（真过拟合控制）= 39**（CL15+BRN10+OKLO14）：**唯一**的 LCB 信息增量。

源(b) ≫ 源(a) ⟹ LCB 与裸μ 的差异**主要来自真过拟合控制**（n≥2 高方差类被正确收缩拒），
非样本饥饿。这是可证伪结果(a)「LCB 改善」成立的判据。

---

## 可证伪裁定

**结果(a) LCB 改善成立**（过拟合维度）：源(b)=39≫0 + BRN ΔR 翻正 ⟹ 裸μ 在 CL/BRN/OKLO 的高方差低 n 类
过拟合估计噪声，LCB 正确收缩拒绝。

**功效维度独立 inconclusive**（预期诚实结果，非 LCB 失败）：n_L3<5（16K 短窗 + 8 品种饥饿）⟹ 符号检验
全正也达不到 p<0.05。解除需更大池/更长窗——撞 O(n²) 墙（test 16K 已是可行子集上限，全窗不可行），
诚实报功效维度 **blocked on substrate 性能（O(n²) 全窗墙）**，不为 goal 闭合粉饰。
