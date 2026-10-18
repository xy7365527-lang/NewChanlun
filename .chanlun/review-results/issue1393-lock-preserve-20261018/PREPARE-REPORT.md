# #1393 保旧锁解析入口准备

本轮已形成可审差异，作者定向验证通过；**完整合同冻结仍阻塞于原资源帽数值缺失**。工作流为 `False`，E/tree、批准、独审及资源帽均准确 `UNBOUND`。未运行真实解析，未接受旧广泛生成锁，未批准任何下一步。

名分：绑定 #1393 的工作草稿。工位为外部 Sandcastle Codex 工蜂，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者 `/root/manage_1393_sandcastle`；真实 session_id 由运行时登记，本报告不声称注册为父任务的原生子代理。未启动其他工蜂，作者检查不替代独审。

## 固定基线、写集和合同

起点 clean，HEAD `8542c00cb1b1013e8adc0ffdddc54e4d42f894a4`，tree `036351df2c741a6c6967945e38d4a1419f2eb3aa`，分支 `codex/1393-lock-preserve-prepare-20261018`；结束保留未暂存差异。源码仅三处：

- `.github/scripts/issue1393_lock_prepare.py`：新合同、原输入硬绑定、保旧锁命令、双口径边、registry 原件和失败门。
- `.github/scripts/test_issue1393_lock_prepare.py`：保留原 28 项并适配新合同，增加至 47 项。
- `.github/workflows/issue1393-lock-prepare.yml`：新精确 ref、步骤名和有界上传门；唯一执行门始终 `False`。

另新增本目录报告、静态核验脚本和测试原件；未改旧报告。已读根 CLAUDE/AGENTS、Wayfinder、交付纪律、世代宪法、旧 REPORT/FIX-REPORT、两次独审和测试。原有限独审结果只复用未改的身份门、预算、监督和隔离基础，不冒充本轮已独审。固定 helper 摘要仍为 `a98e7e6f54dacef35476f245a1f938df6cd0a952191fd7ea67938dd7185b1830`。

新合同使用 `schema=issue1393-lock-preserve/1`，`command=operation=UPDATE_WORKSPACE_PRESERVE_LOCK_ONLY`，ref 固定 `refs/heads/codex/1393-lock-preserve-once-20261018`，消息封套固定 `ISSUE1393-LOCK-PRESERVE-CONTRACT-BEGIN/END`。旧 schema、旧 ref、旧 operation、旧封套和旧批准文本均拒绝。`approval_text/review_text` 未来须分别是 JSON 字符串，精确含 `schema`（`issue1393-lock-preserve-approval/1` 或 `issue1393-lock-preserve-review/1`）、`scope_sha256`、`text`；scope 是排除这两组文本及摘要后的完整合同 canonical SHA256。改变 E、树、入口、操作或资源帽而沿用旧原件将拒绝。摘要只作溯源，不授予运行权限。

S 固定 `8e50d40933c2a6a25f882481f3c46aa885ce6407` / tree `0a2e04a4ce22a6b8732ad3b55f57446ee3f066f0`。强制原 manifest SHA256 `f129440150dd1eb148ddb07ca5828e14a13c41122e018920c05efa1bea2c8727` 与原旧锁 `c591858344c1ff7c62335e10c0e2f1f5cf9e7447a8caf81cd92e556bac248b62`。旧广泛生成锁 `ce204e12ec8b8d25b41668fb3c466315a309252d873c6dbb89bdf64ba97f6ab6` 明确不合格。run15 三份原件只读核过摘要，未用新锁作输入，SUPERSEDE 不等于产品接受。

S/E/F 完整 Git 身份、E 同 ref 预发布、F 单父直接继承 E、F 只改唯一 False→True、事件与实际消息一致、精确 job/attempt/仓库身份及干净工作树门继续保留。旧报告目录因历史 S→E 路径需要继续从产品清单排除，同时新增固定 `git ls-tree -rz 8542c00… -- <旧报告目录>` 校验，锁住其完整路径/mode/blob 集合。当前全部 38 份历史原件逐字保全；全树不可变产品清单仍为 8,375 项，SHA256 `e1059a227dc513426f1a201f02ad94b42c5dfad5d2299d96e88e6901c676785a`。

## 唯一未来解析动作

```text
cargo +1.99.0-x86_64-unknown-linux-gnu update --workspace --manifest-path <独占临时目录>/rust/Cargo.toml
```

保留旧锁，不删锁；运输时与调用前都核原 manifest/旧锁字节。调用前缺 lock、lock 被改或 manifest 被改，均零解析调用。命令失败不再运行 generate、第二次 update、全升级或其他 Cargo 命令。安装和版本读取沿原固定工具链及官方来源；不新增 build/check/test/metadata。完整候选、输入、差异和原始命令输出在可用预算内保留；失败不改记成功。

依据是已经归档且重新核 SHA256 的 Cargo commit `5f94df4789f005f9a352888e8355ffc645b7ed0e` 三份原件：`cargo-update.md` 的 `--workspace` 条款，`cargo_update.rs` 的 `update_lockfile`（缺旧锁会调用 generate；workspace 不清掉已有锁偏好），以及 `update-tests.rs` 的 `workspace_only/update_only_members_with_workspace`。原件位于管理者指定 run15 的 `cargo-1.99-reference/`，摘要见其 REFERENCE-MANIFEST，未重新安装参考环境或执行其中测试。`--workspace` 是尝试保留旧解析的指定动作，不是零升级或数学最小证明。

## Registry 与差异证据

选用合同明确允许的保守取证分支：**只读本次独占 sparse cache，缺失就 proof_gap；不补网**。固定路径是 `cargo-home/registry/index/index.crates.io-1949cf8c6b5b557f`；仅按真实变化包及两张图中其约束父推导 `.cache/<crate 路径>`。不扫描整个 registry，不读宿主缓存、配置或凭据，也没有额外 fetch argv。

先检查输出已有字节/文件帽和同一 Budget，再读取普通原件；拒绝内部 symlink、越界、非普通文件、超单文件/累计字节或项数。保存公共 config 原字节，并核官方 dl/api、无 auth-required；保存完整被选 cache 原字节与 SHA256。只接受已声明的 sparse cache v3/index schema 2，未知格式准确失败。精确匹配所需旧/新版本与 lock checksum，保留 JSON 原文及其 SHA256、完整 record（deps/req、features/features2、rust_version、yanked 和可得附加字段），不从旧报告推造。

`lock-changes.json` 同时保留：全部前后 packages、增删、同身份字段变化、版本集合变化；父 identity + 原 dependencies 字符串的完整文本边；解析到唯一 `(name, version, source)` 的完整目标边及增删；文本不变但实际目标变化清单。歧义、悬空、未知 source、重复身份/重复边均失败。变化范围包含增删身份、同身份内容变化及精确边变化的两端，并按包名列完整旧/新版本集合。

`registry.json` 对变化身份及其两图直接父保存精确记录；每条父边链接真实 registry 依赖声明，根边链接原 manifest 声明。rename、target、kind、optional、default_features 和 features 原文保留。记录约束不等于完成 semver/feature 求解，`version_satisfaction=PENDING_REVIEW`。缺 cache、缺版本、checksum 不符、缺父声明、来源异常均产生 `proof_gap` 并阻断接受；已写原件保留。预算、解析、写入、摘要异常或摘要阶段越期，同样使取证失败；最后摘要写后才过期也会撤销完整标记，写 `registry-failure.json` 和最终 FAILED。

必要项明确只有 `tempfile="3"` 的包存在性及根边义务。具体 tempfile 版本、传递版本必要性、可避免性均待真实结果与约束独审；不把旧矩阵的分类或本次生成成功写成已求解。

## 资源边界与实际缺项

保留同一个 900 秒 wall/monotonic 截止，含 registry、差异、封包和最后出口；受监督命令保留最后 90 秒，jobs=1、10 秒 telemetry、2 GiB 仅启动磁盘门、原进程组 TERM→KILL/wait/subreaper 逻辑均复用。2 GiB 不代表内存、全程磁盘或 artifact 硬帽。最终 ZIP 按不超过 1 MiB 分块摘要；原收据/print/return 信号及截止负控继续通过。上传只有原两次 artifact 步骤，timeout 从同一剩余窗向下取整，帽/截止不满足则跳过。

**未能从给定基线确认的五项数值**：`single_bytes`、`total_bytes`、`files`、`items`、`upload_bytes`。已检查固定锁入口、固定 helper、telemetry、workflow、旧 REPORT/FIX-REPORT/合同及独审；可找到 32 KiB 消息上限、1 MiB 事件上限、1 MiB 摘要块、900 秒和 2 GiB 启动门，未找到所称原输出/数量/上传帽。不能把这些不同含义的数值替代取证帽，也不能声称沿用了不存在于所查基线的数值。

因此源码 `RESOURCE_CAPS` 与草案都为 `UNBOUND`，合同准入、取证和上传门均拒绝。已向管理者请求精确值或指针，交付时仍未收到。测试临时注入 4 MiB 单文件、32 MiB 累计/上传、512 文件、100,000 项的 **toy 参数**，它们不进入生产常量/合同，不是建议或批准的运行预算。只补 E/批准文本还不足以运行，必须先确认既有帽并补冻结与复核。本轮未宣称已完成持续子进程输出的硬字节帽验证；固定旧监督器没有可核的对应数值，真实执行继续阻断。

## 实际验证、失败史和未运行项

- 最终标准库定向 unittest：47 项通过，1.215 秒，退出 0。原件 `toy-verified.stdout/stderr`。仅两个既有监督器测试启动临时 Python toy；其余 Popen 为 mock，真实 Cargo/网络解析调用为零。
- 7 组静态检查通过，退出 0：写集/基线、AST 复用、38 份原件保全、YAML/内嵌 Python AST/bash -n、6 种上传 toy、固定输入/完整 Git 清单、草案摘要与拒绝。YAML 使用环境中既有 PyYAML；未安装依赖。原件 `static-verified.stdout/stderr`。
- 新覆盖包括新 schema/ref/operation、旧批准与旧封套、真实 ce204 摘要拒绝、旧锁存在及调用前篡改、失败无 fallback、两图精确目标及隐式目标变化、删除包旧版本、rename/null alias、缺 cache/缺版本/父约束、重复/畸形原件、checksum/未知 source、单文件/累计/数量/项数/上传帽、读/解析/写/摘要异常与截止、summary 写后越期。沿原 11 个尾部边界 × monotonic/wall/SIGTERM 共 33 个负控。
- 首轮 28 项出现 3 个 failure、2 个 error：新增读取器将 macOS `/var` 的根外别名误判为逃逸，以及旧 tail toy 匹配旧状态名。修正根内相对路径校验和状态名匹配后，第二轮 43 项、第三轮 46 项及最终 47 项通过。全部轮次 stdout/stderr 保留，不覆盖失败史。

全部 **NOT_RUN**：本机/远端 cargo、rustc、rustup、真实安装/解析/registry 获取、网络/API/浏览器、Rust/业务/Lean/fixture、真实 GitHub checkout/run/upload、Linux subreaper 实机、新独立评审、E 发布、F 激活/push、主验证、生成锁接受、第二次 Cargo、main 合入。未暂存、commit、push、merge、关票、评论、dispatch、切分支、改 Git 配置，未读取 .env、认证文件、钥匙串或任何密钥。

交还管理者：先解决资源帽事实缺口并复核候选，再安排新独审/冻结 E 与新批准原件；本报告及草案均不授权这些外部动作。完整测试索引、摘要和 NOT_RUN 见 `PREPARE-VALIDATION.json`。
