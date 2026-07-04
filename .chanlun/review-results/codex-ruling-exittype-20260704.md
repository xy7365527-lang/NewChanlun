# codex 裁定：ExitType 语义（进桶键 vs 诊断切片）

- **工位**：swarm/ws-exittype | Task #9（goal m1m4-signalfull-oos b1）
- **裁定源**：codex exec（codex-cli 0.142.5，gpt-5.5，`--skip-git-repo-check --sandbox read-only`），仅审计文档/代码未改文件
- **编排者持续授权**（2026-07-04「有问题直接问 codex」「让他给你裁定」）——裁定直接消费为实施依据
- **裁定时刻 HEAD**：`bd01afc1b81b5413f2c95d1352bc6b8cc66d0aa8`
- **认识论**：L0（定义/口径裁定，零 L2 信息增量）。裁定与生产代码现状一致，属定理类（no-patch 严格性）

---

## 矛盾陈述

`docs/formal-chain/路线.pdf`（同日 codex 产出路线图）与 `.chanlun/review-results/final-prereg-20260704.md` §2 对 `ExitType` 维度处置冲突：

- **A 侧（路线.pdf）**：p5 Step 2/Step 4 + p13 第五关公式 `Z_decision = (level, bsp_class, parent_dir, ForceState, ExitType, …)` 把 ExitType 与 ForceState 并列放进主裁决桶公式；p13 底部要求逐笔存档 `entry, exit, X, d, δ, Z_δ-free, ForceState, ExitType`。
- **B 侧（final-prereg §2 exit-μ-BUCKETING-FROZEN，#180 裁定）**：exit_type **禁止**进 μ 桶键（MuClass key）、禁止作 χ_t 门控可查询协变量；**允许**只通过 X^full 进入样本收益值 + 事后诊断切片。依据：出场信息在入场决策点 t 不可观测（post-treatment），进桶键 = 未来信息泄漏 + 样本碎裂（657）。

可测性不对称：ForceState（ex-ante，入场可观测）vs ExitType（post-treatment，出场后才知）。

---

## 唯一裁定：**(甲)**

> **ExitType 语义 = 逐笔账本字段 + 收益计价来源 + 事后诊断切片，不进 accept/reject 主裁决聚合基，也不进 χ_t 门控可查询键。**
> `路线.pdf` 的 `Z_decision = (…, ForceState, ExitType, …)` 应读作路线图层面的混写 / 诊断扩展维简写——其中 ExitType 不能按 μ 桶键解释。p13 逐笔存档清单保留：**存档 ExitType 合法，进裁决桶键不合法**。
> 在现有 μ(z)=E[X|z] 作为入场可交易 alpha 判据的定义下，**(乙) 不成立**。

### 裁定理由（两轴）

1. **因果可测性轴**：live 入场时刻 t，exit_type 尚未发生。若 μ 桶键含 realized ExitType，χ_t(z,a) 必须查询一个 t 时刻不可观测的未来标签桶 → 不是 actionable alpha，是 post-treatment 泄漏。final-prereg §2/§3.1 已写死"exit_type 只作诊断，绝不进裁决聚合基/门控"。
2. **样本量代数轴**：n≈2209 typed-exit，再乘 exit_type 5 层 ⟹ ≈29.5/桶；若走 (entry_z, exit_z, exit_type) 三元组 ⟹ ≈2.0/桶。只能作诊断/异质性阅读，不能作确认层（#180 §3 继承条款）。

### 翻转条件（唯二）

1. realized exit_type 替换为**入场时可预测的 ex-ante exit regime 代理** + 重新 prereg（届时是新入场维）。
2. 整个 Z / χ_t 语义从入场决策点 t 改到出场点 τ（改策略存在论，属定义冲突，须重新 escalate）。

两条均不属本终局重跑域。

---

## 独立复核（引用文件亲验，非仅采信 codex）

| codex 引用 | 亲验结果 |
|-----------|---------|
| `MuObservation = {class, x_gamma}` | ✅ mu_estimator.rs:294-297 确认二字段，无 exit 维 |
| 生产 observe 走 `class: t.entry_z` | ✅ l3_delta_r_alpha.rs:179 `est.observe(MuObservation { class: t.entry_z, x_gamma })` |
| MuClass 桶键无 exit_type/exit_z | ✅ grep exit_type/exit_z 在 mu_estimator.rs 零命中 |
| TypedTrade 携 exit_type（账本层载体） | ✅ runner.rs:1321 `pub exit_type: ExitType` |
| mu_estimator.rs 文档已声明"ExitType 已接（G4）不进 F_t 可测桶键" | ✅ mu_estimator.rs:80 |

**复核结论**：生产代码现状 = (甲)。裁定不引入 μ 桶键改动，是对既有实装的定义层确认。

## 复核判定（否定质询）

- 问题真实存在？是——路线.pdf 公式与 prereg §2 文本层面确有冲突（ExitType 出现在 Z_decision 公式）。
- 已被其他机制覆盖？部分——μ 桶键侧已由代码固化为 (甲)；但**诊断切片侧未落地**：ResidualTrade 未携 exit_type，W-VERIFY 未按 exit_type 拆解占比（grep wverify_run.rs/decontam.rs 零命中）。这是 (甲) 语义要求但尚缺的实装。
- 严重性合理？合理——冲突是文档层"简写 vs 铁律"，非实现错误。裁定关闭文档冲突，同时暴露诊断切片实装缺口。

---

## 结果包（简化版，纯技术性裁定）

- **结论**：ExitType 采 (甲)——账本/计价/诊断切片，不进裁决聚合基与 χ_t 门控。exit-μ-BUCKETING-FROZEN 保持不变。
- **边界条件**：exit_type 替换为 ex-ante 代理 + 新 prereg，或 Z/χ_t 语义移到出场点 τ（须 escalate）——两条翻转。
- **影响声明**：不改代码、不改谱系、不改 prereg 冻结文本。新增本裁定记录。下游子任务 = ExitType 诊断切片实装（ResidualTrade 增 exit_type 列 + W-VERIFY 按 5 变体拆解占比），**不进 μ 桶键**。
