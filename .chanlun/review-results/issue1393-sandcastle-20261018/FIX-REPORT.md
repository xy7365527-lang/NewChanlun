# #1393 首独审返修交付

本轮完成三项已授权合同修复和 Python/纯 Node 定向验证，保留未暂存 WIP，待管理者收包及新的独立 Sandcastle 会话复审。自测不构成独审通过或整票验收。

名分：绑定 #1393 的工作草稿。实际起始/结束 HEAD 均为 `ab3f3fb0e1638f1ef3543b8b1948cd1856dc8675`，分支 `codex/1393-sandcastle-w-20261018`；起始工作区干净，未切分支、暂存或提交。首独审针对产品 `c25f35f1dfe088affa07f18479f6736d069fc9a6`，报告中的 review session 为 `01a14d3d-f69b-7281-a1b9-f4e9502218bd`，真实进程 exit=0、业务 blocked。本工蜂不将该 session 冒认为自己的 session，也未声称是父任务的原生子代理。

## 三项缺口与实际修复

| 首独审发现 | 本次修复及证据 |
|---|---|
| P1：只认 W 后缀，未知基串、重复 W、空 token 被放行 | Python `base_rule` / JS `baseRule` 先识别五个完整获准组合，再选 schema/轴；删除完整轴 fallback。保留四个已签旧组合及本树 SECOND/1 + W 组合，明确拒绝尚未组合的 new-bi/2、segment-second/2、center-seed/1、center-pair/1。11 个非法组合均先重算内容身份再要求规则门拒绝；六个段 schema 组合覆盖正确接纳及错误版本拒绝。 |
| P1：组 `initial_direction_evidence` 未绑定真实原件与同 cut 初始化对象 | Python 来源提取加入初始化证据两端；生产审计将原始账传入 `validate_sources`，按确切八键引用与 seq 比对 OHLC。`_verify_image` 在每代出生/撤回处理完成后检查唯一初始化对象、各组首对与原始引用一致性，未定/知识状态亦与同 cut 初始化对齐。撤回记录仍在其出生代核原件，不借当前 raw 修订。 |
| P2：JS 新增 payload 只核 input_refs，未核原件坐标/OHLC | `validateTB02CutSources` 将 payload 坐标限定在对象引用内，并以 `identity_key + revision` 查对应 raw 原件，核坐标与 OHLC；活动和撤回记录都执行。按对象出生/撤回边界投影活动集检查初始化链接，撤回组不能借当前初始化对象通过。抽取并复用实际生产 `validateTB02Identity`，Node 回归联合调用 typed、sources、identity。 |

只核来源、版本和对象引用，不重算 W 几何。普通方向见证的 `previous_acc` 仍可为合并累积段；只有首 raw 对的两端都作为原件比对。Python 既有无 raw 上下文的来源提取调用保持可用，生产入口显式传入完整 raw 账，不在失败后改走无上下文调用。

同 owner 接线仅改 `s_query_integrity.py` 和 JS candidate 的调用。核对 `IncrFactsState.append_folded` 可见方向来源逐步携前序来源，当前组 input_refs 已含初始化依赖，无需修改 Rust producer。没有修改已审 W 算法、已裁 geometry、Rust/Lean/冻结 source 或 #1419/#1488 工位。

## 回归与实际检查

- 从固定基线经 `git show` 只读加载 Python 模块到内存，复现未知基串、重复 W、空 token、组首对左端 high/close 从4改3被放行。用 Node VM 只读加载基线 JS，复现未定 raw.high=11、账中 high=10 的合法几何反例通过 typed、来源与重算 SHA。未更换 checkout 源码。
- `test_tb02u_review_regressions.py`：6 项通过。共享矩阵含6个合法快照（活动、撤回精确修订、空输入、稀疏坐标），63个来源反例、6个同 cut 链接反例，另有5个合法规则、11个非法规则、6个段 schema 组合及同 cut 修订不一致检查。69个来源/链接反例均在来源门前通过字段与实际内容身份检查，并显式检查合法 OHLC 几何。
- 原有 `test_tb02u_contract.py` 5项、`test_tb02d_validate_once.py` 9项通过；合计20个 unittest 方法。旧 JS 子项的 schema-only 发生史不改写，新增 Node 联合校验补足其缺口。
- `node --check s_session/browser/tb01c-client.js`、`git diff --check` 通过。已有 `verify_static.py` 通过：五个几何核与原始基线逐字相同、规则 token 仅追加、原 Python 文件 AST/Lean 无占位扫描通过；新增测试由 unittest 实际加载。
- 检查命令、次数、源码字节数和 SHA256 见 `FIX-VALIDATION.json`。它不写自己的递归哈希；收包时应另算该 JSON 的 SHA256。`MANIFEST.json`、首独审实体、旧验证原件及 reference-evidence 保持原字节。

编码前 reference-coding 复用管理者已归档实体。本轮读取已有严格校验器、合同测试和 Nautilus `_run_streaming` 原件：session 覆盖整个流，engine 跨 chunk 保持；该结论仅约束状态寿命，不是 W 的定义或算法来源。没有重新联网、检索索引或引入外仓判据。

## 明确 NOT_RUN 与尚待复审

cargo/rustc、Rust 编译/测试、产品二进制、正式 S 会话/API/浏览器、产品新进程重放/故障恢复、Long/真实历史批处理、lake/Lean 构建、fixture drift、CI、#1419/#1488 最终组合验收均 NOT_RUN；无资金/订单等外效。没有认领、评论、push、merge、关票、改 claim/依赖/环境配置或读取凭据；未开启其他工蜂。

新增 Lean 的 fixture drift 义务没有豁免。本机禁止可能自动 lake build 的脚本，本轮未运行。后续退出码必须保留真实含义：0=无漂移，1=真实漂移，3=缺 lake，4=构建失败，5=导出失败。Lean 尚未接默认 roots，属于待实际验证入口接线；静态无占位不等于已核证明。

首独审的三项代码缺口本轮均已修复并通过定向自测，但仍标记“待独立复审确认”。新的独立 Sandcastle 会话须检查全部三项及本轮尾部 diff。正式执行证据、组合接口、Lean 验证和封包义务保留，不能凭本报告关票。
