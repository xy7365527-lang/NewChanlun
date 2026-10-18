# #1527 I-1 实施候选交付

本包交付当前独立 checkout 的真实 Rust 差异，状态为 **BLOCKED / BLOCK_NOT_MERGEABLE**。已实现 D1 受限来源域的固定 FD 私有检查、非零物理基线测量、同一 Job 起钟与前导回灌、writer/maintenance 双锁，以及 S3/S2/legacy 的准备态和开库资格传递。**未完成 D2 生产容量接线、私有完整业务规则计费，以及受写集限制的全部外围 I/O 计费**，因此不报 I-1 全部完成。

固定 HEAD：`5cb7253d46d973669e7d10cd3331e12c9ef313b1`。管理者：`/root/manage_1527_sandcastle`；父线程：`01a12e01-45a1-7173-a3e8-d338d108fb93`。我是外部 Sandcastle Codex 工蜂，本包不声称已注册为父任务原生子代理。本目录是绑定 OPEN #1527 的工作草稿。

## 来源与复用

- 只读取得 `r4-repair-worktree` 的 `admission_anchor.rs`、`anchor_tests.rs`、`model_checks.py`，以及四份修正提案。四提案保留为 `INHERITED-*`，只表示继承材料；其中 JOB/CALLERS 的代码已应用后继续实现，不能将旧提案直接再套到本候选。
- M-01 修正保持：`StartedAdmission` 私有持 Job，provider 只借 `AdmissionAccounting`；无 `job_mut`、`Deref<Job>`、`AsMut<Job>` 或可替换原账的公开槽位。已有 `charge_wire_bytes` 整个函数及 Job 原文件其余部分逐字不变，仅增加 `new_started`。
- D1 精确源：`b9f38106a5c0cc6f320798cf146e3bc7a6677c1b:rust/src/bin/p1504_d1/vfs.rs`。适配落 `admission_private/fd_wal.rs`，没有导入 prototype target、driver 或 feature。主要变化是接受 E6 固定 File、借受限记账门面、逐次 pread 前后计费、删除原型轨迹容量、删除三次 EINTR 重试魔数、取消测试驱动方法。仍使用 SQLite 自己建立私有 WAL 索引，未写 WAL/页解释器。
- D2 精确源：`e1d7e0b6bffb78f757e59cdfe3e35f3618e6790a` 的 `p1505_fd_journal/{overlay,vfs,allocator,driver}.rs` 及顶层 Limits 已只读核对。**D2 机制没有冒充已在本包生产接线**；下面说明其具体缺项。
- 先读父目录 `xerj-*query.json` 缓存，只抽取命中路径与片段；没有网络、安装或重索引。图 MCP 未提供可调用工具，代码发现退回定向源码检索。
- Pythonlane 仅交叉核 `--admission-job-limits`、两个 env、owner3/binding6/identity4/limits8 与 published/pending 名称，不导入/执行其在制品。输入 SHA 在 CHECKS 中，不把它当冻结的 Python 实施验收。

`INHERITED-SOURCES.json` 记录来源 SHA；历史审查/原型 CI 成绩不计入本轮执行结果。当前 HEAD 是指定候选基线，不冒称 main producer 已获准。

## 已落地接口和所有权

`PreparedAdmission` 保存唯一真实首读时刻、冻结限额、原控制 FD lease 和实际前导量；CLI 的 SResources、ClockPlan、首遍 profile、one-shot input 在 Job 尚未构成时经 `read_file` 分段读入同一前导账。成功读取后解析失败尽可能带回该账的诊断；原 loader 的期限优先追认规则保持。

`WriterScope::admit` 按值消耗准备态，核 requested DB 与 lease DB，然后按 writer → maintenance 顺序取得 `try_lock`，每个锁调用前后复核原 lease/期限。writer 名字复用现有 `process_lock` 的 `<canonical-db>.writer.lock`；本实现维护锁命名为 `<canonical-db>.maintenance.lock`，无等待或锁失败降级。固定基线无旧 maintenance 实现，不能声称已证其他进程也遵守这个新维护锁约定。

E6 的 `MeasuredDomain` 遍历 DB basename 家族和 owner 指定的整个 state 子树，包含 DB、WAL/journal/SHM、控制原件、锁、目录 metadata 和可识别私有残件。每个普通文件核单链接/无 symlink，记录 dev/inode/uid/mode/nlink/len/blocks/纳秒 mtime/ctime，固定只读 FD。文件与目录物理量按 OS `st_blocks * 512` checked 累计，显式核 frozen max_store/max_staged 与 64 GiB/store 原门；不以逻辑长度估分配。未知 DB 侧件、别名、特殊文件和未知私有残件具名拒绝。未删除任何未经证明归属的残件。

测量结果构成 `Existing`，然后才执行唯一 `Job::new_started` 和一次前导回灌。staged 的 0 来自此受限域没有暂存文件的实际枚举；store 来自真实已分配量，未填 `Existing {0,0}`。文件系统隐藏 inode/xattr 开销、共享 extent 独占归属和全域峰值仍未证，详见 I-2 保留项。

E7 只在 `MainOnly` 或 `Wal` 介质调用 D1：源 File 只读，SQLite 显式选择本次非默认 VFS，所有 write/delete/truncate 拒绝，未提供 SHM 文件写口，无磁盘 clone。私有 SQLite 执行 `quick_check`，取完并关闭 statement/connection，再注销 VFS。其**新增磁盘文件分配上界确实为 0**，因此原 Job 中 reserve_staged(0) → reconcile(0) 是对这一机制的实际预留/收尾，不是用 0 掩盖未知 clone 分配。SQLite 自有内存不冒充已纳入磁盘账。失败也先关闭私有对象，再对账，不自动清掉原 Job 的停止状态。

E8 使用消费型 `seal` 调用原 Job finish_check，随后核完整源 membership/metadata 和原期限，再创建有资格的 scope。`FinishedAdmission` 保存的仍是同一 Job，供 CLI 最后输出；不返回裸 Job。S3 one-shot/recover 的 `CommandOutput` 仍持 scope，到 dispatch 的实际 write 返回 n 时记账，再收尾。S2 recover 的成功 stdout 同样计入原账。服务在监听前明确结束启动账，运行期 E9 不把已结束的启动 deadline 当永久服务期限。

`WriterAdmission` 的两个变体只表达“尚未准入的 Prepared”与“preflight 已取得的同一 scope”，不表示两个判据。advance preflight 先 admit，检查通过后把同一 scope 移给 Driver，避免重复取锁/重复创建 Job。所有 S3 Driver 包装都转移该一次性对象；直接 S3 SQLite open 前均 require_admitted。S2 serve/recover 和 legacy accept/advance/recover 安装 guard；根唯一 `open_db` 每次都转发当前 guard 资格。初始化/读命令仍按原分支工作，本包不新增 bootstrap、无锚接管或迁移授权。

运行期只复核主库/目录对象身份与原 lease/锁，不要求合法写入后的 mtime/长度仍等于启动前值。末次 pathname 检查不能证明之后连续性，特别是新 WAL/SHM 的来源连续性仍 OPEN。

## 成功域与失败域

| 情况 | 当前行为 |
|---|---|
| 既存非空主库，有完整有效 R4 锚，无 journal/SHM，物理基线和前导计费在限额内，私有 quick_check 成功 | 源码存在完整准入成功路径；新 Rust 正控源码覆盖，**运行 NOT_RUN** |
| main + WAL，无 SHM/journal，固定 File 和来源核验通过 | D1 私有 WAL 机制；无失败后默认 VFS 重判 |
| journal 存在且无 WAL/SHM | `D2ProductionCapacityMissing`，保留 main/journal；不打开原件 SQLite |
| SHM、journal+WAL、未知 DB 侧件或无法归属的私有残件 | 具名拒绝，不隐藏介质状态或删掉残件来制造成功 |
| 控制锚缺失/变化、锁不可得、真实 Existing 超额、实际 wire/原 deadline 超限、私有 SQLite 失败 | 不产生 admitted，失败路径不进入原件 SQLite |
| admitted 后原件业务审计/命令执行或 stdout 失败 | 仍可能已经打开过原件；不冒称“整个命令所有失败都零原件 I/O” |

当前私有检查只做 SQLite 完整性，**未完成完整 S3/S2/legacy 业务规则的私有预审与 node/record 全费用**。因此这里的源码成功域只证明这一候选功能链可表达，不构成正式恢复准入签字。现有原件业务审计仍保留，不用 quick_check 代替其判据；把它前移到私有同父账的精确受阻点见下节。

## 需要管理者/根处理的三个具体边界

### B1：D2 生产 RAM 额度没有来源

精确源的 `Limits` 六项为 `max_source_main_bytes`、`max_source_journal_bytes`、`overlay_payload_capacity_bytes`、`overlay_metadata_capacity_bytes`、`evidence_capacity_bytes`、`sqlite_heap_limit_bytes`。其中源码文件大小可以来自本 provider 测量；其余不能从 Job 的 `max_store_bytes` / `max_staged_bytes` 推导：这两项约束磁盘分配，64 GiB/store 不是 heap 帽。SResources7 的 `audit_cache_source_bytes` 约束跨请求保留源，也不是启动私有回滚内存。D2 原 driver 通过 `allocator::install(sqlite_heap_limit_bytes)` 安装全局 SQLite allocator；本 lane 明禁把它搬入正式 producer。

可由根选择的最小工程路径：

1. 给出已冻结、属于原父资源所有者的私有恢复 RAM reservation API 和明确额度来源，按真实申请的 overlay payload/slot metadata 与 SQLite 内存约束逐项接入；不得读取 #1488 未冻结 WIP，也不需要等待 I-2 完整验收。
2. 明确裁定独立的私有恢复预算输入与授权 owner，之后才设计其 exact 合同。当前没有获准新增字段，不能自行选这一路或提供数值默认。

本包没有为了等 B1 停在 loader：真实测量、D1 私有机制、锁和入口均已落地。D2 未被全域 AllocationUnproven 拒绝替代，也没有用 D1 在 D2 失败后重判。

### B2：当前写集禁止必要的外围读写钩子

精确现有位置记录于 `CALL-SURFACE.json`：根 `load_profile` 的 `std::fs::read_to_string(path)` 被 `audit_owner::startup` 再次调用；根 `cmd_accept` 直接读 input；`cmd_accept/cmd_advance/cmd_recover` 直接 println；根 `main` 在 run 返回、guard 已析构后 eprintln。当前授权对根文件只允许 guard/每次 open_db 转发，不能改这些 body，也不能改 audit_owner。

已接：S3 CLI 首遍资源/clock/profile/input，S2 clock，S3 正常输出、S2 recover 正常输出。未接：上述重复 profile、legacy input/成功输出，以及所有早期失败最终 stderr 的实际写量。**不能按文件 metadata 长度补计，也不能声称未计字节为 0。** `read_startup` 已提供同父账计量读取接口，剩余需根扩展这些具体调用点写集，并决定如何把失败账对象带至最终错误 sink。S3 Driver 的启动栈、S2/legacy 已有同 scope guard，可用于该窄转发；不能将原失败账丢掉后再开一个报告 Job。

### B3：完整私有业务审计的安全门面接缝

`state::audit_with_access` 要求 `&mut dyn s_content::access::Access`；`Access::owner_job` 的合同要求返回可变 Job，而 M-01 禁止 provider 获得这种可替换槽位。更直接的原算法 `state::audit_with_budget(tx, deadline, max_render_bytes, Option<&dyn audit_budget::Budget>)` 当前私有，父/兄弟模块无法调用，且 `state.rs` 不在本 lane 写集。

最小可审方向是把这一个原算法入口显式开放给本票私有模块，使用内部共享 RefCell 的受限 AdmissionAccounting 实现原 `audit_budget::Budget`，让 node/record 与 VFS 实读共用原 Job；不改变判据，不引另一本 Job，不假报 `owner_job=None` 来跳计。还需把原业务 SQL 所需的 read-only authorizer 动作按实际调用列出，不能用宽松 fallback。S2/legacy 同样需要识别原完整审计入口与费用钩子。本包没有擅自暴露裸 Job 或复制旧业务算法来绕过写集。

上述 B2/B3 是本轮独占写集与验收要求的具体接口冲突；不是把 I-2..I-5 完整验收重新设为 I-1 前置。

## 本轮验证与未执行项目

实际结果以 CHECKS 的 argv、rc、输入 SHA 和输出为准：rustfmt 最终语法解析通过；限定静态/合同 AST 检查通过；6 项纯账本/小临时文件模型通过；精确 patch 在固定基线临时文件镜像上检查并重建比较。Rust 测试只留源码：非零基线与不改原件、双锁 failfast、具名 D2 拒绝、坏库不生成 WAL、同字节换 inode、未知侧件等。旧 Driver 的 36 处构造/恢复测试调用已补真实 TestOnly 发布锚后走 loader；CLI 测试 helper 也补必填 flag，不使用生产默认锚。

过程中一次 rustfmt 拒绝新增测试的中文 byte literal，已改为 UTF-8 `as_bytes()`；一次纯模型错误假设 seek 会稀疏分配，本机实测 1,052,672 分配字节 / 1,048,577 逻辑字节，已改为核物理量及其与逻辑长度的差异。旧失败记录原样保留，未改报成功。

**cargo / rustc / build / Rust 测试 / 实际 SQLite / 正式 S/Q / 旧 binary / 历史重型 / 网络全部 NOT_RUN。** 语法检查不证明类型、借用、FFI ABI、VFS 生命周期或真实恢复行为通过；小文件模型不是 Rust 验收。没有改 Cargo/CI/Python/audit_owner/native/request_audit/capture_allocation，没有开启工蜂、认领/评论、提交、push、merge、切分支、改 Git 配置或读取凭据。

完整 9011、49 GB **含 WAL 的布局估算**、64 GiB/store、具名 AgentStorage 148 GiB 双跑准入、I-2 全资源/共享 extent/metadata 峰值、I-3 连续来源、I-4 同步硬 I/O、I-5 真实 OS EINTR/取消、金融/隔离/真实恢复以及独立双跑义务均继续 **OPEN**。本包不批准后续运行、main 合入或关票。
