# #466 D1 前置调研：重基丢身份机理与身份保持方案

- 日期：2026-07-28
- 票据：[#466](https://github.com/xy7365527-lang/NewChanlun/issues/466)
- 仓库基线：`main @ dc3a0c4acedef2916d2f75ee09a35be3b2daa770`
- D0 落地主线祖先：`e294c5b85a`（已核 `git merge-base --is-ancestor e294c5b85a HEAD` 返回 0）
- 性质：**纯调研，不含实装，不含提交**
- 090 口径：把“现行生产流可证”“临时探针可证”“方案推演”“未能判定”分开；临时探针结论不冒充现行生产契约。

## 0. 结论先行

1. **现行三流为什么丢身份**：`CenterId=(start_index, zd, zg)` 是当次前三个构造单元的派生值；开放 frontier 每 bar 会被 pop 后重扫。只要第三个源单元的外缘修订，`zd/zg` 就可能改变；若旧三单元交集失效，扫描起点右移，`start_index` 也随种子窗重切而改变。生命周期层随后以新链全量 `adopt`，挂起簿只做 `CenterId` 精确集合核对，旧三元组即被报作 `RebaseVanished`。
2. **八条实例归因**：临时只读构造探针显示，seq `3/20/24/47/53/55/56` 的旧、新父对象有相同有序前三 source ID，且第三源的 `CenterId` 不变、仅外缘/终点修订，属于“同一种子窗的 frontier 修订”连续候选；seq `66` 的种子从 `#494,#495,#496` 右移为 `#495,#496,#497`，属于旧构造窗撤出、新构造窗建立，不是分裂。
3. **严格区分两个计数**：现行生产三流没有构造谱系证书，按 D0 §6 目前仍是 **0/8 可合法自动过继**；若把本报告的最小 seam 做成生产契约并通过机检，则预计 **7/8 可合法过继，1/8 必须核销**。
4. **推荐方案**：构造层只产 `RebaseTransformTxn` 可复核事务证书；生命周期层维护 `LineageBook`，把稳定 `CenterLineageId` 与可变 `CenterId` 修订分开；接线层在无主核对前原子迁移挂起及 campaign owner。挂起首次绑定的四边框仍冻结，不能被新修订框覆盖。
5. **关于“桶到 0”**：不改终结来源分类时，诚实预期是 `RebaseVanished 8→1`，seq=66 走既有核销，并满足票面“剩余非零逐条归因”的备选验收。若坚持字面 0，须另行裁定把“构造谱系明确终止”单列为 `LineageEnded/ConstructionRemoved`（清算仍为 `WriteOffUnclosed`）；本调研不授权静默改桶名来制造 0。

## 1. 权威输入、现场与方法

### 1.1 D0 与现行教义

- D0 明确没有实施 D1，也没有改变 adopt、挂起、核销、动作或订单路径：`.chanlun/review-results/rebase-observability-d0-20260728.md:1-29`。
- D0 八条表及既有上限：同 start 只能是提示，不是谱系；seq=66 连同 start/core 提示都没有：同报告 `:96-118`。
- D0 允许过继仅接受构造层稳定边，且要求一对一、唯一、函数性、单射、双射；分裂、真删除、歧义均拒绝自动过继：同报告 `:203-221`。
- D0 已登记四项缺口：`stable_lineage_id / source_unit_ids / rebase_transform_edges / split_merge_generation_witness`：同报告 `:223-232`。
- ADR 补充十一规定 Superseded 不是中枢之死，挂起继续等待原中枢三类点：`docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:213-220`。
- ADR 补充十三把中枢定义为四边框；`RebaseVanished` 保持核销终态并定位为工程丢身份警报：同 ADR `:228-237`。
- ADR 补充十四把破坏、更替、延伸分开，升级/扩展不许伪造破坏；更替后的清算锚仍是冻结旧框：同 ADR `:241-246`。
- ADR 补充十五允许同一事件对多个合法 owner 分别投递，但每个 owner 的框仍各自冻结：同 ADR `:248-251`。

### 1.2 新鲜 #489 态现场

现场目录：`/tmp/main489-on.I7S288/`。

| 文件 | 行数 | SHA-256 |
|---|---:|---|
| `center_lifecycle.jsonl` | 2542 | `083fa44978265412147524d7edce00f408e5d38861848b211f2ad8537e6b6dc6` |
| `rebase_observability.jsonl` | 76 | `a4223a17b7ff8629919fb08bc94466acbcdbc8c120217a0762d6d8c8cbe10f83` |
| `tower_events.jsonl` | 3183 | `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |
| `trades.jsonl` | 779 | `e10a51dc5d2f4d7ef14d3730b4951280270f8e24f80ce7b4b53ed176c50584ec` |

`rebase_observability` 的 76 个 `rebase_seq` 与 D0 表一致，目标八条仍在 seq `3/20/24/47/53/55/56/66`。D0 报告记录的是先前转储指纹，本报告采用上表新鲜 #489 态指纹，不把两组哈希混写成相同文件。

### 1.3 临时构造探针

现行 `tower_events` 不能直接当谱系证书：它按向量下标对齐旧、新 `LeveledMove`，同下标只要 `end_index` 改变就记 `extend`，没有比较 `start/zd/zg/sub_moves`；代码见 `rust/src/theta_v0/backtest/opsem_dump.rs:661-754`。seq=66 正好证明同 ordinal/同下标可以承载完全不同的种子窗。

为回答构造层实际发生了什么，建立了独立 detached worktree：

- 工位：`/tmp/research-466d1/wt`
- 独立 target：`/tmp/research-466d1/target`
- 唯一临时改动：`rust/src/theta_v0/backtest/opsem_dump.rs` 增加 148 行 test-only 诊断；
- **没有修改** `coverage.rs / persistent.rs / center.rs / recursive_tower.rs`，也没有修改主仓任何 Rust 文件；
- 原始探针日志：`/tmp/research-466d1/probe-run.log`；
- 交叉核验脚本：`/tmp/research-466d1/analyze_rebases.py`。

同一 wf8 测试以独立 `CARGO_TARGET_DIR` 运行，结果为 `1 passed; 0 failed; 2241 filtered out; 17.03s`，证据在探针日志 `:426-444`。八条探针分别在该日志 `:430-436,438`；额外 start 漂移演示 seq=59 在 `:437`。分析脚本对八条、三流目标行、源序列和 7/1 计数做了断言，运行结果为：

```text
连续候选=7，真删除/新建=1；断言通过。
```

`trades.jsonl` 只在目标 bar 的一部分有成交记录，且没有 `CenterId`、构造窗或谱系边；它可证明同 bar 有交易活动，不能裁决中心身份，故不作为连续/删除分类依据。

## 2. 问题一：八条歧义案例逐条归因

### 2.1 证据链读法

每一行按以下顺序交叉：

1. `rebase_observability`：旧、新完整 `CenterId`、挂起侧、现行分类；
2. `tower_events`：同 bar 各级输出变化，但其 `extend` 只作“同下标变值”现场；
3. `center_lifecycle`：目标生命周期级确实在该 bar 触发 `chain_sync/rebase`；
4. 临时探针：pop 前旧父走势与重扫后新父走势的有序 `direct_sources`、源 `ElementId`、源框和父框。

### 2.2 八条全表

| seq / bar / L / side | 三流现场行 | 构造探针（旧源 → 新源） | 本次调研归因 | D1 处置 |
|---|---|---|---|---|
| `3 / 27124 / L2 / Long` | rebase `:3`；tower `:337-339`；lifecycle `:220` | `#12,#13,#14 → #12,#13,#14`；前两源逐值相等，#14 的 source ID/CenterId 相同、外缘/终点修订；probe `:430` | 同一种子窗重算，父 `zd` 漂移；**连续候选，非分裂** | 有生产构造证书后可 1→1 过继 |
| `20 / 55967 / L2 / Short` | rebase `:20`；tower `:727-729`；lifecycle `:485` | `#27,#28,#29 → #27,#28,#29`；前两源逐值相等，#29 frontier 修订；probe `:431` | 同一种子窗重算，父 `zd` 漂移；**连续候选，非分裂** | 同上 |
| `24 / 85532 / L1 / Long` | rebase `:24`；tower `:1050-1053`；lifecycle `:761-766`，rebase 在 `:765` | `#176,#177,#178 → #176,#177,#178,#179,#180`；前三源身份序列不变，#178 frontier 修订，随后吸收两源；probe `:432` | 同一 seed 的延伸窗从 3 源增至 5 源；阈值未到 9，仍只产一个父对象；**连续候选，非分裂** | 同上 |
| `47 / 170041 / L1 / Long` | rebase `:47`；tower `:2086-2087`；lifecycle `:1616` | `#357,#358,#359 → #357,#358,#359`；前两源逐值相等，#359 frontier 修订；probe `:433` | 同一种子窗重算，父 `zg` 漂移；**连续候选，非分裂** | 同上 |
| `53 / 193252 / L1 / Short` | rebase `:53`；tower `:2299-2300`；lifecycle `:1815` | `#395,#396,#397 → #395,#396,#397`；前两源逐值相等，#397 frontier 修订；probe `:434` | 同一种子窗重算，父 `zg` 漂移；**连续候选，非分裂** | 同上 |
| `55 / 193486 / L1 / Short` | rebase `:55`；tower `:2305-2308`；lifecycle `:1818-1822`，rebase 在 `:1822` | `#395,#396,#397 → #395,#396,#397,#398,#399`；前三源身份序列不变，#397 修订后吸收两源；probe `:435` | 同一 seed 的 3→5 源延伸，仍低于九段重切阈值；**连续候选，非分裂** | 同上 |
| `56 / 200401 / L1 / Short` | rebase `:56`；tower `:2374-2375`；lifecycle `:1892-1893`，rebase 在 `:1892` | `#409,#410,#411 → #409,#410,#411`；前两源逐值相等，#411 frontier 修订；probe `:436` | 同一种子窗重算，父 `zg` 漂移；**连续候选，非分裂** | 同上 |
| `66 / 240310 / L1 / Long` | rebase `:66`；tower `:2897-2900`；lifecycle `:2289-2294`，rebase 在 `:2293` | `#494,#495,#496 → #495,#496,#497,#498`；旧第一 seed #494 被排除，窗口右移；probe `:438` | **旧构造窗撤出 + 新构造窗建立**；一旧一新，不是 1→N 分裂；同 ordinal `#121` 只是位置复用 | 拒绝过继，保持核销 |

上表的 `:行号` 分别指：

- `/tmp/main489-on.I7S288/rebase_observability.jsonl`
- `/tmp/main489-on.I7S288/tower_events.jsonl`
- `/tmp/main489-on.I7S288/center_lifecycle.jsonl`
- `/tmp/research-466d1/probe-run.log`

### 2.3 seq=66 的承重判断

旧父窗口的前三源为：

```text
#494 [dd=4182110000000, gg=4317400000000]
#495 [dd=4291997000000, gg=4378735000000]
#496 [dd=4311413000000, gg=4388236000000]
=> start=238178, zd=4311413000000, zg=4317400000000
```

重扫时 #496 的外缘收缩为 `[4321576000000,4347000000000]`。于是旧起点三元窗的交集变为：

```text
max(dd494, dd495, dd496_new) = 4321576000000
min(gg494, gg495, gg496_new) = 4317400000000
```

前者大于后者，旧 seed 不再成立。扫描 `i += 1` 后，`#495,#496,#497` 成立：

```text
start=238612
zd=max(4291997000000,4321576000000,4311413000000)=4321576000000
zg=min(4378735000000,4347000000000,4388236000000)=4347000000000
```

#498 随后触及冻结核心而被吸收为延伸。这个过程与 `detect_centers_windowed_resume` 的“seed 失败右移 / seed 成立后延伸”逐字一致，见 `rust/src/theta_v0/classifier/recursive_tower.rs:738-798`。

特别要排除一个假证：旧、新父对象都显示 `ElementId L2#121`。ordinal 是 pop 后按 `prefix_count+i` 重发的确定性位置号，见 `recursive_tower.rs:819-878`；它不是跨重切的稳定谱系。seq=66 是现成反例。

### 2.4 本题 090 边界

- **可判定**：七条同 seed frontier 修订；seq=66 旧窗撤出、新窗建立；八条均无 1→N 输出，因此本组没有分裂。
- **现行生产仍不可直接过继**：上述分类依赖临时探针暴露的 `direct_sources`。当前 `rebase_observability` 自报 `construction_witness.status=none`，所以在 D1 seam 落地前，生产决策仍须按 D0 视为歧义。
- **未能判定**：造成每个源单元 frontier 修订的最底层首次原因究竟是 parser 线段重分、下一级窗口重算还是二者级联；现有四流没有 `dirty_e / resume_start / old-new source transform`。

## 3. 问题二：start_index 与核心边界漂移机理

### 3.1 代码级路径

```text
parser frontier 末段可回退重算
  segment.rs:508-560
        │
        ▼
Segment → UnitRange(start/end/lo/hi)
  classifier/mod.rs:222-239
        │
        ▼
检测输入回缩或扫描域 frontier_mutated，传播 dirty_e
  classifier/mod.rs:1781-1824
        │
        ▼
保留安全前缀，失效可变后缀
  classifier/mod.rs:1825-1919
        │
        ▼
pop 末个开放构造整窗，从旧 win_start 重扫
  classifier/mod.rs:1937-2001
        │
        ▼
前三单元重新求 start / zd / zg；之后延伸只改 end / dd / gg
  center.rs:120-135,252-267
  recursive_tower.rs:738-790
        │
        ▼
新 tail 替换旧 tail，并投影给更高一级，可能继续级联
  classifier/mod.rs:2040-2065,2226-2260
        │
        ▼
CenterId 尾身份不同触发 Rebased；adopt 全量覆盖
  center_lifecycle.rs:487-595
        │
        ▼
挂起簿按 CenterId 精确集合删 absent，产 RebaseVanished
  center_oscillation_trade.rs:635-669
  backtest/fill.rs:897-978
```

关键公式：

- `start_index = seed[0].start_index`
- `zd = max(lo1,lo2,lo3)`
- `zg = min(hi1,hi2,hi3)`

见 `rust/src/theta_v0/classifier/center.rs:120-135,252-267`。普通延伸只在新单元触及冻结核心时更新 `end_index/dd/gg`，不更新 `start/zd/zg`，见 `recursive_tower.rs:741-750`。因此：

- 七条同 start 案例不是“延伸直接改 CenterId”，而是**先 pop、再以同 seed 重算**，源 frontier 外缘变值后父核心重新求交；
- seq=66 的 start 漂移发生在**seed 判定失败后的窗口起点右移**，不是挂起层、生命周期层或合并窗口事后改名；
- 九段升级是另一条可导致 1→N 的重切路径，代码在 `recursive_tower.rs:756-790`，但本八条的探针没有触发它。

### 3.2 数值演示 A：seq=66，seed 右移一位

上一节已逐数演算：

- 旧 `#494,#495,#496`：`238178/4311413000000/4317400000000`
- #496 frontier 外缘收缩后，旧三元交集为空；
- 新 `#495,#496,#497`：`238612/4321576000000/4347000000000`
- #498 仅作为延伸吸收。

现场链：

- lower tower：`tower_events.jsonl:2897-2899`
- parent 同下标变值：同文件 `:2900`
- lifecycle rebase：`center_lifecycle.jsonl:2293`
- direct sources：`probe-run.log:438`

### 3.3 数值演示 B：seq=59，seed 右移两位

seq=59 不属于八条核销样本，但同一新鲜 wf8 中给出了第二个 start 漂移实例：

- 旧源 `#414,#415,#416`，父为 `200900/4269983000000/4346171000000`
- 新源 `#416,#417,#418`，父为 `202536/4391001000000/4423309000000`
- 只有旧第三源 ordinal #416 在新窗第一位复用；没有任何逐值相等的 direct source edge。

证据：

- `rebase_observability.jsonl:59`
- `tower_events.jsonl:2409-2412`
- `center_lifecycle.jsonl:1921-1926`
- `probe-run.log:437`

这说明“输出 ordinal 相同”“链长 103→103”均不能证明连续；真正改变发生在开放 seed 窗的成员重切。

### 3.4 数值演示 C：seq=3，同 seed、仅核心漂移

旧、新前三源均为 `#12,#13,#14`。前两源逐值相同；第三源的自身核心身份仍是：

```text
start=24614, zd=2585233000000, zg=2595630000000
```

但其外缘下沿从 `2580000000000` 修订为 `2571704000000`。因此父核心：

```text
old zd=max(2545760000000,2556555000000,2580000000000)=2580000000000
new zd=max(2545760000000,2556555000000,2571704000000)=2571704000000
zg 均为 min(...)=2598750000000
```

即父 `CenterId` 变化，但构造 seed 没换。证据在 `probe-run.log:430`。

### 3.5 “前缀重切 / 单元重分段 / 合并窗口移动”裁定

| 层次 | 本次能否判定 | 结论 |
|---|---|---|
| 目标父中枢发生在哪一步 | 能 | 开放 frontier 整窗 pop 后的 seed 重新判定与重扫 |
| 七条同 start 的直接原因 | 能 | 同前三源中的 frontier 源外缘修订，父 `zd/zg` 重新求交 |
| seq=66 的直接原因 | 能 | 旧 seed 交集失效，扫描起点右移一位，随后建立新 seed 窗 |
| 下一级源为何修订 | 部分 | lower tower 的 frontier 回退/新建可见；例如 seq=66 的 #496 end 回退并出现 #497/#498 |
| 首个脏源来自 parser 还是更低级构造 | **未能判定** | 四流没有 dirty 源及跨级 transform；代码两条路径都存在，不能按可能性冒充本案事实 |
| “合并窗口移动”是否为八条原因 | 没有证据 | 现有算法是非重叠 seed 扫描 + 延伸/九段重切；八条探针无多旧窗合成一新窗证据 |

## 4. 问题三：不碰 #59 禁区的身份保持设计

这里的“不碰”含义是：**主身份状态、挂起簿和迁移逻辑不放进 #59 塔构造文件**。但若要求可复核而非几何猜测，构造层仍须另行协调一个只产事实的 seam，详见问题四。

### 4.1 方案对比

| 方案 | 依赖的见证输入 | 能诚实保证什么 | 改造面 | 对 D0 §6 |
|---|---|---|---|---|
| A. 生命周期层谱系表 | 每次重基的旧、新构造节点；有序源 lineage；构造 transform 边；左右度数；事务原因 | 对证书覆盖的边保证精确 1→1、函数性、单射、唯一；有删除/新建时是**部分双射**，不能谎称全事务 complete bijection | `center_lifecycle` 持 `LineageBook`；`fill` 把构造证书接入重基事务 | **满足但有条件**：没有构造 seam 时退回 ambiguous，不得猜 |
| B. 双层身份：`CenterLineageId + CenterId` | A 的 1→1 证书；token mint/继承规则；挂起首次绑定的冻结四边框 | token 跨结构修订稳定；`CenterId` 保留当前修订指纹；token 本身**不能生成谱系证据** | 生命周期对象、挂起 key、campaign owner、观测 schema；塔结构仍用 CenterId | **单独不满足**；由 A 供证后满足。禁止“先造 token 再凭 token 自证” |
| C. **推荐：构造事务证书 + 生命周期谱系簿 + 原子接线** | 原始 old/new 窗事实、递归源 lineage、transform 图、校验器版本；B 的双层 owner | 构造层事实与领域身份分离；可机检 1→1 / split / removed / created；挂起迁移与无主核对同事务原子化 | #59 只增加只读事实 seam；长期状态在生命周期；挂起/campaign 只在接线层迁移 | **完整满足**；不唯一、不单射、无证书即拒绝 |
| D. 几何/位置捷径：同 start、同 core、最近框、相同 ordinal、内容 hash | 现有三流即可 | 只能给候选，不能证明谱系；seq=66 已证 ordinal 可复用，七条已证 core 会漂移 | 改动小但越过 #292 F | **不满足，拒绝** |

### 4.2 推荐状态模型

建议把“经济 owner”与“结构修订”明确分开：

```text
CenterLineageId             稳定 owner token，只能由构造证书 mint/继承
CenterRevision {
    lineage_id,
    center_id,              当前 (start,zd,zg) 修订指纹
    full_center,            当前 start/end/zd/zg/dd/gg
    revision_no,
    construction_node_ref
}
SuspensionOwner {
    lineage_id,
    bound_frame,            首次绑定时冻结的四边框，不随重基修订
    side,
    ...
}
```

约束：

1. `CenterId` 不废弃，仍供结构链、诊断和修订指纹使用；不能让 lineage token 反过来掩盖结构变化。
2. 挂起清算判据继续使用首次绑定的 `bound_frame`。1→1 谱系迁移只更新“该 owner 仍在构造链上”的归属，不把旧框替换成新框；这满足 ADR 补充十三、十四。
3. `CenterLineageId` 不能从 `(start,zd,zg)`、价格容差或 ordinal 哈希出来，只能：
   - 新构造节点无入边：mint；
   - 唯一 1→1 `continued` 入边：继承；
   - 1→N / N→1 / 非唯一：不自动继承；
   - removed：终止。
4. 同一重基事务内，所有 `continued` 边须同时满足：
   - `left_degree=1`
   - `right_degree=1`
   - `functional=true`
   - `injective=true`
   - `unique=true`
   - `bijective=true`
   - 证书引用的 old/new revision 与生命周期收到的 before/after 链逐项一致。

### 4.3 原子接线顺序

当前路径是在 lifecycle 发现 rebase 后，直接让挂起簿按新链 `CenterId` 删除 absent，见：

- `rust/src/theta_v0/classifier/center_lifecycle.rs:498-535,580-590`
- `rust/src/theta_v0/backtest/fill.rs:943-978`
- `rust/src/theta_v0/strategy/center_oscillation_trade.rs:644-669`

推荐顺序：

1. classifier 交付同一 bar/level 的 `RebaseTransformTxn`；
2. lifecycle 对 before/after chain 与证书做一致性校验；
3. `LineageBook` 先应用唯一 1→1 边，产生 `old CenterId → lineage → new CenterId` 的修订迁移；
4. 接线层以 lineage 原子迁移挂起 key、campaign owner 和相关观测归属，保留冻结框；
5. 再对未获合法后继的 old lineage 做 removed/split/ambiguous 处置；
6. 最后断言：

```text
dangling_after_rebase == 0
ambiguous_transform == 0
non_injective_transform == 0
duplicate_lineage_claim == 0
```

若证书缺失、bar/level 不匹配或校验失败，fail closed：保持现有 `RebaseVanished` 核销并增加工程错误观测，不得 fail-open 过继。

### 4.4 为什么 C 优于“把 token 塞进 Center”

- `Center` 是结构结果，`CenterId` 是结构修订指纹；长期 owner token 属生命周期域。
- 塔构造仍可保持纯推导，只暴露事实，不持有挂起/campaign/交易状态。
- 证书可以离线重放审计，同一原始事务应得同一 transform 图。
- split/merge/remove/create 是图关系，不被强迫压成一个 token 字段。
- #59 的改动可收窄为 seam，不把策略语义扩散进 `center.rs` 或 `recursive_tower.rs`。

## 5. 问题四：若构造见证不可或缺，最小 seam 规格

### 5.1 不可或缺

结论：**不可或缺**。现行三流只能看见“旧链三元组消失、新链三元组出现”；`tower_events` 的同 ordinal `extend` 又会把 seq=66 的窗口替换伪装成延伸。没有 old/new 构造窗及有序源谱系，就无法复核：

- 同一 seed 的修订；
- seed 起点右移；
- 九段升级的 1→N；
- 多旧窗到多新窗的非唯一关系；
- source ordinal 复用是否只是位置复用。

### 5.2 seam 放置点

最小事实同时存在的地方是 classifier frontier 事务：

1. 旧开放整窗在 `classifier/mod.rs:1937-1977` 被 pop；现有代码已捕获 `popped_upper`，但会丢掉对应 old `WinMeta`，需在 truncate 前一并形成只读快照。
2. 新 tail 在 `classifier/mod.rs:1991-2001` 由 `compose_level_resume` 返回。
3. old/new 完整后、写回前的自然 emit 点在 `classifier/mod.rs:2040-2065`。
4. cascade 的 P=0/P>0 后缀失效还发生在 `classifier/mod.rs:1825-1919`；seam 必须覆盖这类被提前清掉的 old suffix，不能只看常态末窗 pop。
5. 九段一窗产 k 个子对象的事实在 `recursive_tower.rs:756-790`；需把同一父窗与 emitted=k 的关系带进事务。

### 5.3 最小字段

建议事件名：`RebaseTransformTxnV1`。

| 组 | 必需字段 | 用途 |
|---|---|---|
| 事务头 | `schema, txn_id, bar, level, cause, dirty_e, resume_start, prefix_count` | 将证书绑定到唯一重基上下文；区分常态 frontier、cascade、len-shrink、upgrade |
| old/new 节点 | `side(old/new), node_ref, output_ordinal, full_center(start/end/zd/zg/dd/gg), revision_digest` | ordinal 仅诊断；完整修订用于与生命周期 before/after 对账 |
| 窗口 | `win_start, win_exit, read_end_src, emitted` | 复核窗口边界、停止哨兵、一个窗产几个对象 |
| 源见证 | 有序 `direct_source_refs`；每源 `lineage_id, revision_ref, start/end, envelope, center_id`；单列前三 seed 与 absorbed tail | 判定同 seed 修订、seed 右移和延伸吸收；不能只给无序集合 |
| 递归链接 | `lower_txn_id/lower_edge_refs` | 证明“源 #496 是同 lineage 的修订”而非 ordinal 复用；终止自证循环 |
| 变换边 | `old_node_ref, new_node_ref, relation, basis, left_degree, right_degree, functional, injective, unique, bijective` | D0 §6 的直接机检输入；relation 至少含 continued_1to1/split/merge/removed/created/unknown |
| 校验 | `algorithm_version, source_order_digest, txn_digest` | 让离线 verifier 能重算并发现 schema/算法漂移 |

只有 `WinMeta` 不够：它有窗口索引和 emitted 数，没有源对象身份。只有 `ElementId` 也不够：seq=66 的 `L2#121` 已证明 pop/recompose 会复用位置号。只有 `sub_moves` 当前值也不够：还需 old/new 对照与下一级 transform，才能区分“同源修订”和“同 ordinal 替换”。

### 5.4 #59 领土声明

上述 emit 点、`WinMeta`、`LeveledMove/sub_moves`、九段重切都在塔构造链，属于 **#59 线领土**。本报告只给 seam 规格，**没有修改该区域**。进入实装前须与 #59 owner 协调：

- seam 只产构造事实，不接触挂起、campaign、清算或订单；
- 长期 `LineageBook`、token、原子迁移留在生命周期/接线层；
- 若 #59 不允许在 `LeveledMove` 增字段，可用与 `upper_moves/win_meta` 1:1 对齐的独立 sidecar，但证据字段不能删。

## 6. 问题五：方案评估与多窗验证量

### 6.1 八条处置

| seq | 生产 seam 落地后的预期 | 理由 |
|---:|---|---|
| 3 | 合法过继 | 同前三源 lineage；仅第三源 revision 及父核心修订；1 old→1 new |
| 20 | 合法过继 | 同上 |
| 24 | 合法过继 | 同前三 seed lineage；3→5 是 absorbed tail 增长，`n=5<9`，父输出仍 1→1 |
| 47 | 合法过继 | 同前三源 lineage；仅第三源 revision 及父核心修订 |
| 53 | 合法过继 | 同上 |
| 55 | 合法过继 | 同前三 seed lineage；3→5 absorbed tail，未触发九段重切 |
| 56 | 合法过继 | 同前三源 lineage；仅第三源 revision 及父核心修订 |
| 66 | **必须核销** | seed lineage 从 `494,495,496` 换为 `495,496,497`；旧第一 seed 撤出；没有 D0 允许的 1→1 连续边 |

合计：

- 临时研究归因：连续候选 `7`，真删除/新建 `1`，分裂 `0`；
- 当前生产契约下合法过继：`0/8`；
- 推荐 seam + verifier 落地并通过后：合法过继 `7/8`，必须核销 `1/8`。

### 6.2 RebaseVanished 目标

两种诚实验收口径：

1. **不改来源枚举**：`RebaseVanished 8→1`；seq=66 保持核销，报告本节即逐条归因。符合 #466 票面“跑到 0 或给出剩余非零的逐条归因”。
2. **经新裁定拆桶**：身份跟丢 `RebaseVanished=0`；seq=66 记 `LineageEnded/ConstructionRemoved=1`，清算仍是 `WriteOffUnclosed`。这是观测分类细化，不得在无裁定时偷偷实施。

本报告推荐先按第 1 种实施与验收；第 2 种只作为后续裁定选项。

### 6.3 wf8 之外的验证工作量

工作量判断：**中到大**。难点不在七条迁移，而在证明 split/cascade/多级递归不误过继。

建议最小验证束：

1. 纯构造/证书单测约 `12-16` 个聚焦场景：
   - 同 seed、第三源外缘修订；
   - 3→5 absorbed tail；
   - seed 右移一位、两位；
   - source ordinal 复用但 lineage 不同；
   - `n=8→9` 九段升级 1→N；
   - cascade P=0、P>0；
   - input len shrink；
   - 缺证书、重复 claim、非单射、非唯一、bar/level 不匹配；
   - old/new 顺序变化及事务摘要破坏。
2. `LineageBook` 与接线单测：
   - 1→1 token 继承；
   - split/remove 不迁移；
   - 同 side/跨 side 多挂起；
   - campaign owner 同步；
   - 冻结 `bound_frame` 不被新 revision 覆盖；
   - 迁移后 `dangling=0` 与重复 lineage claim=0。
3. wf8 靶向核验：
   - 先对八个 bar 周边做诊断级重放；若现有 harness 不支持 micro-range，则不要发明“已做靶向”，直接跑一轮完整 wf8；
   - 再跑完整 wf8 一次，核 `76` 次事务、`7` 继承、`1` removed、`ambiguous=0`、`non_injective=0`；
   - 与现 main #489 态交易/生命周期基线做差异面全枚举，预期差异只在七条挂起归属及其后续清算链。
4. wf8 外建议至少 `3-6` 个异质窗口：
   - 同 seed 高频边界修订窗；
   - 真 start 漂移窗；
   - 实际九段升级/分裂窗；
   - cascade/full-clear 窗；
   - 若能取得，再加 commodity/equity 等不同价格尺度窗。

目前没有给定这些外窗的清单、数据可用性及运行时，因此**未能诚实给出日历工期或总分钟数**。可确认的量级是两阶段实施（构造证书；谱系簿/接线）+ 上述测试束，而不是一次局部改 key。

## 7. 进入实装的门

建议**有条件进入实装**，顺序如下：

1. 先与 #59 owner 协调 D1a：只落 `RebaseTransformTxnV1` seam、离线 verifier 与观测，不迁移任何挂起；
2. 在新鲜 wf8 上复现本报告的 `7 continued_1to1 + 1 removed + 0 split + 0 ambiguous + 0 non_injective`；
3. 再做 D1b：生命周期 `LineageBook`、双层 owner、接线层原子迁移；
4. 默认验收 `RebaseVanished=1` 且 seq=66 精确归因；若产品目标必须字面为 0，先补“真删除单列来源”的裁定，再改观测分类。

不建议直接进入“按同 start 迁移”或“把 ordinal 当 lineage”的实现；两者都已被现场反例证伪。

## 8. 未能判定与风险登记

1. 八条 source frontier 修订的最底层首因（parser 重分 vs lower tower 重算）未能逐条判定；最小 seam 应补 `cause/dirty_e/lower_txn_refs` 后再回答。
2. 八条没有真实 1→N 分裂样本；九段升级规则可由代码推出，但挂起在分裂后的教义继承仍无裁定，故本方案一律拒绝自动复制。
3. seq=66 是否应在更高层教义上视为“跨 start 仍同经济中枢”没有现行裁定；D0 明令无明确跨 start 连续边即拒绝，本报告依现规则判 removed。
4. 多窗的实际连续/分裂/删除分布未跑，未能用 wf8 的 7/1 外推总体比例。
5. lineage sidecar 的 CPU/内存开销未测；必须在 D1a 观测阶段量化，不能凭字段少就声明零成本。
6. 现有 `tower_events.kind=extend` 是下标 diff 事件，不得继续被文档或测试当成身份延续证书。

## 9. 复核命令

八条交叉核验：

```bash
python3 /tmp/research-466d1/analyze_rebases.py
```

临时探针测试：

```bash
cd /tmp/research-466d1/wt/rust
M8_WIN_FILTER=wf8 \
VOICE_EXEC=1 \
THETA_CENTER_OSCILLATION=1 \
OPSEM_DUMP_DIR=/tmp/research-466d1/probe-dump \
CARGO_TARGET_DIR=/tmp/research-466d1/target \
cargo test --release --lib \
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
  -- --ignored --nocapture
```

报告未包含任何实现提交；临时探针与分析脚本均留在 `/tmp/research-466d1/`。
