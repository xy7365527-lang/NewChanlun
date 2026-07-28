# #554 影子评审 · Standards 轴终局门（7b4547b623…HEAD，7 commits）

标准源：`.claude/rules/common/*.md` 全部 + Fowler 12 条。独立复核，未沿用前两轮判词。

**结论：FAIL**（3 条可判违反；无正确性缺陷）。

## 前轮项抽验（实测，非采信报告）

已真修：FNV 三魔数收命名常量（`cand_event.rs:17-19`）；`streams()` 改 `Rc::clone`（`:151`）、`append_bar` 经 `append_parse` 直调无事件入口（`streaming.rs:114`），全簿深拷贝已灭；`same_projection` 改 `CandidateProjection` derive 比较（`:278-308`），手写 9 字段比较消失；`Side` 直入键（`types.rs:262` 补 `Ord/Hash`），`Side→u8` 两处逐字重复消失；Pan 的 `center_ids`/`previous_center_start` 改 `None`（`:388/425`），占位值冒充语义已除。`cargo test --release --lib cand_event` 实跑 8 passed。

## 仍存违反

**HIGH — coding-style.md「Functions are small (<50 lines)」**
本 diff 新写、非继承的超标函数 4 个：`cand_event.rs:317 structural_observations_for_level`（90）、`:155 advance`（60）、`bin/issue550_event_battery.rs:92 run`（74）、`:39 load`（52）。两轮未动。

**HIGH — Fowler Dead Code**
`cand_event.rs:111 candidate_is_sub` 全仓零调用者（含测试，实测 grep 为空）；`:106 interval_is_sub` 仅服务该死函数与自身真值表测试。`no-patch-mentality.md`「无用代码直接删除」。

**MED — testing.md「Test Types (ALL required)」**
真实数据逐 bar 对拍只以手工 bin 存在（`issue550_event_battery.rs`，无 `#[test]`，不入 CI）；合成侧逐段重放锁已补（`mod.rs:4058`）。

## 基线臭味（判断题）

**MED — Duplicated Code（概念级，新引入）** — `cand_event.rs` 的 `CandidateKey/CandidateState/is_terminal/CandidateEventBook`（append-only 修订簿）与既有 `nest_lifecycle.rs:105/170/183/631` 的 `LifecycleKey/NestEventState/is_terminal/NestLifecycleBook` 是同一状态机的第二份实现。#535「nest 三件 diff=0」禁的是改 nest，不使这份平行实现不存在——两簿今后同步演化即 Shotgun Surgery。

**MED — Duplicated Code** — `mod.rs:493-502 ≡ 2289-2298`，`candidate_segments/anchors` 构造 10 行逐字两份，两轮未消。

**LOW — 冗余排序** `cand_event.rs:404 → 480`（内层排序被外层整体重排覆盖）；**LOW — 生产路径 `.expect`** `:368-370` 三处；**LOW — Mysterious Name** `candidate_group_id`/`pair_id` 名不副实（已加诚实注释，名未改）；**LOW — 每 bar 全量克隆** `mod.rs:2314`。

## 体量增量方向（按指令只评方向）

`classifier/mod.rs` 生产段 +226 行（4698 总）、`backtest/incremental.rs` +301 行但其中绝大多数为既有代码的 rustfmt 重排，语义净增约 30 行——该文件不构成本 diff 的体量恶化。新模块 `cand_event.rs` 677 行合规。方向仍为向已超标的 `mod.rs` 追加，未改善。

**翻 PASS 条件**：拆 4 个新超标函数 + 删两个死函数（或给出生产调用点）。
