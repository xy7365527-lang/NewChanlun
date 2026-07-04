# codex #179 终裁：XZD C2/C3 level 分级（C2-only 维持 vs 全 level C3 硬门）

- **任务**：Task #179（ws-xzdgrade）
- **日期**：2026-07-04
- **异质源**：OpenAI Codex（gpt-5.5，read-only sandbox）
- **裁定域**：选择类（编排者授权「有问题问 codex」）
- **上游**：#170（xzd-c3-review，报数+提裁，本工位倾向维持现状）
- **原始交互落痕**：`.chanlun/review-results/codex-review-*.md`（CLI 自动持久化）；本次直接调 `codex exec`（wrapper 600s 超时对本 prompt 不足），完整 prompt 见 `/tmp/codex-xzd-prompt.txt`，裁定原文见下文 §二。

---

## 一、裁定问题

#148 中枢升级语义落地后 XZD C3 脱 0（BTC 全历史 `c3_new_center_exists` 0→13、`breakout_ok` 0→3）。
`gate_pass()`（`econ_positive.rs:1135-1137`）中 `level>=2` 是否应从现行 **C2-only** 翻转为 **C3 硬门**（并入硬合取）？

```rust
pub(super) fn gate_pass(&self) -> bool {
    self.type2_confirmed && (self.level != 1 || self.c3_new_center_breakout_ok)
}
```

---

## 二、Codex 裁定（原文）

**裁定：条件翻转，当前维持现状。**

不把 level>=2 的 C3 并入 `gate_pass` 硬合取。当前语义应保持 `self.type2_confirmed && (self.level != 1 || self.c3_new_center_breakout_ok)`。

**理由**：
1. 第43课支持「C3 判据可定义于全 level」，但不自动推出「C3 必须在全 level 成为消费侧硬门」。**硬门是有效域裁定，不是字面定义域裁定**。PDF 没有给 XZD 内部 C2/C3 的 level 分级口径，所以 burden of proof 在翻转方。
2. level>=2 当前数据 `breakout_ok=1/217=0.46%`，翻转会把吞吐从 217 压到 1。这个信号不能解释成「高级别 XZD 本应极严」而直接落代码。更合理的裁定：C3 在 level>=2 可诊断、可继续观测，但**硬门有效域未证**。要说「严格是正确的」，必须证明那 1 个 survivor 是正确 XZD、被拒的 216 个是伪 XZD 或无效交易；现在没有这个证据。
3. level==1 保持 C2∧C3，是因为 #44 有第43课「新中枢+反向突破」的明确承接裁定，并且已脱 0。level>=2 只有脱 0 但近退化，没有等大经验支撑。

**翻转条件（同时满足才翻）**：
1. **脱退化**：预注册 OOS / 多窗口 / 多标的下，level>=2 的 `breakout_ok` 有可判别样本量（例如 `n>=10`），且命中率与 level==1 同量级，不能继续是 0.x% 单点残留。
2. **C3 有判别力**：`C2∧C3` 的语义审计精度或 OOS 指标显著优于 `C2∧¬C3`，且被硬门拒掉的大块 C2-only 样本不承载正向结果。
3. **排除观测口径问题**：按 level=2/3/4+、方向、δ 分桶复测，确认不是有限塔截断、as-of 区间、次级中枢确认时点或 breakout 口径导致的 under-observation。

不到这些条件，C3 对 level>=2 只保留为诊断字段；代码不翻。注释「结构性死门」应改为「level>=2 硬门有效域未证 / 当前近退化」，不再沿用过强表述。

---

## 三、独立复核（非照单全收）

对 codex 裁定执行简化质询——三问，均通过：

1. **裁定是否基于误读上下文？** 否。codex 正确识别了核心张力：第43课 C3 语义级别无关（定义域）vs level>=2 命中率 0.46%（有效域）。裁定精确落在本仓库 `formalization-validity-domain.md` 规则上——「形式化工具的有效域可能严格小于定义域」，且「声明有效域等于定义域必须有等大验证」。level==1 有 C3 脱 0 的 L2 实证（有效域已证），level>=2 无（仅 1/217 单点，L2 否证边界内）。**burden of proof 在翻转方**的裁定与规则一致。

2. **裁定是否已被其他机制覆盖？** 否。#170 已报数+提裁但明确不自决（选择类）；codex 是唯一有权在此裁定的异质源。裁定新增了 #170 未给出的**明确翻转条件三条**（可判别样本量阈值 + 判别力 OOS + 观测口径排除），把「维持」从「当前倾向」升级为「带明确重裁触发器的终裁」——这是信息增量。

3. **裁定与本工位倾向的差异？** codex 与 #170 倾向（维持现状）一致，但**否定了 #170 的部分归因**：#170 把维持理由归为「命中率极低≈#41 死门原意」，codex 明确拒绝把低命中率解读为「判据在高级别不适用」，改判为「有效域未证」（更弱、更严谨的表述）——这是异质源的独立贡献，避免了「低命中率=判据无效」的过强推论。本工位接受此订正。

**复核结论**：裁定成立。维持现状（level>=2 C2-only），gate_pass 逻辑 0 行改动。codex 建议的注释表述修正（「死门/结构性死门」→「有效域未证/近退化」）已落地（§四）。

---

## 四、代码改动（注释订正，逻辑 0 行）

裁定=维持 ⟹ `gate_pass()` 逻辑**零改动**。仅两处过期/过强注释按 codex 建议订正（no-patch-mentality 诚实性——裁定已终结，「待裁」表述过期；「死门」表述过强）：

1. `econ_positive.rs:1125-1128`（`c3_new_center_breakout_ok` 字段注释）：level>=2 C2-only 从「本次翻案不改 lvl>=2」补为「codex #179 终裁：有效域未证，非判据不适用，翻转条件三条」。
2. `econ_positive.rs:4225-4228`（死门断言前注释）：从「★下游口径待裁（上浮 Lead）」改为「★下游口径已裁（codex #179 终裁，条件翻转/当前维持现状）」+ 翻转三条件。

编译验证：`cargo check --lib` 通过（仅既存 27 warning，0 error）。

---

## 五、结果包（简化版——纯技术性裁定产出）

1. **结论**：codex #179 终裁「条件翻转，当前维持现状」——level>=2 保持 C2-only，gate_pass 逻辑 0 行改动，注释订正 2 处。独立复核通过（裁定落在 formalization-validity-domain 规则上，且否定了 #170「低命中=判据不适用」的过强归因，改判「有效域未证」）。
2. **边界条件（=翻转条件）**：三条同时满足则重裁翻转为「全 level C3 硬门」——① level>=2 `breakout_ok` 脱退化（预注册 OOS/多窗/多标的下 n>=10 且命中率与 level==1 同量级）；② C3 对 level>=2 判别力 OOS 显著（C2∧C3 优于 C2∧¬C3 且被拒 C2-only 大块不承载正向）；③ 排除观测口径伪影（level/方向/δ 分桶复测排除有限塔截断/as-of/确认时点/breakout 口径 under-observation）。届时改 `gate_pass()` 的 `self.level != 1` 单一条件即可。
3. **影响声明**：`gate_pass()` 逻辑零改动（level 分级唯一决策点，三消费点自动跟随）；`econ_positive.rs` 注释订正 2 处（字段注释 1125-1128 + 死门断言注释 4225-4228）；`cargo check --lib` 通过；新增终裁报告 1 份。裁定终结 #179，销 #170 上浮项。
