# #466 第一阶段：重基身份漂移调研

- 日期：2026-07-27
- 拟交付路径：`.chanlun/review-results/rebase-identity-research-20260727.md`
- 执行口径：只读；未写入文件，未运行回测。
- 代码快照：`main@1d21934c8a470f8e92f39448cfe191f82a0286cc`
- 既有 dump：`/tmp/wt-414-dump/center_lifecycle.jsonl`，共 2223 行；其生成快照由既有报告记录为 `e45e5020a9`。

## 一句话结论

**[代码可证]** wf8 既有 dump 中可重建的 76 次 Rebased 触发尾全部发生了 `zd/zg` 变化，其中 71 次 `start_index` 不变、5 次三字段同时变化，故主因不是单独的 `start_index` 漂移，而是可变前沿重算改变核心边界；真正使挂起成为无主欠账的直接原因，是 `adopt()` 全量替换链且不产出旧身份到新身份的映射。**[未能判定]** 当前 dump 无重基前后整链，也无 8 条挂起实例的 CenterId，不能判定这 8 条各自能否保持。**[推断]** 应先做“稳定谱系 ID + 结构修订 ID”双层身份，重基时只按塔构造阶段产生的精确谱系证据原子过继；不能采用事后区间或 `(zg,zd)+邻接` 模糊匹配。

## 第一题：漂移机制

### 1. CenterId 的稳定边界

**[代码可证]**

`CenterId` 由三项构成：

- `start_index`：构造种子首个运动单元的 L0 起点。
- `zd`：初始三个运动单元低点的最大值。
- `zg`：初始三个运动单元高点的最小值。

`end_index/dd/gg` 被明确排除在身份之外，因为它们会随延伸变化。正常延伸只更新 `end_index/dd/gg`，冻结 `start_index/zd/zg`，因此：

- 单纯尾部延伸不会改变 CenterId。
- CenterId 变化说明发生了重新构造、重新切段或升级重切，不是正常延伸造成的。

锚：`rust/src/theta_v0/classifier/center_lifecycle.rs:145-174`、`rust/src/theta_v0/classifier/center.rs:120-135`、`rust/src/theta_v0/classifier/recursive_tower.rs:738-750`。

### 2. zd/zg 为什么变化

**[代码可证]**

前沿运动单元允许被原地改写。缓存检测到已扫描 `UnitRange` 的长度收缩或内容变化后，会截断/清空本级中枢和上层派生缓存，再从恢复点重新扫描。下层前沿变化还会级联重置上层，因为上层是有损投影，不能独立证明未受影响。

重新扫描时，`Center::build` 再次由三个当前运动单元计算：

- `zg = min(hi₁, hi₂, hi₃)`
- `zd = max(lo₁, lo₂, lo₃)`

因此即使扫描仍从同一个 `start_index` 开始，只要其中一个种子单元的 `lo/hi` 被前沿重算改写，`zd` 或 `zg` 就会变化。

既有 dump 与此机制一致：76 次触发尾中有 71 次 `start_index` 保持不变而核心边界变化。

锚：`rust/src/theta_v0/classifier/center.rs:120-135`、`rust/src/theta_v0/classifier/mod.rs:804-829`、`:1662-1668`、`:1709-1786`、`:1865-1906`、`:1919-1994`。

### 3. start_index 为什么变化

**[代码可证]**

窗口检测器在位置 `i` 尝试构造中心；构造失败就右移，成功后根据延伸终点继续推进。缓存 pop/recompute 后，如果运动单元切分、构造成功位置或升级重切边界改变，新的种子 `a` 可能来自不同的 `i`，于是 `start_index=a.start_index` 改变。

九段以上升级还会把一个延伸结果重切为多个中枢：它们共享原中心的 `zd/zg`，但各自采用不同切片起点的 `start_index`。这证明“同一 `(zd,zg)`”在结构上并不唯一。

锚：`rust/src/theta_v0/classifier/recursive_tower.rs:738-799`，尤其 `:761-790`；`rust/src/theta_v0/classifier/center.rs:210-230`、`:252-267`。

**[推断]**

升级重切是“核心相同、起点不同”的代码可行机制，但既有 76 次 Rebased 触发尾中没有观察到该类别；不能据此断言 wf8 从未在整链深处发生这种变化。

### 4. Rebased 如何被触发，身份又在哪里丢失

**[代码可证]**

`consume_chain()` 只执行 O(1) 尾部守卫：

- 新链缩短；或
- 新链旧长度位置的尾 CenterId 与已消费尾不同；

满足任一条件即调用 `adopt()` 并返回 `Rebased`。

该守卫明确承认看不到“链长度及尾身份均未变、但深前缀已变化”的情况，所以 76 是被尾部守卫识别出的重基数，不等于所有深前缀变化数。

`adopt()` 随后直接：

1. 用新链覆盖整个 `consumed`；
2. 把新尾设为 `alive`；
3. 把新倒数第二个设为 `prev`；
4. 不保留旧链，不返回 removed/added 集合，也不返回身份映射。

挂起表在收到 `Rebased` 后，仅用新链精确 CenterId 集合检查旧挂起键；不存在即删除并核销为 `RebaseVanished`。

所以应区分：

- 结构漂移根因：可变前沿重切和重推导。
- 无主欠账根因：`adopt()` 与挂起表之间没有连续身份协议。

锚：`rust/src/theta_v0/classifier/center_lifecycle.rs:463-490`、`:525-549`；`rust/src/theta_v0/strategy/center_oscillation_trade.rs:416-435`、`:605-644`；`rust/src/theta_v0/backtest/fill.rs:799-850`。

## 第二题：76 次重基的可保持比例

### 可复算口径

**[代码可证]**

`chain_sync` 只记录重基后的尾/前尾，没有整链快照。不过事件流中每次正常追加都有 `born`，因此可以顺序重建事件机在每次 Rebased 之前的已消费尾，再与该 Rebased 记录的新尾比较。

该口径回答的是：

> 76 次被尾部守卫识别的重基中，“旧消费尾 → 新链尾”的字段变化。

它不是整条旧链到新链的匹配结果，也不是那 8 条挂起实例的匹配结果。

### 复算结果

| 旧尾 → 新尾类别 | 次数 | 比例 |
|---|---:|---:|
| `zd/zg` 相同、仅 `start_index` 不同 | 0 | 0.0% |
| `zd/zg` 变化 | 76 | 100.0% |
| `zd/zg` 变化、`start_index` 不变 | 71 | 93.4% |
| `start_index/zd/zg` 全部变化 | 5 | 6.6% |

核心边界变化进一步分解：

| 字段变化 | 次数 |
|---|---:|
| 仅 `zd` 变化 | 36 |
| 仅 `zg` 变化 | 31 |
| `zd`、`zg` 均变化，起点不变 | 4 |
| 三字段全部变化 | 5 |
| 合计 | 76 |

按层级：

| 层级 | Rebased 数 | 核心变、起点不变 | 三字段均变 |
|---|---:|---:|---:|
| L0 | 17 | 14 | 3 |
| L1 | 43 | 41 | 2 |
| L2 | 15 | 15 | 0 |
| L3 | 1 | 1 | 0 |

另外，73 次属于链长位置上的尾身份替换，3 次属于链收缩。

样例：

- 同起点、`zd` 改变：旧尾见 `/tmp/wt-414-dump/center_lifecycle.jsonl:40`，新尾见 `:43`。
- 起点和核心同时改变：旧尾见 `:1696`，新尾见 `:1702`。
- 链收缩触发：`:1581-1583`。

### 能回答与不能回答的比例

**[代码可证]**

若严格按本题给出的可观察分类，对 76 个“触发尾”：

- 同 `(zg,zd)`、不同 `start_index`：`0/76`。
- `zg/zd` 也改变：`76/76`。

这足以否定“当前漂移主要只是 start_index 位移”。

**[未能判定]**

以下比例不能从现有产物得出：

1. 76 次重基的旧整链与新整链中，有多少非尾中心只是改变了 `start_index`。
2. 8 条 `RebaseVanished` 挂起各自对应哪一次重基、其旧 CenterId 是什么。
3. 这 8 条在新链中是否存在同一物理谱系的继任中心。
4. 8 条中可安全过继的准确比例。

原因是：

- `adopt()` 返回前已经覆盖旧链。
- `chain_sync` 只写新尾和前尾。
- `suspension_by_source` 只聚合到 `(level, side, label)`，不记录 bar、CenterId 或 rebase 序号。
- O(1) 守卫还可能遗漏深前缀变化。

### 最低成本取证方案

**[推断]**

不需要全量回测，也不需要逐 bar 输出完整链。最低成本是在发生 Rebased 时才记录：

1. `rebase_seq/bar/level`。
2. adopt 前后的 `removed CenterId`、`added CenterId` 集合。
3. 每条当时挂起的 `side/old CenterId`。
4. 塔构造阶段可提供的精确来源谱系或构造见证。
5. 映射结果枚举：`continued / split / genuinely_removed / ambiguous`。
6. 每次映射的唯一性和双射检查。

该输出只覆盖现有 76 次重基，数据量很小。若仍只记录几何字段，则只能量化候选，不能安全决定过继。

## 第三题：保持方案分析

### 语义边界

**[代码可证]**

#456 已裁定 `RebaseVanished` 为终态核销和工程警报桶；#292 F 禁止的是中心已经消失后，再按相似区间寻找对象复活。当前重基事务中，基于精确连续谱系同步迁移挂起，发生在终态核销之前，属于身份保持，不属于复活。

锚：`docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:222-230`。

### 方案对比表

| 方案 | 主要改动面 | 对既有数据的适配 | 风险与语义冲击 | 结论 |
|---|---|---|---|---|
| a. 确定性重推导 | `LevelCache`、前沿运动单元切分、递归塔恢复游标和中心构造 | 相同输入本来已确定性；现有漂移来自输入前沿被合法改写。要让旧 CenterId 不变，实际需冻结核心、延迟确认或重定义重切规则 | 极高。可能改变分段、中枢出生时点、级联结构及交易信号；仍不能覆盖合法重切产生的新修订 | **[推断] 不宜作为第一解。** 可另票减少非必要重基，但不能承担稳定身份协议 |
| b. `(zg,zd)+时间邻接` 映射表 | `ChainConsumed::Rebased` 返回值、`adopt()`、fill、挂起表、campaign 归因、dump 和测试 | 76/76 触发尾核心均变化，精确 `(zg,zd)` 匹配捕获为零；放宽容差后才可能匹配 | 高。相同核心可对应升级重切出的多个中心；时间邻接或容差匹配会越过 #292 F 边界 | **[代码可证] 原始判据不足；[推断] 不应实施模糊版本。** 映射表这个传输形状可以保留，但映射证据必须升级为精确谱系 |
| c. 挂起绑定 `(zg,zd,level)` | `CenterOscillationBook` 键、SuspensionBatch、CoverAssignment、CampaignBook、witness、序列化和测试 | 核心在 76/76 中变化，不能消除无主；同核心又不唯一 | 极高。会合并不同起点/代际的挂起，破坏“只能由来源中心覆盖”的现有约束 | **[代码可证] 不能解决当前样本；[推断] 排除** |
| d. 稳定谱系 ID + 结构修订 ID | 递归塔产生 `CenterLineageId`；中心保留现有 CenterId 作为修订指纹；Rebased 携带精确连续映射；挂起和 campaign 绑定谱系 | 可覆盖核心或起点发生修订但构造谱系连续的情况；真正删除、分裂或歧义可显式暴露 | 中高。需建立不可复用 ID、递归来源见证、分裂规则和原子迁移；但不改变 `zd/zg` 的结构计算语义 | **[推断] 推荐。** 实现形状借用 b 的映射表，但映射由塔的构造事务产生，不做事后几何猜测 |

### 为什么挂起表和 campaign 必须一起迁移

**[代码可证]**

当前不仅 `CenterOscillationBook.suspended` 以 CenterId 为键，`SuspensionBatch`、`CoverAssignment`、`SuspensionAttribution` 和 campaign 的来源约束也携带 CenterId。后续补仓明确禁止由外国中心覆盖。

因此，只重绑结构挂起表而不重绑 campaign 来源，会把同一笔挂起在后续阶段判成外国中心；这不是完整身份保持。

锚：`rust/src/theta_v0/strategy/oscillation_campaign.rs:159-208`、`:223-328`、`:567-569`、`:913-949`。

## 推荐

### 推荐顺序

**[推断]**

先做方案 d，但分两步落地：

1. **D0：先补重基事务观测和失败夹具。**  
   记录旧/新集合、挂起实例和构造见证，确定原 8 条分别属于连续、分裂、真删除还是歧义。该步骤不改变交易语义。

2. **D1：建立双层身份。**
   - `CenterId(start_index,zd,zg)` 保留为当前结构修订指纹。
   - 新增不可复用的 `CenterLineageId` 作为挂起和 campaign 的业务身份。
   - 递归塔在 pop/recompute/upgrade 时直接输出旧谱系到新修订的精确变换。
   - Rebased 事务先原子迁移挂起和 campaign，再检查是否仍有无主项。
   - 一旦已经产生 `RebaseVanished` 终态，不允许以后补匹配复活。

3. **D2：再评估 a。**  
   若 D0 证明部分重基来自可消除的恢复游标不一致，可另票修复确定性/恢复等价；不能把它与身份协议混成一处大改。

不得直接把现有 `ElementId(level,ordinal)` 当稳定谱系 ID：既有报告已经观察到收缩后链索引被不同 `(start_index,zd,zg)` 重用。稳定 ID 必须不可复用，或携带明确的世代/来源谱系。

锚：`rust/src/theta_v0/classifier/recursive_tower.rs:60-88`；`.chanlun/review-results/center-event-machine-r3-chain-consumption-20260726.md:116-142`。

### 验收建议

**[推断]**

#### 单元与原型级

1. 同一谱系仅 `zd`、`zg` 修订：稳定 ID 不变，不产生 `RebaseVanished`。
2. 精确来源见证成立但 `start_index/zd/zg` 均修订：稳定 ID仍不变。
3. 升级重切的一对多情形：必须有显式规则；不得复制同一挂起到多个后继，也不得静默选择时间最近项。
4. 真删除或映射歧义：拒绝自动过继；输出逐项证据。
5. 同一次重基重复消费时幂等，不得二次迁移或重复核销。
6. 结构挂起表与 campaign 来源同时迁移，后续补仓仍通过“来源中心一致”检查。
7. 重基本身不得产生交易动作、成交或凭空改变数量账。

#### wf8 与多窗口靶向验证

每个验证产物必须同时给出：

- `rebase_events > 0`
- `suspended_at_rebase > 0`
- `identity_map_applied`
- `ambiguous`
- `non_injective`
- `dangling_after_rebase`
- `rebase_vanished`

核心验收式：

- `before_suspended = mapped + explicitly_removed`
- `dangling_after_rebase = 0`
- `ambiguous = 0`
- `non_injective = 0`
- wf8 原有四个格子的 `RebaseVanished` 全为 0：
  - L1 long：3 → 0
  - L1 short：3 → 0
  - L2 long：1 → 0
  - L2 short：1 → 0
- `suspension_by_source[label=RebaseVanished]` 总数：8 → 0。

如果存在真正结构撤销，不能通过改名或漏记把警报桶“做成 0”；必须先逐项证明它不属于身份连续，再按裁定流程决定是否需要新的终态来源。#466 的通过条件应是：身份丢失为零，而不是观测消失。

验证采用 wf8 和若干确实发生“重基时仍有挂起”的多窗口靶向样本即可；本票不要求全量重放。

## 未能判定清单

1. **[未能判定]** 原 8 条挂起的具体 CenterId、bar 与对应 rebase 序号。
2. **[未能判定]** 这 8 条中有多少是核心修订、起点修订、升级分裂或真正删除。
3. **[未能判定]** 76 次重基的整链 removed/added 比例；当前仅能重建触发尾。
4. **[未能判定]** O(1) 尾守卫遗漏了多少深前缀身份变化。
5. **[未能判定]** 现有塔节点是否已有足够的不可复用来源信息，可以直接生成稳定谱系，还是需要从 L0 开始新增世代 ID。
6. **[未能判定]** 升级重切一对多时，业务上的挂起继承对象；在该规则被正式裁定前不得自动选最近后继。
7. **[未能判定]** 现有 8 条是否全部可通过“同 start_index 的精确连续”覆盖；聚合 witness 没有实例级信息。

## 引用锚

- `docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:222-230`
- `rust/src/theta_v0/classifier/center_lifecycle.rs:132-174`
- `rust/src/theta_v0/classifier/center_lifecycle.rs:452-549`
- `rust/src/theta_v0/classifier/center.rs:120-135`
- `rust/src/theta_v0/classifier/center.rs:210-267`
- `rust/src/theta_v0/classifier/recursive_tower.rs:700-805`
- `rust/src/theta_v0/classifier/recursive_tower.rs:822-883`
- `rust/src/theta_v0/classifier/mod.rs:804-829`
- `rust/src/theta_v0/classifier/mod.rs:1662-1668`
- `rust/src/theta_v0/classifier/mod.rs:1709-1786`
- `rust/src/theta_v0/classifier/mod.rs:1865-1906`
- `rust/src/theta_v0/classifier/mod.rs:1919-1994`
- `rust/src/theta_v0/strategy/center_oscillation_trade.rs:416-435`
- `rust/src/theta_v0/strategy/center_oscillation_trade.rs:605-644`
- `rust/src/theta_v0/strategy/oscillation_campaign.rs:159-208`
- `rust/src/theta_v0/strategy/oscillation_campaign.rs:223-328`
- `rust/src/theta_v0/strategy/oscillation_campaign.rs:913-949`
- `rust/src/theta_v0/backtest/fill.rs:799-850`
- `rust/src/theta_v0/backtest/opsem_dump.rs:597-629`
- `.chanlun/review-results/final-verification-dual-arm-wf8-20260727.md:191-207`
- `.chanlun/review-results/final-verification-dual-arm-wf8-20260727.md:230-235`
- `.chanlun/review-results/superseded-anatomy-20260727.md:50-66`
- `.chanlun/review-results/center-event-machine-r3-chain-consumption-20260726.md:116-142`
- `.chanlun/review-results/center-tolerant-arena-window-20260726.md:79-85`
- `.chanlun/review-results/center-tolerant-arena-window-20260726.md:131-139`
- `/tmp/wt-414-dump/center_lifecycle.jsonl:40`
- `/tmp/wt-414-dump/center_lifecycle.jsonl:43`
- `/tmp/wt-414-dump/center_lifecycle.jsonl:1581-1583`
- `/tmp/wt-414-dump/center_lifecycle.jsonl:1696`
- `/tmp/wt-414-dump/center_lifecycle.jsonl:1702`
