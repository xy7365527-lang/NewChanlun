# #550 实装 Standards 轴复审（fixed point 7b4547b623 → HEAD 39ce41b896，5 commits）

标准源：`.claude/rules/common/*.md`（8 份全读）+ Fowler 12 条臭味基线。rustfmt/clippy/编译器已强制的项不计。
本轮 = 首轮 FAIL 后的复审（修复 commit `ad19cb9499`「整合修 13 条」）。

**结论：FAIL**（2 条硬违反未修且偏离扩大 + 4 条新引入项；无正确性缺陷，正确性属 Spec 轴）。

## 首轮项已修（登记，不再重复上浮）

FNV 三处魔数收 `FNV_OFFSET_BASIS`/`FNV_PRIME`/`PAIR_ID_SEED` 命名常量（cand_event.rs:17-19）；FNV golden 改锁 `classify_with_tower_events` 真实产出（mod.rs:4152）；全量/增量候选逻辑归并到 `observations_for_level`（mod.rs:503 ≡ 2299 共用）；`Pan` 域生产化（cand_event.rs:385）解除该项 Speculative Generality；`streams()` 改 `Rc::clone` + `append_bar` 恢复直调（streaming.rs:83）灭全簿深拷贝；恒真谓词加诚实注释并把中间形态推给 #551（cand_event.rs:63-66）；修复 commit 本身零格式混批（281 行全为语义改动），Divergent Change 提交卫生项已改善（历史 commit 2c2214d30c 不可追改）。

## 硬违反（文档标准可判）

**HIGH — coding-style.md「Files are focused (<800 lines)」/「200-400 lines typical, 800 max」**
`classifier/mod.rs` 4697 行（基线 3736，本 diff +961）、`backtest/incremental.rs` 2160 行（基线 1859）。首轮已判，本轮修复又向 mod.rs 加 279 行，偏离继续扩大。新建 `cand_event.rs` 656 行合规。

**HIGH — coding-style.md「Functions are small (<50 lines)」**
未修：`mod.rs:1637 classify_with_tower_incremental`（812 行）、`mod.rs:362 classify_impl`（209 行）。
新引入：`cand_event.rs:290 structural_observations_for_level`（93 行）、`cand_event.rs:149 advance`（60 行）、`bin/issue550_event_battery.rs:92 run`（74 行）、`:39 load`（52 行）。

**MED — testing.md「Test Types (ALL required): Unit / Integration / E2E」**
部分修复：合成侧已补逐段重放锁 `mod.rs:4057`（实测 `cargo test --release` 0.49s 通过）+ 非空/Pan 断言。仍缺：真实数据逐 bar 对拍只以手工 bin 存在（`issue550_event_battery.rs`，无 `#[test]`，不进 CI）。

**LOW — coding-style.md「Handle errors explicitly at every level」**
`cand_event.rs:341-343` 三处生产路径 `.expect(...)`（A 段/λ_C/方向）在不变量破裂时 panic。与本文件基线一致，仅记录。

## 基线臭味（判断题）

**MED — Shotgun Surgery（新引入）— cand_event.rs:76 / 112 / 243 / 272 + 生产点 361、410**
加一个业务字段要同步改 5 处；`same_projection` 手写 9 字段比较，漏一项即静默丢 revision（当前 9 项无遗漏）。

**MED — Duplicated Code（新引入 + 残留）— cand_event.rs:343-346 ≡ 392-395（`Side`→u8 映射逐字两份）；mod.rs:493-502 ≡ 2289-2298（segments/anchors 构造 10 行逐字重复，首轮 22 行已缩短未消除）**

**MED — Mysterious Name（未修）— cand_event.rs:365-366、464**
`candidate_group_id` / `pair_id` 是同一 `key` 的两次不同种子哈希，与 key 双射 ⇒ 不承载任何「分组/配对」信息。

**MED — 字段语义失真（新引入）— cand_event.rs:405、413**
Pan 域无前中枢，`previous_center_start` 与 `center_ids` 双端一律填 `cert.center.start_index`，占位值冒充语义字段（身份键因此对 Pan 少一维）。

**MED — Speculative Generality（降级留存）— cand_event.rs:52、107、102**
`CandidateState::Unresolved` 生产不可达（仅 `:611` 一测试构造）；`candidate_is_sub` 全仓零调用者（含测试）；`interval_is_sub` 只服务该死函数与自身真值表测试。

**LOW — Primitive Obsession — cand_event.rs:41、32-33**
`side_tag: u8` 绕过既有 `types::Side`（阻力仅是 `Side` 缺 `Ord/Hash` derive）；`ParentFingerprint.zd/zg: i64` 丢弃 `Tick` 别名。

**LOW — Middle Man — mod.rs:2451、backtest/incremental.rs `classify_at_events`**
纯转委托 + `streams()`；成本已降为 O(1)，仅形状残留。

**LOW — 冗余排序 — cand_event.rs:380 → 462**
`structural_observations_for_level` 内部排序后，`observations_for_level` 立即整体重排，前者对唯一调用方无效。

**LOW — 每 bar 全量克隆 — mod.rs:2314**
`cached_candidate_observations.iter().cloned()` 每 bar 每级复制全部观察（首轮 O(n²) 主项已由 `bsp_key` 缓存消除，此为残留常数项）。
