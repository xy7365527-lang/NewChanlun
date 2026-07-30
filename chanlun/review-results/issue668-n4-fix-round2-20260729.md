# #668（N4）修复轮 2——#670 影子评审第 2 轮 FAIL 回炉：两条新 HIGH + 残留处置

- ticket：#668（blocked-by #666）；对应评审票 #670 第 2 轮；母裁定链 #666 四问四裁 + 三轮/四轮
  supersede；map #529。
- 工位：`/tmp/wt-668c`，分支 `ticket-668c`，起点 = 复审尖端 `7b425a06e9`（复审报告
  `chanlun/review-results/shadow-668-review2-20260729.md`）。
- 执行器：claude sonnet 5（无头 dispatch 直达，本次为叶子层战术执行，未派发任何子代理/后台任务）。
- **结论**：dispatch 点名的 R2-HIGH-1 / R2-HIGH-2 / 3b（R2-HIGH-4）/ MED（三类右端等值残留）/
  LOW 残留（3 处 `ambiguous_keys=0` 未标注引用）**全部处置**；回归面（`cargo test --lib` 0
  failed、p92/runner/golden 零 diff、负控保持红绿分辨力）逐条复核成立。R2-HIGH-3 / R2-MED-2 /
  R2-MED-3 按 dispatch 明文排除在本轮修复单外，**本轮不处置**，如实登记交编排者另裁。不关票。

提交（2 个，均在 `ticket-668c`，均含 `#668`）：

| hash | 说明 |
|---|---|
| `efbf6405fe` | R2-HIGH-1/HIGH-2/HIGH-4（3b）/MED：多物理点载荷形态改集合 + 三类迁移 episode 覆盖 + 真平价锁 + battery bin 幂等/查询硬门 |
| `1a82746ed0` | ADR 第四轮 supersede + LOW-1 三处 `ambiguous_keys=0` 撤回 + 真值表 bin 文书订正 |

---

## 一、根因（R2-HIGH-1/R2-HIGH-2 同根）

第三轮 supersede 裁定①的语义——「同一 episode 内的多个物理一类点是同一候选身份的**修订史**」
——在修复轮 1 的实现里被映射成了错误的载荷形态：`resolve_first_class_episode_edges` 对一个
episode 覆盖的 k 个**同时并存**的物理点，按 `source_index` 升序产出 **k 条独立观察**，全部共享
同一 `BridgeKey`，交给 `BspBridgeBook::apply()` **顺序处理**。而 `apply()` 的幂等/终态判据
（prior vs next 二元比较）隐含假设「每次 `observe()` 每个 key 只产一条观察」——k 条独立观察
打破了这个前提：

1. **幂等破**（R2-HIGH-1）：同一 `(classification, streams, as_of)` 重跑，k 个点被重新逐个
   `apply`，与「当前链头」比较逐一不等 ⟹ 无条件 append k 条 churn revision（评审复核探针
   300k 窗实测每次重跑 +12、边数 29→41→53 无上界增长）。
2. **终态挡提前生效**（R2-HIGH-2）：失效时，遍历序第一个物理点的 `apply` 已把该 key 判
   `Invalidated`（终态），后续物理点的 `apply` 因终态挡直接 `return None`——链头因此**回退**到
   遍历序第一个物理点（而非最新物理点），其余物理点的 `Invalidated` 永不落簿，查询入口
   `edges_for_bsp_point`（按 `heads()` 过滤）永久丢失这些点（评审实测 300k 窗 7/29 点查无）。

## 二、R2-HIGH-1/R2-HIGH-2 处置：载荷形态改集合

`BspBridgeEdge::bsp_source_index: usize` 升级为 `bsp_source_indices: Vec<usize>`（有序去重集合，
新增 `head_source_index()` 取集合内最大值，兼容旧「链头=最新物理点」语义）。`observe()` 重构为
两阶段：

1. 先产**原始逐点观察**（`RawPointObservation`，一类经 `resolve_first_class_episode_points`
   事件侧驱动、二/三类经 `resolve_point_driven_observations` 点侧驱动，各自遍历入口不变）；
2. **按 `BridgeKey` 分组折叠**（`BTreeMap<BridgeKey, (u32, BTreeSet<usize>, ...)>`）成一条
   `BridgeObservation`——同 key 命中的全部物理点合并为一个集合。

保证每个 key 每次 `observe()` **恰好一条**观察，`apply()` 的幂等/终态二元比较前提重新成立。
`edges_for_bsp_point` 同步改为 `bsp_source_indices.contains(&source_index)` 集合成员判断（原
`==` 精确匹配）。这不是撤销第三轮裁定①，是把裁定①「多物理点=同一身份的并发载荷，非顺序时间序」
的语义正确落地到实现里。

**实测（`p_issue668_bsp_bridge_battery`，release，BTC 300k 窗，本 bin 本轮新增
`ISSUE668_IDEMPOTENCE`/`ISSUE668_QUERY_ENTRY` 硬门）**：

```
ISSUE668_IDEMPOTENCE bars=300000 delta1=22 edges1=22 delta2=0 edges2=22 delta3=0 edges3=22
ISSUE668_BRIDGE_BATTERY bars=300000 reference_edges=29 produced_edges=29 heads=22 missing_in_bridge=0 extra_in_bridge=0 cmp=0
ISSUE668_QUERY_ENTRY bars=300000 distinct_points_with_edges=29 query_reachable=29 query_lost=0
```

对照评审复核探针基线：`delta2` 从 **12** 降到 **0**（无上界增长止住）；`query_lost` 从 **7**
降到 **0**（29/29 全可查）；`produced_edges=29`（全部物理点仍在参照集里逐一留痕，只是折叠到
22 条 revision 里，与真值表 `episode_owned_one=29` 口径一致）。100k/20k 窗同绿（§五）。

## 三、3b（R2-HIGH-4）处置：跨 `as_of` 平价锁换真两驱动形态

旧测试 `bridge_book_incremental_equals_full_replay_across_as_of` 是「同一 `inputs` 列表跑两遍」
（`f(x)==f(x)`，结构上不可能失败），对齐的是 N3 最弱一面且未抄 N3 的三条非真空锁。新测试
`bridge_book_incremental_final_state_equals_fresh_full_replay_from_empty` 改为两个真正不同的
驱动（对齐 N1 `..._full_replay_equals_incremental` 先例）：

- 驱动 A = 逐 `as_of` 递进推进的增量簿（历经生长/新增二类点/失效四个中间态）；
- 驱动 B = 仅用**终态**一步 `(classification, streams, as_of)`、从空簿单次 `advance`（不是同一
  序列跑两遍——只喂最后一步）；
- 非真空锁三条（对齐 N3 `distinct_as_of>1`/`with_edges>0`/`certificates` 非空）：`inputs.len()>1`、
  增量序列 `total_revisions>1`、终态簿非空。

**据实登记该锁的判别力边界**（poison 注入法验证，见 §四）：该锁对「同一 `observe()` 调用内
多条同 key 原始观察被错误顺序 `apply`」这类缺陷**不具判别力**——因为该缺陷是 `observe()`+
`apply()` 管线内部对固定终态输入的**确定性函数**，driver A 与 driver B 最终都会经过同一条
（可能有缺陷的）管线收敛到**同一个**（可能错误的）不动点，二者不会分道。该锁的真实职责收窄为
「验证增量簿的记账不携带隐藏状态偏移」——一个不同于 R2-HIGH-1/R2-HIGH-2、但同样值得锁住的
性质（类比 N1 锁的是 `TowerCache` 增量 vs 非缓存全量的一致性，二者代码路径真正不同）。
R2-HIGH-1/R2-HIGH-2 这类缺陷的实际回归锁是本轮新增的 4 条直接单测（§四），已用 poison 注入法
逐条验证转红/转绿。这是本轮对 dispatch「锁要能捕获 R2-HIGH-1/2 这类偏差」这一要求的**如实
处置**：真平价锁已落地且非重言式，但没有夸大其判别范围。

## 四、MED（三类右端等值残留）处置：迁移到与一类同构的 episode 区间覆盖

三类原用 `leave_interval.1` 对 `trend_index_by_interval_end`（右端精确等值）反查，与一类修复前
的失效模式相同：Trend 候选 `growth_revision` 后索引键随当前 `interval.1` 漂移，而三类点自身
记录的 `leave_interval.1` 是过去时，二者错位后点永久失联（`trend_index_by_interval_end` 头注
「离开段右端不随本 episode 后续生长而漂移」混淆了匹配两侧坐标——评审 R2-MED-1 已指出该论证
不成立）。

处置：三类判据迁移到 `find_episode` 区间覆盖反查（与一类/二类同构），`trend_index_by_interval_end`
连同其 `debug_assert`/单测（`trend_index_collision_trips_debug_assert`）一并移除——静默覆盖
风险迁移到统一的 `find_episode`，新增 `overlapping_episodes_trip_find_episode_debug_assert`
覆盖（合成夹具：两个区间重叠的 episode 覆盖同一坐标，`debug_assert` 应响）。

新增测试锁住 MED 修复本身：

- `third_class_point_survives_trend_event_growth_via_episode_covering`：Trend 候选生长后三类点
  不再失联（旧判据下最终失效断言必然为 0 而 panic——已用 poison 注入法验证转红，见下）。
- `third_class_multiple_points_sharing_leave_segment_collapse_to_revision_history`：与一类同构
  的折叠机制对三类同样生效（合成夹具，不声称生产数据必然产生这种共享）。

## 五、Poison 注入验证（`observe()` 临时还原为「不分组」的旧实现，验证后已完整回退）

临时把 `observe()` 的分组折叠步骤替换成「逐点直接产出单元素观察」（复现修复轮 1 的旧 bug），
`cargo test --lib bsp_bridge` 结果：

```
test result: FAILED. 13 passed; 5 failed
failures:
    first_class_multiple_points_observed_simultaneously_collapse_to_one_revision
    first_class_new_point_added_later_appends_revision_with_expanded_covered_set
    invalidation_after_multi_point_episode_covers_all_points_and_stays_queryable
    multi_point_episode_same_as_of_rerun_is_zero_delta
    third_class_multiple_points_sharing_leave_segment_collapse_to_revision_history
```

`bridge_book_incremental_final_state_equals_fresh_full_replay_from_empty` **仍通过**（§三已
分析原因：两驱动对确定性管线收敛到同一不动点，该锁结构上无法区分此缺陷类别）。回退 poison
后（`diff` 核对与当前提交逐字节一致）`cargo test --lib bsp_bridge` 18 passed / 0 failed。

## 六、回归读数（全部本工位实测，`/tmp/wt-668c`，`CARGO_TARGET_DIR=/tmp/wt668c-target`）

```
git diff --stat 1b7ba1a1a5..HEAD -- rust/src/bin/p92_nest_replay_postruling.rs \
                                     rust/src/theta_v0/backtest/runner.rs   → 空（零消费接线不动）
git diff --stat 1b7ba1a1a5..HEAD -- rust/tests/                            → 空（golden 未动）
cargo check --all-targets                                                  → 绿（仅既有无关 warning）
cargo test --lib             → 2568 passed; 0 failed; 138 ignored（复审基线 2563 + 本轮净增 5：
                                 -1 移除的 trend_index_collision_trips_debug_assert + 6 新增 = +5）
cargo test --lib --release   → 2565 passed; 0 failed; 138 ignored
cargo test --lib bsp_bridge  → 18 passed; 0 failed（复审基线 13，净增 5，构成见 §九）
```

### battery bin 三窗（`p_issue668_bsp_bridge_battery`，release）

```
bars=20000   ISSUE668_IDEMPOTENCE delta1=0  edges1=0  delta2=0 edges2=0  delta3=0 edges3=0
             ISSUE668_BRIDGE_BATTERY reference_edges=0  produced_edges=0  heads=0  cmp=0
             ISSUE668_QUERY_ENTRY distinct_points_with_edges=0  query_reachable=0  query_lost=0
bars=100000  ISSUE668_IDEMPOTENCE delta1=3  edges1=3  delta2=0 edges2=3  delta3=0 edges3=3
             ISSUE668_BRIDGE_BATTERY reference_edges=3  produced_edges=3  heads=3  cmp=0
             ISSUE668_QUERY_ENTRY distinct_points_with_edges=3  query_reachable=3  query_lost=0
bars=300000  ISSUE668_IDEMPOTENCE delta1=22 edges1=22 delta2=0 edges2=22 delta3=0 edges3=22
             ISSUE668_BRIDGE_BATTERY reference_edges=29 produced_edges=29 heads=22 cmp=0
             ISSUE668_QUERY_ENTRY distinct_points_with_edges=29 query_reachable=29 query_lost=0
```

三窗 `cmp=0`/`delta2=delta3=0`/`query_lost=0` 全绿，退出码均 0（bin 已把三项硬指标全部纳入
退出码判定，`cmp==0 && idempotence_ok && query_lost==0` 才算 SUCCESS）。

### 真值表（`p_issue668_bsp_key_truth`，release，三窗，未改动逻辑，复核数值不变）

```
bars=20000  bit_instances=54   fingerprint_unresolved=17  episode_owned_one=0  episode_owned_many=0
bars=100000 bit_instances=348  fingerprint_unresolved=88  episode_owned_one=3  episode_owned_many=0
bars=300000 bit_instances=1199 fingerprint_unresolved=280 episode_owned_one=29 episode_owned_many=0
```

与复审报告 §四逐位一致（此 bin 本轮未改判据逻辑，仅文书订正，见 §八）。

## 七、逐条处置表（dispatch 修复单）

| # | dispatch 点名 | 处置 | 验收 |
|---|---|---|---|
| R2-HIGH-1 | 幂等证伪：同输入重跑非零 Delta | 载荷形态改集合（§二） | 300k 窗 `delta2=delta3=0`，边数稳定 22（覆盖 29 点不变）；100k/20k 同绿 |
| R2-HIGH-2 | 失效只落首点 + 查询丢点 | 同上（§二） | 300k 窗 `query_lost=0`（原 7/29 丢失）；单测 `invalidation_after_multi_point_episode_covers_all_points_and_stays_queryable` 锁死 |
| 3b（R2-HIGH-4） | 平价锁重言式 | 改真两驱动形态，如实登记判别边界（§三） | poison 验证：非重言式（结构真变化），但据实标注对本类缺陷不具判别力；4 条直接单测承担实际回归锁职责，均 poison 验证转红/转绿 |
| MED（R2-MED-1） | 三类右端等值残留 | 迁移到 episode 区间覆盖，同构一类（§四） | `third_class_point_survives_trend_event_growth_via_episode_covering` poison 验证转红；`trend_index_by_interval_end` 连同旧测试移除 |
| LOW 残留 | 3 处 `ambiguous_keys=0` 未标注引用 | ADR line 15/Considered Options 末项/`bsp_bridge.rs` resolve_second_class_anchor 文档三处删除线+订正 | commit `1a82746ed0`，append-only 式撤回 |
| 回归 | 复审确认绿的面不许回退 | 逐面复核（§六） | `cargo test --lib` 0 failed；p92/runner/golden 零 diff；负控（`trend_event_growth_does_not_orphan_earlier_first_class_point`）保持红绿分辨力（本身即在旧判据下会 panic 的负控，未改判据基线） |

## 八、本轮不处置（dispatch 明文排除，如实登记）

1. **R2-HIGH-3**（`find_episode` 的 `debug_assert` 在一类/三类路径无机器保证 + 真值表 20k 检验域
   为 0 报 SUCCESS 是空域重言式）——dispatch 修复单未列出此项，未处置。架构变化附带影响：
   `find_episode` 现在被一类/二类/三类三条路径共用（不再是二类专属），机器保证的覆盖面比修复轮 1
   更宽，但检验域是否非空、真值表退出码是否该按检验域大小分级，仍未处置，交编排者裁。
2. **R2-MED-2**（二类锚在生产数据上结构性不可解，`resolve_second_class_anchor` 第二条件恒假，
   0/280 命中）——未处置，`Type1Anchor` 语义与「已确认一类点坐标」的错位问题原样保留。
3. **R2-MED-3**（对拍规模：20k 窗仍空对空，二/三类两侧同为 0 使 cmp=0 对这两条路径零信息量）——
   未处置。

## 九、密度/规模对照

`bsp_bridge.rs` 实现 638 行（修复轮 1 约 570 行）；`tests.rs` 659 行 18 例（修复轮 1 444 行
13 例）——净增 5 例。构成：3 例原地替换（语义已变，非单纯改名）——
`first_class_multiple_points_in_same_episode_collapse_to_revision_history`→
`first_class_multiple_points_observed_simultaneously_collapse_to_one_revision`（同时并存改判
1 revision 非 2）、`trend_index_collision_trips_debug_assert`→
`overlapping_episodes_trip_find_episode_debug_assert`（跟随 `trend_index_by_interval_end` 移除
迁移到 `find_episode`）、`bridge_book_incremental_equals_full_replay_across_as_of`→
`bridge_book_incremental_final_state_equals_fresh_full_replay_from_empty`（重言式改真两驱动）；
另 5 例纯新增——`first_class_new_point_added_later_appends_revision_with_expanded_covered_set`/
`multi_point_episode_same_as_of_rerun_is_zero_delta`/
`invalidation_after_multi_point_episode_covers_all_points_and_stays_queryable`/
`third_class_point_survives_trend_event_growth_via_episode_covering`/
`third_class_multiple_points_sharing_leave_segment_collapse_to_revision_history`。

## 十、订正（append-only，2026-07-29，修复轮 3 补记，不改写以上原文）

- **R2-LOW-2**：`p_issue668_bsp_bridge_battery.rs` 头注「现役拼缝跨对象族不可执行」的论证列了
  三条障碍——`NestCandidateEvent` 跨对象族、`OwnerAnchorCtx`（owner 判同 oracle）、
  `event_bsp_book_level` 级别移位。复核（评审 #670 第 2 轮 R2-LOW-2）：只有 `NestCandidateEvent`
  成立且是唯一承重的一条（全仓无 lib 侧构造入口，产出只在 p92/p123/p124 bin 内的
  `collect_target_candidates`）；`OwnerAnchorCtx` 不成立——p92 自己传的是 4 行 `never` stub
  （p92:1038-1043，注释明言「本 bin 是归档研究/审计工具，未接事件锚账本」），复制它零成本；
  `event_bsp_book_level` 不成立——`nest.rs:493` 一行 `pub fn`，一行调用。结论不变（此路仍判
  不可执行，不退回），但论证里两条虚障碍应删，只留承重那条。本条已在修复轮 3 落地到
  `p_issue668_bsp_bridge_battery.rs` 头注（删除线标注 + 订正段）。
- **R2-LOW-3**（本报告 §九密度对照的行数订正，本段本身不改写 §九原文）：`bsp_bridge.rs` 实现
  行数复核为 638 行（本报告 §九已是此数，无需再订正）；`issue668-n4-fix-round1-20260729.md`
  §五「443→约 570 行」的实测值应为 597 行（该报告为修复轮 1 产物，订正落在该报告自身的
  append-only 订正段，见其文件）。ADR-0008 三个 `## Consequences` H2 标题（与 0003/0004 先例
  「单个 Consequences 段」不符）已在修复轮 3「第五轮 supersede」段处置：第三/四轮的两段改为
  `### Consequences（第 N 轮更新）` H3 子节，只保留原始 `## Consequences` 一个 H2。ADR 编号
  跳号（0005-0007 无实体）复核确认仍为既有事实，无新处置，照实登记非隐瞒。

不关票。
