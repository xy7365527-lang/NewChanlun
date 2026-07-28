# #550 实装 Standards 轴评审（fixed point 7b4547b623 → HEAD aa7acd22c0）

标准源：`.claude/rules/common/*.md`（8 份全读）+ Fowler 12 条臭味基线。rustfmt/clippy/编译器已强制的项不计。
**结论：FAIL**（2 条硬违反 + 1 条测试标准缺口；无正确性缺陷，正确性属 Spec 轴）。

## 硬违反（文档标准可判）

**HIGH — coding-style.md「Files are focused (<800 lines)」/「200-400 lines typical, 800 max」— `rust/src/theta_v0/classifier/mod.rs`（4534 行，+798）、`rust/src/theta_v0/backtest/incremental.rs`（2160 行，+301）**
超限是基线既有（3736/1859），但本 diff 把新特性全量塞回同一文件而非留在新建的 `cand_event.rs`（528 行，合规），偏离进一步扩大。

**HIGH — coding-style.md「Functions are small (<50 lines)」— mod.rs:1632『classify_with_tower_incremental』(806 行)、mod.rs:362『classify_impl』(207 行)**
本 diff 分别向其内联 +24 / +30 行候选逻辑（mod.rs:2173-2194、455-476），未抽函数。

**MED — testing.md「Test Types (ALL required): Unit / Integration / E2E」— `rust/src/bin/issue550_event_battery.rs`**
真实数据逐 bar 对拍只以手工 bin 存在（commit message 标 `test(classifier)`，但无 `#[test]`，不进 CI）。塔内自动化只有 mod.rs:3992『fresh 单发』相等断言；bin 实测的「逐 bar 增量≡全量」无自动化覆盖。

**MED — coding-style.md「No hardcoded values (use constants or config)」— cand_event.rs:334-335、350、486**
FNV 种子 `0xcbf29ce484222325` / `0x84222325cbf29ce4` / `0x100000001b3` 三处字面量散落；`structural_predicates` 三字段恒写字面 `true`。

**LOW — coding-style.md「Handle errors explicitly at every level」— cand_event.rs:308-312**
生产路径 3 处 `.expect(...)`（A 段/λ_C/方向）在不变量破裂时 panic。与本文件基线一致，故仅记录。

*非违反（明确登记，避免下游重复上浮）*：`CandidateEventBook` 原地推进受 coding-style.md「补（2026-07-28）：带修订留痕的单线程状态机/账本」豁免；bin 的 OHLC 边界校验满足 Input Validation；security.md 全项无命中。

## 基线臭味（判断题）

**HIGH — Speculative Generality — cand_event.rs:38-124、88、228**
生产侧只产出 `kind=Trend` / `state=Provisional` / `third_class_proof=None` / 谓词恒 `(true,true,true)`；`CandidateKind::Pan`、`CandidateState::{Unresolved,Confirmed}`、`make_revision` 的 `confirmed_at` 分支、`pub fn candidate_is_sub`（零调用者，含测试）全部只活在测试与类型声明里。模块声明的状态机远宽于它产出的。

**MED — Duplicated Code — mod.rs:455-476 ≡ mod.rs:2173-2194（22 行逐字重复）、mod.rs:561 ≡ 2402（as_of 同式）；incremental.rs:97-112 重抄 `classify_at_with_l0` 的 append+断言而非委托**

**MED — Data Clumps — mod.rs:608/2440、incremental.rs:97、streaming.rs:92**
`(Classification, Vec<Rc<Vec<LeveledMove>>>, CandidateStreams)` 三元组在 4 处公开签名重复出现，未命名成结构体；`(hist, dif, closes_tick, close_src, gauge)` 同行旅行迫使 cand_event.rs:262 挂 `#[allow(clippy::too_many_arguments)]`。

**MED — Mysterious Name — cand_event.rs:334-335、345**
`candidate_group_id` / `pair_id` 是同一 key 的两次不同种子哈希，与 key 双射 ⇒ 不承载任何「分组/配对」信息；`first_provable_at`、`revision_at`、`interval.1` 三名同值（均 `seg.end_index`）。

**MED — Middle Man — mod.rs:2440『classify_with_tower_events_incremental』**
纯转委托 + `cache.candidate_book.streams().to_vec()`；该 `to_vec()` 每 bar 深拷贝全量事件簿。

**LOW — Primitive Obsession — cand_event.rs:38、25-26**
`side_tag: u8`（手写 Long→0/Short→1）绕过既有 `types::Side`——受阻于 `Side` 未 derive `Ord/Hash`，修法是补一行 derive；`ParentFingerprint.zd/zg: i64` 丢弃了 `Tick` 别名。

**LOW — Divergent Change（提交卫生）— commit 2c2214d30c**
该 feat commit 的 mod.rs 改动 1462 行中，特性相关约 120 行，其余为全文件 rustfmt 重排（含 `pub mod` 声明重排序）。git-workflow.md 的 type 分类（feat/chore）本意单一目的；混批使特性 diff 在评审中不可读。

**LOW — 测试脆性 — cand_event.rs:479-490**
golden 摘要取 `format!("{:?}", streams)` 的 FNV——任何字段增删或 Debug 派生变动都会翻转，与结构语义不绑定。
