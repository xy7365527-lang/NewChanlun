> 链接转换阅读版；正文语义与原稿一致。签署原字节：[原稿](../../../../payload/sources/newchanlun-1339-architecture-20260908/evidence/CLASSIFICATION-OUTPUT-PROBE.md)，SHA256 `2d4d30541664199ead754267d9a66881a7643534238288e508534971ce397830`。当前批准状态见归档根APPROVAL.json；原稿中的待审状态保留为发生时记录。

# #1339 三个新增细分类轴：Classification 生产与导出核查

本报告仅覆盖 `TrendSixState` 六态、`CenterStates` 发展三态及其核心分离域三歧、`BuySellPredicate` 六谓词 64 签名三个家族。是静态源码核查，不是所有完全分类叶子的生产验收；未执行 Lean、Rust/Python 测试、行情回放、服务或交易。

快照：`/Users/silencehan/Projects/NewChanlun`，HEAD `6df8d1921c72c84da90987f8f6c02ebc11fe6aac`，2026-09-08。**工作区有已修改及未跟踪文件，本报告依据 dirty 工作区实际字节，不宣称干净 HEAD。** 核查末尾再次读取 SHA 未变；本次所读三份 Lean 与 `rust/src/theta_v0/` 的 `git diff --name-only` 为空，但这不把全工作区转为干净快照。配套 JSON 保存引用文件 SHA256。

发现方法：先用代码图 MCP；有效 project 是 `Users-silencehan-Projects-NewChanlun` / `Users-silencehan-Projects-NewChanlun-rust`。返回未附索引时间或版本。图中 `Classification` 节点仍定位 `classifier/mod.rs:217`，snippet 为 `source not available`，而当前 `mod.rs:190-194` 已重导出 `pipeline`，故图只作导航，字段、调用点以当前文件核对。图还把模块内测试列为 `include_tests=false` 的 callers，故再次以 `#[cfg(test)]` 边界核验。图的零命中不作为不存在证明。未打开、引用或采用旧 #1321/#1322/#1325/#1326 原件或方案原稿；首次宽查询偶然出现旧报告标题元数据，未据此展开。

## 结论矩阵

| 家族 | Rust 计算 | Classification 承载 | 本次核对的对外出口 |
|---|---|---|---|
| 六态 `⊥/I/U⁰/U¹/D⁰/D¹` | `rlevel_of` 实现存在；当前 `rust/src` 对该符号、包装函数和载体的引用仅在 `level_state.rs`、`six_state.rs` 两文件，生产分类管线未见调用 | 实际 `pipeline::LevelState` 没有 `RLevel`、`PointState` 或等价六态字段；头注“R6 态”不能代替字段 | `PyThetaStream` 不导出它；所核 JSON 候选出口亦无六态字段。断点在 Classification 构造前，不能称“已算出，只被 Python 丢掉” |
| 中枢发展：延伸/新生/扩展；核心分离域上涨/下跌/扩展 | `classify_relation` 四态确实在 `decompose_resume` 被调用 | 四态折入 `MoveBlock` 的 `(kind,dir,level_lift)` 与中枢 span。没有单独的 `CenterDevelopment` 字段，但区分未在完整 Classification 内消失 | 流内 `newly_confirmed_step` 明确清空 moves/centers；`PyThetaStream` 的公开返回值不出发展态。下游只拿这份信号差分时不能恢复该轴 |
| 六谓词/64 签名 | `endpoint_to_bsp` 在一、二、三类生产路径真正置六位；`BspBits::class_index` 真有 0..63 编码 | `levels[ℓ].bsp[*].bits` 完整保留逐条六位；一三类点和二类点直接 append + sort，没有在此装成唯一的每级 `LevelView → SignalVector` | Rust 信号差分保留逐点 bits，Γ 候选 JSONL 的 `bsp_bits_class_index` 保留六位；`PyThetaStream` 公开结果不出该签名。不能把“逐点 bits 已保留”写成“每级每时刻 64 签名全分类已贯通” |

## 1. TrendSixState 六态

Lean 的对象和适用域：

- `formal/Origin/TrendSixState.lean:82-103` 定义六构造子和 `SixStateContext(lastCenter,price,b3,s3)`；中枢可以为 `none`，非空中枢用 `Center` 的良构核心。
- `:115-124` 的 `classifySixState` 为全函数：无中枢为 bot；有中枢按位置三态分流；above 由 b3 二分、below 由 s3 二分，within 不拆。
- `:134-174` 证明给定上下文后的穷尽和唯一标签。`CenterStates.lean:64-80` 规定闭核心 `[ZD,ZG]` 与位置判定。**这些证明的输入已经包含最后中枢和 b3/s3，并不构造其生产供给，也不证明市场可达性。**

Rust 定义式对应存在：`rust/src/theta_v0/classifier/level_state.rs:32-58` 为 `RLevel/RContext`；`:67-88` 为 `rlevel_of`；`:96-102` 为 `PointState{source_index,position,signal}`。`six_state.rs:85-91` 只是薄包装。源码边界 `level_state.rs:105`、`six_state.rs:103` 之后为各自测试模块。

本次从图入站查询到当前 `rust/src` 全部 `rlevel_of|classify_six_state|RLevel|RContext|PointState` 引用核对，仅这两文件命中；`rlevel_of` 的包装调用在 `six_state.rs:86`，其余实际调用位于上述测试边界内。没有把“枚举被声明为 pub”或测试调用计算为生产调用。

真正生产输出是 `pipeline.rs:47-89` 的 `LevelState` 与 `:116-119` 的 `Classification{levels}`。字段是 moves、centers、cp_ownership、bsp、pan_div、first_class_grades、level_projection；装配点 `:1511-1519` 完全对应这些字段，没有六态。`pipeline.rs:45` 虽写“R6 态”，当前结构并未承载它。其 `level_projection` 在 `:1501-1509` 只从 bsp/fractals/merged_bars 构造，本次没有把它推定为六态实现。

断点：**六态纯函数/对照层 → 生产输入构造及 Classification 装配**尚未在此核查找到连接。就该实际结构和入口而言，不能说六态已经由 Classification 计算/承载/输出。也不将这个受限符号核查延伸成“全仓不存在任何语义等价六态函数”。

## 2. CenterStates 发展三态与核心分离域三歧

Lean 域必须分清：

- `formal/Origin/CenterStates.lean:168-174` 的 `CenterWithOuter` 自带 `dd ≤ zd ≤ zg ≤ gg`。`:215-230` 定义外缘上涨、下跌和“核心分离且外缘重叠”的扩展。
- `:237-247` 的 `center_theorem_two_total` **要求 `SeparatedNewPair`**，只覆盖核心已分离的前后中枢对；互斥证明为 `:267-302`。不能把核心重叠也归入该三歧。
- `:314-341` 定义 `CenterDevelopment` 和 `classifyDevelopment`：核心重叠=extension；核心分离且外缘分离=newBirth；核心分离且外缘重叠=expansion。`:347-380` 的发展三态穷尽/互斥覆盖任意两份良构 `CenterWithOuter`，这里不另要求核心分离。

Rust 的计算载体是 `center.rs:105-114` 的四态 `CenterRelation`：UpContinuation、DownContinuation、LevelExpansion、CoreOverlap。`:312-322` 的实际分支先判外缘上/下分离，再判完整扩展公式，最后 CoreOverlap。对于满足 Lean 外缘包含核心不变量的中心对，这是发展三态中新生支按方向再细分，也正好在核心分离域给出上涨/下跌/扩展。**Rust `Center` 是公开整数域字段（`types.rs:125-136`），没有类型携带的 Lean 不变量证明；不能把任意非法手造 Rust Center 也计入形式化有效域。** 本次不重审中枢构造端点教义。

实际生产链：

`classify → classify_incremental → classify_incremental_inner`（`pipeline.rs:412-414,478-485`）→ `decompose_resume(&lc.centers,...)`（`:1251-1255`）→ `classify_relation`（`decompose.rs:140-152`）→ `fold_rel`（`:71-94`）→ `moves` → `LevelState`（`pipeline.rs:1511-1519`）→ `Classification { levels }`（`:1650`）。

`fold_rel` 的四路映射是：

| CenterRelation | MoveBlock.kind | dir | level_lift |
|---|---|---|---|
| UpContinuation | Trend | Up | 0 |
| DownContinuation | Trend | Down | 0 |
| LevelExpansion | Consolidation | None | 1 |
| CoreOverlap | Consolidation | None | 0 |

依据 `decompose.rs:72-76`。`:78-85` 只将相同三元组的连续关系合并为 run，块保留 `start_center/end_center`（`:47-62`）。因此**非单点块**内部每条中枢关系仍可由 span 和三元组恢复；`decompose.rs:154-163` 的单中枢 Consolidation 块没有中枢对，不应虚构为已发生的延伸关系。原始 `centers` 也在完整 Classification 内保留。

在下游信号差分 `stream.rs:298-325`，moves、centers 被明确清空（`:310-311`）。这份差分输出保留 BSP，不承载上述关系。此处是已找到的具体丢失点；不重做扩展升级链或其他消费链审计。

## 3. BuySellPredicate 六谓词与 64 签名

Lean 域与证明强度：

- `formal/Origin/BuySellPredicate.lean:82-97` 的 `LevelView` 为 cand1/cand2/cand3 各一个 `Option BspEndpoint`，注释明确这里**不构造市场状态投影函数**，证明以给定投影为前提。
- `:144-180` 的 B/S 调用三类厚谓词，按候选方向置 Bool；`:207-208` 还证明**同一类别**买/卖不可同时为真。这不是六位在生产域无条件独立的证明。
- `:231-260` 定义六位 `SignalVector`、`signalVector(v)` 和 64 向量全枚举；`:267-315` 证明枚举长度/无重复/穷尽和每个给定 v 的唯一签名。该结论是编码空间 64 态及单值性，**不等于生产投影能达到所有 64 态**。

Rust 六位真 producer 与字段：

- `types.rs:201-213`：`BspBits{buy1,buy2,buy3,sell1,sell2,sell3,...}`；`:291-314` 是六位到 u8 及逆编码。附带 `third_class_entry` 不进六位编码。
- `bsp.rs:80-112`：`is_first/is_second/is_third` 各判，并在 `endpoint_to_bsp` 按买/卖侧独立置对应位。它的输入是 `EndpointSituation`，并非 Lean 的每类独立候选 `LevelView`。
- 一类生产路径：`scan.rs:960-977` → `signal::judge_first_from_gates` → `signal/gates.rs:402-429` 构造 EndpointSituation、调用 endpoint_to_bsp、构造 BspPoint。
- 三类：`scan.rs:1000-1019` → `signal::judge_third_cert`；`signal.rs:303-338` 真判离开/回试价格并调用 endpoint_to_bsp；点随后 push。
- 二类：`pipeline.rs:1414-1427` → `extract_second_resume` → `second_for_parent`（`:1796-1804`）→ 双侧 `signal::extract_second_signals`（`:1728-1754`）→ `signal.rs:745-779` 构造端点和六位。
- `BspPoint.bits` 定义在 `bsp.rs:139-143`；pipeline L0/Ln 的 merged_scan 调用在 `pipeline.rs:1336-1411`，二类与前者经 `:1433-1445` append、sort、Rc 缓存，并在 `:1515` 进入 `LevelState.bsp`。

关键边界：上述生产路径分别产出条目。`pipeline.rs:1433-1436` **只拼接排序，不按 source_index OR 合成**；同一级同坐标出现二/三类时，可以是两条 BspPoint。`stream.rs:318` 又按 `(level,source_index,六位)` 去重，保留这种多条身份。这证明逐条信号没有被压成互斥单类，但并未建立“每个市场状态 x、每级 ℓ 恰好一个 `LevelView/SignalVector`”的生产映射。静态核查也不替代厚判据与 Lean 的逐叶同输入同输出验证。

## 对外接口的实际边界

### Python：本次核验 PyThetaStream 直接出口

注册于 `rust/src/lib.rs:2822-2823`；`theta_v0/ffi.rs:94-111` 返回目标净仓位，`:117-125` 返回订单诊断及四元快照，`:132-144` 返回声部腿，`:157-251` 返回收尾账本字典。该公开方法集没有 Classification、六态、中枢发展态或逐点六位签名的返回值。`finish_full` 仅静态阅读，**未调用**。

所以这三个轴都不能经本次核验的 PyThetaStream API 原样读出。这个结论仅限定这一直接出口，不宣称所有 Python 接口和历史引擎均无相关状态。

### JSONL：本次核验 Γ 候选诊断出口

`strategy/interp.rs:977-986` 从 `Classification.levels[*].bsp` 为 Candidate 逐条复制 `bits: point.bits`，另外算主类 `bsp_class`。`backtest/gamma_dump.rs:157-180` 的 JSONL **同时写** `bsp_bits_class_index`（完整六位）和 `bsp_class`（主类）；不得把后者当完整签名。`:64-79` 为环境变量启用的诊断落盘，`backtest/fill.rs:5645-5665` 确有对 `write_step` 的条件调用，不是孤立 writer。

该 JSON 候选行保留逐候选六位，未输出六态或中枢发展态；它不是 Classification 全量序列化，也不补上每级每时刻单一签名。`Classification/LevelState/BspBits` 当前 derive 未声明 Serialize/Deserialize（`pipeline.rs:46,115`、`types.rs:200`）；本报告不由此推断全仓不存在手工序列化实现。

另只读 `trading/tape.rs:27-65` 的 SignalTape/BarSig 字段，可见它并非这三个轴的完整同构载体；**未追其 producer 或 Python marshal，不能把该结构检视当成完整 SignalTape 对拍结论。**

## 未覆盖与验收限制

- 未重做走势粗类、小转大、扩展/UI 断边代表链；中枢关系只追到此次新增轴所必需的计算、字段和一个具体清空点。
- 未审计所有 exporter、Python 层或历史引擎；出口判断限定 PyThetaStream 与 Γ JSONL，SignalTape 仅结构观察。
- 未运行测试或编译，不声明本次已重新 machine-check Lean、证明 Rust/Lean 全叶一致、运行时命中六态/全部 64 态、用户界面可消费三轴。
- 不提出教义新裁定、实现方案、默认批准或合入动作；本件只为 #1339 提供可逐行核实的事实。
