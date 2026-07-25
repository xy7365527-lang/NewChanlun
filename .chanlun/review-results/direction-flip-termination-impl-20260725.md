# #233 实装证据：方向翻转显式化为声部终结事件（父翻向=父终结，蓝图两步形）

- 日期：2026-07-25
- 性质：实装证据落盘（票面验收 4「持久层方向不可变机器锁/审查证据落盘」）。实装基线 HEAD `d7fa0e01a5`（#226 结案）。
- 票面：issue #233；裁决：#227（HITL 2026-07-24）；依据：#230 蓝图考证（voice-direction-flip-doctrine-20260724）、(2,98) 生命周期考古（leg-2-98-lifecycle-20260724）。
- 构建口径（与勘察同）：`RUSTC_WRAPPER=$REPO/.chanlun/locks/cargo_gate.sh RUSTFLAGS="-C debug-assertions=on" cargo test --release`（门在纯 release 被剥离）。

## 1. 实装内容（3 文件机制 + 1 文件见证基线）

### 1.1 持久层方向不可变机器锁（票面要求 1 + 第 4 条）——`rust/src/theta_v0/strategy/persistent.rs`

- upsert（tree 段与 candidate 段**同一闭包**）改 I2 方向守卫：已存在条目方向冲突 ⟹ **拒绝静默覆写 dir**（dir 永固首见方向），其余 snapshot 字段（level/lambda/rho/structural_parent_id/snapshot_present）照刷（§9 rule 1 坐标/关系可变、I3）；冲突显式计数 `FlipGuardProbe{tree_blocked, cand_blocked}`（thread_local 恒在计数，断言①②③同款模式，release 可观测）。
- candidate 段同路径守卫 ⟹ χ 域外候选 flip-flop 不再覆盖 registry dir（票面第 4 条「同路径一并守卫」；考古副产 (2,98) registry dir 与物理背离面消除）。candidate 胜序收窄为「除 dir 外全字段」（on2w3 断言2 注释已收窄）。
- 守卫形态 = **拒绝**（非异常）：翻向是合法事件，终结事件化在 coverage 对位层兑现——本层是持久防线的机器锁（I2 anc.pdf p.4–5/27 §7）。

### 1.2 翻向显式化为终结事件（票面要求 2，#227 裁决）——`rust/src/theta_v0/strategy/coverage.rs`

- `held_leg_tree_index_indexed` 加 I2 方向守卫：Exact 命中且树元素 `eps ≠ leg.dir` ⟹ 新变体 `HeldLegMatch::Flipped`——**frontier/confirmed 同判**（#227 焊缝规则：凡方向不一致即终结）。
- held 循环 Flipped 分派：旧世代腿**不保留**（不当 Exact 对位、不重注册、不 restore——**方向盲 ID 对位复活路径废除**，`element_as_leg` 静默改向路径自此不可达：Exact 时 `eps==leg.dir` 恒成立），进当 bar 翻向种子 `flipped_seeds`（探针 `AncokProbe::held_flip_terminated`）。
- `flipped_seeds` 并入 `step_active_set_with_subtree_close` 的关闭种子：**现成 𝒟_x^† 子树清仓**连清后代（不新造清仓逻辑）——只作用 A_t 段（ℬ_x 段反手/新世代候选不连坐，#183 分段/ID-3 反手保护同构）。
- **close 事件入轨**：翻向腿+连清腿不在 close 桶、不在 next_active ⟹ 自动落 `StepTrace.silent_drops` ⟹ runner 既有消费链（typed ledger `via_structural_prune` + 账户镜像 `StructuralPrune` + TW 腿计数）——零 runner 机制改动。
- **新世代重登记**（蓝图两步形②）：翻向同 bar ℬ_x 候选经 open 父注入 restore 的 id_idx 树复用（新方向元素在场）⟹ AncOK 准入 ⟹ #220 idx 配对 opened 外化 ⟹ runner A9 generation+1 登记（新 posId/generation）；同 id 反手候选同受 ℬ_x 段保护（ID-3 兼容）。

### 1.3 子树清仓数济判据——`rust/src/theta_v0/strategy/exit.rs`

- `subtree_close` 链上溯断裂（parent_id ∉ by_id——翻向父旧世代不保留于 A）时，断裂点 `parent_id ∈ seeds` ⟹ **连坐**（父已终结，后代不得因父缺席而漏清）；∉ seeds ⟹ 现状（孤儿归声部层 AncOK，#148 验收2 路径不回退）。仅扩「父是种子但不在 A」一面；runner/nautilus 级联（投影链无断裂面）行为不变。

## 2. TDD red→green（票面验收 1）

先红（6 枚断言失败在案）后绿；后补 T7（直接绿，实装已覆盖）。

| 测试 | 文件 | 锁定面 |
|---|---|---|
| `i2_direction_guard_blocks_silent_dir_overwrite` | persistent.rs | tree 段守卫：dir 永固+字段照刷+探针=1 |
| `i2_direction_guard_candidate_segment_same_guard` | persistent.rs | candidate 段同守卫（χ 域外 flip-flop 同路径） |
| `parent_direction_flip_terminates_voice_and_liquidates_subtree` | coverage.rs | 父翻向=父终结+Exact 子腿连清（next_active 空） |
| `flip_termination_externalizes_close_track_via_silent_drops` | coverage.rs | close 事件入轨（silent_drops 含父子，closed/opened 桶不染） |
| `flip_same_bar_new_generation_reregisters_via_open_candidate` | coverage.rs | 新世代登记：ℬ_x 候选不连坐+新世代父树复用在场（回归锁） |
| `flip_same_bar_reverse_candidate_reregisters_new_generation_same_carrier` | coverage.rs | 同 carrier 反手=新世代立即重登记（ID-3 兼容） |
| `subtree_close_liquidates_descendants_of_seed_absent_from_active` | exit.rs | 父不在 A 数济判据（deepest-first 保留；无种子现状不变） |

复合检查（不回退）：#216（`held_leg_id_hits_candidate_copy_keeps_held_identity`、open 判重三规则）、#220（`opened_restore_leg_not_externalized_for_same_carrier_candidate_pair`）、#226（`open_parent_restore_skips_closed_seed_no_resurrect`、`open_reverse_candidate_on_closed_carrier_unaffected`、`open_parent_restore_mid_chain_seed_aborts`）守护测试全绿；`cargo test --lib` **1831 passed / 0 failed**（含 7 枚新测试）。

## 3. m3 / m6（票面验收 2）

- **m3 硬门全窗**（debug-assertions on，843.8s）：win7 跑完 |ledger|=909、win8 跑完 |ledger|=856（基线 535/539——typed 翻动=翻向终结入轨+新世代重开，预期内）；**win9 过 bar 211851/222794**（(2,98) gen-10 已随父终结，Core{2} 残余 14.72 实例消解 ✓ 票面两具体目标达成），红于 **bar 232810 Core{1} 残余 2.774918787705654**（另案面，见 §5）。
- **归因降级全窗**（断言①门降级为 eprintln，断言②/③ 硬门保持激活）：win9 跑完 |ledger|=830，**断言②全程零违例** ✓；win10/win11 结果见 §5 占位。
- **m6**（525.5s）三窗（p3fold/wf7/wf8）全跑完保持绿：resid=-1.79e-6/-2.18e-6/-2.29e-6（≈0 资金无泄漏物证），硬门全程零违例。

## 4. BTC train 16000 窗三锁双口径（票面验收 3）

**基线对拍**（`git archive HEAD` /tmp 副本复跑，逐位复核 #200/#209 在案基线）：26 笔 typed、五枚举 CloseRoot=8/CloseShortDiff=9/ReduceCore=7/Hold=2/RiskExit=0、prune 6 笔、断言①②③违例=0。

**新轨迹**（debug-assertions on/off 双口径逐位一致）：**50 笔 typed**、五枚举 CloseRoot=32/CloseShortDiff=17/ReduceCore=1/Hold=0/RiskExit=0、prune 47 笔、断言①②③全程违例=0、47 笔 prune 跨账一致（exit_type⟺account）、smoke 五枚举守恒。三锁基线按 #179/#200 先例随票更新（runner.rs `btc_type2_open_short_channel_witness` 基线锁 26→50 + 逐条对账注释）。

**翻动逐条解释**：

| 面 | 基线 | 新 | 机制归因 |
|---|---|---|---|
| typed 总笔数 | 26 | 50（+24） | 翻向终结旧世代腿经 silent_drops 入轨（via_structural_prune） |
| CloseRoot | 8 | 32（+24） | 非 ShortDiff 翻向/连清腿归 core structural exit（silent_drop_exit_type 单源） |
| CloseShortDiff | 9 | 17（+8） | ShortDiff 翻向/连清腿同轨 |
| prune 腿 | 6 | 47（+41） | 基线 6 笔全同 entry/同型/同账户、exit 一致提前（(0,52) 6306→6175、(0,61) 8366→7011、(0,86) 10365→9941、(0,92) 10422→10368、(0,114) 14653→14286、(0,128) 15625→15356）——旧轨翻向续命至 AncOK/Stale 剪，新轨翻向 bar 即终结；+41=翻向父终结+子树连清 |
| ReduceCore | 7 | 1（−6） | 二类减仓对象核心腿在二类信号前已翻向终结，减仓触发面消失 |
| Hold | 2 | 0（−2） | 窗尾 censored 持仓在窗尾前已翻向终结 |
| 开空单 Short×OpenShort | 0 | 2 | 翻向反手/新世代重登记使「二类×无父空根」形态真实显现（#200 合成见证机制的真实数据实例；逐笔标注断言对 2 笔成立） |
| 开仓分布 | Core{0}×Open=10 / ShortDiff×Open=10 / Short×Open=3 / Core{1}×Open=3 | 22 / 17 / 6 / 3 | 翻向终结释放 carrier ⟹ 新世代重登记（旧轨候选被 #216 规则①「持仓身份优先」让位；A9 generation+1） |
| 断言①②③ | 违例=0 | 违例=0（断言①评估=1、断言②评估=0、断言③前半=1） | 硬门全程保持 |

## 5. 另案面（090 如实标注）

- **m3 win9 bar=232810 Core{1} 残余 2.774918787705654**：(1,470) FollowParent/Short、parent=Some((2,107))。**归因对拍**（基线 HEAD+断言①降级全窗复跑，1589.8s）：基线**同 bar 同腿同角色**残余 4.784784502310997——该形态既存（被 222794 先炸掩盖，与 #220/#226 先例同构）；q 差异（4.78→2.77）为 211849 起轨迹分叉（(2,98)/(3,22) 等新轨迹）下 sizing 上下文演化的正常后果。该腿 = 顺父级联 Short 仓（父 (2,107) 在场），一类卖 lvl-1（平 Long）不关 Short ⟹ 残余求和（当 bar role_v=FollowParent→Core{1}，#185 映射）计入——**#227 裁决预告的「门口径不对称（冻结 entry_v vs 当 bar operation_role）若修复后仍有可达路径，另案处理」的可达实例**。断言①门零触碰（#226 纪律），另案处理。
- **win10 同族面**：(2,50) FollowParent/Short、parent=(3,11) 在 137398/140590/141033 三处违例（q=8.64/8.33/8.30；基线同 bar 同腿同角色 16.03/15.75/15.65——同样既存形态，q 差异=轨迹分叉 sizing 后果）。该腿顺父级联**合法存续**（父 (3,11) 在场 Short、自身未翻向——本票机制对「父/自身翻向」才会终结它，二者均未发生），残余同属门口径另案面。**win11 全程干净**（无违例）。
- **win9/win10/win11 断言②核对**（票面验收 2）：归因降级版断言②/③硬门保持激活，三窗全跑完（win9 |ledger|=830、win10 |ledger|=830、win11 |ledger|=596）——**断言②全程零违例** ✓；win7/win8 断言②在硬门全窗跑（843.8s 版）已证 ✓。
- **restore-from-registry 旧方向复活面（按票面第 4 条体式记录另案，不扩大本票）**：`restore_ancestor_chain_from_registry` 经 registry 取元素重建（`eps: pe.dir`）的路径，在 I2 守卫下恢复的是**首见方向**（旧世代方向）。当 bar 面已闭环：翻向 pid 必在树（Flipped ⟺ id_idx 命中）⟹ restore 永远先经 id_idx **树复用**（新世代元素），registry 取元素路径当 bar 不可达。**跨 bar 面未闭环**：翻向元素退出 snapshot 后，其 pid 经 registry restore 可以旧方向复活入 raw（「翻向后旧世代不得复活」在此一路径未机器锁定）。实装层 ElementId 不含方向（pid 无法区分新旧世代），封锁该面需世代身份改造（超出本票）；与勘察 §6 副产物②（χ 域外候选/registry dir 对 restore 取用的方向保真影响，#226 §5 在案留疑）同案归并，另票处理。
- **runner.rs:2303 注释**「关闭的腿（buckets.close）在 registry 中标记 invalidated（§9 rule 5）」与生产行为不符（registry 无生产 invalidated 写点，runner.rs:6238 自证）——既有不符，非本票引入；翻向腿的 registry 条目经 #233 守卫保持首见方向、invalidated=false。
- **#199 兼容声明**（code-review Spec 轴 (b) 采纳）：ReduceCore 7→1、CoreResidualCorrection 5→1、二类卖 ReverseType2 3→1——#199「仅残余才纠错」机制的**生成逻辑零改**（`reason_of_reverse_close` 单源未触碰），触发面随轨迹演化（翻向终结使二类减仓/纠错对象提前离场），与 #209 注释「理由轴分桶随轨迹演化」先例同款；#216/#220/#226 语义不回退（§2 守护测试全绿）。
- **m3 typed 翻动**：win7 535→909、win8 539→856、win9（降级）528→830、win10（降级）507→830、win11 347→596——翻向终结入轨+新世代重开（机制目的本身）；win9/win10 的 (2,98) 同族僵尸腿（基线降级实证 (2,98) 222794-225787 六次违例**全部消解** ✓ 门案随解；(1,470)/(2,50) 属顺父级联存活合法面，门口径另案如上）。

## 6. 验证清单（本报告所据实跑）

1. `cargo test --lib`：1831 passed / 0 failed（7 枚新测试 red→green）。
2. m3 硬门全窗（843.8s）：win7/win8 过、win9 过 211851/222794、红于 232810（§5 另案）。
3. m3 归因降级（断言①降级，断言②/③硬门保持）：基线全窗 1589.8s（全违例清单在案）；新版 win9 355.8s（唯一违例 232810 (1,470)）+ win10/win11 512.2s（(2,50) 三处、win11 干净；断言②三窗零违例）。
4. m6 三窗 525.5s 绿。
5. BTC train 三锁+smoke 双口径：基线 26 笔对拍复跑一致；新轨迹 50 笔双口径一致，四枚全绿。
6. clippy --lib：改动文件零新增命中（persistent.rs 0 命中；coverage.rs/exit.rs 全既有）。

*report 完。生成史（红→绿、归因实验、另案面）均如实保留。*
