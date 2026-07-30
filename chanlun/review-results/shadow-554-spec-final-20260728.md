# #554 影子评审 · Spec 轴终审（2026-07-28，新上下文核验）

- 评审面：`git diff 7b4547b623...HEAD`（9 commits）；只读，未改代码
- 自跑：`cargo test --release --lib` = **2030 passed / 1 failed / 135 ignored**，与报告一致；唯一红为既有 `signal::tests::extract_signals_bit_exact_digest_guard`，本轮 `signal.rs` diff=0（`git diff --stat` 空）

## 结论：PASS

## 逐项核验

① **HIGH-1 已实修** — `mod.rs:1166-1189` `clear()` 字段清单不含 `candidate_book`（文档 `:1162-1165` 明述「保留不可回填的候选因果事件簿」）。保簿测试 `mod.rs:2784-2810` 为真验证：clear 后 `advance` 只带 retained ⟹ removed 经 `invalidate_unseen`（`cand_event.rs:169-189`）追 `Invalidated`/`revision=1`，retained `revision=1` 且 `observed_at=50`（首见钟不塌缩）；若簿被清，三条断言均会失败。US6 不复活由 `cand_event.rs:196-201`/`:181` 双侧 terminal 短路保。

② **窗口投影文档已落** — `mod.rs:620-623` 明写 fresh-full = 「每 key 一条终态」的终态窗口投影、因果簿正本归增量宿主、等价锁归 #551。

③ **Pan `confirmed_at` 已因果化** — `cand_event.rs:279-290` 首确认取入簿 `as_of`、revision 继承首确认钟；测试 `cand_event.rs:663-675` 以 Pan 几何位 35 / `as_of=42` 锁 `Some(42)`。

④ **`extreme_proof` 注记已补** — `cand_event.rs:88-89`、`:122-123` 登记「恒为 `key.seg_a` 副本，无独立业务信息」，报告 `issue550-t1-impl-20260728.md:137` 同步。

⑤ **快照 commit 节部分闭合** — 已补 `7a9faeb2d0`（`:150`），但 `:151` 仍写「commit id 由编排 session 补注」，终轮 `ef585d21a6` 未登记。残 **LOW-1**。

⑥ **无新规格偏离** — `candidate_scan_inputs`（`mod.rs:263-279`）行为等价（`unit_to_segment` 保 `direction`，`mod.rs:255`）；`advance` 拆分 `continue`→`return None` 语义等价；FNV 翻转 `3608191067574153658` 归因诚实（digest 哈希 `{streams:?}` 全字段，`mod.rs:4234-4240`）。

**LOW-2（新登记）** — 空 L0 早退 `mod.rs:1691-1693` 在 `clear()` 后即 return，绕过 `:2419` 的 `advance`，该 bar 消失候选不当场追 `Invalidated`（下一个非空 bar 补）。非无痕消失，不破 US5/US6，仅失效延迟。
