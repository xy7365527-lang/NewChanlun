# #1527 H-01 / M-02 返修交付

已完成本轮授权的候选代码返修与限定验证，留差异供经理冻结后独审。**本轮状态 completed；生产与整票仍 BLOCK_NOT_MERGEABLE，I-1 未最终验收。** 静态和 Python 模型通过不代表 Rust 测试、编译、SQLite 恢复或跨进程联调通过。

本目录为绑定 OPEN #1527 的工作草稿。管理者 `/root/manage_1527_sandcastle`；父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`。实施者为 Sandcastle 外部 Codex 工蜂；真实 session_id 由运行时事件登记，不声明为父任务的原生子代理。

## 固定输入和真实写集

| 对象 | 本轮依据 |
|---|---|
| 实施基线 | `f7d34f47cba8a0ea0ceb55499206520f058da61f`；开工工作区干净，交付时 HEAD 未移动，仍在独立 `codex/*` checkout |
| 独评输入 | `formal-i1-review-worktree` 的固定 Git source `3dde745d3909f882beac67de106a42d0f097aa65` 中 `issue1527-formal-i1-review-20261018/REVIEW.md`；SHA-256 `02887e075dab2857f225c4aad213c737e49a1a3e35bf101feaeb5f2086290379`。报告内部被审候选为 `3f79ce63…`，没有将其误写为本轮 f7d 基线 |
| 范围输入 | 已读 AGENTS.md、父目录 ROOT-I1-IMPLEMENTATION-SCOPE.md、I1-RESPONSIBILITY-AND-REUSE.md、ROOT-R4-ENGINEERING-SCOPE.md，以及 PRIOR-R4-V3-DESIGN-TRANSCRIPT.md §1–6；§7 不并入 |
| 参考复用 | 父目录既有 XERJ 缓存；只读固定 Git 源码。本轮无可调用图工具，采用限定路径读取；未安装、索引或联网 |
| 方法 | 采用仓内 `diagnosing-bugs` 的反例/回归方法；用户禁止 Rust/SQLite 运行，故实际执行仅限定模型及普通文件检查，不冒称运行真实缺陷路径 |
| 产品改动 | `rust/src/bin/s_session_v3/admission_provider.rs`、`admission.rs` |
| 必要测试扩展 | `s_session/tests/issue1527_recovery/provider_tests.rs` 仅追加 3 项测试及辅助函数。复用已有完整 S3 库夹具和私有 WriterScope 测试入口，避免增加裸 Job/时钟/可替换槽位出口；原文件全部旧字节保持为前缀 |
| 新产物 | 仅本目录 REPAIR.md、CHECKS.json、MANIFEST.json、FINAL.patch、models.py、verify.py、BEFORE-MODELS.json、AFTER-MODELS.json |

`FINAL.patch` 只包含相对 f7d 的上述 3 个产品/测试文件，合计 **7 个 hunk**；SHA-256 为 `da75b61856923eb4206191c869b8024e13e1a9513d182f0d27f30e7c94518e7b`。逐 hunk 旧/新行坐标、上下文字节、删除和新增数量均在内存核对，零 fuzz 重建结果与当前文件逐字相同。报告文件另由 MANIFEST 绑定，避免补丁自包含递归。

## H-01：纳入监督器既有控制锁

Controller 在固定基线 `s_session/s_service_control.py:207`、`:210` 创建并持有 state 与 `<db>.control.lock`；`:230` 的 close 只关 FD、留路径。原 provider 七项白名单将第二个锁当作未知侧件，持锁与关 FD 后留路径两种合法状态都在 E6 被拒绝。

本轮在 `admission_provider.rs:237` 将精确后缀 `.control.lock` 纳入原成员枚举，仍沿原 `Pinned::open`（`:47`）使用只读 `O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC` FD、文件身份/完整 stamp 复核和单链接校验。该原函数逐字未改。控制锁路径若是目录，在 `:134` 具名拒绝 `ControlLockNotRegular`；未知侧件仍在 `:244` 拒绝。

控制锁与其他真实成员共同经过 `:141` 的 `st_blocks × 512` checked 计算和原 store 累计、上限检查。没有用逻辑长度代分配量，没有默认零量；空锁的实际 st_blocks 可为零。控制锁不算临时恢复件，也不引入新预留。测量后的 `verify_before_open` 继续核固定 FD/路径、stamp 和成员集合；`:348` 的运行期门也核该锁完整 stamp，不能用同字节新 inode 替换。锁本身从未被 provider 创建、删除、截断或重新争抢；WriterScope 仍仅取得原 writer/maintenance 两锁。

实际小普通文件检查从源码提取白名单，分别保持独占 flock、关闭持锁 FD 后保留路径。两例旧白名单拒绝，新白名单接受；各例锁实际分配 **4096 字节**，限定模型总账 **8192 字节**。独立探测 FD 证明原锁持续由模拟监督器持有，直至该 FD 关闭；provider 式只读 pin 没有延长锁所有权。未知侧件仍拒绝，同字节替换仍能检测出 inode 不同。这些是 **Python 普通文件/分类检查**，不是 Controller 或 Rust provider 的运行验收。

新增 Rust 源码测试位于 `admission_provider.rs:447`、`:451`、`:455`、`:466`、`:479`，覆盖上述两种合法状态、精确物理账、未知侧件、软/硬链接和目录替身、同字节新 inode。**5 项均 NOT_RUN。** 它们只需普通文件及真实锚 loader，不执行 SQLite。

## M-02：保留失败收尾态及同一本账

必须区分旧评审和本轮基线：旧评审对象在 `finish_output` 之前 `state.take()`，三类失败都会丢槽；f7d 中 B2 已将清槽移至成功之后，该成果原样保留。现有基线剩余反例来自 `rust/src/s_content/job.rs:1042` 的原 `finish_check`：**未结预留只报 `job_reservation`，不设 stopped**；`admission.rs` 原 startup_gate 只查期限，因此第一次收尾报错后资格检查仍可成功，释放预留后再 finish 也可成功。

本轮 `admission.rs:64` 增加仅属于 Inner 的首次收尾错误槽，不更改 Job 的停止算法。`:196` 先返回已保存首错，再在原 `state.as_mut()` 上调用原 finish；任何错误都在清槽前保存并返回。仅首次成功在 `:208` 清除原启动槽、调用 B2 的 `finish_terminal_startup`。已经成功结束后的重复 finish 不再次清终端槽。`:70` 的资格门先检查该失败记录，所以 require_admitted、保留的 OpenGuard / require_current 都不能将失败当作启动结束。

失败时保持同一个 FinishedAdmission 及其原私有 `Rc<RefCell<Job>>`，不更换 Job、不清累计、不释放预留、不移动 t0、不重建期限；B2 的终端 Rc 同时保留。以后即使合法清理已对账/释放预留，也不能重开这个失败 scope；原 Job 若出现另一错误，scope 仍返回首次收尾错误。成功启动后请求仍沿原请求账，未把启动期限延长为永久租约。

失败保量反例及模型对照：

| 状态 | 旧评审对象（3f79） | 本轮基线 f7d | 本轮修正模型 |
|---|---|---|---|
| 原超限停止，wire=17，t0=123456 | 首次失败后丢槽，重复 finish/require 放行 | 同对象保留、重复拒绝 | 同对象/实量/t0 保留，首错持续拒绝 |
| 原期限到期，wire=17，t0=123456 | 同上 | 原 Job 停止，保账拒绝 | 原期限及到期首错保留，持续拒绝 |
| pending=33、1 个未结预留、wire=17、t0=123456 | 丢槽并放行 | 保账，但 require 放行；清理后 finish 放行 | 首次错误保持；原预留未被丢弃，require 和重复 finish 拒绝，清理后仍拒绝 |
| 无失败 | 正常结束 | 正常结束 | 成功才清槽，重复 finish 与运行期资格正常 |

BEFORE-MODELS.json 保留修改产品逻辑前的反例结果；AFTER-MODELS.json 绑定最终源码摘要，含每项前后量与同对象判断。模型依据源码中的取槽顺序/错误门选择控制流，并独立模拟原 Job 的停止、期限和预留语义；它不执行 Rust，不能证明类型、借用、Drop 或终端真实运行。

新增 3 项 Rust 源码测试在 `provider_tests.rs:212`、`:263`、`:299`：未结预留后反复 finish/require/OpenGuard、清理后仍保首错；原超限尾量、预留和 stopped 保持至 stderr；正常成功清槽/重复 finish。测试还核原 deadline 与 TerminalReceipt 的原 t0 一致，原 scope 析构后终端仍持同账并按实际 sink 返回量计费。**3 项均 NOT_RUN**；到期分支本轮只有静态追踪和模型证据，未新增可执行时钟替换出口。

## B2/B3 保全与验证分栏

| 分类 | 实际范围和边界 |
|---|---|
| 实际执行 | 固定 HEAD/`codex/*`/写集检查；22 份历史报告与基线逐字和 SHA-256 比对；12 个未修改关键源码/测试文件字节比对；admission 内 7 个原 I/O/转发函数体及既有 provider 测试前缀比对；源码控制流核查；3 次 rustfmt stdin 语法解析；2 份 Python AST；H-01 两种普通文件状态与 M-02 四状态模型；git diff --check；3 文件/7 hunk 内存重建。CHECKS.json 记录 **55 条子命令 argv/rc**，不是 Rust 测试数 |
| 继承 | 旧独评及 B1、B2/B3 已落成果、父目录缓存/固定设计。只复用已存在证据，不重跑旧 28/6 模型、既有测试、历史独评或原型 CI，不将其计为本轮执行 |
| NOT_RUN | cargo/rustc/build、全部 Rust 测试/类型/借用/ABI/Drop、SQLite 实际执行、Controller/Rust 跨进程联调、正式 S/Q、旧 binary、历史重型、完整资源峰值与恢复、金融隔离独立双跑、网络 |

`admission_anchor.rs` 整文件含 B2 原 Job/终端最终 stderr、`admission_private/{mod,fd_wal}.rs` 含 S3 原算法审计与受限门面、state/audit_owner/driver/writer_cli/S2/根文件、`s_content/job.rs` 和 B2 终端测试均与 f7d 逐字相同。admission 内 write_result、read_startup、read_profile_text 等原 I/O body 未改。既有 `issue1527-formal-i1-20261018/` 和 `issue1527-i1-b2b3-20261018/` 共 **22 件报告产物**逐一保留原字节；摘要在 CHECKS.json 的 `historical_reports_unchanged`，没有回写历史结论。

复核命令为 `python3 .chanlun/review-results/issue1527-rust-h1m02-repair-20261018/verify.py`；只在需要重新验收差异时运行。收包清单命令为同脚本加 `manifest`，仅生成摘要，不重跑模型或 Rust 语法解析。MANIFEST 不含自身摘要，以避免递归。

## 保留的未完成事项

**B1 D2 生产 RAM 来源、S2/legacy 旧域计量仍精确 OPEN。** 基线现有 `LegacyAdmissionBudgetAdapterMissing` 缺口未被删除或冒充完成；本轮不造旧域费用接口、不用全拒作为成功验收。#1488 ScratchParent/Native/Recover/Carry 未触碰，不公开裸 Job，不扩旧域计量合同。

I-1 最终验收、I-2 全 S/store/metadata/shared extent 峰值归属、I-3 来源连续、I-4 同步硬 I/O、I-5 真实 OS EINTR/取消、完整 **9011**、真实恢复、**64 GiB/store**、具名 `/Volumes/AgentStorage` **至少 148 GiB** 与同卷槽位协调、金融/隔离/独立双跑及**全部六目标**均保持 OPEN。49 GB 仍是 9011 含 WAL 的库布局估算，不是输入量或 RAM 帽；没有改成 74 GiB、重测容量或批准后续运行。

未认领票、发布评论、commit、push、merge、切分支、改 Git 配置、dispatch/main/关票或开启子工蜂；未读取 .env、认证文件、钥匙串、凭据或密钥。交付停在经理可冻结、可独审的未提交差异，不自行批准后续动作。
