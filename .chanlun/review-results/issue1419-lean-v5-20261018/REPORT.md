# #1419 formal v5：原始前缀与固定第二种折叠

工作草稿，活期绑定 #1419。**已实际修改两份 formal 源码，修正 P2 的投影域误述，并补原始笔前缀分类和固定 Rust 第二种折叠的具体计算投影。** 不是整票通过，也不是完整 Rust 精化已完成。所有 Lean 类型检查、导出、Rust 执行及同树业务验收均为 **NOT_RUN**；本轮独立评审待管理者冻结后安排。

固定基线 `112e1c3ca49da3c5ff21d0b6fa6a8bcd3abcd7b1`，tree `e19a4cfcea3db6964ab435c466372d879e30105d`，分支 `codex/1419-lean-v5-20261018`。HEAD、分支未变，差异未提交。管理者 `/root/manage_1419_sandcastle`；父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`。身份为 Sandcastle 外部 Codex 工蜂，真实 session_id 由运行时登记，未声称是父任务原生子代理。

## P2 的实际修复

`Origin/SegmentFeatureSeq.lean` 原有 `turnWindowStateUp/Down`、`FirstKindConfirmsUp/Down` 的函数体和旧定理保持；说明改为**给定同向投影列表的分类**。投影列表 OPEN 只说明列表内元素均在 b 内，不说明被过滤的原始笔安全。

新增 `projectStrokes` 从真实 `Stroke.direction/startPrice/endPrice` 得到方向投影；`rawCrossesP` 计算每根原始笔的 P 侧极值；`rawTurnObservation/rawTurnWindowState` 对 b 之后全部已知原始笔按序分类。所有笔都能以严格越 P 触发 kill；只有与 b 同向的笔能确认。与 b 同向笔复用旧窗口的单元素判断，不另设较宽判据。b 不被窗成员合并改写。

反例锁 `rawTurnWindowState_reverse_tail_counterexample` 保留 a=[0,8]、b=10→2、反向尾笔2→12：方向投影为空而 OPEN，完整给定原始前缀 KILLED。相等 P、反向尾笔不参与确认、对偶、连续交替的确认正例也有证明体。数学模型另锁第一种同笔双越界先 kill，以及先 kill 后不可复活。

`RawTurnSafe` 和 `rawTurnWindowState_open_iff` 给出确切 OPEN 前件：每根原始笔不越 P，且每根 b 同向笔被 b 包含。`rawTurnWindowState_eq_projection` 证明**所有被过滤笔均不越 P**时可使用投影结果；这只是充分前件，不声称必要。`firstKindRawUp/Down_iff_projected` 将这一前件带入无缺口确认桥。终态追加保持、开放前缀接越界笔即 kill 都有证明体。

这里的“原始前缀”明确指调用者提供的 b 后完整 `List Stroke`。没有证明其来源覆盖行情/解析器的全部已知笔，没有构造 a/b、没有证明连续性或段端门，也没有把新定义接进旧 Rust 第一种状态机。**D_FIRST_RAW_SOURCE_BRIDGE / D_FIRST_RUNTIME_BINDING 继续保留**。对任意带方向字段但不连续的负控输入，OPEN 的准确陈述是 `RawTurnSafe`，不能泛称每个区间都在 b 内；连续交替且方向与价格一致的数学样本另外核了两者相同。

明确撤销 v4 `REPORT.md` 的“完整无缺口前缀确认定义”“完整确认显式要求 a/b 无缺口”在旧投影函数上的完整原始前缀读法。旧报告按任务要求保留原字节，由本报告更正其主张；没有删旧报告包装通过。

## 固定第二种接口的落码

`SecondFold` 命名空间复用 Origin 的 `Stroke`、`FeatureElem.ofStroke`、`Contains`、`mergeInclusion`、严格顶/底分型；不是接受任意 oracle 的参数化空壳。它逐项对应固定 `545e9f75…` 与当前 `112e1c3…` 的 Rust，三个输入源 `types.rs / feature_seq.rs / second_fold.rs` 在这两个提交及当前字节完全相同。

- `Candidate/Element/State` 携冻结 P、笔数组下标、原始元素、标准元素、成员下标、包含方向、水位、位置、终态和最小确认/kill 见证。原始和标准列表以逆序保存，head 对应 Rust Vec.last；每个元素的 members 仍按时间正序。
- `includeElem` 按实际包含分支更新末元素；None 与 Up 用高高合并，Down 用低低合并；合并保持方向与此前成员，非包含时按两端变化更新方向。
- `consumeFeature/consume/step/run` 读实际 Stroke，先数原始笔再过滤；同旧段方向才进入第二序列。先包含、再查末三元素任意分型、最后用**合并前**极值判 kill。与第一种同笔优先序不同。
- `finish/evaluate` 由 b+1 的实际后缀和具名预算产生结果；未决且预算不足时语义为空，完整覆盖仍未决才 OPEN。None 映射 Rust scan_window=0 的无限预算，Some n 是显式获批量；未将 Some 0 当成 Rust window=0。
- 证明体包括端点分支等值、合并成员/方向保持、末三元素判据、原始水位更新与界、全消费前缀水位折叠不变量、步进位置、过滤保持、实际原始投影、未决消费数、分批折叠、终态前缀吸收以及预算出口。`run_undecided_raw` 从输入构造 `indexedProjection`，未假设输入等于一个任意 oracle。
- `Phase2/Claim10_SegmentV1.lean` 显式导入上述定义，新增 Origin→既有 Dir/Interval/Stroke 映射、第二序列方向投影桥和三态输出桥。原三态枚举的证明本身仍是静态分类；新增前缀定理没有被拿来冒称 K1 恢复证明。

完整逐字段映射、局部定理和剩余具名义务见 [RUST-MAPPING.md](RUST-MAPPING.md)。新增声明逐项列于 [DECLARATIONS.md](DECLARATIONS.md) 和机器可读 `DECLARATIONS.json`，均附当前源码行号。**有证明体不代表已通过 Lean elaboration。**

## 仍保留的外部边界

- 第一种 Rust 仍使用严格分型与先试后合并。`D_FIRST_RUNTIME_BINDING` 归 #1493；依本轮管理者已核状态，其原生 #1405/#1420 仍 OPEN，只有 PREPARATION_ONLY。本树没有修改 Rust，没有宣称已修或批准其执行。
- `E_REFERENCE_FIXTURE_BINDING` 保留：旧 ref_v1/fixture 域未迁移，不能手改 expected 补绿。独审给出的 e2UpEndpoints 旧 DD=5/后中心起点7 与新 DD=10/起点9 的差异仍在；复用旧证据，不重跑 E 模型。
- C/E/F 及两个 mirror 的 9 份源码逐字不变，仅复用相同字节上的静态证据。F 独立生成来源缺项和实际导出义务仍在。没有重跑 161/1225 等旧独审模型，也未触碰 F5 raw。
- CF/W 组合合同、父账、预算 813e FAIL、K1 持费许可链、其他票工作树均未改变。没有从本次数学模型推出业务运行或父目标资格。

## 验证与封包

允许的定向检查结果在 `MODEL-RESULTS.json`、`STATIC-RESULTS.json`，实际调用 stdout/stderr/退出码在 `RUNS.json`，Git 子命令原输出在 `STATIC-COMMANDS.json`。

数学检查只用 Python 标准库中的整数与列表：2400 个第一种原始前缀（其中812满足全部被过滤笔安全的投影前件）、40个边界单笔观察、1904个第二种前缀、14230个预算格、12326个分批切点以及12个具名案例。第一种另以按端点时间推进的模型核对；第二种对比正序 Rust 控制流转写与逆序 Lean 计算投影转写的所有已建模字段。它们是有限数学检查，既不是 Lean/Rust 执行，也不是独立 oracle 生成器的恢复。

静查确认：仅两份 formal 改动；其余9份 formal 与全部旧报告不变；旧非注释代码逐行按原顺序保留；新增源码无 sorry/admit/axiom/unsafe；153模块静态 import 图无环，受影响闭包7模块。复用已有本机 Lean 4.31.0 源码中的 List/Int 定义，未运行该工具链。

9条相关 blog 原文经行号、逐字、正文界/行首注/行内注核对，原行和来源哈希在 `REFERENCES.json`。XERJ 本轮既有原件 SHA256 `39ac6dd99a589973ec0bb1489426b9f0aa638300c8b9a04a708e0622353c6a1a`，HTTP200、2个 nc1419gate-docs 命中；没有 Lean 专属命中，没有重新请求、安装、索引或 clone。固定 donor 634f 的11项字节身份与 v4 来源清单交叉核对，未借旧验收认证当前树。

**NOT_RUN**：Lean/lake、#eval/导出、fixture 漂移 gate、Rust/cargo/rustc、产品、服务、真实库、Long/历史、正式 S3/Q/浏览器、重放/恢复及完整同树链。CLAUDE 的 fixture 漂移脚本会启动 lake/Lean，故依本轮明确禁令不运行，不能当作通过。

`DELTA.patch` 为相对固定112e的两文件完整差异，已经 `git apply --reverse --check` 只读核验。`SOURCE-MANIFEST.json` 包含11项基线/donor/当前哈希；`RUST-IDENTITY.json` 固定实际 Rust 源；`INPUTS.json` 固定任务/独审/裁定/工具链来源；`SHA256SUMS` 覆盖11份获准 formal 文件及本报告目录全部文件（不含校验单自身）。补丁和总清单身份见 `PACKAGE.json`。

未 claim、发评论、commit、push、merge、切分支、改 Git 配置、启动其他工蜂或读取任何凭据。实际差异留给管理者收包、冻结和独审；本报告不批准后续运行、合入或关票。
