# #1406 C6a 合同恢复 v2：资源继承依据与 guardian 原则已定位

名分：绑定 #1406/#1323 的恢复工作草稿。日期 2026-10-17。此次只读主控指定的当前线程历史两行与明确仓内引用，不扫描其他聊天，不运行产品、cargo、真实信号或浏览器，不访问外盘。

**结论更新：v1 的“缺资源数值来源、不能沿 C7a 继承”阻塞现可解除。原派发明确要求“沿C7a资源数”；原指令还明确指定 D2 的所有权/截止/封口原则与 Linux WNOWAIT/subreaper。可以据下列精确来源继续重建，但没有恢复出 C6a 原 CI v3 字节、审包或原反例，因此不能据此签准备通过或允许运行。**

v1 历史报告保留。本报告订正的是其搜索信息不足，不把先前建议的新文件名当原文件名。

## 1. 原始指令定位、逐字摘引与行摘要

唯一读取的历史文件：
`/Users/silencehan/.codex/sessions/2026/10/15/rollout-2026-10-15T15-04-58-01a12e01-45a1-7173-a3e8-d338d108fb93_01a13e60-df33-7e83-b868-cb432af4fafd.jsonl`

只读取指定第11318、11702行的 JSON 内容。以下 SHA256 对**原始整行字节、含结尾换行**计算，不是摘引文字的摘要。

| 行 | 原始行 SHA256 | 内容身份 |
|---|---|---|
| 11318 | `754e79c18dc458057255347c1c58771114f644b5db8677b43c1df4028d339519` | spawn_agent，prepare_1406_c6a_ci，call_ef21fb1c302b484aa5fd8ba92d351a8e |
| 11702 | `b1f17cc656d3f2ad5ad71cea2c81197135b6900c195cbf43ca25b4c3c1f4bbca` | spawn_agent，review_1406_c6a_ci，call_c33761e23b774a36b925880b378e9832 |

11318原文：

> 参考D2已审监督器wt-1505-late-exclusive-fix/.github/scripts/run-1505-d2.py的身份/未reap leader/截止/归档/取消，但不能盲套旧C7a监督器。外层同t0的绝对阶段端点build1200/runtime1830/verify1920/archive2040/final2070，cleanup包含阶段；内层600/630等不变。沿C7a资源数，实际构建E/B两个bin；build-evidence精确六键 source_sha/source_tree/command(list)/exit_code/source_files(11)/binaries。

> source pending先阻止运行，待源码独审提交才绑定，CI只允许两文件尾差。只push validate/1406-c6a触发此专用workflow，不改ci.yml、不dispatch/PR/commit/push。

> 真实WE5 20/WE7 21经正式Q公共HTTP，内层成功收据仅候选，外层须独核exit0/无取消/所有原件/完整41步/binary前后hash。driver children继承组，outer新建driver组并持未reap leader到最后信号，reap后禁kill；取消后保失败原件不签PASS，handler延续封口。

11702原文：

> 核Linux WNOWAIT/subreaper真实身份所有权/禁止reap后组信号、child继承组、任何取消及迟到不能最终PASS、所有耗时helper受同原点1200/1830/1920/2040/2070阶段截止含回收、无中断重开窗、失败保原件、全41步独立账目/transport/Artifact集合/进程生命周期/二进制前后hash、精确11源码/六键build证据。PENDING及CONTRACT_READY=False保留，最终绑定后必须尾审。

11702还定位旧包：`recovery-20261015/1406-c6a-ci-v1`，9项 MANIFEST `29df23263c0d888a3679d9f1f332cbf72f75af648eb3efd1f4e25f406f31e954`、作者49替身通过；以及 `1406-c6a-driver-fix-v2/OUTER-CONTRACT.json`。这些是历史引用，**本次未恢复原包字节，也未重新验证49项**。

## 2. 正确原文件名与 Git 现状

原两文件实际为 `.github/scripts/run-1406-c6a.py` 与 `.github/workflows/issue-1406-c6a.yml`。对这两条精确路径分别执行 `git log --all --format=%H -- <path>`，当前本地全部 refs 可达历史的提交数都为0。旧 `codex/1406-c6a-ci-prep-20261016` 仍指基线 `4d9b072e5b671fc1a84f7d75b3976471b93d2afb`。

原指令指定 push 分支 `validate/1406-c6a`；当前保存源码的 `validate/1406-c6a-final` 是另一分支。后续重建必须显式对齐实际分支与 trigger；不能沿用先前报告建议的 workflow_dispatch 并称其为原合同。原“不得commit/push/dispatch”属于准备工位权限限定，未因本次发现自动解除。

## 3. 可复核代码版本

| 角色/路径 | 精确 commit | 文件 SHA256 |
|---|---|---|
| C7a资源数，`.github/scripts/run-c7a-remote.py` | `4d9b072e5b671fc1a84f7d75b3976471b93d2afb` | `25f301ff1f202e0fb4e5ef1168a685884649adec69c5878cc59bd06438273bcf` |
| D2监督器，`.github/scripts/run-1505-d2.py` | `b9d3ecde1838e0db3eb67bfd01d364d624ec8619` | `0ef124bd379fe6dcc14284f43ace57e90ce4c4051c301bc027dd7e676c17bf15` |
| 当前C6a driver，`s_session/tests/tb03c_query_runtime.py` | `b6e690ecae5723fbbcb773f15400c71d1010da87` | `902c4eddb9ca55fe0cfb0f0c3a878d7296c8e58761924852cd29c728dd789d6e` |

C7a Git blob `66ae8160763cfdc32b8c4973af4586c0764bf9d4`，18092字节；D2 Git blob `b1b801bdcb9fc290a689a7fa213f0892d4d6654d`，42098字节。D2取本地 `codex/1505-late-exclusive-fix-20261016` 与远端跟踪 `origin/proto/1505` 相同精确HEAD。旧指令没写 D2 commit，本次以该具名分支现存已提交版本作为可恢复代码锚；不能证明这与旧未提交工作树每字节相同。

## 4. C7a资源数恢复表

下列行号均对第3节 C7a 精确版本。它是明确获准继承的资源数来源，**不是获准复用其旧监督控制流**。

| 资源项 | 原值与口径 | 源行 |
|---|---|---|
| 启动可用磁盘 | 至少8GiB，`shutil.disk_usage(work).free` | 297–298 |
| 启动可用内存 | 至少3GiB，Linux `/proc/meminfo` 的 MemAvailable，不用总内存 | 82–87、297–298 |
| 构建组RSS | 最大5GiB，求本次新进程组成员RSS之和 | 185、194、303、309 |
| 运行组RSS | 最大2GiB，同组求和 | 185、194、304、322 |
| 运行磁盘保留 | 最低1GiB | 161、190、304 |
| 运行可用内存保留 | 最低512MiB | 162、193、305 |
| 文件系统增长 | 相对启动free减少不得超过6GiB | 163、191–192、305 |
| 监测周期 | 每轮资源检查后 sleep(0.25)，非固定强实时延迟保证 | 181–198 |
| 证据与工作目录 | 必须同一受监测文件系统（st_dev相同） | 296 |
| 构建并发 | `cargo build --locked --features s_session --jobs 2 --bin e_economic_session --bin b_chong_session` | 307–309 |

GiB=1024³、MiB=1024²（23–24行）。`group_members` 以 `/proc/<pid>/stat` PGID 匹配、RSS页数乘 `SC_PAGE_SIZE`（90–103行）。观测异常进入失败回收路径（199–204行），而不是把缺数据记为0后继续。该实现没有严格限定每次 `/proc`/文件系统读的不可中断耗时；C6a仍须按原新合同为所有耗时helper提供阶段监督。

**输出容量的准确恢复结论**：该C7a源中没有单独的stdout/stderr/归档文件字节容量上限；日志和运行输出只受活跃监督时的同文件系统增长6GiB与最低余量1GiB约束。`archive_run:252–271` 会保全普通文件和逐项长度/哈希、特殊文件列账，但没有独立输出cap。不能编造“原来有某个单文件cap”；也不能据此放开归档阶段的资源监督——C6a原指令要求所有helper同t0有界且失败保原件。若需要新增容量策略，应明确是新增，不冒充恢复。

## 5. guardian可继承原则与平台差别

以下 D2 行号均对第3节精确版本：

- `64–72` handler只累计取消状态；`203–206` Popen返回与OWNED_GROUPS登记不因handler抛错跳过。
- `133–189` 未reap直属leader保有PID/PGID，TERM后不提前poll/wait；leader已收割或组消失后不再发信号；归属/权限异常则记录失败。`174–188` wait及最终组消失观察受传入绝对deadline限制。
- `192–242` 运行与cleanup分别传绝对截止，启动前及返回后检查期限，记录退出/超时/取消/清理；成功要求exit0且组已回收。
- `301–334` 尚有未回收组时不删scratch；先安全保留原件，再归档，再清理。归档等helper可在取消状态下继续有界封包，但不因此恢复成功资格。
- `337–374` 清单、取消、迟到、封口后再次检查共同决定最终exit；RESULT本身不是充分通过证据，LATE/CANCELLED保留。
- `677–679` handler贯穿主流程且要求SIGCHLD默认处理；不把handler在封口前撤掉。

**不能直接复制的部分**：D2是macOS监督器，当前实现没有 Linux WNOWAIT/subreaper；这些 Linux 义务来自11702原指令（C7a代码106–112、195–198也有实现线索）。D2 TERM2/KILL8与1170秒全窗是D2自身数值，**不继承到C6a**；C6a内层 TERM3/KILL3及600/630等以C6a driver/原评论为准。D2 SQLite内存/overlay等D2_LIMITS同样不进入C6a。原指令要求 outer持未reap driver leader直到最后信号，这必须在Linux重建中实际实现并测反例，而不是仅引用D2函数名。

## 6. C6a原时间轴与六键/11文件

原11318直接把同一t0的绝对端点绑定为：build1200、runtime1830、verify1920、archive2040、final2070秒；cleanup包含阶段，不在各阶段后再重开窗口。11702进一步要求所有耗时helper也受这一时轴监督。

当前driver `32–35`：work600、case300、final630、startup10、exchange30、CLI30、TERM3/KILL3、frame1048576字节。`40–42` Q资源为 frame/reply1048576、缓存4194304、snapshots4、domains4、changes32、timeout/operation_timeout30000ms。这是已提交driver合同，不能改成资金参数默认值。

`verify_build:252–265` 要证据键集合**恰好**：`source_sha/source_tree/command/exit_code/source_files/binaries`；command为非空字符串列表，exit_code为整数0，完整40位HEAD/tree吻合；二进制集合恰好E/B且hash相符。`SOURCE_FILES:43–49` 的11条为：

```
s_session/economic_query.py
s_session/economic_http.py
s_session/s_socket_client.py
s_session/s_readonly_server.py
s_session/s_query.py
s_session/s_query_integrity.py
s_session/s_tb02_contract.py
s_session/tests/fixtures/tb01c/query-resources.json
s_session/tests/test_tb03c_query.py
s_session/tests/tb03c_query_runtime.py
s_session/tests/test_tb03c_query_runtime_contract.py
```

driver `1028–1038` 核clean checkout、取得当前源码/二进制hash并验证build证据；其receipt明确 `hard_io_deadline=False`，所以外层硬监督义务不可省略。

## 7. 本次解除与仍保留的门

解除：不知道资源数从哪里来、不能确定阶段端点语义、缺guardian原则授权、误认原文件名/触发方式。

保留：原C6a CI v3源码/审包与旧具名反例未恢复；源tail精确绑定、所有helper硬监督、完整41步transport/公共图像/oracle绑定、startup协议、H3负控、取消后失败封包、Linux组回收及最后独立审查均须由新实现证明。新准备件需依据此次原合同纠正文件/trigger及两文件尾差约束，不得因恢复出数值直接翻 `CONTRACT_READY`、push或运行。
