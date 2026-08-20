# #1083 S5 新基线：p985 读数 bin 逐 bar 重测（替代 advance_every=5000 节拍口径历史读数）

- 票：#1083（SPEC #1077 S5，可并入 S4；S4 装配模块已由 #1088 落地并合 main）。
- bin：`rust/src/bin/p985_s8_consume_baseline.rs`（#1088 已缩为读数 bin，本票零改动）。
- 数据：`analysis/data_cache/btc_1m_full.json`（329MB，未提交仓；BTC 1m 全史同源，同 #641/#711/p985）。
- 运行：debug 构建（`cargo run --bin p985_s8_consume_baseline`）；推进口径 = **逐 bar（无 advance_every）**。
- 本票唯一代码改动：`rust/src/theta_v0/classifier/e2eo.rs` 的 `consume` 幂等簿记（详见 §五）。

## 一、Seam 与认识论等级

```
btc_1m_full.json(+max_bars) → classify_incremental（逐 bar 因果通道）
  → E2eOAssembly::advance（模块两入口之推进，B 增量化去节拍，closed_at 真 bar 位）
  → E2eOAssembly::consume（模块两入口之消费，纯 consume_at + 跨链 prior 幂等累积）
  → 验收读数（链总数/Closed/Open/Invalidated/桥接/消费/幂等/非 Closed 未消费）
```

- **认识论等级 = L1**（管线正确性读数）：只验证「因果通道 → 装配模块 → 消费」接线与幂等语义成立，
  不声明 alpha、不评估 consume_at 市场有效性（L2/L3 归后续票）。

## 二、三窗逐 bar 读数表（新基线）

| 窗口 bars | 链总数 | Closed | Open | Invalidated | revisions | 桥接 heads | event_to_bsp | 与 Closed 链存活节点交集(any/alive) | 受管 BSP | BspLink | 二次 consume | 非 Closed 未消费 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 20k | 1 | 1 | 0 | 0 | 1 | 0 | 0 | 0/0 | 0 | 0 | 0 创建 / 0 链接 | 0 |
| 100k | 15 | 11 | 4 | 0 | 15 | 1 | 1 | 0/0 | 0 | 0 | 0 创建 / 0 链接 | 4 |
| 300k | 70 | 38 | 9 | 23 | 297 | 18 | 18 | 1/1 | 1 | 2 | 0 创建 / 0 链接 | 32 |

逐行原始输出（stdout，debug 实跑，`advance=per_bar`）：

```
20k:
P985_S8_INPUT bars=20000 max_bars=20000 advance=per_bar
P985_S8_CHAIN chains=1 closed=1 open=0 invalidated=0 revisions=1 closed_with_zero_segments=0
P985_S8_BRIDGE revisions=0 heads=0 distinct_events=0 event_to_bsp=0
P985_S8_OVERLAP bridge_events=0 in_closed_any_node=0 in_closed_alive_node=0
P985_S8_CONSUME closed_chains=1 creations_total=0 managed_bsp=0 links_total=0 bsp_links=0 non_closed_not_consumed=0
P985_S8_IDEMPOTENCE second_creations=0 second_links=0

100k:
P985_S8_INPUT bars=100000 max_bars=100000 advance=per_bar
P985_S8_CHAIN chains=15 closed=11 open=4 invalidated=0 revisions=15 closed_with_zero_segments=0
P985_S8_BRIDGE revisions=2 heads=1 distinct_events=1 event_to_bsp=1
P985_S8_OVERLAP bridge_events=1 in_closed_any_node=0 in_closed_alive_node=0
P985_S8_CONSUME closed_chains=11 creations_total=0 managed_bsp=0 links_total=0 bsp_links=0 non_closed_not_consumed=4
P985_S8_IDEMPOTENCE second_creations=0 second_links=0

300k:
P985_S8_INPUT bars=300000 max_bars=300000 advance=per_bar
P985_S8_CHAIN chains=70 closed=38 open=9 invalidated=23 revisions=297 closed_with_zero_segments=0
P985_S8_BRIDGE revisions=39 heads=18 distinct_events=18 event_to_bsp=18
P985_S8_OVERLAP bridge_events=18 in_closed_any_node=1 in_closed_alive_node=1
P985_S8_CONSUME closed_chains=38 creations_total=1 managed_bsp=1 links_total=2 bsp_links=2 non_closed_not_consumed=32
P985_S8_IDEMPOTENCE second_creations=0 second_links=0
```

## 三、与旧节拍口径历史读数的对照

旧基线 `chanlun/review-results/issue985-n7-s8-baseline-20260815.md`（advance_every=5000，三窗
链总数 18 / 89 / 293）**降为「节拍口径历史读数」**，由本报告 §二 的逐 bar 读数取代。

| 窗口 | 旧（节拍口径，2026-08-15 代码）链总数 | 新（逐 bar，当前 main）链总数 | 旧 Closed | 新 Closed |
|---|---|---|---|---|
| 20k | 18 | 1 | 18 | 1 |
| 100k | 89 | 15 | 74 | 11 |
| 300k | 293 | 70 | 245 | 38 |

**照实说明（链总数差异的成因）**：旧读数与新一轮数的差，**主要不是节拍口径 vs 逐 bar 口径造成的**，
而是旧基线产自 2026-08-15 的代码快照（`a0a51ea0f5`），其后 main 落地了 #898（中心定理二扩展支
判据写全 + 本级盘背 lift==0 过滤，其测试注释自记「Pan 链身份减少 birth_closed 38→33」）、#1052
（一类点点锚迁移）、#1053（删全量 classify 循环）、#1054（classify 入口族 7→2）、#1076（departure
终点锚定位）、#1079（3a 生产单扫描）等判定面变更，链身份数随之收紧。

实测证据：当前代码下**即便用节拍口径**（advance_every=5000，复跑旧 bin 的链簿驱动方式）20k 也
得 1 链、100k 得 15 链，与逐 bar 读数逐位一致（诊断：CHECKPOINT/PER_BAR/FULL_REPLAY 三驱动同数）。
即「节拍 → 逐 bar」这一口径变化**不改变链总数**，它改变的是 `closed_at` 从 checkpoint 节拍量化
恢复到**真 bar 位**（这正是 S5 逐 bar 的目标）；链总数的 18→1 / 89→15 / 293→70 来自代码版本推进。

## 四、验收读数（以 300k 窗，唯一非空缝合域）

1. **Closed 链数 = 38**（三窗 1 / 11 / 38；样本量以实跑 Closed 链数为准，不沿用任何历史代表数）。
2. **受管 BSP = 1、BspLink = 2**：1 个 `BspStructuralKey` 被两条 Closed 链授权，创建一次、写两条
   链接（#980 残余规则「跨多条链只创建一次、多条链接写 BspLink 组内」的真实数据实例）。
3. **幂等 violations=0**：二次 consume 零创建、零链接（三窗全 0/0）。
4. **非 Closed 拒绝面正确**：300k 窗 32 条非 Closed 链头（9 Open + 23 Invalidated）不进入消费；
   `consume_at` 对非 Closed 谱系返回 `Err(NoConsumption)`（consume_at 模块单测锁定），装配
   `consume` 入口只收 `Closed`（`e2eo::tests::consume_only_closed_and_cross_chain_prior_accumulates`
   锁定），bin 侧 `non_closed_not_consumed` 读数与「Open+Invalidated」逐位相等（三窗 0/4/32）。
5. **桥接重叠**：300k 窗 18 个 event_to_bsp 键中仅 1 个落在 Closed 链存活节点（`in_closed_alive_node=1`），
   即消费缝合线由这 1 个重叠汇入；20k/100k 窗零重叠（缝合域空）。

## 五、本票代码改动：consume 幂等簿记修复（#1083）

300k 窗首跑时 bin 在「二次 consume」处报 `InconsistentState` 退出（exit 1）。根因：

- 装配 `consume` 每次全扫 `heads()` 的全部 Closed 链，第二次调用拿「已含后闭合链产出（
  `created_at` 较晚）」的 prior 去重评**早闭合**的链，`consume_at` 的 `InconsistentState` 校验
  （prior `created_at` 超前本次 `as_of`）当场命中；
- 合成夹具上未暴露，是因为既有模块测试两条链 `closed_at` 同值（100），真实 BTC 300k 窗 38 条
  Closed 链 `closed_at` 异位才触发。

修复（最小面，不动判定链）：`E2eOAssembly` 增 `consumed_chains: BTreeSet<ChainKey>` 簿记，
`consume` 只扫**尚未消费**的 Closed 链、消费成功后记身份 ⟹ 二次 consume 恒零增量。`consume_at`
纯函数面零改动（#982 A 面保持）；`chain_cert`/`bsp_bridge`/classifier 判定链零改动。

回归锁：`e2eo::tests::consume_idempotent_with_heterogeneous_closed_at`（两条 Closed 链
`closed_at` 50 vs 100，二次 consume 零增量；已按 TDD 验证：无修复时该测试以
`InconsistentState` 失败）。

## 六、验收对照

**Spec**：

| # | 验收项 | 结果 |
|---|---|---|
| 1 | 三窗实跑出读数（链总数/Closed/Open/Invalidated/桥接/消费/幂等/非 Closed） | PASS（§二） |
| 2 | 幂等 violations=0 | PASS（三窗二次 consume 全 0/0；300k 修复前 InconsistentState，修复后归零） |
| 3 | 非 Closed 拒绝面正确 | PASS（300k 32 条非 Closed 未消费；consume_at NoConsumption 单测 + 装配只收 Closed 单测） |
| 4 | 判定链零重写（只经装配模块公共 API） | PASS（唯一改动 = 装配模块 consume 幂等簿记；consume_at/chain_cert/bsp_bridge/classifier 零改动；bin 只经 advance/consume 两入口） |
| 5 | `cargo fmt --check` 净 | PASS（exit 0） |
| 6 | 新基线读数写入 report（注明逐 bar 口径与旧节拍口径对照） | PASS（本文件 §二、§三） |

**Standards**：

| # | 验收项 | 结果 |
|---|---|---|
| 1 | `cargo check` 过 | PASS（exit 0；`cargo test --lib` e2eo 6/6 + incremental_parity 1/1 绿） |
| 2 | 无同语反复 | PASS（读数来自 329MB 真实数据逐 bar 实跑，无硬编码期望值） |
| 3 | 无水平切片 | PASS（只做「读数 bin 逐 bar 新基线 + consume 幂等簿记修复」一层） |

## 七、复跑命令

```bash
cd rust
cargo fmt --check
cargo check
cargo test --lib e2eo                              # 含新的异构 closed_at 幂等回归锁
cargo test --lib e2eo_assembly_incremental_equals_full_replay

cargo run --bin p985_s8_consume_baseline -- \
  ../analysis/data_cache/btc_1m_full.json 20000    # 20k 窗
cargo run --bin p985_s8_consume_baseline -- \
  ../analysis/data_cache/btc_1m_full.json 100000   # 100k 窗
cargo run --bin p985_s8_consume_baseline -- \
  ../analysis/data_cache/btc_1m_full.json 300000   # 300k 窗（debug 约 4.5 分钟）
```
