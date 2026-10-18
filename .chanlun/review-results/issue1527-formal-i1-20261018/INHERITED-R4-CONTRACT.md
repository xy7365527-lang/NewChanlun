# #1527 R4 持久锚消费者候选合同

本文件是 OPEN #1527 的工作草稿，M-01 修正接口合同；前轮结果原件不改写，本轮证据见 `../issue1527-r4-repair-20261018/REPAIR.md`。固定基线为 `5cb7253d46d973669e7d10cd3331e12c9ef313b1`；原候选来自 `codex/1527-r4-anchor-candidate-20261018`，本轮在独立返修 checkout 继承，均非 main 准入。管理者为 `/root/manage_1527_sandcastle`，父任务为 `01a12e01-45a1-7173-a3e8-d338d108fb93`。外部工蜂真实 session_id 由运行时登记，本包不声称已注册为父线程原生子代理。

## 交付与来源

前轮新增、本轮原字节继承 `rust/src/bin/s_session_v3/admission_anchor.rs`，实现既有锚的只读消费者和一次消费准备态；新增 Rust 测试源码、可执行离线模型及接口提案。没有修改既存文件，没有注册生产模块，没有实现或运行正式恢复。Rust 测试和类型检查为 **NOT_RUN**；离线模型成功只能证明模型与小文件试验域。

根本轮完整采纳 `PRIOR-R4-V3-DESIGN-TRANSCRIPT.md` §1–6，准入记录为父目录 `ROOT-R4-ENGINEERING-SCOPE.md`。恢复来源是历史工具输出和旧助手工程派发，**不是旧源码已找回、历史检查重跑或用户新签 SPEC**。`SOURCE-EVIDENCE.json` 分开绑定设计来源、当前根准入、父目录票面、XERJ 既有查询、前轮 main 工具宿主报告和固定 5cb 源码。

历史 52/9 检查不计本轮通过。前轮的原件缺失结论已由当前设计来源和根采纳补齐，本轮不再仅交模板。新库 prepare、旧库无锚接管、迁库均属旧 §7，未纳入实施。

## 磁盘合同

路径由命令实际 DB 的 canonical 绝对路径派生。消费者仅打开既存普通 DB，固定其父目录与 DB FD；owner 为该 DB 文件名追加 `.control-owner.json`。env 中的 db 不参与选择磁盘根。CLI 的 `--admission-job-limits` 必须明确指向 owner 声明的 `state_dir/s-admission/limits.json`，不能用另一份同字节文件替代。

| 对象 | exact 字段与类型 | 文件及权限 |
|---|---|---|
| owner | `db`, `state_dir`, `admission`；前两者为已解析绝对路径，admission 必须是有效 binding 对象 | 普通文件、单硬链接、euid；0600 或历史 0644 |
| binding | `db`, `path`, `sha256`, `file_identity`, `binding_identity`, `directory_identity` | 普通文件、单硬链接、euid、0600；实际自身 FD 匹配 binding_identity |
| identity | `device`, `inode`, `uid`, `mode`；非负 JSON 整数；不接受 bool、浮点、负数或字符串；转换到 libc 本机域必须成功 | 文件 mode=0600，published 目录 mode=0700；uid=euid |
| limits | 下面 exact 八字段；值为 `[1-9][0-9]*` 字符串，1 至 2^63−1 | 普通文件、单硬链接、euid、0600；实际 FD 匹配 file_identity |

八字段逐字复用 5cb 的 `JobLimits::from_canonical_json`：`max_total_wire_bytes`、`max_total_node_bytes`、`max_total_nodes`、`max_total_records`、`max_total_requests`、`max_elapsed_ms`、`max_staged_bytes`、`max_store_bytes`。不增加默认值，不扩大数域，不替换现有 S 资源档七字段。全部 JSON 先走现有 `strict_json`，嵌套重复键也拒绝。

owner.admission 与实际 binding 按 typed 字段相等；binding 的 db 等于实际 DB，path 等于 owner 给定 published 路径。SHA 为 64 位小写十六进制，核 **原始 limits 字节**，不核重新序列化后的 JSON。state 目录不要求 0700，published 必须匹配持久 directory_identity；正常 process 记录更新导致的 state mtime 变化不构成预算变化。

读取目录从 `/` 逐分量 `openat`，每个分量使用 `O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC`。控制文件相对固定目录 FD 只读打开，使用 `O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC`，拒绝 symlink、FIFO、硬链接及错误权限。禁止 basename 带 `/`、`.` 或 `..`。不通过路径重读采用新 authority。

每个控制文件读取前后比对 dev/inode/uid/mode、nlink、type、size、mtime/ctime 纳秒快照。最多实读 65536+1 字节，65537 也计费后拒绝；不根据 stat.size 猜实读量。每次 `read` 成功立刻累计 n，后续 EIO/Interrupted 保留已读量；不使用会掩盖部分成功累计的整体 `read_to_end`。Interrupted 当前闭锁拒绝、不重试，此行为没有取得真实 OS EINTR 验收。

## 生命周期和不重签

`state_dir/.s-admission-pending` 任意目录项存在即拒绝，包括悬空 symlink、final+pending 和仅 pending。已有锚而 final/binding/limits 丢失时拒绝，连续尝试也不创建、不删除、不重签。owner 缺失或 owner 不含有效 admission 时也拒绝，双 env 缺失不取消磁盘检查。

历史 producer 必须继续遵守 **pending 持久 → 外置 owner 锚持久 → 发布**，成功返回前完成 **owner 所在 DB 父目录与 published 所在 state_dir（发布父目录）两处 fsync，并复核身份**。不能因发布路径已经存在而跳过先前失败的 fsync；只有 producer 可按原协议续原 pending。历史 `2734` 的整目录丢失重签、`3630` 的 rename 后 fsync 重试假成功、`4048` 的只看 env/文件 FD 而没有最后路径复核，是本轮保留边界的来源。当前消费者不写入也不负责 fsync，文件存在本身不能证明 publisher 的持久化资格；5cb 的 Python 仍不含恢复 R4 producer。

`PreparedAdmission` 和 `ConsumedAdmission` 不实现 Clone/Copy，字段私有，无外部伪造构造器。`consume(self)` 在 E5 复核并按值移交；失败销毁准备态，不能拿回旧 t0 再起一次。准备态中的 `Arc<BudgetLease>` 持有 DB、父目录、owner、state、published、binding、limits 原 FD 及原控制字节。当前消费结果不是 WriterScope 已准入标记；真正 Job 回灌和最终消费的代码仅在 `INTERFACE-JOB.patch` 提案里，未应用。

lease 不含时钟或可改限额。最后复核重新走实际 DB 父路径、owner 绝对路径、state/published 路径，以及固定目录 FD 下的相对 basename；同时比对原 FD 的当前完整快照。新打开的 FD 只用于比对后关闭，绝不替换原 FD、原字节或原限额。同字节换 inode、父目录换位、原件就地修改、pending 新出现均拒绝。元数据复核不产生 payload，不加 wire/node/record/request。

`require_admitted` 应保存 Arc 并在实际原件开库前再复核。长驻服务后续再次开库只复核 lease 与源身份，不沿用已结束的启动 max_elapsed；新请求仍使用自己的原请求期限和预算。

## 时钟、错误顺序和账

t0 使用首个 owner open/read 之前的真实 `Instant::now()`。在限额尚未可信时仅受每件 64 KiB 有界读保护，错误标明 `budget_trusted=false`，不能声称已有 Job、没有超限或实读为零。owner/binding/limits 验证出可信八字段后，立即按 t0 追认累计量和唯一 deadline：`t0.checked_add(max_elapsed_ms)`，达到截止即失败；溢出或未来 t0 也失败。

仅可信 prelude 的边界先报 Deadline，再报 wire Limit；此处两个同时超过时 Deadline 优先。Job 构造之后的实际 I/O 保留原 charge_wire_bytes 先累计、wire 帽后 deadline，受限门面不在此方法前插期限门。没有 load→consume→Job 的重新起钟。源码 `ControlPrelude::gate_at` 的时间参数仅供同模块单元测试制造边界，生产入口调用真实 now；不以假 Clock 回灌起始时刻，不填零 Existing。

| 阶段 | 本模块/接线责任 | 错误和外效 |
|---|---|---|
| E0 | 参数、双 env 形状、实际 DB 定位 | 未读控制 payload；env 只允许同时缺或同时存在 |
| E1 | owner 原件读取、exact owner3、实际 DB 核验 | AnchorMissing/Invalid 或 Identity；保存真实累计 |
| E2 | pending 缺失、state/published 身份 | AnchorUnpublished/Lost/Changed；不修复现场 |
| E3 | exact binding6、limits8、原字节 SHA 与 FD | BindingInvalid/ResourcesInvalid/Changed；不采纳部分字段 |
| E4 | 可信期限/wire、双 env 交叉核、最后路径复核 | Deadline 优先 Limit；之后 ControlMismatch/Changed |
| E5 | 本模块 consume；根协调 writer 锁后再核 | 本模块不取锁；未来锁文件可能创建，不能宣称整个准入零外效 |
| E6 | 根的原私有清理、源侧件、完整基线、allocation provider | 本轮未实现、未更改算法；不能以全拒绝代替成功域 |
| E7 | 提案 new_started + 一次回灌 + 原私有准入 | 真 Existing、原 t0；构造失败仍保留控制实读量 |
| E8 | 提案 lease 最末复核、同 Job finish、源身份后置核 | 未接线；源身份必须由 scope 完成后才标 admitted |
| E9 | scope 保存 Arc 并在原件开库前复核 | 未接线；不是 SQLite pathname 全生命周期连续性证明 |

`AdmissionError` 只导出稳定类别、E 阶段、真实 t0、已读 wire、限额是否可信。Display/Debug 不带 JSON、env、路径或下层异常原文。后续 Job 总 wire 使用 u128；提案的 Job 收尾错误单独保存 u128 总账，避免截断成控制前导的 u64。CLI stdout/stderr 实际计量依旧归外围 I/O 接线，不用错误文本长度自引用替代管道计量。

## 验证边界

离线模型覆盖有效锚（有/无双 env）、owner 0644、空/无效/重复字段、非法整数、同字节换 inode、symlink/FIFO/hardlink、就地修改、换目录、部分读失败、65536 与 65537 边界、期限/wire 双超限、pending/丢失、单次消费和 FD 生命周期。模型不导入 Rust 或生产 Python；使用隔离临时小文件，库文件只是普通字节，不打开 SQLite。所有模型数值明确为 TestOnly，不是生产额度。

Rust 源码测试写在 `s_session/tests/issue1527_recovery/anchor_tests.rs`，由新模块的 cfg(test) 路径引用；目前模块尚未注册，因此它们尚不可经现有 Cargo target 自动发现，更没有执行。`rustfmt` 只证明语法可解析/格式一致，不证明类型、unsafe ABI 或行为正确。本目录 CHECKS/MANIFEST 是前轮历史记录，保持原字节；本轮实际 argv、退出码、最终写集摘要统一见返修目录 CHECKS/MANIFEST。

生产恢复仍为 **BLOCK_NOT_MERGEABLE**：正式 producer/R4/CLI 接线、真实 Rust 构建/具名测试、返修后独立复评、完整 baseline/provider、完整 9011/真实恢复、64GiB/store、148GiB 具名卷双跑、来源合流、真实恢复资格、同步 I/O 硬截止、监督器取消和 OS EINTR 均未由本包证明。最后 stat/open 后仍可能发生路径变换；信任根管理员可重造完整控制域，这不是消费者可以抵抗的威胁模型。没有扩大原帽，没有把拒绝一切称为成功。


## M-01 原账绑定与历史结果

原作者“独立评审 NOT_RUN”仅是前轮提交时的事实；其后本轮收到独评 **BLOCK，M-01=1、C/H=0**。本返修是作者修正与限定自查，没有新独立 PASS。受限 `AdmissionAccounting` 的完整签名、provider 调用及失败析构/finish 语义见 INTERFACE-PLAN 最后一节。原消费者、anchor_tests.rs、model_checks.py、verify_delivery.py 原字节保留；只在未应用 JOB 提案中添加门面，删除 job_mut，并让 finish 错误保留原 JobSnapshot。字段私有且无裸借用出口，调用方不能替换原账；丢弃/替换门面不清除父账的 wire/t0/预留。原账合法对账后仍可 finish 成功。

资源口径以 #1489 原签 comment6159518677 及 ADR 0037:24 为准：49 GB 是 9011 全库布局**含 WAL 的估算**（主库 33 GB、含 WAL 49 GB、双跑 99 GB），不是行情输入量。64 GiB 是每 store 容量/分配账帽，不是 RSS；148 GiB 是具名 `/Volumes/AgentStorage` 卷双跑可用空间准入，不是可除二的单跑帽。完整 9011、真实恢复、完整资源账、metadata/共享 extent/峰值、allocation_upper、来源/FD/SQLite pathname 连续性、同步硬 I/O/EINTR、独立双跑均保持原义务，仍 OPEN；没有以措辞订正削减验收。
