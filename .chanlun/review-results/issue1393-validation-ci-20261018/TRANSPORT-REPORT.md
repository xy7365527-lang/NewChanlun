# #1393 专属 push 运输修正：False，等待管理者独审

本轮仅完成准备：入口改为精确分支 `codex/1393-validate-once-20261018` 的 push，实际合同改由未来 F 的 commit message 运送。当前 workflow 仍为 **False**；合同草案为 `PREPARED_NOT_AUTHORIZED`，全部待固定身份为 `UNBOUND`。没有真实 CI、产品验证或运输动作，也没有取得一次真实 run 的准入。

固定起止 HEAD 均为 `41e8a326743eabb10c1b07df01ce956235c5ad3d`，本工位分支仍为 `codex/1393-validation-ci-20261018`。外部 Sandcastle Codex 工蜂，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者 `/root/manage_1393_sandcastle`；真实 session 身份由运行时登记，本报告不冒认原生子代理注册。本目录是 #1393 工作草稿。

本报告取代上一轮 `REPORT.md` 的 dispatch 运输方案，上一轮报告、REUSE、19 项 toy 原件及失败发生史逐字保留。

## 指定参考及复用边界

编码前读取 CLAUDE、AGENTS、Wayfinder、世代宪法现役线名单，以及本工位 REPORT、REUSE、CONTRACT-DRAFT。当前工具集没有 codebase-memory 图工具，故定向读取指定文件；没有装索引、联网或读取其它冻结工位。

只读参考目录：`/Volumes/AgentStorage/issue1393-sandcastle-20261018-management/reference-1488-transport/`。

| 原件 | SHA256 | 复用内容 |
|---|---|---|
| `run-1488-driver-ci.py` | `05360b7b0777165e82c2bdf0b8b338e6ee7c3e00f93ac933d07c5001eee440bf` | `gate` 的 GitHub/Linux、精确 ref、push 判据；`identity` 的执行 SHA、干净 checkout、源身份与受限差异判据 |
| `issue-1488-driver-six.yml` | `9c5cf48e969d06fecfbe0dcfd76f149bf09a2a0598dab4685dec90ca055c0790` | 单一具名分支 push；contents:read；并发组不取消在途；完整 checkout 且不保凭据；失败仍保原件 |

#1488 的六 driver、features、45/40 分钟、资源帽、旧核验器动态加载和真实 run 授权均未转授。#1393 沿用自己的 runner，保留全部既有验证主体；本轮新增其需要的 message 封套、E 预发布、attempt=1 与唯一门变化判据。

编码前复读本 checkout 内同票 `reference-evidence/REFERENCE-RESULT.md`，并完整读取/解析四份既有 XERJ JSON，摘要记在 TRANSPORT-VALIDATION.json。沿用上一轮“状态寿命跨切片”的共享截止借鉴；未重跑检索、未重装索引，也未将 #1419 或 Nautilus 业务语义当成本票 oracle。

## 写集和保持项

只修改原有三个入口文件：`.github/workflows/issue1393-validation.yml`、`.github/scripts/issue1393_validate.py`、`.github/scripts/test_issue1393_validate.py`，以及本报告目录内的草案、静态复核脚本和新原件。

`ci.yml`、原 telemetry、业务 Wsource、Rust/Lean/会话实现、之前业务独审原件均未修改。静态检查以固定 HEAD 全部 tracked 差异与 untracked 路径枚举写集，所有差异必须属于上述三个路径或本报告目录；另逐字核对本目录上一轮历史文件。没有暂存、commit、push、dispatch、merge、切分支、Git 配置修改、tracker 操作、凭据读取或新工蜂。

旧验证主体经 AST 对拍保持：单总 3600 秒、30 秒收尾预留、lib 默认 features、S 的 `default,s_session`、每 target 单次 cargo JSON 构建、list/exact 与 binary 身份、Lean 单文件、fixture、回收和封包。原 telemetry 未改，仍是 2 GiB 起始磁盘门、jobs=1、10 秒采样；不将其描述为 OS 硬资源帽。

## message 合同

执行入口只从 `GITHUB_EVENT_PATH` 读取 push 原件中的 `head_commit.message`，不接受 dispatch、合同文件路径或默认合同 fallback。合同字段白名单的唯一实现为 runner 的 `CONTRACT_FIXED`、`GIT_FIELDS`、`HASH_FIELDS` 与 `approval_text/review_text`；完整同形草案见 `CONTRACT-DRAFT.json`。

封套格式固定为下列三段。`<完整 JSON 对象>` 是说明占位，不是可执行合同：

```text
ISSUE1393-CONTRACT-BEGIN
<完整 JSON 对象>
ISSUE1393-CONTRACT-END
```

整个 message 必须恰为一份封套，不允许前置标题、尾随正文或多封套。message 的 UTF-8 总长度最多 32768 字节；批准与独审原文各最多 8192 字节；事件原件最多 1048576 字节。JSON 拒绝缺字段、未知字段、重复键、错误类型、非有限数值、畸形、默认或 UNBOUND 身份。四个 Git 身份为 40 位小写十六进制，七个摘要为 64 位小写十六进制；拒绝重复单字符占位摘要。批准和独审文本逐字计算 UTF-8 SHA256，不作 trim 后重算。

所有固定值逐项核对：schema `/2`、仓库、精确 ref、push、workflow/runner 路径、job、Linux/ubuntu-latest、attempt=1、features、总窗口、E 同分支 False 预发布、唯一 False→True 变化及根单独批准边界。`BOUND_FOR_ROOT_REVIEW` 只表示数据已绑定供审核；任何 `AUTHORIZED` 文本均不授予许可。

实际 Git commit 对象由固定 `git cat-file commit HEAD` 读取；事件 message 与实际 message 比较全部内容，仅容忍 Git 对象和事件序列化之间一个末尾 LF 的表示差异。不 strip 其它空白；实际 message 再解析一次，必须得到同一合同。事件原件和实际 commit 原件分别保留，并分别记录 message 摘要，不能因 LF 规范化而伪称原始字节相等。

**message 任何字段都不作为命令、模块、路径或 argv。** workflow 不把 message 插入 shell。Git argv 从 `GIT_COMMANDS` 固定白名单取得；E 只通过字面 `HEAD^` 访问，S 身份与树通过固定祖先输出比较。唯一输入 Git 的合同数据是已核格式/祖先身份的 82 字节 `S E\n`，写入固定 `diff-tree --stdin` 的标准输入，禁止作为 argv。批准/独审文本只用于存证与摘要；产品 argv 仍来自原有固定验证代码及经 Cargo 原件验证的 binary。

## 身份和根后续合同

运行时要求 `GITHUB_ACTIONS=true`、Linux、固定 repo/ref/job/event、attempt=1。实际 checkout HEAD 必须同时等于事件 `after`、`head_commit.id`、`GITHUB_SHA`、`GITHUB_WORKFLOW_SHA`，workflow ref 必须为指定 workflow 和精确分支。

`created/deleted/forced` 必须是 JSON 布尔 false；`before` 必须等于合同 E，禁止新分支的全零 before。实际 F 必须为 E 的单父直接子提交。E/F 的 raw diff 只能有 mode 均为 100644 的同一 workflow 文件；完整文件必须等于 E 版本唯一一次 False→True 替换，禁止顺带改 mode、其它文件或正文。

S 必须在 E 祖先表中且 tree 匹配，E tree 匹配。固定 diff-tree 核 S→E 所有产品路径零差异，固定 ls-tree 核 E→F 全部产品对象一致，因此 E 的产品清单与 S 的清单相同；再对原 canonical 清单计算 SHA256。产品清单排除范围未扩大，仍仅排除本票三个入口、ci.yml 和本报告目录。source/prepared/execution commit/tree、实际 workflow/runner 摘要、完整 E/F 对象清单、S/E 差异原件与 run/job 身份入包。

后续事项属于根的独立权限，**当前均未执行**：

1. 完成业务最终 S/tree 与相应独审，再固定包含本轮 False 增量的 E/tree。E 固定前不得填最终 E hash，固定后不得为回填合同而修改 E 的 tree。
2. 在 tree 外填完整合同：S/E、产品清单、runner、False workflow、ci.yml、原 telemetry、批准与独审原文及摘要。批准摘要只作溯源；根须另核完整合同、实际差异和唯一动作，不能把 message 的自述当授权。
3. **根另行控制将 E 以 False 预发布到同一精确分支**。该步骤是 F 的 `before=E` 前提；False workflow 会在 checkout 前拒绝，不运行产品。新分支直推 F、拿全零 before 当例外均不允许。
4. 根审核并决定开门后，未来 F 的 tree 只能改变 False→True 一行，最终合同写入 F 的 commit message，不进 tree。F 固定后，根再核单父、唯一差异、message 完整合同及单次 push 动作；F 的 SHA/tree 由 Git 和事件绑定，避免合同自引用。
5. 只有根对该唯一动作明确准入后才可单次 push。当前工蜂没有 push/True/真实 CI 授权。attempt>1 被拒绝；该机制不提供跨独立 push 的全球一次性令牌，根仍须控制单次发布并登记实际 run ID，不因旧消息允许另跑。

单分支 push 路径用于避免新 workflow 必须先进入默认分支才能 dispatch 的注册前提。本轮未访问 GitHub，未验证远端该分支、仓库规则、Actions 配置或实际事件形态；不能将本地 toy 成功视为真实运输成功。

## 实测与限制

最终 `python3 -B -m unittest discover -s .github/scripts -p test_issue1393_validate.py -v`：**29 项通过，2.315 秒**。其中保留原 19 项测试覆盖，新增 9 项运输组测试与 1 项 stdin 数据边界测试。首轮 28 项也通过，原件另存 `transport-toy-round01.*`，随后补 stdin 负控及加强实际摘要错配覆盖，再跑 29 项；最后加强所有实际文件摘要、缺祖先/错父和同义 JSON 字节不符负控，29 项再次通过（verified 原件）。旧轮的 19 项和原失败史未覆盖。

负控覆盖：缺失/畸形/多封套/超界、重复和未知字段、每个字段缺失、类型与全部摘要占位/错误、批准文本篡改、错误路径/features、错误 GitHub 身份/attempt、全零或非 E before、新分支/删除/强推、事件/真实 message 不符、非单父、source/tree/产品差异、workflow mode 变化、False 和坏合同零外部 Popen、注入样式文本只作数据、固定 Git argv 与有界 stdin。True 仅存在于临时 toy 的内存环境；工作流文件始终为 False。

最终静态检查通过：两个入口 Python AST/临时 py_compile、YAML、内嵌 Python AST 与 `bash -n`、精确 push/False/60 分钟门、旧 CI 五行追加保持、既有验证主体 AST 相等、固定 Git stdin 只读协议、允许写集、产品不变、旧历史逐字保留、空白检查。Git 协议检查只读现有 HEAD/父提交，不创建 toy commit 或改 Git 状态。

最终原件见 `transport-toy-verified.stdout/stderr`、`transport-static-verified.stdout/stderr`；摘要与检查收据见 `TRANSPORT-VALIDATION.json`。收据首次生成时把静态通过输出的 8 行误计为 9 行，断言停止在写收据之前；改正计数后生成，原错误见 `transport-receipt-round01.stderr`，不涉及代码或测试失败。本轮仅自查，未冒充管理者独立评审。

本机 cargo/rustc、产品、Long、Lean 构建、fixture、网络、真实 CI 全部 **NOT_RUN**。Linux subreaper、GitHub 事件实际表示、上传结论及真实 Rust/Lean 结果仍待根将最终源和合同固定、独审并准入后的真实运行验证。当前缺最终 S/E、完整批准/独审溯源及根对 E 预发布/F 单次 push 的分别许可；本交付不替代这些缺项，也不关闭票。
