# #1393 独立远端仅锁解析准备

本轮交付可审准备字节，workflow 的唯一开关保持 **`ISSUE1393_LOCK_EXECUTE: 'False'`**。没有执行真实远端解析，没有生成真实新 lock，也没有获得具体运行授权。`CONTRACT-DRAFT.json` 的最终 S/E、全部摘要、批准和独审原件均为 `UNBOUND`，状态为 `PREPARED_NOT_AUTHORIZED`；当前草案会准确拒绝。

名分：绑定 #1393 的工作草稿。外部 Sandcastle Codex 工蜂，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者 `/root/manage_1393_sandcastle`；真实 session_id 由运行时事件登记，本报告不声称原生子代理注册。本轮未启动任何其它工蜂，也未访问 run08 工位。

固定基线与交付时 HEAD：`8e50d40933c2a6a25f882481f3c46aa885ce6407`；tree：`0a2e04a4ce22a6b8732ad3b55f57446ee3f066f0`；分支：`codex/1393-lock-prepare-20261018`。开始时工作区干净；结束仍无 tracked 差异及暂存，全部新文件留给管理者收包。

## 实际变更

仅新建下列入口及本报告目录内的草稿、验证原件：

| 文件 | 用途 |
|---|---|
| `.github/workflows/issue1393-lock-prepare.yml` | 精确分支 push、默认关闭、单 job 15 分钟、稀疏 checkout、失败原件和实际上传收据 |
| `.github/scripts/issue1393_lock_prepare.py` | 固定 helper 复用、完整身份核验、临时解析、固定工具链、全包/依赖边披露 |
| `.github/scripts/test_issue1393_lock_prepare.py` | 23 项标准库 toy；不调用 Rust、网络或产品 |
| 本目录 | REPORT、VALIDATION、完整合同草案、工具链证据副本、静态源清单和 toy/static 原件 |

枚举方法为 `git diff --name-only <固定基线>`、`git diff --cached --name-only`、`git ls-files --others --exclude-standard`，完整检查脚本是 `verify_static.py`。既有验证 runner/workflow、`ci.yml`、telemetry、业务 Rust、Lean、manifest、lock 及配置均未修改。没有认领、评论、commit、push、merge、dispatch、切分支、Git 配置修改或关票。

## 编码前参考与工程复用

已读 CLAUDE/AGENTS、Wayfinder、delivery/generation，以及本票 `issue1393-validation-ci-20261018/TRANSPORT-REPORT.md`、`REUSE.md` 和已保存 XERJ reference-evidence。当前工具集没有 codebase-memory 图工具，故按指定路径定向读取，不重新联网或建索引。

四份 XERJ JSON 均已完整解析，摘要见 `REFERENCE-READ.json`。Nautilus 状态寿命跨切片的经验继续用于一个共享截止覆盖所有步骤；没有借用 #1419/Nautilus 业务判据。只读复用管理目录 `reference-1488-transport/` 的精确 push、GitHub/Linux 身份、干净源、受限差异与失败保件机制；#1488 的 driver、预算、旧运行许可没有转授。

固定复用 `.github/scripts/issue1393_validate.py`，SHA256 为 `a98e7e6f54dacef35476f245a1f938df6cd0a952191fd7ea67938dd7185b1830`。新入口先验证该固定路径的摘要，再加载同一份已验字节到私有模块实例。合同不能指定模块。仅适配该实例的固定常量、job 身份与写集，复用 strict JSON/message、`bind_source`、`Budget`、`Supervisor`、`telemetry_command`、`package`；不调用旧 `main` 或 `execute_checks`，不修改旧文件。

旧 identity 的产品排除范围在私有实例中收窄为本次三个入口及本目录；`ci.yml` 和旧验证入口也必须与 S 完全一致。完整 Git 元数据用于 S/E/F 核验，不能用 sparse checkout 的可见文件数冒充完整源。

## 准备合同与未来运输

完整、同形合同见 `CONTRACT-DRAFT.json`。未来 F message 必须恰为以下一个封套，最多 32768 UTF-8 字节，禁止标题/尾随正文/多封套：

```text
ISSUE1393-LOCK-CONTRACT-BEGIN
<完整绑定 JSON 对象>
ISSUE1393-LOCK-CONTRACT-END
```

JSON 拒绝缺字段、未知字段、重复键、错误类型、非有限数值、占位/错误 hash 和 `UNBOUND`。S/E commit/tree 均是完整 40 位小写 Git 身份，摘要是完整 64 位 SHA256；批准与独审各最多 8192 字节，按原始 UTF-8 计算摘要。消息只用于数据与溯源，永不作为 command/path/argv。唯一传入 Git 的合同数据是已核身份的 `S E\n`，进入固定 `diff-tree --stdin`，不拼 argv。

绑定面包括：完整 S/E commit/tree；完整产品 Git 对象清单摘要；新 runner、False workflow、固定 helper、既有 `ci.yml`、既有验证 workflow、原 telemetry、真实 manifest、旧 lock、工具链证据摘要；批准和独审原文及摘要。运行时另核 repo、精确 ref、event、job、attempt=1、HEAD/after/head_commit.id/GITHUB_SHA/WORKFLOW_SHA、实际 Git message 与事件 message，以及 E tree、S 在 E 祖先中、S→E 全部变化只在本次准备写集。

未来 F 必须是 E 的单父直接子提交，push 的 `before` 必须恰为同分支预发布的 E；拒绝新分支全零 before、created/deleted/forced、attempt>1。E→F raw diff 必须只有本 workflow 的 `100644→100644` 内容变化，完整文件严格等于唯一一行 `False→True` 替换。执行身份 F/tree 由实际 Git/事件绑定，避免 message 自引用。

**摘要与 `BOUND_FOR_ROOT_REVIEW` 仅表示可审数据，不能授予权限。** 本轮根实际只授权准备字节。管理者固定和独审后，根还须分别核准：

1. 最终 S/tree、准备 E/tree、完整源差异和摘要，在 tree 外填写完整合同及真实批准/独审溯源。
2. 将 E 以 False 预发布到 **`codex/1393-lock-once-20261018`**。本分支与正式验证分支 `codex/1393-validate-once-20261018` 严格区分；False 在 checkout 和任何外部命令前通过 shell 内置命令拒绝。
3. 独审未来 F 的单父、唯一门变化、message 和完整合同；仅根对该 F 的单次 push 另行授权后才可执行。失败不自动 rerun；attempt 限制不是跨多个独立 push 的全球一次性令牌，仍需根控制并登记唯一实际 run。
4. 根下载完整 artifact，独审全部 lock 变化，决定是否接受。此后另冻结正式验证 E；本次生成成功绝不替代正式 `--locked` 测试。

## 解析与来源边界

静态复核真实 `rust/Cargo.toml`：`dev-dependencies.tempfile == "3"`；旧 lock 不含 tempfile 包，也没有 `newchan_rust→tempfile` 边。不删已批准依赖，不改 manifest 钉其它版本，不猜 lock/checksum。

固定工具链为 `1.99.0-x86_64-unknown-linux-gnu`。证据来自管理者指定的 main #3315 实际 run `39091453686`，副本 `RUST-TOOLCHAIN-39091453686.txt` 保留原字节：rustc `1.99.0 (b940084d7 2026-09-28)`，完整 commit `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`；cargo `1.99.0 (5f94df478 2026-08-27)`。

受控 runner 使用独占空 `RUSTUP_HOME`，只从 runner 查得的 rustup 可执行字节复制代理，记录其路径与 SHA256，不导入宿主 toolchain/registry 缓存或用户配置。固定版本 install、`rustc +<固定版本> -vV`、`cargo +<固定版本> -V` 全部受同一监督器约束；安装后保存 rustup channel manifest 原件与 cargo/rustc 可执行摘要，逐字段核上述版本。缺 rustup、缺原件、版本不符均停止，不换 stable。

唯一解析 argv 是：

```text
cargo +1.99.0-x86_64-unknown-linux-gnu generate-lockfile --manifest-path <RUNNER_TEMP>/issue1393-lock-work/rust/Cargo.toml
```

不执行 build/test/metadata/Lean/fixture/产品或 build.rs。临时目录只运输真实 manifest、旧 lock、Cargo 声明及自动发现的真实目标入口源码（含存在时的 build.rs 字节，绝不执行）。每项核完整 Git blob、普通文件、无内部 symlink/越界，并记录路径、blob、SHA256、长度；不伪造 lib/manifest，不复制模块树、fixture、数据、凭据或历史目录。静态枚举得到 **85 个文件、2,734,136 字节**，见 `SOURCE-PREFLIGHT.json`；这仅是静态运输清单，不是 Cargo 成功证据。

解析 cwd 位于 checkout 外，CARGO_HOME/RUSTUP_HOME/HOME/XDG/TMP 均独占临时。环境由固定 allowlist 构造，不继承 GitHub token、代理、私有 registry 认证、RUSTFLAGS、Cargo 覆盖或 telemetry 测试采样。祖先目录出现 Cargo 配置即拒绝，不读取它。只允许 manifest 的默认 crates.io 依赖；遇到 workspace、patch/replace、git/path/其它 registry 依赖拒绝，不扩大运输范围。

固定网络来源为 `https://static.rust-lang.org`、官方 rustup update root、`sparse+https://index.crates.io/` 和官方 crates.io 下载配置。留存安装/解析 stdout/stderr、工具链来源及 channel manifest、registry index 每文件摘要与 config 原文；发现非官方 registry config/source/checksum 异常即失败。这里记录的是固定来源、工具原件和缓存摘要，**不是逐网络请求抓包**。

## 预算、失败和产物

一个 job，**总上限 900 秒**，首个 True 步骤即记录墙钟与 monotonic 起点，包含 checkout、工具链安装/核验、解析、回收、封包与上传。原 `Budget` 改用私有实例的固定 900 秒，所有受监督命令预留最后 90 秒做保件/封包/上传，后续步骤不续租；workflow 另有整体 15 分钟终止上限。

保持 jobs=1、原 telemetry 的 2 GiB 起始空闲磁盘门与 10 秒采样。**2 GiB 不是全程硬帽，也不是内存硬帽**。沿用独立进程组、TERM→KILL、wait、Linux subreaper 原件；不能据 macOS toy 宣称 Linux 已实测，也不宣称能约束主动 setsid 逃逸的任意程序。

解析失败后不重跑，仍保留旧 lock、manifest、产生过的完整候选 lock、完整 diff、所有前后 package（name/version/source/checksum/其它字段）、新增/删除/同身份修改/版本或来源变化、全部前后依赖边及增删。`generate-lockfile` 可能重解析已有版本，不能把结果描述为“只新增 tempfile”。坏候选原件照留，解析差异失败也独立尝试保存 registry 原件；结论为失败。

完整运行产物包含合同、push 与 Git 身份原件、产品清单、精确源运输清单、toolchain 原件、命令 argv/退出/stdout/stderr、telemetry、子孙回收、`steps.json`、`result.json`、完整 lock/旧 lock/manifest/patch/包与边清单、`FILES.json`、`evidence.zip`、`PACKAGE.json`。没启动的步骤标 `NOT_RUN`；执行中的步骤失败不会改记为未运行。生成候选仅是 `GENERATED_PENDING_ROOT_REVIEW`，正式验证始终 `NOT_RUN`。

失败也尝试上传，上传 timeout 由共享剩余时间向下取整，不另租窗口。首包不能包含自身上传之后才知道的结果，因此单独生成 `UPLOAD-ACTUAL.json`，记录首包真实 outcome/conclusion、artifact id/digest/URL，打印到 Actions 日志和摘要，并在剩余预算允许时上传第二个收据 artifact。首包未成功会使步骤失败。截止耗尽、强制杀 job 或网络故障可能导致 artifact 不可取得；不能承诺失败必然上传成功，最终以 Actions 实际结论为准。

## 本机验证与未执行项

最终 `python3 -B -m unittest discover -s .github/scripts -p test_issue1393_lock_prepare.py -v`：**23 项通过，0.249 秒**。覆盖 False/坏合同零外部命令、所有字段缺失/错误类型/UNBOUND、严格封套、摘要、身份与 before/attempt、S/E/F 源/父/树/写集篡改、固定 helper、argv 注入、源 blob/链接、固定工具链、隔离环境、共享预算、失败停后续、临时 Python 超时/回收、完整 lock 包/边变化、失败保件，以及 mock 生成成功/失败均恰执行一次解析且 checkout 不变。

`verify_static.py`：7 组检查通过，含 Python AST、YAML、所有内嵌 Python AST、`bash -n`、精确 push/False/15 分钟/只读 checkout、上传预算和真实成功/失败/跳过收据 toy、固定复用及 argv、完整写集、基线与空白。收据 toy 没有真实上传。

首轮 21 项中 20 项通过，1 项因 macOS `/var` 祖先符号链接被过宽拒绝而失败；已把拒绝范围收窄为指定源根内部的链接及路径逃逸。原件 `toy-round01.*` 保留；修复后 21 项通过，再增加两项整链/坏 lock 保件 toy 成为 23 项。最终原件为 `toy-verified.*`、`static-verified.*`；此前原件均未覆盖。

**全部 NOT_RUN**：本机 cargo/rustc/rustup、真实工具链安装、真实 generate-lockfile、网络/registry 获取、真实 GitHub checkout/push/run/upload、Linux runner/subreaper 实测、正式 Rust 编译/测试、Long/产品/Lean/fixture、独立评审、E 预发布与 F 单次 push。未读取 `.env`、认证文件、钥匙串、任何凭据或密钥。

当前缺最终 S/E、完整摘要/批准/独审原件，以及根对 E 预发布和 F 单次 push 的分别许可。这些是**未来执行条件**，不影响本轮默认关闭准备字节的交付完成；本报告不是独审或运行批准。
