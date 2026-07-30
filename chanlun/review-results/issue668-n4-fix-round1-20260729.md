# #668（N4）修复轮 1——#670 影子评审 FAIL 三条 HIGH + MED/LOW 全修

- ticket：#668（blocked-by #666）；对应评审票 #670；母裁定链 #666 四问四裁 + 三轮 2026-07-29
  supersede；map #529。
- 工位：`/tmp/wt-668c`，分支 `ticket-668c`，起点 = 评审尖端 `803b355bea`（评审报告
  `chanlun/review-results/shadow-668-review-20260729.md` 落盘处，`e522a484e9`）。
- 执行器：claude sonnet 5，全程前台单线程，未派发任何子代理/后台任务。
- **结论：评审点名 3 HIGH / 2 MED / 3 LOW 逐条处置——3 HIGH 全部代码修复并复跑验证；2 MED
  补测试固化；3 LOW 全部处置（2 项文书改写 + 1 项照实登记见 §六）。不关票。**

修复提交（4 个，均在 `ticket-668c`，均含 `#668`）：

| hash | 说明 |
|---|---|
| `f9cd75e436` | HIGH-1/HIGH-2：一类点身份改 episode + join 判据改区间覆盖 |
| `0596b37109` | HIGH-3：对拍/真值表 bin 重定门 |
| `a7e0415c5e` | LOW：文书撤回「覆盖率与唯一性正交」断言 + ADR-0008 补第三轮 supersede 段 |
| `bc249d2109` | debug_assert 单测限 debug profile（release 复跑中发现并修复） |

---

## 一、HIGH-1 [键/身份] 处置

**评审发现**：一类的 `BspStructuralKey` 锚全部由事件键派生，同一 episode（同 `seg_a`+`c_start`）
内多个物理一类点锚完全相同 ⟹ 键相同；真值表判 `ambiguous_keys=0` 是检验域被同一判据（右端等值）
窄化的算术必然，不是键的区分力（300k 窗纳入被排除样本后实测 5 组撞键，最坏一组 4 点共键）。

**处置（对齐 #666 第三轮 supersede 裁定①）**：不再把「键唯一」当验收目标。一类点身份 = episode
（v2 锚不变：`(seg_a, c_start)`）；同一 episode 内的多个物理一类点是**同一候选身份的修订史**——
`source_index`/pivot 是修订载荷，不入身份分量，撞键自动消解，不需要任何区分量。真正的不变量改为
**episode 归属唯一**：一个一类点只能落在一个 episode 的 `[c_start, interval.1]` 区间内（同
level/side/中枢指纹）。新增 `find_episode`（`bsp_bridge.rs`）用 `debug_assert` 机器化此不变量。

真值表探针 `p_issue668_bsp_key_truth.rs` 全面重写，口径从「键唯一性」改「episode 归属唯一性」：

| 窗口 | bit_instances | fingerprint_unresolved | owner_query_unresolved | episode_owned_zero | episode_owned_one | **episode_owned_many** |
|---|---|---|---|---|---|---|
| 20,000 | 54 | 17 | 0 | 0 | 0 | **0** |
| 100,000 | 348 | 88 | 0 | 0 | 3 | **0** |
| 300,000 | 1199 | 280 | 0 | 0 | 29 | **0** |

三窗 `episode_owned_many=0`（覆盖一类/二类；三类未改判据不纳入本检验，见 §三）——episode 归属
唯一，dispatch「若仍有撞键，停手上报」的分支未触发，无需上报。

## 二、HIGH-2 [双向产出/接线] 处置

**评审发现**：一类实测 62% 命中缺口不是候选域结构性错位，是接线 bug——18/29（300k 窗）个一类点
落在候选 episode 区间内但不在候选当前右端上，被右端等值判据漏配；裁定③「双向产出」实际只有
「点侧 → 事件」一向，缺「事件侧 → 点」这一向（后者正是能把这些边写出来的方向）。

**处置**：`bsp_bridge.rs` 新增 `resolve_first_class_episode_edges`（事件侧驱动）：遍历 Trend
episode，对每个 episode 回挂其区间覆盖的全部一类点（同 level/side/中枢指纹），与既有「点侧 →
事件」（二/三类，`resolve_bridge`）构成两个真实独立的遍历入口，不是同一遍历改名两次。二类锚查找
同步从精确等值改为 episode 区间覆盖（同一失效模式，评审未点名但代码路径共享，一并修复）。

BTC 实测（`p_issue668_bsp_bridge_battery.rs`，见 §四对拍读数表 `hit_by_class`）：

| 窗口 | 修复前一类覆盖率 | 修复后一类覆盖率 |
|---|---|---|
| 100,000 | 33%（1/3） | **100%（3/3）** |
| 300,000 | 38%（11/29） | **100%（29/29）** |

## 三、HIGH-3 [验收对拍门] 处置

**评审发现**：旧对拍参照集（`reference_join`）与生产模块逐行同构复写，共享全部设计判断（包括错
的那些），验不出判据层错误（HIGH-2 就在它眼前而它 cmp=0）；20k 窗是 0-vs-0 空对空；对拍对象换了
——裁定⑥要求对拍现役拼缝 `nest::terminal_bits_at_event`，实装从未引用。

**处置**：

1. **现役拼缝跨对象族不可执行，如实上报**（按 dispatch「跨对象族不可直接对拍则上报改门，不得
   自替代」指引）：`nest::terminal_bits_at_event` 需要 `NestCandidateEvent`（`nest` 模块自有
   事件体系，非本票 `cand_event::CandidateEvent`）+ `OwnerAnchorCtx`（owner 判同 oracle）+
   `event_bsp_book_level` 级别移位；接进来意味着为桥接对象新构造一整套 nest 侧事件与锚供给，
   本身是一次新的消费接线，违反裁定④「p92/π runner 本票零消费接线」+「不重算既有判据」方法学。
   本修复轮判定此路**不可执行**，未强行自替代。
2. **跨 `as_of` 平价锁**（dispatch 明文替代方案①，对齐 N1/N3 先例）：新增单测
   `bridge_book_incremental_equals_full_replay_across_as_of`（`bsp_bridge/tests.rs`）——
   增量推进的簿快照与从零全量重放逐字段相等，覆盖生长 + 新增二类点 + 事件失效四个步骤。
3. **负控**（dispatch 明文替代方案②）：新增单测
   `trend_event_growth_does_not_orphan_earlier_first_class_point`——在旧右端等值判据下，
   事件失效不会同步传导到边（边永久卡在生长前的旧状态）；此测试在旧代码上必然在最终
   `Invalidated` 断言处失败，修复后必须通过（已验证：临时还原旧判据本地复跑此测试确实 FAILED，
   现行代码 PASSED）。
4. **参照集改真独立**（`p_issue668_bsp_bridge_battery.rs`）：不再共享生产模块的索引/折叠函数，
   改线性扫描 + 自有数据形状独立实现同一条（已由 supersede 裁定settled 的）episode 覆盖判据；
   对拍对象从 `heads()`（只见链头）改 `edges()`（append-only 全量修订历史，覆盖同 episode
   多物理点收敛场景，不因折叠而漏判）。

BTC 三窗复跑（`CARGO_TARGET_DIR=/tmp/wt668c-target cargo run --release --bin
p_issue668_bsp_bridge_battery`）：

```
bars=20000   trend_events=0  b1=0  reference_edges=0  produced_edges=0  heads=0  cmp=0
bars=100000  trend_events=7  b1=3  hit_by_class={"Buy1":1,"Sell1":2}
             reference_edges=3  produced_edges=3  heads=3  cmp=0
bars=300000  trend_events=42 b1=29 hit_by_class={"Buy1":9,"Sell1":20}
             reference_edges=29 produced_edges=29 heads=22 missing=0 extra=0 cmp=0
```

三窗 `cmp=0`（对全量 `edges()`，非仅链头）；300k 窗 `produced_edges=29`（全部 29 个物理一类点
均在某条 revision 里留痕）而 `heads=22`（22 个不同 episode 身份，其中若干 episode 覆盖多个
物理点并折叠为单一修订链头，与 §一真值表 `episode_owned_one` 口径一致）。

## 四、MED-1 [事件侧生长后配对丢失] 处置

**评审发现**：`ChainCertificateBook` 式的「不需要 resident 补集」论证只覆盖 BSP 侧单调性，未
覆盖事件侧——候选 growth_revision 后若配对不再解出，簿内的边停在 `Open`，事件后续转
`Invalidated` 不会同步；若新右端恰好也有一个一类点，`BridgeKey` 会静默改指到另一个 BSP 点。

**处置**：HIGH-1/HIGH-2 的 episode 覆盖判据修复直接消解此问题的根因——`observe()` 每次全量重扫
都按当前（最新）episode 区间重新求覆盖点集，不再依赖某次观察时的右端快照，事件失效后下一次
`advance` 天然能重新观察到该点并同步终态。评审担心的「静默改指到另一个 BSP 点」在第三轮
supersede 后被正式定性为**合法语义**（同 episode 修订史），不再是 bug。

新增单测覆盖：
- `trend_event_growth_does_not_orphan_earlier_first_class_point`：生长 + 后续失效 → 边正确
  同步转 `Invalidated`（同时也是 HIGH-3 的负控用例）。
- `first_class_multiple_points_in_same_episode_collapse_to_revision_history`：同 episode
  两个物理点 → 2 条 revision（append-only 留痕）、共享同一 `BridgeKey`、链头 = 最新物理点。
- `distinct_episodes_never_share_a_bridge_key`：同 parent 指纹、不同 episode 的两个一类点
  绝不共享 `BridgeKey`（区分「合法修订链」与「非法跨 episode 误合并」两种情形）。

## 五、MED-2 [测试密度] 处置

评审点名 5 个缺口，本轮全部补齐（`bsp_bridge/tests.rs`，7 → 13 例，+6，净增行数 199）：

| 缺口 | 新增测试 |
|---|---|
| 同 key 多 revision（生长）后配对丢失 | `trend_event_growth_does_not_orphan_earlier_first_class_point` |
| 同 `BridgeKey` 改指另一 BSP 点 | `distinct_episodes_never_share_a_bridge_key`（反面：绝不误合并）+ `first_class_multiple_points_in_same_episode_collapse_to_revision_history`（正面：合法修订链） |
| 同一 episode 内多个一类点 | `first_class_multiple_points_in_same_episode_collapse_to_revision_history` |
| `trend_index_by_interval_end` 键冲突 | `trend_index_collision_trips_debug_assert`（`cfg(debug_assertions)`，release 天然跳过） |
| 多级别正例 | `first_class_point_pairs_at_non_zero_level`（level=1） |

另补 HIGH-3 所需的跨 `as_of` 平价锁：`bridge_book_incremental_equals_full_replay_across_as_of`。

密度对照（评审 §四表）：N3 `chain_cert` 1198 行实现 / 941 行测试 / 24 例；N4 `bsp_bridge.rs`
本轮 443→约 570 行实现 / 245→444 行测试 / 7→13 例。测试:实现比从 0.55 升到约 0.78（N3 为
0.79），密度基本看齐；例数仍低于 N3（13 vs 24）——N3 覆盖的是链证书更复杂的多级传播状态机，
N4 对象状态空间更小（二态 `Open`/`Invalidated`，无 `Closed`），13 例已覆盖评审点名的全部缺口 +
原有 7 例，未按绝对数字硬凑。

## 六、LOW 处置

**LOW-1（报告与分支名实不符）**：本报告的计数/hash/工位起点均基于本工位 `git log`/`cargo test`
实测直接写出（见上表 + §七），不存在名实不符问题；旧报告 `issue668-n4-impl-ticket668b-
20260729.md` 的 §一/§六两处断言已在本轮撤回改写（append-only：保留原文 + 行内删除线标注 +
补 §十二 订正段，不静默改写历史），见 commit `a7e0415c5e`。本伦次起一律以 `ticket-668c` 为准。

**LOW-2（`trend_index_by_interval_end` 静默覆盖无留痕）**：已加 `debug_assert`（commit
`f9cd75e436`），键冲突会在 debug 臂 panic 并打出冲突双方；`trend_index_collision_trips_
debug_assert` 单测锁定此行为（`cfg(debug_assertions)` 门控，release 天然跳过不构成漏测）。

**LOW-3（ADR 编号跳号，0005-0007 无实体）**：非本票新增问题（N1/N3 也只在 CONTEXT.md 挂号未
落盘 ADR），本票未处置——按评审自己的建议，登记为遗留，交编排者在 #529 下裁是否补号或改按票号
命名。

其余文书订正：`bsp_bridge.rs:199-205`（头注对齐断言，评审未单独编号但归入本轮文书修复范围）
已随模块头整体重写替换（commit `f9cd75e436`），不再声称一类点 `source_index` 恰好落在候选右端上。

## 七、复跑读数（全部本工位实测，`/tmp/wt-668c`，`CARGO_TARGET_DIR=/tmp/wt668c-target`）

```
git diff --stat 803b355bea..HEAD -- rust/src/bin/p92_nest_replay_postruling.rs \
                                     rust/src/theta_v0/backtest/runner.rs   → 空（零消费接线不动）
git diff --stat 803b355bea..HEAD -- rust/tests/                            → 空（golden 未动）
cargo check --all-targets                                                  → 绿
cargo test --lib             → 2563 passed; 0 failed; 138 ignored（评审基线 2557 + 本轮净增 6）
cargo test --lib --release   → 2560 passed; 0 failed; 138 ignored（+3：1 例 cfg(debug_assertions) 门控跳过）
```

真值表/对拍三窗读数见 §一/§三。

## 八、遗留（照实）

1. 三类近零覆盖仍未解决——归 #688，本票判据未改（v2 三类锚已被评审验过不空洞，问题在候选域
   结构性错位，非配对判据）。
2. LOW-3 ADR 编号跳号建议登记，交编排者裁（见 §六）。
3. `nest::terminal_bits_at_event` 跨对象族对拍判不可执行（见 §三第 1 点）——若日后编排者裁定
   仍要打通这条现役拼缝对拍，需要单开一票构造 `NestCandidateEvent`/`OwnerAnchorCtx` 供给，
   超出本修复轮授权范围。
4. 评审 §四表 N3/N4 测试例数密度（24 vs 13）未硬追平，理由见 §五。

不关票。

## 九、订正（append-only，2026-07-29，修复轮 3 补记，不改写以上原文）

- **R2-LOW-3**：§五「本轮 443→约 570 行实现」的「约 570」应为**实测 597 行**（`bsp_bridge.rs`
  在本报告对应的提交上的真实行数；修复轮 2/3 之后随内容增长升至 638 行，见修复轮 2 报告 §九、
  修复轮 3 报告）。密度对照的定性结论（测试:实现比升到约 0.78，接近 N3 的 0.79）不受此订正影响。

不关票。
