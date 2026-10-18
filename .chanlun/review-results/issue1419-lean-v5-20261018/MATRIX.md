# 逐发现与11文件矩阵

工作草稿，绑定 #1419。依据管理目录 `LEAN-V4-INDEPENDENT-REVIEW.json` 的完整 result.findings 与 validation；退出0是评审进程退出，原结论仍为 CHANGES_REQUIRED。

| 独审发现 | v5处理 | 验收边界 |
|---|---|---|
| P2：投影列表误称完整原始前缀，反向尾笔2→12漏 kill | 修正旧定义说明；新增真实 Stroke→投影/原始越P映射与 rawTurnWindowState；保留原反例、镜像、等号、确认正例，补开放/终态追加/条件投影定理 | 已落源码候选、数学检查通过；Lean类型 NOT_RUN。调用者是否传入完整前缀及 #1493 Rust接线未证明 |
| D_FIRST_RUNTIME_BINDING：旧严格分型、先试后合并 | 保留。a=[0,8], b=[2,10] 下 c=[3,9] 旧分型真但新窗OPEN；c=[1,10] 旧分型假但新窗确认 | 归 #1493，当前 PREPARATION_ONLY；没有修改Rust或宣称修复 |
| E_REFERENCE_FIXTURE_BINDING：参考外缘/续扫及旧fixture | 保留原独审反例与未完成清单，不动参考实现或fixture | 仍需修复真实参考消费者并实际导出；禁止手填expected补绿 |
| D_SECOND_FOLD_REFINEMENT：固定step/优先序/预算出口已存在，不能以接口未定延期 | 新增具体折叠与来源映射、53个新增具名定理证明体（两文件合计）；剩余具名全字段/机器边界/恢复义务见 RUST-MAPPING | 不再停在静态三态；但不是完整Rust精化、K1恢复或实际运行证明 |

## 允许写集逐项处理

| 路径（formal/下） | v5动作 | 来源/理由 |
|---|---|---|
| Formal/DivergenceNesting.lean | 原字节保留 | C未变；复用v4固定源静态证据 |
| Formal/RecursiveConstruction.lean | 原字节保留 | 全subs区间不变；不重跑C模型 |
| Origin/CenterConstruct.lean | 原字节保留 | E的Zn/续扫不变；参考fixture差异保留 |
| Origin/CenterConstruction.lean | 原字节保留 | E的seed/core判据不变 |
| Origin/CenterPairFixtureExport.lean | 原字节保留 | F8例未改；实际导出仍NOT_RUN |
| Origin/CenterStates.lean | 原字节保留 | F五叶/四态投影不变 |
| Origin/PromQual.lean | 原字节保留 | C位移证明不变 |
| Origin/ScanAssemblyMirror.lean | 原字节保留 | 仍只有v4域说明，没有新镜像实现 |
| Origin/SegmentFeatureSeq.lean | 修改 | P2域更正、原始笔前缀、具体SecondFold计算投影与证明体 |
| Origin/UnifiedScanMirrorBridge.lean | 原字节保留 | 仍不新增完整桥接资格 |
| Phase2/Claim10_SegmentV1.lean | 修改 | 复用Origin具体折叠，补既有Dir/Interval/Stroke和终态/前缀桥 |

11项来源均由固定634f donor→v4适配→112e基线继承。v5仅在D两文件做新增实装，不盲搬donor。`SOURCE-MANIFEST.json`逐项保存三侧字节和hash；`PRESERVATION.json`检查旧非注释代码未删改，声明更正不缩小原始任务义务。

## 独审已有验证的处置

原独审33路径、11源三侧hash/准备补丁、封包、C/E/F模型、13条引文的历史结果留在原报告。当前只按未变字节复用C/E/F部分；D新差异用本轮源码清单和模型核验。原153模块/52反向闭包是v4全11文件的审阅面；本轮两文件的新闭包为7模块，不能将两者计数混为运行覆盖。

Lean/lake/fixture漂移、Rust/产品、S3/Q、恢复、浏览器和本轮独立评审均未执行。原F独立来源/完整同树链缺项全部保留。
