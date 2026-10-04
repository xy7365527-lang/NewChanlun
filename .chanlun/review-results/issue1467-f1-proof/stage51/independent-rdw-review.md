# Stage51 RDW-v1 独立审查

2026-10-05，研究票 #1467。审查工位 `/root/review_stage51_rdw`；新上下文接收冻结包，未继承作者会话，未把作者自检或另一源义独评的结论作为本次依据。已读本树 `math-research-workflow`、`rigorous-open-math-research` 及独评规则。本报告是工作草稿、独立审查证据，不是教义裁定或生产验收。

**结论：在冻结定义的数值关系范围内通过，未发现需修复的作者件缺陷。** 全部指定订单消息、点对、赋向、证书、归属及可用时刻均与独立重算一致；非空性、最多两词、证书计数和严格单调变换结论可直接证明。此通过不认证 P 完成、原义 Z3、原义父核、完整 F₁/F₂、在线分解或市场价值。几何负例只否定其明确写出的几何蕴含。

## 1. 冻结对象、方法与收据

研究树为 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`，下文作者路径相对 `.chanlun/review-results/issue1467-f1-proof/`。冻结清单 17 件、输入锁 10 项（定义加九项既有输入），均在重放前后通过 SHA-256；清单内有字节数的也逐项核对。清单和输入锁有重叠，不能按 27 个独立输入计算。

| 对象 | SHA-256 |
|---|---|
| `stage51/relative-direction-candidate.md` | `722e1bc374abdebc0442b2ec87f229a6183a7fd96b067daed754470d3e4961c4` |
| `stage51/direction-evidence/frozen-manifest.json` | `c33d90e29dba0398281d6021da51d777bb4e58ecc4fb56a01ff01dd733f48da7` |
| `stage51/direction-evidence/input-lock.json` | `80304398b0cb50a57c15a4d7bb65902e9f4397815382e8801d154ac174b885ff` |
| `stage51/direction-evidence/rdw-v1-definition.md` | `e70dd235e212f2b8061395b8787df5e030e94688f00b96af53a27b9038a190fc` |
| `stage51/direction-evidence/rdw-probe.mjs` | `b36018651641cf80eacfcc7eb294924bf4bdd464ff159339fc5d5414c7437c4d` |
| `stage51/direction-evidence/check-certificates.mjs` | `42cda7cb0ac49f4831fa92c87d097887aa423a37f66e6d71f3065bbaf7f58592` |

所有运行产物仅写到 [仓外证据目录](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/)。最终统一重放在 `final-run/`：两个作者 MJS 与独立重算器退出码均为 0，9 份作者输出 JSON 与冻结结果逐字节相等。完整逐项哈希见 [freeze-before.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/final-run/freeze-before.json) 和 [freeze-after.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/final-run/freeze-after.json)，命令与 stdout/stderr 摘要见 [runs.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/final-run/runs.json)。哈希检验证明本次读到的冻结字节；不凭文件时间字段独立认证作者历史上“先冻定义、再算结果”的操作顺序。

独立重算器 [independent_recompute.py](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/independent_recompute.py) 不导入作者代码。它用 Python 标准库 `Fraction`，按消息中的实际 ID 维护完整存活订单簿，从最优买卖价量直接算 `(bidPrice×askQty+askPrice×bidQty)/(bidQty+askQty)`，再与冻结 q/W 比较。方向见证采用时间顺序扫描：当前达到高阈值时与此前所有低阈值点连接，当前达到低阈值时与此前所有高阈值点连接。每对实际索引恰在其第二个端点被处理时加入，因此既不遗漏，也不重复。这与作者双索引枚举及其首末阈值检查器分别实现。

另对六个固定区域重新枚举全部 290 对内部切点，按冻结目录的完整源区间重建核心归属，得到恰好相同的 24 份必要分区；未把作者给出的必要列表直接当完整性证明。这只是指定区域、指定旧目录内的有限复核，不扩大到其他目录、完成关系或原义分解。

## 2. 有限重算结果

| 计数对象 | 独立结果 | 含义 |
|---|---:|---|
| 旧消息 | 80 | 两条原冻结合成历史各 40 条 |
| 新消息 | 20 | 几何负例 7、端点对照 7、双解对照 6 |
| 重建状态 | 105 | 41+41+8+8+7，含每条历史初态 |
| 旧历史 Q/W 有序值比较 | 3,362 | 2×41²，含自身及重复数值比较 |
| 旧必要分区 | 24 | 六视图各四份；12 份非严格交排除 |
| 严格 RDW 窗口 | 15 | 12 份旧窗口加三个新对照 |
| 见证点对出现次数 | 124 | 按窗口、孩子和方向计，同一源点对可在别的窗口再次出现 |
| 方向赋值检查 | 120 | 每窗口完整八词；其中旧窗口 96 次 |
| 获选证书三元组 | 55 | 旧窗口 52，负例 0，端点对照 1，双解对照 2 |

最终独立运行完成 7,029 个等值/条件断言。它们是验证动作计数，不是样本量、统计显著性或一般证明。两次重放也没有增加消息样本。

| 历史、目录时刻 | 第一严格分区：H、获选词、组合数 | 第二严格分区：H、获选词 |
|---|---|---|
| 正向 e15 | `0→5→8→12`：`[1110,1140]`，UDU，10 | `0→5→9→12`：`[1135,1140]`，空 |
| 正向 e23 | `9→13→16→20`：`[1210,1240]`，UDU，8 | `9→13→17→20`：`[1235,1240]`，空 |
| 正向 e31 | `17→21→24→28`：`[1310,1340]`，UDU，8 | `17→21→25→28`：`[1335,1340]`，空 |
| 镜像 e15 | 同第一切点：`[1560,1590]`，DUD，10 | 同第二切点：`[1560,1565]`，空 |
| 镜像 e23 | 同第一切点：`[1460,1490]`，DUD，8 | 同第二切点：`[1460,1465]`，空 |
| 镜像 e31 | 同第一切点：`[1360,1390]`，DUD，8 | 同第二切点：`[1360,1365]`，空 |

两条旧历史逐状态满足 `q_mirror=2700−q_up`。正向第一窗口 A 的全部 U 点对为 `(0,5)…(4,5)`，B 的全部 D 点对为 `(5,6),(5,8)`，C 的全部 U 点对为 `(8,9)`；唯一方向词不等于唯一证书，确有 `5×2×1=10` 组。第二窗口方向集合是 `{U}/{U,D}/{D}`，八词中逐片可行的只有 UUD、UDD，二者均不交替。

独立核对不止比较这些摘要：全部八词逐片可行性、所有 q/W 点对值、全局索引、`pointsAvailableAt=j`、每个获选词的完整笛卡尔积，以及集合去重均与冻结 JSON 一致。详见 [全部窗口独立结果](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/final-run/independent-output/all-window-recomputations.json)；完整 ID 余额和 q/W 状态保存于同目录五份 `*-ledger.json`。

来源复核同时确认：各片独占 `(s,t]` 内事件，状态支撑是 `[s,t]`；相邻片只共读一个边界状态。主例 e5 归 A、e9 归 C，未因连接不在所拥有核心内而删掉。已知左余部、被审窗口、已知右余部、未消费未来事件拼接后恰为 e1…e40，没有漏用或重复消费。核心归属按完整源区间重新算；其成员腿、退出腿和回返确认腿的实际支撑范围与可知时刻均核对。此处采用 RC49 目录作为冻结候选输入，没有再次授予目录完备性的原义公理或原义核心身份。

## 3. 独立数学证明

**每片 D 非空，且可用同一极值点对覆盖所有严格子带。** 对有限片取达到全最小值的索引 a、达到全最大值的索引 b。设 `[L,U]` 严格且含于该片包络，则 `x_a≤L<U≤x_b`，故 a≠b。若 a<b，(a,b) 是 U 见证；若 b<a，(b,a) 是 D 见证。固定这两个点后，对片内任意严格子带同样成立。此命题保证每片至少一个方向；它没有在三片之间附加相邻异向条件。极值重复可能给相反时间次序的第二见证，不能任取极值点对后宣布唯一方向。

**交替解恰为两个候选词的筛选。** 对二值字母，首个字母给定后，第二个必须是其对偶，第三个必须等于首个。因此候选仅 UDU、DUD。UDU 被接受当且仅当三片依次含 U、D、U；DUD 对偶。两个析取支均成立当且仅当每片同时含 U 和 D。由此最多两词、恰一词和无词的判据全部成立。27 种非空方向集合组合已独立复算，但一般结论来自上述二值论证。

**证书计数是有限乘积。** 固定一个被接受方向词 d，定义没有其他跨片点对兼容条件，故其全部证书就是 `W₀(d₀)×W₁(d₁)×W₂(d₂)`；不同分量组合是不同索引三元组，计数为三基数之积。不同词在严格带下不共享同一方向标注的证书，合计可逐词求和。边界状态共读已在输入合同允许，不应另外删掉共边界证书。双解对照每片 `[0,10,0]` 各有一个 U 和一个 D 点对，故 UDU、DUD 各一份，总计两份，冻结产物完整保留。

**严格递增变换逐点对保持。** 只须 f 定义在全部有限源值和所用端点上，并严格保序。对任意有限集合，`min f(x)=f(min x)`，`max f(x)=f(max x)`。因此三交新端点为 `[f(L),f(U)]`；`x_i≤L` 与 `f(x_i)≤f(L)` 等价，`x_j≥U` 与 `f(x_j)≥f(U)` 等价。时间索引不变，U/D 两见证集逐对相等，方向词和全部索引证书也相等。无须 f 连续；结论只涉及变换后有限支撑的包络，不把整个区间的函数像断言为区间。

**严格递减变换逐点对交换 U/D。** 新片最小/最大分别为 `f(u_r),f(l_r)`，新 H 为 `[f(U),f(L)]`。新 U 的两个不等式分别等价于原先 `x_i≥U,x_j≤L`，即原 D；新 D 同理对应原 U。词按 U↔D 双射，索引证书逐项不变，词数和组合数保持。冻结镜像验证了具体 `f(q)=2700−q`，一般性来自此证明。

对当前 W，直接通分有 `W(q₂)−W(q₁)=20(q₂−q₁)/((10+q₂)(10+q₁))`。q₁、q₂>0 保证分母正，故严格保序。有限精确比较与一般代数结论一致，不依赖浮点容差。

**缩窄带只增加或保留见证。** 若 `[L′,U′]⊆[L,U]`，则 L≤L′<U′≤U；原 U/D 的每个不等式都推出新不等式。冻结例 `[0,10,5,8]` 对全带只有 U，对 `[5,8]` 多出 D `(1,2)`。这一性质并不授权修改 RDW-v1 已固定的三交 H 以挽救失败；改变 H 就是不同输入窗口。

## 4. 反例、诊断和资格边界

作者稿第 110、124、128–130 行的负例成立。其正平移路径为 `1000,1010,1005,1002,1010,1005,1002,1012`，真实范围的交为 `[1002,1010]`，精确 W 带为 `[25801/253,5201/51]`。三片各仅一个 U 点对 `(0,1),(3,4),(6,7)`；D 均空，故无交替词。订单 add 均使用尚未存活的 ID，reduce 均有足量余额，reservoir 最终为 984；所有消息与完整余额快照已独立核对。每条消息只改一个 ID，而整段历史有多个 ID。

它反驳的是“事件接续、完整数值范围、严格三交足以推出 RDW 交替”。三片没有完成、原义核、层级或方向桥证书，因而不满足原义 F₂ 的全部输入前提；没有资格把它升级成原义 F₂ 反例。正向通过窗口同样不能补出这些前提。作者稿第 5、130、156、176–180 行明确保持此边界，没有发现从几何结论越权到原义结论。

第 160–162 行的“首末严格增量”按稿内明确定义就是 `x_t>x_s`。独立重算确认端点都增大时仍可为 RDW U/D/U，证明其区别于端点净方向。该术语不等于另一份源义讨论中的“首个/最后一个非零局部增量”；本审查不把本例借给后者作未检验结论。第 164 行 η 则按核心实际源增量重算：K0/K1/K2 去零后均 D/U/D，首向均 D，与主例 RDW U/D/U 不同；η 仍只是候选形成诊断。

P 的 `conceptDirection`、各片的 `originalCompletionEvidence`、`originalZ3Evidence`、各严格窗口 `originalParentCore` 全部保持 null。D_cert 未执行；手填类别、目录核心或净方向均未暗中代替资格。三份新例是合成订单消息，不是市场记录；80 条旧消息亦是原合成历史，不能与并行源义审查再次相加。

## 5. e12/e15 的精确含义

对外部已经指定的 `0→5→8→12` 或 `0→5→9→12`，完整源状态至少读到 e12；复用 K0/K1/K2 的冻结封口支持需要到 e15。独立检查确认核心所用成员、退出/回返腿均在相应 `sealedAt` 前可知，最大的所需支持时刻确为 15。其他两组分别为范围 e20/e28、目录 e23/e31；镜像相同。

这些是**固定切点输入的回查数据条件**。本包没有提供当时如何发现、决定并发布切点的在线算法，也没有原义完成和方向桥的支持。12 份旧严格窗口全部显式满足 `onlinePartitionPublicationAt=null`、`semanticEligibleAt=null`，`originalCompletionSupportAvailableAt=null`；15 份严格窗口都保持语义资格为空。三个新对照没有在线分区发布声明。不得把范围时刻或目录封口时刻填成在线发布、P 完成或原义可上送时刻。

由相邻三片先得到候选数值 H，再计算相对 H 的方向关系，是一个无需读取父成立布尔值的数学定义。它尚未证明方向不依赖邻居，也没有解决原义上的依赖问题。稿第 33、178 行已明确这一点。

## 6. 复现与可复用范围

实际最终命令为：

```sh
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/run_review.py /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/final-run
```

再次复现应使用新目录，防止覆盖证据：

```sh
rdw_review_parent=$(mktemp -d /tmp/issue1467-stage51-rdw.XXXXXX)
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/run_review.py "$rdw_review_parent/run"
```

该驱动先核输入与作者冻结清单，依次运行两个 MJS 和独立 Python，比较全部九份作者输出，最后复核输入哈希并保存原始 stdout/stderr、命令、退出码和结果清单。

| 审查产物 | SHA-256 |
|---|---|
| `run_review.py` | `ed4da768195770f93fce84a36bd9ff96b8333950e38aa371f00d9ba059e343ff` |
| `independent_recompute.py` | `b87ee6d5832dcc9ea7453820820776cd5a6bd5c6b901f4b3bcfa13455d44fb39` |
| [最终收据](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/final-run/receipt.json) | `89cff286b8d139ec004bc22d6741376384203ead686b4ac68cbea231d0f65dd2` |
| [运行证据清单](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage51/rdw-review/final-run/evidence-manifest.json) | `e91fae3044cc0f6687703ec2eb3f73d9bad6060fbdbf082add0945d3d54501dd` |
| `final-run/independent-output/all-window-recomputations.json` | `9491bdd0fef059717c0b650f98cc63296846cb377eb2096e25007f84e03c1656` |
| `final-run/independent-output/independent-summary.json` | `5fc9e409ce905e67dc04daeaee783ee223a910d07d61cc9ae088be1ed7dde967` |

审查器初次运行曾把镜像常数误写成 2800，订单重放揭示实际为 2700，故更正审查器后重新通过；初次失败 stdout/stderr 原样保留。这是审查工具自身的假设订正，不是作者稿缺陷；没有改作者任何冻结字节。

可复用的是精确定义内的一般引理、指定有限解集与反例，以及带明确来源的回查时刻。原义方向桥、完成、同级连续、父生命周期和下一层闭合仍未交付。没有扩展同型历史、引入 Lean 工程或改 formal/生产/教义/Progress；没有提交或发布。冻结作者件无需为本次结论修复，后续只能在上述限定范围内引用“独评通过”。
