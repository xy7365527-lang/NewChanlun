# #1818 / PR #1826 精确 head 独审：BLOCK

- head：`163fb363a290ecf210a265058f1af71105ca73ce`；差异基线：`a2cf513c25c45f1e835d7732425bdf024dd79b7f`。
- 只读仓库：`/Users/silencehan/.codex/worktrees/layout-publish41e/NewChanlun`。本地 HEAD 与 PR head 一致、工作区干净；2026-11-01T00:45:53Z 完成来源复核。
- 41O 的锁链接反例已闭合。唯一发布阻断是：明知 E/B socket 同样可在 S 打开窗口写入 M，却新增“库派生路径”范围排除，把原布局问题留在放行集合内。

## F1 / P2：E/B socket 的已知并发写入不能借 s.socket 例外排除

位置：ADR 0059 第 30、55 行；对应裁定第 3–4 条（90–99 行）。

**原合同与本次变化**：原 resolution [6550813315](https://github.com/xy7365527-lang/NewChanlun/issues/1818#issuecomment-6550813315) 以“并发写域落入 M”为冲突问题，Q1 选择启动前拒绝，不采用只解决一次启动的排序方案。原 Q4 与裁定第 6 条只排除 `s.socket`，依据是 S 自身 driver open 之后才 bind。原精确目录公式确实只枚举 DB；这是原考古不完整，不能据此证明所有非 DB 写足迹都已获准排除。当前第 30 行首次明确承认 E/B socket 同型竞态却留待另裁，第 55 行首次把结果限定为库派生路径。该限制没有原 Q4 的因果依据。

**静态反例**（无需任何符号链接、并发改配置或 TestOnly）：
1. P=`/abs/s`，A=`/abs/s-state`；S 库与 owner/资源原件有效。E/B/X 库、锁及 SQLite 侧件都在各自安全目录，已有库通过本域审计。
2. 经济 manifest 的 `e.socket=/abs/s/e.sock`，此前不存在，且与各 B socket 不重复；其余配置沿用可启动配置，可使用 System 时钟。`e.sock` 不是 S basename 族。
3. 所有 DB 的 Q₁、Q₂、Q₃ 都安全，当前合同放行。`store.rs:157–183` 仅要求 socket 是绝对路径且 E/B 间不重复；`:296` 取出该路径，没有 S 测量域约束。
4. launcher 在 `production_session.rs:455` 拉起 S，`:467` 拉起 E，没有等待 S driver open；E 的 `service.rs:1501–1524` 只等待自己的库打开、epoch/audit 通过，随后调 `serve_unix_with_deadline`。
5. `session_transport.rs:617–625` 转入 `serve_unix_inner`，`:635` 的 `UnixListener::bind(socket)` 创建 P/e.sock，改变 P 的目录 stamp。它不等待 S，也不需要请求或交易。
6. P 本身是 M 成员。若 bind 发生在 S 固定 stamp 与 `verify_before_open` 之间，就有本票原本要消除的 `SourceIdentityChanged`。E 的成功打开不能替代 S 的打开屏障。B 同理。

**最小修订**：保留已经正确的 Q₁/Q₂/Q₃，把经济 manifest 中 E/B 实际 socket 的创建父目录纳入同一个 M 冲突集合；按 bind 语义解析父目录，不把 socket 最终链接当成普通 open 的链接。补 E 与 B 的启动前拒绝验收：安全 DB＋socket 父目录为 P（并覆盖 A/族目录路径语义），stderr 点名经济 manifest 的具体 socket 字段与 S 字段，目标未 bind、任何域文件未创建。保留 `s.socket` 原例外和安全 socket 正控。修订规则与验收即可；实现仍归 #1781。

若主控选择拆票，必须先明确裁定并记录对原交付范围的缩窄及 #1781 的真实前置；不能仅以文书第 30 行的“留待另裁”发布成已闭合合同。本轮不改票、不加边，也不批准新的缩窄。

## 已通过的窄修订与平台依据

- **41O F1 已修**：普通既存安全 DB 的锁链接指向 P 时，L₂ 的 Q₃ 命中 P。验收 11 明确区分这一反例。
- **L₁/L₂ 与调用链对应**：`process_lock:43–63` 以 exists 分支推锁路径，随后普通 create/open 跟随最终锁链接。E/B 的 `canonical_target` 等值约束（`store.rs:292–294,448–458`）使 config.db 与 CLI d 推出的当前锁基路径一致。缺库时 launcher 先 init（`:341–362`），再 serve，故两次可能分别用 L₁、L₂；X 首启与再启亦分别涉及两分支。验收 12 能识别漏算 L₁ 的实现，验收 13 保留安全文件链接正控。
- **SQLite 侧件依据成立于现役 Unix 构建**：`rust/Cargo.toml:45` 启用 bundled，`Cargo.lock:838–839` 为 libsqlite3-sys 0.38.2；实读其 SQLite 3.53.2。`sqlite3.c:47110–47166` 按路径段解析链接及相对目标，`:64397–64405` 在未指定 SQLITE_OPEN_NOFOLLOW 时接受解析结果；`:64517–64536` 从解析后的路径拼 journal/wal；`:45186–45192` 默认从 DB 路径拼 shm；`:46855–46856`、`:45220–45225` 打开侧件时带 O_NOFOLLOW。因此最终侧件 symlink 不把创建转移到其他目录。
- 上述代码位于 `sqlite3.c:40234` 的 SQLITE_OS_UNIX 分支；macOS/Linux 具备 O_NOFOLLOW。SQLite 在缺该宏的平台会于 `:40538–40539` 将其定义为 0，且 SQLITE_SHM_DIRECTORY 可改变 shm 目录，不能把结论泛化成任意平台/自定义 VFS/自定义编译选项。所查仓内配置未发现这些覆盖，E/B 的本地文件系统门及 Unix transport 与本轮范围一致；这不是本轮第二项阻断。未验证实际生产二进制编译选项，实施验收应沿现有受管构建。

## 连续 BLOCK 是否自动转 P

**本轮不据发布返修次数自动转 P。** 原 decision 执行的 `/Volumes/AgentStorage/decisions/1818/summary.json` 为 phase=land_t、attempts=2：第一轮 BLOCK，第二轮 PASS，convertedToP=false；原 resolution 已存在。graph-engineering.md:73 的“再 BLOCK 转 P”位于决策草稿独审循环，:77 随后才进入落地；:85 明确 land_t/已有 resolution 不再由 claimer 正常重跑。发布后的精确代码事实返修没有证据被定义成重新开启同一个 attempt 计数。当前仍是技术布局修复，不涉及新资金/隔离政策。

若主控正式退回重开原决策，则须按该文 :85 的现役重开流程处理，不能重置历史或假称原来两轮都是 PASS。ADR:7 建议改为“原裁定经两轮审查，第二轮 PASS”。这处计数措辞单独不阻断。

## R1–R8

| 项 | 结论 | 依据 |
|---|---|---|
| R1 引用与事实 | BLOCK | 新锁及 SQLite 引用成立；第 30/55 行把已知 E/B 同窗口写入移出结果，不能由原 s.socket 例外证明布局合同完整，见 F1。 |
| R2 候选与权威一致 | PASS | 复用 41O/41I 未变部分；不迁移 S、不改锚、保留部署工具候选，不改教义或自由度。 |
| R3 二值验收 | BLOCK | 11–13 本身二值且可实施；整套对 F1 无检测，能全部满足而原并发写入仍被放行。 |
| R4 落点与政策范围 | PASS | load_manifest/测试/runbook owner 未变；没有资金、隔离、P0 或 Destination 新政策。F1 是技术范围遗漏，不据此虚造 P 类。 |
| R5 单一判据 | PASS | Q₁/Q₂/Q₃ 同用一拒绝集合，无开关或失败后宽松兜底。 |
| R6 图与 owner | PASS | 复用原审已核且本次未改的边；本轮不宣称重查了所有 live 依赖。F1 修订由原 owner 承接，若拆票须主控明确新前置。 |
| R7 T/P 分级 | PASS | T 类技术布局修复；land_t 后发布 BLOCK 不自动触发草稿循环二次 BLOCK 转 P，见上节。 |
| R8 考古完整性 | BLOCK | 41O 漏项已补；但作者自己查到 E/B bind 的确定写入，把它列为无阻塞后续而不是闭合当前因果链。已复核直接调用、参数来源与原 resolution；其他未变考古复用旧审。 |

## 检查与交回

证据：当前 ADR、现役调用链、bundled SQLite 源码、原 resolution 评论 6550813315、原 decision summary、41O 报告与 41Q 创建路径清单。原未变的 R2/R4/R5/R6 依据按差异范围复用；本轮不是一次新的全仓考古。41Q 的同 head diff-check、merge-tree 与锚句检查只作机械证据复用，不替代 F1 语义判定。未因 ADR 编号、0058 未合或锚句计数阻断。

TestOnly marker、SQLite 临时文件与金融读子进程的潜在写入未在本轮进一步验证，不把未知写成第二个已证反例，也不替作者“无事实缺口”作全面背书。F1 已是有完整来源、可直接修订的决定性反例。

`Prove It Works`：逐段回查实际创建路径，并给出无需改链接或额外请求的反例；未运行 Rust、CI、重型实验或并发复现。throughput checkpoint: n/a, read-only investigation。纯 ADR 差异无新增代码注释，no-comments 的注释审查不适用，不追加审查链。

仅写本目录 REPORT.md 与 review.json。请主控把 F1 交原 owner 修正文书及验收，再审新精确 head；原锁修订结论可直接复用。
