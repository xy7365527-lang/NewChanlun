# #554 影子评审 · Standards 轴终审（7b4547b623…HEAD，9 commits）

**结论：PASS**（两条阻断项已消；余 1 条新 MED + 若干 LOW，均不阻断）。

## 逐项核验（实测，非采信 commit message）

**① 4 个新超标函数 — 已拆。** `cand_event.rs` 与 `bin/issue550_event_battery.rs` 全文件无 >50 行函数（含多行签名重测）：`structural_observations_for_level` 337-368（32）、`advance` 152-167（16）、`run` 100-120（21）、`load` 40-43（4）；新析出 `structural_observation`/`trend_observation`/`compare_streams`/`print_summary`/`load_raw`/`raw_to_bars`/`raw_bar` 全部合规。

**② `candidate_is_sub` — 已删**（`ef585d21a6`，全仓 grep 空）。`interval_is_sub`（`cand_event.rs:107`）保留，零生产调用者，仅自身真值表测试；因其为 `pub mod` 导出的闭区间原语且有直测，降为 LOW，不再阻断。

**③ mod.rs 十行重复 — 已抽。** `candidate_scan_inputs`（`mod.rs:263-278`），两调用点 `:511`/`:2302` 各 2 行。

**④ 前轮项无回潮。** FNV 常量 `:17-19`、`streams` `Rc::clone` `:148`、`same_projection` derive `:300-330`、`Side` 直入键、Pan `None` `:44`/`:120`、`append_bar` 无事件入口 `streaming.rs:100-115`。生产段无新增超标函数（mod.rs/incremental.rs 全部 >50 项均在 `#[cfg(test)]` 后：`mod.rs:2700`/`incremental.rs:176`）。

## 新引入（MED）

`cand_event.rs:417-418` — 拆分时把 `.expect("judge Some => A 段映射成立")`/`.expect("judge Some => lambda_C 成立")` 改为 `a?`/`c_start?`：不变式违反由显式 panic 变为静默丢候选，且同表达式第三个不变式仍 `.expect`（`:419`），处理不一致。违 coding-style「Never silently swallow errors」。

## 未消（不阻断）

LOW 冗余排序 `:366` 被 `:539` 覆盖；LOW `clear()` 删除 `candidate_book` 重置未留注释（`mod.rs:1166-1187`，语义仅由测试名 `:2785` 承载）；MED `cand_event` 与 `nest_lifecycle` 概念级双簿；MED 真实数据对拍仅 bin 无 CI；LOW `candidate_group_id`/`pair_id` 名不副实。

## 体量方向

`classify_impl` 141→201、`classify_with_tower_incremental` 732→804（终轮各 −8）。向已远超标函数净加 132 行，方向仍为负。
