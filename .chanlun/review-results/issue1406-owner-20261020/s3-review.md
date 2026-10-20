APPROVE（复审 8c276a3089；首审 fc42f24f40 为 REQUEST_CHANGES，见下文）

# #1406 S3 独立评审：WE7 TB03C 真实子进程 SIGKILL 矩阵（codex/1406-s3-kill @ fc42f24f40，基线 6c8477ff6f）

评审人：/root/owner_1406/review_c8（非作者，只读）。改动面 `git diff --stat 6c8477ff6f fc42f24f40`：只有 `tb03c_kill_tests.rs`（+731）、`mod.rs`（+2）、报告；生产代码零改动属实。
远端复跑：`NC_TARGET=target-1406-c8review nc-remote codex/1406-s3-kill rust cargo test --features s_session --lib -- tb03c_kill` ⟹ `8 passed; 0 failed; 1 ignored`，137 s（`/tmp/nc-remote/20261021-000926-fc42f24f40.log`）。本机 `python3 s_session/tests/fixtures/tb03c_allocation/test_oracle.py` ⟹ `Ran 7 tests … OK`。

## 结论

机制做得扎实：注入点确在提交边界，kill 时机与 pid 核对可靠，两次新进程是真实的两份数据库，oracle 独立。不批准的原因有三条 MEDIUM：恢复后时间值被整体删掉而不是核对，矩阵只覆盖 21 步中的 8 步，以及没有任何证据表明测试能区分「边界对」与「边界错」。都不是正确性缺陷，但 AC5 要证的恰恰是这三件事。没有 HIGH。

## 逐项核对

**(1) 注入点与 kill 时机——成立。**
- `service.rs:829/890/1145` 的 `before_commit` 位于写完全部行（facts/pending/commits/receipts）并 `bind_message` 之后、`tx.commit()` 之前；`:832/893` 的 `after_commit` 紧跟 `commit`。Apply 的 `after_commit`（`:1161`）在 `reconcile_final` 之后，但 `import_job` 只在 `SCHEMA_V2` 下存在（`:1002`），WE7 是 `economic-session/1`，所以这里等价于紧跟 commit。
- `Config::pause`（`store.rs:384-419`）是 #1374（`c0ed0497a1`）既有钩子，只由 manifest `test_only_fault` 触发；`Config::load` 在时钟不是 `TestOnly` 时拒绝任何故障计划（`store.rs:254-259`）。生产（`System` 时钟）不可触发。风险定为 LOW：生产二进制里确实留着一个无限 park 循环，靠 manifest 校验守住。
- 子进程以 `create_new` 写 marker 并 `sync_all` 后才进入 park；父进程读到能完整解析的 JSON 后，断言 marker 等于 `{domain_id, message_id, phase, pid: a.id(), test_only: true}`，然后才 `kill`，并断言 `signal() == Some(9)`（`tb03c_kill_tests.rs:463-494`）。pid 来自 `Child::id()`，可靠。被杀后先断言进度文件恰有 `target` 行且逐字等于参照回复，再断言四域业务表与 `snapshots[target]`（提交前）或 `snapshots[target+1]`（提交后）逐字相同，并断言消息是否已绑定回执与阶段一致（`:497-520`）。这才是「真在边界」的直接证据，比代码行位置更有力。

**(3) 两次新进程终态逐字节比较——真实。** `matrix` 每次 rep 都用 `create_dir`（目录已存在则失败）新建 `work`，跑完 `rename` 到 `rep-{n}`，`final_tables` 来自该次运行自己的数据库转储（`:653-672`）；参照轨同法（`:420-447`）。比较的不是同一份文件。同路径是为了让含路径的 `manifest_hash` 一致，做法合理（作者待裁 6：同意）。

**(4) oracle——独立。** `HAND-ORACLE.json` 是 C4a 既有夹具，由 `test_oracle.py` 用 `Fraction` 独立复算（只 import 标准库，docstring 声明不导入 Rust 或生产 reducer）。kill 测试用它核三个 B 的数量 [2,5,5]、K [204,506,506] 以及三次成交的分配数量与佣金（`:241-279`）。

## MEDIUM

**M1. 恢复后的时间值整体删除，没有核对（对应作者待裁 1、2、4）。**
`project`（`:146-166`）递归删除这些键，也会深入 JSON 字符串内部：所有以 `_ns` 结尾的键、`root`、以 `_root` 结尾的键、`root_hash`、`image_hash`、`writer_epoch`。按全仓字面量统计，实际命中 `commit_ns / first_known_ns / received_ns / e_first_known_ns / init_ns / recover_ns / at_ns / previous_commit_ns / prev_root / root / previous_root / root_hash / image_hash / writer_epoch`，以及 WE7 不涉及的 `valid_through_ns / reserve_ns / prepared_ns / as_known_ns`。
- **没有业务值被一起删掉**：数量、K、费用、分配、`fact_key`、`allocation_id`、`token`、`seq`、`recorded_seq`、`kind` 都保留；`source_namespace` 不以 `_ns` 结尾，也保留。所以「剔除过宽」在业务值这一层不成立。
- **问题在另一层**：崩溃轨上恢复域的全部时间完全没有被核对。按时钟计划，第 r 列 = r·1,000,000 + t + offset，所以恢复后提交的时间应当恰好等于参照时间加 1,000,000，未崩溃域与目标之前的提交应当一点不差。现在的写法查不出这类错误：重送 Capture 沿用了被杀前那一列的 `first_known_ns`，或者 B 把 E 的旧时间写进 `e_first_known_ns`。作者待裁 4（跨域时间先后）也落在这块盲区里。根链与 `image_hash` 由 `audit_all` 重放核一致性，删掉可以接受。
- **要求**：把时间键从「删除」改为「规范化后逐值比较」。规范化方式是 `v mod 1_000_000`，同时断言商 = 该提交所在域的 epoch−1。被杀域从目标消息起商为 1，其余为 0；E 被杀时，B 记录的 `e_first_known_ns` 从目标事实起商为 1。根链键仍可删除。提交前被杀重送的回执（待裁 2）：规范化后应与参照回执逐字相同（`kind`、`fact_key`、`cut.seq` 等），只有时间商与根不同。现有的 `assert_ne!(resent, reference)` 保留。

**M2. 矩阵只覆盖 21 个持久写步中的 8 个（待裁 3）。** 漏掉的 fill-2/3 的 Capture 与 Resolve、fill-2 的三个 Apply、fill-3 的 N2/N3 Apply，走的代码路径虽然相同，但持久状态不同：已有前序分配、累计前沿已推进、`fill_allocations` 已非空。重放正确性恰恰依赖这些状态。扩展的代码成本只是多几次 `matrix()` 调用。按本次实跑，8 个测试并行约 140 s；全量 21 步估计 4–6 分钟，在可接受范围内。
- **要求**：WE7 全部 21 步 × {before_commit, after_commit} × 2 次新进程 = 84 次真实 SIGKILL，与主控倾向一致。测试名一步一个，或者在单个测试内循环但逐步打印 S3-KILL 行。报告列出 42 个组合的终态哈希。如果 CI 时长是顾虑，可以把 2 次重复缩为 1 次，只对 8 个代表步保留双跑；我仍建议 84 次全跑，成本不高。

**M3. 没有反向验证，测试的区分力没有证据（待裁 5）。** 现有断言在边界正确时通过，但还没有证据表明边界错了它会失败。
- **要求**：
  - (a) 树内加一个常驻断言：对每个目标步，`snapshots[target][owner] != snapshots[target+1][owner]`。这样「提交前 = 前快照、提交后 = 后快照」两条不会退化成同一判据。
  - (b) 树内加一个常驻负控测试：`before_commit` 被杀并 recover 后，跳过重送、直接续跑，断言终态（规范化投影）与参照不同，或续跑中途报错。断言写在比较函数的返回值上，不要用 `#[should_panic]` 兜任意 panic。另做 `after_commit` 被杀后重送的回执与 `seq` 不变的对照：这一条已经由 `resent.seq == kill_seq` 覆盖。
  - (c) 一次性的变异运行，不入仓，日志归档进报告。具体做两个变异：把 `service.rs:829` 的 `before_commit` 挪到 `tx.commit()` 之后；把 `:832` 的 `after_commit` 挪到 `commit` 之前。各对应的 `tb03c_kill_capture_*` 必须失败，记录失败断言文本与 nc-remote 日志路径。Resolve、Apply 各挑一处同样做即可。

## LOW

- **L1** 生产二进制里留有 TestOnly 无限 park 钩子，靠 `store.rs:257` 的时钟校验守门。这是既有设计（#1374），本片未改，在报告中写明即可。
- **L2** Apply 在 `SCHEMA_V2`（带内容导入）下，`commit` 与 `reconcile_final` 之间的窗口没有故障点覆盖。WE7 不涉及，建议在报告的范围段写明。
- **L3** 原始证据目录（`evidence.json`、子进程进度）留在远端临时目录。建议拷回或在 `.chanlun/review-results/` 下留一份摘要（含各组合的 `final_tables_sha256`）。现在本机只有日志可查。
- **L4（待裁 4）** 生产代码没有跨域时间先后检查，在 TestOnly 时钟下可以接受。是否需要这样的不变式应另开票问，不在本片补。M1 的规范化会让测试至少把这一点钉成已知形状。
- **L5（待裁 7）** 范围限于本地事务进程层可以接受。BindVoice 的同类故障点不在 WE7 操作里，建议另开票，在声部链路的验收里覆盖。

## 对作者 7 个待裁点的意见汇总

| # | 意见 |
|---|---|
| 1 | 业务值没有被一起删掉；但时间应改为规范化核对，不要删除（M1） |
| 2 | 应当规范化后逐字等于参照回执，只有时间商与根不同（M1） |
| 3 | 需要扩到全部 21 步 × 2 阶段 × 2 次 = 84 次（M2） |
| 4 | 可以接受；用 M1 规范化钉住形状，跨域不变式另开票（L4） |
| 5 | 必须补：树内 (a)(b) + 一次性变异 (c)（M3） |
| 6 | 同意同路径 |
| 7 | 范围可以接受，BindVoice 另开票（L5） |

---

# 复审（codex/1406-s3-kill @ 8c276a3089，对比 fc42f24f40）

**结论：APPROVE。** 首审提的四条要求都已满足：时间规范化与商规则、84 次 kill 及 42 个组合的哈希、常驻的 (a)(b)，以及两次变异。没有残余 HIGH 或 MEDIUM。

**生产代码**：`git diff --stat 6c8477ff6f 8c276a3089 -- rust/src` 只列出 `mod.rs`（+2）和 `tb03c_kill_tests.rs`，生产代码零改动。本轮 `fc42f24f40..8c276a3089` 只改了测试文件和报告。

**抽跑**：`NC_TARGET=target-1406-c8review nc-remote codex/1406-s3-kill rust cargo test --features s_session --lib -- tb03c_kill_s06 tb03c_kill_s12 tb03c_kill_negative` 结果为 `4 passed; 0 failed`，用时 249.67 s，sha 8c276a3089，日志 `/tmp/nc-remote/20261021-011537-8c276a3089.log`。抽跑覆盖 E 侧 Capture fill-1、首审未覆盖的 Resolve fill-2，以及两条负控。

## 逐条

**M1 已关闭：时间按值比较，商规则来自时钟设计。**
- `normalize`（`tb03c_kill_tests.rs` 约 :212-250）对每个 `*_ns` 字符串值做三件事：取余数 r = v mod 1,000,000，替换原值后参与逐值比较；同时把商 q = v / 1,000,000 与 `Clock::expected_quotient(r)` 比对，不符就记入 `bad`；`compare` 只要 `bad` 非空就返回 Err。这说明余数和商都被实际比较了，没有被剔除。参照轨的商必须全为 0，否则 panic。
- 商规则与生产时钟设计一致，不是迁就测试输出：
  - `service.rs:686-688` 以 `(epoch - 1)` 为列号取 `received/first_known/commit_ns`，`store.rs:1549` 以 `(old - 1)` 取 `recover_ns[0] = 1,000,000`，`init_ns = 1`。
  - 测试 manifest 给消息 m-i 第 r 列的时刻为 `r·1e6 + 1000 + 10i + offset`（offset ≤ 2）。由余数可唯一反推出 i，期望商 =「处理 m-i 的域当时的 epoch−1」：被杀域从 `from` 起取 1，提交前被杀时 `from = target`，提交后被杀时 `from = target + 1`；其余为 0。余数 1 是 init，商为 0；余数 0 是 recover，商为 1；其余小于 1000 的余数一律报错。
  - B 里抄入的 E 时刻按 E 的处理归属判断。这与首审「该域 epoch−1」的写法相比，是更精确的同义表述，我接受。
  - 这条规则能抓住首审担心的情形：如果重送沿用旧的第 0 列时间，期望商为 1、实际为 0，会被判错。
- 剔除的键收窄为 `root`、`*_root`、`root_hash`、`image_hash`、`writer_epoch`。根链一致性由 `audit_all` 核对，被杀域的 `epochs` 表另有逐字断言。
- 提交前被杀后重送的回执：`assert_ne!` 原始字节，同时 `compare(...) == Ok(())`，`seq == kill_seq + 1`，满足首审对待裁 2 的要求。B 被杀时，未被杀的域仍做逐字节比较。

**M2 已关闭：84 次 kill，42 个组合。** `kill_step!` 宏生成 21 个分步测试，覆盖 E 的 Capture×6、Resolve×3，以及 B 的 Apply×12。我核了作者日志 `/tmp/nc-remote/20261021-002009-a7f852c7a6.log`（sha a7f852c7a6，`23 passed; 0 failed; 1 ignored`，1149 s）：
- `^S3-KILL` 共 84 行，`"signal":9` 共 84 处；
- 按 (label, phase) 分组得到 42 组，每组恰好 2 次且 rep-0 与 rep-1 的 `final_tables_sha256` 相同，42 个哈希两两不同；
- 报告中的 42 行表格与日志一致（抽查若干行）。

报告说明的哈希与首审不可比，原因是目录名进入了 `manifest_hash`，解释成立。

**M3(a) 已关闭。** `kill_and_recover` 在 kill 之前对每个目标步断言 `snapshots[target][owner] != snapshots[target+1][owner]`，21 步都会执行这条断言。

**M3(b) 已关闭。** `skip_resend` 在提交前被杀并恢复后，跳过重送直接续跑，对与正向路径共用的 `final_diff` 断言 `expect_err`，没有用 `should_panic`，并且断言差异落在被杀域（`终态域 0：` / `终态域 2：`）。作者日志中的 `S3-NEGATIVE` 两行显示：步 6 续跑出现 12 个 Error，第一处差异在 E 的 `/commits`；步 9 续跑出现 2 个 Error，差异在 B2 的 `/applications`。我的抽跑中两条负控也都通过。

**M3(c) 已关闭：失败确实由边界断言触发。**
- 变异提交 `a9c3225f5c`、`177328230f` 在本地仍可读。`git diff a7f852c7a6 a9c3225f5c` 只改 `service.rs` 三处，把 `before_commit` 的 pause 挪到 `tx.commit()` 之后；另一个提交同理挪动 `after_commit`。两者相对 a7f852c7a6 都只改 `service.rs`（+6/−3）。
- `20261021-004703-a9c3225f5c.log`（ref `tmp/1406-s3-mut-before`）：s00、s01、s07 三个测试全部 FAILED，panic 位置都在 `tb03c_kill_tests.rs:733:9`，报错为 `before_commit 被杀后域 0/1/0 业务表`。
- `20261021-004919-177328230f.log`（ref `tmp/1406-s3-mut-after`）：before 阶段各两次 S3-KILL 都通过，因为这一阶段没有被变异；after 阶段在 `:733:9` 失败，报错为 `after_commit 被杀后域 1/0/0 业务表`。
- 我核了 a7f852c7a6 中 `:733` 一行，它正是 `business(&at_kill[d]) == plan.snapshots[boundary][d]` 这条边界断言，不是别的断言碰巧失败。变异测试覆盖了 Capture、Resolve、Apply 三条路径。

## LOW（不阻塞）

- **L1**：全部 kill 测试串行累计约 1149 s。如果放进 CI 常驻门，需要评估时长；必要时可以按步分片或放进夜间任务。
- **L2**：负控只覆盖步 6（E）和步 9（B）。由于比较函数是共用的，这已经足以证明它能区分「重送」与「漏送」，不要求扩展。
- **L3**：首审的 L1–L5 都已在报告中回应。其中跨域时间先后不变式（L4）和 BindVoice 故障点（L5）仍需主控另开票。原始证据目录仍留在远端临时目录，本地只有日志和报告中的哈希表作为摘要。
