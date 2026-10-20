# #1406 S3 报告：WE7 上 TB03C 持久写操作的真实子进程 SIGKILL 矩阵

- 分支：`codex/1406-s3-kill`（基于 `origin/codex/1406-owner-20261020` @ `6c8477ff6f`）
- 代码提交：`7db30b70ff`
  - 新增 `rust/src/economic_session/tb03c_kill_tests.rs`
  - `mod.rs` 增加一行 `#[cfg(test)] mod tb03c_kill_tests;`
- 生产代码零改动。`tb03c-book/1`、`FROZEN-MANIFEST.json`、`RAW.json`、`HAND-ORACLE.json` 均未改。
- 范围：本地事务进程故障缝，即生产既有的 `Config::pause` 位于 `tx.commit()` 紧前和紧后。不涉及正式 socket、S 进程或浏览器。
- 本报告只陈述做了什么、观察到什么，不作通过或不通过的结论，结论交协调者裁定。

## 1. 覆盖范围：WE7 会写持久层的操作

WE7 = `s_session/tests/fixtures/tb03c_allocation/RAW.json`，共 6 个事件：

| 事件 | 类型 |
|---|---|
| N1-open / N2-open / N3-open | 开仓 |
| net-fill-1 / net-fill-2 / net-fill-3 | 净额成交 |

**WE7 没有 Reduce，也没有 correction 事件**，所以本矩阵不含这两类操作。如果要覆盖它们，需要另一份夹具，这不在本切片范围内。

WE7 主流程共 21 步，消息 `m-0` 到 `m-20`：

| 步 | 内容 |
|---|---|
| 0 / 2 / 4 | E Capture 开仓 |
| 1 / 3 / 5 | B1 / B2 / B3 Apply 开仓 |
| 6 / 11 / 16 | E Capture net-fill-1..3 |
| 7 / 12 / 17 | E Resolve（cursor 4..6） |
| 8–10、13–15、18–20 | B1 / B2 / B3 Apply 净额分配 |

### 矩阵

8 个目标 × {before_commit, after_commit} × 2 次独立新进程 = **32 次真实 SIGKILL**：

| 测试 | 目标步/消息 | 操作 | 被杀域 | 说明 |
|---|---|---|---|---|
| `tb03c_kill_capture_opening` | 0 / m-0 | CaptureExternal | E | 首个事实 |
| `tb03c_kill_apply_opening_n1` | 1 / m-1 | ApplyAllocation | B:chong-N1 | 开仓 Apply |
| `tb03c_kill_capture_netted_fill1` | 6 / m-6 | CaptureExternal | E | 净额 Capture |
| `tb03c_kill_resolve_fill1` | 7 / m-7 | ResolveEvidence | E | 唯一的 Resolve 类 |
| `tb03c_kill_apply_fill1_n1_zero_share` | 8 / m-8 | ApplyAllocation | B:chong-N1 | 零份额分配（HAND quantity=0） |
| `tb03c_kill_apply_fill1_n2` | 9 / m-9 | ApplyAllocation | B:chong-N2 | |
| `tb03c_kill_apply_fill1_n3` | 10 / m-10 | ApplyAllocation | B:chong-N3 | |
| `tb03c_kill_apply_fill3_n1` | 18 / m-18 | ApplyAllocation | B:chong-N1 | 最后一份报告；B 已有 3 个提交 |

frontier 原建议的矩阵是 {Capture, Resolve, Apply@B1/B2/B3} × 2 × 2 = 20 次，本矩阵是它的超集。没有覆盖每一步：fill-2/3 的 Capture、Resolve，以及 fill-2 的 Apply 未单独注入（见 §6 裁定点 3）。

## 2. 注入点如何保证恰好位于持久提交紧前和紧后

### 故障缝位置

`Config::pause` 的调用点（`service.rs` 当前行号）：

| 操作 | before_commit | `tx.commit()` | after_commit |
|---|---|---|---|
| Capture | 829 | 831 | 832 |
| Resolve | 890 | 892 | 893 |
| Apply | 1145 | 1147 | 1161 |

- 每个操作的业务写（facts/pending/applications/book/commits/receipts）和 `bind_message`（messages 表）都在同一个 IMMEDIATE 写事务内完成，然后才调用 `before_commit`。
- Apply 在两处 pause 之间：
  - 1146 是 `check_operation_deadline`，本测试为 None；
  - 1148–1160 是 `reconcile_final`，只在 `/2` 内容载体下运行，WE7 是 `/1`，所以为 None。
- 存储设置：WAL + `synchronous=FULL` + `fullfsync=ON`（`store.rs:528`）。

### 激活与 marker

- manifest 的 `test_only_fault {message_id, phase, marker_path}` 指定唯一一个故障点。只在 TestOnly 时钟下生效。
- pause 以 `create_new` 写 marker `{domain_id, message_id, phase, pid, test_only}`，fsync 后无限停驻。
- 父进程只在读到完整 marker 后才 `kill()`，并断言：
  - marker 的 pid 等于被杀子进程的 pid；
  - domain、message、phase 都是计划值；
  - `ExitStatus::signal()==Some(9)`。

### 落盘边界（经验证据，独立于代码阅读）

- 生成参照计划时，在进程内记录了每步之前四个域的业务表全量转储（除 meta 外的全部表），记为 `snapshots[i]`。
- 被杀后，直接以只读方式打开四个库做转储，断言：
  - **before_commit**：四域业务表逐字节等于 `snapshots[target]`。目标消息在事务内写入的所有行都没有落盘，messages 中也没有绑定。
  - **after_commit**：四域业务表逐字节等于 `snapshots[target+1]`，且 messages→receipts 中已绑定的原回执逐字节等于无崩溃回复。
- 被杀进程的 progress 文件每步回复 fsync 后才执行下一步。断言它恰好有 `target` 行，并与计划回复逐字节相同，即目标消息没有回复。
- WAL 字节数记录在 evidence.json 的 `wal_bytes_at_kill` 字段。

## 3. 每次运行的流程与断言

每次运行在一个新目录中 init 四个域（E + 3 B），分三个新子进程：

**A：执行到目标消息并被杀**
- `steps[..=target]`，epochs 全为 1。
- 停在故障缝后被 SIGKILL，做 §2 的边界断言。
- 对四个域执行 `store::audit`。

**R：恢复**
- 新进程执行 `store::recover`，得到 `Recovered`，`writer_epoch="2"`。
- 断言 recover 不改业务表（除 meta、epochs 外逐字节不变）。
- 断言 epochs 恰好为 `[(1,init,"1"), (2,recover,"1000000")]`。

**C：重送并续跑**
- 被杀域用 epoch 2；如果被杀的是 E，B 的 Apply 以 `e_epoch=2` 读 E。
- 原样重送目标消息两次，然后续跑剩余步骤。

**断言 2（恰好一次 / 原回执）**
- **after_commit**：重送回复逐字节等于被杀前已持久的原回执，`commit_seq` 等于被杀时的 seq，即**无新增提交**。
- **before_commit**：重送新增**恰好一个**提交（`seq = kill_seq + 1`）。
  - 回复不逐字节等于无崩溃回复，因为提交时刻取第二时钟列；
  - 投影（§4）后相等。
- 两种情况下，第二次重送都与第一次重送逐字节相同，且 seq 不变。
- 续跑每步的回复 kind 与投影都等于无崩溃参照。

**断言 1（终态与公开查询对参照）**
- 四个域都执行 `audit`。
- 四域全部表（除 meta、epochs）的投影等于无崩溃参照。
- **被杀域为 B 时**：E 和另外两个 B 的业务表**逐字节**等于参照，它们的公开查询也逐字节相等。
- 公开查询包括：
  - 各域 `ReadView`；
  - 各域 facts 表中每个事实的 `QueryFact`；
  - 各 B 的 applications 表中每个 allocation 的 `QueryAllocation`。
- 被杀域（被杀者为 E 时则是全部域）的公开查询比较投影。

**断言 4（独立 oracle）**
- 终态 B1/B2/B3 的 `actual_quantity` = [2,5,5]，`cost_basis.K` = [204,506,506]。
- E 的 `fill_allocations` 中三份净额报告逐腿的 `units` 和 `commission` 等于 `HAND-ORACLE.json` 的 `quantity` 和 `fees`。
- 参照生成时还逐份报告核对了中间账本（[1,4,3]/[100,402,301]、[1,4,4]/[100,402,403]）。
- `HAND-ORACLE.json` 由 `test_oracle.py` 用标准库 `fractions.Fraction` 独立复算，不导入 Rust 或生产 reducer。本机执行 `python3 test_oracle.py`：`Ran 7 tests ... OK`。

**断言 3（两次新进程字节一致）**
- 同一组合（目标 × 阶段）的 rep-0 和 rep-1 都是全新目录、全新 A/R/C 进程，按顺序执行。
- 断言四域**完整全表转储**（含 meta、epochs、commits 根链与时间）的 canonical JSON 逐字节相同。
- 续跑回复序列也逐字节相同。
- 两次运行使用同一个绝对路径，上一次运行结束后改名为 `rep-N`。原因是 meta.manifest_hash 依赖绝对路径。

**无崩溃参照**
- 先在进程内生成计划，再由新进程无故障重放两次（同路径）。
- 断言：
  - 每步回复逐字节等于进程内生成；
  - 业务表等于进程内终态；
  - 两次完整全表逐字节相同；
  - 参照终态通过 oracle。

## 4. 投影规则：去掉了什么，为什么

`project()` 递归去掉以下字段，字符串形式的 JSON 先解析再投影：

- 所有以 `_ns` 结尾的键（received/first_known/commit/e_first_known 等时刻）；
- `root`、所有以 `_root` 结尾的键、`root_hash`、`image_hash`；
- `writer_epoch`。

理由：

- 被杀域恢复后，TestOnly 时钟按 `epoch-1` 取第二列（`1e6 + t + offset`），因此恢复后该域的所有新提交时刻在定义上就与参照不同。
- root 链包含 `commit_ns` 和 image hash，所以随之不同。

**保留逐值比较的内容**：

- 数量、金额、K、cost_basis；
- 分配 id 与 units/commission；
- fact_key、fact_hash、token、receipt 绑定；
- `commit_seq`；
- 表的行集合与主键。

**例外：被杀者为 B 时，未被杀的域不经投影，直接逐字节比较。**

## 5. 远端结果

所有 cargo 都在远端执行（`NC_TARGET=target-1406-s3`，ref 为分支名，带 `--features s_session`）。本机没有运行 cargo。

| 命令 | 结果 | 日志 |
|---|---|---|
| `cargo test --lib --features s_session -- tb03c_kill --nocapture` | 8 passed / 0 failed / 1 ignored（子进程入口 `tb03c_kill_child`）/ 3269 filtered；154.36s | `/tmp/nc-remote/20261020-235148-7db30b70ff.log` |
| `cargo test --features s_session --lib economic_session` | **163 passed / 0 failed / 9 ignored** / 3106 filtered；224.24s | `/tmp/nc-remote/20261020-235702-7db30b70ff.log` |

- 两份日志都已用 `grep '^test '` 确认 8 个 `tb03c_kill_*` 测试名确实执行并显示 `ok`。
- 第一份日志有 32 行 `S3-KILL {...}`，内容见下表。
- 完整套件中 9 个 ignored 都是既有项，加上本切片的子进程入口：
  - c5_stage HFS 1 个；
  - finance `durable_child`；
  - financial_content 6 个；
  - `tb03c_kill_child`。

32 行 `S3-KILL` 的摘要：

| 组合 | phase | 被杀时 seq | 重送 kind / seq | rep-0 与 rep-1 终态 sha256（前 16 位） |
|---|---|---|---|---|
| capture_opening | before | 0 | Recorded / 1 | 021ea527d07c2241 ×2 |
| capture_opening | after | 1 | Recorded / 1 | 904aa816c9b874bb ×2 |
| apply_opening_n1 | before | 0 | Applied / 1 | 0b4ddea3958703df ×2 |
| apply_opening_n1 | after | 1 | Applied / 1 | e1fd87d8ec0a7e38 ×2 |
| capture_netted_fill1 | before | 3 | Recorded / 4 | c37d072f1526379d ×2 |
| capture_netted_fill1 | after | 4 | Recorded / 4 | 9a504882c66fc77a ×2 |
| resolve_fill1 | before | 4 | Resolved / 5 | 52d0f630ff5246be ×2 |
| resolve_fill1 | after | 5 | Resolved / 5 | 4feda324f2e49193 ×2 |
| apply_fill1_n1_zero_share | before | 1 | Applied / 2 | 1c81f8f203f60959 ×2 |
| apply_fill1_n1_zero_share | after | 2 | Applied / 2 | 35c726e0dc08f337 ×2 |
| apply_fill1_n2 | before | 1 | Applied / 2 | 0e30d4c8a85179bb ×2 |
| apply_fill1_n2 | after | 2 | Applied / 2 | a923390d256ace91 ×2 |
| apply_fill1_n3 | before | 1 | Applied / 2 | 16469363398e871b ×2 |
| apply_fill1_n3 | after | 2 | Applied / 2 | f1d083a9ebc95337 ×2 |
| apply_fill3_n1 | before | 3 | Applied / 4 | 2068e2f5dfe3fcc2 ×2 |
| apply_fill3_n1 | after | 4 | Applied / 4 | 20a57eb583311f85 ×2 |

所有 32 次的 signal 都是 9。

原始证据目录在远端机器的临时目录，路径见每行 `S3-KILL` 的 `raw_dir`（未设 `TB03C_KILL_ROOT` 时为 `$TMPDIR/nc1406-s3-kill-<pid>/...`）。每次运行包括：

- `evidence.json`，含 marker、pid、`wal_bytes_at_kill`、原回执、恢复回执、重送、续跑回复、终态全表与公开查询；
- 各子进程的 job、progress、stdout、stderr。

这些证据**没有回传本机**，本机只有上述日志。

## 6. 需要协调者裁定的事项

1. **被杀域的终态比较用的是投影相等，不是逐字节相等。** 恢复后时钟列不同，`*_ns` 和根链必然不同，所以我在被杀域上去掉了 §4 列出的字段再比较。被杀者为 E 时，B 的 image 中 `e_first_known_ns` 等也随之不同，因此 B 也只能比投影。这是否满足“公开查询与账本图像逐值一致”，需要裁定。投影键表是手写的白名单式剔除，可能剔除过宽或过窄，建议复核。
2. **提交前被杀后重送得到的回执是新提交回执，而不是“无崩溃回执”。** 断言只要求恰好新增一个提交（seq+1）、投影相等，以及二次重送幂等。
3. **矩阵是代表性的，没有覆盖全部 21 步。** fill-2/3 的 Capture、Resolve 以及 fill-2 的三个 Apply 没有单独注入。全量扩展到 21 × 2 × 2 = 84 次在机制上是同一个 `matrix()` 调用，单测试约 60–150s。是否需要全量，请裁定。
4. **跨域时序观察（生产语义，不是本切片的发现结论）。** 下面是按时钟计划推导的，没有逐值打印核对。在 E 崩溃的运行中：
   - E 恢复后的提交时刻取第二列，约为 1e6 + t；
   - 随后 B Apply 的 `received_ns/first_known_ns` 仍取 B 的 epoch-1 列，约为 1e3 量级；
   - 因此 B 观察时刻早于 E 的提交时刻。
   
   生产代码没有跨域因果时刻检查，所以测试通过。这在 TestOnly 时钟形状下是否可以接受，请裁定。
5. **没有做负控或变异验证。** 例如，没有验证把 pause 挪离 commit 后测试会失败。“恰在边界”的证据是 §2 的落盘快照对拍加代码位置，而不是变异实验。
6. **“两次独立新进程”的含义。** 两次运行按顺序执行，复用同一个绝对路径（中间改名），以便 meta.manifest_hash 也逐字节一致。如果要求两次路径不同，就只能比较除 meta 外的表。
7. **覆盖边界。** WE7 没有 Reduce 或 correction。BindVoice 的故障缝（`service.rs:1253/1256`）不属于 TB03C WE7 操作，未覆盖。
8. 子进程入口 `tb03c_kill_child` 是 `#[ignore]` 测试，计入完整套件的 9 个 ignored 中，模式与 finance `durable_child` 相同。
