# #641 Spec 轴终态复审（新上下文，禁自评）

diff = `039cf86974...0de84f4b09`。规格源 = #641 票体 + 四裁定（碰撞处置 / 线况 / 地板条款 comment-5121572134 / 取舍裁定 comment-5121793896）+ #636 Resolution。全量 `cargo test --release chain*` 绿（chain_cert 15 测 + 主缝 2 测）。

## 抽验通过项（回代码，非采信自述）

- **地板合取**：`mod.rs:606` `head_confirmed && !extendable && has_segment`，`has_segment` 由 `mod.rs:593` 真取 `edges.iter().any(is_segment)`；链头独活 ⟹ `edges` 空 ⟹ 落 `Open`（`mod.rs:609`）。测试已按裁定翻转并改名 `head_only_survivor_stays_open_by_floor_conjunct`（`tests.rs:650`），断言 `status == Open` + `closed_at == None` + `probe.floor_blocked > 0`。监视格 `closed_with_zero_segments`（`mod.rs:923`）在主缝与 bin 双面锁 0。
- **extends 查簿命中才写**：`resolve_extends`（`mod.rs:824-832`）对每个真前缀查 `self.latest`，未命中即 `None`；正/负两侧各一测（`tests.rs:492` / `tests.rs:536`），主缝断言 `extends_not_materialized > 0` 且 heads 全 `None`（`classifier/mod.rs` golden 测）。幽灵前缀 12/12 → 0 已锁。
- **B 择优三件**：死因分档恰两支 `HeadInvalidated / PredicateFailed`（`mod.rs:119-130`），连坐支缺席有反事实锁（`tests.rs:370`）；`summarize()`/`digest()` 入库，主缝 golden、`issue550_event_battery`、单测三处同源（`mod.rs:903/970`）；事实边指名见证 `PredicateBreach`（`mod.rs:227`）含谓词名+两端点+两区间+失败合取项，`tests.rs:240-254` 逐格锁。
- 护栏：`classifier/mod.rs` 既有行零改，仅加只读口 `candidate_streams()`；p123 侧信道 env-gated、独立 sink、无第二读点。

## (a) 规格要求但缺失

1. **Acceptance 4 后半「CONTEXT.md 词表收编」未落地**。`CONTEXT.md` 有「证书（Certificate）」「链（Chain）」条目，diff 未触碰该文件，全仓 grep 无 `TowerChainCertificate` / `chain_cert`。实施报告 §287、§413 照实登记为「待编排侧」，但本 diff 是 main 终态，缺件仍在。

## (c) 看似实装但错了的

2. **`chain_certificate_book_incremental_equals_full_replay`（`classifier/mod.rs`）是自比，不是双路径**。循环内 `replay` 与 `prefix_incremental` 都是 `chain_book_over_prefixes(&segments[..n], &closes, &cfg, fresh TowerCache)` —— 同函数、同参数、同 fresh cache 的两次调用，恒等，只锁确定性。真正的增量对象 `incremental`（全长 + 复用 cache）从未进入任何 `assert_eq`，只被 `!certificates().is_empty()` 用一次。Acceptance 1「全量/增量双路径产出逐字节一致」因此**未被锁住**；把 `incremental` 整行删掉测试照样绿。
3. **链头 `Absent`（查无）与 `Invalidated` 同判死，超出裁定授权**。#636 裁定②原文：「`Invalidated` = 谓词判不过**或**链头 `Invalidated`」。`mod.rs:596` 判据是 `!head_alive`，`ChainNodeStatus::Absent` 一并落入 `HeadInvalidated`（`mod.rs:122-126` 自承「本档覆盖 Falsified 与 Absent 两种」）。`Absent` 在终态窗口投影驱动下可达（模块自称支持该驱动，`tests.rs:395` 即用该口径造出 `Absent`），且判死为终态不复活 —— 一次窗口投影抖动即永久判死一条未被任何谓词否证的链。
4. **`floor_blocked` 探针分辨力不纯**（次要）。`mod.rs:615` 条件缺 `all_segments` 合取：`edges` 非空且全为事实边时 `has_segment == false`，链实际被 `PredicateFailed` 判死，探针仍记一次「地板拦下」。当前实测该格为 0 故无污染，但它作为「地板真被走到」的机器载体不排他。

## (b) scope creep

未发现。`chain_every` CLI 参数、`ChainBookSummary` 分桶、`chain_probe` 均在 Acceptance 3 读数/非真空锁要求内；floor 完整口径、`selection_policy`、44 课证据路径均照裁定留 fog。

## 结论

**FAIL** —— Acceptance 1 的双路径一致未被任何测试锁住（`rust/src/theta_v0/classifier/mod.rs` 的 `chain_certificate_book_incremental_equals_full_replay`，自比恒真）；Acceptance 4 词表收编缺件（`CONTEXT.md`）。另需裁定链头 `Absent` 是否判死（`rust/src/theta_v0/classifier/chain_cert/mod.rs:596`）。地板合取、extends 查簿、B 择优三件三项修复轮全部落实，PASS。
