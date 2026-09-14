# #1373 TB-02-A 候选交付报告

本报告绑定 [#1373：已定方向与无同价竞争的包含及局部分型](https://github.com/xy7365527-lang/NewChanlun/issues/1373)，名分为活期绑定票的工作草稿。当前结论是**本叶具名域的本地验证与独立评审已经完成，候选交付待 main 批准**。Rust 冻结候选通过适用测试及 F1/F2 独评；最终 runtime-v3 完成 20 次正式轨迹、10 对完整核心字节一致和有界独立 oracle 核验；浏览器两项 HIGH 已修复并获独立复评 Approve，真实 GUI 六个场景、11 项核对通过。候选 commit、PR 与 CI 尚未执行，不预记结果。

本报告不关闭 #1373、父票 #1360、SPEC #1340 或 #1323 整图。#1342 初始方向一般域与 #1343 同价极值身份一般域仍未解决。当前工作树基准为 `65298686cf3e7c9e94e0b9ceeb83aebad2b5530b`，候选实施尚未记录 commit、PR、CI 或 main 合入结果。证据索引为本目录的 [EVIDENCE-INDEX.json](EVIDENCE-INDEX.json)；索引逐文件记录 SHA-256、字节数和真实保全位置，不把仓外工作目录当作仓内交付实体。

## 1. 外部行为与输入边界

本片要求完整、具名的原始 OHLC 经正式 S 会话入口产生包含步骤、包含组、原始成员映射及四种局部形态，并由公共查询、变化和浏览器核对来源与生命周期。成功实例的前件是**包含发生前已有真实方向见证，且每次实际选中的 high/low 各有唯一原始根**。输入账簿是按定义构造的完整整数工作例，具有明确单位和源坐标，不是交易所行情记录，也不定义 tick 聚合或交易政策。

新增 [ohlc_integer_tb02a_v1 profile](../../../s_session/profiles/ohlc_integer_tb02a_v1.json) 与 `s-ohlc/1` 输入固定配对，四价为规范整数字符串，满足 low≤open/close≤high。它使用完整已给定 OHLC、固定序号 clock、无重采样，且明确 `not_product_default=true`。公共会话继续 `s-session/2`；旧 tick profile 保留独立 schema、规范字节和顺序版本，不通过宽松回退解释新域。

| 对用户可核对的事实 | 实施合同及现有证据中的行为 | 本片边界 |
|---|---|---|
| 包含过程与方向 | 同一包含核输出比较、端点等价类、包含位、方向建立见证及逐步折叠；S 绑定真实源修订。 | 初始包含无方向时具名等待，不前瞻补方向。 |
| 组与原始映射 | 组保留全部实际成员、组首锚、open/close、high/low 原始根；组序号与原始锚分别核验。 | 不把窗口外依赖伪装成员，不把同价多根自动选成 first/last。 |
| 局部形态 | CC-006 由真实三个后包含相邻组形成 RISING/TOP/FALLING/BOTTOM；窗口使用三组锚。 | 不造第五种叶，不以原始三 K 直接替代后包含三组。 |
| 描述与后续发展 | CC-007 只随 TOP/BOTTOM 产生；形态描述、未裁强弱标签、实际后续 OHLC 与六参考价关系分别表达。 | 强弱无私设数值交易门，形态不推出后续成笔、反转或交易。 |
| 知识与等待 | CC-054 逐请求保留输入质量、适用性、计算、有效性四轴；九个目录轴来自相应封存 cut。 | 运行成功不把 `proof_status=not_proved` 改成一般域已证。 |
| 修订与历史 | 原始修订、组拆并、撤回/替代进入公共变化；内容改变产生新 ID，旧 cut 可复查。 | 不覆盖首次获知，不恢复已撤回旧身份。 |

核心实现入口可审阅：[包含核](../../../rust/src/theta_v0/parser/inclusion.rs)、[S 会话入口](../../../rust/src/bin/s_structure_session.rs)、[OHLC 输入与持久绑定](../../../rust/src/bin/s_session_v2/tb02a.rs)、[结构事实与来源](../../../rust/src/bin/s_session_v2/tb02a_facts.rs)、[Q 合同校验](../../../s_session/s_tb02_contract.py)、[浏览器 collector](../../../s_session/browser/tb01c-client.js)、[浏览器页面](../../../s_session/browser/index.html)。这些链接指向候选工作树，历史收据的受测版本以各自 source manifest 为准。

## 2. 独立输入、oracle 与运行范围

仓内保留 [完整原始账簿](../../../s_session/tests/fixtures/tb02a/raw-ledger.json)、[独立手算 oracle](../../../s_session/tests/fixtures/tb02a/hand-oracle.json) 和 [范围说明](../../../s_session/tests/fixtures/tb02a/ORACLE.md)。手算常量按已裁包含与分型定义编写；正式对象 ID、cut、规范封装与生命周期由实际运行产生，不预造在 expected 中。

| 来源 | 冻结 SHA-256 |
|---|---|
| raw-ledger.json | `b321c037723cc00d597b441cd5eca6736249240d812dde2ede03e28e44fca7bf` |
| hand-oracle.json | `6a9489be8220d794950fec676d316f6f55e4910f157c174954d57c2cea1304ec` |
| [独立核验器](../../../s_session/tests/tb02a_oracle_check.py) | `6bcd5c973495055bf95deef5112d118fa94a75a4af51cfafa4d207203dadd280` |
| S 二进制 | `762af6ffdc47171298e3b0b5a7340c4711587198769a991bb2963aca57026fa2` |
| 最终浏览器 client | `e0907290a2f89570454a06f309ea1c917e24ca2d308b41070c1615c70ed81793` |
| 最终浏览器 HTML | `fef9c421aaee89f891bbad2ca78d73d6cd9dc14b644c5722552537288d49038e` |
| 最终浏览器回归文件 | `18be8d6709748926393de55613c4c34acb62c1fedfe64cb69a55a8369de5ea01` |

runtime-v2 与最终 runtime-v3 各有 10 个具名场景，每个以 a/b 两个新进程执行。它们涵盖 UP/DOWN 包含、原始成员等号与跨组相触、四种局部叶、high-only 预处理反例的修订前后、修订恢复，以及初始方向/同价 high/同价 low/双端同价等待。原始成员等号未成为实际获选极值的双根；跨组相触也是明确的关系实例，均不替代同价极值身份的一般裁定。

运行入口与核验程序分别为 [tb02a_runtime.py](../../../s_session/tests/tb02a_runtime.py) 和 [tb02a_oracle_check.py](../../../s_session/tests/tb02a_oracle_check.py)。实际参数、输入路径、clock、故障点、执行退出与受测 source hash 保存在仓外 RUN-PLAN、EXECUTION 及逐 run 原件中，由索引定位。数据库和 82 MB 核心原字节继续保全在仓外，不复制入 Git。

## 3. 实际完成的验证

下表每行只对其指明版本和测试缝负责。数量不相加为覆盖率，函数/替身测试、正式 HTTP 轨迹、独立 oracle 与真实 GUI 分别记账。

| 验证 | 实际结果与版本 | 小报告索引项 |
|---|---|---|
| 采集/清理/运行库锁及驱动独评 | 原始工具日志无损提取记录 9 项实际测试在 0.329 秒通过、退出 0；驱动独评 APPROVE、无阻断或非阻断发现，确认四份 Python 源未变并审查新增 CI 测试入口。该确认未重跑测试或远端 CI。 | `harness-nine-original-receipt`、`root-harness-independent-review` |
| 最终包含核测试 | 23 passed、0 failed、0 ignored；最终 inclusion.rs=`cc48d963…`，前后源未变。旧 `inclusion-tests.json` 绑定较早 `0fead18…`，不用于冒充最终源通过。 | `rust-final-inclusion` 及其 stdout/stderr |
| 最终 S 会话测试与构建 | 71 passed、0 failed、1 既有 ignored；运行 36.50 秒；构建成功并得到 S762 二进制。ignored 为既有隔离数据库诊断，不计入已执行通过项。 | `rust-final-tests`、`rust-final-build`、`rust-final-source` |
| 独立 Rust F1/F2 | PASS，2,647 次检查；F1 四代、F2 三代；全部历史 cut 逐字段不变，最终根可读，绑定同一 S762。 | `rust-independent-result`、`rust-independent-preservation` |
| Q 定点回归 | 最终 S762 上 OHLC Q 23/23，旧 tick Q 31/31；来源在该阶段前后相同。 | `q-ordinal-delivery`、`q-ordinal-q23-result`、`q-ordinal-legacy-stderr` |
| 浏览器函数及正式 makeHooks | 37/37：旧 32 项加实际 OHLC 5 项；使用真实 S/Q 导出的 0..6 七个 cut，累计 77 次 typed 记录校验。此为 TestOnly 响应与生产函数路径。 | `q-ordinal-browser` |
| 本机真实 Q HTTP + 正式 helper | current6 → AsKnown5 → Watch6；3 个命令、11 次 HTTP、3 次完整安装。读取已停止的自建 S 六代库；不是可视 GUI。 | `q-ordinal-http` |
| 原失败库复查 | 对 runtime-v1 首失败 G5 库逐字副本新鲜核验通过，原数据库/WAL/SHM 字节保持。 | `q-ordinal-original-recheck` |
| runtime-v2 正式轨迹 | 20 次已采集；810 条完整核心记录，合计 82,274,030 字节。各 run 的采集状态本身仍写明待 oracle/浏览器，不单独宣称验收。 | `v2-run-plan`、`v2-execution`、逐 run 小结果 |
| runtime-v2 独立 oracle 与双跑 | `PASS_BOUNDED_ORACLE_AND_EXACT_DOUBLE_RUN`；10 对完整核心字节相同，无排除字段、无身份重命名；总检查 2,178,747，比较叶值 879,302。核验器执行退出 0，执行前后 SHA 相同。 | `v2-oracle-result`、`v2-oracle-execution` |

| 最终浏览器修复回归与 HTTP | 47/47（旧 32 + 目录漂移 1 + 原 OHLC 5 + 完整候选 9）；原三反例拒绝、健康提交；真实 HTTP 3 命令/11 请求/3 完整安装。Q23/Q31 因 Python/S 源逐字未变复用。 | `q-final-delivery`、`q-final-green`、`q-final-http` |
| 最终浏览器独立复评 | Approve、findings=[]；独立原脚本健康基线 COMMITTED cut-6，缺目录/伪 receipt/依赖伪成员三例均 REJECTED。 | `q-final-independent-review`、`q-final-independent-replay`、`q-final-independent-manifest` |
| 最终 runtime-v3 | 20 次退出均为 0；20 份完整采集结果；810 条/82,274,030 字节；10 对逐字相同，无排除字段、无身份重命名；独立 oracle 总检查 2,178,747，叶值比较 879,302，核验器前后未变，退出 0。 | `v3-run-plan`、`v3-execution`、`v3-oracle-result`、`v3-oracle-execution` |
| 真实 GUI | `PASS_BOUNDED_GUI`：Codex In-app Browser，6 个正式 v3 场景、11 项核对；截图、AX 和可见 DOM 已保全。6 个只读 Q 均停止，数据库主文件字节不变。 | `gui-final-result`、`gui-final-manifest` |

最终 v3 全部 20 个 run 的浏览器三文件哈希与最终独评、GUI 相同。v3 oracle 本身仍写 `browser=not_evaluated`；真实 GUI 由独立的 `gui-final/RESULT.json` 承担，不篡改 oracle 的范围标签。

2,178,747 是核验结果顶层的总检查数，包含跨 run/配对检查；逐 run 检查数合计为 2,176,974，二者不是两个可相加的覆盖统计。810 条与 82,274,030 字节包含 a/b 两臂完整核心记录，不表示独立语义场景数量。

## 4. 已修问题与证据链

**F1：窗口外方向、分组边界依赖和请求来源。** 仅保留三组成员会漏掉建立方向或确认相邻分组所用的窗口外输入，原始修订可能没有进入结构身份。合同补入 `dependency_refs`，保持 `raw_refs` 仅含成员；CC-006、CC-007 与相关知识请求按实际依赖传递来源。CC-007 的形态描述、原始强弱描述输入、后续发展各自绑定实际来源。F1 独评使用真实 BOTTOM 窗口及窗口外 raw0 的多次修订，核对新身份、等待撤回、恢复和历史保留。

**F2：成员变化后数值/三锚恢复。** 三组 RISING 追加输入改变右组成员，再修订该后继使原数值与三锚恢复；右组此时已有新的确认边界见证，不能复用曾撤回的 open 版本。独评的三代目标 ID 各异，旧 cut 保持，最终根可读。修复的构建/测试以 `rust-checks/f1/` 收据封存，独评以 `independent-review-final-rust/RESULT.json` 为准；原临时独评目录已由 `PRESERVATION-MANIFEST.json` 指向的保全副本接续。

**Q 组序号与原始锚混淆。** runtime-v1 的实际 G5 为 `window=[0,1,4]`、`merged_index=[0,1,2]`、成员 `[0] / [1,2,3] / [4]`，旧 Python/浏览器误将组序号等同原始锚，拒绝合法结构。修复后分别核连续组序号、真实成员首坐标与窗口锚；Python 全 cut 审计再绑定同 cut CC-005 分组、方向与确认来源。原 v1 FAIL 和此前 setup RED 均保留。Q23、37 项函数检查、真实 HTTP 和原失败库副本复查说明这项错误拒绝已在对应版本修复；它们没有覆盖下一节发现的错误接纳。

## 5. 浏览器两项 HIGH 的历史反例与最终修复

修复前独评使用冻结正式 collector、正式 makeHooks 和真实 S762 的健康 cut-6，通过 TestOnly fetch 提供封装自洽的响应。`independent-q-review-pre-fix/replay-2/RESULT.json` 实际记录以下结果：

| 情形 | 旧版实际结果 | 判断 |
|---|---|---|
| 健康基线 | COMMITTED cut-6 | 基线可提交。 |
| CC-006 raw_ref 的 receipt 改成 raw_history 中不存在的值 | COMMITTED cut-6 | HIGH H1：错误接纳了伪来源收据。 |
| 把 CC-006 dependency_refs 移入 raw_refs，伪装成员 | COMMITTED cut-6 | HIGH H1：错误接纳了成员/依赖名分混淆。 |
| 已签 116 项目录删去 CC-005，只剩 115 项 | COMMITTED cut-6 | HIGH H2：错误接纳了不完整目录。 |

独评冻结 client SHA 为 `6168c96bf3fc8e909a942f8d5e37b4c6286defa92e25f27eb434eedd04923fdc`，HTML SHA 为 `0578f720a3e378dba226f5f60868d15efe8ac85f3e34f5dda2c59bc1738edb60`。67 个文件的清单见 `MANIFEST-v2.json`，包含冻结源、请求、响应、投影、提交候选和实际复跑结果。证据写入器的首次补采 `replay-1` 曾因本地运行元数据序列化失败；该 REJECTED 不能作为生产拒绝证据，完整反例以修正写入器后的 `replay-2` 为准。

这些反例不是实际网络或 GUI，也不是结构 oracle；它们证明了旧版正式消费者的错误接纳。旧版 BLOCK 与 runtime-v2 原件保留，v2 的 PASS 仍只覆盖其绑定版本的有限 oracle/核心字节范围。

最终修复把全部 116 项目录的 ID、版本、数量和 id/kind/title/domain/branches 静态列绑定到完整摘要；对象八键 raw_ref 逐值核完整 raw_history，活动对象取相应 cut 有效修订，撤回对象按首次获知代选择当时唯一 CC-005 组，分别核成员、组序号和方向/确认依赖。CC-006 也核真实内容身份；内部仅用 BigInt 恢复 Rust canonical 整数，不重算市场结构、不把 raw.seq 当 generation。

作者最终回归 47/47，沿原独评脚本接新冻结源得到健康 cut-6 提交、三个旧反例具名拒绝；实际 HTTP 再完成 current6 → AsKnown5 → Watch6 的 3 命令、11 次请求、3 次完整安装。最终独立复评 `independent-q-review-post-fix/REVIEW.json` 为 **Approve、findings=[]**，并在 `replay-1/RESULT.json` 独立接回同一正控与三项 REJECTED：缺目录报“完整冻结目录数量不符”、伪 receipt 报“对象八键来源未绑定完整原始历史”、依赖伪成员报“形态序号/真实成员与该cut组映射不符”。

独评另核原始修订历史、活动/撤回组来源，验证对象代际整体平移 100 而 raw.seq 不动仍通过、错误 first_known cut 被拒；目录漂移测试进入既有浏览器测试入口。该独评批准的范围是 H1/H2 最终修复，与后接的 v3 和 GUI 证据共同支撑本叶候选交付；它本身不替代正式轨迹或 GUI。

## 6. 未覆盖范围

独立 oracle 原件明确列出四项未独立手算覆盖：未列出的方向建立见证快照全价；CC-007 后续每条价格关系及全部描述文本的独立 expected；未单独暴露的两向包含原始关系见证；成员等号与跨组相触原始价虽完整保全，但未独立重写包含判断。组确认后继已经按账簿核对。报告保留这些限制，完整字节自洽与双跑相同不把它们提升为独立语义证明。

最终负例独评、runtime-v3 与真实 GUI 收据均已接回。GUI 读取已经提交完毕的正式 v3 示例数据库，未验证界面打开期间的持续输入或实盘交易；同价等待 GUI 核对的是 high 双根，其他负控由正式轨迹覆盖，不冒充 GUI 全枚举。本报告不宣称全市场回放、一般域形式化证明、生产行情默认启用、订单/资金/账户动作、父域完整经营模型、候选 CI 通过或 main 合入。#1342/#1343 的一般域及跨片成功义务仍由原票和最终父域验收承担，不能凭本片收据勾完父域条目。

## 7. 真实 GUI 与交付状态

真实 GUI 使用已完成正式 v3 的六个数据库：UP、DOWN、四种局部叶、修订恢复、初始方向等待和同价 high 等待。核对 UP/DOWN 的真实组成员、组首及 high/low 根；四叶均可见；CC-007 强弱标签与后续发展分开展示；初始方向缺失保持准确等待，同价 high 保留两个原始根。

修订场景在当前 cut-6 显示三个新窗口，切至 AsKnown cut-5 显示旧窗口，再回当前 cut-6 后，meta、objects、withdrawn 的可见内容逐字恢复。展开原始见证时，旧 raw2 为 revision=1、seq=2、source_coord=2，当前 raw2 为 revision=2、seq=5、source_coord=2，明确区分输入修订、接纳序和源坐标。截图、AX 与可见 DOM 原件均由 GUI manifest 保全。

GUI 的 `up_member_equal_and_touch-a/01-current.ax.txt`、`four_local_leaves-a/01-current.ax.txt` 和 `four_local_leaves-a/01-current.png` 是加载中或无变化采集，已具名排除，不作为有效验收证据。六个 Q 全部停止，报告只称数据库主文件字节未变，不把它扩展为所有运行元数据未变。

| 交付 ID | 当前状态 | 对应证据或剩余工作 |
|---|---|---|
| `FINAL_Q_REVIEW` | 已完成 | 最终三文件冻结，47 项作者回归、正控与三反例、独立 Approve/findings=[] 已接回。 |
| `FINAL_RUNTIME_V3` | 已完成 | 20 次正式运行、10 对完整字节及有限独立 oracle 通过，源版本与最终独评一致。 |
| `FINAL_GUI` | 已完成 | 六案例、11 项真实 GUI 核对通过，当前/历史/返回当前、原始修订与等待均有可见证据。 |
| `FINAL_SOURCE_AND_PRESERVATION` | 已完成 | FINAL-SOURCE 固定 27 项变更源及 S 二进制；FINAL-PRESERVATION 对明确纳入的 12 个目录封存 12,160 个文件、1,257,533,627 字节，各文件 SHA/大小可核。 |
| `FINAL_DELIVERY` | 候选待提交，main 待明确批准 | commit、PR、CI 尚未执行；只能在实际完成后填写。#1360/#1323 保留原验收边界，不随本片关闭。 |

## 8. 证据保全与复核入口

仓内可审阅实体是本报告及同目录索引。所有仓外保全位置共同前缀为：

```text
/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1373-tb02a-implementation-20260913
```

索引对 INTEGRATION-CONTRACT、S 构建/测试、独立 Rust 结果及保全 manifest、Q 定点与最终修复收据、v2/v3 的计划/执行/oracle、浏览器修复前后独评及最终 GUI 分别给出具体文件位置、字节数与 SHA-256。v2/v3 大核心文件可由各自 `v2-oracle-result`、`v3-oracle-result` 中逐 run 的 case/arm、core_bytes、core_sha256 对照原件；数据库及原始请求/响应继续留在对应仓外 run 目录。最终 `final-preservation` 索引项绑定 FINAL-PRESERVATION.json（SHA-256 `c8b3af91fc5c7647bfe5714cc25267b0d34a0fa9be1b76712862a1e0ea1d7b51`），对其明确纳入目录的 12,160 个文件逐项保存大小和 SHA，合计 1,257,533,627 字节；范围仅为 `runtime-v1`、`runtime-v2`、`runtime-v3`、`gui-final`、`rust-checks`、`q-validation`、`independent-review-f1`、`independent-review-final-rust`、`independent-q-review-pre-fix`、`independent-q-review-post-fix`、`reappearance-probe-v1`、`reappearance-probe-v2-boundary`。它排除构建缓存及中间编译产物、可变 CURRENT-HANDOFF、尚不存在的 commit/PR/CI 收据、清单自身和单独保留的 driver-smoke-v1/v2 原型，不代表整个仓外工作目录无遗漏封存。最终二进制另由 FINAL-SOURCE 与运行前源副本绑定。

总保全清单冻结之后补到的 `HARNESS-NINE-ORIGINAL-TOOL-RECEIPT.json` 与 `ROOT-HARNESS-INDEPENDENT-REVIEW.json` 单独列入本索引，作为附加保全，不改写 FINAL-PRESERVATION。前者从本次主控原会话日志提取当时完整 event/result，真实记录 9 项通过，不是重构或重跑；较早六项 HARNESS-TESTS 原件继续保留，不冒充九项。

复核应先核收据与其绑定源，再按各 `*-COMMAND.json` 或 RUN-PLAN 的实际入口在新输出目录复现；不覆盖既有失败或成功原件。报告写入阶段只整理现有证据，没有重新运行产品轨迹或改变生产判据。
