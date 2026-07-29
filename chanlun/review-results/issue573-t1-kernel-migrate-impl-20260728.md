# #573 T1 migrate 车收口报告（claude opus 实施车二）

> 日期：2026-07-28；性质：实施车二 stdout 收口报告原文（全链完成：commit 1 expand + commit 2 migrate）
> 原始日志：/tmp/issue573-t1-migrate-dispatch-20260728.log

全链完成。收口如下。

## 1. 两 commit

| # | SHA | 标题 | 规模 |
|---|---|---|---|
| 1 | `94acacd4d2` | `refactor(theta): #573 T1 expand——账本对象无关泛型内核新增（nest 未重基）` | 3 文件 +1259（`ledger_kernel/mod.rs` 579、`tests.rs` 677、`classifier/mod.rs` +3 登记） |
| 2 | `d40607d805` | `refactor(theta): #573 T1 migrate——nest_lifecycle 重基于泛型内核（行为 bit-exact）` | 1 文件 +268 −183 |

两 commit 合计只碰 4 个文件，全在本票面内；11 个并行线未提交面（roster、7 份 review-results、`p100_cert_bsp_recon.rs` 等）逐一核过，一个都没被吞。

## 2. 测试指纹

| 时点 | lib | 全量 `--no-fail-fast` |
|---|---|---|
| 阶段 0 重取基线 | **2075 / 1 / 137** | **2137 / 1 / 146** |
| commit 2 收口 | **2075 / 1 / 137** | **2137 / 1 / 146** |

逐数相同。唯一红 = `extract_signals_bit_exact_digest_guard`（#491 在案恒红，与本票无关），无其它红。测试区一个字节未动（diff 最后一个 hunk 落在 1554 行，测试模块从 ~2317 行起）。lib 警告 37 条不变，`nest_lifecycle`/`ledger_kernel` 零命中。

## 3. p123/p92 对拍证据

**diff 计数 = 0**（8 个面逐字节 `cmp` 全同，非只比 SHA）：p123 20k/100k × {stdout, `P421_LIFECYCLE_DUMP`, `P116_DUMP`} + p92 20k/100k stdout。另跑两条独立通道：`issue533_p123_byte_guardrail` 仓内门（三窗 × 三产物面）2/2 绿；p409 探针 `P409_INVARIANTS ok`、`verify_mismatches=0`。release 二进制 mtime 21:49:59 晚于源改动 21:47:01，确为真编译而非缓存假绿。

## 4. §5 方案对 #619 的修正点

1. **「基线已过期」不成立**——重取的 8 个面 SHA 与第一车 #619 前基线**逐位相同**，#619 确为行为 bit-exact。第一车的过期判断是保守推定，实测证伪。
2. **`migrate_entry` 已由 #619 收敛为单一写入点**，接缝比原方案更干净：它现在只做「来源码 → 修订词汇」的域映射，搬运整体交给 `ledger.migrate`（两者次序逐步同构：退表 → 留痕 → 改键 → 追加 → 入表）。
3. **`center_upgrade_match` 的双返回值 `(Option<Key>, bool)`** 是 #619 新增，原方案未覆盖：迁移臂走 `migrate_entry`，留痕臂走 `open_on_observation` + 手工补 `CenterUpgraded` 修订，两臂分别落位。
4. **`settlement_stats` 的 `claimed` 集合**（#619 H1）改读 `self.ledger.values()`，口径逻辑一字未动。
5. **`vanish_cause` 投影点**从 `invalidate` 内移到 `write_settlement`（内核落账的域侧写入部），「单一写入点 ⟹ 两字段不可能不一致」的结构性保证不变。
6. `NestLifecycleEntry` 的 #619 新字段全部并入 `open` 初值与访问器面，公共字段面一个 bit 未动。

## 5. 两个存疑项的处置

- **存疑①（内核新不变量在真实 replay 上未验）→ 关闭，无需降级。** 「留档时序非降」与「首条修订为建项词汇」经 `assert_invariants` 首行接入后，在 p123 release 全程显式核验（生产接线每 trigger 调一次）下于 BTC 20k/100k 无一违反，p409 独立通道同样 ok。两条约束保留在内核，nest 行为零改动。
- **存疑②（`NestEventState` 变体 doc 归属）→ 无损处置。** 三个变体的缠论出处（061:26 活假设、024:24 完成时复核、061:26/061:28/E2E §1:81 三类失效）逐条转写进类型别名的 doc，域词汇的文档归属仍在 nest 侧，只是从变体级挪到类型级。

## 6. 遗留 / 偏离 / 存疑

- **偏离（开工 HEAD）**：票面写 `6381debba9`，实际开工 HEAD 是其后的 `886a439c78`（#603/#619 关票链 docs commit，只动 `chanlun/` 三个文件，未触 `rust/`）。已核实对本票代码面零影响，按既定事实继续，未停手。
- **文件行数 5326 → 5411（+85，+1.6%）**：`LedgerEntryCore` 访问器样板与三处别名 doc 是净增项；逻辑本体反降（`advance` 247 → 213 行）。超 50 行函数 28 个 → 28 个，零新增，我写的全部新函数均 ≤50 行。
- **修订计数溢出处置变了**：`self.revision += 1` → 内核「留档长度现算 + `u32::try_from().expect()`」，从「debug panic / release 静默 wrap」变成两 profile 一律 fail-loud。纪律加强，非行为改动（`revisions.len()` 触不到 u32 上限）。已写进 commit message。
- **留给影子评审的一处内核瑕疵**：`ledger_kernel/mod.rs:455` 的 `expect("建仓即有一条建项修订：{key:?}")` 用了 `{key:?}` 但 `expect` 不做格式化插值，panic 时会打出字面量而非键值。属 commit 1 既有工件，不在 migrate 的 diff 面内；按票面「禁顺手 refactor、diff 面最小化」未动，在此登记。
- **`assert_invariants` 159 行（重基前 155）**：+4 行为内核调用与其口径注释，既有域断言一条未删未改。它与 `advance` 同属既有超限函数，本票未拆分（票面明示存量不强制）。
