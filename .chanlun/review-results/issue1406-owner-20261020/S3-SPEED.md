# #1406 S3 kill 矩阵降时长报告（覆盖不变）

分支 `codex/1406-s3-speed`，起点 `origin/codex/1406-owner-20261020 @ aab509ed08`。代码提交 `5a0303feca`，只改 `rust/src/economic_session/tb03c_kill_tests.rs`，生产代码零改动。作者自报，待非作者复审。

## 结论

Linux（windows-5090 WSL，8 vCPU）上 `cargo test --features s_session --lib -- tb03c_kill --nocapture` 从 **1149 s 降到 279 s**。覆盖不变：84 次真实 SIGKILL，每个组合两次独立新进程，规范化逐值比较，(a)(b) 常驻检查都保留。

| | 提交 | 测试数 | 用时 | 日志 |
|---|---|---|---|---|
| 优化前 | `a7f852c7a6`（S3 返修） | 23 passed / 1 ignored | 1149.17 s | `/tmp/nc-remote/20261021-002009-a7f852c7a6.log` |
| 优化后 | 树同 `5a0303feca` 的临时提交 `2368e62662`（squash 前；squash 后 tree 哈希逐字相同） | 44 passed / 1 ignored | **279.30 s** | `/tmp/nc-remote/20261021-030654-2368e62662.log` |

优化后日志里有 84 行 `S3-KILL`（signal 全为 9），两条 `S3-NEGATIVE` 的差异位置与优化前一致（终态域 0 `/commits`；终态域 2 `/applications`）。测试数从 23 变成 44：21 步 × 2 阶段 = 42 个组合测试，加 2 个负控。

## 时间花在哪（profile）

在每次运行里加了 `S3-TIME` 分段计时，并在子进程的进度行里加了逐步 `ms` 字段（只用于计时，任何断言都不读它）。先在单测试上量（`/tmp/nc-remote/20261021-023935-791f7d23e7.log`，s10，不与其他测试争用），一次运行约 52 s：

| 段 | 耗时 |
|---|---|
| 被测子进程 A 与 C 的业务步骤（含生产 audit） | ≈ 3.1 s |
| kill / recover 及边界断言 | ≈ 1.0 s |
| 续跑回复比较 | ≈ 0.6 s |
| audit_all | ≈ 0.9 s |
| **终态四域全表规范化比较** | **≈ 30 s** |
| 公开查询（每次查询生产 `read` 都会 audit） | ≈ 8 s |
| **公开查询规范化比较** | **≈ 9 s** |
| 参照计划（所有测试都要等它） | ≈ 29–38 s |

结论：时间主要花在测试自己的比较函数上，不在子进程启动、轮询 sleep 或前缀重放。

## 改法与等价性

1. **`first_diff` 每个子树只走一次（主因）。** 原写法遍历对象时用 `x.keys().chain(y.keys())`，两侧共有的键各递归一次，同一子树被重复比较，代价随嵌套深度指数增长（库内图像是 JSON 字符串里套 JSON 再套原件）。现在先判 `a == b`，相等就直接返回 None；否则按 x 的键序递归，再查 y 独有的键。返回值的语义不变：相等时返回 None，不等时返回 x 键序下的第一处差异路径，和原写法找到的路径相同。这一项单独做完后，终态表比较从约 30 s 降到约 1 s。
2. **规范化记忆。** 在同一个 `Normalizer` 里（同一次比较、同一个时钟），内容相同的 JSON 字符串只解析和规范化一次；命中缓存时，按当前路径前缀还原商不合的记录。规范化只依赖字符串内容和时钟，所以结果和逐次重算逐字相同。参照的规范化（终态表、公开查询、逐步回复）只算一次，存起来复用。
3. **目标步从「目标前」的库文件副本起跑。** 原做法是：A 进程在新 init 的空库上逐步重放 `steps[0..target]`，再执行目标步。现在的做法：
   - 进程内无故障生成计划时，每步之后对四个域执行 `PRAGMA wal_checkpoint(TRUNCATE)`，断言返回 (0,0,0)、`-wal` 长度为 0，再复制主库文件到 `snapshots/before-NN/`。
   - 每次运行在**被测进程启动之前**把副本复制到本次目录。库里唯一和目录或故障计划相关的持久值是 init 写进 meta 的 `manifest_hash`，按本目录的 manifest 改写它。
   - 然后断言整库逐表逐行等于「用本 manifest 在 scratch 路径新 init 出来的 meta」加上「snapshots[target] 的全部业务表（含 epochs、commits 根链）」。
   - 这个状态就是旧做法里 A 进程执行到目标步之前的持久状态，所以 A 现在只执行目标消息。之后的 SIGKILL、边界快照断言、新进程 recover、新进程重送与续跑都和原来一样，SIGKILL 仍然打在真实子进程上，「新进程从同一持久库恢复」的语义没有变：复制发生在被测进程启动之前，不是在被杀之后。
   - 前缀步骤在新进程里逐步执行的覆盖仍然保留：`reference()` 的两次无故障新进程全程重放会逐字核对 21 步的回复和终态业务表，各步的边界快照也逐字核对。少掉的只是同一前缀在每个组合里的重复执行。
   - 进度文件断言从「恰有 target 行且等于参照」改为「为空（目标消息无回复）」。
4. **参照轨延迟计算。** 原来的 `plan()` 一次做完进程内生成和两次新进程参照重放（约 30 s），所有测试都要等它。现在拆成 `plan()`（进程内生成加副本，约 4 s）和 `reference()`（两次新进程重放、公开查询、oracle、规范化，约 32 s，`OnceLock`）。`reference()` 只在终态比较时才用到，各组合可以先开始杀进程。内容和断言都没变。
5. **每个（步，阶段）一个测试。** 原来每步一个测试，里面两阶段串行；现在宏为每步生成 `<步名>::before_commit` 和 `<步名>::after_commit` 两个测试，每个测试内仍做 rep-0、rep-1 两次新进程，并断言终态全表和续跑回复逐字节相同。调度粒度减半，工作量不变。`-- tb03c_kill`、`-- tb03c_kill_s07` 等过滤仍然能命中。

marker 轮询间隔（10 ms）和子进程启动开销实测都可以忽略，没有改。

## 改后仍然保留的检查

- (a) 每个目标步都断言 `snapshots[target][owner] != snapshots[target+1][owner]`。
- (b) 两个跳过重送的负控，断言写在比较函数的返回值上。
- 提交前被杀的重送回执规范化后与参照相同、`seq = kill_seq + 1`；提交后被杀的重送等于原回执、`seq` 不变。
- B 被杀时，其他域的业务表和公开查询逐字节等于参照。
- HAND oracle 对拍。

## 变异复核（在新结构上重做；临时分支跑完已删，远端未推送）

在 `5a0303feca` 上对 Capture、Resolve、Apply 三处同时挪动暂停点，然后跑 `tb03c_kill_s00 / s01 / s07`。

| 变异 | 结果 | 失败断言 | 日志 |
|---|---|---|---|
| `before_commit` 挪到 commit 之后 | 3 个 `::before_commit` FAILED，3 个 `::after_commit` ok | `before_commit 被杀后域 0 业务表`（s00、s07）、`… 域 1 …`（s01） | `/tmp/nc-remote/20261021-031430-b0aece0c1c.log` |
| `after_commit` 挪到 commit 之前 | 3 个 `::after_commit` FAILED，3 个 `::before_commit` ok | `after_commit 被杀后域 0 业务表`（s00、s07）、`… 域 1 …`（s01） | `/tmp/nc-remote/20261021-031654-0ffa630c77.log` |

拆成每阶段一个测试后，失败能精确落到被挪动的那个阶段。

## 其他远端结果

- `cargo check --features s_session --lib --bins --tests`（`5a0303feca`）：EXIT=0，日志 `/tmp/nc-remote/20261021-031935-5a0303feca.log`。
- `cargo test --features s_session --lib economic_session`（`5a0303feca`）：**213 passed / 0 failed / 9 ignored，341.80 s**，日志 `/tmp/nc-remote/20261021-032007-5a0303feca.log`。44 个 `tb03c_kill_*` 全部 ok；9 个 ignored 都是既有项，含子进程入口。作参照：S3 分支上同一条命令此前是 1117 s（`/tmp/nc-remote/20261021-005338-a7f852c7a6.log`）。
- macOS CI 的 60 分钟上限：本次未在 macOS 上实测。主要耗时项是测试侧的比较函数，与平台无关，预期 macOS 也同比例下降，以 CI 实跑为准。
