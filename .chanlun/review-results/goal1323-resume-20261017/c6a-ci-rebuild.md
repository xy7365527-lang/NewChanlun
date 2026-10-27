# #1406 C6a 远端装配重建首版：BLOCKED_PREPARATION

本次只交可审准备件，**不是完整 runner，不具备 dispatch 资格**。`CONTRACT_READY=False`；实际 `--preflight` 返回 2，打印缺口，不构建、不启动产品。

## 基线与写入范围

- 工作树 `/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1406-c6a`。
- HEAD `b6e690ecae5723fbbcb773f15400c71d1010da87`。
- tree `601a3f4bd54c22358eddfbf6ad51cd268fa9de5b`。
- 仅三个新文件，未提交：`.github/scripts/run-c6a-remote.py`、`.github/scripts/test-run-c6a-remote.py`、`.github/workflows/ci-c6a.yml`。
- `git diff --exit-code` 返回 0；`git status --short --untracked-files=all` 仅上述三文件，原产品、driver、fixture 和旧 CI 无修改。

已读本树 `CLAUDE.md`、Wayfinder 工作流相关段、主控给定的 `economic-next-step.md` 与 #1406 最新评论；已调用 Serena initial_instructions。知识图未索引当前工作树，返回 project not found，继而按明确文件路径读取。没有进一步派代理。

## 已实现及实际验证

准备模块提供独立函数，暂未串成真实运行器：

1. 原清单哈希、文件长度/摘要、重复条目、路径穿越及符号链接拒绝。
2. 调用现有 driver 构建合同，绑定实际源文件、实际 E/B binary hash 及冻结输入；不复制金额 reducer。可信 git 精确身份/受控构建退出的取得尚未装配，函数调用者负此责。
3. owner 每次原件与具名请求的原字节、socket 角色、完整响应 envelope、因果身份及 payload hash 绑定；不按全 case 集合凑数。
4. H3 Capture 绑定：从原请求 raw/pointer 独立计算事实；Recorded fact key/hash 及 cut 对 after owner；after 原件各字段对请求；首次 Capture 要求 before 无该事实且单次提交。
5. 进程日志按 spawn 实例闭合；CLI 永须 exit0；服务 -15 必须有对应 TERM 原件；未知、重收割和未回收拒绝。此函数不是进程 guardian，只消费日志。
6. 同 t0 的 1200/1830/1920/2040/2070 绝对端点判定；晚到成功拒绝；取消只保留有界清理/封包。内层600/630/300/10/30及TERM3/KILL3原值保留。纯函数不能替代硬期限执行监督。

实际执行命令及结果：

```
python3 -B .github/scripts/test-run-c6a-remote.py
Ran 12 tests in 0.007s / OK / exit 0

python3 -B -m unittest discover -s s_session/tests -p test_tb03c_query_runtime_contract.py -v
Ran 48 tests in 1.639s / OK / exit 0

python3 -B .github/scripts/run-c6a-remote.py --preflight
BLOCKED_PREPARATION / CONTRACT_READY=false / real_execution=false / exit 2
```

新增两项 H3 负例并非只改摘要字符串：一项实际修改 raw、重新计算 raw_sha/fact_hash/image_hash/root，再绑回执 cut，仍因请求到 after 原件断裂拒绝；另一项伪造 Recorded cut 并重新生成合法完整响应 envelope，现有 validate_reply 通过而新 cut 绑定拒绝。新增具名请求/角色、CLI -15、原件边界及期限负控均实际运行。全部均为离线受控测试，**没有真实 HTTP、E/B/Q 或真实进程回收证据**。48 项只证明原 driver 替身合同继续通过。

Python AST 解析通过，三文件无行尾空白；未安装额外依赖。独立 Python reviewer 尚待主控派发（本任务禁止再委派）。

## 精确阻塞与尚未完成工作

### 缺冻结事实，不能自行替换

1. 旧 CI v3 审包未在给定候选树找到；历史 `/private/tmp/goal1323-native-20261012/recovery-20261015` 当前不存在。最新票面只列时间端点，未给出原 C6a 的 RSS/磁盘/输出容量/监测周期完整数值和规则。不能用 C7a 资源配置臆代。
2. 同一 t0 的 guardian、取消、保有组身份直到回收及失败封包完整已审实现/合同没有恢复。时间常量本身不足以证明监督合格。
3. 旧 CI 审包六反例及原 H3 两反例原件不可用。新反例可验证本准备件，但不能宣称原独审反例逐项关闭。

### 实施欠项，不能混称外部阻塞

- 全量逐序 transport 回放，把 WE5 20步/WE7 21步、前后 owner、公共 current/fixed/watch/as-known、oracle 输出与每次完整 wire 绑定；现有 driver oracle 可复用，无需第三套 reducer。
- Q-STARTUP 原件及每次 zero-send ECONNREFUSED、首末序号、原 deadline/成功回包逐值审查。
- 实际 git/构建身份获取、冻结 source tail allowlist 检验、编译命令/退出/耗时封存、运行前后 binary/source 验证。
- 完整运行监督、资源门、取消硬停止、未收割 leader 的组身份与后代回收、失败封包以及 always artifact 上传。
- 完整正例、旧具名反例、独立审查，然后才能改变准入状态。不能只翻 `CONTRACT_READY` 即运行。

现 workflow 仅手动准备入口、指定候选分支、35分钟 job 上限和两组离线测试，最后必定失败退出；**没有真实 runtime 步骤，也没有上传/完整封包能力，不可冒称生产验收工作流**。未 push、未 dispatch、未关闭任何票。没有 cargo、外盘、SQLite、HTTP 服务或浏览器操作。

## 文件摘要

| 文件 | 字节 | SHA256 |
|---|---:|---|
| `.github/scripts/run-c6a-remote.py` | 10730 | `be4bde0d3b0cb07a9ba19bb19087335b897b124b437959619de4e863cacdea71` |
| `.github/scripts/test-run-c6a-remote.py` | 9787 | `9c933bd2a7302e014036bb9315e5c30fe80edadf0e7c2ef0f4575bf616b24c74` |
| `.github/workflows/ci-c6a.yml` | 1102 | `120b1b05dc62d4cf888537c20138620e02bef28ee0d07840fa699ddbf6b00e57` |
