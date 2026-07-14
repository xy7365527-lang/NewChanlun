# 种子失败几何归因调研（task #65）

- 日期：2026-07-13
- 性质：只读调研 + 隔离重放；未修改生产 Rust、塔、Lean、定义或原文
- 对象：#61 的 `1,333` 个 exact-three 真不可组合漏段（k1=`767`、k2=`353`、k3=`213`）
- 数据：BTC 1m 全历史 `4,613,599` bars，`as_of=4,613,598`，L0 `40,003` 线段
- 隔离原型：`/private/tmp/task65-seed-geometry/rust/src/bin/task65_seed_geometry.rs`
- 逐对象证据：`/private/tmp/task65-seed-geometry/task65-seed-geometry.log`（每个对象一条 `T65_OBJECT`，35 个残余另有 `T65_RISK`）

## 0. 结论先行

1. **[已验证] 1,333 个对象的全部 3,999 个实际含自身连续三元组，第一失败原因全部是 a：三段无公共重叠。** 对象矩阵与三元组矩阵均为 a=`100%`，b/c/d=`0`。逐层原数见 `/private/tmp/task65-seed-geometry/task65-seed-geometry.log:797,1163,1381`，守恒断言见同日志 `:1382`。
2. **[已验证] 方向不是本样本的失败源。** k1 的 2,301 个候选三元组全部先通过方向交替，再因 `ZD>ZG` 失败；k2/k3 的生产上级判据本来就只查几何，不查方向交替。生产顺序见 `rust/src/theta_v0/classifier/center.rs:113-118,136-162,174-192`，上级投影有效域见 `rust/src/theta_v0/classifier/recursive_tower.rs:387-407`。
3. **[已验证] c“包含处理后退化”在当前塔 seed seam 不可发生。** 塔把既成 `UnitRange` 三个一组直接交给 builder；成功 `i+=3`、失败 `i+=1`，中间没有包含吸并或删段步骤。见 `rust/src/theta_v0/classifier/recursive_tower.rs:180-207,221-232`。若未来改变上游 lower 分段，它属于 `rule_version/settlement` 变化，不是本判据的第一失败支。
4. **[已验证] host 与 seed 必须分开。** 1,333 个无 seed 身份对象中，基线已有 `1,281` 个按坐标获 host，仅 `52` 无 host；重组使其中 `19` 个获 host、`7` 个进入 Completed，残余 `35`。见 `/private/tmp/task65-seed-geometry/task65-seed-geometry.log:1382`；#64 的独立原数与边界见 `chanlun/review-results/orphan-reduction-paths-20260713.md:72-85,191-205`。
5. **[推断，给 #66] 明确结论：seed 失败是划分策略无关的，host/Unassigned 状态是划分策略相关的。** 在固定 settled lower ledger 与当前 rule version 下，任何 exact-three 划分只能从合法连续三元组中选择；该 1,333 个对象没有任何含自身合法三元组，所以任何划分都救不了其 **seed 身份**。但消费层可通过坐标 chain、延伸或重组吸收它们，所以不能据此墓碑。

## 1. 权威判据与重放口径

### 1.1 塔中枢 seed 判据源码锚

生产塔扫描的精确调用链是：

1. `compose_level` 在 k1 选择 `center_from_segments`，在 k>=2 选择 `center_from_window`：`rust/src/theta_v0/classifier/recursive_tower.rs:221-232`。
2. `detect_centers_windowed` 对连续三段直接调用 builder；成立消费 3 段，不成立消费 1 段：同文件 `:180-207`。
3. k1 完整判据按源码顺序先查 `dir_alternates`，再算 `ZD=max(lo)`、`ZG=min(hi)` 并拒绝 `ZD>ZG`：`rust/src/theta_v0/classifier/center.rs:113-118,136-172`。
4. k>=2 几何判据只拒绝 `ZD>ZG`，不读取方向：同文件 `:174-201`；上级 `UnitRange` 的 lo/hi/坐标投影见 `rust/src/theta_v0/classifier/recursive_tower.rs:387-407`。

这也复核了 #61 的口径：“k1=方向交替+共同重叠，k>=2=外缘区间共同重叠”，见 `chanlun/review-results/q5-orphan-attribution-research-20260713.md:21-31`。

### 1.2 对象与“第一失败”

**[已验证]** 每个目标对象 q 都不在序列边界，故恰有三个实际含自身窗口：`j=q-2,q-1,q`；总枚举量 `1,333×3=3,999`。原数见 `/private/tmp/task65-seed-geometry/task65-seed-geometry.log:797,1163,1381`；枚举边界实现见 `/private/tmp/task65-seed-geometry/rust/src/bin/task65_seed_geometry.rs:547-555,597-602`。

每个三元组按生产顺序记录第一失败：

- a `A_NO_COMMON_OVERLAP`：方向门（若适用）已过，`ZD=max(lo)>ZG=min(hi)`；
- b `B_INVALID_DIRECTION`：仅 k1 可触发，且早于几何门；
- c `C_CONTAINMENT_DEGENERATE`：若 seed seam 曾做包含吸并并少于三段；当前源码无该步骤；
- d `D_OTHER`：生产 builder 返回 `None`，但不属于前三支；本次为 0。

对象级矩阵采用“到达最深判据支”：只要某个含自身窗口通过早期门、到达几何门，就把对象归到 a；同时原样保留三个窗口的 first-failure vector。**本样本全部 vector 都是 `A+A+A`，故该聚合规则不影响计数。** 实现见临时原型 `:572-619`，逐三元组输出见 `:640-669`。

## 2. k1..3 × 第一失败原因矩阵

### 2.1 对象级互斥矩阵

| target k | a 无公共重叠 | b 方向/交替非法 | c 包含后退化 | d 其他 | 合计 |
|---:|---:|---:|---:|---:|---:|
| 1 | **767** | 0 | 0 | 0 | 767 |
| 2 | **353** | 0 | 0 | 0 | 353 |
| 3 | **213** | 0 | 0 | 0 | 213 |
| **合计** | **1,333（100%）** | **0** | **0** | **0** | **1,333** |

事实锚：`/private/tmp/task65-seed-geometry/task65-seed-geometry.log:797,1163,1381-1382`。

### 2.2 三元组级 first-failure 矩阵

| target k | 实际含自身三元组 | a | b | c | d |
|---:|---:|---:|---:|---:|---:|
| 1 | 2,301 | **2,301** | 0 | 0 | 0 |
| 2 | 1,059 | **1,059** | 0 | 0 | 0 |
| 3 | 639 | **639** | 0 | 0 | 0 |
| **合计** | **3,999** | **3,999（100%）** | **0** | **0** | **0** |

事实锚同上。k2/k3 日志虽保留方向投影作诊断，但生产 `center_from_window` 不以其为门；不能把 `alt=0` 误报为 b。源码锚：`rust/src/theta_v0/classifier/center.rs:174-192`。

## 3. host 交叉：52 基线无 host 与 35 重组残余

### 3.1 按 k 交叉

| k | exact-three 非种子 | 基线已有 host | 基线无 host | 重组后残余 | 52 中获 host | 其中 Completed |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 767 | 727 | 40 | 25 | 17 | 5 |
| 2 | 353 | 343 | 10 | 9 | 1 | 1 |
| 3 | 213 | 211 | 2 | 1 | 1 | 1 |
| **合计** | **1,333** | **1,281** | **52** | **35** | **19** | **7** |

事实锚：`/private/tmp/task65-seed-geometry/task65-seed-geometry.log:797,1163,1381-1382`；#64 独立交叉复核见 `chanlun/review-results/orphan-reduction-paths-20260713.md:76-85,193-205,281-285`。

### 3.2 按第一失败原因交叉

| 原因桶 | 全部对象 | 基线无 host 52 | 重组后残余 35 | 52 中获 host | 其中 Completed |
|---|---:|---:|---:|---:|---:|
| a 无公共重叠 | **1,333** | **52** | **35** | **19** | **7** |
| b 方向非法 | 0 | 0 | 0 | 0 | 0 |
| c 包含后退化 | 0 | 0 | 0 | 0 | 0 |
| d 其他 | 0 | 0 | 0 | 0 | 0 |

**[推断]** 这张表否定“a 桶可直接墓碑”：a 同时包含 `1,281` 个基线已归属对象，还包含本次重组新吸收的 `19` 个对象。seed 几何失败不是 host 不存在的充分条件。

## 4. 无公共重叠的几何细分

### 4.1 互斥定义

对每个 a 三元组使用闭区间：

1. `GAP_ADJACENT`（跳空型）：`I1∩I2=∅` 或 `I2∩I3=∅`；
2. `MONOTONE_STEEP`（单调陡峭型）：相邻两对都相交、首尾不相交，且三段 `(lo,hi)` 同向非降或同向非升；
3. `GEOMETRY_OTHER`：`ZD>ZG`，但不满足前两型。

实现锚：`/private/tmp/task65-seed-geometry/rust/src/bin/task65_seed_geometry.rs:557-570`。

对象级要从三个失败窗口选一个代表形态，因此取 `ZD-ZG` 最小（最接近成 seed）的窗口，平局取最早 j；三个窗口的原始形态仍全部保留在逐对象日志。选择实现：临时原型 `:621-631`。

### 4.2 对象级分布

| k | 跳空型 | 单调陡峭型 | 其他 | 合计 |
|---:|---:|---:|---:|---:|
| 1 | 531（69.230769%） | 120（15.645372%） | 116（15.123859%） | 767 |
| 2 | 202（57.223796%） | 97（27.478754%） | 54（15.297450%） | 353 |
| 3 | 127（59.624413%） | 58（27.230047%） | 28（13.145540%） | 213 |
| **合计** | **860（64.516129%）** | **275（20.630158%）** | **198（14.853713%）** | **1,333** |

事实锚：`/private/tmp/task65-seed-geometry/task65-seed-geometry.log:797,1163,1381`。

补充三元组级分布：跳空 `2,772/3,999=69.317329%`，单调陡峭 `689/3,999=17.229307%`，其他 `538/3,999=13.453363%`；原数同上。

### 4.3 九例画像

下表价格均为生产整数 tick；每例展示对象三个失败窗口中 `ZD-ZG` 最小的代表窗口。完整三个窗口见对应日志行。

| k / lower / source range | 形态 | 代表 j 的三段区间 `[lo,hi]` | `ZD` | `ZG` | `ZD-ZG` | 锚 |
|---|---|---|---:|---:|---:|---|
| k1 / `L0#56` / `[7868,7901]` | 跳空 | `[378681000000,396499000000] / [378681000000,384900000000] / [385826000000,392154000000]` | 385826000000 | 384900000000 | 926000000 | log `:2` |
| k1 / `L0#354` / `[45646,45931]` | 单调陡峭 | `[373025000000,384541000000] / [375801000000,399000000000] / [384833000000,405500000000]` | 384833000000 | 384541000000 | 292000000 | log `:8` |
| k1 / `L0#674` / `[86977,87258]` | 其他 | `[568469000000,570000000000] / [566065000000,574899000000] / [570002000000,578891000000]` | 570002000000 | 570000000000 | 2000000 | log `:13` |
| k2 / `L1#81` / `[34191,34509]` | 跳空 | `[415964000000,431500000000] / [417388000000,429998000000] / [380100000000,413311000000]` | 417388000000 | 413311000000 | 4077000000 | log `:798` |
| k2 / `L1#100` / `[42118,42315]` | 单调陡峭 | `[348300000000,374001000000] / [365804000000,387000000000] / [375100000000,390000000000]` | 375100000000 | 374001000000 | 1099000000 | log `:800` |
| k2 / `L1#1361` / `[498611,498860]` | 其他 | `[746481000000,756498000000] / [752879000000,765100000000] / [757000000000,763192000000]` | 757000000000 | 756498000000 | 502000000 | log `:826` |
| k3 / `L2#41` / `[57281,59179]` | 跳空 | `[385005000000,409995000000] / [410100000000,430000000000] / [396500000000,425100000000]` | 410100000000 | 409995000000 | 105000000 | log `:1164` |
| k3 / `L2#79` / `[108217,109672]` | 单调陡峭 | `[632500000000,661393000000] / [656104000000,698886000000] / [668510000000,730000000000]` | 668510000000 | 661393000000 | 7117000000 | log `:1171` |
| k3 / `L2#248` / `[319948,320558]` | 其他 | `[772800000000,810000000000] / [762700000000,810900000000] / [736001000000,769034000000]` | 772800000000 | 769034000000 | 3766000000 | log `:1188` |

上述 `log` 均指 `/private/tmp/task65-seed-geometry/task65-seed-geometry.log`。

## 5. 墓碑风险评估

### 5.1 reason/geometry 层结论

- **[已验证] 固定 settled lower 坐标 + 固定 rule version 时，a 桶三种形态都结构性永久无 seed。** 每个对象唯一可能的三个含自身窗口都已经穷举并为 `ZD>ZG`；未来只追加更右侧 lower 不会创造第四个含自身连续三元组。守恒见日志 `:1382`。
- **[推断] 这里的“永久”只限定 seed 身份。** 若 parser/上级对象被 supersede、坐标或 rule version 改变，候选 lower ledger 本身已换版本，应重新枚举，不能拿旧版本结论跨版本封死。
- **[已验证] a 桶仍可能被延伸/坐标 chain/重组吸收。** #58 明定 chain 从完整 lower ledger 按坐标取料，而不是只拼 seed subs：`chanlun/review-results/assembler-spec-20260712.md:88-121`；#64 实测 19 个基线无 host 获 host、7 个 Completed：`chanlun/review-results/orphan-reduction-paths-20260713.md:193-205,281-285`。
- **[推断] b 若未来出现，在 fixed settled directions 下同样是永久无 seed、但不推出永久无 host；c 更明显依赖上游包含/分段版本；d 必须逐子类审计。本样本 b/c/d 均为 0，不能用本数据估计其 host 吸收率。**
- **[已验证] 当前没有墓碑授权。** #61 要求 settled、后继封闭、显式 coordinate finalization、穷举无合法 host 且无 Pending 跨越、精确 `judge_at` 五门合取：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:135-145`；#64 也明确开放域 Tombstone=0：`chanlun/review-results/orphan-reduction-paths-20260713.md:268-287`。

### 5.2 35 个残余逐对象风险标签

标签：

- `S-permanent`：当前 settled lower + rule version 下，seed 身份结构性永久失败；
- `H-boundary-open`：当前重组视图中对象跨越相邻 Move group 起点，host 归属对分组边界敏感，仍有重组/延伸吸收风险；
- `B/R`：基线/重组视图是否命中该边界诊断；33 个为 `1/1`，2 个为 `0/1`；
- `禁止墓碑`：五门未齐，不得把 seed failure 写成 `TOMBSTONED`。

| k | lower | source range | 代表几何 | B/R | 风险标签 |
|---:|---|---|---|---:|---|
| 1 | `L0#2498` | `[294379,294678]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#3705` | `[417456,417565]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#4869` | `[542428,542539]` | 其他 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#10581` | `[1169165,1169293]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#11956` | `[1319311,1319352]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#12211` | `[1349713,1349876]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#13019` | `[1437139,1437152]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#14718` | `[1621897,1622008]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#20127` | `[2264911,2264933]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#20299` | `[2286677,2286887]` | 其他 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#22171` | `[2510715,2510734]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#22729` | `[2576876,2576984]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#24227` | `[2759962,2760015]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#25368` | `[2903157,2903192]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#27013` | `[3084709,3084854]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#28272` | `[3229763,3229845]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#30608` | `[3499553,3499792]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#30760` | `[3516730,3516953]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#31288` | `[3577921,3577959]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#32469` | `[3716206,3716484]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#33046` | `[3783351,3783467]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#33574` | `[3847558,3847605]` | 其他 | 0/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#35944` | `[4132879,4133078]` | 其他 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#36442` | `[4190140,4190318]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 1 | `L0#37199` | `[4277801,4277866]` | 跳空 | 0/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 2 | `L1#3278` | `[1183803,1184381]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 2 | `L1#3700` | `[1333681,1334083]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 2 | `L1#5304` | `[1939180,1939671]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 2 | `L1#6576` | `[2446451,2446609]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 2 | `L1#8479` | `[3192471,3192734]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 2 | `L1#9472` | `[3578942,3579315]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 2 | `L1#9573` | `[3616553,3616977]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 2 | `L1#10947` | `[4171769,4172028]` | 跳空 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 2 | `L1#11518` | `[4397210,4397392]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |
| 3 | `L2#107` | `[145672,147050]` | 单调陡峭 | 1/1 | S-permanent；H-boundary-open；禁止墓碑 |

逐项原始邻接 host 坐标与机器标签见 `/private/tmp/task65-seed-geometry/task65-seed-geometry.log` 的 35 条 `T65_RISK`；判定实现见临时原型 `:679-702,831-855`。35 的几何构成为跳空 20、单调陡峭 11、其他 4；但三型均为 `H-boundary-open`，没有一种获得墓碑豁免。

## 6. 给 #66 架构复审的输入

### 6.1 明确二分

| 问题 | 结论 | 架构含义 |
|---|---|---|
| “能否让该 lower 成为某个 exact-three seed 的三段之一？” | **划分策略无关：不能。** 3 个含自身窗口全失败；换非重叠选择只是在合法窗口集合中改选，不能创造新合法窗口。 | `SeedEligibility(lower_id, lower_version, rule_version)` 可作为可重算、确定性的派生事实。 |
| “能否让该 lower 属于某个 Completed/Pending host chain？” | **策略相关：能。** 基线已有 1,281/1,333 有 host；52 中又有 19 被重组吸收，7 Completed。 | `HostMembership` 必须带 `as_of + partition/move version + rule_version`，并通过 append/supersede 表达。 |
| “35 个残余能否现在墓碑？” | **不能。** 全部仍位于重组 group boundary；且缺显式坐标最终化等五门。 | 保持 `Unassigned/REOPENED`，不得把 `NO_CONTAINING_SEED_WINDOW` 升格为 `FINALIZED_NO_LEGAL_HOST`。 |

### 6.2 推荐给 #66 的裁决句式

**[推断/建议]**：

> 在固定 settled lower ledger 与 rule version 下，`NO_CONTAINING_SEED_WINDOW` 是 partition-invariant 的 seed 资格否定；它只禁止对象充当 exact-three seed 成员，不禁止对象作为完整 lower ledger 成员进入 Pending/Completed Move host。Host membership 属于消费层、as-of 与 partition-version 相关状态，必须与 seed eligibility 解耦，并通过 append-only supersede 演化。不得以 seed failure 单独触发 tombstone。

该句与 #58 的“塔窗口是不可变 seed、完整走势由消费层组装”边界一致：`chanlun/review-results/assembler-spec-20260712.md:11-21,54-73,88-121`。

## 7. 隔离重放、稳定断言与可复现命令

### 7.1 隔离边界

本次复用 #64 模式：复制 `/private/tmp/task64-orphan-replay/rust` 到新目录，数据缓存只读软链到 #58 工作树。生产树只写本报告。#64 原隔离约定见 `chanlun/review-results/orphan-reduction-paths-20260713.md:364-388`。

```bash
mkdir -p /private/tmp/task65-seed-geometry/analysis
cp -R /private/tmp/task64-orphan-replay/rust \
  /private/tmp/task65-seed-geometry/
ln -s /private/tmp/c58-assembler-work/analysis/data_cache \
  /private/tmp/task65-seed-geometry/analysis/data_cache

cd /private/tmp/task65-seed-geometry/rust
CARGO_TARGET_DIR=/private/tmp/c58-assembler-work/rust/target \
  cargo build --release --features backtest_bin --bin task65_seed_geometry

/private/tmp/c58-assembler-work/rust/target/release/task65_seed_geometry \
  > /private/tmp/task65-seed-geometry/task65-seed-geometry.log \
  2> /private/tmp/task65-seed-geometry/task65-seed-geometry.err
```

### 7.2 固定截点前缀稳定

每级固定 `1/4、1/2、3/4` 三个 settled-lower 截点，分别断言：

1. 全输入按 `as_of=t` 裁剪得到的 exact-three failure vector/geometry，等于只传实际前缀；
2. 全输入按 `as_of=t` 重组得到的 `MoveView + selected windows + gaps + score`，等于只传实际前缀。

| k | cut / as_of | seed geometry | recomposed host |
|---:|---|---|---|
| 1 | `10000/1106020`, `20001/2250635`, `30002/3431564` | 3/3 pass | 3/3 pass |
| 2 | `3016/1089157`, `6032/2228868`, `9048/3416055` | 3/3 pass | 3/3 pass |
| 3 | `889/1076894`, `1778/2229328`, `2667/3393706` | 3/3 pass | 3/3 pass |
| **合计** | 9 个截点 | **9/9 pass** | **9/9 pass** |

事实锚：`/private/tmp/task65-seed-geometry/task65-seed-geometry.log:794-796,1160-1162,1378-1382`；断言实现见临时原型 `:858-878,939-945`。

### 7.3 输出摘要

```text
DATA bars=4613599 as_of=4613598 l0_segments=40003
OBJECTS k1=767 k2=353 k3=213 total=1333
CONTAINING_TRIPLES k1=2301 k2=1059 k3=639 total=3999
FIRST_FAILURE a=3999 b=0 c=0 d=0
BASELINE host=1281 no_host=52
RECOMPOSE residual=35 resolved_any=19 resolved_completed=7
PREFIX_GUARDS seed_geometry=9/9 host_view=9/9
```

### 7.4 重复性

```bash
/private/tmp/c58-assembler-work/rust/target/release/task65_seed_geometry \
  > /private/tmp/task65-seed-geometry/task65-seed-geometry.verify.log \
  2> /private/tmp/task65-seed-geometry/task65-seed-geometry.verify.err
cmp /private/tmp/task65-seed-geometry/task65-seed-geometry.log \
    /private/tmp/task65-seed-geometry/task65-seed-geometry.verify.log
shasum -a 256 \
  /private/tmp/task65-seed-geometry/task65-seed-geometry.log \
  /private/tmp/task65-seed-geometry/task65-seed-geometry.verify.log
```

两次 stdout 均为 1,382 行且 `cmp` 相等；SHA-256 均为：

```text
c346ad6750556d91f48955c2b0671dc9a8121cec6fae5dd2316e33afbdf1080d
```

两次运行 stderr 均为 0 行。最终守恒断言为 `1,333 / 52 / 35 / 19 / 7 / 9 / 9`，实现见 `/private/tmp/task65-seed-geometry/rust/src/bin/task65_seed_geometry.rs:939-957`，输出见 `/private/tmp/task65-seed-geometry/task65-seed-geometry.log:1382`。

## 8. 认识论边界

- **[已验证]** 本报告证明的是 BTC 1m 当前全历史、当前 lower ledger、当前 rule version 下的 exact-three seed 几何与 #64 sequence-envelope/DP host 交叉，不是所有市场/周期的统计定律。
- **[推断]** “结构性永久无 seed”只在 lower 对象坐标与 rule version 固定时成立；规则迁移或对象 supersede 后必须重算。
- **[已验证]** `Completed` 只沿用 #58 当前组装器判定；本任务没有发明 A/C pair、方向 provider 或生产接线。#58 的完成/待定边界见 `chanlun/review-results/assembler-spec-20260712.md:123-164`。
- **[推断]** 因此 #66 最重要的架构防线不是尝试“修复”这 1,333 个 seed，而是防止把 immutable seed ineligibility 与 versioned host membership 合并成一个永久孤儿布尔值。
