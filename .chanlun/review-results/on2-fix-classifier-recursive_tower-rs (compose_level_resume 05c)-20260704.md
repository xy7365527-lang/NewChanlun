# H9 compose_resume 05c 热点复核（recursive_tower.rs，20260704）

## 结论：NO-SHIP（两处根因修复已落地，残余为收益递减区）

H9 清单的两个具名根因修复**已在 HEAD 祖先 commit 4c23604f4c 落地**（详见
`05c-compose-alloc-20260702.md`），本次复核确认工作树中 `recursive_tower.rs`
与 HEAD 逐字节相同（`git diff HEAD --quiet` CLEAN），无新增改动，不 commit。

- **Fix A（切片消灭临时数组 clone）**：`compose_level_resume` line 502
  `let subs = &subs_moves[win.0..=win.1];`——已是切片借用，无 `[a.clone(),b.clone(),c.clone()]`。
- **Fix B（`RMove::Compose.subs`→`Rc<Vec<RMove>>`）**：`descend.rs:74` 确认为
  `subs: Rc<Vec<RMove>>`，`m.rmove.clone()`（05c2a）对 Compose 变体退化为 O(1) 引用计数。

## 计时（修前=修后，无本次改动）

400K BTC（本域代表窗，本次实测）：

| 阶段 | 耗时 |
|------|------|
| 墙钟（400K bar 逐 bar classify_with_tower_incremental） | 6.14 s |
| 05_compose_resume | 758.8 ms |
| 05c_tail_upper_build | 508.3 ms |
| 05c2_compose_call | 416.2 ms |
| 05c2b_submoves_alloc | 141.4 ms |
| 05c2a_rmove_clone | 107.7 ms |
| 05_span avg（续扫跨度） | 6.72（有界，非 O(n²)） |

1M（Fix A+B 落地后，取自 `05c-compose-alloc-20260702.md` 计时表）：
05_compose_resume = 2193.7 ms（<2.5s，符合 H9 清单"绝对值 1M<2.5s"判定）。

残余 05c2a（107ms@400K）+ 05c2b（141ms@400K）= 每窗小块堆分配，O(n·k)、k 有界
（05_span avg 6.72→7.73，SPAN 不随 n 线性增长）。非 O(n²) 巨头，非阻塞主线。

## 守卫（全绿）

- `cargo test --release --lib theta_v0::classifier::`：252 passed / 0 failed / 6 ignored
  （含 `compose_level_resume_matches_full_compose`、`incremental_tower_*`、各 `*_bit_exact`）。
- `cargo test --release --lib backtest::incremental`：4 passed / 0 failed
  （`bit_exact_synthetic`、`bit_exact_confirmed_len_open_tail` 等；`bit_exact_per_bar` 为
  CL 数据门控 ignored）。

## 边界条件（NO-SHIP 翻转条件）

进一步压缩 05c2a/05c2b 需把 `sub_moves` 从 `Rc<Vec<LeveledMove>>`（每窗独立分配）
改为对上级 `subs_moves` 的**切片视图**，消除 `subs.to_vec()`。该改动触及
`descend_leveled`（返回 `Rc<Vec<LeveledMove>>`）+ 全部 `sub_moves` 消费点
（`voice_eat`/`cand_predicate`/`coverage`/`bsp` 等），且窗口在 seal 前可变（frontier
truncate/resume 语义）⟹ 切片视图与"前缀不修改、尾部 append"契约冲突，bit-exact
风险高、收益 <2.5s@1M。当且仅当 05c 在未来标度测得 exp>1 逼近 O(n²)（05_span avg
随 n 线性增长）时，才值得承担此结构重构——当前 SPAN 有界，判 NO-SHIP。

## 阻塞披露（他域，非本工位）

1M 全量 profile 未能完成：并发 agent 在 `classifier/mod.rs:1406/1412` 的 Rc 化改动
（`lc.projected_units` 改 `Rc<Vec<UnitRange>>`）留下 `debug_assert` 类型不匹配
（`Rc<Vec<_>>` vs `Vec<_>`）导致 lib 编译失败。该文件不在本工位域（本域仅
`recursive_tower.rs`），本工位从未 Edit/Write 触碰；待对方稳定后 1M 计时可复测
（1M 数字已有祖先 commit 落地记录，见上表）。
