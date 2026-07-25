# #237 实装证据：断言门按方向拆查（该级该方向——卖批查多侧/买批查空侧）

- 日期：2026-07-25
- 性质：实装证据落盘（TDD red-green、m3/m6、三锁、「死给你看」见证）。实装基线 HEAD `2459cd9524`（#233 结案）。
- 票面：issue #237（parent #193 spec 断言①口径修正，用户裁定 2026-07-24）；蓝图依据 #234（`.chanlun/review-results/full-close-direction-semantics-20260724.md`：出场方向锁定「多头声部由卖点证书平仓；空头声部由买点证书平仓」+ 分侧账禁净额 P_sep，拆查为唯一一致查法）；遗留来源 #233 §5（m3 win9 bar=232810 (1,470) 误报面 + 「门口径不对称可达路径」登记）。
- 构建口径（与 #233 同）：`RUSTC_WRAPPER=$REPO/.chanlun/locks/cargo_gate.sh RUSTFLAGS="-C debug-assertions=on" cargo test --release`（门在纯 release 被剥离）。

## 1. 实装内容（2 文件机制 + 1 文件账面查询 + 4 枚新测试）

### 1.1 断言①批次边界门按方向拆查——`rust/src/theta_v0/strategy/coverage.rs`

- 残余计算加方向过滤 `.filter(|s| s.side == l.dir)`：一类卖批（被关腿 dir=Long ⟺ Exit_v δ(γ)=−σ_v 的生产同义式）⇒ 查该级**多侧**分量=0；一类买批（被关腿 dir=Short）⇒ 查该级**空侧**分量=0——S2「该级该方向」的良构形式。
- 顺父级联 Short 腿（FollowParent×Short，当 bar role_v=FollowParent ⟹ Core{level}，**#185 映射不动**）归空侧分量：一类卖批下合法存活（蓝图动作表「空头声部×卖点=同向信号：持有/加仓/记录」），不计入卖批残余；其合法出场路径 = 一类买点/元素终结/父关连清/风险强平（#234 Q4.3）。
- 探针两枚（评估/违例）语义注释随口径更新；panic 消息改带方向标签（`Core{L} {side}侧`）。fold 规则2（一类全平该级全部**反向**命中腿）零触碰——拆查只动门口径，不动决策轨迹（bit-exact 保持：门是只读观测面）。

### 1.2 断言②（账面 balance）同口径拆查——`rust/src/theta_v0/backtest/runner.rs` + `rust/src/theta_v0/strategy/account.rs`

- 批次收集 `t1_core_levels: Vec<u32>` → `Vec<(u32, VoiceSide)>`：方向键 = 被关腿 `leg.dir`（与断言① `l.dir` 同源同口径）。
- 批末硬门改查 `balance_side(Core{lv}, side)==0`（原净额 `balance` 查零废除——净额同时掩盖双侧残余，ker N 不可识别，#234 Q3.3）。
- `account.rs` 新增 `ParallelAccountLedger::balance_side(account, side)`（**派生视图**：现算投影，非存储）：按实例键 `key.position.side` 分侧求和；净额 `balance` 口径不动（两视图共存）。顺父级联 Short 腿实例归空侧（#185 映射不动）。
- 断言③（二类卖身份/残余纠错）与 `reason_of_reverse_close` 分流判据（core_residual 净额读数）**零触碰**——#199 机制非本票范围。

## 2. TDD red→green（票面验收 1）

| 测试 | 文件 | 锁定面 | red 证据 |
|---|---|---|---|
| `t1_target_zero_assertion_sell_batch_checks_long_side_only` | coverage.rs | 卖批⇒多侧查零+空侧存活（(1,470) 型最小复现：two_parent_tower，Long 子腿被一类卖关、FollowParent×Short 子腿存活） | 旧口径 panic：Core{0} 残余 300（存活 Short 腿被计入）；临时摘除方向过滤复现 panic（生成史在案） |
| `t1_target_zero_assertion_buy_batch_checks_short_side_only` | coverage.rs | 买批⇒空侧查零+多侧存活（镜像） | 同上临时摘除实验：Core{0} Short侧 残余 300 panic（双向对称 red） |
| `balance_side_splits_account_balance_by_voice_side` | account.rs | 分侧投影正确性 + 卖批后净额≠0（旧净额口径误报面复现）+ 买批平空=0 | 编译红（`balance_side` 缺席）→ 绿 |
| `t1_core_close_zero_assertion_buy_batch_checks_short_side_only` | runner.rs | π loop 生产路径：一类买批全平级联 Short（L2 空父塔），断言②方向收集/`balance_side` 消费链真实工作、探针=1 违例=0 | 平凡域（合成层异侧共存不可伪造，见 §5） |

- 双向 red 验证法：实装后临时注释 `.filter(|s| s.side == l.dir)` 复跑两枚 coverage 测试——双双 panic（卖批枚 Core{0} Long侧 300、买批枚 Core{0} Short侧 300），恢复后双绿。
- `cargo test --lib`：**1835 passed / 0 failed**（#233 基线 1831 + 新增 4 枚）。

## 3. m3 / m6（票面验收 2）

- **m3 硬门全窗转绿**（debug-assertions on，1642.8s）：win7 |ledger|=909、win8=856、**win9=830**（过 bar 232810——(1,470) 误报消解 ✓ 票面主目标）、**win10=830**（(2,50) 三处同族误报消解 ✓ 同族核对）、win11=596——全 5 窗零违例（`Σ|ledger|=Σkept=Σ|records|=4021`）。typed 窗计数与 #233 降级跑（830/830/596）及硬门跑（909/856）**逐位一致**——拆查是只读门口径，决策轨迹零翻动（fold 规则2/开仓/镜像均未触碰）。
- **m6 三窗 + m8 同窗三系统**（debug-assertions on，534.1s）双绿：resid=-1.79e-6/-2.18e-6/-2.29e-6（与 #233 逐位一致，资金无泄漏物证），m8 三窗守恒硬校验过、报告落盘正常。
- 断言①②③ 全程违例=0（新口径，m3 硬门全窗 + BTC train 窗双证）。

## 4. 「死给你看」见证（票面条款）

观测轨 = `voice_verdicts`（#201 裁决轨：每 bar 每持仓声部恰一枚显式裁决，restore 腿同覆盖）+ typed_ledger（若曾登记）；测试 `runner.rs::tests::m3_follow_parent_short_leg_termination_witness`（#[ignore]，BTC 真实数据，352.3s 实跑）：

| 目标腿 | 身份核实 | 首现 bar | 终结 bar | 终结路径 | 判定 |
|---|---|---|---|---|---|
| (1,470) | FollowParent×Short、parent=Some((2,107)) ✓（票面身份逐字一致） | win9 228965（restore 入飞，在飞 5580 bar——232810 误报点在其存活中段） | win9 **234544** | **CloseRoot（一类买点平空）**——Exit_v 方向锁定 δ(γ)=+1=−σ_v 的合法出场（#234 Q4.3 第 1 条） | ★自然死亡确认 |
| (2,50) | FollowParent×Short、parent=Some((3,11)) ✓（#233 §5 同族身份） | win9 121254 | win9 **132158** | **ReduceCore（三类买减仓平空）**——买侧出场证书方向锁定同族 | ★自然死亡确认 |

- **「不误报 ≠ 不清理」铁证**：(1,470) 在误报点（232810，一类卖批——非其出场域）后 1734 bar 被一类买点真实平掉——拆查只是停止把合法存活误报为违例，该腿的合法出场确实发生。两目标均一见终结即确认，无新僵尸（票面停手条款未触发）。
- (2,50) 在 win10 的三处旧口径违例（#233 §5：137398/140590/141033）属同 carrier 新世代；其消解由 m3 win10=830 全窗零违例核对 ✓（硬门不再误计入）。
- 该腿不入 typed_ledger（restore 腿未经 open_trades 登记，「表中无登记 ⟹ 不入 ledger」）——裁决轨是其在飞/离场的完整可观测面，此见证路径如实声明。

## 5. 090 如实标注

- **runner 合成层「异侧存活共存」不可伪造**：同级「多侧＋级联 Short」共存只能经 restore/跨 bar 结构路径形成（任何同级反向候选开仓必先关对方——规则2 方向锁定；(1,470) 的共存本体即 #233 §5 登记的 restore 形态）。runner 层合成测试因此锁「方向收集 + 拆查消费链 + 平凡域零违例」；异侧共存的新口径零违例由 m3 全窗真实生产数据承担（票面验收 2 主体）。
- **断言② m3 无现存违例面**：#233 降级实验在案「断言②三窗零违例」——(1,470) 类 restore 腿未经 open_trades 登记、不在账实例内，净额恰好为零；本票拆查是口径一致性收口（防未来对称误报 + 净额掩盖漏报），非既有违例修复。拆查语义 = 收窄（违例子集 ⊆ 旧全集），不产生新违例方向。
- **BTC train 16000 窗三锁双口径零翻动**（票面验收 3）：四枚（`btc_type2_open_short_channel_witness` 基线锁 50 笔 / `btc_type2_residual_correction_witness` 探针对账 / `btc_prune_leg_exit_type_matches_account_identity` 47 笔跨账一致 / `typed_ledger_btc_smoke` 五枚举守恒）× debug-assertions on/off 双口径全绿、逐位一致——typed=50、五枚举 CloseRoot=32/CloseShortDiff=17/ReduceCore=1/Hold=0/RiskExit=0、prune=47、OpenShort=2、开仓分布 Core{0}×Open=22/Core{1}×Open=3/ShortDiff×Open=17/Short×Open=6、探针（新口径）断言①评估=1 违例=0、断言②评估=0 违例=0、断言③前半=1（同级 Core 残余=0）、残余硬门=1——与 #233 在案基线逐项一致，基线锁 50 笔不动（零翻动形态 = 无需逐条对账）。三锁零翻动的机制理由：门是只读观测面（断言块不改决策/订单/镜像），拆查只收窄检查集。
- **clippy --lib**：改动三文件零新增命中（coverage.rs 17 处命中行号全部落在本票 hunks 之外——既有；account.rs/runner.rs 0 命中）。

## 6. 验证清单（本报告所据实跑）

1. `cargo test --lib`：1835 passed / 0 failed（4 枚新测试 red→green 在案）。
2. m3 硬门全窗（debug-assertions on，1642.8s）：全 5 窗零违例，win9/win10 误报消解（§3）。
3. m6 三窗 + m8 同窗三系统（debug-assertions on，534.1s）：双绿，resid 与 #233 逐位一致（§3）。
4. 「死给你看」见证（352.3s）：(1,470) CloseRoot@win9 bar 234544（一类买点平空）、(2,50) ReduceCore@win9 bar 132158（§4）。
5. BTC train 三锁+smoke × debug-assertions on/off 双口径：四枚全绿、逐位一致（§5）。
6. clippy --lib：改动三文件零新增命中（§5）。
## 7. code-review 两轴（`.agents/skills/code-review`，fixed point=HEAD 工作区 diff）

- **Standards 轴**：**硬违规=0**。两项仓规清单触线判 judgement call（coverage.rs/runner.rs 文件长度、新测试函数 >50 行——既有超限文件上续写测试，仓内同文件 `mod tests` 惯例延续，非本票新引入）；两项 Fowler smell judgement call 不采纳在案：Duplicated Code（`side_sum` 闭包在两枚镜像测试逐字重复——卖/买镜像属刻意对称，提取 helper 反损「每测试独立规格」可读性）；Primitive Obsession 轻（runner.rs 批末排序键 `side as u8`——给 `VoiceSide` derive `Ord` 会臆造领域序并扩大公共类型改动面，局部 `as u8` 键是诚实局部方案）。
- **Spec 轴**：(b) scope creep 无发现；(c) 实现有误无发现；四项特别核对全过——①方向映射正确（`reverse_signal` 持多遇卖侧 ⟹ 被关腿 Long ⟺ 卖批查多侧，同义转换成立）；②断言②与①同源（方向键同取被关腿 `leg.dir`，实例键 `position.side` 与 `balance_side` 过滤键口径一致）；③#185 `identity_of` 零触碰；④无越界机制改动（fold 规则2/决策轨迹/净额 `balance` 口径零触碰，门保持只读观测面）。(a) 唯一 partial 提示「死给你看见证 harness 有、落盘无」——落盘由本文档 §4 承担（评审范围仅三文件 diff，未见本文档；#233 先例同体式）。
- **结论：两轴硬违规清零**（票面验收 4）。

*report 完。生成史（红→绿、双向临时摘除实验、runner 合成层不可伪造面）均如实保留。*
