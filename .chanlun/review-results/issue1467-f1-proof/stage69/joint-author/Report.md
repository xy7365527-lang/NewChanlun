# Stage69：同一 JointCut 规则在既有 H2 上的检验

研究票 #1467，作者 A，2026-10-06。工作草稿，等待独立数学审查。产物仅在本目录；无新历史、R_W 重跑、参数变更、生产/formal/正本/tracker 修改。

**固定 S1 起点上没有非空候选分解。** 同一 Ω68-JointCut-v1 规则在 Stage64 H2 的 78 个语法窗口中给出 5 条候选边，但没有一条从 S1 出发。因此全部 583 前缀都只有空路径，移除 NoPP 仍不能提供三孩子。这个失败来自本 H2 的根窗口条件，不是沿用 Stage68 的 20 段长度证明。

图内存在一条 P/D/U 数值链，三交为 [50000,80000]，证齐 558；其起点是 S4，漏掉固定根之后的 S1…S3，即 E22…81。它可以展示规则的局部三孩子数值非空性，不能作为本次固定起点的成功。不能把前导根改到 S3，也不能把旧 LocalDyn64 从 S0 开始的 U/D/U 搬来补这个缺口。

## 输入身份与不变规则

唯一新检验输入是既有 `T64-H2-dynamic-repair`，583 观察、582 事件、144 笔、S0…S27 共 28 个所选已审 raw 段。H1 没有参与计算。三个原始文件的实际 SHA256 同时匹配 Stage64 作者 manifest 和独评 input-hashes：

| 原始 H2 文件，位于 `stage64/dynamic-author/v2/run/` | SHA256 |
|---|---|
| source.json | `ce7ba6ab38666b59b978dc942913e6a2b0c38c26409e9c14b70114a69b3260c6` |
| segments.json | `89cf6da6cd2174c13d5c3786cfd8cb28a059a125836c86abe5d8cf09fd9e0a3e` |
| reference.stdout | `7491058def0984a3b3ea5eb6b387f8b65b5484beb3cd203860e07c88afd66792` |

原始段身份、known_at 与 Stage64 独评 `H2-prefix-audit.json` 的全部证书相等。每段所用真实笔从该段 known_at 到 EOF 的稳定身份共作 7,700 次核对。没有从价格重新运行 R_W 或另选 raw 段。S28 保持待证，EOF 的未封尾保持未封。

WorkCard 在新候选计算前冻结，SHA256 为 `70894471ec53fc72fbc5fd38b74b995fb47699bb2552fab5bbbeb71c62b2dcef`。这记录作者过程声明和字节身份，不冒称外部预注册。唯一枚举变化是把可用 raw 上界 21 改为实际 `len(segments)=28`。固定前导 S0、从 S1 起步、4k+1 语法、Outer0=core、严格入出跨界、whole Extreme、有符号 L 和 raw 端点证均未改变。

窗口长度与数量为 5/23、9/19、13/15、17/11、21/7、25/3，共 78。`force`、`meet`、`f2_contract` 从旧脚本逐字保留。窗口计算块仅替换两个上界表达式，见 [rule-extraction.diff](rule-extraction.diff) 和 [rule-binding.json](run/rule-binding.json)。保留 W68/IC68 标识前缀表示同一规则族，不表示 H2 对象与旧历史对象身份相同。

H2 比 Stage68 输入长，但不是在旧历史后面接一段。两输入重叠的 443 个观察有 442 个价格不同；第一处就在 state0，旧价 75800、H2 价 40200。因此本轮不能把差异归因于长度单一变量。

## 单入口修复与旧结果控制

新入口在遍历每个节点时先输出当前路径，再遍历后继，所以直接产生空路径和所有边路径，不依赖旧 maximal 结果的补闭包。随后分别标记 maximal、非空 NoPP、三孩子及固定 F2 必要条件。原义完成保持 unknown，原义准入的 false 只表示未认证，不是原义命题被否定。

正式 H2 计算之前，新函数对 Stage68 已审冻结结果作了完整字段控制：21 个 raw 力度、40 个窗口、443 前缀的全部路径和路径分类全部相等。既有 32 项封存清单随后再次全部匹配，作者 FINAL/manifest 也匹配 Stage68 独评钉住的身份。旧脚本未导入或执行，旧冻结没有修改。[stage68-control.json](run/stage68-control.json) 与 [stage68-seal-audit.json](stage68-seal-audit.json) 保存两项不同核验。

`check_joint.py` 现在有 main guard、排他新输出目录、真实 `Path.cwd()` 和 `sys.argv` 记录。独立 Python 静态审查给有限域 Approve，没有必修代码问题；它没有重算数据依赖计数，不替代独立数学审查。

## 全窗口的拒绝层次与五条边

每个窗口的全部布尔条件及失败原因均列在 [all-windows.json](run/all-windows.json)。以下是逐层筛选，拒绝数互不重复；另附的重叠原因计数不能相加当窗口数。

| 层次 | 本层拒绝 | 剩余 |
|---|---:|---:|
| 4k+1 语法 | 0 | 78 |
| 正宽核、成员方向交替、类型与末臂一致 | 55 | 23 |
| 同向比较臂及严格入出跨界 | 16 | 7 |
| whole Extreme | 0 | 7 |
| 直接带号 L(c)<L(b_ref) | 2 | 5 |
| 从固定 S1 根开始接续 | 5 条边均不可到达 | 0 非空路径 |

最后两份力度失败窗口为 S9…S13、S18…S22。重叠失败项共涉及 exit_cross 59 次、whole_extreme 63 次、signed_force_weakening 64 次、coherent_type 40 次、entry_cross 31 次、strict_cores 31 次；方向交替和比较臂同向没有失败。[rejection-layers.json](run/rejection-layers.json) 完整列出各层被拒 ID。

| 候选边 | 类型 | Own | whole | b_ref/c | L(b)/L(c) | 发生/局部证齐 |
|---|---|---|---|---|---|---|
| 4→9，S4…S8 | P | E82…181 | [50000,80000] | S4/S8 | 200/50 | 181/198 |
| 9→18，S9…S17 | D | E182…361 | [38000,80000] | S13/S17 | 150/−300 | 361/378 |
| 13→18，S13…S17 | P | E262…361 | [38000,78800] | S13/S17 | 150/−300 | 361/378 |
| 18→27，S18…S26 | U | E362…541 | [38000,82000] | S22/S26 | 200/50 | 541/558 |
| 22→27，S22…S26 | P | E442…541 | [40800,82000] | S22/S26 | 200/50 | 541/558 |

核成员、各核 Own、core、实际首末笔速度与证齐时刻都保存在边记录中。S9…S17 与 S13…S17 共享后半材料，S18…S26 与 S22…S26 同样共享；两条重叠边不能同时作为相邻独立因子消费。

## 固定根的有限无解证

任意非空根路径必须先取一条从 raw index 1 出发的边。27 段预算只允许 k=1…6，所有首窗口如下：

| 首窗口 | 结构类型 | 全部失败项 | L(b)/L(c) | whole 先前极值/末价 | 局部证齐 |
|---|---|---|---|---|---:|
| S1…S5 | P | exit_cross、whole_extreme、signed_force_weakening | −100/−100 | min 48000 / 68000 | 138 |
| S1…S9 | U，与向下 c 不同向 | coherent_type、exit_cross、whole_extreme、signed_force_weakening | −100/−100 | min 48000 / 76000 | 218 |
| S1…S13 | U，与向下 c 不同向 | coherent_type、whole_extreme、signed_force_weakening | −100/150 | min 48000 / 58000 | 298 |
| S1…S17 | 无统一类型 | coherent_type | 150/−300 | min 48000 / 38000 | 378 |
| S1…S21 | 无统一类型 | coherent_type、exit_cross、whole_extreme、signed_force_weakening | −300/−100 | min 38000 / 40800 | 458 |
| S1…S25 | 无统一类型 | coherent_type、entry_cross、exit_cross、whole_extreme、signed_force_weakening | −100/−100 | min 38000 / 64800 | 538 |

S1…S17 给出最清楚的阻碍。它的 whole 极值和力度都通过，唯一失败是所有核必须严格同向分离。四个 core 依次为 [50000,60000]、[68800,71200]、[76800,78800]、[59200,61200]，先上移、再下移，不能赋统一 U/D。增加末尾材料未消除这个已出现的方向转折。

六个首窗口都被同一谓词拒绝，故最终图无根出边。每个前缀只取最终边中 known_at≤t 的子集，也不可能凭空出现根出边。这证明全部 583 个前缀都只有空路径。此论证不使用 NoPP，因此同一图移除 NoPP 的对照仍为零因子。

与 Stage68 的根窗口直接比较，S1…S5 两次的 core 均为 [50000,60000]，Own 均为 E22…121；旧历史向上 c 的末价 72000>64000，L 为 −500<1000，全部通过。H2 向下 c 末价 68000 未低于此前最低 48000，L 为 −100<−100 为假，且未向下跨过该核下界。差异是实际方向、跨界、whole 极值和力度，不能只写“数据变长但仍失败”。

长度证明也已按 27 段重新计算。n 个独立因子满足 NoPP 至少需 5n+4⌊n/2⌋ 段；四因子至少 28 段，仍不容。三因子却不再只有旧 5/9/5，允许的核数序列共有 13 个：

`(1,2,1), (1,2,2), (1,2,3), (1,3,1), (1,3,2), (1,4,1), (2,1,2), (2,1,3), (2,2,1), (2,2,2), (2,3,1), (3,1,2), (3,2,1)`。

这些长度型全部落到实际从 S1 接续的窗口并保存每因子失败字段，见 [budget-obstruction.json](run/budget-obstruction.json)。根无出边的证明已经一并排除它们，不需要事后改参数或改起点。[finite-obstruction.json](finite-obstruction.json) 给出可机器检查的六个首窗口和证明步骤。

## 内部 P/D/U 数值链与未覆盖的根前缀

五条边中 4→9→18→27 可以按源序接成 P/D/U，其实际技术方向为 Up/Down/Up；grade 都为 0，Own 连续且不重计，没有相邻 P。三个 whole 的严格三交是 [50000,80000]。对应内部核为：

| 因子 | 自有 core | Own | 完成候选证齐 |
|---|---|---|---:|
| S4…S8 | [68800,71200] | E82…181 | 198 |
| S9…S17 | [76800,78800]、[59200,61200] | E182…361 | 378 |
| S18…S26 | [40800,44000]、[64800,67200] | E362…541 | 558 |

这些已算边交给同一 `f2_contract` 必要条件函数，数值通过。只调用消费者检查保存边，没有重算窗口、换根枚举或生成第二候选。完整表在 [internal-chain.json](internal-chain.json)。

固定根仍在 state21。这条链从 state81 才开始，E22…81 没有因子承接。该缺口只有 S1…S3 三段，短于一个语法 whole 的最少五段；它不能补成同级独立前因子，也不能被并入已经固定为 S0 的前导。把缺口计入 unresolved tail 只给出账面材料，并不完成初始分解。

因此内部数值三元组为 1 个具体例证，根路径中的三元组为 0，原义获证三元组为 0。`f2_contract` 只核必要关系；本轮没有执行生产 F2，也没有用三交倒证三个孩子原义完成。内部 P 的概念方向仍为空，Up 是技术方向，未把它改称 U。

## 全前缀、完成边界与来源义务

候选边在 198、378、558 分别由 1 增至 3、5 条。138/238/338/438/558/582 等全部关键前缀中，根路径数始终是 1，只有空路径；maximal 数为 1，非空 NoPP、三孩子、F2 必要条件通过数均为 0。空路径在图中 maximal 只说明无出边，不能当成非空完成。

[all-delta-paths.json](run/all-delta-paths.json)、[path-classes.json](run/path-classes.json)、[prefix-results.json](run/prefix-results.json) 直接来自同一新入口。后者含实际消费者结果与去 NoPP 对照。空路径的尾在 198 后已包含后续可选候选，不能据账面一次覆盖推导原义完整或唯一分解。此次申请关系恰只有空路径，不提供原义唯一性证明；一般申请多解同样不会自动成为原义多解。

前缀检查以既有 raw known_at、实际价格支持和已知时刻稳定笔身份为依据。进程载入了完整源数组和参考笔数组；本轮没有验证物理未来读取隔离，也没有实现全时刻原义进行态 L(c,t)。局部发生与证齐保持两列；OriginalGeneralDiv、OriginalMovementCompleted、原义发生/确认/发布均未赋值。既有 LocalDyn64 的发布和 knownAt 未回写。

来源边界沿用已审 Stage68 和 Stage65 的实际前件：084 允许初始构造选择，并不自动认证这份映射；RootArm68 将已封 raw 外臂作为比较根的角色桥、Outer0=core 到原义外缘的解释，以及局部弱化和 raw c 封证到 whole 完成的推导仍未证明。结构笔来自已审有限 R_W 输出，订单对象到 source 笔角色的语义桥仍须给出。若主动调用 037 的真正趋势定理，须兑现所选定理的 c 角色与内部结构前件；本轮不把这些前件扩大成所有替代 F1 的统一门槛。固定递归 F2 的三个已完成次级走势资格仍没有兑现。

Stage64 的 LocalDyn64+A64-global 不相容保持原结论。其 CatalogSucc 没有成为原义生命周期 Next；本轮不能因内部数值链通过而复活该联合合同。Stage65 的 M65-pol 仅有两项几何关闭及开放尾、M65-mono 不能原样运输旧完成证等边界也保持。归属、真正接续、一般完成、合法分解覆盖/唯一性和全部合法延长上的 NE 均未获证。本 H2 合法性和 raw 证书不因候选拒绝而撤销。

## 运行与复算

主运行一次，exit 0，elapsed 0.132 秒，峰值 RSS 87,523,328 字节，低于 96 MiB；回执时新增产物 2,915,479 字节。真实 cwd 为研究 worktree R，argv 为本脚本绝对路径加 `--out` 新目录。输入前后指纹一致。准备期间几次只读文件读取暂挂后自行完成，未修改文件、未执行全局修复，也没有因此重复候选主运行。

复算命令使用新的输出子目录，不能覆盖 `run`：

```bash
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/joint-author/check_joint.py \
  --out /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/joint-author/replay-new
```

独评若须只写自己的目录，可复制 `check_joint.py`、`WorkCard.md`、`WorkCard.freeze.json`、`original-rule-extract.py`、`rule-extraction.diff` 到独占新目录，再对该目录下的新输出路径运行。脚本依赖已有冻结 E/R 绝对输入，无第三方包；使用普通 Python，不能用 `-O` 跳过断言。`assemble.py` 只分析已冻结输出和封存身份，输出文件同样排他创建。原 Stage68 脚本与结果保留不变。

作者交付的是一次完整有限检验和可复算的根失败证。原义 F1/F2、P1–P4、R_W/R_D 的整体目标没有因此结案。
