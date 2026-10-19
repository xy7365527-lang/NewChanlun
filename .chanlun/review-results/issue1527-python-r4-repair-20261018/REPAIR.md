# #1527 Python R4 三项返修

本轮已修改实际代码并补增量锁，供管理者收包。**本轮独立复评：NOT_RUN**；原独评 `BLOCK（0C/1H/2M）` 是历史结论，本报告不改写该结论，也不自签候选 PASS 或可合入。

名分：#1527 工作草稿。管理者 `/root/manage_1527_sandcastle`，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`。执行者为 Sandcastle 外部 Codex 工蜂，不声称登记为父任务原生子代理。固定 HEAD `561dd8725436077ce870d8189a583b5727d5d7d4`，tree `c3d2f2f30579183b424eddfbf74397f4f9e3d9e2`。只留当前 `codex/*` checkout 差异，未 commit、push、merge、切分支、改 Git 配置、发评论、认领票或开启工蜂。

读取了 AGENTS、根 I-1 范围、原 CONTRACT、冻结清单以及独评 REVIEW/probe/CHECKS。使用 diagnosing-bugs 的先复现再修复流程；运行时没有代码图 MCP，按已知路径读取代码。没有读取 .env、认证文件或密钥；验证环境使用空 env 和人工夹具。

## H1：控制权限与写准入分离

原反例已在固定源码重现：半组 env 使共享构造在 `ControlMismatch/E0/0 bytes` 失败；owner.admission 的 file mode 为 0644 时，在 `BindingInvalid/E1/717 bytes` 失败，status/stop 均未到达。见 `baseline-repro.stdout.log`，原独评脚本未修改。

根因是 `owner_only=True` 仍执行 `_parse_env` 和 `_validate_binding`，动作还未选择，写门已否决控制权限。修复落点：`s_session/recovery_admission.py:132` 的 `require_owner`、`:167` 的 `prepare`，以及 `s_session/s_service_control.py:295` 的 status、`:683` 的 start_q 和两个写动作装饰入口。

新的动作边界如下：

| 入口 | 验证及计量 |
|---|---|
| Controller 构造 | 固定原 startup 绝对 deadline，保持既存 DB/state 路径要求及两个控制锁；只保存两项 env 声明，不读 owner payload、不解析写准入。 |
| status/stop/start_q | 首次动作从实际 DB 派生 owner，读取并保持原 owner/DB FD，核 owner2/owner3 exact 顶层、db/state_dir、euid、普通单链接及权限；不验证 admission 内容。随后动作复核原 owner/DB，不采用替身。进程记录和进程身份原判据继续执行。 |
| start_s/recover | 先解析两项 env，再走同一租约的 owner/full binding/limits 验证及可选交叉核；只有完整写门成立才发布/派发。owner2、null 或坏 binding 不能写入。 |

**首读时机是本轮明确的合同增量**：只有动作确定后才读 owner，才能同时做到控制动作不受半组 env 影响、首次写动作仍在 E0/0 bytes 拒绝。`first_read_at` 在实际首次 owner 读取前设置；若同一 Controller 已查询/停止过再尝试写入，E0 错误必须如实保留已有 owner 字节与 t0，不能伪报零读取。原 startup deadline 始终在构造时固定，延迟动作不续期。

增量锁覆盖 owner2/owner3 + 半组 env，以及 owner3 + 坏 mode/null admission 下的 S/Q status、stop 和 Q 启动可达；stop 的 `os.kill` 全部为 mock。写入口仍拒绝；首次 start_s/recover 的半组 env 均 E0/0 bytes，修复 ambient env 后重试仍返回原首错且无读取。另核错误 db/state_dir、PID 复用仍拒绝，控制之后转写保留原计量，延迟动作不能续 startup deadline。

原测试中两项构造首读假设随边界调整：缺 owner 在 status/start_s 首次动作拒绝；t0 测试先执行真实 status 读取，再延迟并验证写准备到期、首错复用。原断言的缺锚禁止启动和已读 t0 不重置均保留并加强。历史 CONTRACT/CHECKS/MANIFEST/patch 不动。

## M1：可选文件失败纳入同一准备态

原脚本两个反例均重现：同字节 0666 文件先报 Identity，0600 的 `{}` 先报 ControlMismatch；移走旧文件并补同字节 0600 新 inode 后，同一 Controller 均多读 241 字节并到达一次启动 stub。

根因是可选交叉核失败未进入 `_preparation_error`，下一次调用可以重开路径。修复落点 `s_session/recovery_admission.py:399`：首次可选路径被固定；读取、权限、内容及复核失败锁存为同一个准备错误。`prepare` 在 `_loaded` 快路前先返首错，因此 start_s、直接 publish、改成锚内路径都不能绕过。后续没有 reopen，没有新增 FD/计量，没有续 t0，也不发布 pending。

增量锁覆盖同字节 0666、内容 `{}`、真实返回 7 字节后注入 EIO 三种情况；首次计量分别为已读锚量加 0/2/7。换 inode 后 patch 掉全部 `os.open`，重试仍返回同一个异常对象、原 FD 列表和原 t0/计量，close 后 FD 均不可再用。

fsync 失败不写入该准备错误。另用已成功交叉核的配置文件，对 DB 父/state 父分别注入发布后的持续 EIO，同 Controller 连续两次拒绝；解除故障后 start_s 到达封闭启动 stub，configured FD、t0、wire 不变。原 pending 发布顺序、双父同步和末次原对象复核保留。

## M2：动作错误保留控制 prelude

原反例重现：实际小 Python recover stub 退出 7，租约已有 `E4/1372 bytes`，CLI 却只有 category/returncode，缺 stage/wire/elapsed；私有 stderr 标记原本未泄漏，此性质继续保持。

根因是 CLI 只为 AdmissionError 导出计量。修复落点 `s_session/s_service_control.py:201` 的 `admission_action`：在 start_s/recover 控制器边界捕获稳定动作阶段和原租约计量，附在原异常上，不另造 Job、异常文本或计时起点。非 AdmissionError 的首次动作失败锁存，重试返回同一个异常和同一份收据，不重新运行 probe。AdmissionError 的原阶段、计量、elapsed 和准备首错不被覆盖；合法 fsync 续发保留。

收据约定：`action` 标识写动作，`action_stage` 标识当前步骤；非 AdmissionError 的 `stage` 为 `runtime_probe` / `recover_process` / `recover_reply` 等稳定值，`admission_stage` 保留租约 E 阶段。AdmissionError 的 `stage/admission_stage` 保留其原失败阶段。`wire_scope=python_control_prelude` 明写范围，`wire_bytes` 是真实控制文件读取，`elapsed_ms` 从原 first_read_at 计算，绝不导出单调绝对时刻。已知 subprocess returncode 才附回执；非零保持原值，坏回执保留成功退出的 0。

增量锁通过实际小 Python stub 覆盖：start_s/recover 的 runtime probe 非零退出 9、probe 输出 null 造成 TypeError（returncode 0）、recover 非零退出 7、recover 非 JSON 回执（returncode 0）。公开收据逐项核 stage、E4、三份原文件的实读总和、elapsed、returncode，且不包含 stderr 标记或 t0。同 Controller 重试另核原异常、原收据、原 t0/计量保持、probe 只调用一次。旧重复键回执拒绝锁继续通过。

## 验证记录与收包

`CHECKS.json` 每条记录保存真实 argv、rc、耗时、执行时三个源文件 SHA，以及 stdout/stderr 原件 SHA。以下次数不累加为独立测试数量：

| 记录 | 实际结果及含义 |
|---|---|
| baseline-repro | rc=0；旧原脚本三组均 REPRODUCED，表示缺陷复现，绝非候选通过。 |
| regression-red | 新 7 个测试在原生产源码上 rc=1，1 failure / 6 errors；控制阻断、失败后 reopen、收据缺失均可捕捉。 |
| regression-green-1 | 第一版修复后 7 个增量测试通过。 |
| targeted-compatibility-1 | 受影响的本票文件 41 测试，rc=1；两项旧构造首读假设失败。日志保留，未删除或重写。 |
| targeted-compatibility-2 | 调整两项读取触发点并补延期 deadline 锁后，本票文件 42 测试通过（34 原测试 + 8 增量测试）。耗时约 2.3 秒，并非仓库全量。 |
| receipts-final | 后续仅补 action_stage/admission_stage 注记后，重跑相关 4 项通过。 |
| repair-final | 强化部分读取及公开实读量断言后，最终源码上的 8 项增量测试通过；不把它再加到 42。 |
| static-final | 三个源文件 AST、固定 HEAD/tree、9 件冻结基线核对、6 份历史报告原字节、写集、diffcheck、精确 patch 及反向应用检查。 |

42 项记录对应 CHECKS 内当时源码；最后的改动是收据注记及测试断言加强，由 final 两组覆盖。其余已通过行为证据复用，不无理由重跑大套。原独评 8 组正控和旧作者 34 项日志仍是原证据，不被改写为本轮独评或新增成绩。

交付 `python-r4-repair.patch` 是固定 HEAD 到当前三个生产/测试文件的精确 diff。报告目录另交；`MANIFEST.json` 列输入、基线/当前源 SHA、报告/日志/patch 的 SHA 与长度（不自哈希）。生产写集只有 `s_session/recovery_admission.py`、`s_session/s_service_control.py` 和本票测试文件。

## 未执行与保留义务

本轮独评、真实信号/进程回收、硬 I/O/掉电、OS EINTR/取消、Linux renameat2、Rust 类型检查/构建/正式恢复、正式 S/Q、旧 binary 均 **NOT_RUN**。未运行 cargo/rustc/build、网络或历史重型验证；未改 Rust、CI、Cargo、launch_s.sh 或其他工位。封闭临时文件、临时 SQLite、真实 fsync/rename 与 Python stub 不能抵扣这些义务。

有效外锚消费和已锚 pending 续发布仍是唯一写路径；没有 bootstrap、首次签锚、既存无锚接管、默认限额或另建 Job。完整 9011、49GB WAL 布局估算、64GiB/store、148GiB 卷独立双跑、完整资源归属/metadata/shared extent 峰值、源 RW 连续、硬 I/O、EINTR/取消、同核金融及恢复、全部六目标均保持 OPEN。#1405/#1420 生产 v4 的 BLOCK_NOT_MERGEABLE 不由本包解除。
