# Stage47 直接输出桥：独立数学比较与机器复核

采用两根证明的组合，范围为 `Intent-v1.md` 的“直接路线的精确目标”三项。在给定有限事件、总定义簿状态和可空报价列表满足 `Through`、`Reads`、长度条件时，原 `MovingQuote.construct` 的输出具有原事件来源、完整且不重复的归属、符号块确认对应；其每个 some 几何具有原路径每切点的有效 L1 报价和精确区间包络；未经筛选的相邻输出共用切点与报价记录，两边都有几何时端价相同。此结论采用 `DirectOutput.exact_root` 与 `DirectOutput.actual_exact_root` 一起支撑。

v1 的 `HULL_TARGET` 单独写的是兼容 Flow 重放的支撑，没有显式写给定 `bs/qs` 的逐切点支撑。补充声明和证明确实补上该义务。未发现冻结声明的反例、额外未封闭叶子、循环前提或迫使成功几何域为空的条件。拒绝将此结果提升为最低级别完整构造、原义 a₁、F₂、真实 feed 完整性或实时物理钟结论。Intent 的“最低级别构造的决定性检验”不属于本工位的采用范围。

本报告由独立比较工位 `/root/direct_output_comparison` 形成，未继承作者会话；接收冻结声明、证明、原意图及冷读后作比较，因而它本身不是盲读。原冷读仅看声明与定义；补充冷读由同一原盲读员接续，只增加实际路径声明和定义，没有接收作者新 Proof 或 Intent。补充稿不是另一个全新代理的冷读。独立性依赖协调者保留的原生派发/完成记录，文本与哈希不能独自证明隔离。

本次只写本报告及专属仓外目录 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output-independent`。原 Intent、声明、Proof、formal、生产和教义文件均未改动。代码图工具返回本研究树未索引，随后直接读取任务点名的实际源文件。采用 `lean-verify`、`v2-verification.md` 和 `v2-verification-loop.md` 的执行、精确声明、公理闭包、语义范围分账口径。

## 冻结对象与冷读绑定

| 对象 | SHA-256 |
|---|---|
| `stage47/Intent-v1.md` | `44ac43b6f75426d026780e1d76d06adc7fbb8312556daeadd79f66a84365a926` |
| `DirectOutputSpec.lean` | `23d38c0d49b57d7438efd90757dc7a80e138d54400de123955280857164c8e6b` |
| `DirectOutputSourcePathSpec.lean` | `9cccbdf9b546c51aa3eaf323f282ea81a58032d3bb15c889c18efdc17bf9d648` |
| `DirectOutputProof.lean` | `a891306d2e94fe60e06acdde4bb1806f37e6f71460147f4fa095d8f77b187f2a` |
| `DirectOutputSourcePathProof.lean` | `531a46969a849d5e6641a7685c398404366a308b7135e9eed79b0f3c1e8cba23` |
| 原冷读 `direct-output-readback.md` | `54cfca5f7151889c1ae4dca219685173af18eac51ced9d710b946830c88c1cc7` |
| 补充冷读 `direct-actual-path-readback.md` | `7f91a02dd13f73d10a93c961104be3f308d0cddf02d3725f4eba4cb5f83dd666` |

两个 Spec 的哈希均与派发的冻结值相同。独立项目从 R 中复制 15 个研究 Lean 源文件，复制时逐一与作者 project 同名文件核对；未复制任何作者 olean，也未复制其 Probe。原冷读清单中的 19 份数学源码与 2 份工作流文件、补充冷读清单中的 20 份数学源码全部重算一致。完成前再次核对 15 份研究源码未漂移。详细记录在 [independent-input-snapshot.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output-independent/independent-input-snapshot.json) 和 [final-input-binding-check.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output-independent/final-input-binding-check.json)。

## 意图与实际数学对象的比较

1. 来源与 None 保留符合第一项。`MovingQuoteWireSpec.lean:9` 的 construct 在长度匹配后映射全部 located 块，不过滤缺失几何。`DirectOutputSpec.lean:12` 的 Source 绑定原列表连续片段和起点长度，`SOURCE_TARGET` 同时绑定整个 located 列表、decode 回原事件列表和 Good。`DirectOutputProof.lean:39` 的来源归纳、`:76` 的来源根及 `:118` 的覆盖证明实际使用分组和位置不变量，没有把来源或覆盖结论放进输入假设。

2. COVER 的字面唯一性确为 Output 值唯一。列表成员量词不会区分两个相等条目的出现次数，不能仅凭 `COVER_TARGET` 把“值唯一”偷换为“位置唯一”。实际构造的每块至少一个事件，locate 累加的 start 严格增加，所以同值输出不能重复。本工位另在仓外证明 `IndependentChecks.output_nodup`，其完整类型为 `∀ es qs out, construct es qs = some out → out.Nodup`。结合 COVER 与所有块的正长度，足以支持原意图的每个原事件完整覆盖且不重用。该额外证明未修改冻结声明。

3. 符号块确认对应符合原有 Completed 的范围。Good 给 `start < finish ≤ n`、最大同号区间、firstKnown 和 knownAt；`completed_iff` 把 Completed 与 `knownAt = some (finish+1)` 接上。Completed 的确认来自位置 finish 的第一条反号事件，须观察到 finish+1 个事件。末块 `finish=n` 不 Completed；some 几何不意味着完成走势。这里的事件符号只看侧和动作，不是价格极值反转；Event 的 amount 与 price 不参与符号判定。

4. v1 的端点、同号 Flow 和兼容支撑包络成立。`DirectOutputProof.lean:182` 的 `hull_contract` 从实际原块窗口提取全部 some 报价，用 Reads 对齐原 bs，再由 Through 与来源切片形成 `flow_indexed`。首尾状态明确是 `bs start`、`bs finish`，事件明确是原块的事件。`DirectHull.Support` 的中间状态仍是合法兼容重放的存在见证，不能直接把其定义读成“本次输入每个 bs k”。原冷读对这个区别的提示准确。

5. 补充根补齐实际每切点的义务。`DirectOutputSourcePathSpec.lean:7` 的 ActualSupport 只使用给定 bs、qs、起止切点和整数价格，既没有候选 Segment，也没有 Flow 或目标包络作参数。`:12` 的目标把“每个 start≤k≤finish 有该 qs 的正确报价”与“实际支撑的 ExactHull”分别列为结论。`DirectOutputSourcePathProof.lean:7` 从 readout 的 all quoteReady 得出每个原窗口索引有 some 报价，再用 Reads 得到 QuoteOf。`:77` 从 Through 的原事件 Step、Source 的原索引和 Uniform 推出相邻实际报价同方向弱单调，`:90` 归纳传递到任意两个原切点。`:92` 对任意 ActualSupport 见证以其原索引取得上下界，`:102` 至 `:109` 用实际首尾报价取得极值。`:110` 后回到原 leg 的 segLow/segHigh。这里没有用兼容 Flow 的 mid 来替换实际 bs k，也没有把包络结论藏在假设中。

6. 不必额外证明两个支撑集合相等才能完成当前意图。补充根直接证明实际支撑具有同一原 leg 的精确包络，v1 根另证明兼容 Flow 支撑的包络，已覆盖合同。两份声明都未给出 `ActualSupport = DirectHull.Support`，本报告不把集合等式加入采用结论。

7. 原始邻接符合第三项。`ADJACENT_TARGET` 使用 `out = pre ++ x::y::post`，未过滤 None。`adjacent_contract` 先把该原始列表分解映射回 locate，得到共用切点和反号关系，再由两个 some 几何在公共索引读取到同一报价证明端价相等。共用查询可以是 `some none`，此时端价子句没有两个 some 几何实例。删除 None 后的新邻接不在结论内。

两份冷读与以上比较相符。补充稿对“每切点有效”与“只有极值取得”的区分尤其必要，因为 ExactHull 本身不保证每个切点存在报价。补充目标用第一合取项明确关闭了这个缺口。

## 非空、退化与条件边界

独立检查源码为 [IndependentChecks.lean](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output-independent/checks/IndependentChecks.lean)，成功日志为 [attempt2.stdout.log](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output-independent/checks/attempt2.stdout.log)。本工位用 Lean 检查以下实例和性质，而非把模型域非空当作口头假设。

- `demo_nonvacuous` 取买侧新增 90、卖侧新增 110 两事件，账本依次为 `([100],[102])`、`([90,100],[102])`、`([90,100],[110,102])`，每次报价均为 `(100,102)`。Through、Reads、长度与实际 construct 同时成立，恰有两个 some 几何，knownAt 为 `[some 2,none]`；第一块 Completed，末块不 Completed。`exact_checks` 还把实际路径根专门应用于这个有实例的域。两个几何分别为 100→102 与 102→100，公共端价为 102。
- `none_retained` 取已有 BookQuote 的三次买侧删除历史。该历史 Through/Reads 有效，原 construct 保留一个 geometry 为 none 的输出，原事件全部 decode 回来。故 None 保留是有意义的约束，并不依赖“成功几何无处出现”的整体空真。
- 空事件列表可产生空输出；非空历史也可所有几何皆 none。此时逐 some 几何和相邻端价部分可空真。这是合同允许的条件域，不是无条件产生几何的保证。对 some 几何，ExactHull 要求两极值都在支撑中，不能由空支撑取巧。Block.first 又保证其原事件区间不是零长。

构造器自身只检长度，定理承担条件证明。具体边界例是“买侧 add、amount=0、price=90”配两份 `(100,102)` 报价：construct 可给出 some 几何，但 Step 的 `0<amount` 不可能成立，因此不存在满足 Through 的簿路径。原构造没有检测这个违例，HULL/ACTUAL_PATH 的前提把它排除。这不破坏当前条件合同，也不能被报告成输入验证器已实现。

None 在有 Reads 前提时严格表示模型内不存在有效双边报价，不能用来填捕获未知。Step 的库存是整数列表，cancel 和 execute 共用删除关系，要求连续的重复价格片段；没有订单 ID、真实撮合、成交必须发生在最佳价或时间优先级。所有价格为 Int，索引为 Nat。独立 pp.all 输出核对了实际加载的 Segment、Tick、Index、segLow、segHigh 和展开后的 ACTUAL_PATH_TARGET，与冷读的载体解释相符。

本例新增价格 90 和 110 都在 L1 包络 `[100,102]` 外，清楚表明此包络不覆盖全深度事件价格。ExactHull 表示取得到的最小/最大整数界及全支撑受界约束；没有声称支撑填满区间内每个整数。它也没有连续时间插值、真实捕获完整性、实时物理钟、原义 a₁、固定 F₂ 或递归资格结论。研究程序叫 construct 并不意味着 Rust 生产实现已被验证或修改。

## 独立机器结果

本次在全新的仓外 project 中，使用本机 Lean 4.31.0、Lake 5.0.0-src+68218e8、direct 模式和已冻结合同分别 fresh 编译。运行时 Lean commit 为 `68218e876d2a38b1985b8590fff244a83c321783`。两个不可变 run 都保留了完整编译、提取、导入解析和 stdout/stderr；作者的 receipt 未覆盖，也未作为通过依据。

| 独立根 | durable job ID | verifier run ID | 结果 |
|---|---|---|---|
| `DirectOutput.exact_root` | `stage47-independent-v1` | `8bacfe6d172a4f28898787cdb5e53468` | machine passed；exact root passed；closed |
| `DirectOutput.actual_exact_root` | `stage47-independent-actual` | `61e9e4335666423aadcb04c9a3eae37a` | machine passed；exact root passed；closed |

v1 的实际类型是 `SOURCE_TARGET ∧ COVER_TARGET ∧ HULL_TARGET ∧ ADJACENT_TARGET`，补充根为 `ACTUAL_PATH_TARGET`。两次精确对照均为 definitionally_equal、binder_kinds_match、universe_parameters_match；根均无 universe 参数。传递公理都是 `[propext, Classical.choice, Quot.sound]`，unexpected axioms、unknown dependencies、unsafe dependencies 均为空，无 sorryAx。这里检查了根及 expected statement 的依赖闭包，不是只扫描目标文件有没有 sorry。

| 绑定 | v1 | 实际路径补充 |
|---|---|---|
| type SHA-256 | `ee8ceea86627208dca498bffda55baad8a0ac964cddc184ef9807d6c64680df8` | `9bda3c4d537964860d3c585b8b1e677535ae3ef7afef41e6508235a884c55d80` |
| semantic SHA-256 | `a64c15a8f4d26d0c08fdb1595ec656b835fe4d2e3aa239312bcc1920a8eef5a1` | `67c2f30066c85bdcc74c04fc0fae51376874b0bb29dfeb9353e8fe1641f7939b` |
| environment SHA-256 | `c3c0dcd844b466ad29d9cb85151869b52e8948f1302b7a750ddec341400d2b17` | `95caf99a8e563504cc0ae5999db44b8ac628f37aecfa9d605084d251db3b3723` |
| 本次 fresh 编译 | 13 个研究模块 + 提取 Probe | 15 个研究模块 + 提取 Probe |
| 实际加载模块清单 | 2279 项 | 2281 项 |

两根的 type/semantic SHA 与作者冻结回执一致；独立工作目录及新产物导致环境身份另行记录，不沿用作者环境身份。machine manifest 中 `semantic.status=not_reviewed` 是因为没有把本报告预灌入编译器；本文件提供独立的语义采用结论，不能把编译器状态冒称自动语义审阅。

[根一 manifest](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output-independent/independent-v1/run-manifest.json) 的文件 SHA-256 为 `ad321c24beb16ca6fe2ec07a3ba20a136e92246937fbf8b384999eb3cfb4426b`；[根二 manifest](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output-independent/independent-actual/run-manifest.json) 为 `3c3ea92cefd3df331ac8fb216f6f55022dc19bc05fe78d3d96d4049bf58488de`。简明提取保存在 [machine-summary.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output-independent/machine-summary.json)。重新消费两份回执时，在原记录 `LEAN_PATH` 下复核当前模块解析，均返回 `current`、`snapshot_current=true`、`exact_root_passed=true`、空 reasons。独立检查的 `output_nodup`、`demo_nonvacuous`、`none_retained`、`exact_checks` 均编译成功，打印公理仅含允许基础公理；这些是辅助声明，未替代两根精确验证。

formal 部分只复用了预编译 olean，没有声称 fresh 编译 formal。两次实际加载的 7 个 Origin 模块为 CenterComplete、CenterConstruction、CenterFull、CenterStates、ChanlunElements、CompleteClassification、SourceAxioms；各自当前源码 SHA 与实际加载 olean 路径/SHA 已成对写入 [formal-source-olean-bindings.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output-independent/formal-source-olean-bindings.json)。该记录绑定“当前读取源码和所加载预编译产物”，不独立证明这些 olean 是由当前源码新生成的。实际类型与关键定义经 pp.all 另核；其余信任边界仍是本机 Lean、预编译 formal/标准库和校验器。未运行另一种 kernel，未触动 formal，故本工位没有 fixture 再生成或漂移门执行义务。

复现入口是专属目录内 `./verify-v1.sh` 和 `./verify-actual.sh`，两个 durable job 的输入、前后哈希与完整日志保存在 `.research-state/jobs/`。初次尝试给 local-job 的 `--input` 传目录被工具拒绝，未派发；改成逐文件输入后成功。辅助检查第一次仅因 Lean API 名称和证明展开写法失败，失败日志保留，修正只发生在本工位的 IndependentChecks.lean。回执消费第一次未带原 LEAN_PATH，被准确判为 `runtime_search_path` 不匹配；在记录环境复核后通过，前后日志均保留。这些工具与辅助检查修正没有改动冻结数学目标。

采用范围至此关闭。当前没有直接输出来源桥的未闭合数学义务；最低级别构造、原义资格、F₂、数据捕获、真实执行及生产采用仍须各自证据，不能由本报告代领。
