# PR #1676 独审第 1 轮（claude-fable-5-1，经本机 CLIProxyLocal，非作者）

VERDICT: APPROVE

核查结论：PR 只改了 `.chanlun/definitions/sanjieduan.md` 与 `#[cfg(test)]` 的 `tb03c_book_tests.rs`（`mod.rs:25-26` 确认该模块仅测试编译），生产代码零改动。新测试把订正后的空头蕴含锁住了，没有恒真断言或真空分支；`realized_step` 原件与 `book.rs` 的具名实现利润校验逐条对得上；Lean 覆盖面声明核心内容属实。以下按严重度列出我认为值得登记但不阻断的问题。

**HIGH**

无。

**MEDIUM**

1. `.chanlun/definitions/sanjieduan.md:36` ——「`TWState` 不带方向」「没有任何资格谓词」两处措辞比 Lean 实际状况略宽。核对 `formal/Origin/TotalWealth.lean`：
   - `TWState`（:97-109）确实没有 `side` 字段，但 `units : Int`（:105-108，#891 注记）明写「多正空负」，方向是以符号编码进了状态的，只是没有任何定理按它分方向。
   - 整个文件没有 `growth_eligible` 对应物，也没有「资格 ⇒ 阶段」定理，这点属实；但 `LegalEnterEarningStrict`（:665-667，含 `stage = capitalRecovered ∧ withdrawn ≥ i0`）是一个进入 `EarningShares` 的入口合法性谓词，形式上就是多头「本金全退」判据，后来的读者很可能把它当「资格谓词」而判这句声明为假。
   建议把这句收窄为「无 `side` 字段（`units` 符号仅作 #891 注记，未被任何定理用于分方向）；无 `growth_eligible` 对应谓词、无资格⇒latch 蕴含定理；`LegalEnterEarningStrict`（:665）是 `enterEarning` 入口证书而非资格谓词」。不改也不违反验收项 3（票号在、原句未删），所以不阻断。

**LOW**

2. `rust/src/economic_session/tb03c_book_tests.rs:531-532` —— `eligible > 0` 与 `lost_after_latch > 0` 是 12 个种子合计的全局计数，满足票面「至少一个种子触发」。但若日后有人调生成器参数（例如把 `principal_contribution` 分支概率调高），可能只剩个别种子命中而锁依然绿；建议在 panic 信息里顺手打印命中的种子数，方便看出覆盖退化。非阻断。

3. `rust/src/economic_session/tb03c_book_tests.rs:569-571` —— `if gross == fee { gross += 1 }` 的用意是避开 `Pi == 0` 时 `book.rs:327-331` 要求的 `no_other_income_book_changes` 见证（`realized_step` 不带该见证，否则会被拒收）。逻辑正确，但只有一行注释都没有，读者容易误以为是随手规避某个断言；建议补一句说明。非阻断。

**逐项核对记录**

- 锁的实质：空头 `derive`（`tb03c_book.rs:603-609`）只在 `Eligible` 时把 `rank` 设为 1，`r <= 1`（测试 :508）直接挡变异 1；`assert_eq!(status, growth_eligible_of(b))`（:515）加上 `lost_after_latch > 0`（:532）挡变异 2——latch 读入资格后，追加保证金批或亏损回补都无法让资格失效，计数会归零。`lost_after_latch` 只在 `r >= 1` 时累加，而空头 `r >= 1` 只能来自先前的 `Eligible`，所以这个计数确实表示「曾合格、现失效、latch 保留」，不是别的路径凑出来的。
- 真空防护：`books.len() == 61` 每种子都断言，生成器五个分支每步必推一条事件；空头开局必带 M0，`principal` 永远 `Some`，`NotEvaluable` 不会出现，`:525` 的 `NotEligible` 断言不是恒真。
- `realized_step` 与 `book.rs:336-371` 的校验：`net = gross − listed_fees = Pi`；两个 witness 值与声明一致；`known_cost_components` 里只有一条 `commission` 且金额等于 `listed_corresponding_fees`（:354-360）；`fee_scope_complete` 为 true；`source_ref` 均登记。分量只用了 `commission`、`fee_only`、`principal_contribution`，与 `SHORT_KINDS`（`tb03c_book.rs:60`）完全一致；`du ≠ 0` 配 `commission` 通过 `check_quantity_class`（:262-281）。
- 旧测试改动（:472-484）：把夹具里空头的断言从 `latch == CapitalRecovered` 改成 `rank ≥ 1` 并新增空头不进 `EarningShares`，与 ADR 0043 裁定三（`0043:33`）一致。
- 换向不覆盖的声明属实：tb03c `side` 在 `open_book`（:427）冻结，生成器也不换向，归 #1378 的表述合理。
