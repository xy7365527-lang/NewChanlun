# #1818 / PR #1826 41O F1 窄修订报告（layout-targets41q）

时间（date -u）：2026-11-01T00:41:12Z

## 结果

- PR：https://github.com/xy7365527-lang/NewChanlun/pull/1826 （OPEN、Draft、MERGEABLE）
- 新精确 head：`163fb363a290ecf210a265058f1af71105ca73ce`，普通追加提交，父为 `a2cf513c25c45f1e835d7732425bdf024dd79b7f`；未 force、未 rebase。开工前核 HEAD = origin 分支 = PR head = `a2cf513c25`，工作区干净。
- 工位与分支：`/Users/silencehan/.codex/worktrees/layout-publish41e/NewChanlun`，`codex/1818-adr0059-publish`。
- 只改 `docs/adr/0059-production-session-layout-contract.md`（+33/−18）。PR 正文同步，正本 `pr-body.md` 在本目录，改前原件 `pr-body.before.md`。

本轮写目标：上述 ADR 一份、PR #1826 正文、本目录 `REPORT.md`、`pr-body.md`、`pr-body.before.md`。未写实现、测试、runbook、System 工位、票或 main。

## 前提与核法

两轮漏项共用的前提是「写足迹由库路径的目录决定」。41I 证伪了它的父目录版本，41O 又证伪了它的目录版本：锁路径本身的最终链接会把创建带走。本轮不再从反例倒推，先把现役并发写者 E/B/X 由库 d 派生的每个创建调用读到落点，再据此改规则。

## 修订内容

- **裁定第 3 条**：判定目录改为三类。`Q₁ = canon(parent(d))`、`Q₂ = parent(canon(d))` 不变；新增锁目标目录 `Q₃ = parent(canon(L))`。锁路径 L 按 `process_lock` 分支推出：`L₂ = canon(d)` 接 `.writer.lock` 恒取；d 不存在（跟随链接判存在，悬空算不存在）时另取 `L₁ = Q₁/basename(d)` 接 `.writer.lock`。L 非链接时 Q₃ 落回 Q₁ 或 Q₂，所以原有各验收判决不变。
- **为什么 d 不存在时两个锁路径都要判**：launcher 在 d 不存在时先跑 E/B init，init 走不存在分支（L₁）并建库，随后 serve 走存在分支（L₂）；X 首启走 L₁ 并建库，再启走 L₂。d 存在时 L₁ 从不被打开，故不取。
- **第 4 条**拒绝条件作用于 Q₁、Q₂ 与每个 Q₃；**第 8 条**错误信息标明命中的是哪个判定目录。
- **Q2** 改写为按创建调用逐条推导：锁文件、主库、SQLite 侧件三类，各带源码出处；末尾写明本条只覆盖库派生路径。**Q8** 补锁路径同用 canon 语义及反例 `/abs/x/x.db.writer.lock → P/x-external-lock`。
- **考古冲突第 8 条**：记 41O 来源、同一前提两次漏项，以及三类不纳入的非库派生足迹。第 7 条的验收编号订为 9、10、13。状态行同步。
- **验收**：新增 11 `layout_rejects_writer_lock_symlink_into_s_parent`（41O 反例：安全目录内既存普通库 `x.db`，`x.db.writer.lock` 悬空指向 `P/x-external-lock`；核 `InvalidDomain`、`x.db`、`s.db`，P 项集不变、目标未建）；新增 12 `layout_rejects_absent_db_writer_lock_symlink_into_admission_state`（d 为悬空链接指向安全目录，Q₁、Q₂、L₂ 的 Q₃ 都安全，只有不存在分支的 L₁ 指向 A 内，能区分「只算 L₂」的错误实现）。原 11–21 顺延为 13–23；安全链接正控现为 13。锚句加 `锁目标目录`。外部旧引用「验收 17/18」现为 19/20。
- **保持不变**：原同目录、目录链接、A 子树、S 族、不存在父目录规则与正控；`s.socket`、launcher `state_dir`、S admission、隔离与 socket 范围均未动。

T/P：仍是既有 InvalidDomain 规则的输入口径对齐现役写者，无新政策、无开关，方向只多拒，未超出 T。

## 源码覆盖清单

### 已证（逐段读源码）

| 足迹 | 落点 | 依据 |
|---|---|---|
| 并发写者的 `process_lock` 调用点 | 只有 X `rust/src/bin/x_execution_session.rs:176` 与 E/B `rust/src/economic_session/service.rs:1569`（`e_economic_session.rs`、`b_chong_session.rs` 均进 `economic_session::cli`，`mod.rs:28`）。其余调用在 S 自身（`s_session_v2/mod.rs:2166`、`s_session_v3/mod.rs:537`、`s_structure_session.rs:2267`）与测试 | `rg process_lock rust/src` |
| 锁文件 | 存在分支 `canonicalize(d)`+`.writer.lock`，不存在分支 `canon(parent(d))/basename(d)`+`.writer.lock`；create=true 普通打开，无 `O_NOFOLLOW`，跟随最终链接 | `session_transport.rs:43-63` |
| E/B 锁路径与 launcher d 等价 | E/B 用经济 manifest 库路径，`canonical_target`（同分支法）须与 CLI d 相等，否则 IdentityConflict | `store.rs:292-294`、`:448-458` |
| init 与 serve 分支切换 | d 不存在时 launcher 先 init E/B，再拉起 | `production_session.rs:341-362`、`:455`、`:467`、`:490` |
| 主库 | 打开不带 `SQLITE_OPEN_NOFOLLOW`（E/B `store.rs:529-532`；X `x_execution_session.rs:177`，rusqlite 默认标志 `rusqlite-0.40.2/src/lib.rs:1256-1266`）；SQLite 逐级解析全部链接含悬空最终链接（`sqlite3.c:47110-47166`、`:47195-47220`，`HAVE_READLINK`/`HAVE_LSTAT` 于 `:177-178`）；pager 接受解析结果（`:64397-64405`）。建库于 `canon(d)`，目录 Q₂ | 内置 SQLite 3.53.2，libsqlite3-sys 0.38.2（`rust/Cargo.lock:838-839`） |
| `-journal`、`-wal`、`-shm` | 由解析后路径拼名（`sqlite3.c:64517-64536`、`:45186-45192`），全部 `O_NOFOLLOW`（`:46855-46856`、`:45220`）。只落 Q₂；侧件名处预置链接只致打开失败 | 同上；E/B WAL `store.rs:539`，X 无 journal pragma |
| `-mj` 超级日志 | 只在多库事务出现；E/B 自述无 ATTACH（`service.rs:1038`），X 与 E/B 非测试文件 grep 无 ATTACH | grep，有限 |

### 已证存在、本裁定不纳入

1. **E/B socket bind**：`session_transport.rs:635` 在经济 manifest 给出的 socket 路径（`store.rs:296`）建目录项。若 `e.socket` 或某 `chongs[].socket` 放进 P、A 或 S 族条目，会在 S 打开窗口内改 stamp，与库同型竞态。本条属 socket 范围，任务禁止改，需主控另裁。建议：作为 #1818 的后续 T 类小票，判据可复用本 ADR 的 Q 集合（socket 的 `canon(parent(socket))`），launcher 需读经济 manifest。
2. **TestOnly 故障标记**：`store.rs:395-406` 以 `create_new` 在 `marker_path` 建文件，只在经济 manifest 时钟为 TestOnly 且配了 `test_only_fault` 时发生。ADR 0056:53 称 launcher 拒 TestOnly 时钟，但本轮在 `production_session.rs` 未找到该检查落点，launcher 路径能否走到标记写入属未知。
3. **SQLite 临时文件**：目录候选 `/var/tmp`、`/usr/tmp`、`/tmp`、`.`（`sqlite3.c:46449-46456`）；launcher 的 `clean`（`production_session.rs:257`）清空环境，`SQLITE_TMPDIR`/`TMPDIR` 不传入。只在 P、A 或 S 族条目恰为该临时目录时相关；服务期是否产生临时文件未核。
4. **TestOnly 传输诊断** `ReadTrace`：`session_transport.rs:159-170` 以 `create_new` 写诊断文件，标为 TestOnly；路径来源未追。

### 未知（未读）

1. E 金融读路径会拉起 S 读子进程（`finance_transport.rs:179`、`:286`）。该子进程是否非 immutable 地打开 S 库、从而在 P 建 `-shm`，本轮未读。若是，它是 S 族条目的创建，不在库布局规则内，影响 S admission 同一窗口。
2. 创建调用 grep 只覆盖 `rust/src/economic_session/` 非测试文件、`session_transport.rs`、`x_execution_session.rs`、`production_session.rs`。全仓 grep 另有 `c_segment_verify.rs`、`recursive_t/`、`trading/` 与若干研究 bin 的写文件调用，本轮未追它们能否从 E/B/X serve 到达。
3. 目标平台语义：Rust `OpenOptions` 在 Linux 与 macOS 都跟随最终链接、悬空链接加 `O_CREAT` 会建目标，属 POSIX 语义推论，未运行验证。

## 实际验证

- `git diff --check` 通过；`git merge-tree --write-tree origin/main HEAD` 无冲突（origin/main `71df2ae749664714e6bff10f8f71d9e15cc4c815`）。
- 验收 19 后缀计数 `1`；验收 20 十条锚句 `rg -c -F` 计数 3/10/2/2/2/3/2/2/4/3。
- 新增与改写处引用的行号都在本轮逐段读过：仓内 `session_transport.rs:43-63`、`:635`，`store.rs:292-296`、`:395-406`、`:448-458`、`:529-539`，`production_session.rs:257`、`:341-362`、`:455-490`，`x_execution_session.rs:171-178`，`service.rs:1038`、`:1561-1569`；仓外 `sqlite3.c` 与 `rusqlite` 行号读自 `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` 下对应版本。
- 未跑 Rust、实验或 CI，未等待 CI。push 时本机 pre-push 提示本地 main 落后 origin/main，属本地引用陈旧提示，推送成功。

## 待独审与剩余

候选 head `163fb363a290ecf210a265058f1af71105ca73ce`，对 `a2cf513c25` 的差异即本次修订。作者为本子代理（Claude 家族），独审宜交非作者 gpt-6-astra。重点：Q₃ 与 L₁/L₂ 是否完整对应 `process_lock` 两分支及 init→serve 切换；SQLite 侧件「无须另判」的源码依据是否成立；验收 11、12 是否二值可执行且能区分只算 L₂ 的实现。

剩余需主控处置：E/B socket bind 足迹是否另开票（上文不纳入第 1 条）；TestOnly 时钟拒绝的实现落点核实；合后动作（#1818 关票、#1781 范围、#1323 Notes）仍归主控。
