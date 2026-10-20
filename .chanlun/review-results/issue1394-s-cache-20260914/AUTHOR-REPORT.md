# #1394 正式 S 审计缓存（cache.rs）性能修复 — 实施报告

- 固定基线：`a6a83440e418016c575d1f6d75bfcd8da72debd7`（本 checkout HEAD，未 commit、未 push、未切分支）
- 唯一变更文件：`rust/src/bin/s_session_v2/cache.rs`（`git diff --stat` = 1 file changed, 314 insertions(+), 42 deletions(-)）
- 变更后文件 blob：`877df29252f5357c8a542fb000d2a2ccef3f16fc`
- 未改：其他生产模块、公开 schema/持久化字节、审计检查、用户配置；未启服务/Prime/监督器；未派工蜂；未 commit/push

## 1. 问题（修复前）

`store_batches` 与 `store_root` 各自判断「decoded 记账 + root 记账 > limit」即把整个 `Memo` 清空。
真实 gen58 上「全部可达批次解码 + 完整 root」已经越过 268MB 临界 ⟹ 每次事务都清空 ⟹ 下一次
必然冷解析全部历史（`resume` 恒 0），缓存形同不存在。

## 2. 最小策略落地（对照授权口径）

1. **单一原始总量上限**：只有一个 `limit`；总量口径 = `root.bytes`（行字符串 + 每行 16B +
   `walk.source_bytes()` + identity 字段）+ `batch_bytes`（Σ「id + 原字节 + 等额解码规范源字节」）。
   store_batches 的预算 = `limit - root.bytes`（`saturating_sub`），不给两个池各发一份 limit。
2. **root 优先**：接入新 root 前先按剩余预算驱逐 decoded（`trim_batches`，先丢最大条目、同大小按 id
   降序，尽量保留更多条目）；decoded 全部装不下也不会连 root 一起丢。
3. **新 root 单独超限**：不再清整个 `Memo`——旧 root 若「仍在当前 limit 内 + identity 相同 + 仍是当前
   行前缀」则保留（`resume` 仍可复用该已核前缀），否则只丢该 root；decoded 缓存不动。
4. **store_batches 只收本次 capture 集合**：入参 `batches` 是唯一成员来源，旧条目仅用于排序优先
   （`warm`），不参与成员判定 ⟹ 移出可达集的旧缓存不会复活。
5. **装不下就跳过**：`select_batches` 逐条试装，单条超预算跳过并继续试后面的条目，不清空其他缓存。
6. **limit = 0 完全禁用**：`store_batches`/`store_root` 进入即整块置为 `Memo::disabled()`。
7. **记账口径不变、加法饱和**：条目录账仍是 `id + bytes*2`；root 仍是行字符串 + 每行 16B +
   `walk.source_bytes()` + identity；所有累加改为 `saturating_add`（含原 `store_root` 内的 `+` 链）。
   注释明写该上限**不是** allocator/RSS 上限，不声称 RSS 受其约束。
8. **未动的安全前提**（仅重排预算，不改判据）：`decoded` 仍逐字比较当前读到的 bytes；`resume` 仍比较
   完整 identity + 全部当前行前缀 + 调用 `walk.resume_from`；fresh 全域 capture/audit、损坏/ghost/
   index/Begin 验证、内容寻址门均未改；未引入 mtime/generation/data_version 信任。

## 3. 实际验证（命令 / 退出码 / 通过数）

全部在本 checkout 的 `rust/target` 下执行，未触碰其他工位 target；单次 release 构建 36.01s（首次），
后续增量数秒。

| # | 命令 | 结果 |
|---|---|---|
| 1 | `cargo fmt --check`（cwd `rust/`） | exit 0，无输出（rustfmt 1.9.0） |
| 2 | `CARGO_TARGET_DIR=<checkout>/rust/target cargo build --release --offline --features s_session --bin s_structure_session` | exit 0（`Finished release profile`；含 lib 既有 57 warnings，无新增 error） |
| 3 | `... cargo test --release --offline --features s_session --bin s_structure_session cache` | exit 0；**8 passed; 0 failed; 0 ignored; 77 filtered out**（13.70s） |
| 4 | `... --bin s_structure_session v2_bounded_cache` | exit 0；1 passed; 0 failed（既有 #1372 缓存复用测试） |
| 5 | `... --bin s_structure_session v2_warm_cache_cross_connection_damage_stops_all_writes` | exit 0；1 passed; 0 failed（既有跨连接篡改门测试） |

第 3 条覆盖的 8 条测试（6 新增 + 2 既有）：

- `cache::tests::root_survives_and_decoded_fills_only_the_shared_budget`：小容量下 root 保留、
  decoded 只填剩余预算，单条 1MiB 过大条目被跳过且不牵连其他条目，`root+batches == limit`。
- `cache::tests::growing_root_evicts_decoded_instead_of_dropping_everything`：新 root 仍在 limit 内但变大
  ⟹ decoded 部分驱逐（2 条 → 1 条）、root 装入，总量 ≤ limit。
- `cache::tests::oversized_new_root_keeps_previous_verified_prefix_usable`：新 root 单独超限 ⟹ 旧前缀与
  decoded 缓存都保留，`resume` 仍返回旧前缀长度 2；前缀变动后 `resume` 归 0（不留死前缀、不误用）。
- `cache::tests::decoded_requires_identical_bytes_and_identity_and_prefix_are_checked`：同 id 不同 bytes
  不命中；identity 改变、前缀内容改变、行数变短三种情况 `resume` 均归 0。
- `cache::tests::zero_limit_disables_cache_entirely`：limit=0 时 root/decoded 全空，`decoded` 无命中，
  `resume` 归 0。
- `cache::tests::decoded_cache_that_left_the_reachable_set_is_not_revived`：本次 capture 只给 `batch-a`
  ⟹ 旧 `batch-b` 被移除且不会复活。
- `v2::tests::v2_bounded_cache_reuses_only_fresh_equal_dependencies`、`v2::tests::v2_warm_cache_cross_connection_damage_stops_all_writes`：既有回归与篡改门保持绿。

测试内构造的最小 `AuditWalk` 只用于组装 `Root`（空索引、`source_bytes()==0`），不替代任何审计路径；
缓存语义断言全部落在 `Memo` 记账与 `resume` 判据上。

## 4. “公开字节未改”的证据面

- `git diff` 只含 `rust/src/bin/s_session_v2/cache.rs` 一个文件，改动面是该文件内部的内存 `Memo`：
  新增私有 helper（`batch_cost`/`batches_cost`/`root_cost`/`select_batches`/`trim_batches`/`Memo::disabled`）
  与 `#[cfg(test)] mod tests`。
- 对外接口零变化：`decoded`/`store_batches`/`resume`/`store_root`/`configure` 的签名与返回类型未变
  （diff 内无任何 `pub(super) fn` 增删行）；未新增依赖、未改 `Cargo.toml`。
- 该文件不写库、不生成 JSON、不触碰 `canonical_bytes`/schema/持久事实：本次改动只决定「哪些已验证条目
  留在进程内缓存」，任何路径下的审计与持久写入代码均未修改。

## 5. 未验项（照实）

1. **未在真实 gen58 库上复现 268MB 临界**：本 checkout 没有隔离库副本，按授权不得启服务/Prime，故未跑
   `testonly_audit_cost_breakdown`（需 `S_AUDIT_COST_DB`）与任何真实库计时。新旧命中率差异在真实库上的
   数值收益**未实测**。
2. **未做 RSS/内存实测**：本上限是规范源字节记账，不是 allocator/RSS 上限，报告与注释均未声称 RSS 受其限制。
3. **未跑全量验证**：未跑库级 3000s 级全量 `cargo test`，只跑了上述靶向过滤；也未跑 `s_session/tests` 的
   Python 端服务测试（需启服务）。
4. 未验证极端规模（数千 decoded 条目）下的驱逐耗时曲线；驱逐路径已改为一次性排序（O(k log k)），未做压测。

## 6. 交付面

- 差异留在当前 `codex/1394-s-cache` checkout 工作区（未 commit）：`rust/src/bin/s_session_v2/cache.rs`。
- 本报告：`.scratch/issue1394-s-cache/report.md`。
