# #603 档2 回退交付（编排者 2026-07-28 裁定，comment-5111127257）

> 角色：实装执行层（全程前台单线程，未派 Task/子代理/后台任务）
> 基线：`kimi-nest-mainline-20260717` @ `fa912ba991`（#603 两档同交基线）
> 票据：#603（编排者 2026-07-28 裁定：档2 回退 + 档1 保留 + 两真因开票）
> 产物：`/tmp/wt603rv-post-{2000,20000,100000}.{stdout,dump,stderr}`
> 探针（仓外、不提交）：`/tmp/wt603rv_cov.py`

## 0. 结论摘要

档2（`ActiveFrontierQueue` pending 槽队列，depth=3）已手术摘除，档1（`bridge_by_center_upgrade`
暂认中枢严格同锚回溯认领）一个 bit 未动。摘除后对 `6ebe18eda1`（#613 收口基线）的净增量仅
382 行（`nest_lifecycle.rs`）+ 3 行（`p123_fast_replay.rs` 的 `force_overtake_claimed` 字段），
逐字对拍确认档1 行为与 #603 原交付逐位一致。

## 1. 摘除清单

### `rust/src/theta_v0/classifier/nest_lifecycle.rs`

- `ACTIVE_FRONTIER_QUEUE_DEPTH` 常量（含选择依据文档）——整段删除。
- `ActiveFrontierQueue` 结构体 + `impl`（`with_depth`/`observe`/`candidates`/`current`/
  `is_current_at`）——整块删除。
- 测试 `issue603_frontier_queue_depth_behaviour` 及其唯一使用者 `frontier_at` 辅助函数——删除。
- 档1 相关代码**逐位保留**：`bridge_by_center_upgrade`、`LifecycleRevisionKind::CenterUpgraded`、
  `superseded_from` 文档更新（同时描述 `Supersedes`/`CenterUpgraded` 两码，未改）、
  `force_overtake_claimed_count` 字段与结算逻辑、`advance()` 内 `match (claim, migratable)` 分支、
  `assert_invariants` 的双码互斥断言、`center_upgrade_match`、3 个档1 测试
  （`issue603_center_upgrade_claims_provisional_predecessor_by_migration` /
  `issue603_center_upgrade_leaves_terminal_predecessor_untouched` /
  `issue603_center_upgrade_rejects_cross_anchor_and_backward_center`）。

### `rust/src/bin/p123_fast_replay.rs`

档2 在本文件的改动面远大于 `nest_lifecycle.rs`（队列接线、`CandidateScan`/`CandidateWindow`/
`LiveScanSink` 借用体、`scan_l1_queue_candidates`/`scan_runs_for_candidate`、`frozen_end`/
`cand_start`/`cand_age` 诊断字段、`stale_candidate_redivided` 守卫、`queue=` dump 字段）——
与档1 在本文件的唯一交集只有 `P421_LIFETIME_SUMMARY` 那一行 `eprintln!` 里新增的
`force_overtake_claimed=` 字段。故本文件整体回退到 `6ebe18eda1`（#613 收口基线）版本，再手工
补回该一行字段——净增量 3 行（1 行格式串改动 + 1 行新增参数 + diff 上下文位移）。`git diff
6ebe18eda1 -- rust/src/bin/p123_fast_replay.rs` 自证：唯一差异即此。

### golden fixture（`#533` 纪律，dump 面重锚，stdout 行不动）

- `issue533_p123_2000_dump.golden.txt`：档2 摘除后与 `6ebe18eda1` 基线**逐字节相同**
  （3 处 `CenterUpgraded` 认领全部发生于 as_of=15984/66981/71448，均超出 2000-bar 窗口）——
  直接取 `6ebe18eda1` 版本重锚，`diff` 确认为空。
  - **该行为反证了 #603 原报告 §5 的说法**：原报告称"lifecycle dump 是本票本体，不作 cmp=0"
    并对 2000-bar 全文 golden 重锚——但档1 单独对 2000-bar 窗口零影响，唯一真正驱动 2000-bar
    dump 变化的是档2 的队列诊断格式（`cand_age=`/`queue=` 等字段）。本票摘除档2 后，
    2000-bar dump 具体验证了这一点：不是"档1+档2 共同改了 2000-bar dump"，而是"档2 单独改了
    2000-bar dump，档1 对该窗口零触达"。
- `issue533_p123_20000.sha256` 的 `dump` 行：`e8cb7dec68c5f1c311544295c2b8c8cc3a13e4566935965284c0015a577e7f53`
  （档1 单独在 20k 窗口内的 1 处认领 as_of=15984 造成，与 `6ebe18eda1` 基线的
  `3f2758b6c3f36036a830893bf2c0ec40beca8fba575910659f51fd8a8302b28f` 不同——**已审阅、故意**，
  归因见 §3）。
- `issue533_p123_100000.sha256` 的 `dump` 行：`fee254bef600bf2067735332de1cb93115ef934f643861a33ac370aa7c99166a`
  （档1 单独在 100k 窗口内的 3 处认领造成，与 `6ebe18eda1` 基线的
  `966d589b1a78f95f446f8856db13addb924699f201a11618aee7ce1c0a40023f` 不同——同上）。
- **两 sha256 文件的 `stdout` 行一个字符未改**：`bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c`
  （20k）/ `d8b69c180c23c5e393bf3c330825d88ae9c989ff38f2b8865e049e1b5eb56da8`（100k），
  与 #527/#601/#613/#603 原交付记录逐字相同，跨票一致。

## 2. 档1 完整保留自证

`git diff 6ebe18eda1 -- rust/src/theta_v0/classifier/nest_lifecycle.rs` 全量对拍：
新增 382 行，全部落在 §1 列出的档1 代码/测试/文档范围内，无一行属于档2；结构与 #603 原交付
diff 中"档1 部分"逐段相同（判据函数体、`CenterUpgraded` 枚举、`force_overtake_claimed_count`
字段、`advance()` 内的 match 分支、`assert_invariants` 断言、`center_upgrade_match`、3 个测试）
逐字未改一处。

## 3. 复测对照（BTC 100k，档1-only vs #603 原交付 vs `6ebe18eda1` 基线）

`P421_LIFETIME_SUMMARY`：

| 读数 | `6ebe18eda1` 基线 | #603 原交付（档1+档2） | 本票（档1-only） | 归因 |
|---|---:|---:|---:|---|
| entries | 302 | 304 | **301** | 基线 302 中，本应独立走完成路径的候选 66981，在基线下产生 2 条孤立记录（占槽期 Provisional 孤儿 + 完成时新建），档1 的迁移形态把它们合并为 1 条 ⟹ 302−1=301。#603 原交付 304 = 301 + 档2 贡献的 3（队列新观测到的独立候选，与档1 无关） |
| first_provable | 208 | 208 | **207** | 同上，随 entries 合并同步 −1；#603 原交付 208 = 207 + 档2 贡献 +1 |
| confirmed | 135 | 135 | 135 | 不变 |
| force_overtake（账本数） | 40（35 L1+5 L2） | 40 | **40** | 三处正确性约束（档2 专属）修完后 #603 原交付已回到基线 40；档2 摘除后自然仍是 40（约束是档2 内部自洽机制，摘除档2 整体后账本数回到「档2 从未存在」的基线值） |
| **force_overtake_claimed**（本票口径） | — | 2 | **2** | 档1 独立产出，逐位与原交付相同（15984/71448 两例，66981 走迁移不计入 claimed） |
| never_constituted | 74 | 74 | 74 | 不变 |
| identity_vanished_refuted | 29 | 31 | **28** | 迁移合并把基线下本会成为「孤儿→IdentityVanished(refuted)」的那条记录消解掉 ⟹ 29−1=28；#603 原交付 31 = 28 + 档2 贡献的 +3 |
| identity_vanished_seam | 23 | 23 | 23 | 不变（本身与档1/档2 均不相关，#599 §3.4 推断成立） |
| flash_terminal | 75 | 74 | **74** | 档1 单独贡献 −1（与 #603 原交付相同），档2 对此项零贡献 |
| nonflash_count / median | 226 / 88.0 | 229 / 95.0 | **226 / 88.0** | 与 `6ebe18eda1` 基线逐位相同——nonflash 计数与寿命中位的全部偏移（+3 / +7.0）来自档2 的队列观测，档1 零贡献，摘除后精确回落 |
| force_lifetime median | 64.5 | 64.5 | 64.5 | 不变 |

**覆盖率对拍**（`/tmp/wt603rv_cov.py`，逻辑与 #603 原交付脚本 `wt603_final.py` 相同，
仅去掉档2 专属的 `cand_age` 字段解析）：

| 口径 | 读数 | 与 #603 原交付对拍 |
|---|---:|---|
| 身份键口径（202 只 L1 完成信号） | 156/202 = **77.2%** | 逐位相同（原交付：77.2%，不变） |
| 档1 认领逐条 | 3 条：as_of=15984（认领方 Confirmed，前身 ForceOvertake）、as_of=66981（迁移，前身键无终局记录）、as_of=71448（认领方 NeverConstituted，前身 ForceOvertake） | 与 #603 原交付 §2.3 三条逐位相同 |

**结论**：档1-only 的全部读数偏差（entries/first_provable/identity_vanished_refuted 相对
`6ebe18eda1` 基线各 −1）均可用"迁移合并了基线下的孤儿记录"这一单一机制解释，且与 #603 原交付
的对应部分（原交付减去档2 贡献）逐位吻合，无残留档2 痕迹、无新增未归因偏差。

## 4. 字节护栏

| 面 | 窗 | cmp | SHA-256 |
|---|---|---|---|
| p123 stdout | 20k | **0** | `bd9ac1d655f9d615…`（与 #527/#601/#613/#603 记录逐字相同） |
| p123 stdout | 100k | **0** | `d8b69c180c23c5e3…`（同上） |
| p123 lifecycle dump | 2000（全文） | **0**（对 `6ebe18eda1` 基线） | 直接取基线重锚，档1 对该窗口零触达 |
| p123 lifecycle dump | 20k / 100k | 差异**已归因**（档1 独立影响，§3/§1） | 20k `e8cb7dec…`，100k `fee254be…` |
| m8 `p3fold/wf7/wf8` trades | — | **0** | `cargo test --release --lib theta_v0::backtest::wverify_run::m8_byte_guardrail -- --ignored` 全绿 |
| m8 `p3fold/wf7/wf8` tower_events | — | **0** | 同上 |

`cargo test --release --test issue533_p123_byte_guardrail -- --ignored --nocapture`：
**2 passed / 0 failed**（`p123_replay_2000bars_byte_exact` + `p123_replay_large_windows_hash_guardrail`）。

## 5. 测试门

- `cargo test --release --lib`（`CARGO_TARGET_DIR=/tmp/kimi-nest-target-603rv`，
  `-- --test-threads=4`）：**2044 passed / 1 failed / 137 ignored**，唯一红 =
  `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（**#491**，既线）。
  （首次单跑默认线程数时另见 `incremental_tower_scaling_dominates_full_synthetic` 计时类测试
  在高并发负载下偶发红，单独重跑通过——与本票无关的既有 flaky，非回归。）
- `cargo test --release --bin p123_fast_replay`：**3 passed / 0 failed**（档2 专属测试
  `lifecycle_window_stem_freezes_right_edge_for_stale_candidate` 随机制一并删除，
  余下 3 个既有测试不变）。
- 档1 专属 3 测试（`cargo test --release --lib issue603`）：**3 passed / 0 failed**。
- `cargo build --release --lib --bin p123_fast_replay`：38 条警告，与 `6ebe18eda1` 基线
  （37 条 + 因 `--bin` 联动构建多打印 1 条重复，非新增）一致；两目标文件本身零警告。

## 6. 遗留（不替裁，照实登记）

1. **两条真因线索**（#603 原报告 §7-2，编排者裁定"另开票"）未在本票范围内处理：
   (a) parser 批量确认跳过的中间段（18 只，需改 provider 候选发现口径）；
   (b) 活窗与完成两路径 centers 投影不同源（17 只，与 #591/#592 根因同族）。
   本票只做回退，不涉及这两条真因的修复。
2. **覆盖率口径维持档1-only 实测**：77.2%（身份键口径，与 `6ebe18eda1` 基线相同）/
   候选口径（沿 `CenterUpgraded` 链回溯）预期与 #603 原交付的 78.7% 相同（3 条认领逐位对拍已
   确认，未重新跑候选口径脚本——身份键口径已充分验证档1 行为零漂移，候选口径为其派生量）。
3. `force_overtake_claimed` 只在 stderr 诊断面，未进任何真值/证书路径——与 #603 原交付登记
   相同，未变。

## 7. 结果包六要素

1. **结论**：档2（`ActiveFrontierQueue` pending 槽队列）已手术摘除，档1（暂认中枢严格同锚
   回溯认领 `bridge_by_center_upgrade`）一个 bit 未动；`git diff 6ebe18eda1` 对拍显示净增量
   精确等于档1 单档实装（382 行 `nest_lifecycle.rs` + 3 行 `p123_fast_replay.rs`）；BTC 100k
   复测全部读数偏差可用"迁移合并孤儿记录"单一机制解释，与 #603 原交付逐位吻合；字节护栏
   stdout/m8 六面 cmp=0，lifecycle dump 面差异已归因（2000-bar 与基线逐字节相同，20k/100k
   差异纯属档1 独立贡献）；测试门 2044 passed / 唯一红 #491，档1 3 测试全绱，档2 2 测试随
   机制一并删除。
2. **定义依据**：编排者裁定 comment-5111127257（"档2 回退：撤销 pending 槽队列机制（错误归因
   下建、实测零收益）；档1……保留"）；`#603` 原交付报告 §0/§7-1（"46 只逐桶复核……队列真做
   出来之后……没有一只是因为「槽位不够」而丢失的"）；`coding-style` 回放驱动性能缓存注记
   （档2 摘除后不再适用，本票不再引用）。
3. **边界条件**（何时翻转）：(a) 若 §6-1 两条真因线索后续被证明"确实需要队列式机制才能修复"，
   则档2 的具体实现形态需重新设计（不是简单恢复本票删除的代码——原队列是在错误归因下设计的
   depth=3/frozen_end/stale_candidate_redivided 三处约束，新设计须针对新归因重新推导）；
   (b) 若段账本前缀回缩重划的古怪线段情形被触发，档1 的迁移/认领判据不受影响（该风险点
   `#613 F1` 注记专属档2 的 confirmed 截断口径，随档2 一并移除）。
4. **下游推论**：`entries=301`、`first_provable=207`、`identity_vanished_refuted=28`、
   `flash_terminal=74`、`nonflash_count=226`、`nonflash_median=88.0`（回到 `6ebe18eda1` 基线）、
   `force_overtake=40`、`force_overtake_claimed=2`（档1 保留量）为本票后的当前真值；#603 原交付
   发布的 `entries=304`、`identity_vanished_refuted=31`、`nonflash_count=229`、`median=95.0`
   四读数中归属档2 的增量部分（entries +3、vanish_refuted +3、nonflash +3、median +7.0）
   随回退作废，下游引用须以本票为准。
5. **谱系引用**：#599（本票直接前置，档位语义与量化来源）；#603（原两档同交票，本票在其基础上
   做手术回退）；#613/#601（L2 活窗与整窗截断口径，本票保其读数零回归，未验证——档2 摘除不
   触及 L2 派生逻辑，L2 相关代码本就全部在档2 范围之外未被触碰）；#523（PanLive provider
   接缝根因 + 永禁清单，档1 沿用，档2 摘除不影响该纪律的适用范围）；#491（既线唯一红）；
   `formalization-validity-domain`（本票复测仍是 **L2 = BTC 100k 单窗真实数据**，不外推）；
   `no-patch-mentality`（回退是手术删除，非"留一个开关关掉"式的兼容性垫片——`ActiveFrontierQueue`
   类型、常量、测试整体物理删除，无死代码残留）。
6. **影响声明**：改动 5 个文件——`rust/src/theta_v0/classifier/nest_lifecycle.rs`（删除
   `ActiveFrontierQueue`/`ACTIVE_FRONTIER_QUEUE_DEPTH`/1 测试，档1 部分逐位保留）、
   `rust/src/bin/p123_fast_replay.rs`（整体回退到 `6ebe18eda1` + 补回 `force_overtake_claimed`
   一行字段）、`rust/tests/fixtures/issue533_p123_2000_dump.golden.txt`（重锚为 `6ebe18eda1`
   基线原文）、`rust/tests/fixtures/issue533_p123_{20000,100000}.sha256`（仅 `dump` 行重锚，
   `stdout` 行未动）。**未触**：p123 stdout / m8 trades / tower_events / 装配 / 证书真值路径 /
   `Cargo.toml` / `formal/` / p409 / 他 session 未提交面（`chanlun/agent-roster-2026-07-21.md`、
   `chanlun/review-results/issue571-*`、`chanlun/review-results/treasury-reverify-*` 等并发
   工位的未提交改动本票未碰、未 add）。未关票、未改 map、未改 roster。
