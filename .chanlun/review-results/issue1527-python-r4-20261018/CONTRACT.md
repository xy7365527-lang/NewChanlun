# #1527 Python R4 恢复接线合同

名分：本票工作草稿，供管理者收包及独审。固定基线 `5cb7253d46d973669e7d10cd3331e12c9ef313b1`；仅当前独立 checkout 的未提交差异。本轮没有 commit、push、merge、dispatch、切分支、认领或回票，也没有开启工蜂。

已实现已有有效 R4 锚的消费、已锚 pending 续发布以及 `Controller.start_s/recover` 的预算路径/环境转接。首次签锚和无锚既存库接管未实现。正式 Rust 类型检查、Rust 生产执行及真实恢复均为 `NOT_RUN`。

## 输入、输出与持有者

| 项目 | 合同 |
|---|---|
| 输入 DB | 实际命令的既存 DB，解析到绝对实际路径；据此派生 `<db>.control-owner.json`，不从 env 指向另一个 DB。DB 父、DB 文件保持 FD，并核普通文件、单链接、euid 与身份。 |
| owner | 严格 JSON，拒绝重复键/浮点/常量；写入口 exact `db/state_dir/admission`。普通单链接、本 euid、0600 或旧 0644。 |
| binding | exact `db/path/sha256/file_identity/binding_identity/directory_identity`，三个 identity 均 exact `device/inode/uid/mode`，必须为系统可匹配的非负整数，拒绝 bool/float/字符串/超 u64；文件 0600，目录 0700。 |
| limits | exact 八字段，值为规范十进制正整数字符串，域 `1..2^63-1`；全部来自磁盘锚，没有补值、默认或无限量。原文件 SHA；不规范化后另算 SHA。 |
| 路径 | `owner.state_dir/s-admission/{binding.json,limits.json}`，pending 固定 `.s-admission-pending`。state 本身不增加 0700 要求。逐段打开目录时无跟随；控制文件 `O_NOFOLLOW/O_NONBLOCK`，每份最多读 65536+1 字节。 |
| env | 两项 `S_CONTROL_ADMISSION_SHA256` / `S_CONTROL_ADMISSION_IDENTITY`，半组和非法形状在 E0 拒绝；双缺仍完整验证磁盘。env 只作交叉核。子进程环境从已核锚产生这两项，不输出全环境。 |
| 旧具名配置 | 如配置显式含 `admission_job_limits`，不得忽略：指向本锚 published 路径时复用原持有对象；其他普通文件只作原字节一致性交叉核，接受 0600/0644，实际读取照计且持有 FD。无此可选交叉核路径时，预算仍完整来自 exact 磁盘锚。它不具有签锚、替换预算或选择另一 DB 的资格。 |
| 输出给 Rust | `--db <实际DB> --admission-job-limits <锚内published limits路径>` 和上述两个 env；S3 继续传 `--protocol-revision s-session/3`，S 的 exact 七资源字段原函数、原值不变。 |
| 持有者 | `Controller.admission` 持有父目录、DB、owner、state、预算目录、binding、limits，以及可选配置源 FD；`Controller.close()` 统一释放。内部原字节/元数据不替换为复查对象。 |

`launch_s.sh service` 已直接 exec 当前控制器，逐字未改。裸旧 shell 的新库/一次性流程不在本轮 `start_s/recover` 写集内；不借此增加新库准备或批准旧 binary 执行。

## 状态机和持久化

| 磁盘状态 | 处理 |
|---|---|
| 有效 owner3 + final，pending 不存在 | 消费并复核原对象；成功前仍同步，不跳过此前失败的同步义务。 |
| 有效 owner3 + pending，final 不存在 | 读取 pending 内原 binding/limits；身份必须等于 owner 内原锚，binding.path 仍须是最终 published 路径；随后续发布同一目录 inode。 |
| final 和 pending 同时存在 | `AnchorConflict`，保留双方。 |
| 已锚但 final/pending 均缺 | `AnchorLost`，不创建目录或重签。 |
| 无 owner、坏 owner、owner2/无 admission、坏 binding/limits、替身 inode | 写入口拒绝；不进入 bootstrap，不删件、不修权限、不换锚。 |

续发布顺序为：原 limits/binding 文件 fsync → 原 pending/预算目录 fsync → state 父 fsync → 原 owner 文件 fsync → DB 父 fsync → 复核 → 原子不覆盖 rename（仅 pending）→ DB 父 fsync → state 父 fsync → 再核原对象。

R4 历史复审的共同成功出口是“双父同步后再核”，本实现每次调用均经过该出口；持续 EIO 即使 rename 已发生、重启读到 final，也仍失败。失败后的原件不回滚或删除，故可保留已发布但同步未完成的现场；解除故障后只能续用原锚/原 inode。

原子发布使用 Darwin `renameatx_np(RENAME_EXCL)`，Linux `renameat2(RENAME_NOREPLACE)`；后者本机未执行。平台没有该原语即拒绝，不能降级到可能覆盖并发空目录的 rename。控制器持 state/DB 的既有控制锁，状态机串行；该机制不宣称抵抗能任意删除/重建信任根的管理员。

## 期限、实读量与错误副作用

启动绝对截止在 Controller 首次控制读取前固定，值为当时单调时刻加原 `startup_timeout_ms`。`start_s/recover`、runtime probe、子进程 timeout、注册与 Ready 均使用原截止/remaining；失败清理不再得到一份全新的等待额度。

owner 第一次读取前另存真实 `first_read_at`。每次 `os.read` 成功返回即增加 `wire_bytes`，包括第 65537 字节、随后 EIO/超时前的部分读取。metadata、SHA 和解析不虚增 payload。完整 limits 解析后追认 `first_read_at + max_elapsed_ms` 及 wire；先判截止，再判 wire。Controller 可延迟读取 binding/limits，但不能延迟 t0、丢弃已读量或重新采用失败后换来的对象。失败的准备态保存首错，不重新加载；fsync 失败只允许在原已验证对象上重试。

错误仅含稳定分类、阶段、实读数量和可导出的 elapsed；不序列化单调时刻、不输出原 JSON/env/子进程 stderr。CLI 的子进程非零退出保留 returncode；坏回执统一拒绝。所有控制文件读取失败均无锚写入或删除，但控制器构造期间可能已创建两个既有协议的控制锁文件，不能声称整个目录零变化。发布成功之后发生 runtime/资源/socket/子进程失败，已完成的发布或合法 socket 清理不回滚。

读取 owner 的控制权限判断与写入准入分开：已有 owner2 可继续查状态/停止；已存在 owner3 的预算目录损坏不妨碍读取进程记录。它们均不能绕过 `start_s/recover` 的 exact owner3 与完整预算验证。S 不存在 DB 或 owner 时不创建新控制域。经济域沿用原 owner2 生命周期，仅有隔离构造回归检查。

## 与 Rust lane 的交接

Rust 必须独立验证实际 DB 派生的 owner、published 路径、binding/limits 与原 FD；不能只信传参存在或 Python env。Python 已处理合法 pending 续发，Rust 正常消费者仍应拒绝任何 pending；Rust 的 PreparedAdmission/WriterScope/Job 一次消费和真实 Existing 归 Rust lane。

Python 的 FD 不跨 exec 传送，单调 t0 不跨进程序列化。Rust 为自己的实际控制读取保留原 t0/实 wire，并由其 Job 消费；本包不把 Python 计数冒充 Rust Job 已计费，更不声称跨进程完整 I/O 账已验证。Python subprocess 的 remaining 仅保留监督器原启动总截止。每次实际开库的源身份及完整连续性仍归 Rust lane 和后续 I-3。

同步 `open/read/stat/fsync` 只有前后时限门，本包没有证明可中断硬 I/O 或真实 OS EINTR/取消。生产进程回收、实际源文件连续性和完整控制/输出资源账不能用本轮小测试抵扣。

## 证据和精确缺口

本轮根据 `ROOT-I1-IMPLEMENTATION-SCOPE.md`、`I1-RESPONSIBILITY-AND-REUSE.md` 和 `ROOT-R4-ENGINEERING-SCOPE.md` 的当前授权实施；历史助手派发和历史复审仅为工程证据，不冒充新用户签字。`PRIOR-R4-PROVENANCE.json` 明写旧源码未恢复。

证据 2672 恢复了 R2 的具名预算路径及首次发布代码片段，但其“整目录缺失就重建”已被 2734 判为缺陷，不能复用为 R4 初次发布实现。3630 记录 R3 外锚顺序及 R4 返修；3140 给出 R4 双父 fsync 与原对象复核的独审边界；4048 明确旧 Python 52/9 检查只是复用。这里没有完整 R3/R4 首次签锚的源码、无锚资格证明或新库状态机，因此只恢复有效外锚之后的消费和续发。经理如需首次签锚，须另交完整精确源码/方案与授权；不能把 R2 的缺陷分支移植回来。

`CHECKS.json` 记录本轮实际 argv/rc 和原始输出 SHA；`MANIFEST.json` 记录基线/现文件及输入证据原始 SHA。`python-r4.patch` 只含三个生产/测试文件的精确未提交差异，报告随目录另收。增量测试用真实小临时文件、临时 SQLite、真实文件系统 fsync/rename 和封闭小 Python stub，非独立审查、非 Rust 模型、非 Rust 生产执行。旧 52/9 不计入本轮通过。

完整 9011 恢复、49GB（仅含 WAL 布局估算）、64GiB/store、AgentStorage 148GiB 双跑、完整资源账与共享 extent/metadata 峰值、源连续、硬 I/O、EINTR/取消、独立评审和全部六目标仍 OPEN；#1405/#1420 生产 v4 仍不能由本包签为可合入。
