# #1419 C/D/E/F Lean 同树候选 v4

工作草稿，活期绑定 #1419。**已在允许的 11 个 formal 路径落实定义与带证明体的定理，保留未提交差异供管理者收包；不是整票通过。** 本轮静态与有限数学模型检查通过。Lean 类型检查、导出求值、正式 fixture 漂移 gate、Rust/产品运行及独立评审全部 **NOT_RUN**。下列当前 Rust 差异与动态证明缺口仍须处理，不能将注释中的 donor 绑定或 donor 旧验收当成本树已通过。

基线 `545e9f75faec49be39de7799e0aaf3f6c8276de9`；分支 `codex/1419-lean-v4-20261018`；来源候选 `634f62cddfaddf656a35ca9b9db20cb716b62bff`。HEAD 和分支未变。身份是 Sandcastle 外部 Codex 工蜂，真实 session_id 由运行时登记；不是父任务的原生子代理。管理者 `/root/manage_1419_sandcastle`，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`。

## 已落实的行为与证明边界

| 义务 | 本树实装 | 当前 Rust 核对与边界 |
|---|---|---|
| C | `Move.interval/listInterval` 对全部 subs 递归取 min/max，空列表 `(0,0)`，非空不引入零；新增中枢/级别无关引理及正价、负价、嵌套见证。`PromQual` 以互递归证明 shift 保区间。`DivergenceNesting.OpenTail.currentExtreme` 改读真实走势区间，补正价下行见证。 | `descend.rs::RMove.lo/hi`、`recursive_tower.rs::LeveledMove.compose/envelope` 源码同值；同树执行未验。空子走势作为成员仍贡献自己的 `(0,0)`，不会被跳过。 |
| D 第一种 | 先破谓词、冻结 b 的转折点窗三态、包含成员跳过、首次出窗、终态后追加保持、无并列域与严格分型的 iff、完整无缺口前缀确认定义，以及同笔越过两端时新极值优先的证明体。 | **D_FIRST_RUNTIME_BINDING**：545e 第一种实现仍旧，见下节。新增定义落实 #1490，不宣称已接入 `SegmentEndUp/Down` 或全部 Lean/Rust 构造链。 |
| D 第二种 | `SecondKindTerminal` 三态及“终结 iff confirmed”的静态证明体；逐项记录 `second_fold` 同方向过滤、独立包含方向、确认优先、原始极值失效和预算出口。 | **D_SECOND_FOLD_REFINEMENT**：未伪造动态折叠/恢复定理。两个终结过程的同笔优先序不同，不互作兜底。 |
| E | seed 外缘 `computeGG/DD` 仅聚合同向 Zn，首段保证外缘包含核心；`segHL` 携方向，`extendWeak/refStep/refLoop` 传递方向。非 Zn 弱接触仍成为成员、核心冻结、外缘不变有具名引理。保留 #812 Z-5 C 续扫，4 组见证使用 `decide` 证明体。 | 与 `scan_center_facts` 的 **不足 9 成员窗口** 对齐：全成员弱接触定成员，Zn 定外缘，突破段跳过、失败 seed 滑窗、开放尾不越界。**E_REFERENCE_FIXTURE_BINDING**：旧 `ref_v1.rs` 与旧 center fixture 尚未跟进。 |
| F | 五叶分类、独立谓词 iff、恰一证明；新增四态类型与投影、四态谓词 iff/恰一证明，同时保留旧三态投影及旧公式等价。分支顺序适配当前 Rust：上新生→下新生→下扩展→上扩展→核心未分离。新增 8 例导出入口。 | `CenterWithOuter` 有序域 `DD≤ZD≤ZG≤GG`；S 域进一步要求 `ZD<ZG`。8 例均在 S 域内。核心相切不叫 CC-018 延伸，扩展关系不证明上级对象已构造。 |

`ScanAssemblyMirror` 和 `UnifiedScanMirrorBridge` 均只加域说明注释，**没有新增镜像实现、跨类型桥或运行证明**。`PairRelation` 的四态证明在 `CenterStates` 内；旧镜像不因此获得新的端到端精化定理。

11 项逐文件来源/适配与必要更改矩阵见 [MATRIX.md](MATRIX.md)。精确三侧哈希在 `SOURCE-MANIFEST.json`。`SOURCE-INDEX.json` 保留管理者准备件原字节，其中 `execution_authorized=false` 指准备阶段；本轮实施授权来自当前任务，不回改历史准备件。

## 当前差异与具名未验项

1. **D_FIRST_RUNTIME_BINDING**：`parser/segment.rs:160` 为 `segment-second/2`，但 `feature_seq.rs:520` 的 append 仍“先 push 试分型、失败后合并”，`:670` 仍调用 `is_fractal_and_gap`；`:234` 上段判据为严格 `b_h>a_h && b_h>c_h`。固定 a=[0,8]、b=[2,10]，c=[3,9] 时旧局部严格分型真、已裁窗口仍开放；c=[1,10] 时旧严格分型假、先破确认真。`D-FIRST-COUNTEREXAMPLES.json` 是这些局部公式的纯数学核验，未运行 Rust。还缺冻结 a/b、窗成员身份、首次出窗、作废后旧段延续与正式段端的 Rust/Lean 桥；不能只看 `/2` 名称就认为已接入 #1490。Rust 不在写集内。
2. **D_SECOND_FOLD_REFINEMENT**：`second_fold.rs:189` 的 step 先过滤与旧段反向的笔；同向笔经 `apply_inclusion` 后，`:267` 任一末三元素分型确认并 return，`:275` 才用合并前 reach 严格判越界。第一种窗口同时越过两端先 kill，第二种先查分型；不能合用一个转移定义。需要固定 `Stroke/FeatureElement`、成员下标、`dir_state`、P、包含后序列与 Lean Interval 的映射及逐步保持。`:301` finish 在预算先耗尽时 semantic_state=None/IncompleteResourceBound，不是 OPEN；K1 grant、未封存末笔、一步撤销、输入修订拒绝与恢复等价均未形式化。接口存在不等于 Lean 精化关系已固定，故只交静态三态，不添参数化空壳动态证明。
3. **E_REFERENCE_FIXTURE_BINDING**：生产 `recursive_tower.rs:535` 的 `scan_center_facts` 在 `:583` 取 Zn、`:647` 走 C 续扫；但参考 `ref_v1.rs:150` 仍全成员外缘，`:183` 附近仍 A 口径 `(j-2).max(end)`。`theta_v0_center_parity.json` 仅旧 3 组中心场景及 v0/v1 区间，缺本轮 4 组新场景与 `meta.break_up_scope/break_up_note`。`E-FIXTURE-GAP.json` 登记**源码可见的形状不一致**；正式漂移 gate 尚未运行，不能写成已通过或已实际报漂移。最终组合须修复参考消费者并由 Lean 真导出重落 fixture；不得手写 fixture 来补绿。
4. **F_EXPORT_EXECUTION / F_INDEPENDENT_ORIGIN**：8 例名称、A/B 的 8 个端点、五叶及旧三态投影已静态核对当前 JSON 一致。没有实际 Lean stdout；文件字节未宣称等于新导出字节。`oracle_pairs.py` 及 S0 `issue1419-tb02e-20261013/worked-examples.json` 在基线和固定 donor 的 tracked 路径中均未找到（`MISSING-SOURCES.json`）。候选数值的可复核来源是固定 donor 文件和现有 fixture，独立生成过程仍缺。16 格比较矩阵可由输入核算，本轮记录了 8×16 格数学值；8 例导出 schema 本身没有矩阵/四态字段，未声称新增这两种导出或运行 Rust 矩阵对拍。
5. **LEAN_TYPECHECK / FINAL_SAME_TREE_PARITY**：所有新增证明体、互递归终止 elaboration、`decide` 归约和受影响消费者须在获准的统一远端 source 上实际类型检查。`DEPENDENCIES.json` 按 153 个 `.lean` 文件的 import 静态枚举反向闭包；不是编译覆盖声明。`SubLevelDescent/RMoveCompose` 直接 unfold 的已查数值见证使用 segment，未发现需要越出写集修改的源码证据；闭包仍需类型检查。当前 Rust 若干注释仍说“当前 Lean 是旧定义”，因写集限制保留，管理者应在最终组合绑定时同步，不得继续以 donor 注释代替执行证据。

≥9 升级重切、上级构造、趋势实现、两段式破坏/R5、完整市场/历史及正式 S3/Q 非空流、修订双跑与恢复验收均未消费。预算 813e 的 FAIL 未被改变；K1 持费许可链未处理。没有修改 Rust、lake 依赖、资源帽或取消工位。

## 假设与有效域

- C 的价格类型是 Lean `Int`；Rust 映射域为可表示的 Tick。仅 min/max 不产生数值溢出。`Move` 类型自身不要求 ≥3 subs、非空 centers 或级别递减；这些是既有 `WellFormed`/调用方义务，未被本轮区间引理证明。`DivergenceNesting` 的 pending/settled_sound 等既有候选契约照旧；未从几何值推出真实背驰。
- D 的 `FeatureElem.valid` 只保证 low≤high；时间顺序、与 b 同向笔、a/b 来源和段端门来自调用方。无并列 iff 的 `¬Contains b c` 与端点不等前提保留；完整确认显式要求 a/b 无缺口。原 `BoundaryNoInclusion` 几何谓词与“跨边界不作合并”的实际映射未在本轮重裁。第二种静态枚举未证明某真实输入一定到达哪一叶。
- E 的方向是输入字段，按 seed 首段相等判 Zn；不拿奇偶代替方向。`centerFromThree` 保留 `computeZD<computeZG` 前提。`refStep/extendWeak` 是公开燃料函数；完整扫描从 `n=comps.length`、fuel=n 入口进入，不把任意 n/fuel 的调用当生产同拍域。seed 三段、全成员边界、Zn 外缘、连接与尾部各管其字段；走势区间仍取全 subs。未结算 reference `breakUp=true` 是既有占位，不当事实；对拍语义只比 settled 时该值。
- F 的 `CenterWithOuter` 携 core.valid、outer_lo、outer_hi 六个双中心不变量，是五叶 iff/唯一性证明的前提。S 的严格核心域是它的子域，≥9 子窗可能不满足外缘包核，不外推。四态/三态仅是五叶投影；旧三态 extension 标签无新的延伸语义。
- 导入链 `ChanlunElements→CompleteClassification→SourceAxioms` 以类型/定理前提承载源义务，已读源中没有新增公理。新增代码关键字扫描未引入 sorry/admit/axiom/unsafe；donor 新加的 4 个 native_decide 见证改用 decide，旧 15 处 native_decide 保留且不借其旧通过认证本树。未设置更宽资源限制。

## 来源和实际检查

管理者指定 XERJ 原件已核 SHA256 `39ac6dd99a589973ec0bb1489426b9f0aa638300c8b9a04a708e0622353c6a1a`：HTTP 200，查询 `listInterval OR PairLeaf OR SecondKindTerminal OR FirstBreakConfirmsUp`，只有 **2 个 nc1419gate-docs 命中**；不是 Lean 专属检索结果。本轮未重装、重索引或 clone。代码图没有当前 formal 固定源索引，按管理者允许的已知 Git 路径 fallback。实际参考是固定 donor 和本地 Lean 4.31.0 源码（Int 次序、List.cons_append）；没有本 checkout 的 `.lake/packages/mathlib`，这些模块不需要引入 mathlib。

`INPUTS.json` 固定 22 份管理/裁定/Rust/fixture/工具链源文件身份。`REFERENCES.json` 保存新增/订正援引的 13 行原文、行号、文件 hash 与正文界；现场核了行首及行内注，不将 028:770 的“跳空”主语扩成所有突破段。Z-5 直接承接已裁 `.chanlun/definitions/zhongshu.md` §6.1，不重裁。

实际执行 `python3 .chanlun/review-results/issue1419-lean-v4-20261018/CHECKS.py`，exit=0，原 stdout/stderr 在 `CHECK-RUN-1.json`，子命令原输出在 `STATIC-COMMANDS.json`：

- 11 项基线/donor/patch 字节匹配，写集和空白检查通过；新文件也纳入哈希与静态扫描。
- C：1625 个有限树，列表递归模型、Rust 递归极值模型、缓存归约与级别平移取值一致。
- D：3375 组三元素×双方向，核受限 iff、首出窗、包含跳过、终态追加保持和同笔优先；两个旧严格判据分离例保留。
- E：4 组固定 donor 字面期待值由整数扫描核算一致，已登记旧 fixture 缺项。
- F：4900 个 `DD≤ZD≤ZG≤GG` 中心对五叶恰一、四态公式一致；8 例与当前 JSON 逐字段一致。

这些是**静态检查和 Python 数学模型**，不是执行 Lean 定义或 Rust 产品，也不是独立 oracle 生成器恢复。最后只订正说明注释；封包再核源码 hash/空白/写集，没有重复模型测试。

CLAUDE 的 `python3 scripts/check_fixture_drift.py` 会在冷树执行 `lake build`，随后执行 Lean 导出（已读脚本），与本轮明确禁令冲突，故 **NOT_RUN**，不是豁免为通过。后续待根明确授权远端同树检查与重新导出；本报告不批准该动作。

## 收包

`DELTA.patch` 含相对 545e 的全部 11 项 formal 差异（包括未跟踪的导出器）；`ADAPTATIONS.patch` 含相对固定 donor 的本轮适配。二者零上下文，避免旧报告 patch 空上下文行的尾空格噪声。`PACKAGE-COMMANDS.json` 保存补丁反向只读检查和最终写集核验的实际结果；反向可应用不代表类型检查。`SHA256SUMS` 覆盖 11 项 formal 和报告目录全部其他文件，不含它自身，封包方法见 `PACKAGE.json`。

未 claim/发 tracker 评论/提交/push/merge/切分支/改 Git 配置/开其他工蜂/删除工位；未读凭据；未运行 Lean、lake、cargo、rustc、产品、服务、真实库、Long 或历史执行。差异留当前独立 checkout，待管理者独审与统一源验收。
