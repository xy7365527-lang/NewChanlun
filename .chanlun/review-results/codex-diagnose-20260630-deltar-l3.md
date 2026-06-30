# Codex diagnose — delta-r-alpha L3 否定性结论异质审

生成：2026-06-30  模型：gpt-5.5 xhigh  模式：diagnose（read-only）
被审：commit 230691e0bd l3_delta_r_alpha.rs（χ 选择器无系统性 alpha）

## Prompt

# 异质诊断任务：对抗一个"无 alpha"否定性结论

你是异质审查者。下面是一个量化回测得出的**否定性结论**（"χ 选择器无系统性 alpha"）。你的职责是**对抗这个否定性结论**——否定性结论同样可能因方法缺陷（前视泄漏 / 统计功效不足 / 截断偏置）而错。若否证本身有缺陷，则"无 alpha"是**伪否证**，结论应降级为 inconclusive 而非"否证成立"。

请逐条诊断下面 4 个攻击点，对每点给出：**根因判断**（否证稳健 / 否证有缺陷）→ **证据**（引用代码事实）→ **若有缺陷，缺陷如何系统性偏向否定结果**。

---

## 被审结论（commit 230691e0bd）

χ_t = 1[μ(z)>θ] 因果选择器在 8 品种 OOS-split walk-forward（θ=0，每窗截断 32000 bar）上**无系统性 alpha**：
- 2 确认 (c': CL/GC 单品种 ΔR>0 且 p≤.05) / 3 否证 (b: BRN/DX/OKLO) / 3 退化 (d: χ 全滤空仓 BTC/ES/QQQ)
- 进入 L3 符号检验的"有效品种" n_L3=5，其中 ΔR>0 占 3/5，符号检验单边 p=0.5000，池均值 4.33e-6 ≈ 0
- 最终判定："L3 系统性 alpha 不成立"（鞅定理印证：walk-forward μ 未捕获持续可预测性）

## 检验设计（关键代码事实）

**数据切分**（`delta_r_alpha_multi_symbol`，l3_delta_r_alpha.rs:328-357）：
- 每品种取预注册 OOS 时间窗 `oos_full`，截断到 `cut = min(32000, len)` bar
- `split = cut / 2`；`train = bars[0..split]`，`test = bars[split..cut]`（时间上严格不重叠，前半 train 后半 test）
- train 或 test < 252 bar ⟹ inconclusive

**μ 表估计**（`build_walk_forward_mu`，l3_delta_r_alpha.rs:119-214）：
- 只在 train（`train.bars`，长度 n=split）上逐 bar 因果分类（`IncrementalClassifier`，只用 ≤i 数据）
- 对每个**新确认**买卖点（append-only diff vs seen-set）在确认 bar i 取 z（确认时点 F_i 可测）
- 退出兑现规则**事前固定**：持有到下一个反向方向新确认信号的确认 bar；**无反向信号则 censored 截断到 train 末最后可交易 bar**（`(0..n).rev().find(...)`，n=train 长度，**不触及 test**）
- entry_px = 确认 bar close，exit_px = 退出 bar close（F-可测真实兑现，非端点后视）
- μ 表 frozen 后喂 test；test 决策不更新 μ

**test 窗回测对比**（l3_delta_r_alpha.rs:375-385）：
- baseline：`chi_theta=None`（χ≡1 全覆盖）
- χ 过滤：`chi_theta=Some(0)`，`treat_empty_as_pass=false`（空类 μ=None ⟹ 不交易，最诚实）
- ΔR 序列 = nav0·(r^χ_t − r^0_t) 逐 bar 配对差（codex 既往审核 Q2：Sharpe(A)−Sharpe(B)≠Sharpe(A−B)，故用配对差序列非 Sharpe 差）
- ΔR block bootstrap（block=20，1000 重采样，seed 冻结）单边 p（H0: mean≤0）

**L3 有效品种过滤**（l3_delta_r_alpha.rs:416）：
- `l3_valid = st.chi_changed_trades && chi.n_orders>0 && base.n_orders>0`
- 即排除：χ 未改交易集 (c)、χ 全滤空仓退化 (d，chi.n_orders==0 && base.n_orders>0)、双 0 单 (a)
- 退化品种 (d) 的"正 ΔR"被判为"空仓躲亏伪影"而非选择 alpha，故排除出 n_L3

**符号检验**（sign_test_pvalue，l3_delta_r_alpha.rs:281-299）：
- n_pos/n 个品种 ΔR>0 ⟹ P(X≥n_pos | Bin(n,0.5)) 上单边
- 8 品种需 ≥7/8 正才 p<0.05；本次 n_L3=5，n_pos=3 ⟹ p=0.5

---

## 4 个攻击点（请逐条对抗）

### 攻击点 1：walk-forward split 是否真无前视？
train/test 时间切分严格不重叠（bars[0..split] vs bars[split..cut]）。μ 估计的 censored 截断用 `(0..n).rev()` 其中 n=train 长度——是否真的不触及 test？entry/exit 价格用确认 bar 与退出 bar 的 close——是否存在端点后视？test 段的 χ 决策是否只读 frozen μ 表（不回填）？**重点查**：μ 估计中"下一个反向信号"的查找 `signals[idx+1..]` 是否可能越过 train 边界（signals 是否只含 train 内信号）？

### 攻击点 2：否定性结论是否被"退化 (d)"掩盖？
3/8 品种 (BTC/ES/QQQ) 被判 (d) χ 全滤空仓而排除出 n_L3。判据是 `chi.n_orders==0 && base.n_orders>0`。问题：
- 这是"χ 门太严把所有交易滤光（躲亏非选 alpha）"，还是"这些品种 train 期 μ 表全负 ⟹ test 期所有 z 都 μ≤0 ⟹ 正确地不交易"？
- 若是后者，那"全滤空仓"恰恰是 χ **正确识别了无 alpha 的品种**——把它们排除出 n_L3 是否让否证**偏乐观**（把 χ 表现差的品种藏起来）还是**偏悲观**（把 χ 正确避险的样本删掉）？
- θ=0 + treat_empty_as_pass=false 的组合：test 期出现 train 期**未见过的 z 类**（μ=None）⟹ 不交易。若 test 期信号多数是 train 未见的新 z，χ 会大面积空仓——这是 walk-forward 的**样本外覆盖率问题**还是真无 alpha？

### 攻击点 3：符号检验 p=0.5 的统计功效
n_L3=5，n_pos=3，符号检验 p=0.5。问题：
- n=5 的符号检验，即使**真 alpha 存在**，需要几个品种为正才能 p<0.05？（5 个全正 P(X≥5|Bin(5,0.5))=1/32≈0.031；4/5 正 P(X≥4)=6/32≈0.19>0.05）——即 n=5 需 5/5 全正才显著。
- 故 p=0.5（3/5 正）是"确认无 alpha"还是"n=5 样本太小，符号检验 power 不足以否证"（**inconclusive** 而非否证）？
- 这是 alpha2 §11 鞅定理的真否证（无预测性则无 alpha），还是 substrate 饥饿（O(n²) 截断 32000 bar + 退化品种排除把 n 从 8 砍到 5）导致的**功效不足**？

### 攻击点 4：32000 bar 截断的有效域偏置
每窗截断 32000 bar（O(n²) 全窗不可行），再 split 成 train+test 各 16000 bar（1min bar ⟹ 约 11 个交易日 train + 11 日 test）。已知事实：同项目 level-amplitude-gate 研究证明 **alpha 集中在高级别**（recL2/recL3 涌现层）。问题：
- 16000 bar（约 11 交易日）的 train 窗，高级别（recL2/recL3）能涌现多少个买卖点？高级别 buildup 需要长样本——短窗是否**结构性地杀掉了高级别 z 类的样本**（μ 表里高级别 z 计数极少或为空）？
- 若 alpha 存在于高级别但短窗 train 估不出高级别 μ，则 test 期高级别信号全是 μ=None（空类）⟹ treat_empty_as_pass=false ⟹ 不交易 ⟹ 整个 alpha 来源被静默丢弃。这是否让否证**系统性偏向"无 alpha"**（不是 χ 选不出 alpha，是短窗根本没让 alpha 所在的级别有样本）？

---

## 产出要求

对 4 攻击点逐条裁定，最后给总判定二选一：
- **(A) 否证稳健**：4 攻击点均不构成方法缺陷，"无系统性 alpha"是有效否证（缩小有效域）
- **(B) 否证有缺陷**：≥1 攻击点构成系统性偏置，"无 alpha"是伪否证，结论应降级为 **inconclusive**（有效域 = 32000bar 短窗 + n_L3=5 功效不足，不能外推到"χ 选择器无 alpha"）

请标注每条裁定的认识论等级（L0 纯逻辑 / L1 管线 / L2 需真实数据 / L3 交叉验证）。简洁，聚焦缺陷是否**系统性偏向否定方向**。

## Codex Response

我会先做一个很轻量的记忆和代码入口确认，然后只围绕这 4 个攻击点查证。这里是诊断任务，不会改文件。

记忆里没有直接命中这次 L3/χ 回测条目，所以我会把当前工作区代码作为主证据。已加载 codebase-memory-mcp，接下来优先用它定位 `l3_delta_r_alpha.rs` 和相关函数。

`codebase-memory-mcp` 的项目列表调用被取消了；我按指令降级到本地只读查证。先定位仓库和目标文件，再抽取关键行号。

当前 `/private/tmp` 下没有直接命中 `l3_delta_r_alpha.rs` 文件名，只有几个 Rust 项目入口。我会扩大到内容搜索，确认这份代码是否在临时目录里或需要以 commit 文本事实为依据。

`/private/tmp` 里有 `nc-bisect` 工作树和若干审计输出；目标文件没有直接出现在当前文件列表，可能只存在于指定 commit。下一步我用 git 对 `230691e0bd` 做只读抽取。

git 能读到目标 commit，文件在 `rust/src/theta_v0/backtest/l3_delta_r_alpha.rs`。我现在抽取该 commit 的实现行，重点看 μ 表构建、split、test 回测、L3 过滤和符号检验。

核心代码已经确认：train/test 分割和 μ 的 `signals` 都局限在 `train.bars`。对攻击点 2 和 4，还需要看实际输出有没有记录空类覆盖率或级别分布；没有的话只能判为需数据验证，而不能把退化解释成真实否证。

目前代码没有记录 BTC/ES/QQQ 的 `μ=None` vs `μ≤0` 分解，也没有记录 test z 覆盖率或 recL2/recL3 级别计数。这本身是攻击点 2/4 的核心证据：它无法把“正确不交易”和“样本外覆盖饥饿”区分开。

附带看到了工作树有未跟踪 `analysis/data_cache`，我没有动它。现在只剩下把证据压成裁定：前视问题偏干净，退化/功效/截断是会把结论推向假阴性的地方。

`codex-deltar-err.txt` 主要是先前诊断会话的 stderr/transcript，不是干净的实验日志；我不会把它当作独立数据证据。实际可用证据以 commit 中的代码和你给出的汇总结果为准。

我会在最终里区分两类结论：代码级前视审计是 L1 稳健；统计功效、退化根因和短窗覆盖率是 L2/L3 证据不足，且偏向假阴性。

我会把 “退化品种排除” 判为缺陷，不是因为排除一定错，而是因为代码没有记录足以判定它错或对的分解：`μ=None`、`μ≤0`、level 分布和 test 覆盖率都缺失。这个缺口使 “无 alpha” 不能从剩余 5 个样本外推。

总判定：**(B) 否证有缺陷**。不是前视泄漏导致，而是退化根因未分解、`n_L3=5` 功效不足、32K 短窗对高级别 alpha 的覆盖偏置，使“χ 无系统性 alpha”应降级为 **inconclusive**。

1. **攻击点 1：walk-forward 前视**
根因判断：**否证稳健**。认识论等级：**L1 管线**。

证据：`build_walk_forward_mu(train, ...)` 只取 `train.bars`，`signals` 只在 `for i in 0..n` 内收集；`signals[idx+1..]` 因此只能扫 train 内信号。无反向信号时，`exit_bar` 用 `(0..n).rev()` 截到 train 末，不可能跨入 test；entry/exit 都取确认 bar/退出 bar close。见 [l3_delta_r_alpha.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:119) 和 [l3_delta_r_alpha.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:189)。test 只调用 frozen `est`，baseline/χ 都在 `test` 上跑，见 [l3_delta_r_alpha.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:374)。

偏向：这里没有看到系统性偏向否定的前视缺陷。

2. **攻击点 2：退化 (d) 是否掩盖**
根因判断：**否证有缺陷**。认识论等级：**L2 需真实数据**。

证据：退化只由 `chi.n_orders == 0 && base.n_orders > 0` 判定，并直接标为“全滤=躲亏非选”，再从 L3 排除，见 [l3_delta_r_alpha.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:388) 和 [l3_delta_r_alpha.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:414)。但 χ 对空类的语义是 `None => treat_empty_as_pass`；本测试传 `false`，所以 train 未见的 test z 会被拒绝，见 [selector.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/selector.rs:45) 和 [selector.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/selector.rs:147)。`MuEstimator::mu` 明确区分 `None` 与 `μ=0`，见 [mu_estimator.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/mu_estimator.rs:165)。日志只打印 `est.n_classes()`、订单数和归因，不打印 `μ=None / μ≤0 / μ>0` 覆盖率，见 [l3_delta_r_alpha.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:419)。

偏向：若 BTC/ES/QQQ 是 `μ≤0` 的正确全拒，排除它们会删掉 χ 正确避险样本，偏悲观；若是 `μ=None` 覆盖饥饿，则它们不是 alpha 否证，而是样本外覆盖失败。两种情况下都不能把剩余 5 个样本的失败外推成“χ 无系统性 alpha”。

3. **攻击点 3：符号检验功效**
根因判断：**否证有缺陷**。认识论等级：**L0 统计逻辑 / L1 实装**。

证据：符号检验实现的是 `P(X≥n_pos | Bin(n,.5))`，见 [l3_delta_r_alpha.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:277)。`n=5` 时只有 `5/5` 才到 `1/32≈0.031<0.05`；`4/5` 仍是 `6/32≈0.1875`。代码最终只要求 `n_l3 >= 2`，然后 `sign_p` 不显著就打印“χ 选择器无系统性 alpha”，见 [l3_delta_r_alpha.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:473)。

偏向：这把“没有足够功效拒绝零假设”升级成“确认没有 alpha”。在 `n=5` 下，除非全胜，否则几乎必然不显著，系统性偏向否定结论。

4. **攻击点 4：32000 bar 截断**
根因判断：**否证有缺陷**。认识论等级：**L2/L3 需真实数据交叉验证**。

证据：代码明确 `MAX_BARS=32_000`，split 后 train/test 各 16K，这是工程可行域而非理论有效域，见 [l3_delta_r_alpha.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:90)。同项目资料显示 alpha/幅度集中在高级别：`recL2+` 幅度高于摩擦、低级别多为噪声，且高级别 candidate 稀缺，见 [all_memory_constraints.md](/private/tmp/nc-bisect/docs/all_memory_constraints.md:675)；另有“只交易高级别大幅正但极稀有”的记录，见 [why_not_profitable.md](/private/tmp/nc-bisect/analysis/why_not_profitable.md:128)。

偏向：短 train 窗若估不出高级别 z，test 高级别信号会落入 `μ=None`，在 `treat_empty_as_pass=false` 下静默不交易。这样不是 χ 选不出 alpha，而是 alpha 所在层级没有进入 μ 表，系统性推向“无 alpha”。

**最终裁定**
前视审计干净，但否证不稳健。当前结论只能写成：**在 32000bar 短窗、`treat_empty_as_pass=false`、退化样本未分解、有效 L3 样本仅 5 的条件下，未观察到显著系统性 alpha**。不能写成“χ 选择器无系统性 alpha”。