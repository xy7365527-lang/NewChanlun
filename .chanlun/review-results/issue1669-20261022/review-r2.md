# PR #1676 复审（claude-opus-5-5，非作者）

VERDICT: APPROVE

无阻断问题。三条意见都已处理到位，返修没有引入新问题。

**MEDIUM 1（覆盖面措辞过宽）：已处理，措辞和 Lean 源一致**
- 「`TWState` 没有 `side` 字段，`units` 按 #891 注记『多正空负』」：与 `formal/Origin/TotalWealth.lean:97-108` 一致。结构里没有 `side`，`units : Int` 的注释写的正是「多正空负」。
- 「没有定理按它分方向」：我在 TotalWealth.lean 里查了 `units`，只出现在结构字面量里（`:178`、`:295`、`:394`、`:553/555`、`:573`、`:597`），确实没有哪条定理按 `units` 的正负分方向。
- 「没有 `growth_eligible` 的对应谓词」：全 `formal/` 搜不到 `growth_eligible` 或 `growthEligible`，成立。
- 「`LegalEnterEarningStrict`（`:665`）不是扩量资格谓词」：结论成立，而且 Lean 本身就能佐证。这个谓词的第一个合取项是 `s.stage = TStage.capitalRecovered`，也就是把阶段（latch）当成了输入。sanjieduan.md:30 明确规定「latch 不得作为资格输入」，所以它不可能是 `growth_eligible` 的对应物。

**LOW 2（只打印全局合计）：已处理**
每个种子开始前记下两个计数器的值，跑完比较是否增加，命中就把对应的种子数加一。逻辑正确，失败信息会带上「命中种子 x/12」。

**LOW 3（gross == fee 缺注释）：已处理，注释属实**
注释说 Pi 为 0 时需要「无其他利润变化」见证。对照 `book.rs:323-335`：当 Pi、A、W 全为 0 时，强制要求 `no_other_income_book_changes_in_this_allocation` 见证；这个见证一旦和非零分量同时出现，就报 `IdentityConflict`。注释描述的约束和代码一致。

**非阻断小建议（可选，不必为此再返修）**
1. `:665` 那句话可以补上上面那条理由，例如「其合取项含 `stage = capitalRecovered`，以 latch 为输入，故不能充当资格谓词」。现在只给了结论，读者得自己去 Lean 里找原因。
2. Lean 注释（`:658-663`）里，「入口证书」指的是 `LegalEnterEarning`，`LegalEnterEarningStrict` 是它的加强版，叫「严格 EnterReady」。返修文本把 Strict 直接叫「入口证书」，稍微混了两者的称呼，但不影响结论。
3. 「事件史单调性」这个说法沿用了上一版。严格讲，`stage_rank_monotone`（`:194`）和 `earning_no_regress`（`:212`）都是单步定理，并且排除了 `clearCampaign`。这不是本轮引入的问题，记一笔即可。
4. 注释里「与具名实现见证互斥」这一点，我只核实了「Pi=0 时强制要求无变化见证」这一半。另一半我没在 `book.rs` 里找到同等明确的拒绝分支，也可能藏在 `declared_realized_profit` 的后续检查里。这一点不影响测试本身的正确性。

按要求，我没有跑 cargo。fmt 和测试 8/0 的结果来自你给的远端信息。
