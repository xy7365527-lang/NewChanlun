# #550 实装收尾 Spec 轴评审（2026-07-28）

- 评审面：`git diff 7b4547b623...HEAD`（3 commits，6 文件，+2375/-464）
- 规格源：SPEC #547 全文、#550 票体验收标准、票内三评注（R1 返工点 / 底座更正 / 裁定(i)）
- 性质：只读评审，未改任何代码文件

## 结论：FAIL

对象骨架、通道透传、身份键口径、`nest` 零改动四项成立；但 (a) 规格要求的事件载荷与状态机在生产路径上未实装、(b) 主缝三件套第②③件形同空转、(c) 两处实装与 #535 因果红线及「零生产行为变更」相抵。

## 发现

**HIGH-1 生产事件只有一种形状** — `cand_event.rs:336-345`：`kind` 恒 `Trend`、`state` 恒 `Provisional`、`structural_predicates` 恒 `{true,true,true}`、`third_class_proof` 恒 `None`、`extreme_proof` 只是 `seg_a` 的副本。`Pan`/`Unresolved`/`Confirmed` 在生产路径不可达（仅单测构造）。
> SPEC #547:46-56「kind: Trend | Pan…structural_predicates[], extreme_proof, third_class_proof?」；US6「事件状态机 ∅→Provisional/Unresolved/Confirmed」。

**HIGH-2 终态几何配首见钟** — `cand_event.rs:228-234`：`confirmed_at = observation.first_provable_at`，确认发生在更晚的 `as_of` 却盖首见钟；`cand_event.rs:461-463` 自证（`as_of=40` 而 `confirmed_at=35`）。报告 line 27-28 把这条当作「诚实更新 golden」的理由。
> SPEC #547:65「因果红线（#535）列为验收约束：禁终态几何配首见钟」。

**HIGH-3 revision 门被换成区间单调增长** — `cand_event.rs:149-152`：`observation.interval.1 <= prior.interval.1` 即 `continue`。后果：投影（state/谓词/center_ids）变了但 C 右端不动 → 静默丢 revision；C 段回缩 → 既不追加修订也不 Invalidate，陈旧几何长期挂 active。
> SPEC #547:59「同状态仅业务载荷投影变化才追加 revision，投影相同零输出」；#547:65「禁丢 Invalidated 路径」。

**HIGH-4 生产热路径每 bar 全量重扫 + 全簿深拷贝** — `mod.rs:2168+`：`structural_observations_for_level` 在 `classify_with_tower_incremental` 每级无条件执行 `decompose(centers)` + 全 segments `judge_first_cached`，无 resume/memo，与本文件 frontier-resume 纪律相反；`streaming.rs:84,109`：既有 `append_bar` 现改道 `append_bar_events`，每 bar `streams().to_vec()` 深拷贝 append-only 全史后丢弃。二者叠加把 #345 修掉的 O(n²) 重新引入生产入口。
> SPEC #547:28-29 US12/13「零生产行为变更…既有 Classification/塔快照/BSP/订单流逐字节不动」；#547:61「消费方零接线」。

**MED-5 主缝②与非真空锁缺席** — `mod.rs:4059` 的 `candidate_event_stream_fresh_incremental_equals_full` 是单次 fresh-cache 调用（不是逐 bar），且不断言事件流非空——流全空亦绿。裁定(i) 只豁免了 fresh-full ≡ 因果簿终态投影这两把等价锁，没有豁免逐 bar 锁与非真空锁。
> SPEC #547:74「新事件流增量≡全量逐 bar `assert_eq!`」；#547:77「覆盖非真空锁：oracle_probe 式计数…防空转绿」。

**MED-6 FNV golden 锁错了对象** — `cand_event.rs:479-491` 摘要的是手搓 `CandidateObservation` 喂出的 book 的 `Debug` 串，不是 `classify` 产出的事件流；`structural_observations_for_level` 的任何产出漂移都不会让它变红。
> SPEC #547:30 US14「事件流有 FNV golden 摘要锁，以便任何产出漂移当场变红」。

**MED-7 US17 未做，换成规格外的 bin，且计数口径失真** — diff 未触 `p123_fast_replay.rs`；改为新增 `rust/src/bin/issue550_event_battery.rs`（规格未要求 = scope creep）。该 bin `:125-133` 把 `streams.len()`（= 级别数）当身份流数、每级只看 `last()` 计 active，故报告 line 46/53「3 条身份流…active 2」不成立。
> SPEC #547:33 US17「p123 dump 侧信道扩一路事件 dump（env-gated、只写不判）…双跑 diff=0 + 双侧 SHA 封印」。

**MED-8 验收簿记与分支状态不符** — 报告 line 11-12/101/94-96 声称 commit 因沙箱权限 BLOCKED-ENV，而分支实有三枚 commit（`2c2214d30c`/`b562404307`/`aa7acd22c0`），报告本身即由第三枚提交入库。「快照 commit」这项因此悬空。
> #550 验收标准「计数纪律 = passed/failed/ignored 三元组 + 快照 commit」。

**LOW-9 全文 rustfmt 重排掩盖改动面** — `mod.rs` diff 2240 行中绝大部分是格式化，含 `pub mod` 声明整体重排（`mod.rs:65-95`），实质改动面被淹没。
> SPEC #547:42「既有文件改动面 = `classify_impl` 产出点接线 + 输出通道透传 + `TowerCache` 增量态扩展」。

**LOW-10 五钟塌成四钟、钟域混用** — `cand_event.rs:210-213`：`observed_at` 恒等于 `first_provable_at`，二者无独立语义；`revision_at` 正常修订取 `interval.1`（结构坐标）、失效分支取 `as_of`（bar 坐标）。
> SPEC #547:54「observed_at, first_provable_at, confirmed_at?, invalidated_at?，五钟，一次写入不后移」。

## 未列为发现（对照后成立）

身份键（`c` 右端与 `as_of` 不入键，`cand_event.rs:31-40`）；判据单源（唯一调 `signal::judge_first_cached`，力度只进 BSP bit）；`interval_is_sub` 闭区间含端点、不复读 `nest::is_sub`；`nest` 三件 / `CandDeltaEvent` / `NestCandidateEvent` / typed 链 diff = 0；既有 `classify_with_tower`/`append_bar`/`classify_at` 签名不变、消费方零接线。
