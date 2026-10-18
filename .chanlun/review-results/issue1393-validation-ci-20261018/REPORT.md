# #1393 必要 CI 实施交付（False，待管理者独审）

本目录为绑定 #1393 的工作草稿。外部 Sandcastle Codex 工蜂在独立 `codex/1393-validation-ci-20261018` checkout 实施；父任务为 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者为 `/root/manage_1393_sandcastle`。未声称注册为其原生子代理，真实 session 身份由运行时事件登记。

本轮交付的是**默认拒绝执行的必要 CI 文件及本地 toy 证据**，没有批准或执行远端验证。固定基线和结束 HEAD 均为 `ab3f3fb0e1638f1ef3543b8b1948cd1856dc8675`，tree 为 `3c383be7f45fe55d4a270ac3ef32a0b253de793b`；保留未暂存 WIP，由管理者固定最终提交。没有 commit、暂存、push、dispatch、merge、切分支、Git 配置修改、tracker 操作或新增工蜂。

## 实际写集

| 路径 | 内容 |
|---|---|
| `.github/workflows/issue1393-validation.yml` | 新具名 workflow_dispatch；唯一 `ISSUE1393_EXECUTE: 'False'`；默认分支在 checkout 前退出，所有执行步骤都要求精确字符串 True；60 分钟 job；同一开始时间、缺件记录、失败原件上传 |
| `.github/scripts/issue1393_validate.py` | 新独立 runner；默认 False；完整合同/实际 source/workflow 绑定、单次 target 构建、Cargo JSON/list/exact、Lean/fixture、共享截止、回收和封包 |
| `.github/scripts/test_issue1393_validate.py` | 独占标准库纯模型及临时 Python/shell toy 测试，不调用真实 cargo/rustc/lake/fixture/产品 |
| `.github/workflows/ci.yml` | 仅新增五行：fixture job 在已有 `lake build` 后显式执行 `lake env lean Origin/RawInclusionInitialization.lean`；原 build/fixture 和其它内容逐字保留 |
| 本目录 | 编码前复用记录、False 合同草案、报告、静态复核脚本、各轮检查原件、WIP 摘要与验证收据 |

产品 Rust/Lean、s_session Python/JS、其他 workflow 均未修改；没有访问主仓/返修工位或 #1419/#1488/#1394 冻结工位。新路径开工时不存在，无未知 WIP 覆盖。完整逐文件 SHA256 和路径见 `WIP-MANIFEST.json`。

## 编码前读取与复用

`REUSE.md` 记录了代码前核查。已读 CLAUDE/AGENTS、ask-matt、Wayfinder、delivery/generation、CODING_STANDARDS、原 REPORT/INDEPENDENT-REVIEW-01 与仓外 `VALIDATION-CONTRACT-DRAFT.md`。原 reference-evidence JSON 在本 checkout 只读复用，无联网或凭据读取。

原 telemetry 壳原样使用，提供 2147483648 字节**每次命令的起始磁盘门**和 10 秒资源采样；Rust 的独占 target 放在 `rust/target/issue1393-{lib,s}`，原 `du target` 能覆盖其占用。Lean/fixture 仍保 free/df/进程采样；其 `du target` 不等于 `.lake` 的专项存储统计。没有把这些读数称作 OS 内存/store 硬帽。

从已有 `tb02d_harness.py:execute` 复用进程组、TERM/KILL、wait 和错误留存思路；没有复用其每次 run 重置截止。Linux runner 领养同组孤儿，直接子进程与同组残留均检查；这是进程组监督，不宣称限制任意自行 setsid 的进程或提供 cgroup 沙盒。Linux subreaper 分支本机未运行，须在最终远端验证中核实。

## 执行合同

### 单一 60 分钟窗口

workflow 第一个执行步骤在 checkout 前写一次 monotonic/墙钟起点；所有 Git 核验、官方 elan 安装、版本查询、Rust、Lean 和 fixture 子命令都使用这个共享起点，剩余时间取两种时钟的较小值。job 外层保持现有 60 分钟上限。

主体工作最迟在总截止前 30 秒停止，回收最多使用其中 5 秒，余量留给封包和上传；这属于 60 分钟内的时间分配。没有任何步骤重新得到 60 分钟。安装使用原 CI 的官方 elan 命令；Rust 沿用 ubuntu runner 已有工具链，不安装项目依赖或扩 features 域。

每个步骤保存精确 argv、cwd、剩余时间、原 stdout/stderr、实际退出、信号、TERM/KILL 记录及回收结果。前步失败/超时/中断/缺原件即停止依赖步骤，完整未执行名单写 NOT_RUN。telemetry 退出与实际命令退出分开记，不能用外壳 exit0 代替实际命令收据。

### Rust 精确清单与命令

lib 固定默认 features（Cargo JSON 中为 `default`）；S 固定 `default,s_session`，不引入 backtest_bin。每 target 仅调用一次编译入口：

```text
cargo test --locked --lib --no-run --jobs 1 --message-format=json
cargo test --locked --features s_session --bin s_structure_session --no-run --jobs 1 --message-format=json
```

经 telemetry 包裹，但 Cargo JSON 独存于 `*-build.command.stdout`，不与 telemetry 文本混合。只接纳该次 JSON 中当前 crate/源路径/target kind/features 对应的唯一测试 executable：必须在本次新建 target 的 debug/deps 内、fresh=false、列于 filenames、末尾只有一份成功 build-finished。拒绝猜路径、旧 binary、缺/重复 artifact 或错误 features。

随后执行实际 binary 的 `--list`；完整原件必须对以下每个完整名恰好命中一次，整个 list 不得重名，短名也不得跨模块歧义；同时核对当前源码中的 `#[test] fn`。从相同 binary 逐项执行：

```text
<该次 Cargo JSON 的绝对 executable> <完整名称> --exact --nocapture --test-threads=1
```

| target | 完整名称 |
|---|---|
| lib | `theta_v0::parser::inclusion::tb02u_tests::tb02u_independent_oracle_all_fields_and_first_pair` |
| lib | `theta_v0::parser::inclusion::tb02u_tests::tb02u_batch_incremental_and_every_boundary_resume` |
| lib | `theta_v0::parser::inclusion::tb02u_tests::tb02u_revision_retracts_and_reestablishes_without_mutating_prior_facts` |
| lib | `theta_v0::parser::inclusion::tb02u_tests::tb02u_exhaustive_small_raw_domain_keeps_established_geometry` |
| lib | `theta_v0::parser::tb02u_tests::tb02u_parser_pending_activation_sparse_coords_and_resume` |
| lib | `theta_v0::parser::inclusion::tests::tb02a_trace_matches_existing_kernel_and_preserves_actual_roots` |
| lib | `theta_v0::parser::inclusion::tests::tb02a_direction_is_from_actual_accumulator_not_previous_raw` |
| S | `tests::tb02u_old_cut_explicit_replaces_revision_and_recovery` |
| S | `tests::tb02u_pending_append_keeps_first_known_and_policy_gate` |

这些是已核源码的预期名，**尚无真实 list 原件**。远端名称不符就拒绝，不自动换短名过滤。list 前、exact 前后核 SHA256、device/inode、大小、mtime、mode；exact 必须有具名 ok 和 `1 passed; 0 failed; 0 ignored; 0 measured`，零测试 exit0 不算成功。

### Lean、fixture 与持续 CI

具名远端任务依序运行 `cd formal && lake env lean Origin/RawInclusionInitialization.lean` 和 `python3 scripts/check_fixture_drift.py`，同属上述截止。现有常规 CI fixture job 另加同一单文件检查，不只放一次手工 workflow。新增文件的六定理、两实例尚未在本轮编译。

fixture 映射固定为：0 无漂移，1 真实漂移，2 内部错，3 缺 lake，4 构建失败，5 导出失败；未退出和未知码另记。1 不冒充环境错误，2–5 不冒充漂移或通过。

### 原件与失败封包

封包包括根批准/独审文本与摘要、source/prepared/execution commit/tree、全树 Git 对象清单、产品清单摘要、实际 workflow/runner/toolchain 摘要、GitHub run/job 身份、精确命令/features、binary 身份、list、各 exact/Lean/fixture 原始退出与 stdout/stderr、telemetry、监督和回收记录。`FILES.json` 对封包输入逐文件作摘要，`evidence.zip` 和独立 `PACKAGE.json` 记录封包结果。它们不把自己递归纳入自身摘要。

封包失败返回非零，并保留已存在原件、FAILED receipt，不把部分 ZIP 称作成功封包。上传额外保存散件，所以部分封包可供排障。checkout/runner 启动缺件有显式失败记录；外部硬取消可能使原步骤停在 RUNNING 或没有最终封包，这些状态不能推断通过。`UPLOAD.json` 的 REQUESTED 只代表提出上传；真正上传结果及取消/超时以 GitHub step/job conclusion 为准，不能在 ZIP 内预写上传成功。

## False → True 的无自引用绑定方法

当前 `CONTRACT-DRAFT.json` 为 PREPARED_NOT_AUTHORIZED，所有未固定身份为显式 `UNBOUND`，不是空/null 默认放行。旧产品 `c25f35f...` 有独审阻断，本轮没有把它填成可运行身份。

管理者后续在已授权流程中依次完成：

1. 业务三项返修与独审完成后固定产品提交 **S**/tree。将最终 CI/runner/报告收在 False 准备提交 **E**/tree；S 必须是 E 的祖先，二者产品对象相同。
2. 合同通过 dispatch JSON 输入提供，不把 E 的 commit/hash 写回 E。按 `canonical(product_entries(git ls-tree -rz S))` 的字节算 `product_manifest_sha256`；产品清单只排除本票三个新代码文件、`ci.yml` 与本报告目录，其余完整树对象必须与执行尾一致。此处用 Git blob 对象绑定全部产品/规则/input/oracle；SHA256 绑定规范清单。
3. 在 E 固定后，填入 S/E commit/tree、runner SHA256、**False 版 workflow** SHA256、ci.yml SHA256、原 telemetry SHA256、批准/独审原文与原文 SHA256；根核现有 CI 仅五行追加、19 项 toy/静态检查及业务独审覆盖最终字节，再将外部合同 status 改为 AUTHORIZED_ONCE。
4. 只有根准入后，才由管理者制作 E 的单父直接子提交 **F**，变化必须仅 workflow 中 `ISSUE1393_EXECUTE: 'False'` → `'True'` 一行。runner 检查 parent=E、diff 只有该文件、完整文件字节等于 E 的唯一替换；实际 GITHUB_SHA/GITHUB_WORKFLOW_SHA/ref 必须对应 F。F 的真实 commit/tree 在运行时记录，无 hash 自引用。
5. 运输与 dispatch 不属于本工蜂授权。workflow_dispatch 要求 GitHub 默认分支已识别该 workflow；未注册时是运输缺项，不能擅自推 main/建 PR。根登记一次 run ID；runner 拒绝 attempt>1，不自动 rerun/dispatch。手动另发新 run 仍需根另行授权，不能用旧合同自行续跑。

当前缺项：最终业务 source/tree、False 准备提交/tree、最终文件摘要、产品清单摘要、业务与 CI 独审原件、根准入文本、运输/dispatch 位置与 run ID。最终产品变化后需重核相应 source/list/测试结果，不能从本 WIP 或其他票借通过数。

## 本机实际验证

最终检查针对本 WIP 字节。只有 Python 语法编译、AST、YAML/内嵌 shell 语法、纯模型和临时 toy 子进程；没有启动真实产品。

```text
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s .github/scripts -p test_issue1393_validate.py -v
# exit 0，19 tests，2.366s；toy-final.stdout / toy-final.stderr
python3 .chanlun/review-results/issue1393-validation-ci-20261018/verify_static.py
# exit 0；static-final.stdout / static-final.stderr
```

19 项包括 False 主入口完全无 Popen、未知/缺失 source/hash 拒绝、实际绑定字段错配拒绝、唯一门差异、完整产品对象变化、外来/fresh/缺失/重复 Cargo artifact、binary 同字节换 inode、缺 list/零匹配/重复名/歧义短名、exact 零测试/ignored、源码九名、失败后 NOT_RUN、共享截止不重置、TERM 拒绝后 KILL 与回收、收到 TERM 原件、启动失败、清理失败、永久未知进程组拒绝回收成功、telemetry 与命令退出分开、fixture 六种真实 toy 退出、墙钟/monotonic 边界、封包缺件和 ZIP 失败。

静态检查实测 PyYAML 6.0.3；两个 Python 文件 py_compile/AST 成功（pyc 放临时目录后清理）；两个 YAML 可解析；内嵌 Python AST 与 `bash -n` 成功；默认 False 与后续步骤门值明确；`ci.yml` 去掉新增五行后与固定基线逐字相同；已跟踪及新代码空白检查通过。19 是本 runner 的 toy 测试数，不是 Rust/Lean 通过数。

失败发生史完整保留：

- toy round01：16 项中两项错误；macOS 沙盒在退出竞态对进程组探测返回 EPERM，原实现未捕获。修为记录探测错误、继续有界回收、最终未知判 CLEANUP_FAILED；round02 的16项通过，round03 增至18项通过，最终增至19项通过。
- static round01：语法/YAML/追加比较已通过，检查脚本误把 `git diff --no-index --check` 的“文件有差异、无空白诊断”退出1当失败；改为核退出码及空诊断后通过。原输出和错误说明保留。

本工蜂自查不冒充管理者独立评审。原独审指出的三个业务协议缺口属于另一写集，本轮未修也未宣称已经修复。

## 明确 NOT_RUN 与边界

本机 cargo/rustc、Rust 编译与真实测试 binary、lake/Lean 构建、fixture 脚本、公共业务 Python/JS 回归、产品/真实 S 会话/API/浏览器、新进程持久恢复、Long/历史、网络、CI/push/dispatch 全部 **NOT_RUN**。不据本轮关闭 #1393 或上层票，也不替代正式 U12/U13 真实链证据。批准后真实 CI 若失败，按真实源码/环境/资源原因报告，不换判据、不删测试、不改 ignore。
