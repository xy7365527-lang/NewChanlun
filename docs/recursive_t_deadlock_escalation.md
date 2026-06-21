# 矛盾上浮：区间套递归（连续下钻）vs flat（跳级路由）的架构冲突

> 触发规则：`no-workaround.md`（不允许绕过概念矛盾）。
> 认识论等级：L3（CL/DX/BTC 三标的真实数据，否定性结果）。
> 日期：2026-06-21。

## 1. 矛盾（精确描述：什么跟什么冲突）

**A — 编排者 2026-06-20 裁决（rec 区间套递归）**
- "非绝对 ladder，骑走势节点 TrendNode，sink 创造 **level=父-1** 子（连续递减），沿活跃链下钻，每级别检查自己级别 BSP"。
- 实装 `reconcile_recursive`（rec_driver.rs:141）：从 root(core_level) 下钻，sink 只在**活跃链末端**（无子那层）触发，条件 = `candidate@level + 次级别 type1_sell@(level-1)`。

**B — flat 引擎（编排者最新裁决"参考 flat 完成递归"）**
- `route_bsp`（t_engine.rs:589）：BSP@level j（任意 j）→ `nearest_active_parent(j)`（t_engine.rs:425，`(j+1..MAX).find(active)`）**跳级**找 core 父 → `sink(父, j)`（t_engine.rs:525，减父 1/3，建 **ladder j** 子）。
- 绝对 ladder 数组。低级别 BSP 频繁 → 跳级 sink core → **CL +120% 做空赚钱**（battle-tested）。

## 2. 冲突点（为什么不可弥合）

### 死锁（A 的结构性失效）
连续下钻要求第一次 sink 在 core_level（活跃链初始只有 root）。触发需 `candidate@core_level` = **最高涌现级别走势 c 段衰减**。但：
- core 持多骑牛（C1），core 走势是最高涌现走势（BTC L3/L4 跨数年），其 c 段衰减 = 接近**最终大顶** = 极罕见。
- `candidate@core_level=false`（绝大多数重跑）→ reconcile `break`（rec_driver.rs:175）→ 永不 sink → 永远无子 → 永远到不了低级别（candidate 频繁的级别）。**鸡生蛋死锁**。
- **实测（L3）**：CL/DX/BTC 三标的全 `sink=0`，`short_pnl=0`。BTC `candidate@core_level=108 ∩ 次级别 type1_sell fresh=220 = 0`。做空腿完全未激活。

### B 无死锁
低级别 BSP@j（频繁）经 `nearest_active_parent` **跳级**直接 sink core，不需要 core 自己回调。这是 flat CL+120% 的根因。

### 架构互斥
- A 的"连续 level=父-1"要求中间层都存在才能到达低级别（逐层建链）。
- B 的"跳级"直接跨过中间层（core → j）。
- A **明确否定** B 的绝对 ladder（2026-06-20："非绝对 ladder"；"三失效点=flat 模拟递归的绝对/相对错配产物"）。
- B 是 A 的死锁解药，但用的**正是 A 否定的绝对 ladder**。

### 编排者两个裁决自身冲突
- **2026-06-20**："非绝对 ladder" + 连续下钻区间套。
- **最新**："参考 flat 完成递归，缺的补"——但 flat 的有效机制 = 绝对 ladder 跳级（`nearest_active_parent`）+ `emergence_upgrade`（自下而上，t_engine.rs:491 编排者亲笔注释："补齐原引擎只有自上而下（区间套约束）的缺口"）——**正是 2026-06-20 否定的两样东西**。

## 3. 依据的定义

- memory `project_recursive_t_architecture_v2`（编排者 2026-06-20 裁决）。
- flat `t_engine.rs:425`（nearest_active_parent 跳级）+ `491-494`（emergence_upgrade 自下而上，编排者注释）。
- 缠论第 27 课区间套：高级别买卖点通过次级别走势**逐层**定位下钻到笔。
- C1（编排者 2026-06-20）：核心持多骑牛永不翻空。
- P3b 北极星：引擎消费 BSP（type1_sell）而非走势结构——`type1_sell` 即顶背驰**完整确认**（滞后到回调底），与 candidate（背驰候选，及时）时序错配（fresh 事件 ∩ candidate 状态）。

## 4. 接受 A 则 B / 接受 B 则 A

- **接受 A（纯区间套连续下钻）**：架构纯净（相对级别，无绝对 ladder 错配），符合 2026-06-20。但 `candidate@core_level` 罕见 → `sink=0` → 做空腿永不激活 → 退化为"持多骑牛单腿"（BTC +1423% > BH，但编排者要的双腿做空对冲**不存在**）。
- **接受 B（flat 跳级 + 自下而上）**：做空腿有效（CL+120% battle-tested）。但回到绝对 ladder（2026-06-20 否定的"绝对/相对错配三失效点"）。

## 5. 推荐（第三条路：调和，非只抛问题）

死锁根因**不是嵌套链架构**（child 链 OK），是 **candidate 投影级别错位** + **BSP 事件 vs 结构状态错配**：

- sink@core_level 用 `candidate@core_level`（core **整体**衰减，罕见）触发，但区间套的正确语义是"core 走势出现**当前回调**"——即 `extract_chain` 已计算的 `nodes[1]`（core 走势 last leg 反父向，映射 level core_level-1）。**当前回调频繁**（每次次级别下跌），死锁自解。
- 修复：reconcile **用 nodes 链**（已有的区间套当前回调链，rec_driver.rs:223 extract_chain 已投影）驱动 sink/recover——nodes[depth] 存在（该层有当前回调）+ 仓位链无对应子 → sink 建子（level=父-1，**保持连续，不跳级**）；nodes 回调消失 + 有子 → recover。
- candidate 降为**回调级别过滤**（区分次级别 move 回调 vs 笔噪声），保留编排者引入 candidate 的"及时性"本意（trend_candidate 注释 line 218："不需完整背驰确认"）。
- **这保留 A 的嵌套链 + 连续性（不跳级，符合"非绝对 ladder"），获得 B 的频繁启动（nodes 链 = 区间套逐层下钻视图），但部分回归走势结构驱动（偏离 P3b "消费 BSP" 北极星）。**

**需编排者裁决**：A（接受 sink=0 单腿）/ B（flat 跳级，回归绝对 ladder）/ C（nodes 链调和，推荐，但偏离 P3b 北极星）。

## 7. 编排者裁决（2026-06-21，RESOLVED）——第四方案 D：严格区间套级联

编排者否定 A/B/C，给出**第四方案 D：递归 type1 级联确认**：core_level AND candidate（结构∧MACD 双确认，过滤 ep5 假顶）+ 次级别 type1_sell + ... 递归到最低级别。**confirm = 最低级别 type1_sell**（完整背驰确认，但级别最低 → 确认最快 → 最接近顶部）。

关键洞察：confirm 不是 candidate 持续状态（C=workaround，confirm 在底），不是次级别第一证据（笔分型不严格），而是**最低级别完整 type1**——既严格又最快。**最低级别 type1_sell 确认快、频繁，在 cand_sell@core 状态期间 fire（交集>0）→ 解死锁（旧用 core-1 大走势 type1，∩candidate@core=0）+ 开空在顶部附近**。candidate 作级联中间环节"接近顶对齐"（状态，合法）≠ candidate 作 confirm（C，禁止）。

落码（89 单测绿，L3 回测中）：trend_candidate→AND；ChainView.cand_sell/cand_buy+sink_cascade/recover_cascade；reconcile 严格级联（sink 末端建一层 break，递归加深逐重跑涌现）；extract_chain 分方向投影。

残留风险（L3）：cand_sell@core_level（AND 比旧 MACD-only 更严，可能<0.27%）仍是 sink 前提——死锁是否真解取决于最低级别 type1_sell 能否在 cand_sell@core 稀疏窗口内 fire 且级联连续。sink>0=解除；sink≈0=级联连续性过严需复查。

## 8. 方案 D 实测否证（2026-06-21，L3）——C1 与"级联到 core_level"不可调和

**结果：sink=0（BTC/CL/DX 全标的全模式），做空腿未激活。方案 D 失败，死锁加剧。**

**per-level candidate（AND）+ type1_sell 分布（铁证）**：
```
BTC:  cand_sell=[256, 486, 390,  0,  0, 0]   t1sell=[1692, 792, 271, 142, 30, 0]
CL:   cand_sell=[392, 302, 726,  0,  0, 0]   t1sell=[2007, 994, 322,  50,  0, 0]
DX:   cand_sell=[158, 177,  60,  0,  0, 0]   t1sell=[ 861, 398, 148,  25,  0, 0]
              L0    L1    L2   L3  L4 L5
```
- **candidate@core_level=0**（所有标的）：core=最高涌现走势（BTC L4/L5），C1 持多骑牛 → 永不见顶 → AND candidate=0（旧 MACD-only=108，AND=0）。
- **candidate 频繁在次级别 L0/L1/L2（256-726）**，但 **L3/L4（高级别）=0**。
- type1_sell 全级别有但高级别稀疏（L3=142, L4=30）。

**不可调和矛盾（C1 vs 级联到 core）**：
- 方案 D 要求级联确认**到 core_level**（core AND candidate + 往下 type1）。但 candidate@core=0（C1 必然）→ 级联在 core 断。
- 即使去掉 core candidate 前提，级联从 core 往下第一步（core-1=L3）就断：L3 candidate=0 + type1 fresh 稀疏。**高级别（L3/L4）是级联断点**，candidate 频繁的次级别（L0/L1/L2）在断点**下方**。
- rec 嵌套链**连续下钻**（level=父-1）：core 无子 → 检查 sink@core → cand_sell@core=0 → break → 永不建子 → 到不了次级别。无论级联方向（自上而下/自下而上），都被高级别断点阻断。

**根因 = escalation §2 死锁的再现**：方案 D（confirm 用最低级别 type1，洞察正确）解决了 timing，但没解决"级联/下钻必须穿过高级别断点"。这是 rec 连续架构（level=父-1）vs flat 跳级（nearest_active_parent）的同一矛盾。

**裁决选项**：
- **A**（rec 连续 + 级联到 core）：sink=0（高级别断点不可逾越）。
- **B**（flat 跳级 nearest_active_parent）：低级别 type1_sell（L0-L2，candidate 频繁）直接跳级 sink core（跳过 L3 断点），CL+120% battle-tested。违反 2026-06-20"level=父-1 连续"。
- **D'（推荐微调）**：保留方案 D 核心洞察（最低级别 type1 confirm），但 **sink 在次级别（candidate 频繁层）跳级触发**——低级别 type1_sell 级联（L0-L2）驱动，nearest_active_parent 跳级 sink core（core 减仓建次级别子，子.level=type1 级别非 core-1）。**core 持多（C1）不参与级联确认**——core 永不见顶是正确的，不该 gate 做空（做空对冲的是回调=次级别，不是 core 大顶）。

当前改动（级联框架）保留在工作区，89 单测绿，sink=0，等裁决。

## 9. 穷尽验证（2026-06-21，4 角度反证）——rec 连续 + C1 → sink=0 结构性不可能

Workflow 穷尽验证（candidate 重定义 / 级联重定义 / core 级别管理 / 反证），**4 角度全部 `infeasible`，`has_rec_internal_solution=false`**。

**反证链（铁证）**：
1. reconcile 从 root(core) 下钻，core 无子 → 只能 sink@core 才能建子下钻。
2. sink@core 需 core 级别确认（cand_sell@core 或 type1@core）。
3. C1 → core=最高涌现 Up 走势，**进行中**（c 段未离开末中枢）→ candidate@core=0；**完成时**已 fire type1（转回调）→ candidate 窗口已过。"不存在进行中就背驰的状态"。
4. 高级别断点（L3 candidate=0 + type1 稀疏 142/30）不可逾越。
5. **次级别 cand_sell 也不交集**：L0/L1/L2 candidate（256/486/390）是它们各自**顺势腿接近顶**时；但 core 回调对冲时次级别是 **Down 走势**（cand_sell 需 Up）→ 不交集。

**结论：不是 candidate 定义问题（AND/MACD/Structural 都不解），是 C1（core 持多骑牛升最高涌现永不见顶）与"sink 必须 core 级别确认"的内在矛盾。** rec 连续架构（level=父-1 不跳级）下做空腿结构性不可能激活。

**三个出路（全部违反某条裁决）**：
- **(a) 放宽 candidate**（Structural/OR 替代 AND）：sink>0 但罕见 + 牺牲假顶过滤（违反"AND 过滤 ep5"）。且 core candidate 本质罕见，不一定解。
- **(b') 跳级嵌套链（推荐）**：保留 rec child 链（相对父子，**非绝对 ladder 数组**），但 sink 子.level=触发 type1_sell 的级别（跳级），非 cur.level-1。低级别 type1_sell（L0-L2 频繁 1692/792/271）经 nearest_active_parent 跳级 sink core。confirm=该级别完整 type1（方案 D 洞察保留）。**core 持多（C1）不参与级联确认**——core 永不见顶是正确的，不该 gate 做空（对冲的是回调=次级别，非 core 大顶）。融合 rec 嵌套链 + flat 跳级（CL+120%）+ 方案 D 最低 confirm，唯一改"连续→跳级"（对 2026-06-20 微调，仍非绝对 ladder）。违反"level=父-1 连续"。
- **(c) 放弃 C1**（core 可翻空）：违反 C1，且根翻空有效域⊂非上行已否证（[[project_t14_t5_root_flip_necessity]]）。

**需编排者裁决**：(a) / **(b') 跳级嵌套链（推荐）** / (c)。

## 10. 编排者最终裁决（2026-06-21，RESOLVED）——第五方案：对照 flat 完全递归化

编排者否定 A/B/C/D，给出**第五方案**：「对照 flat 引擎，把它的逻辑递归化。不要自创约束。flat 没有 C1（永不翻空），也没有 candidate gate——这些都是递归引擎自创的。删除所有自创约束（C1/candidate/连续 level=父-1/方向 gate），逐行对照 flat route_bsp 用 TInstance/TRoot 重新实装。flat 的 nearest_active_parent → 递归里找最近 active TInstance。flat 已验证 ⟹ 递归化=用递归架构表达 flat 已验证行为，不加不减。」

**根因诊断（为何前 5 个方案全失败）**：所有失败都源于**递归引擎自创的约束**（C1 永不翻空 + candidate gate），它们与"做空腿赚钱"目标矛盾。flat 引擎从来没有这些约束——它用 `route_bsp`（nearest_active_parent 分派 sink/recover/drain + 核心级 enter/ascend/flip）+ 三阶段会计，core **可翻空**（flip），无 candidate gate。死锁是自创约束的产物，不是架构必然。

**落码（rec_engine/rec_driver/rec_stream 重写）**：
- TInstance 按 level 索引（= flat Layer），单 campaign 三阶段在 TRoot（= flat），删 child 链/per-instance 三阶段/C1/candidate/方向 gate。
- TRoot 实现 flat 全套：route_bsp/sink/recover/drain/enter/ascend/emergence_upgrade/flip/clear_all/on_bar，nearest_active_parent 遍历 instances。
- extract_view 用 `tree.emergent_top()`（= flat，非自创"最高走势方向"）。
- MAX_LEVEL=8（= MAX_LADDER-BASE_LADDER，对照 flat ceiling）。
- rec_stream 每 bar 调 on_bar（= flat step 每 bar，强平每 bar）。

**结果（L3，rec vs flat 对照）**：
- **死锁解除**：sink=0（数周撞墙）→ 做空腿激活（sink 千次级，short_pnl 非0）。
- **bit-exact 复现 flat**：CL/Structural(+22.8%)、CL/And(+12.8%)、DX全(-1.0%)、BTC/And(-14.3%)。
- **残差（调试中）**：BTC/Structural(rec-14.9 vs flat+38.9)、BTC/Or、CL/Or——liq>0 + 高波动标的核心方向差异，隔离 Agent 对照 dump 定位中。

**矛盾化解**：第五方案证明前述"C1 vs 级联到 core"矛盾（§9）是**伪矛盾**——它建立在"C1 永不翻空"自创前提上。删除 C1，flat 的 flip（核心可翻空）+ route_bsp 让做空腿自然工作，无需级联/candidate。**编排者洞察：不要自创约束，flat 已验证的就照搬。** 73 单测绿。

## 6. 影响声明

- 本报告未改任何代码（诊断 + 矛盾上浮）。
- 涉及裁决：编排者 2026-06-20（区间套递归连续下钻）+ 最新（参考 flat）+ P3b 北极星（消费 BSP）。
- 裁决后影响模块：rec_driver.rs `reconcile_recursive`（sink/recover 触发逻辑）、rec_engine.rs `sink`（是否支持跳级 level）、extract_chain（candidate vs nodes 链投影角色）。
- 次级缺口（与本矛盾正交，裁决后补）：rec 无逐 bar 强平驱动层（flat step 边界算子）、campaign reset——sink=0 时不触发，激活后需补。
