# 三路对比 L2 结果：裸μ vs LCB vs shrinkage（acc-three-way-l2）

重跑：cargo test --release three_way_l2 --ignored（12.95s，background bd35kcz70）。θ=0, z_α=1.645, τ²=1。

## 逐品种（χ单=准入交易数，ΔR=配对差/nav0）
| symbol | χ0(裸μ) | χL(LCB) | χS(shrink) | ΔR裸μ | ΔR_LCB | ΔR_shrink |
|--------|------|------|------|------|------|------|
| BTC | 0 | 0 | 0 | 1.22e-5 | 1.22e-5 | 1.22e-5 [弃权] |
| ES | 0 | 0 | 0 | 3.74e-6 | = | = [弃权] |
| CL | 8667 | 0 | 8667 | 1.72e-5 | 1.995e-5 | 1.72e-5 |
| BRN | 7936 | 0 | 7936 | −1.32e-6 | 1.415e-5 | −1.32e-6 |
| DX | 0 | 0 | 0 | 1.04e-5 | = | = [弃权] |
| QQQ | 0 | 0 | 0 | 8.27e-6 | = | = [弃权] |

## 保功效维度（shrinkage 卖点判据）
- 退化品种数（χ→0空仓）：裸μ=4 | LCB=7 | **shrink=4**
- n_L3：裸μ=4 | LCB=1 | **shrink=3**
- L3 符号检验：裸μ p=0.3125 / LCB p=0.5 / shrink p=0.5 —— **三路全 inconclusive（n_L3<5）**

## 结论（落可证伪分支 b）
**shrinkage 不解 inconclusive**。保功效诊断 n_L3(shrink=3) < 裸μ(4) → shrinkage **未兑现保功效**（n_L3 仍降，虽退化数 4<LCB 7 比 LCB 好）。
关键：shrink 在非退化品种（CL/BRN）的 ΔR **等于裸μ**（shrunk mean 仍 >θ，准入集与裸μ同）——shrinkage 在本数据**对准入决策无实质改变**。
- LCB：拒绝低n类 → 退化 7，n_L3 压到 1（毁功效）
- shrink：低n类收缩后仍多数 >θ → 准入≈裸μ（CL/BRN χ单同 8667/7936）→ 既没像 LCB 那样过度拒绝，也没改善
- **三路全 inconclusive 证实：根因是功效不足（16K短窗+8品种 n_L3<5），非估计方法可解**（裸μ/LCB/shrink 都判不出 alpha）。

## 谱系
- 落 660 同根：功效不足非过拟合控制可解。shrinkage（保功效解药）在本数据样本量下也无法把 inconclusive 升为判定。
- 161：退化品种 ΔR 标[弃权]不计改善。
- 印证第5份PDF §31 Le Cam 下界：n·d²≪1 时任何估计方法不可区分——shrinkage 也是估计方法，逃不出信息论下界。
- **下游**：要判 alpha 必须攻功效不足本身（更大池/更长窗=第5份PDF 跨品种pooling goal，或承认 Le Cam 硬墙）。

L2，标 n/窗口/z_α/τ²。待 codex 异质审。
