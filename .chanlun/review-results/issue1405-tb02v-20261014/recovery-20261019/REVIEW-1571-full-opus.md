# #1405 恢复部分（PR #1571）非作者全量独立评审 — review-0001

- **评审者**：claude-opus-5-5（Claude Code），**非作者**（未参与 #1405 / #1571 任何实施或补记会话）。
- **评审对象**：`git diff 5baeb030a6..1870e95895` 全量（6973642619、f60430fe91、74c386f19e、c4bbdd9f8a、1870e95895 五个提交，只改 `rust/src/bin/s_structure_session.rs`，+363/−1），经 #1571 合入为 `b34b0278ee`。基线 main `b63a3fd586`；只读检出 `/Volumes/AgentStorage/issue1405-review-ro-20261020`，HEAD `299485822f06af217f85b49a06f3fca96295ca1a`（= main + 文档补记提交）。
- **行号口径**：下文 `s_structure_session.rs` 行号按检出 HEAD（含 #1569 在 1870 之后的改动；被评代码段与 1870 相同，仅整体平移）。

## 方法与实际执行的命令

全部只读；**未跑 cargo，未调用 nc-remote**（结论不依赖新的编译/测试；下文凡属推理而非实测处均注明）。

- `git rev-parse HEAD`；`git log --oneline 5baeb030a6..1870e95895`；`git diff --stat` 与全量 `git diff 5baeb030a6..1870e95895`
- `git diff c4bbdd9f8a 1870e95895`（核 Q4 范围）；`git diff 1870e95895 HEAD -- rust/src/bin/s_structure_session.rs rust/src/bin/s_session_v2/`（核合入后漂移）
- 阅读：`open_db`/`open_historical_readonly`（277–332）、`ensure_initialized`（1112）、`init_core`（1221–1252）、`accept_core`（1379–1383）、`advance_core`（2097–2103）、`cmd_catalog/snapshot/query/meta/pragma/watch`（3752–3944）、`recover_core`（4017–4080）；`s_session_v2/mod.rs` 的 `init_v2`/`handle`/`serve`/`retained_watch`（942–1250、1348）；`s_session/s_readonly_server.py:55-60`（Python 查询层 `mode=ro` + `query_only`）
- `grep` 全部 `Connection::open` / `open_db` 调用点（产品代码只此两处开库）；`grep` 信号处理（`SIGTERM|signal_hook|sigaction|ctrlc|signal(`）于 `rust/src/session_transport.rs`、`s_structure_session.rs`、`s_session_v2/`、`rust/Cargo.toml`
- 阅读 `docs/agents/delivery-discipline.md:7-22`、`AGENTS.md:46-58`、`DELIVERY.md`、`RECOVERY-DELIVERY.md`、`browser.sh`、`browser-225737/` 全部收据、`s_session/tests/tb02a_runtime.py:226-301`、`tb02b_runtime.py:160-221`、`tb02v_runtime.py:150-205`、`s_session/s_service_control.py:133-160,214-239,314-339,481-505`
- `shasum -a 256` 两份收据：`test-s_session-bin-1870e95895.txt` = `4b548d50…ecd62`，`mutant-be4268a1af.txt` = `64808b91…a879`，与 RECOVERY-DELIVERY 表中前缀一致；`grep` 两份日志的 `tb1405`/`test result`/`EXIT=`

## 逐问结论

### 1. 正确性

- **只读判规则先于写连接**成立：`open_db`（277–283）在库存在时先走 `open_historical_readonly`；后者以 `SQLITE_OPEN_READ_ONLY` 打开（300–307），已初始化且 `rule_revision != RULE_REVISION`（含键缺失 → `None != Some`）即返回只读连接（329–332），不执行 `PRAGMA journal_mode=WAL / synchronous / fullfsync`，不可能检查点（只读连接关闭不检查点、不删 `-wal`）。只读连接对 WAL 库最多新建空 `-wal`/重建 `-shm` 索引，与注释（293–297）一致。
- **热回滚日志拒绝**成立：首个查询以 `extended_code == SQLITE_READONLY_ROLLBACK` 识别（308–318），返回 `StorageUnavailable`；rusqlite 默认开启扩展错误码，测试 `tb1405_old_rule_store_with_hot_rollback_journal_is_refused_unchanged` 实测通过。
- **recover 版本门与不建库**成立：`recover_core` 先判 `db.exists()`（4025–4030），再 `ensure_initialized` + 规则门（4031–4035），均在 `transaction_with_behavior(Immediate)` 之前；旧规则库在只读连接上即使越过门也写不进去（双保险）。
- **残余改写路径逐项**：检查点/WAL 设置/`fullfsync`——旧规则库不经写连接，无；`-shm`——非持久事实，按设计排除；并发写者——本二进制对旧规则库永不开写连接，旧二进制写者不在本票范围；未初始化库/`rule_revision` 缺失——前者走写连接（见 INFO-4，既有行为），后者按旧规则只读；**TOCTOU**——探测与写连接之间若要出问题，须 `rule_revision` 在窗口内由当前变为旧，而 `rule_revision` 只在 `init_core`（1252）写入、`init` 拒绝既有文件（1223），产品代码内不可达；外部替换文件不在威胁模型内（INFO-5）。
- **当前规则库路径**：仅多一次只读探测开/关；读数、恢复、继续推进由 `tb1405_current_rule_store_reopen_reads_identical` 锁住。唯一行为差异是当前规则库带热回滚日志时由「写连接自动回滚」变为拒开，而 S 库自 `init` 起即 WAL（`open_db` 在建 schema 前设 WAL），该现场不可达，且报错文案已写明（INFO-5）。
- 读路径（`verify_reachable_root`、`read_catalog/snapshot_in_tx`、`retained_watch`、`lookup`、`ready`、`tb02a::enrich_raw`）无写语句，旧规则库只读连接不会在读入口误报只读错误。

### 2. 测试

五个 `tb1405_*` 在 `1870e95895` 全量 86/0/1 中均 `ok`（收据 469/471/473/481/508 行，`EXIT=0`）。变异 `be4268a1af` 使 hot-wal、hot-journal 两例转红（收据 425/427、450 行，`EXIT=101`），另两例不依赖探测，按设计不受影响——收据与说明一致。缺口见 INFO-1/2/3：干净关闭例对只读探测不构成区分；hot-wal 例在变异下先死于「拒写」断言，字节对拍半边未被收据单独证明；矩阵缺 `rule_revision` 缺失、当前规则热日志、合入 #1569 后上一代生产规则串等。

### 3. 与交付合同/纪律的一致性、runtime FAIL 归因

- 归因主体**站得住**：`tb02a_runtime.py:257-289` 只有主体全成功才置 `CAPTURED_…`，异常写 `error`；`tb02b_runtime.py:199-221` 浏览器子进程返回码非 0 即抛错。`browser-run/RESULT.json` 无 `error`、有 `core_records: 154`、`cleanup.errors` 仅 `stop-s` ⟹ 主体（含真浏览器）成功，FAIL 只来自收尾；`check_browser` 只读浏览器原件。**验收 3 的 PASS 不受影响。**
- 但补记对「S 在 SIGTERM 后 10 s 多才退出」及其可能成因的表述无据（WARN-2）。
- 悬空评审指针（WARN-1）；serve 指引 recover 成死路（WARN-3）；session/browser 两检查对 runtime 状态的门控不对称（INFO-7，不构成 #799 违规）。
- 错误文案与注释：热日志文案准确；`cmd_pragma` 注释有一处易误读（INFO-6）。

### 4. 浏览器复核二进制的代表性

`git diff c4bbdd9f8a 1870e95895`：+34/−2，非测试改动只有 319 行一处报错字符串与 3874–3875 行文档注释；热日志分支在 real_C（新建 WAL 库）不可达 ⟹ **c4bbdd9f8a 二进制对 1870e95895 运行时行为有代表性**。构建来源与「终版」措辞的边界见 INFO-8。

## 发现

### BLOCK

（无）

### WARN

**WARN-1 补记引用的评审报告在检出中不存在（悬空指针）**
- `.chanlun/review-results/issue1405-tb02v-20261014/recovery-20261019/RECOVERY-DELIVERY.md:47` 写「报告见本目录 `REVIEW-1571-full-opus.md`（结论以该文件末行 `VERDICT:` 为准）」，HEAD `299485822f` 中该文件不存在（`ls recovery-20261019/REVIEW*` 无匹配）。
- 按 `docs/agents/delivery-discipline.md:12`（第 3 项：声明须在关票时已为真）与 `:15`（第 6 项：报告入 git 以仓内相对路径引用），**关票前必须**把本评审落到该路径（或改指针到实际入仓路径），并核对末行 VERDICT。现在不修不算缺陷，但关票时不处理即违反关票门。

**WARN-2 「S 在 SIGTERM 后 10 s 多才退出」及成因猜测与代码不符，应改为观察事实**
- `RECOVERY-DELIVERY.md:41` 断言「S 是在 SIGTERM 后 10 s 多才退出，没有遗留进程」，`:43` 把成因候选列为「库大小、检查点或关闭时序」。
- S 未安装任何信号处理：`rust/src/session_transport.rs`、`rust/src/bin/s_structure_session.rs`、`rust/src/bin/s_session_v2/` 无 `SIGTERM/sigaction/signal_hook/ctrlc`；`rust/Cargo.toml:65` `s_session` 特性只引 `sha2/rusqlite/libc`。SIGTERM 默认处置立即终止进程，SQLite 关闭/检查点根本不运行，「检查点、关闭时序」两个候选被代码排除。
- 收据只能证明「`stop()` 在 10 s 观察窗内未观察到 stopped」：`s_service_control.py:491-498` 把 `ps` 查询失败（`process_identity` 第 144–145 行抛 `ValueError`，`ps` 自带 1 s 超时）吞为 `state: unknown` 继续等待，超时后只抛「尚未退出」而不带 observations（505）；前一次 `225314` 的 `stop-q` 同样超时（RECOVERY-DELIVERY:43 自述），更像观察侧问题。同一 S 二进制在 session-run 的收尾 `stop-s` 零错误（`session-run/RESULT.json`）。
- 不影响验收 3 的结论；但补记里的这句是陈述为事实的推断，按关票门第 3 项应改写为「10 s 观察窗内未观察到退出（原因未查）」，并删去被代码排除的候选。

**WARN-3 旧规则库带未决 Begin 时，serve 指引「先 recover」而 recover 已被禁止，形成死路**
- `rust/src/bin/s_session_v2/mod.rs:1224-1225`：`advance_state != idle` ⟹ `StaleWriter：未完成Begin保持闭门，请监督器先正式recover`；本 diff 起 `recover_core` 对旧规则库返回 `VersionMismatch`（`s_structure_session.rs:4033-4034`），`s_service_control.recover` 只会得到子进程失败。
- 结果：旧规则且留有未决 Begin 的库无法以 serve 形态提供只读服务（CLI `catalog/snapshot/watch` 仍可读），报错指引错误。字节安全不受影响，属行为/文案后果；建议至少在票面或报错中登记「旧规则库未决 Begin 永久闭门，只能 CLI 读」，或让 serve 在旧规则库上给出 VersionMismatch 文案。是否改由主控裁定，不构成本次关票阻断。

### INFO

**INFO-1 干净关闭例对只读探测不构成区分（推理，未实跑变异）**
- `s_structure_session.rs:6659-6684`：无 `-wal` 的旧规则库上，若去掉探测，写连接 `PRAGMA journal_mode=WAL` 为无操作、无帧可检查点、关闭时删 `-wal`，主库字节不变 ⟹ 该例字节断言在变异下仍会通过；它真正咬住的只有 recover/advance 的版本门。`RECOVERY-DELIVERY.md:25` 已如实说明变异未覆盖此例。建议补 `meta_set(&reader, …).is_err()` 断言使其直接锁只读连接。

**INFO-2 hot-wal 例在变异下先死于「拒写」断言**
- `mutant-be4268a1af.txt:436-437` 失败点为 `s_structure_session.rs`（1870 行号 6287，HEAD 行号 6553）「历史库连接须在 SQLite 层拒写」，在末尾字节对拍之前 ⟹ 「主库/-wal 字节不变」断言本身未被收据单独证明会咬（推理上会：变异下先前的 recover/advance/accept 写连接关闭即检查点并删 `-wal`）。不影响结论。

**INFO-3 测试矩阵缺口**
- `rule_revision` 键缺失而 `session_id` 存在（代码按旧规则只读，`s_structure_session.rs:329`）无测试。
- 当前规则库带热回滚日志的拒开（文案 319 行声称「含当前规则库」）无测试。
- 非 SQLite 文件/未初始化既有库、经真实二进制 CLI 的端到端旧库用例均无。
- 合入 #1569 后 `RULE_REVISION` 已升为 `…+new-bi/2+segment-second/2`（HEAD `s_structure_session.rs:66`），#1405 自身二进制建出的 `…+new-bi/2+segment-second/1` 库成为最现实的旧库，但不在 6487–6494 的规则矩阵内（代码按「≠ 当前」统一处理，故只是覆盖缺口）。

**INFO-4 范围外的既有改写/建库路径（非本 diff 引入）**
- 存在但未初始化（无 `meta` 或无 `session_id`）的 SQLite 文件仍走写连接，`PRAGMA journal_mode=WAL` 会改文件头（`s_structure_session.rs:284-286`）。
- 除 recover 外，`catalog/snapshot/query/meta/watch/accept/advance/serve` 对缺失路径仍会经 `open_db` 新建空 WAL 库再报未初始化（3753、3766、3781、3867、3938、1379、2097、`s_session_v2/mod.rs:1212`），留下的空文件随后使 `init` 拒绝（1223）。与 recover「不建库」口径不一，建议另票统一。

**INFO-5 TOCTOU 与当前规则库行为差异的边界**
- 探测（只读）与写连接之间无锁；只有 `rule_revision` 在窗口内被改写或文件被外部替换才会出问题，前者产品代码内仅 `init_core:1252` 写且 `init` 拒绝既有文件，后者不在威胁模型内。
- 当前规则库遇热回滚日志由自动回滚变为拒开，S 库自 init 即 WAL，不可达；文案已说明须人工处置。

**INFO-6 `cmd_pragma` 注释易误读**
- `s_structure_session.rs:3874-3875`「旧规则库上为只读连接，读数不含写连接的 WAL/fullfsync 设置」：`journal_mode` 是库文件持久属性，只读连接读出的仍是 `wal`；真正缺的是 `synchronous`/`fullfsync` 的连接级设置。建议改为「journal_mode 为库持久值；synchronous/fullfsync 为只读连接默认值」。

**INFO-7 session 与 browser 两检查对 runtime 状态门控不对称（不构成 #799 违规）**
- `s_session/tests/tb02v_runtime.py:158`：session 检查要求 `CAPTURED_…`（收尾错误即 FAIL）；`:189-205` `check_browser` 只记录 `runtime_status` 不门控。两者判断对象不同（会话合同 vs 页面可见性），按 `AGENTS.md:54` 限定词 1 属不同判断，且 DELIVERY 照实记 runtime FAIL；但验收 3 的「收尾失败不影响」依赖人工读 `RESULT.json`（无 `error` + `core_records`），建议今后让 `check_browser` 显式断言「`error` 缺失且失败只来自 cleanup」。

**INFO-8 Q4 代表性成立；构建来源与「终版」措辞的边界**
- 代表性：见第 4 问，成立。
- 来源：`browser.sh` 在 nc-remote 工作树 `wt/20261019-225709-c4bbdd9f8a` 中构建到**共享** `CARGO_TARGET_DIR`（`~/.local/bin/nc-remote:19` 设 `~/nc-remote/target`）；运行期间二进制在 `source_hashes` 内（`tb02a_runtime.py:107-108,279`）故运行中稳定，构建→prepare 之间被他人覆盖只在理论上可能。sha256 `08dfc651…7bcb` 未与独立构建交叉核对。
- 措辞：`DELIVERY.md:87` 与 `RECOVERY-DELIVERY.md:27-29` 的「终版二进制」指 #1571 终版；main 在 #1569 后规则串已为 `segment-second/2`，该二进制不是 main 当前二进制（对 #1405 验收不构成问题，避免被误读即可）。

## 总评

被评 5 个提交的持久边界逻辑正确，未发现会改写旧规则库主库或 `-wal` 字节的残余路径；测试真实锁住了两个关键现场（变异收据可核）；Q4 代表性成立；runtime FAIL 只来自收尾、不影响验收 3。3 条 WARN 中，WARN-1 是关票时必须处理的簿记动作（落盘本评审或改指针），WARN-2 是补记措辞，WARN-3 是行为后果登记，均不需要改被评代码。

VERDICT: PASS
