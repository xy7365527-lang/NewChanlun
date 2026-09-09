# #1370 TB-01-A 最终运行验收

结论：本片在具名 TestOnly 逐笔一对一退化 OHLC、相邻严格无包含、无同价端点竞争的 CC-006 域内，AC1–AC9 所需成功路径已接通并经过独立评审及真实运行验证。代码已具备 PR 审阅条件；#1370 仍开放，main 合入、整图 #1323 及其他 76 张实施叶票均不因此完成。

受验生产源码为 `76a641606d7097292fff2dc21f58c6156ae38916`；正式增量评审落在 `c30feedd05a6e814af537405a00c5f0b70fb8dc7`。随后只将已审归档分支 `38a6b3f6f7d6118ffeb22c49c4a67a736be1ab92` 整合到本实施分支，再加入本验收材料。Rust、S 外壳、浏览器和本片 CI 步骤与已验代码逐字相同；整合未写 main。签署目录原件未改。

## 逐 AC 证据

| 条件 | 实际结果与证据 |
|---|---|
| AC1 正式入口 | 正式 launcher 依次 init/accept/advance/只读服务；macOS 实际构建为 arm64 Mach-O。无 profile 的调用拒绝且不建库；正常 stop/重放恢复已验证。当前受验库的旧身份和收据保留。 |
| AC2 原始事件与一对一投影 | 固定 TestOnly profile；原始 identity、接纳序、源坐标、时间戳、price、volume、raw_text、revision 与 receipt 均可沿 input_refs/见证回查。三个大整数输入 revision=7，结构对象 revision=1；源坐标大于 2^53，接纳序仍为 0/1/2，各自无损。 |
| AC3 真实四分支 | 正式 Rust ParseLayerIncr 与 CC-006 的同次计算，未加第二个生产判定器。四分支库 8 个窗口独立整数 oracle 200 项核对通过；大整数库 1 个窗口 32 项通过。低知识/同价/域不满足保留观察，不制造 Other 分型。 |
| AC4 耐久与可达根 | 独立本机探针在 e79 写路径上完成 7 场景、46 主断言、39 次 native 命令。两个新只读连接在根仍旧切面时已读到完整新 batch；完整 hash/字节/引用闭包核过，后续根恰指向该 batch。实际写连接读回 SQLite 3.53.2、FULL=2、fullfsync=1。正常前沿交错取消自身后可重试；epoch/token/generation 不符不清理外部推进权。最终目录修复前的整个写路径与原有 8 测试逐字未变，另有最终 9 测试实跑。 |
| AC5 精确身份与读协议 | 重放返回原 receipt；异内容不覆盖；同 profile_id 异规范字节拒绝，旧切面 profile 不被重绑。R6 独立 8 脚本 213/213 通过，覆盖 HTTP→Node→Rust 的 i64 字符串往返、typed JSON/整数坏值拒绝、损坏响应 503 和 WAL 交错单事务读取。 |
| AC6 完整目录 | 真实 signed 目录 116 项：82 个 array branches、34 个 object branches。Rust CLI 和两条 HTTP 目录投影按 id/kind/title/domain/branches（含嵌套）逐值核对；未实现条目保留。推进后仅 CC-006 为 implemented/not_proved/run，其余 115 为 not_implemented/not_proved/not_run。新增目录测试已纳入具名 s_session CI 命令。 |
| AC7 浏览器同源观察 | 单个 /api/state 返回目录与 snapshot 的同一 cut；真实浏览器从目录按钮进入实例，展开输入修订、服务关系与原始成员见证。实际显示 price=9007199254740993、ts=9007199254741993 及大整数源坐标；比较栏明确 cur < prev。自建备份库从正常 116 项/1 实例注入坏 scope 后，HTTP 503，页面清除旧切面/对象/目录；修复后原切面恢复，HTTP 完整响应与注入前相等。 |
| AC8 经济未启动仍推进 | 整个试验未启动 E/B/X、未建其库、未加载经济配置。真实追加和重放继续推进 S 的 cut；scope 明确 economic=not_started。 |
| AC9 证据与检查 | 本机锁定构建、fmt、clippy、9 个具名 bin 行为测试通过；独立容器也执行最终 9 测试及 API 链。408 个 Rust 源/Cargo 文件构建前后哈希一致。下列独评、运行 JSON、浏览器可访问性记录和来源索引保留。远端 CI 以 PR 当前 head 的实际 checks 为准，不能由本报告代证。 |

本机二进制 SHA-256：`b9e35a9200eb391e8ab5902c2034d601bdacd507de7c85623fa2f1919b584766`。源码、二进制和所有入包证据的逐项绑定见 [验收索引](ACCEPTANCE-INDEX.json)。

## 独立评审与失败记录

- [正式整片评审](review-final.md)保留 e79 的 FAIL：合法对象型 branches 被误拒，AC6/AC7 不成立；没有用此前自述或旧 PASS 覆盖它。
- [正式增量复核](review-incremental.md)确认 76a 的修复和实际 116 项读取恢复，新增 0H/0M。该评审的 Linux 观察与本报告的 macOS/GUI 外部验证分列。
- [目录静态复核](acceptance/view/TB01-A-CATALOG-FIX-REVIEW.md)、[独立耐久复核](acceptance/view/REVIEW-e79.md)、[Python R6](acceptance/view/TB01-A-PYTHON-R6-REVIEW.md)及其 JSON 原件随包保留。R5 与耐久前轮失败报告也保留，供复核缺陷闭合。
- [L2–L4 合同补核](acceptance/view/TB01-A-REVIEW-LOW-DISPOSITION.md)订正评审的来源坐标前提：合同没有 seq=source_coord 要求。parser 的本地原始 K 序号和来源游标本就分字段；本机已实测两者不同仍精确回查。input_revision 与原始 revision 在本片是别名，object_revision 独立，不新造输入档案修订域。

## 范围与未完成义务

只有具名受测域的 CC-006 运行证据，未证明其余 115 项结构目录的实现，更未完成全分类、经济资格或交易。混合包含序列只作为域观察与成员映射负向检查，不能计入本片无包含成功样本；原始窗口观察和 merged 对象窗口处在不同坐标空间。当前域警告尚未显式标 raw，catalog.evidence.tested_domain 是能力范围声明，不能当作逐批原始输入域验证器。完整输入投影、初始化方向、同价端点身份仍由 TB-02 和具名残留决定承接。

TB-01-B #1371 的真实进程杀/重启、原始更正/结构撤回替换、历史和 Delta 尚未实施；TB-01-C、TB-05/07 的分页/背压/Gap、恢复与跨进程全链也未验收。当前正常服务重启和读取损坏恢复不代替这些试验。本机没有重复全历史 cargo/replay；GitHub 的标准 CI、fixture-drift 仍独立运行。本片没有更改 formal 源。

本包名分为开放票 #1370 绑定的工作草稿与验收证据，不是新教义/架构正本。原始独评载荷保持字节；view 只转换阅读链接。阅读版 Markdown 链接的未归档目标见 [外部定位](acceptance/UNBUNDLED-REFERENCES.md)；其他未入包明细路径及哈希保留在各 payload JSON 的来源记录中，不以省略原始输出扩大结论。
