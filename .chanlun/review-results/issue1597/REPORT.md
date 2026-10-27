# #1597：完整事实入口来源坐标 Θ(n²) → 共享表示

- 分支：`codex/1597-coords-shared`。基于 #1596 分支 `eng-debt-1588/memory`（`33b8036784`），再合入 origin/main `df6e680930`（合并提交 `e7f309be83`）。**依赖 #1596**：#1596 先合入后本 PR 的 diff 只剩本票内容。
- 远端：windows-5090 WSL，`nc-remote` 私有 target（`NC_TARGET=issue1597*`），release 构建。
- 数据：`btc_1m_full.json`（sha256 `16ea13d5…707b`，探针启动时校验，4,613,599 根）。窗口沿用 #1493/#1588：尾 `W` 根去前 5 根取前 n 根，默认 `W=50000`；更大前缀用 `ISSUE1588_WINDOW=W`。
- 修前 = `issue1597-pre2`（#1596 + main 合并 + 仅探针改动，`179b0ed049` → `d4a3fc687b`）；修后 = 本分支。两侧探针源码相同（只有 `SourceCoords` 计数口径不同）。

## 改了什么

`process_inclusion_with_facts` 在 source 坐标严格递增时改走 `IncrFactsState::append` + `materialize_facts`（#1394 的共享证据表示，原本就与全量入口逐位对拍）。来源坐标字段改为新公开类型 `inclusion::SourceCoords`：

| 字段 | 修前 | 修后 |
|---|---|---|
| `InclusionDirectionEvidence.source_coords` | `Vec<usize>` | `SourceCoords` |
| `InclusionStepFact.source_coords` | `Vec<usize>` | `SourceCoords` |
| `stroke::Endpoint.source_coords` | `Vec<usize>` | `SourceCoords` |
| `stroke::Confirmation.source_coords` | `Vec<usize>` | `SourceCoords` |

`SourceCoords` 内部是 `CoordsRef` 共享 AVL 树（升序去重）或扁平 `Rc<[usize]>`（保存原顺序）。对外合同：`Debug`（含 `{:#?}`，走 `debug_list`）、`serde::Serialize`（带长度序列）、`PartialEq/Eq`（按元素序列）与 `Vec<usize>` 逐位相同；`Clone` O(1)；有 `iter/len/is_empty/to_vec/contains/first/last/clear`、`From<Vec<usize>>`、`FromIterator`、`PartialEq<Vec<usize>>`。不实现 `Deref<Target=[usize]>`，展开必须显式 `to_vec()`。

端点来源（相邻三组成员 + 方向/封口证据）与确认证书来源（两端点并集）用 `union_into`：取最大值最大的证据树为基底，其余集合与基底做有序归并，缺的元素用 `CoordsRef::insert` 持久插入。精确性不依赖形状前提（第一版曾假设「证据坐标是完整 raw 前缀」，`tb02v_six_witnesses_through_production_entry` 等 5 个测试经调试断言抓到反例 `[15,16,17]` vs `[11,12,13,15,16,17]`，已改掉）。

非单调输入走保留的扁平参照 `process_inclusion_with_facts_flat`（#1597 前的唯一实现）；`IncrFactsState` 退化分支全部改调 `_flat`，无自递归。任一证据是扁平形态时端点/确认证书回落旧的排序去重构造。

**等价性证明**：包含层——`materialize_facts` 与全量入口的逐字段等价是 #1394 已有的逐前缀对拍合同，本票只把 `materialize()` 换成共享句柄（同一棵树的中序叶序，内容不变）；新笔层——`union_into` 是「基底 ∪ 其余」的逐元素集合并，结果升序去重，与「拼接 → sort_unstable → dedup」数学上相等。对拍锁：`assert_per_prefix_bit_exact` 改为以扁平参照为准，逐前缀比较公开入口的 `PartialEq`、`{:?}`、`{:#?}`；`shared_full_facts_match_flat_reference_for_every_prefix` 在两组 LCG 稀疏行情上逐前缀比较新笔事实的 `PartialEq`、`{:?}`、`{:#?}` 与 JSON，并断言严格递增输入下坐标为共享形态。

## 公开合同影响与调用方

受影响调用方（只在读取处加 `.to_vec()`，行为不变；main 行号）：

- `bin/s_session_v2/content_bi.rs` :1094、:1123-1124、:1260-1261、:1676、:1750
- `bin/s_session_v2/content_producer.rs` :3832
- `bin/s_session_v2/tb02a_facts.rs` :23、:504、:510、:745、:751
- `bin/s_session_v2/tb02b.rs` :172、:191、:256
- `bin/s_session_v3/object_records.rs` :1678
- `bin/s_structure_session.rs` :3285
- `bin/issue1588_parse_mem_probe.rs`（计数口径）
- parser 内部测试 `stroke.rs` 夹具一处 `vec![]` → `SourceCoords::new()`

其余读取点（`.iter()`、`.contains()`、`extend(&x.source_coords)`、`assert_eq!(x.source_coords, vec![..])`、`json!`/`serde_json::to_value`）源码不变即可编译。`cargo check --keep-going --all-targets --features backtest_bin,s_session` 零错误。

合同收窄：`SourceCoords` 含 `Rc`，这四个类型及 `InclusionFacts`/`StrokeFacts`/`GroupParseEvidence` 变为 `!Send`。全仓源码未见跨线程传递，编译通过即证。

**生产路径也受益**（独审 M-2）：S 生产入口 `parse_layer_with_group_evidence`（`s_session_v3/generation.rs:490`）经 `materialize_groups` → `endpoints_from(expand=true)`，修前同样逐组展开证据、逐端点展开来源，即 Θ(n²)。

写集协调：#1488 回复（2026-10-21）上列 bin 文件除 tb02b.rs 外 #1488 合并面都有大改、行号已漂；约定本 PR 只做最小 `.to_vec()`，谁后合入谁按意图吸收，#1488 不需要 rebase 到本分支。

## 读数

### 完整事实入口 `facts` 模式（W=50000）

| n | retained 修前 | retained 修后 | alloc_total 修前 → 修后 | peak 修后 | secs 修前 → 修后 | layer / facts 摘要 |
|---|---|---|---|---|---|---|
| 2,000 | 72.9 MB | 3.6 MB | 191.6 → 12.4 MB | 4.9 MB | 0.07 → 0.05 | `114c7ac9…0d78` / `1d44af38…e7f6` 相同 |
| 5,000 | 439.3 MB | 10.2 MB | 1,161.2 → 34.5 MB | 14.0 MB | 0.43 → 0.12 | `e8c11892…01a0` / `c668ab9f…4563` 相同 |
| 10,000 | 1,592.9 MB | 20.5 MB | 3,990.1 → 69.2 MB | 28.1 MB | 1.60 → 0.47 | `eef58c81…cfa6` / `d497d77f…34ad` 相同 |
| 20,000 | 6,518.5 MB | 41.8 MB | 16,601.1 → 140.3 MB | 57.1 MB | 7.73 → 5.27 | `4eddc809…5ebc` / `6d1ee5de…2b54` 相同 |
| 30,000 | 14,835.7 MB | 59.7 MB | 38,174.5 → 204.0 MB | 79.2 MB | 21.45 → 10.52 | `5fb961dd…6915` / `049f4463…e528` 相同 |
| 49,995 | 41,112.6 MB | 98.0 MB | 105,547.4 → 320.7 MB | 129.5 MB | 68.97 → 31.73 | `0b2f23ed…4dde7` / `4c926f23…ee05` 相同 |

修前翻倍 ×4，修后翻倍 ×2（2k→20k：×11.6 / n×10）。修前 49,995 的 VmHWM 43.7 GB（远端总内存 47 GB）。

摘要与 #1596 报告（`52e595c5…`/`21d820a0…` 等）不同：本分支合入了 main 的 #1582（TB-02-X 线段改动，20k 段数 46 → 210），修前侧是同一合并基，修前修后逐位相同。

### 更大前缀

| 前缀 | 入口 | 修前 | 修后 |
|---|---|---|---|
| 149,995（W=150000） | facts | 不可测（外推逻辑坐标元素 3.8×10¹⁰，约 300 GB） | retained 344.9 MB，peak 463.1 MB，alloc 1,200.4 MB，232 s；stroke_facts `4ca1de12…0c3f` |
| 149,995 | layer（紧凑入口） | `3a6e95ed…6e4e` | `3a6e95ed…6e4e`（facts 入口的 layer 摘要同值） |
| 299,995（W=300000） | layer | `94028455…5127` | `94028455…5127` |
| 299,995 | facts | 不可测（外推约 1.2 TB） | retained 700.2 MB，peak 937.3 MB，alloc 2,449.3 MB，2,587 s（远端有并发作业） |

更大前缀上 facts 修前侧装不下内存，逐位对拍面取 layer（修前紧凑入口 vs 修后紧凑入口与 facts 入口）。新笔事实与包含事实的逐位等价由上面 49,995 的整份 facts 摘要与逐前缀单测承担。

### S 生产入口 `group-evidence` 模式（`parse_layer_with_group_evidence`）

| 运行 | 修前 | 修后 | 摘要（layer/groups/stroke_facts/segment_facts，修前=修后） |
|---|---|---|---|
| n=49,995 末根调用一次 | 83.24 s，alloc 91,798.8 MB，peak 27,632.6 MB | 21.94 s，alloc 258.4 MB，peak 118.4 MB | `0b2f23ed…` / `bcc44397…5e5b` / `152214cd…dc78` / `2d1a1645…36c1` |
| n=20,000 每 500 根调用（40 次） | 194.08 s，alloc 199,225.7 MB，peak 4,335.6 MB | 232.46 s，alloc 1,158.8 MB，peak 50.6 MB | rolling `dd8b0180…410c` 相同 |

第二行耗时含每次调用后的 Debug 摘要（组证据 Debug 文本本身 Θ(n²)，两侧相同），且两侧运行时远端有其他作业并发；它不是解析耗时的干净对比。单次调用（第一行，只在末根摘要一次）修后快 3.8×。

## 测试

- `cargo fmt --all -- --check` ok。
- `cargo check --keep-going --all-targets --features backtest_bin,s_session` 零错误。
- `cargo test --lib theta_v0::parser`：187 passed / 0 failed / 22 ignored（`1c6e51f4bd`；含 M-1 修正的 `issue1597-wip4` 复跑同数）。
- `cargo test --lib theta_v0::backtest::incremental`：9 passed / 0 failed / 24 ignored。
- `cargo test --features s_session --bin s_structure_session`（`issue1597-wip4`）：511 passed / 22 failed / 14 ignored，22 个失败全部是 `v3::financial_session::tests::*`。这 22 个失败在修前基线 `issue1597-pre2` 上同样失败（`financial_session_tests.rs:40` `start_with(..).unwrap()`，WSL 环境），日志 `/tmp/nc-remote/20261021-074836-75329e8b8b.log`：`4 passed; 22 failed`。

## 变异负控（本地临时分支，未推送）

| 变异 | 内容 | 结果 | 日志 |
|---|---|---|---|
| M1 `ddbd2a22fb` | `union_into` 只插入 > max(基底) 的元素（回到「完整前缀」假设） | `coords_ref_insert_and_union_into_match_set_semantics` FAILED | `20261021-073843-ddbd2a22fb.log` |
| M2 `d4511cf7d7` | 步骤与证据坐标改回逐步展开的扁平 `Vec` | `shared_full_facts_match_flat_reference_for_every_prefix` FAILED（共享形态断言） | `20261021-074144-d4511cf7d7.log` |
| M2b `8efe620bda` | 同 M2，基于独审 M-1 修正后的留存锁 | `shared_full_facts_retain_subquadratic_nodes` FAILED：扁平元素 33,816,726 ≠ 0 | `20261021-074627-8efe620bda.log` |
| M3 `561d179fc8` | merge 步来源坐标漏掉 incoming | 5 个 FAILED（`completeness_matches_full_*`、`shared_full_facts_match_flat_reference_*` 等） | `20261021-074433-561d179fc8.log` |

## 非作者独审

`reviews`：`review-claude-opus-5-5.md`（子代理，模型 claude-opus-5-5；本机 claude CLI OAuth 过期，CLIProxyLocal 401，改用并发槽子代理）。结论 APPROVE，0 HIGH / 2 MEDIUM / 6 LOW。

- M-1（留存锁对回落扁平盲视）：已修（提交「test(#1597): 留存锁计入扁平展开」），M2b 负控证明有效。
- M-2（生产路径受影响、读数缺席）：本报告补 facts 2k→49,995 与更大前缀读数，并新增 `group-evidence` 模式测 S 生产入口修前/修后。
- LOW 未处理：`!Send`（已在合同影响里写明）；`high/low_sources` 在长平盘组内 Θ(m²)（与前缀长度无关，非本票）；`len()` O(len)（文档已写）；其余为措辞/clippy 级。

## 残余与需裁

1. **时间仍超线性（需裁）**：`union_into` 对其余证据集逐元素归并，单个端点 Θ(i) 时间，完整事实入口与 S 生产入口单次调用总时间仍是 Θ(n²)（facts：49,995 31.7 s、149,995 232 s、299,995 2,587 s；修前同为 Θ(n²) 时间且内存装不下）。对 Destination ④ 全史回放（460 万根）这仍不可行。可行的下一步是子集判定走树结构共享（证据树之间大量共享子树，按指针相等剪枝，单端点降到 polylog），建议另开票，本票按验收只交内存。
2. #1597 观察项（`replay-drop` 累计分配略超线性，段重建）未处理，与本票 schema 无关，建议单列。
