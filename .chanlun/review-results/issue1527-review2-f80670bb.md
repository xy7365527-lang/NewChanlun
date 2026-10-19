# #1527 f80670bbf9 第二轮非作者独立评审

- 评审对象：`f80670bbf9`（fd_wal.rs 三处 + generation.rs 一处），worktree `/Volumes/AgentStorage/issue1527-owner-20261019`
- 方式：只读静态阅读；SQLite 依据为本仓锁定的 `libsqlite3-sys 0.38.2` 内置 amalgamation（`SQLITE_VERSION "3.53.2"`，下文 `sqlite3.c:N` 均指 `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libsqlite3-sys-0.38.2/sqlite3/sqlite3.c`）。未运行 cargo，未运行任何测试。
- 首轮 review-f80670bb.md 未采用。fd_wal.rs:321-322 确实在所有分支前先执行 `buf.fill(0)`，short-read 合同成立。

## 结论：PASS-with-notes

四个问题都没有找到确凿的数据丢失、准入绕过或可利用的授权缺口。问题 3 的新放行面比注释写的「count(*)」更宽，而且没有单测锁，列为 note，不构成 BLOCK。

---

## 问题 1：删去 `ctx.wal.is_some()` 后，残留或投放的 -wal 会不会被静默忽略

**结论：不会。** 能导致忽略的两个时间窗都在更早处或更晚处被拒绝。

1. **测量时已经存在的 -wal 不会被判成 MainOnly。** `admission_provider.rs:126-185` 会完整枚举 DB basename 家族。只要 `-wal` 是成员（零长度也算），就有 `wal_index = Some`（:166-168），介质判为 `Medium::Wal`（:181-183），随后 `ctx.wal = Some(Source::pin(..))`（fd_wal.rs:901,912），读取的是真实 FD。`-shm` 存在、或 journal 与 WAL 同时存在时会直接拒绝（:176-178）。`Medium::MainOnly` 只在「测量时没有 -wal 成员」时成立（:184-185）。
2. **测量后新出现的 -wal 会在审计之后被拒绝。** 准入在测量前已经持有 writer 锁和 `.maintenance.lock`（admission.rs:136-140），并发写者无法合法产生新的已提交帧。审计结束后，`verify_before_open`（admission_provider.rs:299-343，admission.rs:153-159）会重新枚举家族；成员集变化时返回 `SourceMembershipChanged`，准入失败。
3. **VFS 不按路径访问磁盘。** `open()` 只借用 Registry 固定持有的 FD（fd_wal.rs:247 注释与 :249-258），`access()` 对 WAL 名仍返回 `ctx.wal.is_some()`（:536-537，本 commit 未改）。所以 MainOnly 会话内，即使磁盘上冒出 -wal，SQLite 也看不到它的字节；它只会看到一个长度为 0 的空 WAL。

**MainOnly 下 SQLite 为什么还要打开 WAL：** `access()` 报告 WAL 不存在，`pagerOpenWalIfPresent` 不会打开（sqlite3.c:62950-62967）。但主库头 `page1[19]==2`（WAL 头库）会让 `lockBtree` 调用 `sqlite3PagerOpenWal`（sqlite3.c:76477）。独占模式下 `sqlite3PagerWalSupported` 为真（sqlite3.c:67154-67157），于是会请求打开一个 WAL。旧代码对这次打开返回 `SQLITE_CANTOPEN`，导致 MainOnly 的 WAL 头库无法审计；新代码接受这次打开，并让它表现为空 WAL。打开条件仍然要求名字精确等于 `wal_name`、类型位为 `SQLITE_OPEN_WAL`、`exclusive_armed` 和 `main_open` 都已成立，并且只能打开一次（fd_wal.rs:214-243）。

**判定：** 不会丢失已提交数据，也不会掩盖篡改。存在 -wal 时会走 Wal 介质真实读取；-wal 后来出现时由成员复核拒绝。

## 问题 2：ctx.wal 为 None 时空 WAL 各回调的返回值与副作用

| 回调 | 返回 | 位置 |
|---|---|---|
| open 输出 flags | `SQLITE_OPEN_READONLY` | fd_wal.rs:261-264 |
| read | 已清零后返回 `SQLITE_IOERR_SHORT_READ`（新分支） | fd_wal.rs:321-322, 332-337 |
| size | `*out=0`，`SQLITE_OK`，并记录 `size-absent-wal`（新分支） | fd_wal.rs:416-422 |
| write | `SQLITE_READONLY` | fd_wal.rs:395-402 |
| truncate | `SQLITE_READONLY` | fd_wal.rs:403-405 |
| sync | `SQLITE_OK`（只读 noop） | fd_wal.rs:406-408 |
| xDelete | `SQLITE_READONLY` | fd_wal.rs:565-569 |
| shm | VFS/io methods 为 `iVersion: 1`（fd_wal.rs:498,931），没有 xShmMap；open 只接受 MAIN/WAL，`-shm` 一律 `open-denied`（:209-221），access 报不存在（:538-540） | — |

在只读审计路径上，这些回调都不会产生写盘副作用：

- 因为 open 回报 READONLY，WAL 被标为 `WAL_RDONLY`（sqlite3.c:69164）。连接同时设置了 `query_only=ON`（fd_wal.rs:1050），不会写 WAL 头或帧。
- 独占模式使用堆内存 wal-index，不需要 shm。恢复逻辑只在 `nSize>WAL_HDRSIZE` 时读取 WAL（sqlite3.c:68857）；size=0 时根本不读，新的 read 分支只是防御性兜底。即使被调用，全零头也会因为 magic 不符而被当作空 WAL。
- 关闭时设置了 `SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE`（fd_wal.rs:995），`sqlite3WalClose` 拿到的 zBuf 为 0，不做 checkpoint，也不会进入删除分支（sqlite3.c:69957-69999）。即使走到删除分支，`sqlite3OsDelete` 的返回值也被忽略（:69999），READONLY 不会变成关闭错误。这一点与本 commit 前 `Medium::Wal` 的既有路径完全相同。

**判定：** 安全，没有副作用。

## 问题 3：audit_sql_allowed 新增的 `db.is_none() && b == Some(b"")` 放行

**SQLite 何时上报这种组合：** 3.53.2 中，空列名 `SQLITE_READ` 只来自 select.c 的「引用了表但没有取任何列」路径（sqlite3.c:156815-156842，典型例子是 `SELECT count(*) FROM t1` 和 `SELECT t1.* FROM t1, t2` 里的 t2）。此时：
- 第 5 参数 `zDb` 等于 `pItem->u4.zDatabase`，也就是 SQL 文本里写的 schema 限定词。**表名不带限定时它为 NULL**。
- 如果 FROM 项是子查询，`zDb=0`。
- `fixedSchema` 的项（来自视图、触发器 DDL 或 FK 生成的 SrcList，sqlite3.c:126014,138613,138639）会给出真实库名。

真实取列的读取走 `sqlite3AuthReadCol`，库名总是真实的 `aDb[iDb].zDbSName`（sqlite3.c:126250-126254），因此不会因为新分支而带着 `db=None` 进来。

**放行面：** 前置条件 `origin.is_some() → false`、`db ∈ {None, main}`（fd_wal.rs:654-656）没有改变，所以视图/触发器内部访问依旧被拒。新分支对表名 `a` 没有限制，确实会放过不带限定词的 `sqlite_schema`/`sqlite_master`，以及解析到 temp 库的 `sqlite_temp_master`/`sqlite_temp_schema`/temp 表的「零列引用」。但这种授权只允许 SQLite 访问表结构本身，不允许取出任何列值；最多泄露行数、存在性或连接基数。只要真正读取列值，就会以 `db="temp"`（或附加库名）重新经过授权，并被前置条件拒绝。ATTACH 不在白名单中（`_ => false`，:692），所以不存在附加库；连接为 `query_only`、`temp_store=MEMORY`，且不允许 CREATE，temp 库只剩空的 schema 表。

**审计 SQL 来源：** 授权器只装在私有审计连接上（fd_wal.rs:1017-1024），这个连接只执行 `PRAGMA quick_check`（admission_private/mod.rs:284-286）以及 `audit_business`→`state::audit_with_budget`／`v2::audit_private_startup`（mod.rs:198-218）。这些路径中的 SQL 都是编译期常量；少数 `format!` 拼接的 SQL，表名和列名来自源码中的常量数组（如 s_session_v2/staged_nodes.rs:375-385），其余都在 `#[cfg(test)]` 中。外部输入无法影响 SQL 文本。

**判定：** 不可利用。授权器在这里是兜底锁，用来防止「审计代码将来误写了越权 SQL」，而外部输入不能注入 SQL。

**Notes（建议，非阻断）：**
1. 注释写「count(*) 优化路径」，但这个分支同样覆盖 `t1.* FROM t1,t2` 中未取列的表和 FROM 子查询项，范围比注释宽。
2. 本 commit 没有为新分支增加 `authorization_tests` 锁（:742 起的测试本身也标了 NOT_RUN）。
3. 最小收紧方案：对表名加白名单，或至少排除 temp schema 名，例如
   ```rust
   || (db.is_none() && b == Some(b"") && !matches!(a, Some(b"sqlite_temp_master" | b"sqlite_temp_schema")))
   ```
   更严格的做法是只接受审计实际 count 的业务表名集合。另外可以补一条正例（`a=s_content_nodes, b="", db=None`）和一条反例（`db=Some("temp"), b=""` 仍拒）作为源码锁。

## 问题 4：generation.rs oracle 改走 `advance_business`

改动位于 `generation.rs` 的 `#[cfg(test)] mod tests`（:2181 起）中的 `OracleDb::advance`（:4172-4185），生产代码不受影响。

- `cmd_advance` = `advance_core(db, epoch, None)` + `v3::admission::write_current_result(&out)`（s_structure_session.rs:3074-3077）。
- `advance_business` = `advance_core(db, epoch, None).map(|_| ())`（s_structure_session.rs:5591-5593，位于 `#[cfg(test)] mod tests`）。

两者都完整执行 `advance_core`，其中的业务拒绝路径全部保留，包括 writer_epoch、规则修订、`advance_state` 非 idle 的 StaleWriter、模式登记与 profile 绑定等（:3086-3125）。唯一省掉的是 `write_current_result`：它要求存在启动准入 scope（admission.rs:338-342，缺失时返回 `StartupScopeMissing`），并负责写结果和 `finish_startup`。这是生产准入的输出端，不属于业务判定。oracle 只读取持久态（:4188 起），原本就没有建立准入 scope。生产入口 `cmd_advance` 没有被修改。

**判定：** 没有改变任何生产拒绝路径，也没有让生产流程绕过准入。小提醒：`write_current_result` 的覆盖需要由准入侧测试单独承担，与本 oracle 无关。
