# Sandcastle 的 Codex 工蜂与交付封包

本入口绑定 #1456。它供当前 Codex 原生子代理管理单次 Sandcastle/Codex CLI 工蜂：主控派任务，管理子代理启动、等待或停止实际工蜂，再将结果回传。管理子代理 ID、父任务 ID 与外部 Codex session ID 分别记录；外部进程没有因此注册为 Codex App 的原生 child。

旧 `main.mts` 和常驻拾取队列仍是各自已有入口。本入口只处理显式指定的 ticket、工作区和版本，使用 Sandcastle 原生 Codex provider，模型为 `.sandcastle/main-model.mts` 的 `gpt-6.1-sol`、推理档 `xhigh`，经本机 CLIProxyLocal（provider `cliproxy-local`，`http://127.0.0.1:8318/v1`）路由到它的 OAuth 订阅认证，与主控同模型；不经 `mf/`、`yd/` 等中转前缀。模型名含 `/` 或 provider 不是 `cliproxy-local` 时，入口在启动前失败。它不经过 PrimeAgent，不改变全局模型、队列或合入设置。

## 单次运行

先为本票准备独立、干净且固定于目标提交的 `codex/*` 分支工作区，以及一份范围明确的提示文件。复核示例：

```bash
node --import tsx .sandcastle/codex-child.mts \
  --ticket 1456 \
  --parent-thread-id '<当前父任务 ID>' \
  --manager-agent-id '<实际管理子代理 ID>' \
  --checkout '/绝对路径/专用工作区' \
  --expected-head '<完整提交 SHA>' \
  --mode review \
  --prompt-file '/绝对路径/任务说明.md' \
  --output-dir '/绝对路径/尚不存在的结果目录'
```

管理子代理必须等到工蜂实际退出并取得结果，才能向父任务报告完成。非零退出、缺少最终结果、取消或版本异常都必须保留对应状态；不能把启动成功、拿到进程号或收到中途文本当作交付完成。主控停止任务时，管理者向运行入口传递停止信号，并确认自己启动的进程结束。

在当前 Codex 协作工具中，新管理工位用 `spawn_agent`；给已结束的工位分派下一项工作用 `followup_task`，并核实工位重新运行。`send_message` 只传递消息，不会启动已结束工位的新轮次；消息投递成功不能作为任务已开工的证据。

本入口的本地执行层使用 Sandcastle 的 `noSandbox`，内层通过 Codex `read-only` 或 `workspace-write` 执行。专用工作区固定任务对象，但这不是 Docker 隔离，也不限制所有宿主文件读取；提示中的任务范围仍须管理者审定。入口收紧 SDK 默认权限并禁用自动合并，不加载项目 `.sandcastle/.env`，拒绝带项目 `.codex/config.toml` 的工作区。命令带 `--ignore-user-config`，不读用户 `config.toml`，所以入口用 `-c` 显式给出 `model_provider="cliproxy-local"` 和该 provider 的完整定义（与用户配置逐项相同）。入口显式以 `-c features.apps=false -c features.plugins=false` 关闭 Apps 与 plugins，因为 `--ignore-user-config` 不继承用户配置里的关闭项（#1764）。CLIProxy 的客户端密钥只经环境变量 `CLIPROXY_LOCAL_KEY` 透传给 Codex 进程：进程环境没有时，在 macOS 上用 `launchctl getenv` 取一次，仍取不到就在启动任何进程前失败，提示只含变量名。预检用的 git 子进程只拿 `PATH`、`HOME`、本地化等白名单变量，不含该密钥（#1588）。上游认证由 CLIProxy 的 OAuth 认证文件承担；工蜂执行的 shell 按 `shell_environment_policy.inherit="core"` 只继承核心变量。保留现有网络代理路由；不能把密钥、认证内容或代理值写入提示、日志或报告，运行记录（`state.json`、`events.jsonl`、`result.json`）只记模型名与 provider 名。

每次输出目录必须尚不存在。入口在该目录下创建 0700 的 `codex-home`，同时作为工蜂环境的 `CODEX_HOME` 和 provider 的 `sessionStorage.hostSessionsDir`，避免宿主 Codex Apps 工具缓存进入上游请求；真实 session 日志也落在这个隔离 home。该目录只在本次运行内使用，入口仍禁止隐式 resume/fork，不改变一次性结果目录合同，也不迁移旧会话。

入口写出 `state.json`、`events.jsonl`、原始输出与 `result.json`，后者绑定前后版本、实际退出状态和外部会话。运行完成与任务审查通过是两件事：即使外部工蜂正常退出，仍须读取结果中的发现并解决阻断项。取消或失败保留工作区与证据，管理者负责后续处置。

## Deep Agents 的自报字段

默认结果 schema 严格保持 `status`、`summary`、`findings`、`validation` 四键。Deep Agents 派工通过 `--output-protocol deepagents-self-report-v1` 选择独立六键 schema，额外要求 `confidence` 为 `high`、`medium` 或 `low`，`caveats` 为字符串数组。没有局限时返回空数组。

Boat 派工按 run 冻结的 `runtime_head` 中 `codex-child.ts` 的协议声明选择 schema；旧运行时的新工蜂仍用四键，提示词也保持四键，控制端升级不会改变这一点。本地 launcher 使用当前控制端的运行时。Deep Agents 把选择固定在派工 intent 中，Boat 封包把它带进 request；新协议的本地与云端回执记录 `output_protocol`。恢复核对回执与 intent 的协议，拒绝删除标记来降级结果。无协议标记的旧 intent 和旧回执仍走四键，不重派已有工蜂。

主控工具投影及设计裁判、返修提示保留模型自报值。旧回执仍投影为 `low` 并说明它来自适配器。自报字段不参与 PASS、签批或发布准入。

## 交付与证据

代码及产品所需文档、测试输入入 Git。运行输出、扫描、评审报告和过程证据放在 `/Volumes/AgentStorage/<任务>/`，不要为报告另造提交。`--output-dir` 也使用外卷下尚不存在的目录。

交付时列出 PR、精确 head、实际验证结果、非作者独审记录及外卷证据路径。复用证据须适用于同一产品内容；变更影响了证据前提，就重跑受影响检查。发现 BLOCK 或缺少材料时明确列出，不能以进程退出成功代替通过。

旧 `delivery-bundle.mts` 的 schema 要求报告在 Git 中，它不支持这条外卷交付路径。本入口不再要求调用它；既有封包仍按原 schema 读取，仍需调用旧封包的任务由 #1742 根协调后续兼容工作。

主控按当前任务授权处理合入；本入口不授予合入权限。提交推远端、无未保存工作或活跃进程后，用 `git worktree remove` 回收本任务工位。失败任务先保全未提交内容与恢复信息。
