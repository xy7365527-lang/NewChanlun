# #668（N4）修复轮 3——#670 影子评审第 2 轮遗留补修（R2-HIGH-3/R2-MED-2/R2-MED-3/R2-LOW-2/R2-LOW-3）

- ticket：#668（blocked-by #666）；对应评审票 #670 第 2 轮；母裁定链 #666 四问四裁 +
  三/四/五轮 supersede；map #529。
- 工位：`/tmp/wt-668c`，分支 `ticket-668c`，起点 = 修复轮 2 尖端 `940400723b`（复审报告
  `chanlun/review-results/shadow-668-review2-20260729.md`；修复轮 2 报告
  `chanlun/review-results/issue668-n4-fix-round2-20260729.md`）。
- 执行器：claude sonnet 5（无头 dispatch 直达，本次为叶子层战术执行，全程前台单线程，
  未派发任何子代理/后台任务）。
- **结论**：dispatch 本轮点名的 R2-HIGH-3 / R2-MED-2 / R2-MED-3 / R2-LOW-2 / R2-LOW-3 全部
  处置（前两轮已封的面——episode 身份、点集载荷、幂等、失效全点、查询 29/29、真两驱动平价锁 +
  判别力边界标注、负控、参照独立、零消费接线——未推倒，逐项复核仍成立）。回归面：
  `cargo test --lib` 2569 passed / 0 failed（基线 2568 + 本轮新增 1）；release 2565 passed /
  0 failed（新增测试 `#[cfg(debug_assertions)]` 门控，release 天然跳过，计数不变）；
  p92/runner/golden 零 diff；生产引用面仍仅 `mod.rs:107` 一处。不关票。

---

## 一、R2-HIGH-3 处置：一类路径补齐机器保证 + 真值表检验域纳入判级

### 1a. 机器保证覆盖面

**根因**：`find_episode`（`bsp_bridge.rs`）的 `debug_assert` 机器化「episode 归属唯一」，但
修复轮 2 之前，一类路径（`resolve_first_class_episode_points`）绕过 `find_episode`、自行内联
同款过滤逻辑（外层遍历 episode、内层遍历点，判断 `center_fingerprint == episode.parent` 且
`c_start <= source_index <= interval_end`）——判据与 `find_episode` 完全同构，但没有走
`find_episode` 这个函数体，因此触发不到里面的 `debug_assert`。二/三类路径（`resolve_bridge`）
本就调用 `find_episode`。评审 R2-HIGH-3 指出：真值表检验域里唯一有真实数据的点类（一类）
零机器保证，有机器保证的点类（二类）零数据——两者从未重合。

**处置**：`resolve_first_class_episode_points` 改为逐点遍历（外层点、内层 side/class），对每
个命中 buy1/sell1 位的点调用 `find_episode(episodes, level, side, parent, source_index)` 反查
其所属 episode，语义与原实现逐字等价（同一组 level/side/parent/区间过滤条件），但现在真正经过
`find_episode`——一/二/三类三条路径至此共用同一个带 `debug_assert` 的查找函数。

```rust
// bsp_bridge.rs（节选，完整版见源码）
for (level_idx, level) in classification.levels.iter().enumerate() {
    for (idx, point) in level.bsp.iter().enumerate() {
        for (side, class, hit) in [
            (Side::Long, BspPointClass::Buy1, point.bits.buy1),
            (Side::Short, BspPointClass::Sell1, point.bits.sell1),
        ] {
            if !hit { continue; }
            let Some(parent) = center_fingerprint(level, idx) else { continue };
            let Some(episode) = find_episode(episodes, level_idx as u32, side, parent, point.source_index)
                else { continue };
            // ... 产 RawPointObservation
        }
    }
}
```

**回归验证**：既有 18 例 `cargo test --lib bsp_bridge` 全绿（语义等价，未改变任何既有测试的
断言）。新增单测 `overlapping_episodes_trip_find_episode_debug_assert_via_first_class_path`：
两个区间重叠的 episode（`(6,19)→c_start=25,[25,60]` 与 `(10,24)→c_start=30,[30,60]`）+ 一个
落在交集 `[30,60]` 内的一类点（`source_index=50`），`debug_assert` 必须响。

**Poison 验证**（`git stash` 临时还原本条改动，保留新测试）：

```
test overlapping_episodes_trip_find_episode_debug_assert_via_first_class_path ... FAILED
note: test did not panic as expected
```

旧实现下测试确实不 panic（`should_panic` 断言落空，测试失败）——证明该修复是真实的行为变化，
不是同构复写。还原修复后再跑：19 passed / 0 failed。

### 1b. 真值表检验域纳入判级

**根因**：`p_issue668_bsp_key_truth` 旧版本 `episode_owned_many == 0` 即报 `SUCCESS`，不区分
「域非空且无撞键」与「域本来就是空的」。20k 窗检验域实际是 0 个实例（`episode_owned_zero=0,
episode_owned_one=0, episode_owned_many=0`），旧 exit=0 是空域重言式——与前判 HIGH-3「20k 空
对空」同型未修，只是从对拍 bin 搬到了真值表 bin。

**处置**：新增 `domain_size = episode_owned_zero + episode_owned_one + episode_owned_many`
（真正走到「反查 episode 归属」这一步、已排除 `fingerprint_unresolved`/`owner_query_unresolved`
的样本数），退出码三分：

| 条件 | 判级 | 退出码 |
|---|---|---|
| `episode_owned_many > 0` | FAIL（撞键） | 1 |
| `domain_size == 0` | EMPTY_DOMAIN（空域，非 PASS） | 2 |
| 否则 | PASS | 0 |

同时新增 `ISSUE668_TRUTH_V3_DOMAIN_BY_CLASS`（逐点类检验域大小）与 `ISSUE668_TRUTH_V3_VERDICT`
（人类可读判级行）。模块头「HIGH-2 修复覆盖二类」的不实断言一并撤回（见 §二）。

**三窗复跑**（release，BTC，与复审报告 §四逐位一致）：

```
bars=20000  domain_size=0  episode_owned_zero=0 episode_owned_one=0  episode_owned_many=0  → EMPTY_DOMAIN（exit=2）
bars=100000 domain_size=3  episode_owned_zero=0 episode_owned_one=3  episode_owned_many=0  → PASS（exit=0）
bars=300000 domain_size=29 episode_owned_zero=0 episode_owned_one=29 episode_owned_many=0  → PASS（exit=0）
```

`ISSUE668_TRUTH_V3_DOMAIN_BY_CLASS`：100k `{"Buy1":1,"Sell1":2}`、300k `{"Buy1":9,"Sell1":20}`，
20k `{}`（空）——与真值表 `bit_instances`/`fingerprint_unresolved` 原始读数逐位一致（本轮未改
判据逻辑，只加检验域计数与退出码分级）。

## 二、R2-MED-2 处置：二类锚归属结构性不可解，停手照实上报（判据不改动）

**复核确认**：`resolve_second_class_anchor` 现有实现——「反查同级 `source_index==anchor_idx`
且持有对应 buy1/sell1 位的点」——**已经就是** dispatch 点名的「结构可查的那条路径」（同级同向簿
内查该二类点对应的一类点）。评审 R2-MED-2 指出的问题不在这个查询本身写错了，而在于
`OwnerRef::Type1Anchor(type1_src)` 这个坐标的**语义**：它载的是该走势 m1 走势的**终点坐标**
（`signal.rs` `extract_second_signals`），不是「已确认的一类点坐标」——m1 终点要另外过背驰确认门
才能成为一类点，二者在生产数据上系统性不重合。

**新增分解诊断**（`p_issue668_bsp_key_truth` 新增 `b2_anchor_diag` + `ISSUE668_MED2_ANCHOR_BREAKDOWN`），
把 `resolve_second_class_anchor`（及真值表侧同款 `resolve_fingerprint`）合取的两个条件拆开单独计数：

```
bars=20000  class=Buy2 with_type1anchor=10  anchor_hosts_first_class=0
bars=20000  class=Sell2 with_type1anchor=7   anchor_hosts_first_class=0
bars=100000 class=Buy2 with_type1anchor=44  anchor_hosts_first_class=0
bars=100000 class=Sell2 with_type1anchor=44  anchor_hosts_first_class=0
bars=300000 class=Buy2 with_type1anchor=139 anchor_hosts_first_class=0
bars=300000 class=Sell2 with_type1anchor=141 anchor_hosts_first_class=0
```

条件一（本点携 `Type1Anchor`）三窗合计 17/88/280，与 `b2_points`（17/88/280，见对拍 bin
`ISSUE668_BRIDGE_COVERAGE`）逐位吻合——生产上恒真。条件二（该锚坐标确有点持有一类 bit）三窗
**均为 0**——0/17、0/88、0/280，证实评审复核探针的 `anchor_hosts_first_class_point=0/88`、
`0/280` 读数，本轮追加 20k 窗（同样 0/17）。

**处置**：按 dispatch「生产数据上结构性不可得，停手照实上报，禁发明坐标、禁放宽判据假装修复」
——`resolve_second_class_anchor` **不改动**，`bsp_bridge.rs` 头注（该函数文档）追加「第五轮
supersede」订正段，明确本函数已是能查的唯一结构路径、生产上结构性不可得、非实现缺陷；ADR-0008
「第五轮 supersede」段同步登记。是否归 #688（候选域结构性错位）还是二类键公式本身选错了锚
（该走 别的坐标而非 m1 终点），本轮不裁，交编排者定。

## 三、R2-MED-3 处置：对拍 bin 增加分点类计数

**处置**：`p_issue668_bsp_bridge_battery` 新增逐点类（Buy1/Buy2/Buy3/Sell1/Sell2/Sell3）计数行
`ISSUE668_BRIDGE_BATTERY_BY_CLASS`——reference/produced/missing/extra/cmp 各自的点类拆分；
两侧同为 0 的点类显式打印 `note=此类空域，cmp无信息量`，不让总 `cmp=0` 掩盖检验域窄化。

**三窗复跑**（release）：

```
bars=20000   全 6 点类 reference=0 produced=0 cmp=0，全部 note=此类空域，cmp无信息量
bars=100000  Buy1 reference=1 produced=1 cmp=0；Sell1 reference=2 produced=2 cmp=0；
             Buy2/Buy3/Sell2/Sell3 reference=0 produced=0 cmp=0（空域标注）
bars=300000  Buy1 reference=9 produced=9 cmp=0；Sell1 reference=20 produced=20 cmp=0；
             Buy2/Buy3/Sell2/Sell3 reference=0 produced=0 cmp=0（空域标注）
```

与总 `ISSUE668_BRIDGE_BATTERY cmp=0` 一致（0/3/29 边全部对上），但现在读者能看到：三窗合计
参与比对的 32 条边全部是一类，二/三类六个（点类×窗口）组合全部是空对空、零信息量。三类
`reference=0/produced=0`（20k/100k/300k 均如此）与既有基线一致（三类近零覆盖归 #688，本轮
未处置，非回归）。

## 四、R2-LOW-2 处置：删两条虚障碍，只留承重那条

`p_issue668_bsp_bridge_battery.rs` 头注「现役拼缝跨对象族不可执行」列的三条障碍——
`NestCandidateEvent` 跨对象族、`OwnerAnchorCtx`（owner 判同 oracle）、`event_bsp_book_level`
级别移位——复核：只有 `NestCandidateEvent` 成立且是唯一承重的一条（全仓无 lib 侧构造入口，
产出只在 p92/p123/p124 bin 内的 `collect_target_candidates`，需 tower + MACD `hist`/`dif`/
`close_src` 供给管线）；`OwnerAnchorCtx` 不成立——p92 自己传的是 4 行 `never` stub
（`p92:1038-1043`，注释明言「本 bin 是归档研究/审计工具，未接事件锚账本——二类判同锚不可解
= 诚实判负」），复制它零成本；`event_bsp_book_level` 不成立——`nest.rs:493` 一行 `pub fn`，
一行调用。结论不变（此路仍判不可执行，不退回），论证收窄为只留 `NestCandidateEvent`。

处置落地两处：(1) `p_issue668_bsp_bridge_battery.rs` 头注删除线标注 + 订正段（源码，见 §一
commit）；(2) 修复轮 2 报告 `issue668-n4-fix-round2-20260729.md` 追加 append-only 订正段
§十（不改写原文）。

## 五、R2-LOW-3 处置：三处名实小差订正

1. **行数**：`issue668-n4-fix-round1-20260729.md` §五「443→约 570 行」订正为实测 **597 行**
   （该数是修复轮 1 提交上的真实行数；修复轮 2/3 之后内容增长至 638→659 行，为不同时点的
   不同事实，非同一处矛盾）。订正落在该报告自身的 append-only §九（不改写 §五原文）。
2. **ADR 重复 `## Consequences` 段**：本 ADR 此前有 3 个 H2 级 `## Consequences...` 标题
   （原段 + 第三轮更新 + 第四轮更新），与 0003/0004 先例「单个 Consequences 段」结构不符。
   处置：第三/四轮的两段降级为 `### Consequences（第 N 轮更新）` H3 子节，各自挂在对应的
   supersede H2 段下；只保留原始 `## Consequences` 一个 H2。内容零改动，仅调整标题层级——
   `grep -n '^## \|^### '` 复核：`## Consequences` 现唯一。
3. **ADR 编号跳号**（0005-0007 无实体）：复核 `docs/adr/` 目录仍为 `0001-0004` + `0008`，
   与修复轮 1 报告 §八已登记的遗留一致，本轮无新处置（照实登记非隐瞒）。

## 六、回归读数（全部本工位实测，`/tmp/wt-668c`，`CARGO_TARGET_DIR=/tmp/wt668c-target`）

```
git diff --stat 1b7ba1a1a5..HEAD -- rust/src/bin/p92_nest_replay_postruling.rs \
                                     rust/src/theta_v0/backtest/runner.rs   → 空（零消费接线不动）
git diff --stat 1b7ba1a1a5..HEAD -- rust/tests/                            → 空（golden 未动）
全仓 bsp_bridge 生产引用面                                                  → mod.rs:107 一处（不变）
cargo check --all-targets                                                  → 绿（仅既有无关 warning）
cargo test --lib             → 2569 passed; 0 failed; 138 ignored（前判基线 2568 + 本轮净增 1：
                                 新增 overlapping_episodes_trip_find_episode_debug_assert_via_first_class_path）
cargo test --lib --release   → 2565 passed; 0 failed; 138 ignored（新增测试 cfg(debug_assertions)
                                 门控，release 天然跳过，计数不变——同既有约定，非漏测）
cargo test --lib bsp_bridge  → 19 passed; 0 failed（基线 18，净增 1）
```

真值表/对拍三窗完整读数见 §一/§二/§三。既有 18 例（前两轮全部语义验证锁）本轮零改动、零削弱
——本轮改动只涉及一类路径的遍历方式（点驱动 vs 事件驱动，输出集合等价）与两个诊断 bin 的
计数/退出码，不触碰 `apply`/`observe` 折叠逻辑（R2-HIGH-1/2 的修复面）本体。

## 七、逐条处置表（dispatch 修复单）

| # | dispatch 点名 | 处置 | 验收 |
|---|---|---|---|
| R2-HIGH-3（机器保证） | 一类路径绕过 `find_episode`，机器保证覆盖不到唯一有数据的点类 | 逐点改经 `find_episode` 反查 | 新增 poison 验证过的单测；18→19 例全绿 |
| R2-HIGH-3（检验域） | 20k 空域报 SUCCESS 是重言式 | `domain_size` + 三分退出码 | 20k EMPTY_DOMAIN(exit=2)、100k/300k PASS(exit=0) |
| R2-MED-2 | 二类锚 0/280 命中，「覆盖二类」断言不实 | 判据不改（已是结构可查路径）+ 分解诊断 + 撤回不实断言 | `ISSUE668_MED2_ANCHOR_BREAKDOWN` 三窗均 with_type1anchor 100%、anchor_hosts_first_class=0 |
| R2-MED-3 | 对拍规模空对空/二三类零信息量 | 分点类计数行 + 空域标注 | 三窗 `ISSUE668_BRIDGE_BATTERY_BY_CLASS` 逐点类落地 |
| R2-LOW-2 | 「不可执行」论证含两条虚障碍 | 删虚障碍，只留 `NestCandidateEvent` | battery bin 头注 + round2 报告 append-only 订正 |
| R2-LOW-3 | 行数/ADR 重复段/编号跳号三处小差 | 逐条订正 | round1 报告 append-only §九、ADR 标题层级、编号跳号照实确认 |
| 回归 | 前两轮已封的面不许推倒 | 逐面复核 | §六；18 例既有测试零改动全绿 |

不关票。
