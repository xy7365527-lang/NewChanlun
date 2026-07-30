# #670 影子四审（终判轮）——#668 PWC 收口核验

- ticket：#670（影子评审，blocked-by #668；母裁定票 #666 四问四裁 + **六轮** supersede；map #529）。
- 评审对象：分支 `ticket-668c`，尖端 `152872dfde`；本轮 diff 基 `0e3d0ca82c..152872dfde`
  （1 commit / 5 文件 / 178+ 17-）。本票基线 = `1b7ba1a1a5`。
- 前史：R1 **FAIL**（3H/2M/3L）→ 修复轮 1 → R2 **FAIL**（4H/3M/3L）→ 修复轮 2+3 → R3 **PWC**
  （`0e3d0ca82c`，R2 十条全真修实 + 新增 2 MED / 3 LOW）→ PWC 收口轮（`152872dfde`）。
- 评审器：claude opus 5，工位 `/tmp/wt-668c`，**全程前台单线程，未派发任何子代理/后台任务**；
  未改动被评审仓库任何代码（全程 `git status --porcelain` 空，仅末尾新增本报告）。
- **结论：FAIL**（终判）。
  - **实质面全部 PASS**：回归四面全绿、两个 bin 三窗读数与三审报告**逐位一致**、参照集独立性成立
    （非生产复写）、废弃索引删净、生产逻辑零改动（`bsp_bridge.rs` 确为纯注释）、ADR 六轮链完整。
  - **文书面仍有 2 条 CONFIRMED 名实不符**（§二 F-1 / F-2），按 dispatch「如仍有名实不符按 FAIL
    打回」判 FAIL。两条都是**零功能影响**，但两条都是本票前三轮 FAIL 的同一species——「文书断言
    先于/背离实现」，其中 F-1 更是**本收口轮自己新写进 ADR 第六轮的一句不实陈述**（声称某处已有
    说明，grep 全仓证明该处从未有过，且该处现存的原话与 release 行为相反）。
  - 补丁面极小：3 处文本 + 1 句订正，不涉代码、不涉判据、不需再跑 bin。
- 不关票。

---

## 一、dispatch 五项逐项判定

| # | dispatch 点名 | 判定 | 依据（全部本工位实测） |
|---|---|---|---|
| 1 | R3-MED-1：`CONTEXT.md` 两词条与实现逐句名实相符 | **✗ 部分**（三个重点全对，另有一句漏订正 → F-2；两条从属观察 → §三） | 三个重点逐条对源码核实为真：①`heads()`/revision 携点集——`BspBridgeEdge::bsp_source_indices: Vec<usize>`（`:187`）、`observe()` 按 `BridgeKey` 分组 `BTreeSet` 折叠（`:542-568`）、`edges_for_bsp_point` 走 `.contains(&source_index)`（`:598-602`），与订正段字字对应 ✓；②三类判据已迁——`resolve_bridge:422` 走 `find_episode(..., entry.leave_interval.1)`，`trend_index_by_interval_end` 全仓无代码残留 ✓；③「零违反」已收窄——20k `EMPTY_DOMAIN exit=2` 本轮复现，词条订正段收窄为「仅一类、仅 100k/300k、32 bit」与实测 `domain_size=3/29`、`owned_many=0` 一致 ✓。**但** `CONTEXT.md:134`「遍历 Trend episode，回挂其区间覆盖的**全部**一类点」未订正（→ F-2） |
| 2 | R3-MED-2：参照集三类迁移的**独立性** + 三窗复跑一致性 + 废弃索引无残留 | **✓ 三项全成立** | **独立性成立、非生产复写**：battery bin 只 `use` 两个公有类型（`BspBridgeBook`/`BspPointClass`，`:64`），零调用 `find_episode`/`trend_episodes`；三类改走 bin 自有 `find_covering` closure（`:150-159`，`Vec<(u32,Side,(usize,i64,i64),usize,usize,CandidateKey)>` 线性扫描 + `break` 取首个），与生产的 `&[TrendEpisode]` 迭代器 + `debug_assert` 两条路径分开写，只共享「episode 区间覆盖」这条已裁定的判据本身——正是「判据共享、代码路径独立」的本意。**三窗复跑逐位一致**（§四）。**废弃索引删净**：`trend_by_exact_end` 全仓零代码残留（仅存于报告/ADR 的历史叙述）；单独 `cargo check --bin p_issue668_bsp_bridge_battery` 该 bin 零 warning，无死代码 |
| 3a | R3-LOW-1：battery bin 撤回标注 | **✓** | `:189` 已改为 `（~~HIGH-2 修复覆盖二类~~——已撤销，…二类锚坐标在生产上恒判 `None`、零命中，此判据未实测…）`，与真值表 bin `:29`、ADR 第五轮 §2 同款口径 |
| 3b | R3-LOW-2：ADR 第三轮裁定 2 二类半句原处标注 | **✓** | ADR `:70-73` 原处已插入「**未实测**——第五轮 supersede §3 查明该分支在生产上恒判 `None`、零命中，此处只是判据写法沿用，不是已验证落地的修复」，append-only（原句保留） |
| 3c | R3-LOW-3：`debug_assert` 生效面四处说明 | **✗ 四处在位但漏了最承重的第五处，且登记句不实** | 四处逐一核实在位：`bsp_bridge.rs:37`、ADR`:68-69`、ADR`:203-206`、`CONTEXT.md:170` ✓。**但** `find_episode` 函数自身文档（`:300-308`）既无生效域说明，还写着「不静默择一」——release 下它**正是**静默择一（→ F-1） |
| 4 | 回归面 | **✓ 全绿** | 见 §四 |
| 5 | ADR-0008 六轮 supersede 链完整性 | **✓ 结构完整**（一处轻微不齐，非名实问题） | 见 §五 |

---

## 二、FAIL 级发现（2 条，均 CONFIRMED，均零功能影响）

### F-1 [名实不符 + 新引入不实陈述] `find_episode` 函数文档说「不静默择一」——release 下它就是静默择一；R3-LOW-3 的四处补齐漏了这一处，而 ADR 第六轮反过来声称此处「此前已写明」

**实现事实**（`rust/src/theta_v0/classifier/bsp_bridge.rs:309-332`）：

```rust
let first = hits.next()?;
debug_assert!(hits.next().is_none(), "episode 归属应唯一：…需上报");
Some(first)
```

release profile 下 `debug_assert!` 编译为空操作 ⟹ 归属不唯一时**直接返回遍历序首个**，无 panic、
无日志、无返回值区分。而三窗验收 bin 全部在 release 跑（本轮亦然，见 §四）。

**文书事实**（同文件 `:300-308`，`find_episode` 自身的 doc comment，全文未被本轮触碰）：

> `debug_assert` 机器化「episode 归属唯一」（…）——若失守，说明该不变量在新数据上不再成立，
> 按 dispatch「若仍有撞键，停手上报」处置，**不静默择一**。

这句在 release 下与实现直接相反。它还是全仓**唯一**一处把「不静默择一」写成本函数性质的地方——
模块头 `:38` 已经明写「release 下 `find_episode` 静默取遍历序首个 episode」，两句在同一个文件里
互相打脸；`:300-308` 又是读者查这个函数时第一眼看到的地方，比模块头更承重。

**更重的一层**：ADR 第六轮 `:232-234`（本收口轮新写）登记：

> R3-LOW-3：`find_episode` 的 `debug_assert` 机器保证生效域（仅 debug profile，release 为空操作）
> **此前只在函数自身文档写明**，模块头、本 ADR 第三轮裁定 1、第五轮 Consequences、`CONTEXT.md`
> 四处均补齐生效域说明。

`grep -rn "空操作|debug profile|debug_assertions"` 全仓实测：生效域说明此前只出现在
`bsp_bridge/tests.rs:538` 与 `:575` 两条**测试注释**里（「debug_assert! 在 release profile 编译为
空操作——本测试只锁 debug 臂」），**函数自身文档从未写过，现在也没有**。即：收口轮据以「不必补
第五处」的那条理由，本身是一条不实陈述，且它就写在专门用来消除不实陈述的第六轮 supersede 段里。

判定 **CONFIRMED**。零功能影响（当前数据 `owned_many=0`，三窗两路独立复现）。
处置（本报告不动手）：`:304` 那句加生效域限定或删除线订正；ADR `:233` 的「此前只在函数自身文档
写明」订正为「此前只在 `bsp_bridge/tests.rs` 两条测试注释写明」。

### F-2 [名实不符] 一类路径的遍历形态：`CONTEXT.md:134` 与 `bsp_bridge.rs:54` 仍写「遍历 Trend episode，对每个 episode 回挂其区间覆盖的**全部**一类点」——第五轮 supersede 已把它改掉

**实现事实**（`resolve_first_class_episode_points:465-500`）：外层是 **BSP 点循环**
（`for level → for point → for (buy1, sell1)`），每个点调一次 `find_episode` 反查其所属 episode，
**一个点只落一个 episode**（多归属时 debug panic / release 取首个）。函数自己的 doc（`:459-464`）
写得很清楚：「按物理点遍历（第五轮 supersede，修复 #670 R2-HIGH-3：**旧实现按 episode 外层遍历、
点内层过滤**…）」。

**文书事实**——两处仍是旧形态：

| 位置 | 原话 |
|---|---|
| `CONTEXT.md:134-135`（正本词条） | 「**一类**走事件侧驱动：遍历 Trend episode，回挂其区间 `[c_start, interval.1]` 覆盖的**全部**一类点」 |
| `bsp_bridge.rs:54-55`（模块头「双向产出」段） | 「**事件侧 → 一类点**（[`resolve_first_class_episode_points`]）：遍历 Trend 候选事件，对每个 episode 回挂其区间 `[c_start, interval.1]` 覆盖的**全部**一类点」 |

两处描述的正是 R2-HIGH-3 点名、第五轮 supersede 删掉、三审 poison1 用来证明修复为真的那个
「episode 外层内联遍历」——把已判为缺陷、已修掉的实现形态写成现行设计，与 R3-MED-1 最重那一处
（`heads()` 只见链头）**完全同型**。

语义差不是措辞问题：「回挂**全部**一类点」蕴含「一个点落进两个 episode 就产两条边」，而现行实现
在同一情形下只产一条（release 静默、debug panic）。这恰恰是 R2-HIGH-3 的整个内容。

同一个 `bsp_bridge.rs` 里，`:36-45`（订正段）和 `:459-464`（函数 doc）都写了新形态，只有 `:54`
这条 bullet 冻结在旧轮；`CONTEXT.md` 本轮专门做了 append-only 四处订正，独独漏了紧邻的这一句。

ADR `:70` 同款措辞**不算**（append-only 纪律：那是第三轮裁定原文，第五轮 §1 已显式记录改为逐点
调用 `find_episode`，历史裁定原句本应保留）。

判定 **CONFIRMED**。零功能影响（`owned_many=0`，两条描述在当前数据上产出同一结果集）。

---

## 三、从属观察（不单独构成 FAIL，建议同一次补丁一并扫掉）

1. **`CONTEXT.md`「已知残留」只列三类，漏列二类**。词条写「**已知残留**：三类近零覆盖（…归 #688）」，
   而二类实测是 **0/280 恒判 `None`**（本轮复现：battery bin 三窗 Buy2/Sell2 `reference=0 produced=0`；
   truth bin 三窗 `anchor_hosts_first_class=0`，携锚率却是 17/17、88/88、280/280）——比三类的「近零」
   更彻底。R3-LOW-1/LOW-2 要求的正是把这条在 battery bin 与 ADR 两处**原处**标注；正本词条这一处
   没扫到。词条里「二类=继承一类锚所属 episode，同样按区间覆盖」这句本身**与代码相符**（`resolve_bridge`
   二类分支确实这么写），所以不判 F 级，但「已知残留」的清单漏项会让读者反推「二类正常」。
2. **`CONTEXT.md` 两词条的 provenance 仍写「三轮 supersede」**，ADR 现已到第六轮。
3. **ADR 第六轮段体例与前三轮不齐**：第三/四/五轮各有 `### Consequences（第 N 轮更新）` 子节且
   段标题里引了各自的修复轮报告路径；第六轮两者都没有（正文引了三审报告，但没引自己的收口报告
   `chanlun/review-results/issue668-n4-pwc-closeout-20260729.md`）。纯体例，非名实问题。

---

## 四、复跑读数（全部本工位实测，`/tmp/wt-668c`，`CARGO_TARGET_DIR=/tmp/wt668c-target`）

### 防「重放缓存假绿」（进场第一件事）

缓存 release 二进制 mtime = **07-29 21:53:55**，而本轮源改动
`p_issue668_bsp_bridge_battery.rs` = **22:24:38**、`bsp_bridge.rs` = **22:22:41** ⟹ 缓存**早于**源，
读它就是假绿。强制 `cargo build --release --bins` 重建（exit=0），新二进制 mtime **22:40:26**、
`p92_nest_replay_postruling` 亦 22:40:27（全 bin 重链，确认 lib 真重编）。**本报告全部 bin 读数
出自重建后的二进制。**

### 回归 / 零 diff / 引用面

```
cargo test --lib                    → 2569 passed; 0 failed; 138 ignored   （基线 2569 ✓）
cargo test --lib --release          → 2565 passed; 0 failed; 138 ignored   （基线 2565 ✓）
cargo test --lib bsp_bridge         → 19 passed; 0 failed                  （三审 19 ✓）
cargo check --all-targets           → Finished（exit=0，仅既有 warning，无 error）
cargo check --bin p_issue668_bsp_bridge_battery（touch 后强制重查）→ 该 bin 零 warning
cargo test --release --test theta_v0_lean_parity --test theta_v0_buy_parity → 10 + 1 = 11 passed; 0 failed

git status --porcelain                                        → 空（评审期零改动）
git diff --stat 1b7ba1a1a5..HEAD                              → 16 文件 / 4687+ / 0-（纯新增）
git diff --stat 1b7ba1a1a5..HEAD -- backtest/ p92* rust/tests/
      cand_event.rs bsp.rs signal.rs types.rs                 → 空（p92/runner/golden/老对象零 diff ✓）
git diff 1b7ba1a1a5..HEAD -- classifier/mod.rs                → 仅 +3 行（模块声明 + 注释）
全仓 bsp_bridge 生产引用面                                     → classifier/mod.rs:107 一处（零消费接线 ✓）
```

**「生产逻辑零改动」核实属实**：`git show 152872dfde -- bsp_bridge.rs` 全部改动 = `+3/-1` 行，位于
模块头 `//!` doc comment（补 R3-LOW-3 生效域说明），无任何语句/签名/调用点变更。

### 对拍电池（release，三窗）

```
bars=20000  trend=0  pan=41  b1=0  b2=17  b3=37  hit_by_class={}
            IDEMPOTENCE d1=0 d2=0 d3=0  edges 0→0→0    BATTERY ref=0  prod=0  heads=0  cmp=0
            BY_CLASS 6/6 类「此类空域，cmp无信息量」     QUERY distinct=0  reachable=0  lost=0   exit=0
bars=100000 trend=7  pan=227 b1=3  b2=88  b3=257 hit_by_class={Buy1:1, Sell1:2} b1_hit_by_level={0:3}
            IDEMPOTENCE d1=3 d2=0 d3=0  edges 3→3→3    BATTERY ref=3  prod=3  heads=3  cmp=0
            BY_CLASS Buy1 1/1、Sell1 2/2，4 类空域       QUERY distinct=3  reachable=3  lost=0   exit=0
bars=300000 trend=42 pan=698 b1=29 b2=280 b3=890 hit_by_class={Buy1:9, Sell1:20} b1_hit_by_level={0:21,1:8}
            IDEMPOTENCE d1=22 d2=0 d3=0 edges 22→22→22  BATTERY ref=29 prod=29 heads=22 cmp=0
            BY_CLASS Buy1 9/9、Sell1 20/20，4 类空域     QUERY distinct=29 reachable=29 lost=0  exit=0
```

与三审报告 §四 **逐位一致** ✓（含 300k `heads=22 / ref=prod=29` 的折叠语义差）。三类判据迁移后
`Buy3/Sell3` 三窗仍两侧同为空集 —— 与收口报告「照实报数，未虚报覆盖变化」一致 ✓。

### 真值表（release，三窗）

```
bars=20000  levels=3 bit_instances=54   fingerprint_unresolved=17  owner_query_unresolved=0
            domain_size=0  owned_zero=0 owned_one=0  owned_many=0  DOMAIN_BY_CLASS {}
            MED2_ANCHOR_BREAKDOWN Buy2 with_type1anchor=10  hosts=0 / Sell2 7   hosts=0
            → EMPTY_DOMAIN  exit=2
bars=100000 levels=4 bit_instances=348  fingerprint_unresolved=88  owner_query_unresolved=0
            domain_size=3  owned_one=3  owned_many=0  DOMAIN_BY_CLASS {Buy1:1, Sell1:2}
            MED2_ANCHOR_BREAKDOWN Buy2 44 hosts=0 / Sell2 44 hosts=0        → PASS  exit=0
bars=300000 levels=5 bit_instances=1199 fingerprint_unresolved=280 owner_query_unresolved=0
            domain_size=29 owned_one=29 owned_many=0 DOMAIN_BY_CLASS {Buy1:9, Sell1:20}
            MED2_ANCHOR_BREAKDOWN Buy2 139 hosts=0 / Sell2 141 hosts=0      → PASS  exit=0
```

与三审报告 **逐位一致** ✓。

---

## 五、ADR-0008 六轮 supersede 链完整性

`grep -nE '^## |^### '`：

```
21  ## Considered Options
41  ## Consequences                       ← 唯一 H2（R2-LOW-3 处置成立 ✓）
53  ## 第三轮 supersede   → 82  ### Consequences（第三轮更新）
88  ## 第四轮 supersede   → 140 ### Consequences（第四轮更新）
151 ## 第五轮 supersede   → 199 ### Consequences（第五轮更新）
213 ## 第六轮 supersede   →（无 Consequences 子节，见 §三 观察 3）
```

- 链条闭合：正文（#666 四问四裁 + v1→v2 回炉）→ 第三轮（R1 FAIL）→ 第四轮（R2 两条新 HIGH）→
  第五轮（R2 遗留三条）→ 第六轮（R3 PWC 收口）。每轮 supersede 的**被撤销对象**都显式点名
  （第四轮裁定 2 撤销第三轮裁定 2 的三类例外；第四轮裁定 4 撤销第三轮替代验收物①；第五轮 §5/§6
  订正第四/一轮文书；第六轮 5 条对齐 R3 五条），无悬空撤回、无静默改写 ✓。
- 各轮引用的修复轮报告文件在仓内均存在（round1/2/3 + pwc-closeout + shadow-review1/2/3，共 9 份）✓。
- **第六轮 §5 含一条不实陈述**（F-1 第二层）——链条结构完整，但这一段的内容有假。

**#535 四条禁令**：①终态几何配首证钟（`make_revision:641-646` `prior` 优先，一次写不后移）不触发；
②回填 `first_provable_at`——N1 侧零 diff，不触发；③丢失 `Invalidated` 路径——三窗 `query_lost=0`
本轮复现，仍关闭；④回试段吞入父背驰段——三类锚仍 `[leave_interval, (retest_start, retest_start)]`
两段分列，不触发。

---

## 六、处置建议（不动手修，交编排者裁）

**补丁面 = 3 处文本 + 1 句订正，纯注释/文书，不涉代码逻辑、不涉判据、无需重跑 bin**：

1. **F-1**：`bsp_bridge.rs:304`「不静默择一」加生效域限定（或删除线 + 订正段，与本票已确立的
   append-only 纪律一致）；ADR `:233`「此前只在函数自身文档写明」订正为「此前只在
   `bsp_bridge/tests.rs:538/575` 两条测试注释写明」。
2. **F-2**：`CONTEXT.md:134` 与 `bsp_bridge.rs:54` 两处「遍历 Trend episode…回挂全部一类点」按
   append-only 订正为「逐 BSP 点调 `find_episode` 反查其所属 episode（一点一 episode），判据 =
   episode 区间覆盖」；ADR `:70` 属历史裁定原文，**不动**。
3. **从属**（§三）：`CONTEXT.md`「已知残留」补二类 0/280 恒判 `None`；provenance「三轮」→「六轮」；
   ADR 第六轮补 Consequences 子节与自身报告引用（体例，可选）。
4. **保留（本轮四度核验成立，不许推倒）**：一类/二类/三类三条路径共用 `find_episode`、载荷集合
   形态 + 分组折叠、集合语义查询入口、真两驱动平价锁及其判别力边界的如实登记、负控、参照集代码
   路径独立性（本轮再验：三类迁移后仍不复写生产）、真值表三分退出码、二类停手上报、分点类空域
   标注、零消费接线、老对象零改动、append-only 三只钟纪律。

不关票。
