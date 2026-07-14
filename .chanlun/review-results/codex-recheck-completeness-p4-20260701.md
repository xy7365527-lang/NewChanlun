# Codex 真异质复核：完整实装最小充分集 + P4 类型透传（2026-07-01）

## 元数据

- **任务**：对先前两个同质降级诊断（Codex CLI 二进制丢失时 Claude 代理质询产出）进行真异质复核
- **negation_source**：**真 Codex CLI**（OpenAI gpt-5.5，0.142.5），真异质
- **session id**：019f1eea-3281-76e0-a418-b304151b635a
- **认识论等级**：L0/L1（代码结构事实，trust-but-verify）
- **tokens 消耗**：290,001

---

## 复核 D：完整实装最小充分集

### 原同质诊断结论

最小充分集 = P1（区间套接线缺口）+ P4（类型透传）+ P7（正规出场接线缺口）。

### 真 Codex 裁决

**D-1 三贯通判据**：**CONFIRMED**

> "完整实装"应要求：知识库/规格定义存在、Rust 有实现、目标回测链实际消费。只存在字段或模块但不被目标链调用，不能算完整。

**D-2 P1 是接线缺口**：**REJECTED（按当前代码）**

> 当前 `econ_positive.rs:231` 已用 `build_multilevel_nest_cert(...)` 作为信号准入门；该函数构造 `NestCertificate` 并在 `:426` 调 `cert.n_delta()`。所以"对 `NestCertificate` / `n_delta()` 零调用"已不是当前事实。
>
> 但这不等于 P1 已数学完美：`Candidate.nest_confirmed` 仍只是单级 `nest_confirm(...)` 字段，见 `interp.rs:799`；当前真正准入走的是 econ 自己的 `build_multilevel_nest_cert`。

代码验证路径：`rust/src/theta_v0/backtest/econ_positive.rs:231`

**D-3 P7 是接线缺口**：**CONFIRMED**

> `closed_loop/sell.rs` 有卖侧正规闭环：`SellDecision`、`recog_chanlun_sell`、`sell_transition`，见 `sell.rs:42`。但 econ 诊断链的退出仍是"下一反向确认信号"配对，见 `econ_positive.rs:240`。未见 econ 调 `closed_loop::sell`。

**D-4 最小充分集 P1+P4+P7**：**PARTIALLY-CONFIRMED**

> 如果针对旧状态，P1+P4+P7 是合理候选；但按当前代码，P1 不能再列为"零接线缺口"。当前最小未闭合重点应改成 **P4 + P7**，外加对 P1 新接线的语义正确性复核。P2/P3 不应硬塞进这个单标的 L0/L1 最小集：P2 是 MACD 力度口径选择，P3 是跨标的等价关系维度。

### 同质诊断被真 Codex 推翻的部分

| 条目 | 同质结论 | 真 Codex 裁决 | 推翻/确认 |
|------|---------|-------------|---------|
| 三贯通判据 | 正确 | CONFIRMED | 确认 |
| P1 零调用 | 是 | **REJECTED** | **推翻**：`:231` 有真实调用 |
| P7 接线缺口 | 是 | CONFIRMED | 确认 |
| 最小充分集 P1+P4+P7 | 成立 | PARTIALLY-CONFIRMED → **更新为 P4+P7** | 部分推翻 |

**P1 推翻理由的精确性**：同质诊断基于旧时间点代码快照，当时确实零调用。真 Codex 读当前代码，`:231` 的 `build_multilevel_nest_cert` 接入已完成。但 P1 接入的**语义正确性**（是否完整多级递归）仍待核验——`interp.rs:799` 的 `Candidate.nest_confirmed` 仍是单级版本。

---

## 复核 E：P4 类型信息透传缺口

### 原同质诊断结论

真实缺口：μ̂ 分桶键 `(level, δ)` 不含 bsp_class（econ_positive.rs:779/:781），buy1/buy2/buy3 混一桶，单类型 α 不可辨。最小改动：分桶键从 `(u32, i8)` 加 bsp_class 到 `(u32, i8, u8)`。

### 真 Codex 裁决

**E-1 分桶键不含 bsp_class**：**CONFIRMED**

> `SignalDecomp.bsp_class` 存在，见 `:83`。但分桶键是 `BTreeMap<(u32, i8), Bucket>`，入口是 `(d.level, d.delta)`，见 `:779`。

**E-2 混合池稀释是 L0 结构必然**：**CONFIRMED**

> 同一 `(level, δ)` 桶会混入不同 `bsp_class` 位掩码；桶均值只能给混合均值，不能辨别单类型 alpha。无需 L2 数据即可成立。

**E-3 加 bsp_class 入键是否充分**：**PARTIALLY-CONFIRMED**

> 原则上充分：类键应是 `(level, delta, bsp_class)`。但实际最小改动不能只改主报告的 `buckets`，还要同步 `class_actual_pnl(...)` 和 `train_winner_class(...)` 这类 OOS/helper 路径，它们也只按 `(level, delta)`，见 `:1010` 和 `:1406`。

**E-4 是否已有旁路让 per-class alpha 可辨**：**REJECTED**

> CSV 已输出 `bsp_class`，见 `:964`，但报告和测试判据没有按它聚合。另一个修正：`Candidate.bsp_class` 不是全局孤儿，它进入解释器排序键，见 `interp.rs:841`；但它是有损 `min_class`，且不是 econ 的 per-class alpha 分桶旁路。

### 真 Codex 对同质诊断的修正

| 条目 | 同质结论 | 真 Codex 裁决 | 推翻/确认 |
|------|---------|-------------|---------|
| 分桶键不含 bsp_class | 是 | CONFIRMED | 确认 |
| 混合池稀释 L0 必然 | 是 | CONFIRMED | 确认 |
| 最小改动=加 bsp_class | 主体正确 | PARTIALLY-CONFIRMED | 部分补充：还需改 `:1010` 和 `:1406` |
| Candidate.bsp_class 是孤儿字段 | 是 | **修正**：进入 interp 排序键，非完全孤儿 | 修正（不推翻核心判定） |

---

## 综合裁决摘要

| 复核问题 | 同质诊断是否被推翻 |
|---------|-----------------|
| **D 最小充分集**：P1 是接线缺口 | **部分推翻**：P1 已有 `:231` 接入，不再是零调用缺口；最小充分集应更新为 **P4 + P7**（P1 改为语义正确性待核验） |
| **D 最小充分集**：P7 是接线缺口 | **确认**：sell.rs 已实装，econ 不走该路径 |
| **E P4 真缺口**：分桶键稀释 | **确认**：L0 结构必然成立 |
| **E P4 最小改动**：仅改主 buckets | **部分补充**：还需同步 `:1010`/`:1406` 的 OOS/helper 路径 |

---

## 简化结果包（纯技术性产出）

**结论**：真 Codex 推翻了同质诊断对 P1 状态的描述——P1 已有真实调用（`:231`），不是零调用接线缺口。最小充分集更新为 **P4 + P7**，外加 P1 语义正确性（多级递归完整性）待复核。E（P4）诊断主体确认，但最小改动范围需扩展到三处：`:779` 主 buckets + `:1010` class_actual_pnl + `:1406` train_winner_class。

**边界条件**：
- P1 REJECTED 的前提是当前代码 `:231` 存在，若未来 P1 被禁用/条件变更则可能重新成为缺口
- P4 PARTIALLY-CONFIRMED 的翻转条件：`:1010`/`:1406` 若已有 bsp_class 维度则诊断收窄

**影响声明**：本产出为诊断（L0/L1），未改生产代码。影响：最小充分集从 P1+P4+P7 更新为 P4+P7（P1 接入状态已改变）；P4 修复范围从单点改为三点。
