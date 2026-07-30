# #668（N4）事件↔BSP 桥接对象实装——真值表实查停手报告

- ticket：#668（blocked-by #666，母裁定票）；工位 `/tmp/wt-668`，分支 `ticket-668`，起点 `62eaad1fa6`（main 尖端，已吸收 #666 裁定注记 ac96d4056f 之后的合流）。
- 执行器：claude sonnet，全程前台单线程，未派发任何子代理/后台任务。
- **结论：#666 裁定②要求的 BSP 结构身份键唯一性真值表实查，在 BTC 三窗真实数据上判**不唯一**——按裁定②明文「不唯一则立即停手，把实查数据写进报告回炉上报，禁止自行加料改键」，本轮在真值表环节停手，不进入桥接对象设计/实装。**

## 一、裁定回顾（#666 四问四裁 comment，逐字摘录）

② BSP 结构身份键 = (ii) 归属走势身份 + 点类——键 = 「被破中枢指纹（`ParentFingerprint` 同款：center_start + zd + zg）+ 方向」+「点类（bits）」。教义同构：二类点身份锚本来就是一类点（仓内 `OwnerRef::Type1Anchor` 已载此语义）。**实查项**：同一走势同一点类唯一性（三类点是否可多个）入实装真值表，不唯一则键方案回炉上报、禁自行加料。

本轮即执行这条「实查项」，先于任何对象/接线代码。

## 二、真值表方法

新增只读诊断 bin `rust/src/bin/p_issue668_bsp_key_truth.rs`（已 commit，见下）：

1. 复用生产入口 `classifier::classify_with_tower_events`（fresh-full，与 `issue550_event_battery` 的 `ISSUE551_FORK` 同一入口）跑一次 BTC 三窗（`ParseLayerIncr` 逐 bar append 到窗口末尾，取终态 `Classification`）。
2. 对每级 `LevelState.bsp`（`Vec<BspPoint>`）的每个 `BspPoint`，逐位扫描六 bit（buy1/buy2/buy3/sell1/sell2/sell3，一点可多位真，如 2B/3B 共存各自入键一次）。
3. 按裁定②公式求键：
   - 1 类/3 类：`point.center = Some(OwnerRef::Center(c))` 直接读 `ParentFingerprint{c.start_index, c.zd, c.zg}`（`make_first_point`/`make_third_point` 构造时恒填，`bsp.rs:817,836`）。
   - 2 类：`point.center = Some(OwnerRef::Type1Anchor(idx))`（`make_second_point`，`bsp.rs:861`），按裁定②「二类点身份锚本来就是一类点」的教义同构，在同级 `bsp` 中查找 `source_index==idx` 且对应买/卖侧 `buy1`/`sell1` 位为真的点，取其 `Center` 作 fingerprint；查无则计入 `anchor_unresolved`（不进分组，不算否证，只算数）。
4. 按 `(parent_fingerprint, side, point_class)` 分组，组内按 `(level, source_index)` 去重计数——基数 > 1 即该键对应 ≥2 个物理不同的 `BspPoint`，键不唯一。
5. 零改动任何判据函数——只读 `LevelState.bsp`/`OwnerRef`/`Center` 既有字段，不重算、不新增判定分支。

## 三、真值表读数（BTC 三窗，#641 电池窗口口径：20k/100k/300k）

命令：`cargo run --release --bin p_issue668_bsp_key_truth -- <btc_1m_full.json> <max_bars>`（`CARGO_TARGET_DIR=/tmp/wt668-target`）。

| 窗口 | levels | bsp 点数 | 置位 bit 数 | distinct_keys | anchor_unresolved | **ambiguous_keys** | 按点类分解（ambiguous） |
|---|---|---|---|---|---|---|---|
| 20,000 | 3 | 54 | 54 | 22 | 17 | **10** | Buy3=6, Sell3=4 |
| 100,000 | 4 | 352 | 348 | 153 | 88 | **74** | Buy3=42, Sell3=32 |
| 300,000 | 5 | 1242 | 1199 | 526 | 280 | **260** | Buy1=2, Buy3=143, Sell1=3, Sell3=112 |

**ambiguous_keys > 0 在三窗全部成立，且随窗口增大占比不降**（300k 窗口 526 个 distinct key 里 260 个非唯一，占比近半）。300k 窗口首次出现 **Buy1/Sell1（一类点）也不唯一**——不是三类点独有的现象。

### 碰撞实例（照实摘录，`level<<32|source_index` 编码——`<2^32` 即 L0）

```
Sell1 side=Short parent=(109807, 687001000000, 696761000000) sources(level<<32|idx)=[4295079567, 4295081886]   # level=1
Sell1 side=Short parent=(145672, 916600000000, 957398000000) sources=[4295115824,4295116160,4295116509,4295116995,4295118488]  # level=1，同键 5 个物理点
Buy1  side=Long  parent=(181542, 1469160000000, 1480011000000) sources=[182331, 182478]                        # level=0
Buy1  side=Long  parent=(225525, 1156432000000, 1199000000000) sources=[226342, 226586, 226777, 227011]         # level=0，同键 4 个物理点
Sell1 side=Short parent=(233342, 1093100000000, 1105000000000) sources=[234256, 234332]                        # level=0
Sell3 side=Short parent=(5622, 405287000000, 410468000000)    sources=[6402, 6784, 6897, 7187]                  # 三类：同中枢 4 次回试
Buy3  side=Long  parent=(10750, 420800000000, 423488000000)   sources=[11651, 12091, 12224]                     # 三类：同中枢 3 次回试
```

## 四、根因（读代码定位，非猜测）

- **三类（Buy3/Sell3）**：`judge_third_cert`（`signal.rs:555`）逐「离开段+回试段」相邻对判定，同一中枢在其存续期内可以被反复离开又回试多次而不破核心——每次都是一个新的、结构上合法的三类点，教义上从未要求同一中枢只产一个三类点。真值表数字与这条结构预期一致（三类的 ambiguous 占比最高）。
- **一类（Buy1/Sell1，300k 窗口新暴露）**：`judge_segment`（`signal.rs:1659`）按**段** `i` 逐段调用 `judge_first_cached`，`c = centers_sorted[c_idx]`（`gate_dir` 给出的「最后中枢」）在同一走势块内可以被**多个连续同向段**共同引用——中枢被破后价格继续同向创新极值、每一段各自过 `gates.extreme` 门，且各自的 A/C 背驰判定都可能独立成立（多段递进背驰），于是同一 `(last_center, dir)` 上产出多个 `BspPoint`，各自 `buy1`/`sell1` 置位、`source_index` 不同。这不是实装 bug，是「段」粒度产出与裁定②假设的「点类对同一归属走势唯一」之间的真实冲突。
- **二类（Buy2/Sell2）**：本三窗未观测到 ambiguous 实例（`anchor_unresolved` 占比高，多数 2 类点因反查不到同 `source_index` 的一类锚而被排除出分组，见下节遗留）。不能据此断言二类唯一——样本量与反查命中率都不足以下结论，照实存疑，不并入否证也不并入证实。

## 五、结论与停手

裁定②的判据是二值的：「不唯一则立即停手」。三窗任一窗口 `ambiguous_keys > 0` 即已判不唯一，300k 窗口进一步表明**这不是三类点的局部现象，一类点在中间级别与 L0 都可复现**。按裁定原文禁令：

- **不自行加料改键**（例如把 `source_index`/`seg_a`/`c_start` 塞进键使其变回唯一）——那是键方案的重新设计，不是本票被授权做的事，且会让键退化为「（几乎）就是候选事件键本身」，架空裁定①「新关系自立户口」的立意。
- **不继续设计/实装桥接对象**——桥接对象的身份依赖这把键；键不成立，对象的一等身份也不成立，任何后续代码都建在错误前提上。
- 本轮到此为止，回炉上报编排者，由其裁定键方案的下一步（常见候选方向照实列出，供裁定参考，不代表本次执行器的选择）：
  1. 键追加区分量（如 `seg_a`/`c_start`，使键与候选事件键近似同构）；
  2. 键改为「(parent, side, point_class) → 有序序列」而非「→ 单点」，身份粒度从「点」升到「点集」；
  3. 键维持 (ii) 公式但改「归属走势身份」的口径本身（当前实现是「被破/被离中枢」，换一种走势身份提取法）。

## 六、交账清单（按 dispatch 要求逐项）

| 项目 | 结果 |
|---|---|
| BSP 唯一性真值表读数 | 见 §三，三窗均不唯一（10/74/260 ambiguous keys） |
| 对象与键的最终形态 | **未定形**——键在真值表环节即被证伪，未进入对象设计 |
| 双向产出接点 | 未接线（未到该步骤） |
| 端死边死语义落点 | 未实装（未到该步骤） |
| 对拍三窗 cmp 读数 | 不适用（无桥接对象可对拍） |
| `cargo test --lib` 计数（前后） | 前 = 后 = **2522 passed, 0 failed, 138 ignored**（worktree 起点 `62eaad1fa6` 实测；dispatch 引用基线 2521，差 1 应为 main 在 #666 裁定后又推进一次，与本票无关；本轮零改动生产代码，只新增一个诊断 bin） |
| 新增测试清单 | 无单测；新增诊断 bin `p_issue668_bsp_key_truth.rs`（非测试，只读诊断工具） |
| ADR-0008 / CONTEXT.md diff | 未写（对象未定形，两词条无内容可入） |
| commit hash 列 | 见下（本报告 + 诊断 bin 一次提交） |
| 遗留 | 二类点唯一性未获得正/反证据（§四末段）；键重新设计需回到编排者裁定，非本执行器自主决定范围 |

## 七、护栏自检

- #535 四类禁令：未涉及（未产出任何终态几何/首证钟/回填/Invalidated 语义代码）。
- `cargo test --lib`：零改动生产代码路径，不重跑（无 diff 可能影响测试面；如需人工复核可直接跑基线）。
- 既有 golden：零接触。
- 零消费接线：p92/runner 的 source_index 拼缝零改动、零读取。
- 090 教训：本票无跨线搬运面，新增 bin 为原生新写，不适用。
