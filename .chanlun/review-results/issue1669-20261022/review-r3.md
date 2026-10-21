# PR #1676 终审（claude-opus-5-5，非作者，纯措辞返修）

VERDICT: APPROVE

措辞与代码一致。Lean 那句：`TotalWealth.lean:656-667` 的注释原话就是"严格 EnterReady"，也说了它是入口证书 `LegalEnterEarning` 的加强（`:673` 的 `enterReadyStrict_implies_legalEnterEarning` 证明了这一点）。它的第一个合取项正是 `s.stage = TStage.capitalRecovered`，而本节第 30 行写明"latch 仍不得作为资格输入"，所以"不是资格谓词"现在有了站得住的理由。Rust 那句：`book.rs:323-335` 只在 Pi、A、W 三项全为 0 时才另外要求"无其他利润变化"的见证；`realized_step`（`tb03c_book_tests.rs:787-808`）把 A、W 固定写成 "0"，也不带这份见证，所以 `gross == fee` 时会因为缺见证被拒。新注释说的"另要求…realized_step 不带它"是准确的，旧注释"互斥"的错误说法已经去掉，避开 Pi=0 这个点的理由也随之成立。
