# 642 ΔSharpe L2 重测（当前 HEAD，机制修复已实装后）

- 日期：2026-07-02
- 工位：ws-642retest（task #6）
- 谱系依据：`.chanlun/genealogy/pending/642-*.md`（误判降级：定义冲突→工程缺口）+ 裁决⑤ §642（`codex-ritual-resubmit-20260702.md`：成立，限定"机制级已解决"）
- 认识论等级：**L2**（真实 CL/BTC 32K-bar OOS 单窗，可产否定性结果）
- 数据：`analysis/data_cache/{cl,btc}_1m_databento_10y.json` 等 8 品种；OOS 2023-01-01→2025-06-30；1min bar；32000-bar 截断窗（全窗 O(n²) 不可行，~75h/品种）

---

## 一句话结论

机制修复（coverage.rs candidate-carrier-id 入 raw + parent_id AncOK + registry 祖先链）**已生效并在真实 L2 数据上流动**——depth>0 对冲腿现在被准入（预修复为 0）。但 depth 叠加层的**风险调整净值贡献 ΔSharpe = 0.000**（`|Δ|<5e-4`，8/8 品种含 CL/BTC，两次生产回测 NAV 口径实测），且 #5 组合自身 Sharpe 为负、8/8 被 (b) L2 否证。**修复≠盈利（231 铁律）：机制从 b2 迁到 b1，但经验 alpha 仍为零。**

---

## 结论（六要素·完整版，涉及 642 memory 概念修正）

### 1. 结论

两个正交测量在当前 HEAD 同时坐实：

**(A) 机制层——修复已流动（`l3_pi_depth_diag_cl_btc`，真实 CL/BTC）**

| 指标 | BTC | CL | 预修复 | 含义 |
|------|-----|-----|--------|------|
| active_depth1 腿 | 74908 | 90752 | 0 | depth>0 子腿现被 AncOK 准入 |
| active_depth2 腿 | 56947 | 41575 | 0 | 同上 |
| active_depth_ge3 腿 | 20683 | 18014 | 0 | 真 1→2→3 链准入 |
| held_op_parent_alive | 152530 | 150333 | 0 | depth>0 腿的 op_parent 在 registry 存活 |
| sd_parent_held | 294 | 28107 | 0 | 父持仓且子准入（预修复=0=父从不持仓） |
| **‖ΔN‖₁**（净头寸位移） | **8,168,700** | **22,698,700** | **0** | 净目标头寸被 depth 层改变 |
| ΔN_t≠0 的 bar 数 | 22331/32000 | 29972/32000 | 0 | — |
| 裁定 | **b1** | **b1** | b2 | 净敞口确被改变（非 net-cancel） |

预修复态是 **b2**（`‖ΔN‖₁≡0`，两组合逐 bar 净头寸恒等 ⟹ ΔSharpe 机械地必为 0）。当前 HEAD 是 **b1**（`‖ΔN‖₁>0`，两组合真正不同）。**"depth>0 零贡献"这一前提在净目标头寸层被证伪。**

**(B) 净值层——ΔSharpe 实测（`l3_pi_falsify_multi_symbol_significance`，两次生产回测对照）**

`ΔShrp = Sharpe(#5, max_depth=3) − Sharpe(baseline, max_depth=1)`，两条均由修复后的 `run_theta_v0_pi` 生产。

| 品种 | real | trd | Sharpe(#5) | **ΔShrp** | ΔCalmar | boot_p | beats? | 归因 |
|------|------|-----|-----------|-----------|---------|--------|--------|------|
| BTC | 321 | 1480 | −0.980 | **0.000** | 0.000 | 1.0000 | false | (b)否证:收益不显著 |
| ES | 1077 | 6154 | −0.938 | 0.000 | 0.000 | 1.0000 | false | (b)否证 |
| CL | 1140 | 11011 | −0.615 | **0.000** | 0.000 | 1.0000 | false | (b)否证:收益不显著 |
| GC | 970 | 5669 | −1.078 | 0.000 | 0.000 | 1.0000 | false | (b)否证 |
| BRN | 1099 | 9024 | −0.643 | 0.000 | 0.000 | 1.0000 | false | (b)否证 |
| DX | 1110 | 8447 | −2.248 | 0.000 | 0.000 | 1.0000 | false | (b)否证 |
| QQQ | 614 | 5970 | −0.422 | 0.000 | 0.000 | 1.0000 | false | (b)否证 |
| OKLO | 1708 | 14255 | −0.020 | 0.000 | 0.000 | 0.7730 | false | (b)否证 |

聚合：完成 8/8，(b) L2 否证 8/8，(c') 确认 0/8，打败两随机对照 0/8。

**(C) 两测量的调和（关键）**：depth>0 腿被准入且改变**净目标头寸**（‖ΔN‖₁>0，b1），但两次生产回测的 **NAV 权益曲线 Sharpe 差 <5e-4**。即 depth 叠加层进入了净目标账本，却对已实现净值零风险调整贡献——正是 b1 判据的本义（"净敞口被改变但成本/方向/时点/权重抵消，经验无 alpha"），现由**实际两跑 NAV 测量**坐实，而非诊断桩里那句 baked 的"ΔSharpe=0"标签。

### 2. 定义依据

- **642 谱系**：修复 = coverage.rs candidate carrier id 入 raw + parent_id 检查 + registry-live 时 restore_ancestor_chain。裁决⑤ §642 已核实实装并限定"'已解决'只覆盖 H2/AncOK 准入机制，不等价于 ΔSharpe 已非零"。本重测正是补齐该限定语要求的 L2 证据。
- **depth-0 baseline 定义**（`l3_pi_falsify.rs:65-67`）：`voice.max_depth=1` ⟹ `within_max_depth(d)=d<1` ⟹ 仅根声部 d=0 开仓。#5 = 默认 `max_depth=3`。ΔShrp 即两者 NAV Sharpe 之差。
- **b1/b2 判据**（`l3_fullwindow.rs:355-356`，ChatGPT §5 定理2）：`‖ΔN‖₁=0 ∀t ⟹ b2`（净额化后净头寸未变，NAV 原理上看不见）；`‖ΔN‖₁>0 但 ΔSharpe=0 ⟹ b1`（净敞口被改但经验无 alpha）。

### 3. 边界条件（结论翻转条件）

- 若在**全窗**（非 32K 截断）或其他数据/更高频率下重跑，`ΔShrp` 出现显著非零且 boot_p≤0.05 且优两随机对照 ⟹ #5 depth alpha 成立，本"经验零 alpha"结论翻转。
- 若 `‖ΔN‖₁` 退回 0（机制回归）⟹ 回到 b2，机制修复被推翻。当前两品种均 `‖ΔN‖₁≫0`，机制稳定。
- **诚实边界**：`ΔShrp=0.000` 是 `Sharpe(A)−Sharpe(B)` 口径（%.3f 打印，`|Δ|<5e-4`）。codex 已指出 `Sharpe(A)−Sharpe(B)≠Sharpe(A−B)`——ΔR 序列自身的 Sharpe 是另一测量（`l3_delta_r_alpha` 方法论，本工位未跑）。但因**基准口径 ΔShrp=0 且 #5 自身 8/8 (b) 否证**，无论 ΔR 口径如何，本窗内无 alpha 被证实（负结果，不预设盈利）。

### 4. 下游推论

- 642 的 ΔSharpe L2 限定语已闭合：机制级已解决（b2→b1 坐实），净值级 ΔSharpe 经实测≈0（8/8）。裁决⑤ §642 的"限定'机制级'"表述与实测一致，可推进结算流程。
- deltasharpe memory 修正标注（Lead 维护）可写入：`line 24『真定义层冲突/选择类』→『工程 bug/定理类（coverage.rs host key），已修复且 L2 重测坐实机制流动』`；并补一条**新事实**：修复后 `‖ΔN‖₁>0`（净目标头寸确被 depth 层改变），故"depth>0 零贡献"仅在**已实现 NAV** 层成立、在**净目标头寸**层已被证伪——原 memory 的"零贡献"须限定口径。
- 8/8 (b) 否证的有效域不变（642 line 78）：仍是**保守欠对冲版 pi**（ρ漂移 hedge 子腿剪枝 caveat），不否定完整 #5 alpha；但"修复后 ΔSharpe 有望非零"的期望在本 32K 窗内**未兑现**（0.000）。

### 5. 谱系引用

- 642（生成态，本重测的直接上游）：误判降级 + 修复定位。本重测兑现其 line 78 预言"修复后 ΔSharpe 重测（不预设结果）"——结果为 0.000。
- 639（已结算）：σ 来源/持仓准入正交机制分离，本重测印证（depth 腿准入走 §13 AncOK，与 σ 来源分离，机制流动无冲突）。
- 231（formalization-validity-domain）：修复≠盈利，trades≠alpha——本重测 8/8 有 real trades（321–1708）但 0/8 alpha，严格遵守。
- 裁决⑤ §642 + memory `l2-falsify-dual-barrier-not-just-perf` / `newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass`（后者待 Lead 按 §4 修正标注）。

### 6. 影响声明

- **未改任何代码/定义文件**（纯测量，无 git 操作）。
- 新增本报告文件一份。
- 触及 memory `newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass` 的修正标注（口径限定 + 机制流动新事实）——**由 Lead 维护写入**，本工位仅产出标注供 Lead。
- 下游消费者：genealogist（642 结算流程可依本 L2 证据推进）；Lead（memory 修正 + 642 pending→settled 判断）。

---

## 复现命令

```bash
cd rust
# (A) 机制层：depth 腿准入 + ‖ΔN‖₁（真实 CL/BTC）
cargo test --release --lib l3_pi_depth_diag_cl_btc -- --ignored --nocapture
# (B) 净值层：ΔShrp = Sharpe(max_depth=3) − Sharpe(max_depth=1)，8 品种
cargo test --release --lib l3_pi_falsify_multi_symbol_significance -- --ignored --nocapture
```

两测均在当前 HEAD（`gap3-rework-codex9-fix`，机制修复已实装）执行，各约 12–26s（release 已构建）。
