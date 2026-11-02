# ADR 0059：生产会话布局合同（S admission 测量域与并发写域库目录）

> 正本：本文件是 #1818 `production_session` 布局合同的正本，涵盖 S admission 测量域 M 的边界、并发写域库目录的启动前拒绝规则、路径比较语义与 owner 落点。
> 最后裁定：#1818（2026-11-01，T 类，依 #1323 Notes N-1 预授权判定）
> 受影响代码清单：`rust/src/bin/production_session.rs:125`（`load_manifest`，拒绝检查落点）；`rust/tests/production_session_entry.rs`（验收测试）；`docs/runbook_production_session.md:35`、`:40`、`:53`、`:59`（runbook 订正）。实施归 #1781。

**状态**：T 类裁定。原裁定经两轮审查，第二轮 PASS。发布审查（PR #1826）四轮发现并发写足迹漏判：库文件的符号链接、锁文件的符号链接、E/B 服务期 bind 的 socket、该 socket 的最终链接。Q2、Q4、Q8 与裁定第 1、3、4、6、8、9 条及验收 9–21 已据此订正，订正部分经第五轮非作者增量独审（41W，精确 head `8ce09b7b`）PASS，评审链存于 `.chanlun/review-results/issue1818-decision-20261101/`。本 ADR 只定规则与判据；实现与运行证据归 #1781，不以本 ADR 声称实现完成。

## 问题

S 写者打开库之前，要严格核验 admission 测量域 M（下文定义）中每个成员的完整 stamp，并核对成员集合本身（`rust/src/bin/s_session_v3/admission_provider.rs:299-345`；stamp 比对在 `Pinned::verify`，`:74-88`，严格模式不符即 `SourceIdentityChanged`，集合不符即 `SourceMembershipChanged`，`:342`）。打开以后，主库和目录只核身份（`:348-360`）。launcher 拉起 S 后不等待，紧接着拉起 E、各 B 和 X（`rust/src/bin/production_session.rs:455`、`:467`、`:490`）。这些域各自持有 SQLite 库和 `<db>.writer.lock`（`rust/src/bin/x_execution_session.rs:3`，ADR 0039:300）。它们在服务阶段会在库所在目录里建删侧件。只要某个并发写域的库目录落进 M，S 就会在打开窗口内间歇拒启。PR #1773 独审 M-2 记录了远端复现：5 轮中 3 轮失败（`/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1773-INDEPENDENT-REVIEW-6C6E.md:55`、`:84`）。目前 launcher 和 runbook 对这种布局都没有约束（同文件 `:51`；runbook:21-24 只列库名，没有目录关系要求）。

**M 的代码事实**（`admission_provider.rs:206-255`，其中 `walk` 在 `:256-281`）：
- `parent(db)` 目录本身；
- 该目录内 `db` basename 族的条目，递归枚举。族条目指名字等于 basename，或以 basename 开头且下一个字节是 `.` 或 `-` 的条目；
- admission state 目录 A 的整棵子树，递归枚举。

枚举用 `symlink_metadata`，不跟随符号链接；既不是文件也不是目录的成员即报 `SourceTypeUnsupported`（`:278-279`）。S 侧的 `db` 已经 canonicalize 过（`admission_anchor.rs:1293-1294`）。A 是 owner 原件的 `state_dir`，CLI `--admission-job-limits` 必须恰好等于 `A/s-admission/limits.json`，否则报 `Identity` E1（`admission_anchor.rs:1327-1342`，常量见 `:30`、`:32`）。

**一个嵌套反例**：A 就是 `parent(s.db)` 本身，E 的库放在 `parent(s.db)/e/e.db`。按 `:250-253`，A 等于 parent 时要枚举 parent 的全部子项，所以 `e/` 及其中的侧件都进入 M。可见冲突条件是「并发写域落入 M」，而不只是「父目录相等」。

**考古标出的冲突（本稿逐条处置）**：
1. 候选稿把 #1567 评论 6267636964 当作「resolution」引用。该评论首行写着「P 类影子模式，待用户批复」，后来被重写的签署单取代。用户实际批复的是 6310847303，落地正本是 ADR 0046（6311316288、6315085658）。本稿改引 ADR 0046:20-21 和 ADR 0028:46。
2. PR #1773 独审 M-2（`:84`）的「更稳」方案要求同时比较 launcher `state_dir`、拒绝 socket 同目录。代码事实不支持把这两项列进拒绝规则，见裁定第 5、6 条。本稿不采纳这两项。
3. runbook:40 的 `<state>/s-admission/limits.json` 和 :59 的 `state/s-admission/...` 指的是 A。runbook:35、:53 的 `state_dir` 指的是 launcher manifest 顶层字段。两者同名但不是一回事，属文档歧义。（候选稿写的「:59」正确，考古写的「:58」是空行前一行，以 :59 为准。）
4. 候选稿说「残留 socket 由 walk 报 `SourceTypeUnsupported`」，#1823 正文也这么写。实际代码顺序是：`ServeOptions::parse` 在 `writer_cli.rs:393` 调用 `socket_path`，路径已存在就报 `TransportUnavailable：socket路径已占用`（`:357-359`）。这一步早于 driver open（`:725`）。所以残留 socket 会先被这条确定性拒绝挡下，轮不到 admission walk。
5. 候选稿验收 7 把原八项夹具说成「各自独立目录」。实际夹具是 P=`<tmp>/s`、A=`<tmp>/s/s-state`（A 在 P 之下）、E/B/X 库在 `<tmp>`（`production_session_entry.rs:74-80`、`:192-195`、`:348-357`）。本稿把这一条拆成两条正控。
6. 候选稿写「#1781 实施排在 FU-16 合入之后」。#1538（FU-16）已关票，正本 ADR 0056 已在 main，这句话已过期。真实顺序约束是既有边 #1769 → #1781。
7. 原稿把并发写足迹记为 `parent(d)` 内的建删，只取 `canon(parent(d))` 作判定目录，漏了库文件本身的符号链接。现役写者先解析整个库路径再建锁（`rust/src/session_transport.rs:43-62`），所以安全目录内指向 P 内库的文件链接会把 `.writer.lock` 建进 P（PR #1826 发布独审 F1，`/Volumes/AgentStorage/layout-review41i/REPORT.md`）。本稿订正 Q2、Q8 和裁定第 1、3、4、8 条，补验收 9、10、19。
8. 第 7 条的订正仍只判了目录，没判锁文件路径本身。`process_lock` 打开锁路径时跟随其最终链接，安全目录内预置的 `x.db.writer.lock → P/…` 悬空链接会把锁文件建进 P（PR #1826 独审 41O F1，`/Volumes/AgentStorage/layout-delta41o/REPORT.md`）。同一前提两次漏项，本稿不再补单个例子，改为按现役创建调用逐条推出实际落点（见 Q2），订正裁定第 3、4、8 条，补验收 11、12。（本文 `sqlite3.c` 指 `rust/Cargo.lock:838-839` 锁定的 libsqlite3-sys 0.38.2 内置 SQLite 3.53.2 源码。）
9. 第 8 条的订正稿把 E/B 服务期 bind 的 socket 列为「非库派生路径、留待另裁」，并把结果限定为库派生路径。这一缩窄没有依据：原 Q4 只排除 `s.socket`，理由是 S 自己在 driver open 之后才 bind；E/B 由各自进程 bind，不等 S，同样在 S 打开窗口内改 P 的 stamp（PR #1826 独审 41R F1，`/Volumes/AgentStorage/layout-review41r/REPORT.md`）。本稿撤回该缩窄，把 E/B 实际 socket 的创建目录纳入同一判定（Q2 第四类，裁定第 3、4、8 条），补验收 13–16、20、21；`s.socket` 例外不变（Q4、裁定第 6 条）。

   已知但本裁定未纳入、也未定性为反例的足迹，照实留档：
   - TestOnly 故障标记：`store.rs:395-406` 以 `create_new` 在 `marker_path` 建文件。`store.rs:268-270` 只在经济 manifest 时钟为 TestOnly 时接受故障调度；ADR 0056:53 要求 launcher 拒 TestOnly 时钟，但该检查在 `production_session.rs` 的落点未核。
   - SQLite 临时文件：候选目录 `/var/tmp`、`/usr/tmp`、`/tmp`、`.`（`sqlite3.c:46449-46456`）。launcher 以 `clean`（`production_session.rs:257`）清空环境；服务期是否产生临时文件未核。
   - E 金融读路径拉起的 S 读子进程（`rust/src/economic_session/finance_transport.rs:179`、`:286`）是否非 immutable 地打开 S 库，未核。
   - 创建调用的 grep 只覆盖 `rust/src/economic_session/` 非测试文件、`session_transport.rs`、`x_execution_session.rs`、`production_session.rs`；其他模块的写文件调用能否从 E/B/X serve 到达，未追。

   以上不是已穷举的清单。

10. 第 9 条的订正把 socket 判定目录只取 `canon(parent(k))`，理由是「bind 不跟随最终条目」，并写成所有 Unix 相同。这一跨平台断言是错的：Apple 发布的 XNU 源码中 bind 跟随最终链接，悬空链接会在目标父目录创建 socket（PR #1826 独审 41T F1，`/Volumes/AgentStorage/layout-delta41t/REPORT.md`）。本稿保留 `Q₄`，另取 `Q₅ = parent(canon(k))` 并入同一拒绝集合（Q2 第四类、Q8，裁定第 3、4、8 条），补验收 17、18；不加平台分支、开关、socket 清理或启动排序。

## 权威依据

- **launcher 拒启先例**：`load_manifest` 自述「完整校验 manifest；此阶段不创建任何域文件」（`rust/src/bin/production_session.rs:124-125`）。既有 `InvalidDomain` 拒绝都在拉起任何域之前：namespace 相交（`:147`）、`economic.b` 为空（`:172`）、X profile mode 不一致（`:179-184`）。runbook:51 写的是「在拉起 S、创建 S 库之前退出」。ADR 0056:53（§五.2）：「`production_session` 遇到 `TestOnly` 时钟，或时钟 kind 与 `evidence_class` 不符，在拉起任何域之前以 `InvalidDomain` 退出。」
- **独审建议**：PR #1773 独审 M-2，见 `1773-INDEPENDENT-REVIEW-6C6E.md:51-55`、`:84`，建议「launcher 校验……否则 `InvalidDomain` 拒启」。L-2 见 `:49`、`:88`，结论是「只会误拒不会误放」，S 自核留作后续。
- **改写 s.db 与已裁合同冲突**：ADR 0028:46 的失效触发包括「launcher 二进制/OS 身份/DB 路径变更」，失效即「拒绝发送，不回退旧 writer」。ADR 0046:20 规定「不存在运行时自动接管」，:21 规定「换库锚由 R4 现有身份检查拒绝」。代码层面，owner 原件 `db`、锚 `db` 都必须等于 canonical `db`，否则报 `Identity`（`admission_anchor.rs:1337-1342`）。
- **签发不建目录**：ADR 0050:44 写明「签发不建立数据库和 state 目录」「路径每级无跟随」。S 的目录布局由运维给出，S 不会替运维迁移。
- **不设可调字段**：ADR 0041:11 说「同一判断的松紧两个版本不是自由度，事前单选也不得落为可调字段」。ADR 0041:13 是判别句。另见 CONTEXT.md:393 和 AGENTS.md:46-48（#799 收敛通则）。
- **分级依据**：`.sandcastle/docs/graph-engineering.md:23`，T 类涵盖「接口与 seam、owner 落点……测试锁……恢复语义」。

## 候选

**Q1 布局冲突的处置**：
- **A 启动前拒绝**：默认值（出处：M-2 `:84`、`production_session.rs:124-200` 先例、ADR 0056:53 同款），确认或否决。推理链如下。M 包含 `parent(s.db)` 本身，打开前严格比对 stamp。E/B/X 在 S 之后无等待拉起，并在库旁建删侧件，所以同目录或嵌套布局必然间歇失败。布局是静态事实，可以在任何 init 之前确定性判出。这与 launcher 既有的拒启模式同型。它只会多拒，不会多放：S 自己的严格核验仍然在。另外它也覆盖「运维单独重启 S，而 E/B/X 仍在跑」的情形，因为布局不变式与启动顺序无关。
- **B 静默改写 s.db 或自动迁移到子目录**：排除。理由是违反已裁推论：ADR 0028:46 把 DB 路径变更定为失效触发，ADR 0046:20-21 禁止自动接管、要求换库即拒。改写后 owner 原件里的 `db` 不再等于 CLI 的 `db`（`admission_anchor.rs:1337`），S 一定拒启。
- **C 启动排序**（等 S 打开后再拉 E/B/X）：不采用，留档记录。它只能消除本 launcher 进程内的竞态，覆盖不了单独重启 S 的情形；还会把 E/B/X 的启动绑到 S 就绪上。它不违反已裁，只是收益窄。
- **D 部署前显式生成布局的工具**：保留为未来候选，本票不纳入，也不因本裁定被排除。

**Q2 冲突判定集合**：默认值为「并发写域的任一判定目录 Q 落入 {P} ∪ A 子树 ∪ S 族条目子树即冲突；判定目录取库 d 的目录项所在目录、d 解析后实际库文件所在目录、该域实际打开的锁路径解析后所在目录（锁目标目录），以及 E/B 实际 bind 的 socket 的创建目录（socket 目录）」，P 的祖先、兄弟目录、非族且非 A 的子目录都不算（出处：`admission_provider.rs:206-255`、`:299-345`；`rust/src/session_transport.rs:43-63`、`:635`；`sqlite3.c`；`rust/src/economic_session/store.rs:281-296`），确认或否决。推理链：在某目录里建文件只改该目录自身的 mtime，不改其父目录的 mtime。所以只有 Q 本身是 M 的成员目录时，才会触发 stamp 或集合变化。写者在哪个目录建删，由每个创建调用实际落到的路径决定，不由 manifest 字面路径决定。已核的现役并发写者 E/B/X 在 S 打开窗口内的创建调用有下列四类，前三类由库 d 派生，第四类由经济 manifest 的 socket 字段决定；已知未核的足迹见考古冲突第 9 条：

- **锁文件**。X 以 launcher 原样传入的 d 调用 `process_lock`（`x_execution_session.rs:176`；launcher 传参见 `production_session.rs:462-463`、`:471`、`:483`）。E/B 以经济 manifest 的库路径调用（`rust/src/economic_session/service.rs:1569`），`store.rs:292-294` 要求它与 CLI 的 d 经 `canonical_target`（`store.rs:448-458`，与 `process_lock` 同一分支法）相等，所以两种分支下推出的锁路径都与按 d 推出的相同。`process_lock` 按 `d.exists()`（跟随链接）分支：存在时锁路径为 `canonicalize(d)` 接 `.writer.lock`；不存在（含悬空链接）时为 `canon(parent(d))/basename(d)` 接 `.writer.lock`（`session_transport.rs:44-56`）。随后以 create=true 普通打开，不带 `O_NOFOLLOW`（`:57-63`），锁路径最终条目若是链接，会跟随到链接目标，在目标处建文件。launcher 在 d 不存在时先跑 E/B init（`production_session.rs:341-362`），init 建库后 serve 走存在分支；X 首启时建库，再启也走存在分支。所以 d 存在时只用存在分支的锁路径，d 不存在时两个分支的锁路径都会用到。
- **主库**。E/B（`store.rs:529-532`）和 X（`x_execution_session.rs:177`，rusqlite 默认标志）打开时都不带 `SQLITE_OPEN_NOFOLLOW`。SQLite 先逐级解析整条路径的链接，含最终条目与悬空链接，相对目标按链接所在目录解析（`sqlite3.c:47110-47166`、`:47195-47220`）；pager 在未带该标志时接受解析结果（`:64397-64405`），在解析后的路径上建库。
- **SQLite 侧件**。`-journal`、`-wal` 由解析后的库路径拼出（`sqlite3.c:64517-64536`），`-shm` 由已打开库的路径拼出（`:45186-45192`），全部以 `O_NOFOLLOW` 打开（`:46855-46856`、`:45220`）。所以侧件只落在实际库文件所在目录；侧件名处预置的链接只会让打开失败，不会把创建带到别处，无须另判。E/B 用 WAL（`store.rs:539`），X 用默认回滚日志。
- **E/B socket**。launcher 只把经济 manifest 路径 `economic.manifest` 传给 E/B（`production_session.rs:460`、`:471`），socket 路径由 E/B 从经济 manifest 读：E 取 `e.socket`，B 取 `chong_id` 等于该 B 的 `economic.b[i].chong` 的那条 `chongs[j].socket`（`store.rs:281-296`）。`Config::load` 对 socket 只要求绝对路径且互不重复（`store.rs:158`、`:162`、`:176`），不约束目录。launcher 拉起 S 后不等待即拉起 E 与各 B（`production_session.rs:455`、`:467`、`:480`）；E/B serve 只等自己的库打开与 epoch、audit 通过（`economic_session/service.rs:1500-1503`），随后 `serve_unix_with_deadline`（`:1524`）经 `serve_unix_inner` 调 `UnixListener::bind`（`session_transport.rs:617-635`），在 socket 路径建目录项。bind 解析父目录各级链接；最终条目是否跟随，Unix 平台之间不同。Apple 发布的 XNU 源码（commit `f6217f891ac0bb64f3d375211650a4c1ff8ca1ea`）中，`unp_bind` 以 `FOLLOW` 调 `namei`，解析后目标已存在才报 `EADDRINUSE`，目标不存在则在解析后的父目录创建 socket（`bsd/kern/uipc_usrreq.c:1189-1236`；`bsd/vfs/vfs_lookup.c:154-160`、`:1040-1044`、`:1601-1609`）。所以最终条目是悬空链接时，socket 建在链接目标的父目录。其余 Unix 平台本轮未逐一核实。socket 的创建目录因此取两处：`canon(parent(socket))`，以及按 Q8 解析最终链接后的 `parent(canon(socket))`。不跟随的平台落在前者，跟随的平台落在后者；两处都判，对两种语义都只多拒不多放。以上为静态源码证据，未运行验证（PR #1826 独审 41T）。

所以只看 `canon(parent(d))` 会放过两类布局。一是安全目录内的 `alias.db` 链接到 P 内已存在的库，锁文件与侧件都落进 P。二是库和库目录都安全，锁路径却是指向 P 的链接，锁文件建进 P。三是库、锁都安全，E 或某 B 的 socket 放在 P、A 或族目录内，或是指向那里的悬空链接，bind 在 S 打开窗口内改 stamp。三者都是本票要消除的竞态。

**Q3 两个 state_dir**：默认值为「launcher manifest `state_dir` 不进拒绝规则；A 取 `s.admission_job_limits` 的上两级目录」（出处：`production_session.rs:320-323`、`admission_anchor.rs:1336-1342`），确认或否决。推理链：launcher `state_dir` 只在 S 库不存在时、S init 之前写一次 `structure-source.json`，之后再也不碰，不与 S 打开窗口并发。

**Q4 s.socket**：默认值为不立规则（出处：`writer_cli.rs:357-359`、`:393`、`:725`、`:802`，`session_transport.rs:634-635`），确认或否决。推理链：参数期已要求 socket 路径不存在；bind 在 driver open 之后；运行期目录只核身份。残留 socket 在参数期就以 `TransportUnavailable` 确定性拒绝，不是竞态。这一例外只因 S 自己在 driver open 之后才 bind，不涵盖 E/B 的 socket：后者由其他进程 bind，不等 S，见 Q2 第四类。

**Q5 L-2**：默认值为只记录、不改实现（出处：M-2 审查 `:49`、`:88`，`production_session.rs:204-207`），确认或否决。

**Q6 是否做成可调字段**：A 类，沿用 ADR 0041:11、:13。「放行冲突布局」是同一判断的宽松档：放行以后 S admission 仍按严格版拒绝，只是从确定性拒绝变成间歇失败。所以不设字段，不列为待决。

**Q7 分级**：A 类，沿用 graph-engineering.md:23 和 #1323 N-1。

**Q8 路径比较语义**：默认值为「canon = 解析符号链接后的绝对路径，包括路径最终条目的文件链接；路径不存在时取最近存在祖先的 canon，再接上剩余段；最终条目是悬空链接时先取链接目标再求 canon」，库路径与锁路径同用此语义（出处：S 侧 `admission_anchor.rs:1292-1294` 对 db 做 canonicalize，注释称与 `process_lock` 同语义；`session_transport.rs:43-63`；SQLite 解析库路径见 `sqlite3.c:47110-47166`；夹具 `production_session_entry.rs:71-72` 已 canonicalize），确认或否决。推理链：冲突的物理判据是写入落在同一个真实目录。不解析符号链接的话，`/alias→P` 就能绕过检查；只解析父目录、不解析库文件本身，`/abs/x/alias.db → P/x.db` 同样能绕过；不解析锁路径本身，`/abs/x/x.db.writer.lock → P/x-external-lock` 也能绕过。socket 同时取不解析最终条目的 `canon(parent(k))` 与解析后的 `parent(canon(k))`，覆盖 bind 跟随与不跟随最终链接两种平台语义（Q2 第四类）。launcher 用 canon 比较，S 侧不跟随链接，两者语义不同，但 launcher 一侧是超集，方向同样是只多拒不多放。

**Q9 验收落点**：默认值为 `rust/tests/production_session_entry.rs` 和 CI job `rust-test-production-session`（`ci.yml:500`、`:517`、`:523`）。正控拆成两条：原夹具布局一条，五者全独立一条。确认或否决。

**Q10 runbook 订正**：默认值为订正 :40、:59 为 A，在 :53 标注 launcher `state_dir`，并给出分目录示例（出处：runbook:35、:40、:53、:59），确认或否决。

**Q11 图归属与时序**：默认值为维持挂 #1340、blocks #1781，不新增边（出处：`.sandcastle/decision-claimer.mts:36`、`:1486`；#1818 评论 6550407378；#1781 的 blocked_by 为 #1769、#1818），确认或否决。

**Q12 launcher 推导 A 时是否核验后缀 `s-admission/limits.json`**：C 类，真正待决。推荐答案：不核验。理由：后缀不合法时，S 会在 E1 以 `Identity` 确定性拒绝（`admission_anchor.rs:1336-1342`），launcher 再核一遍是同一判据的重复实现，不增加任何保护。launcher 无条件取上两级目录参与布局判定，即使后缀不合，最坏也只是误拒，不会误放。

## 裁定

本票选候选 A。下文是精确规则。

正本落点：docs/adr/0059-production-session-layout-contract.md

1. **canon 定义**：解析符号链接后的绝对路径，路径最终条目是符号链接时也解析。路径尚不存在时，取最近存在祖先的 canon，再按原样接上剩余路径段。路径最终条目是悬空符号链接时，先取链接目标（相对目标按链接所在目录解析，多级链接逐级取），再对目标按本条求 canon。
2. **S 测量边界**：
   - `P = parent(canon(s.db))`，`b = basename(canon(s.db))`；
   - `A = canon(parent(parent(s.admission_job_limits)))`；
   - S 族条目：P 中名字等于 `b`，或以 `b.`、`b-` 开头的条目。

   A 的推导不核验后缀，后缀合法性由 S 的 E1 核验（Q12 推荐答案）。
3. **并发写域的库与判定目录**：库为 `economic.e.db`、每个 `economic.b[i].db`、`x.db`。对每个这样的库 `d` 取下列判定目录：
   - `Q₁ = canon(parent(d))`，即 d 的目录项所在目录；
   - `Q₂ = parent(canon(d))`，即 d 解析后实际库文件所在目录，也是 SQLite 侧件所在目录。d 不是符号链接时 `Q₁ == Q₂`；
   - 锁目标目录 `Q₃ = parent(canon(L))`，对下列每个锁路径 L 各取一个：`L₂ = canon(d)` 接 `.writer.lock`，恒取；d 不存在时（跟随链接判存在，悬空链接算不存在）另取 `L₁ = Q₁/basename(d)` 接 `.writer.lock`。L 不是符号链接时 `Q₃` 等于 `Q₁` 或 `Q₂`；L 是符号链接（含悬空）时，`Q₃` 是其最终目标所在目录。

   `L₂`、`L₁` 与 `process_lock` 的存在、不存在两个分支一一对应（`rust/src/session_transport.rs:43-63`）；d 不存在时两个分支都会被用到，见 Q2。

   **并发写域的 socket 与判定目录**：读 `economic.manifest` 指向的经济 manifest，取 E 的 `e.socket`，以及对每个 `economic.b[i]` 取 `chong_id` 等于其 `chong` 的 `chongs[j].socket`。对每个这样的 socket 路径 `k` 取两个 socket 目录：`Q₄ = canon(parent(k))`，即 k 的目录项所在目录；`Q₅ = parent(canon(k))`，即按第 1 条解析最终链接（含悬空）后 bind 可能创建 socket 的目录。k 不是符号链接时 `Q₄ == Q₅`。两者都判，理由见 Q2 第四类。经济 manifest 读不出、缺这些字段或找不到对应 `chong_id` 时，同样在拉起前以 `InvalidDomain` 退出；E/B 的 `Config::load` 对同一文件本会确定性拒绝（`store.rs:158`、`:176`、`:286-287`），launcher 只是提前。
4. **拒绝条件**：任一 `d` 的 `Q₁`、`Q₂` 或任一 `Q₃`，或任一 socket 的 `Q₄`、`Q₅`（下记 `Q`）满足下列任一条，即 `InvalidDomain`：
   - `Q == P`；
   - `Q == A`，或 `Q` 位于 `A` 之下；
   - `Q` 等于某个 S 族条目的路径，或位于其下。

   `d` 本身取了 P 内的 S 族名字，已被 `Q == P` 覆盖。S 侧原本也会以 `UnknownDatabaseSidecar` 确定性拒绝（`admission_provider.rs:229-243`），launcher 只是把判定提前。
5. **不比较 launcher manifest `state_dir`**。理由是它只在 S init 之前写一次（`production_session.rs:320-323`），不与 S 打开窗口并发。
6. **不比较 `s.socket`**。残留 socket 在 S 参数期就以 `TransportUnavailable` 确定性拒绝（`writer_cli.rs:357-359`），S 自己在 driver open 之后才 bind（`writer_cli.rs:725`、`:802`）。本条只管 `s.socket`；E/B 的 socket 按第 3、4 条判定。重启时谁来清理，归 #1823 调查，不在本裁定范围内。
7. **不扩大范围**：P 的祖先目录、兄弟目录，以及 P 内既非族条目、也不在 A 内的子目录，都不在 M 内，一律放行。
8. **错误信息**：stderr 必须含 `InvalidDomain`，并点名冲突的两条 manifest 字段及两侧 canon 路径。被冲突的字段按情形取：与 P 或族条目冲突时是 `s.db`，与 A 冲突时是 `s.admission_job_limits`。并发写域的字段名为 `economic.e.db`、`economic.b[<i>].db` 或 `x.db`，该侧路径给出命中的判定目录（`Q₁`、`Q₂` 或 `Q₃`）。socket 冲突时，并发写域一侧点名经济 manifest 的具体字段 `e.socket` 或 `chongs[<j>].socket`，该侧路径给出命中的 `Q₄` 或 `Q₅`。
9. owner 落点：`load_manifest`（`rust/src/bin/production_session.rs:125`）。检查（含第 3 条读经济 manifest）位于其返回 `Ok` 之前，只读不写，因而早于 `check_existing_s`（`:300-302`）、任何 init、写 launcher `state_dir` 以及任何拉起。命中时不建任何域文件。
10. **不设可调字段，不设放行开关**。依据是 ADR 0041:13 的判别句和 #799。launcher 判定和 S admission 核验判的是两件事：前者是静态布局，后者是打开窗口内的 stamp。两者不在同一个 if-else 上，也不互相兜底。
11. **L-2 记录**：immutable 预检最多误拒、不会误放；改由 S 自核需要改 S v3 writer CLI，另行开票。本裁定不改实现。
12. **runbook**：`docs/runbook_production_session.md` 写明 M 的三部分和第 4 条拒绝规则；:40、:59 改写为「admission state 目录 A（owner 原件 `state_dir`）」；:53 标注「launcher 自身 `state_dir`」；给出 `/abs/s/s.db`、`/abs/s-state/`、`/abs/e/`、`/abs/b/`、`/abs/x/` 的分目录示例。
13. 本裁定不改资金、隔离、恢复帽、admission 判据，不签 P0，不改 Destination。

## 验收清单

所有测试都落在 `rust/tests/production_session_entry.rs`。判据：在实施 PR 精确 head 的 `rust-test-production-session` job 中，`cargo test — production_session_entry` step（ci.yml:517）的日志里每个测试名逐行出现 `test <名> ... ok`，且两条 step（:517、:523）均以 `test result: ok.` 结束。

1. `layout_rejects_e_db_in_s_db_parent`：E 库与 S 库同目录。launcher 非零退出，stderr 含 `InvalidDomain`、`economic.e.db`、`s.db`。断言 S、E、B、X 库及其侧件都不存在，launcher `state_dir/structure-source.json` 不存在。
2. `layout_rejects_b_db_in_s_db_parent`：同第 1 条，点名 `economic.b[0].db`。
3. `layout_rejects_x_db_in_s_db_parent`：同第 1 条，点名 `x.db`。
4. `layout_rejects_e_db_under_admission_state`：A 就是 P，E 库在 P 下一个已存在的子目录里。拒绝，stderr 含 `s.admission_job_limits`，不产生任何域文件。
5. `layout_rejects_e_db_under_separate_admission_state`：A 与 P 分离，E 库在 A 之下。拒绝。
6. `layout_rejects_e_db_under_s_family_entry`：E 库目录是 P 内的一个 S 族目录（如 `s.db.admission-private`）或其子目录。拒绝。
7. `layout_rejects_symlink_alias_of_s_parent`：E 库目录是指向 P 的符号链接。拒绝。
8. `layout_resolves_not_yet_existing_db_dirs`：E 库父目录 `L/new` 尚不存在，L 是指向 A 的符号链接。拒绝。
9. `layout_rejects_x_db_file_symlink_into_s_parent`：A 与 P 分离；`x.db` 取安全目录内的 `alias.db`，它是已存在文件 `P/x.db` 的符号链接，`P/x.db.writer.lock` 不存在。launcher 非零退出，stderr 含 `InvalidDomain`、`x.db`、`s.db`。断言 P 的目录项集合与拉起前相同（`P/x.db.writer.lock` 不存在），S 库及其侧件、launcher `state_dir/structure-source.json` 都不存在。
10. `layout_rejects_e_db_dangling_symlink_into_admission_state`：A 与 P 分离；`economic.e.db` 取安全目录内的悬空链接 `alias.db`，目标为不存在的 `A/e.db`。拒绝，stderr 含 `InvalidDomain`、`economic.e.db`、`s.admission_job_limits`；`A/e.db` 不存在，不产生任何域文件。
11. `layout_rejects_writer_lock_symlink_into_s_parent`：A 与 P 分离；`x.db` 字段取安全目录内的 `<x>/x.db`，该文件已存在且是普通文件，`<x>/x.db.writer.lock` 预置为悬空链接，目标为不存在的 `P/x-external-lock`。launcher 非零退出，stderr 含 `InvalidDomain`、`x.db`、`s.db`。断言 P 的目录项集合与拉起前相同（`P/x-external-lock` 不存在），S 库及其侧件、launcher `state_dir/structure-source.json` 都不存在。
12. `layout_rejects_absent_db_writer_lock_symlink_into_admission_state`：A 与 P 分离；`economic.e.db` 取安全目录 `<e>` 内的悬空链接 `alias.db`，目标为另一安全目录内不存在的 `<e-real>/e.db`；`<e>/alias.db.writer.lock` 预置为悬空链接，目标为不存在的 `A/e-external-lock`。此时 `Q₁`、`Q₂` 与 `L₂` 的 `Q₃` 都安全，只有不存在分支的 `L₁` 命中 A。拒绝，stderr 含 `InvalidDomain`、`economic.e.db`、`s.admission_job_limits`；`A/e-external-lock` 与 `<e-real>/e.db` 都不存在，不产生任何域文件。
13. `layout_rejects_e_socket_in_s_parent`：A 与 P 分离；S、E、B、X 库及锁都在各自安全目录；经济 manifest `e.socket` 取 `P/e.sock`（不存在，非 S 族名字，与各 B socket 不重复）。launcher 非零退出，stderr 含 `InvalidDomain`、`e.socket`、`s.db`。断言 P 的目录项集合与拉起前相同（`P/e.sock` 不存在），S、E、B、X 库及其侧件、launcher `state_dir/structure-source.json` 都不存在。
14. `layout_rejects_b_socket_under_admission_state`：A 与 P 分离，库同第 13 条；`chongs[0].socket` 取 `A` 下的 `b.sock`。拒绝，stderr 含 `InvalidDomain`、`chongs[0].socket`、`s.admission_job_limits`；`A` 下 `b.sock` 不存在，不产生任何域文件。
15. `layout_rejects_e_socket_under_s_family_entry`：库同第 13 条；P 内已存在 S 族目录 `s.db.admission-private`，`e.socket` 取其中的 `e.sock`。拒绝，stderr 含 `InvalidDomain`、`e.socket`、`s.db`；不产生任何域文件。
16. `layout_rejects_b_socket_parent_symlink_into_s_parent`：库同第 13 条；安全目录内的 `<b>/run` 是指向 P 的目录链接，`chongs[0].socket` 取 `<b>/run/b.sock`。拒绝，stderr 含 `InvalidDomain`、`chongs[0].socket`、`s.db`；P 的目录项集合不变，不产生任何域文件。
17. `layout_rejects_e_socket_final_dangling_symlink_into_s_parent`：库同第 13 条；`e.socket` 取安全目录 `<e-run>` 内预置的悬空链接 `alias.sock`，目标为不存在的 `P/e.sock`（非 S 族名字，与各 B socket 不重复）。此时 `Q₄ = canon(<e-run>)` 安全，只有 `Q₅` 命中 P。launcher 非零退出，stderr 含 `InvalidDomain`、`e.socket`、`s.db`。断言 P 的目录项集合与拉起前相同（`P/e.sock` 不存在），S、E、B、X 库及其侧件、launcher `state_dir/structure-source.json` 都不存在。
18. `layout_rejects_b_socket_final_dangling_symlink_into_admission_state`：A 与 P 分离，库同第 13 条；`chongs[0].socket` 取安全目录 `<b-run>` 内预置的悬空链接 `alias.sock`，目标为不存在的 `A` 下 `b.sock`。只有 `Q₅` 命中 A。拒绝，stderr 含 `InvalidDomain`、`chongs[0].socket`、`s.admission_job_limits`；`A` 下 `b.sock` 不存在，不产生任何域文件。
19. `layout_accepts_db_file_symlink_between_safe_dirs`：`x.db` 取 `<x>/alias.db`，链接到已存在的 `<x-real>/x.db`，两目录都不在 M 内。放行，launcher 正常拉起 S 和 X。
20. `layout_accepts_sockets_in_non_family_subdir_of_s_parent`：A 与 P 分离；`e.socket`、`chongs[0].socket` 都在 P 下一个已存在、非族的子目录里，库都在安全目录。放行，launcher 正常拉起 S、E 和 B。
21. `layout_allows_s_socket_in_s_parent`：`s.socket` 取 `P/s.sock`，其余分离。放行，launcher 正常拉起 S（保留裁定第 6 条例外）。
22. `layout_accepts_db_in_non_family_subdir_of_s_parent`：A 与 P 分离，E 库在 P 下一个已存在、非族的子目录里。launcher 正常拉起 S 和 E。
23. `layout_accepts_original_fixture_layout`：沿用原八项夹具，即 A 在 P 之下、E/B/X 在 P 的父目录。放行。
24. `layout_accepts_separate_directories`：S、A、E、B、X 各自独立目录。放行。
25. `layout_allows_launcher_state_dir_equal_s_parent`：launcher `state_dir == P`，其余分离。放行。
26. 现有测试（`simulation_profile_spawns_isolated_domain_processes` 等）在 :517 step 逐行 `ok`。
27. 正本文件存在：`ls docs/adr/ | grep -c -- '-production-session-layout-contract.md$'` 输出 `1`。编号在合入时按 main 顺序确定，等于当时 main 上最大编号加一（现最大为 0057）。
28. 正本锚句：以下每条在该 ADR 中的 `rg -c` 命中数都 ≥1。
    - `rg -c '最近存在祖先'`
    - `rg -c 'InvalidDomain'`
    - `rg -c '不比较 launcher manifest `state_dir`'`
    - `rg -c '不比较 `s.socket`'`
    - `rg -c 'owner 落点：`load_manifest`'`
    - `rg -c '不设可调字段'`
    - `rg -c 'L-2 记录'`
    - `rg -c '启动排序'`
    - `rg -c '实际库文件所在目录'`
    - `rg -c '锁目标目录'`
    - `rg -c 'socket 目录'`
29. runbook 检查：
    - `rg -c '<state>/s-admission' docs/runbook_production_session.md`：无命中（退出码 1）；
    - `rg -c 'admission state 目录' docs/runbook_production_session.md` ≥1；
    - `rg -c '测量域' docs/runbook_production_session.md` ≥1；
    - `rg -c '/abs/e/' docs/runbook_production_session.md` ≥1。
30. 票状态：resolution 落地后 `gh api repos/xy7365527-lang/NewChanlun/issues/1818 --jq .state` 输出 `closed`；`gh api repos/xy7365527-lang/NewChanlun/issues/1781/dependencies/blocked_by --jq '.[].number'` 仍含 `1769`。
31. resolution 首行等于「本条依 #1323 Notes N-1 预授权判定」。

## 受影响票与 blocking 边变化

- **#1818**（本票）：resolution 落地并经独审后关票。既有边 #1818 → #1781 由关票自然解除，不另删边。
- **#1781**：保留 blocked_by #1769，以及 #1818（直至本票关闭）。实施范围更新如下：
  - 验收 1 定为「启动前拒绝」，即本裁定第 1–10 条；「自动分目录」已排除；
  - 验收 2 按第 12 条执行；
  - 验收 3 按第 11 条只做记录。

  不新增边。
- **#1769**：边 #1769 → #1781 保持，无变化。
- **#1538（FU-16）**：已关票，不加边。订正候选稿里「排在 FU-16 合入之后」的措辞。
- **#1823**：不加边，与该票自述「不接 blocked_by #1818」一致。附注：该票正文所写「残留 socket 在测量域内报 `SourceTypeUnsupported`」与代码顺序不符，见问题第 4 条。残留 socket 先以 `TransportUnavailable：socket路径已占用` 拒绝（`writer_cli.rs:357-359`、`:393`）。该票复现时可以据此核对。
- **#1340**：仍是本票父票，不变。
- **#1323**：图不变，可在 Notes 追加一行 resolution 索引。
- **#1527、#1567/ADR 0046**：不改。
- 另行开票（不阻塞任何票）：L-2「launcher 结构源预检改为 S `serve --structure-source` 自核」，实施票，需改 S v3 writer CLI。
- 不新增 research 或 probe 票。考古冲突第 9 条所列未核足迹照实留档，未定性为反例，本轮不另开票。

## 授权引用

本条依 #1323 Notes N-1 预授权判定。N-1 见 #1323 正文 :74，用户原话见 [#1323 评论 6261281111](https://github.com/xy7365527-lang/NewChanlun/issues/1323#issuecomment-6261281111)：用户 2026-10-18 原话「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」。分级为 T 类（#1818 评论 6550474840），依据是 `.sandcastle/docs/graph-engineering.md:23`。本票不涉资金、隔离放宽、真实场所、关图或 Destination 改写。
