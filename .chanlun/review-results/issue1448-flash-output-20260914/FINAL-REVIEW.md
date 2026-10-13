# #1448 direct Codex Flash 路由最终独立审查

名分：#1448 工作草稿。审查日期：2026-09-14。审查者：`/root/flash_route_final_review`。

结论：`NO_BLOCKING_FINDINGS`。指定差异未发现需阻断交付的代码或安全问题。执行与辅助复核工蜂固定为 `deepseek/deepseek-v4.1-flash / max`，配置、权限及结果校验失败均保留失败状态。本结论仅覆盖以下版本和产品字节，不授予 main 合入、全局排程或业务票关票权限。

## 固定版本与覆盖

- checkout：`/Users/silencehan/Projects/NewChanlun-1448-flash`
- base：`6f99b36c64e0560febdb08e5d1f8e781d2af9299`，本次读取时的 `origin/main`。
- head：`434d7ee4e571ee0ac3a8f3324837852c671a2304`。
- 产品范围：下表三文件的全部差异及其相关调用路径；同时核对 `.sandcastle/CODEX-CHILD.md` 和具名验证记录。审查开始时工作树干净，报告写入前再次核实三文件及说明文件与固定 HEAD 无差异。

| 产品文件 | SHA-256 | Git blob | 模式 |
|---|---|---|---|
| `.sandcastle/codex-child.ts` | `2feba6333802eeeee6f7caa478f5cd887c0c72f0f4f30758d675bd21b3028a03` | `dce240284d9a1269d4177e6b1129838b5e2cf97e` | Git `100644`，文件系统 `0644` |
| `.sandcastle/codex-child.test.ts` | `0cc296b8c8746457c310e69f95896d94c032a3a88e044e29ab87cb4c435f5e8c` | `78d5d8a1507c8f6a8361dcf2438e80d9a5090ce9` | Git `100644`，文件系统 `0644` |
| `.sandcastle/codex-router-runtime.ts` | `190922c7d9688ce4ce4b0e25bb589f22d295e070fa9299d127dac2193a6c5a95` | `3a6b1d1ce4145420e25b02e34876c3516ffaf709` | Git `100644`，文件系统 `0644` |

## 安全与行为判断

审查采用当前入口已有的信任边界：管理者选择任务、独立 checkout 和本机工具链，用户配置及本机 Router 属于可信运行环境；模型输出和运行退出状态须独立校验。`.sandcastle/SECURITY.md` 描述的 Prime/Docker 凭据入口不应用来推定本入口加载 `.env`。direct Codex 入口的实际调用没有加载该文件或启动 Prime 队列。

1. 配置解析限定于本机值。`codex-router-runtime.ts:10-24` 用固定 Python 程序和参数数组执行 `tomllib`，`-I` 隔离 Python 导入路径，只把两个配置值传回调用方。解析错误转为固定文案，不传播含原始输入的 stderr。`29-31` 精确限制为 `http://127.0.0.1:<合法端口>/v1`，拒绝远端、用户信息、查询参数与 caller capability 路径，不会通过删去认证路径降级放行。
2. 模型目录校验拒绝错误注册。`codex-router-runtime.ts:33-44` 要求绝对普通文件路径，拒绝控制字符、末端符号链接及超过 16 MiB 的文件；目录内必须恰有一个完全匹配的 Flash slug，且登记 `max`。缺配置、坏 TOML、坏 JSON、重复模型或缺少 `max` 均抛错。该校验不等于对 Router 上游实际推理行为的证明。
3. 参数没有新增 shell 注入入口。`codex-child.ts:99-100,197-225` 对所有动态 `-c` 值执行 shell 单引号转义，配置字符串经 JSON 字符串编码，提示通过 stdin 传递。SDK 命令形状或默认 effort 变化即拒绝执行。最终命令只含一次 `model_reasoning_effort="max"`，保持 `-a never`、`--ignore-user-config` 和分别对应 review/execute 的 sandbox。
4. 环境与目录边界没有放宽。`codex-child.ts:173-184` 的白名单不透传 API/GitHub 密钥、`OPENAI_BASE_URL`、`BASH_ENV` 或会话续跑环境。已有代理环境继续沿用，入口不将其值写入运行记录。`153-170,468-472,527-539` 保留 realpath 根目录与管理者目录排斥、干净 `codex/*` 分支、固定 HEAD、项目 Codex 配置拒绝、独占输出目录以及紧邻启动的二次预检。
5. 输出提示兼容两种模式。`codex-child.ts:437-453` 将同一 `OUTPUT_SCHEMA` 直接展示给模型，明确 findings/validation 为字符串数组。只读任务仍在返回值中给出必要发现；只有已授权产物的任务才写文件，因此没有强迫 review 模式绕过只读限制。
6. 格式失败没有转成成功。`codex-child.ts:266-280` 仍拒绝非法 JSON、额外字段、错误数组元素和缺项；`546-568` 继续要求实际进程成功退出、正确会话记录、HEAD/分支未漂移及任务状态 completed。未知或错误输出不能触发换模型、降低校验或自动重试。完成 JSON 也不代表其 findings 已经由主控验收。

本次没有确认的新增可利用安全发现，blocking 数量为 0。已有 `noSandbox` 加 Codex sandbox 并非容器级宿主读取隔离，禁止读取凭据仍有任务指令层约束；说明文件已披露这一边界。没有把本次路由修改认定为修复该既有边界。

## 验证证据与版本关系

本次独立执行了固定版本、Git 差异、产品内容摘要及权限核对，`git diff --check` 返回 0。只读的 security-diff-scan 配置能力预检返回 `ready`、exit 0，没有修改配置。代码图工具对该 checkout 返回未索引，因此使用指定文件和固定 Git 差异完成审查。按本任务仅新增本报告的范围，交付为人工逐文件审查，不声明生成过插件完整扫描或 SARIF。

同目录 [CHECKS.json](CHECKS.json)、[REGRESSION.log](REGRESSION.log) 显示 41 项回归、0 失败，绑定的 `codex-child.ts` 和测试文件摘要与本次 HEAD 完全一致。[FINAL-PROMPT-TEST.log](FINAL-PROMPT-TEST.log) 另记录 1 项最终提示真实子进程测试通过。该测试使用伪造 Codex 可执行程序验证 argv、stdin 和监督行为，不是一次额外真模型调用。`41 + 1` 存在覆盖重叠，不声称为 42 项互异用例。

[RUNTIME-RESULT.json](RUNTIME-RESULT.json) 记录最新短只读探针通过，session 为 `01a0a006-9e14-7a80-ba16-bedd090e084d`，模型/effort 为 Flash/max，退出码 0，前后 fixture HEAD 均为 `3a3ed2b83267bee277e6d6ded6120254a4a0efd9` 且工作树干净。这个 HEAD 属于受测 fixture，入口源文件绑定见 CHECKS.json；不能把 fixture HEAD 当成产品 HEAD。

较早的 67 项回归和真实读写探针已直接核对以下原始归档，未重跑：

`/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1374-tb03a-implementation-20260913/evidence/issue1448-deepseek-flash/max/`

主控随后将三份原件归档为 [MAX-VERIFIED.json](max-baseline/MAX-VERIFIED.json)、[TESTS.log](max-baseline/TESTS.log)、[EXECUTE-RESULT.json](max-baseline/EXECUTE-RESULT.json)，来源和摘要见 [ARCHIVE.json](max-baseline/ARCHIVE.json)。本审查逐份执行 `cmp`，三份仓内实体均与上述具名原件逐字相同。新增归档不改变本报告固定的产品版本。

- `MAX-VERIFIED.json` 和 `tests-final.log` 绑定入口版本 `91fd5c7511a7e97f4d0a9b55da24a042ac175b6b`，记录 67 通过、0 失败。其 runtime.ts 摘要与当前 HEAD 相同；child.ts 摘要为 `ff0ddfd5d62e087a60e482637c54f9dfa7e8617dfe4166b3ef173337ce260f84`，test.ts 为 `98bca7b30d0e96ce6819d8b5288f418627a293588e8c5c5d1c1398c4186853da`，二者均早于输出提示补强。该 67 项结果不重标为当前 HEAD 全套已重跑。
- `execute-run/result.json` 的 SHA-256 独立复算为 `fa81ec7de43019dbf291fa56776411e324e7d126ca93412d9cc9ac88adef6748`，与 MAX-VERIFIED.json 一致。它记录 session `01a09fbb-f0a2-77b0-9523-f9fd61780ae6`、Flash/max、workspace-write、退出码 0，并只留下 `flash-max-output.txt`，fixture HEAD 前后均为 `4419130b4ac8d52c934d8aa7dcd2a42f00173792`。

## 保留的限制

提示补强提高格式遵从机会，不能证明 Router 或上游强制执行 JSON Schema。现有报告保留两次历史格式失败；长任务仍可能产生额外字段或非法 JSON，封装器应继续返回 failed。现有通过探针只覆盖各自声明的短任务。

本次未重跑真模型、服务、Prime 或全局队列，未读取认证/session 内容，未修改产品、用户配置或审批闸。依赖清单不在本次差异内，本报告也不提供全仓依赖漏洞扫描结论。除新增本报告外没有写入仓库，不做 commit、push 或 merge。
