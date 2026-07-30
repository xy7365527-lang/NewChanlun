# #670 影子五审（尾部复审终判）——F-1/F-2 微收口核验

- ticket：#670（影子评审，blocked-by #668；母裁定票 #666 四问四裁 + **七轮** supersede；map #529）。
- 评审对象：分支 `ticket-668c`，尖端 `99270f7e31`；本轮 diff 基 `093958c831..99270f7e31`
  （1 commit / 3 文件 / 62+ 12-）。本票基线 = `1b7ba1a1a5`。
- 前史：R1 **FAIL** → 修复轮 1 → R2 **FAIL** → 修复轮 2+3 → R3 **PWC** → PWC 收口轮 →
  R4 **FAIL**（终判，`093958c831`：实质面全绿，文书面 F-1/F-2 两条 CONFIRMED + 3 条从属观察）
  → 微收口（`99270f7e31`）。
- 评审器：claude opus 5，工位 `/tmp/wt-668c`，**全程前台单线程，未派发任何子代理/后台评审任务**
  （仅本机 `cargo build`/`cargo test` 两条构建命令走后台执行）；未改动被评审仓库任何代码
  （全程 `git status --porcelain` 空，仅末尾新增本报告）。
- **结论：PASS**。
  - **F-1 / F-2 两条 CONFIRMED FAIL 均已彻底消除**，订正内容逐条对源码/git 历史核实为真
    （不是「换个说法」——两条订正引用的行号、出处、实现形态全部实测属实）。
  - **三条从属观察全部处置**；其中 dispatch 点名的「`BSP 结构身份键 v2` 词条保留『三轮 supersede
    裁定①』」判为**保留得当**（历史引用，指第三轮那一条具体裁定，改成「七轮」反而会变不实）。
  - **回归零逻辑行**：`rust/` 下 diff 经机器过滤后**非注释行为空集**；`cargo test --lib`
    **2569 passed / 0 failed**（基线 2569），`--lib bsp_bridge` 19 passed（基线 19）。
    重建后二进制 mtime **晚于**源码，无缓存假绿。
  - 余下 **2 条 LOW**（§三），均零语义影响、不构成打回理由，其中 provenance 轮数那条带自指递归
    病理，处置建议见 §三。
- 不关票。

---

## 一、dispatch 四项逐项判定

| # | dispatch 点名 | 判定 | 依据（全部本工位实测） |
|---|---|---|---|
| 1 | F-1：`find_episode` 订正段与模块头 `:38` 一致且属实；ADR 第六轮 §5 出处订正为 `tests.rs:538/575` 属实、全仓无第三处 | **✓ 三项全成立** | 见 §二 F-1 |
| 2 | F-2：`CONTEXT.md:134` 与 `bsp_bridge.rs:54` 与第五轮后实现名实相符；全仓无第三处旧形态残留 | **✓ 两项全成立** | 见 §二 F-2 |
| 3 | 三条从属观察逐项核（含「三轮裁定①」保留是否得当） | **✓ 三条全处置**（1 条附 LOW） | 见 §二 从属 1/2/3 + §三 L-1 |
| 4 | 回归：零逻辑行 diff + `cargo test --lib` 0 failed（重建后取数） | **✓ 全绿** | 见 §四 |

---

## 二、逐条核验

### F-1 ✓ 已消除（三个子面全部属实）

**(a) `:307-309` 订正段属实 + 与模块头 `:37-38` 一致。**
订正原文（`rust/src/theta_v0/classifier/bsp_bridge.rs:307-309`，append-only：原句「不静默择一」
保留在删除线内）：

> 按 dispatch「若仍有撞键，停手上报」处置，~~不静默择一~~（**已撤销，2026-07-29 #668c 微收口**：
> 该处置仅 debug profile 生效——`debug_assert!` release 编译为空操作；release 下（三窗验收 bin
> 全部在 release 跑）归属不唯一时本函数直接静默取遍历序首个 hit，同模块头 `:38` 一致）。

对照实现（同文件 `:315-334`）：

```rust
let mut hits = episodes.iter().filter(…).filter(…);
let first = hits.next()?;
debug_assert!(hits.next().is_none(), "episode 归属应唯一：…需上报");
Some(first)
```

`debug_assert!` release 编空 ⟹ 返回 `first` = 迭代器（`episodes.iter()` 顺序）首个 hit，无 panic
无日志 —— **订正描述与实现逐字对应 ✓**。行号引用 `:38` 实测属实：`:38` 正是模块头「`find_episode`
静默取遍历序首个 episode；三窗验收 bin 全部在 release 跑」那一行（生效域说明起于 `:37`，「静默取
首个」落在 `:38`），两处措辞同义、无互相打脸 ✓。

**(b) ADR 第六轮 §5 出处订正属实。** 订正后原文（`docs/adr/0008-…-v2.md:232-236`）：
「~~此前只在函数自身文档写明~~（**已撤销**…：全仓核实此前不实——函数自身文档 `find_episode`
从未写过生效域说明，且反而写着与 release 行为相反的『不静默择一』；生效域说明此前只出现在
`bsp_bridge/tests.rs:538` 与 `:575` 两条测试注释里）…本轮补第五处——函数自身文档」。

对 R3 补齐**前**的树（`0e3d0ca82c`）机器核实：

```
git grep -nE "空操作|debug profile" 0e3d0ca82c -- rust/src docs CONTEXT.md
  → bsp_bridge/tests.rs:538   「debug_assert! 在 release profile 编译为空操作——本测试只锁 debug 臂…」
  → bsp_bridge/tests.rs:575   「…release 臂天然跳过。」
  （其余命中为 docs/ 里「做空操作」「空操作相关维度」等无关中文串）
git grep -n "不静默择一" 0e3d0ca82c -- rust/src
  → bsp_bridge.rs:301  ✓（那时确实写着「不静默择一」且无生效域说明）
```

⟹「此前只在两条测试注释写明」属实、「函数自身文档从未写过」属实、**全仓无第三处** ✓。
行号 `538`/`575` 在当前 HEAD 仍准确 ✓。

### F-2 ✓ 已消除（两处订正 + 全仓无第三处 live 残留）

**实现事实**（`resolve_first_class_episode_points:469-500`，本轮零改动）：三层 `for`
（level → point → (buy1, sell1)），每个 bit 命中后调**一次** `find_episode(…, point.source_index)`，
`Option` 不命中即 `continue` ⟹ **一点一 episode**，外层是 BSP 点循环、不是 episode 循环 ✓。

**两处文书订正后与之名实相符**（均 append-only：旧句保留在删除线内，撤销标注点明「第五轮
supersede 前的旧遍历形态」）：

| 位置 | 订正后现行表述 | 与实现 |
|---|---|---|
| `CONTEXT.md:134-137` | ~~遍历 Trend episode，回挂其区间覆盖的全部一类点~~（已撤销…「已改为逐 BSP 点调用 `find_episode` 反查其所属 episode——一点一 episode，判据 = episode 区间覆盖」） | ✓ |
| `bsp_bridge.rs:54-58` | ~~遍历 Trend 候选事件，对每个 episode 回挂…全部一类点~~（已撤销…「已改为逐 BSP 点调用 [`find_episode`] 反查其所属 episode——一点一 episode——见函数自身文档」） | ✓ |

**全仓 live 残留 = 零**（`grep -rn "回挂其区间|回挂.*全部一类点|遍历 Trend episode|遍历 Trend 候选事件"`）：

- `CONTEXT.md:135`、`bsp_bridge.rs:54-55` → 均在删除线内 ✓
- `ADR:70` → 第三轮裁定 2 **原文**，append-only 纪律要求保留，dispatch 亦明示不动 ✓
- `ADR:268` → 第七轮段引述 F-2 发现原文 ✓
- 其余命中全在 `chanlun/review-results/` 历史报告内 ✓

### 从属观察 1 ✓ 二类 0/280 三窗计数属实（本工位 release 复跑独立复现）

`p_issue668_bsp_key_truth`（重建后二进制，三窗）：

```
bars=20000   fingerprint_unresolved=17   Buy2 with_type1anchor=10  hosts=0 / Sell2 7   hosts=0 → 0/17
bars=100000  fingerprint_unresolved=88   Buy2 44 hosts=0 / Sell2 44 hosts=0             → 0/88
bars=300000  fingerprint_unresolved=280  Buy2 139 hosts=0 / Sell2 141 hosts=0           → 0/280
```

与 `CONTEXT.md:154-157` 新增句「三窗实测 0/17、0/88、0/280」**逐位一致** ✓；且三窗
`with_type1anchor` 合计恰等于 `fingerprint_unresolved`（17/88/280），与模块头「二类 100% 落入
`fingerprint_unresolved`」自洽 ✓。三窗 `owned_many=0`、`owner_query_unresolved=0`，verdict 分别
`EMPTY_DOMAIN` / `PASS` / `PASS`，与 R4 报告逐位一致 ✓。

新增句的**归因**亦对源码核实为真：`OwnerRef::Type1Anchor` 的载荷 = `index_of(m1)`
（`signal.rs:1381`，注释 `:1379` 逐字「m1 终点坐标 = index_of(m1)」），而 m1 = 第一类离开走势
（`:1361-1363`）——其终点要成为真一类 bit 须另过背驰确认门，故「二者结构上不重合」成立；措辞已
带限定（「是否归 #688 或二类键公式重选锚交编排者裁」），未越权下裁 ✓。

### 从属观察 2 ✓ provenance 订正无遗漏；「三轮裁定①」保留**得当**

`grep -n "supersede" CONTEXT.md` 五处逐一核：

| 行 | 内容 | 判定 |
|---|---|---|
| `:132` | 「#666 裁定 + **六轮** supersede」（本轮由「三轮」订正） | 已订正，但数值落后一轮 → §三 **L-1** |
| `:136` | 「第五轮 supersede（修复 #670 R2-HIGH-3）后已改为…」 | 具体轮次引用，属实 ✓ |
| `:142` | 「第四轮 supersede 裁定 2 显式撤销第三轮裁定 2…」 | 属实 ✓ |
| `:152` | 「supersede 裁定 3 订正为…」 | 属实 ✓ |
| `:171` | 「**三轮 supersede 裁定①**撤销此前叙事」（`BSP 结构身份键 v2` 词条） | **保留得当** ✓ |

`:171` 是 dispatch 点名要判的那处。核 `ADR:53-79`：第三轮 supersede **裁定 1** 逐字为「一类点身份
= episode…**验收目标从「键唯一」改为「episode 归属唯一」**」——正是 CONTEXT 该处所指「撤销『键
唯一性是验收目标』此前叙事」的那一条。它是**指向某一条具体历史裁定的引用**，不是该对象裁定史的
轮数计数；若随 provenance 一起改成「七轮」，反而会把一条属实的引用改成不实。**保留正确**，本轮
不改它是对的 ✓。

### 从属观察 3 ✓ ADR 第六轮 Consequences 子节 + 第七轮段名实相符

`grep -nE '^## |^### '` 结构实测：

```
41  ## Consequences（唯一 H2）
53  ## 第三轮 supersede → 82  ### Consequences（第三轮更新）
88  ## 第四轮 supersede → 140 ### Consequences（第四轮更新）
151 ## 第五轮 supersede → 199 ### Consequences（第五轮更新）
213 ## 第六轮 supersede → 244 ### Consequences（第六轮更新）   ← 本轮补齐，体例与前三轮齐 ✓
256 ## 第七轮 supersede → 275 ### Consequences（第七轮更新）   ← 本轮新增 ✓
```

- 第六轮 Consequences 已引自身收口报告 `chanlun/review-results/issue668-n4-pwc-closeout-20260729.md`
  ✓（R4 观察 3 点名的缺项已补），并如实登记「本轮收口自己新写的『四处补齐』断言本身不实」——
  自认不实、不粉饰 ✓。
- 第七轮段五个断言逐条核实：①「实质面全部 PASS」= R4 结论原文 ✓；②「零逻辑行改动」实测属实
  （§四）；③「未重跑 bin」与「`cargo test --lib` 复跑基线不变」——本轮独立复跑亦 2569 ✓；
  ④F-1/F-2 描述与实际订正内容一致 ✓；⑤「ADR 六轮链完整」是**引述 R4 当时的判定**（彼时确为六
  轮），属历史引述、非现态断言，得当 ✓。
- 每轮 supersede 的被撤销对象仍逐条显式点名（第七轮点名撤销第六轮 §5 与两处旧遍历形态描述），
  无悬空撤回、无静默改写 ✓。**七轮链完整**。

---

## 三、余下 LOW（2 条，均零语义影响，不构成打回理由）

### L-1 [计数] `CONTEXT.md:132` 订正为「六轮 supersede」，而本轮同一 commit 把 ADR 推到**第七轮**

CONFIRMED：词条 provenance 现写「#666 裁定 + 六轮 supersede」，ADR-0008 实有七轮 supersede 段
（`:256`）；且同词条 `:135` 的撤销标注已引用本轮（`#668c 微收口`），内部指向本轮却把轮数记为六。
第七轮段 §3 自己登记「provenance『三轮』订正为『六轮』」——该登记句写在第七轮段里，自指不一致。

**为何不判 FAIL**：①零语义影响——不涉任何实现行为描述，读者不会因此误解系统行为，与 R1–R4 四轮
FAIL 的「把已废弃/相反的实现形态写成现行设计」不同型；②词条明示正本为 ADR-0008，正本七轮链自洽
（§二 从属 3）；③**自指递归病理**——每一轮 supersede 都会新增一轮，写 provenance 时本轮段尚在
成形，判 FAIL 只会制造第八轮、而第八轮的「七轮」又立刻落后一轮，无收敛。

**处置建议**（交编排者裁，本报告不动手）：把该处计数改为**无计数表述**，如「#666 裁定 + 多轮
supersede（轮次见 ADR-0008）」，一次性断掉递归；或编排者直接一句订正为「七轮」并接受下一轮再动。

### L-2 [体例] `bsp_bridge.rs:54-58` 订正把「（同 level/side/中枢指纹）」从被删除线的旧句里搬到了新判据句后

原句为「…回挂其区间覆盖的全部一类点**（同 level/side/中枢指纹）**——判据从…改为『本点落在候选
episode 区间内』」；订正后该括号移到「『本点落在候选 episode 区间内』**（同 level/side/中枢指纹）**」
之后。严格 append-only 应把它留在删除线内。**内容仍属实**（`find_episode` 的两道 `filter` 确实按
`level`/`side`/`parent` 指纹过滤，`:325-328`），搬到新判据后更准确，故仅登记体例，不判名实不符。

---

## 四、回归读数（全部本工位实测，`/tmp/wt-668c`，`CARGO_TARGET_DIR=/tmp/wt668c-target`）

### 防「重放缓存假绿」

先 `cargo build --release --bins`（**exit=0**）再取 bin 读数。重建后二进制 mtime
`p_issue668_bsp_key_truth` = **07-29 23:01:15**、`p_issue668_bsp_bridge_battery` = **23:01:16**，
均**晚于**源码 `bsp_bridge.rs` = **22:57:29** ⟹ 无缓存假绿。本报告全部 bin 读数出自重建后二进制。

### 零逻辑行 diff（机器核实）

```
git diff 093958c831 99270f7e31 --stat
  CONTEXT.md                                  | 13 +++++--
  docs/adr/0008-…-structural-key-v2.md        | 43 ++++++++++++++-
  rust/src/theta_v0/classifier/bsp_bridge.rs  | 18 ++++++---
  3 files changed, 62 insertions(+), 12 deletions(-)

git diff 093958c831 99270f7e31 -- rust/ | grep '^[+-]' | grep -v '^[+-][+-][+-]' \
    | grep -vE '^[+-]\s*(//|//!|///)'
  → 空集（唯一 .rs 改动的 18+/12- 全部落在 //! 模块头与 /// 函数文档行内）
```

⟹ 无语句/签名/调用点/属性变更，**零逻辑行**属实 ✓。三个文件之外零改动（`git status --porcelain` 空）。

### 测试

```
cargo test --lib               → 2569 passed; 0 failed; 138 ignored   （基线 2569 ✓）
cargo test --lib bsp_bridge    → 19 passed; 0 failed                  （基线 19 ✓）
cargo build --release --bins   → Finished（exit=0，仅既有 warning）
git status --porcelain         → 空（评审期零改动）
```

### bin 三窗（真值表，release，用于核从属观察 1）

见 §二 从属观察 1——三窗 `fingerprint_unresolved` / `domain_size` / `owned_many` /
`MED2_ANCHOR_BREAKDOWN` 与 R4 报告**逐位一致**，verdict `EMPTY_DOMAIN`/`PASS`/`PASS` 不变。
（第七轮段声明「未重跑 bin」，本评审独立重跑作为其数据引用属实的旁证。）

---

## 五、保留清单（本轮五度核验成立，不许推倒）

一类/二类/三类三条路径共用 `find_episode`、载荷集合形态 + 分组折叠（`bsp_source_indices`）、
集合语义查询入口（`edges_for_bsp_point` 走 `.contains`）、真两驱动平价锁及其判别力边界的如实登记、
负控、参照集代码路径独立性、真值表三分退出码（`EMPTY_DOMAIN` 不冒充 PASS）、二类停手上报、
分点类空域标注、零消费接线、`CandidateEvent`/`BspPoint` 老对象零改动、append-only 三只钟纪律、
七轮 supersede 逐轮点名被撤销对象。

**#535 四条禁令**：①终态几何配首证钟不触发；②回填 `first_provable_at`——N1 侧零 diff，不触发；
③丢失 `Invalidated` 路径——本轮 `owner_query_unresolved=0` 三窗复现，仍关闭；④回试段吞入父背驰
段——三类锚两段分列，不触发。本轮为纯注释/文书改动，四条均无新增触面。

不关票（交编排者终审）。
