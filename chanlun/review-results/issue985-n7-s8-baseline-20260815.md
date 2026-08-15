# #986 N7 E2E-S8 基线：consume_at 三窗真实数据验收（报告）

- 票：#986（map #529）；报告文件名按票面命名 `issue985`（票面原文「报告落 …/issue985-n7-s8-baseline-20260815.md」）。
- bin：`rust/src/bin/p985_s8_consume_baseline.rs`（新增，唯一代码改动）。
- 数据：`analysis/data_cache/btc_1m_full.json`（329MB，未提交仓）。
- 运行：debug 构建（`cargo run`；release 未跑——debug 一窗 100k 约 38s、300k 约 5.8min，已满足「一窗能出读数即可」，且读数与 release 无别——全整数/确定性判定链，无浮点平台差异）。

## 一、Seam 与认识论等级

```
btc_1m_full.json(+max_bars) → classify（逐 bar 因果通道）→ ChainCertificateBook(Closed 链)
  + BspBridgeBook(event_to_bsp 映射) → consume_at → 验收读数（Closed 链数/受管 BSP 数/
  BspLink 数/幂等/只收 Closed）
```

- **认识论等级 = L1**（管线正确性读数）：只验证「现役对象 → consume_at」接线与幂等语义成立，不声明 alpha、不评估消费逻辑市场有效性（L2/L3 归后续票）。
- **判定链零改**：lib（classifier/strategy/consume_at/consume_router/chain_cert/bsp_bridge）零改动，只经库公共 API（见下「三、实现要点」）。

## 二、读数表（三窗，advance_every=5000，复现 #641 三窗口径）

| 窗口 bars | 链总数 | Closed | Open | Invalidated | 桥接 heads | event_to_bsp | 与 Closed 链存活节点交集 | 受管 BSP | BspLink | 幂等 violations | 非 Closed 拒绝 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 20k | 18 | 18 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0（18/18） | 0/0 |
| 100k | 89 | 74 | 15 | 0 | 3 | 3 | 0 | 0 | 0 | 0（74/74） | 15/15 |
| 300k | 293 | 245 | 36 | 12 | 22 | 22 | 3 | **3** | **4** | 0（245/245） | 48/48 |

逐行原始输出（stdout，debug 实跑）：

```
20k:
P985_S8_CHAIN chains=18 closed=18 open=0 invalidated=0 revisions=18
P985_S8_BRIDGE revisions=0 heads=0 distinct_events=0 event_to_bsp=0
P985_S8_OVERLAP bridge_events=0 in_closed_any_node=0 in_closed_alive_node=0
P985_S8_CONSUME closed_chains=18 consumed=18 creations_total=0 managed_bsp=0 links_total=0 bsp_links=0 closed_errors={}
P985_S8_IDEMPOTENCE checked=18 violations=0
P985_S8_NON_CLOSED_REJECTED checked=0 rejected_no_consumption=0 unexpected=0

100k:
P985_S8_CHAIN chains=89 closed=74 open=15 invalidated=0 revisions=99
P985_S8_BRIDGE revisions=3 heads=3 distinct_events=3 event_to_bsp=3
P985_S8_OVERLAP bridge_events=3 in_closed_any_node=0 in_closed_alive_node=0
P985_S8_CONSUME closed_chains=74 consumed=74 creations_total=0 managed_bsp=0 links_total=0 bsp_links=0 closed_errors={}
P985_S8_IDEMPOTENCE checked=74 violations=0
P985_S8_NON_CLOSED_REJECTED checked=15 rejected_no_consumption=15 unexpected=0

300k:
P985_S8_CHAIN chains=293 closed=245 open=36 invalidated=12 revisions=959
P985_S8_BRIDGE revisions=23 heads=22 distinct_events=22 event_to_bsp=22
P985_S8_OVERLAP bridge_events=22 in_closed_any_node=3 in_closed_alive_node=3
P985_S8_CONSUME closed_chains=245 consumed=245 creations_total=3 managed_bsp=3 links_total=4 bsp_links=4 closed_errors={}
P985_S8_IDEMPOTENCE checked=245 violations=0
P985_S8_NON_CLOSED_REJECTED checked=48 rejected_no_consumption=48 unexpected=0
```

**四验收读数（以 300k 窗口，唯一非空缝合域）**：

1. **Closed 链数 = 245**（实际样本量，替代「66 张」；三窗链总数 18/89/293 与 #641 逐位一致）。
2. **受管 BSP 数 = 3**（= 唯一 `BspStructuralKey` 数；`creations_total=3` ⟹ 跨链只创建一次，无重复创建）。
3. **BspLink 数 = 4**（`links_total=4`；单 rule policy ⟹ 组内 1 链接/授权；3 BSP 产 4 链接 ⟹ **一条 BSP 被两条链授权**，正是 #980 残余规则「跨多条链只创建一次、多条链接写 BspLink 组内」的真实数据实例）。
4. **幂等**：245 条 Closed 链各连续 consume 两次，第二次零增量 violations=0。
5. **只收 Closed**：48 条非 Closed 链头（36 Open + 12 Invalidated）调 consume_at 全部 `Err(NoConsumption)`，unexpected=0。
6. **无 panic、无数值溢出**：debug 构建（整数 overflow 检查开启）三窗实跑无 panic；`closed_errors={}`（Closed 链 consume 零错误分支触发）。

## 三、实现要点（seam 与对齐）

- **候选事件**：`classifier::classify_with_tower_events_incremental`（`classifier/mod.rs:1910`）逐 bar 因果推进（同一 `ParseLayerIncr` + 同一 `TowerCache`）。票面步骤 2 写 `classify_with_tower_events`（`mod.rs:821`，p_issue668_bsp_bridge_battery.rs:317 用法）——本 bin 取**因果通道**，理由：票面步骤 3「逐 as_of 调 advance」要求流随 as_of 演化，终态窗口投影（`classify_with_tower_events`）会把全部 `closed_at` 钉在窗口末尾（p127 头注已登记该伪影）；因果通道末 bar 输出与 `classify_with_tower_events` bit-exact（#93 铁律），三元形状 `(classification, _tower, streams)` 不变。
- **链证书**：`ChainCertificateBook::default()` + 按 `advance_every=5000` 节拍逐 as_of `advance(&streams, as_of)`（`chain_cert/mod.rs:867`，末根必推，同 issue550/#641 口径），从 `heads()` 收集 `status == Closed`。
- **桥接边**：`BspBridgeBook::default()` + 同节拍逐 as_of `advance(&classification, &streams, as_of)`（`bsp_bridge.rs:661`），从 `edges()` 的 `BridgeKey{event, bsp}`（`bsp_bridge.rs:163`）投影 `event_to_bsp: HashMap<CandidateKey, BspStructuralKey>`（last-wins 有损函数投影）。
- **consume_at**：对每条 Closed 链按 `closed_at` 升序，`consume_at(as_of=closed_at, &prior_bsp, &prior_links, &cert, &policy, &event_to_bsp)`（`consume_at.rs:128`），`policy` = 单 rule 合法 policy（policy_id=1/rule_id=1/slot=0）。升序保证 prior `created_at/written_at ≤ as_of`，不触发 InconsistentState。

## 四、与「66 张 nest 口径」的对照说明

- 票面口径订正：「66 张历史证书」是 2026-07-28 charting 时 **nest 证书**（p105 口径 A=41/B=25）的代表数，**不是**塔内链证书。
- 本票消费的是**塔内 Closed 链证书**（`TowerChainCertificate`），样本以实跑 Closed 链数为准：三窗 **18 / 74 / 245**（链总数 18/89/293 与 #641 逐位一致）。
- 结论：consume_at 消费端的正确样本量是 **245（300k 窗 Closed 链）**，不是 66；「66 张」这个 nest 数字不得用作 E2E-S8 的样本量口径。

## 五、验收对照（两轴）

**Spec**：

| # | 验收项 | 结果 |
|---|---|---|
| 1 | `cargo check --features backtest_bin` 绿 | PASS（exit 0；bin 零警告） |
| 2 | 判定链零改：`git diff --name-only` 只应有 bin | PASS（tracked 改动为空；唯一新文件 `rust/src/bin/p985_s8_consume_baseline.rs`；lib 六模块零改） |
| 3 | bin 实跑一窗出读数（五项） | PASS（300k 窗：Closed=245 / 受管 BSP=3 / BspLink=4 / 幂等 0 violation / 只收 Closed 48/48） |
| 4 | 报告落本文件 + 读数表 + 66 张对照 | PASS（见 §二、§四） |

**Standards**：

| # | 验收项 | 结果 |
|---|---|---|
| 1 | `cargo fmt --check` 绿 | PASS（exit 0） |
| 2 | 无实现耦合 | PASS（只经库公共 API：`classify_with_tower_events_incremental` / `ChainCertificateBook::advance` / `heads` / `summarize` / `BspBridgeBook::advance` / `edges` / `heads` / `consume_at`；零 mock 私有、零 `pub(crate)` 内部路径） |
| 3 | 无同语反复 | PASS（读数来自 329MB 真实数据逐 bar 实跑，无硬编码期望值） |
| 4 | 无水平切片 | PASS（只做「consume_at 消费验收」一层，lib 零改） |
| 5 | bin 模块头写 seam/认识论等级/冻结语义来源 | PASS（bin 头 12–45 行） |

## 六、关键发现（照实登记）

1. **缝合域极窄**：300k 窗 22 条桥接边（22 个 event → 22 个 BSP），其中只有 **3 个 event 落在 Closed 链的存活节点上**（`P985_S8_OVERLAP in_closed_alive_node=3`）；100k 窗 3 条桥接边与 74 条 Closed 链节点**零交集**。即「桥接事件（BSP 产出侧）」与「链证书节点（跨级包含路径）」在 100k/300k 窗上近乎不相交，只有 3/22 汇入消费缝合线。
2. **跨链组内多链接实测**：300k 窗 `managed_bsp=3` 而 `bsp_links=4`——同一 BspStructuralKey 被两条 Closed 链授权，创建一次、写两条链接（#980 残余规则）。
3. **三窗链总数逐位复现 #641**（18/89/293），`advance_every=5000` 是复现前提（#641 经 issue550 默认 chain_every=5000 测得）。

## 七、复跑命令

```bash
cd /tmp/wt-n7-s8/rust
cargo check --features backtest_bin            # 验收 1
cargo fmt --check                              # Standards 1
cargo run --bin p985_s8_consume_baseline -- \
  /Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json 300000 5000
# release 亦可：cargo run --release --features backtest_bin --bin p985_s8_consume_baseline -- <json> 300000 5000
```
