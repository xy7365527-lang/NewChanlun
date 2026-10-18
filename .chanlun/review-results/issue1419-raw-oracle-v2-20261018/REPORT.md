> **2026-10-18 v3 修复状态更新**：外部 v2 独审结论为 **CHANGES_REQUIRED**，八项见 `fixes-v3/V2-INDEPENDENT-REVIEW.reference.json`。本目录 driver 已进入 v3 修复树；当前为修复工蜂定向验证通过、**PENDING_INDEPENDENT_REVIEW_OF_V3_DIFF**，不能沿用下文旧测试通过声明认定新树已独审。具体修复、实际失败与复验、字段量词和新候选合同见 [fixes-v3/REPORT.md](fixes-v3/REPORT.md)。
>
> 下文保留 v2 报告及其 `5cb7253d…` 原始身份，`validation/` 原输出不改。v3 接手基线为 `7cc205c8…`；新的代码/结果身份仅在 `fixes-v3/` 登记。原泛称“工具已补齐”不再适用于当前验收；静态字段子量词通过不覆盖 first_visibility、known_at、AsKnown、entity_revision、raw_history、withdrawn_objects。v3 候选合同在 `fixes-v3/EXACT-RUN-PROPOSAL.json`，原合同保留历史。全部授权仍为 false。

# #1419 第二轮：受控 raw 与独立见证

已补成一个声明为 **TestOnly** 的非空 raw 定义工作例：139 根 OHLC，经独立逐项演算得到 35 个分型、34 笔、10 个完成段、1 个进行中尾部、2 个非退化 strict 中枢、1 个连接段和 1 个完整相邻对。完整链条在 `INDEPENDENT-WITNESS-LEDGER.json`，逐项引文在 `DEFINITION-REFERENCES.json`。本包是开放票 #1419 的工作草稿，**不代表本票完成**。

`authorization=false`。Rust、cargo、rustc、Lean 构建、产品、服务、真实库、正式新进程重放、持久恢复和浏览器验收全部 **NOT_RUN**。没有 commit、push、merge、切分支、tracker 写入或新工蜂。只修改本目录；基线仍为 `5cb7253d46d973669e7d10cd3331e12c9ef313b1`。外部 Sandcastle 工蜂身份不等于父任务原生子代理注册；真实 session_id 由运行时事件登记。

## 原件与适用域

`RAW-INPUT.json` 是本次新造的完整原始 OHLC 原件，输入值本身就是整数 tick、受控单位量 `volume=1`。这不是市场数据认证，也没有把产品段输出改名为 raw。`raw/CONSTRUCTION-RECIPE.json` 保存构造法：先给定简单价格折线控制点，每两个转折点之间均分四步，再展开成宽度 8 tick 的 OHLC；生成法不读取任何产品输出。产品候选入口只接 `s-ohlc/1` 事件，不接 SegUnit。oracle 则只读实际 bars，从三 K 窗口重新求出分型和笔，没有读取生成法的宏观控制点来当段答案。

本次使用已裁定义造 implementation fixture，不新增语义选择。Python 演算器有显式有限域：raw 无包含、相邻分型异型、特征元素无包含、非退化初三笔。域外直接失败，不能替产品准入判据。实际每组仅一根 raw，最高/最低根集均为单根；全部 138 对 raw、137 个内部三 K 窗口、34 个笔的双坐标间隔和所有终结见证均列出。没有 raw 同价、相邻同型分型、多根极值或转折窗成员，因此这些分支不触发；不声称覆盖其全分类。不同非相邻顶底重复同价不构成同型相邻取舍。

原 600 市场 raw 和原缓存仍只读留在前轮包 `/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/raw-run01-artifacts/`。本轮复用其原件清单、条件账簿和 23 项检查原输出，没有重封大缓存、重选历史窗或重跑原 23 项。其已知 6 完成段 + 1 进行段、1 中枢 / 0 相邻对保持原结论；受控 raw 不替代该市场样本，更不代替全图完整历史。

量域的实据是冻结 `s_session/profiles/ohlc_integer_tb02a_v1.json:10` 的 `volume_unit="1"` 与 `raw_input.rs:54–62` 的规范整数/精确相等门；OHLC 事件字段来自 `tb02a.rs:68–81,165–196`，close 为价格索引。`s_session/README.md:162` 声明该 profile 是非默认整数 OHLC。**这证明现有具名 profile 的实现合同，不推出 SPEC 普遍只允许单位量。** 前轮市场原量首条 8.33847 与它不相容的问题仍保留。本轮未改生产 profile、volume 载体或资金政策。

## 独立推导的结果

全部索引 0-based。组锚等于 raw ordinal 是本夹具无包含的推论，不是把两个坐标域定义成同一个。raw 0 提供首底的左邻；raw 138 提供最后底的右邻。每笔两中 K 相距 4，根集间排除两端恰有 3 根 raw，两个分型的三 K 支持集不相交。最后 b33 仍是未确认笔；b32 已在第 139 条输入时确认。

| 完成段 | 方向 | 成员笔 | raw端点 | 全成员区间 | 终结 | 首次具备见证的输入条数 |
|---|---|---|---|---|---|---|
| u0 | UP | b0–b2 | 1→13 | [800,1600] | CASE_TWO | 39 |
| u1 | DOWN | b3–b5 | 13→25 | [1120,1600] | CASE_ONE | 39 |
| u2 | UP | b6–b8 | 25→37 | [1120,1920] | CASE_TWO | 63 |
| u3 | DOWN | b9–b11 | 37→49 | [1440,1920] | CASE_ONE | 63 |
| u4 | UP | b12–b14 | 49→61 | [1440,2720] | CASE_TWO | 87 |
| u5 | DOWN | b15–b17 | 61→73 | [2400,2720] | CASE_ONE | 87 |
| u6 | UP | b18–b20 | 73→85 | [2400,3200] | CASE_TWO | 111 |
| u7 | DOWN | b21–b23 | 85→97 | [2720,3200] | CASE_ONE | 111 |
| u8 | UP | b24–b26 | 97→109 | [2720,3520] | CASE_ONE | 123 |
| u9 | DOWN | b27–b29 | 109→121 | [2080,3520] | CASE_ONE | 135 |

上述都是定义演算的前缀获知点，不是捏造的产品 generation/known_at。账簿另记终结见证笔全部确认的较晚前沿，例如 u9 的 b32 见证首次可见在 135，全证据笔确认在 139。正式源若给出不同获知点，须保留差异核查，不能移动 expected 使其通过。

流首段起点为 raw 1、最低价 800，u0 全部成员笔的反向极值唯一在那里；无更早已成笔需要并入，不触发首段重定位。每段都恰好 3 笔是此输入的结果，演算器不是“三笔即成段”：缺少其后终结见证时不会发布；前后缀负控逐个检查了 38/39、62/63、86/87、110/111、122/123、134/135。

例如 u0 的第一特征三元组来自 b1/b3/b5，区间分别为 `[880,960] / [1440,1600] / [1120,1520]`。主顶分型严格成立，a/b 有缺口且 c 未封闭。第二序列按原段同向取 b4/b6/b8，分别 `[1440,1520] / [1120,1280] / [1200,1920]`，无包含，出现底分型。b8 同笔高点越过旧峰 1600，但 #1344 明定同笔先做分型再做越界；此前逐笔无越界，故先确认，不是被越界杀死后复活。其他三处 CASE_TWO 同样逐笔列出，不拿 donor 的段数组作 oracle。CASE_ONE 中原有缺口被 c 封闭者按已登记 strict 规则处理，未启用 optimized。

| 中枢语义键 | seed | 全成员 | Zn | ZD/ZG | DD/GG | D/G |
|---|---|---|---|---|---|---|
| c0 | u0,u1,u2 | u0…u4 | u0,u2,u4 | 1120/1600 | 800/2720 | 1440/1600 |
| c6 | u6,u7,u8 | u6…u9 | u6,u8 | 2720/3200 | 2400/3520 | 2720/3200 |

两个 seed 都是完整五位 `11111`，核心宽度均 480；所有段初三笔重叠严格大于零，因此未依赖尚有分歧的相切退化路径。u5 的 low=2400 > c0.ZG=1600，排除为成员，按 `last_member+2` 从 u6 续扫；左右两端分别为 c0/c6。u10 为进行中尾部，归属 `NOT_YET_ASSIGNED`，不参与 seed。c0 在第 63 条形成，u4 到达时 D 从 1120 升至 1440、GG 从 1920 升至 2720，而核心不动。c6 于第 123 条首次具备 seed，第 135 条加入非 Zn 的 u9；u9.low=2080 小于 Zn 的 DD=2400，正好区分“全成员外包络”与已裁 Zn 外缘。

相邻对完整 16 格在 JSON 中：前 `(DD,ZD,ZG,GG)=(800,1120,1600,2720)`，后 `(2400,2720,3200,3520)`；前 GG 对后 DD 为 GT、对后 ZD 为 EQ，其余 14 格为 LT。核心严格上移且外缘交叠，关系叶为 `UP_EXPANSION`。这是 #1424 的候选输入准备，不宣称已构造高级中枢、趋势实现或关掉 #1424/#1429。

## 修订、撤回与恢复

`raw/ORDER.json`、`expected/TRANSITIONS.json` 和四份具名 expected 绑定全部字节、顺序和差异。每臂是 139 个初始单事件消息，随后 3 个具名修订消息，共 142 次提交候选、270 个接纳事件；最终 input_frontier=269。

1. `base`：上述完整非空结果。
2. `revise-non-zn-tail`：同一 raw 121 修订为版本 2，四价均加 32。u9 低点 2080→2112，u10 起点随变；两中枢成员、六变量、相邻对值均保持。不得把非 Zn 的新低写进 DD。
3. `withdraw-second-center`：同一消息中将 raw 74…138 修订为严格递增尾部。raw 121 为版本 3，其余为版本 2；不是删除 raw。现存只有 u0…u3 完成，u4 进行；c6、原相邻对、u4…u9 的旧完成事实失去依据。c0 保留，成员缩为 u0…u3，D/G=1120/1600、DD/GG=800/1920。旧对象的真实 withdrawn_objects/历史证据必须在产品运行时核实，不能用这个预期代替真实撤回。
4. `restore-original`：同一批位置更高版本恢复原始价格，raw 121 为版本 4，其余版本 3。领域值与 base 相同；原始事件历史、entity_revision 和获知时点不能回填成旧值。

持久恢复候选固定在第 111 条输入后：停 S、reap、同库 recover 到 epoch 2，one-shot 接第 112 条，再接确定后缀。两空库 fresh-a/fresh-b 各按同样控制顺序串行执行，比较完整原件和领域值。实际故障点注入、Begin 中断、EINTR 和数据库恢复 **NOT_RUN**。具名原价恢复也不等于真实持久恢复。

## 离线工具已补齐的范围

| 前轮缺口 | 对应现有接口 | 本轮实现与验证 |
|---|---|---|
| 启动/监督 | S3 `init/serve/advance/recover/query-serve`；Python Popen/selector/killpg/wait | 精确候选 argv 已绑定；新增监督器只开放固定 Python toy，测成功、截止、TERM→KILL、日志超限、owner失联、人工取消和 reap；产品启动仍关闭 |
| 七字段 projection | `q3-consumer.js:2196–2226`；`projection.rs:390–439` | Python 独立内容编码，固定六族重建七字段根；qcut3=frame(0x22,root digest)，由重建根得到 projection_ref，不从 qcut3反推 |
| 内容闭包 | `s-content-v3.js encode/verifyRange`；`/api/v3/content` | 遍历字段/数组/json_text 的内容引用，校所有节点 hash、树形、种类/extent、UTF-8/number、范围页内容与完整可达闭包；假传输跨页验证通过 |
| 非空 expected | 634f `tb02c/tb02e/tb02f` wire 字段 | 从同一已验证 projection 取实际笔端点/根集、段、中心、Zn角色、归属和16格比较；负控误计DD、缺节点、错锚、改页内容、撤回后残留pair均失败 |
| watch续接 | 既有 Collector.watch | 沿用 input_frontier→last_seq；不是 affected seq_range.to；前轮对应检查原件直接复用 |

完整内容验证针对**收集后的完整节点集合**，不声称每张稀疏证明页单独全闭合。分页连续性仍由原 Collector 核验。运行时逻辑节点/字节/工作量帽不等于 OS 硬帽。产品后代树追踪、同步 IO 硬截止、OS 内存/磁盘 quota 仍 **UNQUALIFIED**。toy 超限输出明确 `logs_complete=false`，不以截断日志冒充完整输出。

公共字段适配器依据 634f 冻结原件完成离线正负控；最终 source 尚未冻结，真实对象 ID、规则版本、全部 raw 血缘/低级证据 schema、22 轴目录、AsKnown 生命周期和 Native 测量仍需与最终源绑定并实际验收。完整原件对拍不得只比较本适配器抽出的字段。不存在将 `PENDING_SOURCE` 解释为暂停所有离线工程的情况。

## 验证与边界

新增逻辑定向检查首次 12 项通过，输出 `validation/tests.stdout`；补上撤回后的残留 pair 负控后，只复验相关 2 项，输出 `validation/domain-rerun.stdout`。13 个不同测试均有通过原输出。为保存 toy stdout/stderr 原件只复验监督项一次，见 `validation/supervisor-rerun.stdout`、`validation/toy-logs/` 和 `supervisor-toy.json`。这些是 Python 纯模型/假传输/toy 子进程结果，不是 Rust/正式 owner 运行。

`validation/delivery-check.stdout` 核验 Python AST、13 份输入原件、6 份独立预期资产、恢复 epoch/时钟/修订顺序、原 OHLC schema 及旧资源帽字节；最大单消息 21,647 字节，小于原 1 MiB frame 帽。没有根据这次数值批准产品运行。此前 23 项仅复用原 stdout。

`AC-DOMAIN-MATRIX.json/.md` 对齐票面四项、AC-02/AC-03 和全部 37 个具名 A，保存原文量词与源 JSON pointer。TestOnly 仅能提供已签结构子域的行为锁候选：非空真实业务运行、市场源认证、市场原量重放、全部历史、全级递归、下游消费、浏览器、双跑与持久恢复仍欠。原 600 样本不被替换，整票目标没有改成“受控raw通过即可”。

剩余外部条件是最终 source/tree/binary/独审、根对新输入和精确原帽下的执行准入、实际产品运行及真实市场域事实；没有额外语义裁点需要根选择点数。`EXACT-RUN-PROPOSAL.json/.md` 保持授权 false，原 S7/Q21 与各期限不变；新增累计资源数值只沿用前轮未准入提案，不称 OS 硬帽。Browser 延期也没有从旧 #1497 自动继承。

来源工具：当前无可调用 codebase graph MCP，按不足结果回退 rg；精确尝试 XERJ `127.0.0.1:19200/health` 得 exit 7 connection refused，未重装/重索引。票的两项原生依赖 #1414/#1359 原件均为 closed。全部来源、原件和本轮输出见 `MANIFEST.json` 与 `SHA256SUMS`。
