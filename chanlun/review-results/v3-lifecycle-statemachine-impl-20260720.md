# V3 活假设状态机最小切片实装报告（NestLifecycleBook）

- 日期：2026-07-20
- 工位：实装工位（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`）
- 规格：`chanlun/review-results/v3-lifecycle-statemachine-implementation-card-20260720.md`（下称「卡」，§n 指卡内节号）
- 纪律自检：无 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入；`rust/Cargo.toml` 未触碰；v3 硬禁令合规（全部判据为确定性结构/力度谓词，无概率/统计推断、无回测验证、无 EMH 假设；测试全部为确定性合成序列）。

## 1. 改动面（授权文件 exactly 两处 + 本报告/重审材料）

| 文件 | 改动 | 说明 |
|---|---|---|
| `rust/src/theta_v0/classifier/nest_lifecycle.rs` | **新建**（1283 行，含 T1–T8 测试） | 唯一实装落点 |
| `rust/src/theta_v0/classifier/mod.rs` | +4 行（`pub mod nest_lifecycle;` 注册 + 注释，mod.rs:76-79） | 模块注册 |

`classifier/nest.rs` 只读引用未改（锚点 nest.rs:456-461/:802-810/:842-843/:987-989 仅作文档引用）。**既有行零改动**：level_view.rs / divergence.rs / p92 / p124 / Cargo.toml 全部未触碰——judge_at 一个 bit 不动（卡 §5.2）。注意：worktree 脏树含他工位未提交改动（level_view.rs/nest.rs/mod.rs 等），本切片 diff 与那批改动正交，未触碰、未回滚。

## 2. 实装 ↔ 卡规格对照

- **① NestLifecycleBook**（`nest_lifecycle.rs:319-321`）：`BTreeMap<LifecycleKey, NestLifecycleEntry>`，范式复用 persistent.rs:88-107（Pi 注册表/PersistentElement，唯一含 Invalidated 证伪终态的既有状态机 persistent.rs:59）、recursive_tower.rs:434-457（CpScanOwnership：生命周期挂 book 内对象 + 显式推进函数）、p92_nest_replay_postruling.rs:127-129（YieldBook，first-write-wins `entry().or_insert()` p92:751/:759）。`LifecycleKey = (level, side, kind, seg_a, seg_c_full, b_center_start)`（`nest_lifecycle.rs:78-86`）。trend 域 `seg_c_full` 未暴露（level_view.rs:699-702 确认分支收束丢弃全段坐标）⟹ 按卡 §6.3 白名单豁免桥接：`bridge_identity()`（`nest_lifecycle.rs:110-123`）= 除 seg_c 右端外全等判同身份，记 `Supersedes` 不记 Invalidated，模块头与字段文档均诚实标注「工程桥，非 E2E WireV1 EventKey，并轨前不得进入证书真值路径（卡 §9）」。
- **② 三态 + 五钟**：`NestEventState { Provisional, Confirmed, Invalidated }`（:205-213，Unresolved 出切片——卡 §2.4 声明=能力，模块头同声明）；五钟 `observed_at/first_provable_at/structure_end_at/confirmed_at/invalidated_at`（:229-251）。D1–D4 逐要素映射既有机器（模块头表，卡 §3）：trend 复用 `view.confirm_times`（level_view.rs:695-698 纯函数复用）——t* 重释为 `first_provable_at`（卡 §0.4），pan 复用 `segments_diverge_or`（divergence.rs:334-349）。**judge_at 与 first_provable 分离**：judge_at 字段/写入点/回填/CERT 主键/D3 统计全部不动，新五钟只活在新 entry；首次写入不后移（`first_provable_at.is_none()` 门 :449-477 + T4 锁定 + `assert_invariants` :558-619）。
- **③ 反超即失效**（:479-505）：`first_provable_at.is_some() ∧ ¬force_ok` ⟹ `Invalidated(ForceOvertake)`。trend：T5-OR 转假即终假（level_view.rs:619-621）的可观察面 = `confirm_times` Some→None ⟹ provider 回扩坐标改发未确认事件，本 book 接住既有机器丢成 None 的终假——零新判据。pan：活窗 `segments_diverge_or` 现算（:649-655），单调性（卡 §4.2）⟹ 首次翻假唯一且永久（T3 谓词层证伪器锁定）。**等力亦失效**（严格 < 口径，divergence.rs:331-333）按卡 §4.3 登记于模块头 + T2 等力子场景。身份消失第二触发 ⟹ `Invalidated(IdentityVanished)`（:530-553），原因码入 revision 载荷（T6 可区分）。
- **④ 单测 T1–T8**（:621-1280）：全部照卡 §7，确定性合成序列（T5 用真实 provider 夹具），无随机、无统计断言。详见 §4。
- **⑤ 测试**：见 §3，全绿零变红。
- **⑥ #43 重审材料**：`chanlun/escalate/v3-lifecycle-r43-review-request-20260720.md`（四问备料，不代裁）。

## 3. 测试输出（`cargo test --release --lib`）

```text
   Compiling newchan_rust v0.1.0 (/tmp/kimi-nest-mainline/rust)
    Finished `release` profile [optimized] target(s) in 17.31s
     Running unittests src/lib.rs (target/release/deps/newchan_rust-9cb5b83727a3eeb0)

test theta_v0::classifier::nest_lifecycle::tests::t1_trend_force_overtake_invalidates_and_terminal_absorbs ... ok
test theta_v0::classifier::nest_lifecycle::tests::t2_pan_live_window_force_overtake_and_equal_force_boundary ... ok
test theta_v0::classifier::nest_lifecycle::tests::t3_no_revival_after_invalidation ... ok
test theta_v0::classifier::nest_lifecycle::tests::t4_clock_invariants ... ok
test theta_v0::classifier::nest_lifecycle::tests::t5_bit_exact_guard_provider_stream_untouched ... ok
test theta_v0::classifier::nest_lifecycle::tests::t6_identity_vanish_on_structure_switch ... ok
test theta_v0::classifier::nest_lifecycle::tests::t7_completion_time_recheck ... ok
test theta_v0::classifier::nest_lifecycle::tests::t8_trend_identity_migration_whitelist ... ok

test result: ok. 1770 passed; 0 failed; 129 ignored; 0 measured; 0 filtered out; finished in 1.02s
```

**全绿零变红**。基线对账（诚实声明）：任务书基线 1755 passed 对不上当前脏树——本切片 +8 测试，扣除后既有通过数 = 1762，与 1755 差 7；该差值归属 worktree 内他工位未提交改动（`git status` 显示 level_view.rs/nest.rs/divergence.rs/mod.rs 等在本人开工前已 M），非本切片引入。本切片前后对照：0 failed → 0 failed，无一既有测试变红（全量 1899 = 1770 passed + 129 ignored 一次跑完，无过滤）。

## 4. T1–T8 验收明细（卡 §7 逐条）

| 测试 | 卡要求 | 实装与断言 | 结果 |
|---|---|---|---|
| T1 `t1_trend_force_overtake_invalidates_and_terminal_absorbs` | trend 反超可触发 | observed=149 →（收束迁移 Supersedes）first_provable=155（t*）→（T5-OR 终假回扩）invalidated=169 / ForceOvertake；t=179 再 advance 零输出（终态吸收）。同时证明「现状丢成 None 的反超被接住」 | ok |
| T2 `t2_pan_live_window_force_overtake_and_equal_force_boundary` | pan 活窗反超可触发 + 等力边界 | a 窗固定、c 活窗延展：as_of1=129 三通道成立（first_provable=129），as_of2=139 三通道全假 ⟹ Invalidated；等力子场景 c 面积==a 面积（60==60）严格 < 不成立 ⟹ 等力亦失效（卡 §4.3） | ok |
| T3 `t3_no_revival_after_invalidation` | 单调不复活 | 谓词层：翻假后延展窗 `segments_diverge_or` 仍假（单调性可执行证伪器）；状态机层：延展窗与构造性「复活」弱窗（右端回缩）均零新 revision、revision 数零增长 | ok |
| T4 `t4_clock_invariants` | 钟不变量 | observed/first_provable 不后移（125 写入后 130/135 不变）；同 as_of 重复 advance 零 delta（幂等）；`assert_invariants` 全列（钟序、≤ as_of、终态互洽、反超 invalidated ≥ first_provable） | ok |
| T5 `t5_bit_exact_guard_provider_stream_untouched` | bit-exact 护栏 | 真实夹具（复刻 level_view.rs:1169-1239 extended_windows + :1424-1463 R1 全合取 MACD，补 retest 块满足 structural_pair_span level_view.rs:511-524）：advance 后入参事件流逐字节不变 + 挂/不挂 book 重跑 provider 输出相等（PartialEq + Debug 序列化双判） | ok |
| T6 `t6_identity_vanish_on_structure_switch` | 身份消失 | pan 窄锚（seg_a=(50,69)）→ A′ 回退（seg_a=(20,39)）切换 ⟹ 旧 key `Invalidated(IdentityVanished)`@99，entry 保留（禁删除模拟失效），与 ForceOvertake 原因码可区分 | ok |
| T7 `t7_completion_time_recheck` | 完成时复核防「曾经弱过」 | (a) 完成窗仍弱 ⟹ Confirmed，structure_end=confirmed=139，first_provable=129 不后移；(b) 完成窗已反超 ⟹ Invalidated(ForceOvertake) 而非 Confirmed，confirmed_at=None（024:24 机械表达，E2E §4.1:151 禁项防护） | ok |
| T8 `t8_trend_identity_migration_whitelist` | trend 身份迁移豁免 | 收束 [120,149]→[120,155] 记 Supersedes（迁移链 `superseded_from` 留痕、钟不动、无 Invalidated）；负面对照 seg_a 改变 ⟹ 白名单不越界 ⟹ IdentityVanished | ok |

## 5. 边界与诚实登记（090：声明=能力）

1. **交付物 = 状态机核心 + 消费契约**，不是生产接线。pan 活窗产出机（`locate_pan_div_structure` 沿用 + `c_start_live` 段序对齐校验，卡 §3）需要 provider 暴露面，本切片授权仅新模块 + mod.rs，未触碰 level_view.rs——活窗由调用方按 `PanLiveWindow` 契约喂入（`nest_lifecycle.rs:266-284` 文档明载），trend 结构完成信号经 `structure_completed` 通道喂入（卡 §4.3 判据 Active→Completed / SegmentTermination 属调用方）。生产 bin 接线（卡 §6.2 伪码的 prefix 循环）属后续切片。
2. **白名单工程桥**（卡 §6.3）：trend 域 `seg_c_full` 未暴露，身份键暂用事件产出坐标 + 桥身份（除 seg_c 右端外全等）吸收收束/回扩/活窗延展三形态。这不是 E2E WireV1 EventKey；并轨前不得进入任何证书真值路径（卡 §9 限制已写入模块头）。pan 活窗右端随 as_of 前进产生逐 prefix `Supersedes` revision——桥的设计内代价，非缺陷（卡 §3「c 窗右端随 as_of 前进是设计内行为」）。
3. **Unresolved 出切片**（卡 §2.4）：交付三态链，非 E2E-D5 全四态；proxy 前提四态与 provider 失败原因通道缺（inventory:104），与 DeferOrphan 重判一并列入后续切片。
4. **feed-every-prefix 契约**：钟序不变量 `observed ≤ first_provable`（卡 §5.3.2）在「每 prefix 投喂」下成立（pair 产出先于 T3 确立，t* ≥ 首见 prefix）；跳 prefix 回填可构造 t* < observed 的越约输入，出切片（T8 负面对照注释明载）。
5. **完成但不背驰**：pan 完成窗 force 假且从未可证 ⟹ 诚实滞留 Provisional（窗口已冻、永不可证），不伪造 Invalidated——反超定义要求「曾可证」（卡 §4.1），代码注释 :526-529 明载。
6. **#43 未动**：N^δ 装配仍只消费已闭合完整 `c_p`（nest.rs:802-810/:987-989 未触碰）；本 book 是观察记录，不进 `d_parent_interval_snapshot/terminal` 输入，不改 `divergence_confirmed` 布尔口径。重审材料只备料不落锤。
7. **出切片项**（照卡 §9 登记）：WireV1 全量 EventKey/StateKey/修订链、谱系两钟 opened/closed（E2E-N5 lineage 半部）、跨级证伪（043:30）、024:28 面积乘 2 外推、postcondition 诊断钟、Lean 侧 `ActiveTail↔OpenTailSystem` 桥（Parse.lean:337）。
