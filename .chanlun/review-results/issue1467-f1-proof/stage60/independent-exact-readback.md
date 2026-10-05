# #1467 Stage60.exact_root 独立声明读回

本次由新启动的读回代理 `/root/readback_stage60_exact` 执行，只收到目标声明、定义、环境路径和只读边界。未读取作者报告、开工卡、ResearchProgress、其他评审结论或期望定理。读取了指定 Lean 验证技能及其验证参考、实际 Lean 源和 import 闭包；源内说明性注释不作数学依据。本报告没有比较原始数学契约，也不作 P/F1 资格或放行判断。

## 实际声明与量词

Lean 实际输出：

```lean
Stage60.exact_root : And Stage60.ATOMS
  (And Stage60.FEATURES (And Stage60.CORE Stage60.FORCE))
```

该声明无参数、无隐式实例前提、无宇宙参数、无外部假设。四支均针对文件内固定数据。内部量词只有 `ATOMS` 对两组 `Fin 30` 的遍历，以及 `NoContainment` 对固定特征列表相邻位置 `Fin (es.length - 1)` 的遍历。它没有对任意价格历史、任意笔列、任意时刻或任意延长历史作全称断言。

数据域：`Tick = Int`；`Index = Nat`；方向是 `up/down`；价格历史是 `List Int`，不是 OHLC `List Bar`；速度和力度属于 Lean core 的 `Rat`，不是浮点或实数。`Stroke` 和 `Segment` 均为方向、起止自然数索引、起止整数价格构成的记录，其类型本身不带良构证明。

## ATOMS 的真实内容

两份笔列表 `positive/negative` 各 30 项，两份价格列表各 123 项。对每组的全部 30 笔，根证明 `StrokeSource ps s`：

1. `s.startIndex > 0`，`s.endIndex + 1 < ps.length`；
2. `s.endIndex = s.startIndex + 4`，并且 `s.startIndex < s.endIndex`；
3. 起止价格等于对应价格列表索引处的整数；
4. 上笔起点比前后各一个价格严格低、终点比前后各一个价格严格高，并且起价严格低于终价；下笔四个邻居比较及起终价方向相反。

这实际证明了固定笔记录与固定标量价格列的一致性及上述局部极值、跨度条件。笔记录和价格列都直接写在 fixture；没有从价格列运行笔生成器后与整份笔列表相等的命题。`StrokeSource` 不检查全部内部价格，也不包含 K 线包含处理、完整笔构造算法、相邻笔接续或方向交替谓词。固定列表中相邻记录的接续与交替可读出，但不是 `StrokeSource` 的判据。

`getElem!` 在定义中允许默认值，不过此处并非借空域或越界成立：外层确证长度为 30，`Fin 30` 非空；价格索引的左右邻居由边界与跨度条件保证在界内。零时长也被该支排除。终点局部极值检查读取 `endIndex + 1` 的价格，故该声明本身不提供在终点时刻即可判定的性质。

## FEATURES 的真实内容

每份固定笔列表取五个 8 笔窗口，起始列表位置依次为 0、5、10、15、20；对应方向依次为上、下、上、下、上。两组共十项，每项证明四件事：

- 固定 `SegEndData` 满足 `SegEndComplete`；
- 它的第一、第二特征元素没有严格区间缺口；
- `SelectedBy d ss` 成立；
- 窗口中与指定方向相反的特征笔，其**原始相邻**特征价格区间互不包含。

`FeatureElem.ofStroke` 取一笔两个端点价格的最小值和最大值，丢弃方向、索引与笔身份。`SegEndComplete` 展开为：上向时中间元素的 high 严格高于两侧 high，下向时中间元素的 low 严格低于两侧 low；加上对应 `SegmentEndUp/Down`；加上按指定方向比较传入的起终价。`SegmentEndUp/Down` 在无缺口分支直接为 `True`，有缺口时才要求反向序列含分型。十份 fixture 的 `revSeq` 都定义为空，本根又证明十份均无缺口，因而本根不涉及有缺口确认路径。

`SelectedBy` 实际执行 `extractSegEndData d.dir d.startPrice d.endPrice ss`。该函数筛取反向笔、按端点区间进行包含合并、扫描第一个匹配的三元素分型；成功后输出方向和**原样传入**的起终价。`SelectedBy` 比较成功输出的方向、起终价，以及三元素的六个 low/high 数值。它不比较 `revSeq`，没有整份 `SegEndData` 相等断言，也没有段索引或原笔身份的相等断言。方向、起终价相等中的起终价来源是调用参数按定义回填，六个特征边界相等则经过实际抽取计算验证。

固定窗口筛取后各有四个反向特征元素，`NoContainment` 各检查三个相邻对，量词并非空域。十份 `d` 的三元素分别由列表位置 `(3,5,7)`、`(8,10,12)`、`(13,15,17)`、`(18,20,22)`、`(23,25,27)` 的笔直接构造。`SelectedBy` 确认扫描所得数值与这些指定数值一致。

`SegEndData` 类型没有时间索引字段。五组起终价是直接指定的 `10000→16000`、`16000→12000`、`12000→15000`、`15000→12500`、`12500→18000`。实际输入窗口末笔结束索引分别为 33、53、73、93、113；对应指定终价在所列数据中的相关端点索引为 21、41、61、81、101。根没有证明以这些较早端点为确认时刻，也没有最早确认、前缀稳定或不看未来的命题。输入窗口及其时序关系是固定定义与记录数值，不能由 `SelectedBy` 名称补出时间性质。

## CORE 的真实内容

三个 `Segment` 直接定义为：

| 段 | 方向 | 起止索引 | 起止价格 |
|---|---|---|---|
| s1 | 下 | 21→41 | 16000→12000 |
| s2 | 上 | 41→61 | 12000→15000 |
| s3 | 下 | 61→81 | 15000→12500 |

`CenterConfirmedComplete` 只要求前两段方向不同、后两段方向不同，以及三个端点价格区间交集的下界严格小于上界。`computeZD` 是三个区间 low 的最大值；`computeZG` 是三个 high 的最小值。根证明方向条件、严格非空，以及确切值 `ZD = 12500`、`ZG = 15000`。

三个段的时间索引在 fixture 中直接给定；此中枢谓词及两项边界计算不读取这些索引。没有声明将三个段绑定为 `segmentsOfComplete` 输出、将它们绑定为十个 `SegEndData` 的构造结果，或证明它们的确认时序。import 了 `SegmentAutoConstruct` 不会给根自动附加该模块其他函数的保证。

## FORCE 的真实内容

`b = positive.take 5`，`c = (positive.drop 20).take 5`，`cn = (negative.drop 20).take 5`。这里的列表来源由定义直接固定。速度为 `(终价−起价)/(终索引−起索引)`，索引各自先转 `Int` 再相减，最后在 `Rat` 中除法。非空笔串的 `impulse` 是末笔速度减首笔速度；空串按定义取零。这里三串各五笔，且 ATOMS 已确证相关笔跨度均为 4。

| 固定串 | 首笔速度 | 末笔速度 | 根所证力度 |
|---|---:|---:|---:|
| b | 500 | 750 | 250 |
| c | 250 | 125 | -125 |
| cn | 250 | 500 | 250 |

`IsImpulseDivergence b c` 的完整定义仅为 `impulse c < impulse b`。根因此还证明 `-125 < 250` 及 `¬ (250 < 250)`。该谓词没有携带中枢、段完整性、极值、价格突破或时间前后约束。四个根合取项同时成立，但根类型没有额外关系命题将中枢、段判据与两串力度拼接为其他判定。`negative` 也并非整份结构失败：本根明确同时证明它的 ATOMS 和五份 FEATURES；仅其指定力度严格比较失败。

## 独立机器读回及证据边界

将实际源的全部 16 个本地 import 模块复制到新仓外目录，按依赖序用显式 Lean 二进制逐一编译，未复用作者模块 `.olean`，未改原源和旧构建。目标两份源与 `construction-evidence` 对应副本逐字哈希相等。执行成功后原源哈希再次检查未变。

实际环境：Lean 4.31.0，`arm64-apple-darwin24.6.0`，commit `68218e876d2a38b1985b8590fff244a83c321783`，Release；direct 模式，无项目 `lean-toolchain`、Lake 配置或 manifest。编译器实际路径为 `/Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean`。主控随后只补充了环境元数据，所报版本及三个运行时文件哈希与本次实测一致。首次普通 `lean --version` 被宿主 cwd 的 elan 选择带往 4.34.1 下载，已立即中断（exit 130）；该次没有用于任何证明结果。

Lean 的传递公理输出：

| 声明 | 实际公理集 |
|---|---|
| exact_root | propext、Classical.choice、Quot.sound |
| atoms_checked | propext |
| features_checked | propext、Quot.sound |
| core_checked | 空 |
| force_checked | propext、Classical.choice、Quot.sound |

精确根未依赖 `sorryAx` 或额外公理。这是 Lean 自身 `#print axioms` 对实际根的传递结果，并非扫描 import 文件的词语。某个被导入文件中存在其他 `native_decide` 定理不等于根依赖它；以根的实际传递集为准。

本次是声明读回和独立重新编译，没有输入 expected-type 契约，不标记数学契约吻合或 v2 exact-root acceptance。运行时清单保存二进制、库和 `lean --deps Readback.lean` 直接列出的 artifact 哈希；它不是 v2 verifier 的全 loaded-environment 清单，不能冒充该工具的完整环境收据。

证据目录：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage60/exact-readback/run-20261005-102423/`。其中 `source-manifest.json` 保存所有 16 份源的路径、哈希、import；`commands.json` 保存每条实际 argv、cwd、LEAN_PATH、退出码和耗时；`readback.log` 保存显式类型、定义与公理输出；`environment-check.json` 保存运行时哈希及原源未变结果。全部 16 条编译、读回、deps 命令退出码均为零。

实际复现入口：

```sh
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage60/exact-readback/replay.py
```

该脚本为每次运行建立新目录。实际核心调用形态（完整展开见 `commands.json`）：

```sh
LEAN_PATH=<本次新目录>/project /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean -o <模块>.olean <模块>.lean
LEAN_PATH=<本次新目录>/project /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean Readback.lean
```

SHA-256：

| 对象 | 哈希 |
|---|---|
| Stage60Proof.lean | f024c8f5665abab18e0c9f0b29ca6e643de8ee0c3b84743f1cb5dc56a203fb85 |
| Stage60Fixture.lean | 5c569612d5e18fee5924d5a0e1aa9a112b5b631ffbc1a5c2d7cc9cb0e1878084 |
| source-manifest.json | 2113ee177d9bf060a2de7f783aceb86b3144d4bc326eccc7eefa226f86ea320a |
| readback.log | e0894c238039a81919a4b5d9ab19688c4a9913c9beb3878a2df249498e41e88d |
| bin/lean | 1b370cfcbf44e80d1b004ab1b1ab9a4c73951f9f7c242140bcff9bc577576554 |
| bin/lake（未用于构建） | 58261a1a2fa1a362376c71e02ca854a093e71cc5e6ea64b287a931cb2565273d |
| lib/lean/libleanshared.dylib | a695a9aa68481c42b99619e0103537e6b433c6fa7021927b184e19a686d03989 |
