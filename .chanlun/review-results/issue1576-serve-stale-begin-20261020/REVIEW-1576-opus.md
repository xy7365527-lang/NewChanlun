# #1576 独立评审：历史规则库有未完成 Begin 时 serve 的拒绝文案

## 范围
- 评审面：`git diff b63a3fd586..d3e7fd9e0e`。
  - a899d33a87：`rust/src/bin/s_session_v2/mod.rs`，新增 106 行，包括 serve 分支、新函数 `stale_begin_on_historical_rule`，和负控测试 `v2_serve_stale_begin_on_historical_rule_points_to_walkable_paths`。
  - d3e7fd9e0e：只有远端收据 `.chanlun/review-results/issue1576-serve-stale-begin-20261020/s-session-bin-a899d33a87.txt`。
- 对照代码：`rust/src/bin/s_structure_session.rs` 的 `open_db` / `open_historical_readonly` / `recover_core` / `cmd_catalog` / `cmd_snapshot` / `cmd_meta` / `init_core` / `run`，以及 `s_session_v2/mod.rs` 的 `dispatch` / `init_v2`。
- 来源：`origin/codex/1405-closegap-20261020` 分支上 `REVIEW-1571-full-opus.md` 的 WARN-3（第 62–64 行）。
- 只读评审，没有运行 cargo，也没有改动任何文件。

## 逐项核查

### 1. 拒绝没有放松
- **仍在 socket 绑定之前。** 新分支在 `mod.rs:1224-1231`，位于 `advance_state != idle` 的判断内，直接 `return Err`。它排在 `pending` 回放（1233-1252）和 `serve_unix`（1253）之前。前面的步骤只有 `process_lock`、`ClockPlan::load`、`open_db`，以及几项只读核查：epoch、可达根、`active`、clock、profile（1210-1223）。
- **没有写入。** 对历史规则库，`open_db` 走 `open_historical_readonly`，返回 `SQLITE_OPEN_READ_ONLY` 连接（`s_structure_session.rs:278-282`、`299-333`），写操作在 SQLite 层就会被拒。新分支只多做了一次 `meta_get_opt(rule_revision)` 读取（`mod.rs:1226`）。`process_lock` 只创建 `<db>.writer.lock` 旁路文件（`session_transport.rs:53-59`），不碰库文件。
- **未决 Begin 没有被清。** 新分支不改 `advance_state`。测试在 `mod.rs:1611-1614` 断言它仍以 `begun:` 开头。
- **判定口径一致。** 三处判定的是同一组库，缺失 `rule_revision` 一律视为历史规则：
  - serve 用 `unwrap_or_default() != RULE_REVISION`；
  - `recover_core` 用 `as_deref() != Some(RULE_REVISION)`（`s_structure_session.rs:4033`）；
  - `open_historical_readonly` 也是 `== Some(RULE_REVISION)` 才放行到写连接（`s_structure_session.rs:329`）。

### 2. 当前规则库的行为不变
- 当 `rule == RULE_REVISION` 时，代码落回原来的 `StaleWriter：未完成Begin保持闭门，请监督器先正式recover`（`mod.rs:1230`），字符串一字未改。
- 测试在 `mod.rs:1570-1575` 对当前规则库做了对照，断言错误以 `StaleWriter：` 开头且含 `recover`。
- 全仓 grep 这条旧文案，只有代码自身和本测试命中；Python 监督器只在 accept 路径（`tb02b_runtime.py:181`）匹配 `StaleWriter` 前缀，不解析 serve 启动错误。改前缀不会影响其他调用方。

### 3. 文案列出的路径能否走通
| 文案里的路径 | 代码证据 | 结论 |
|---|---|---|
| recover 也会拒绝（不换代、不清未决 Begin） | `recover_core` 在 `s_structure_session.rs:4033-4034` 返回 `VersionMismatch`；`dispatch` 的 v2 recover 也调同一函数（`mod.rs:1331-1343`） | 准确，测试在 `mod.rs:1603-1605` 实际调用确认 |
| ① CLI `catalog`/`snapshot`/`meta --db <path>` 只读查看 | `v2::dispatch` 不处理这三个命令，`legacy_write_lock` 也不给它们加锁（`mod.rs:1350-1358`）；它们最终调 `cmd_catalog` / `cmd_snapshot` / `cmd_meta`（`s_structure_session.rs:4181-4183`），都经 `open_db`，因此拿到只读连接 | 可走通，测试在 `mod.rs:1607-1609` 实际调用 |
| ② 人工以当前规则 `init` 新库（新 `--session-generation`），并重新接纳原始输入 | `init_core` 拒绝已存在的目标（`s_structure_session.rs:1223-1228`），因此"新库"必须是新路径，文案的"新库"措辞成立。带 `--session-generation` 会进入 v2 init（`mod.rs:1280-1299`），新库写入当前 `RULE_REVISION` 且 `advance_state=idle`（`s_structure_session.rs:1251-1252`） | 能走，但文案只列了一个参数，见 WARN-1；文案已明确这是人工步骤，并注明没有旧库迁移入口（TB-10 #1368），与票面一致 |

### 4. 测试能否咬住回退
- **文案改回旧版：** `msg.starts_with("VersionMismatch：")`（1595）和 `!msg.contains("请监督器先正式recover")`（1596）都会失败。能咬住。
- **删掉历史规则分支：** 会落回 `StaleWriter：` 开头的旧文案，1595 失败。能咬住。
- **只删文案中的某一条路径：** 1597-1599 逐项检查 `` `catalog` ``、`` `snapshot` ``、`` `meta --db ``、`` `init` ``、`#1368`。能咬住。
- **空 `-wal` 过滤（1583-1591）会不会掩盖字节变化：**
  - 取 `before` 时，写连接已在 1578-1580 的块结束时释放。WAL 库最后一个连接关闭时会检查点并删除 `-wal`，所以 `before` 里的 `-wal` 应为缺失。
  - 过滤只把"空文件"和"不存在"视为相同。要靠它掩盖写入，需要 `-wal` 从非空变成空且主库不变，在 `before` 为缺失时做不到。
  - 一旦有写连接写入并关闭，检查点会改变主库字节，而主库比对没有经过任何过滤。
  - 结论：这里没有假阳性。
- **收据：** 收据记录的 sha `a899d33a87…` 与被评提交一致。命令为 `cargo test --features s_session --bin s_structure_session`，结果 `89 passed; 0 failed; 1 ignored`，`EXIT=0`，目标测试为 `... ok`。

## 发现

### BLOCK
（无）

### WARN
**WARN-1：路径 ② 只写了 `--session-generation`，没写全 v2 init 必需的参数**
- `mod.rs:1267` 只写"`init` 新库（新 --session-generation）"。但 v2 init 必须同时给出 `--db`（新路径）、`--session`、`--catalog`、`--profile`、`--session-generation`、`--clock-plan`、`--delivery-retain-generations`（`mod.rs:1289-1297`）。
- 后果：
  - 若运维照文案只带 `--session-generation`，`dispatch` 会进入 v2 分支，再由 `arg()` 逐个报"缺 --xxx"，仍能自行补齐，不是死路。
  - 若运维把 `init` 理解成 usage 里的 v1 形式 `init --db --session --catalog`（`s_structure_session.rs:4105`），会建出 v1 库。之后 serve 报 `SchemaUnsupported：serve需要显式v2初始化`（`mod.rs:1217`），而该路径已被占用，`init` 也拒绝覆盖，只能再换一个路径。
- 拒绝行为本身不受影响，但验收 (1) 要求"可执行的下一步"，文案可以更准确。建议改为"以当前规则 v2 `init`（`--session-generation`/`--clock-plan`/`--delivery-retain-generations` 等，新路径）"，或直接指向 v2 init 的用法说明。
- 不构成阻断：文案已明确这是人工步骤，且第一次报错就会把运维引到 v2 参数。

### INFO
- **INFO-1：测试注释说的比做的多。** 1619 行注释写"新库可通过 serve 前置的规则与闭门检查"，但测试只断言 `rule_revision == RULE_REVISION` 和 `advance_state == idle`（1632-1643），没有对新库实际调用 serve（那会绑定 socket 并阻塞）。这两项正是本票相关的闭门条件；clock 和 profile 用的是同一份输入，推理上也会通过。建议把注释改成"满足 serve 的规则与闭门前件"。
- **INFO-2：socket 断言区分力有限。** 1600 行的 `!socket.exists()` 在新旧两版代码下都成立。如果有人删掉整个闭门检查，测试会卡在 `serve_unix` 上而不是报失败。闭门本身的保护主要靠 1595 行的前缀断言和 1611-1614 行的未决 Begin 断言，这两处是有效的。
- **INFO-3：`rule_revision` 缺失时显示为空。** 键不存在时文案打印 ``rule_revision=` ` ``（即空值）。语义正确，只是展示不直观；该场景没有测试，与源评审 INFO-3 的缺口相同。
- **INFO-4：历史规则 + idle 库仍会进入 serve（本 diff 之前就如此，超出本票范围）。** 这种库会通过 1224 行的判断，继续走 pending 回放和 `serve_unix`，此后的写请求只能靠 `advance_core` / handle 内的版本门或只读连接拒绝。是否要让 serve 对所有历史规则库都提前给出 `VersionMismatch`，建议由主控另票裁定。
- **INFO-5：只读路径可以多列两条。** 文案列了 `catalog`/`snapshot`/`meta`；`watch`/`query` 同样走 `open_db` 只读连接，也可用（源 WARN-3 提到了 `watch`）。不列不算错。

## 结论
- 验收 (1) 达成：
  - 历史规则库带未完成 Begin 时，serve 仍在绑定 socket 之前、经只读连接、不改未决 Begin 地拒绝，拒绝没有放松。
  - 文案不再指向必拒的 recover，改为给出可走的只读 CLI 路径，以及明确标为人工的新路径重建步骤，并如实说明没有旧库迁移入口（#1368）。
  - 当前规则库的分支和文案逐字不变。
- 验收 (2) 达成：负控测试构造了目标状态，逐条实际走了文案列出的路径（recover 被拒、三条 CLI 命令、v2 新库初始化）。它能咬住文案回退，字节比对的过滤也不会造成假阳性；远端收据记录全绿（89 passed，EXIT=0），sha 与被评提交一致。
- WARN-1 只是文案精度问题，建议顺手补齐 v2 init 参数，不阻断合入。

VERDICT: PASS
