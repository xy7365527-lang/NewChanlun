# #202 阶段 C（仅替换 P2/P3）翻动逐条核对记录

- 日期：2026-07-24。基线：main@1447773411（#201）。实跑环境：本仓工作区（新内核）vs
  `git archive HEAD` 导出副本（旧内核），BTC train 16000 窗（PREREG 窗 OOS 前半，
  与 #198–#201 各见证同窗）。
- 总结论：**`cargo test --lib` 1828 绿、零翻动**（= #201 基线 1819 + 本票新增 9：channel 2、
  interp 4、coverage 2、runner 1）；**BTC typed 轨 26 笔 = 26 笔，一处分歧段（bar 10365）
  翻得对**（票面明知的多腿结构差，裁决级证据链见 §4），五枚举 CloseRoot 9→8 /
  CloseShortDiff 8→9（总 26 不变），下游 7 笔 units 连锁微差（entry/exit/typed 全不变）。

## 1. 实装形态（最小改动，3 改动点 + runner 零改）

1. `channel.rs`：`find_reverse` 补 `nest_confirmed` 证书门（对齐 interp 规则2，「同单源
   判据」前提；channel 烤料全 confirmed ⟹ 自测零翻）；新增 `cert_close_trigger`
   （P2/P3 域判据 + 触发归因**单源**），`voice_predicates` 的 P2/P3 段改经它（不镜像）。
2. `interp.rs`：新增 `interpret_with_external_closes`（规则2 外部化 fold 变体）：
   external（channel 裁决关闭集）预置后 fold 原逻辑零改（`!was_closed` 自动跳过 ⟹
   规则2 只作用 S 组 ShortDiff 腿 = P4 域散装现状）；external 触发候选计入 closed_any
   （一类消费即止、二类 dual-effect 照常，#200 OpenShort 通道等价保持）；归因序
   `(≺_Θ(trigger), active_idx)` 稳定序（与原 fold 归因序同构）。`interpret`/
   `interpret_with_close_triggers` 本体零改（自测/∃! 锚不动）。
3. `coverage.rs`（`pi_theta_step_traced` 正常路径）：C 组（entry_v≠ShortDiff）逐腿经
   `channel::cert_close_trigger` 裁决喂 external；S 组维持散装。环6/7、TW/P1 分支、
   typed 计算（`reverse_exit_type(entry_v, trigger.class)` 与 channel 裁决同单源）、
   verdicts 组装、断言①②全部零改。`runner.rs` 消费段零改（trace 形态不变，
   AccountOrder 经既有 `identity_of`/`reason_of_reverse_close` 单源自然兼容）。

## 2. 备忘 §3a 清单逐条核对（coverage `pi_theta_step_traced` 系 + #201 verdicts 系）

全部**不翻**。共同证据：合成夹具全部落在两链同构域（单腿单候选：channel 逐腿判据
≡ fold 首个命中，判据同单源——`find_reverse`+证书门 ≡ 规则2 反向命中+证书门，
`reverse_exit_type` 同一函数；一类多腿：channel 每腿独立 ≡ fold 一类全平，由本票
`p23_channel_type1_multi_leg_bit_exact_with_fold` 实证）。散装域（S 组/TW/P1/记录/
开仓）判据零改。

| # | 测试（现名） | 结论 | 逐条理由 |
|---|---|---|---|
| 1 | `pi_theta_step_traced_opened_and_bitexact` | 不翻 | prebuilt 路径同走换源后的正常路径（单源），同变 ⟹ bit-exact 保持；opened 结构（规则3/4 域）零改 |
| 2 | `pi_theta_step_traced_reverse_close_attribution` | 不翻 | 单腿单候选同构域；归因序同构（(≺_Θ, active 序) ≡ fold 遍历序） |
| 3 | `pi_theta_step_traced_typed_close_root_and_reduce_core` | 不翻 | 双链同构见证域（票面锚）；一/三类单腿 typed 两链一致（`reverse_exit_type` 单源） |
| 4 | `pi_theta_step_traced_typed_close_shortdiff_overrides_trigger` | 不翻 | S 组（ShortDiff 腿）维持散装 fold 规则2——「P4 维持现状」的守护 |
| 5 | `pi_theta_step_traced_p1_force_flat_risk_exits_all` | 不翻 | P1 上游短路零改（channel 判据在其下游） |
| 6 | `pi_theta_step_traced_p2_close_overlay` | 不翻 | TW P2 组合层分支零改（channel 无 TW 槽，票面外挂保留） |
| 7 | `pi_theta_step_traced_p3_withdraw_consumes_step` | 不翻 | 同 6（TW P3） |
| 8 | `pi_theta_step_traced_p4_enter_earning` | 不翻 | 同 6（TW P4） |
| 9 | `pi_theta_step_traced_p1_masks_tw_predicates` | 不翻 | P1 屏蔽结构零改 |
| 10 | `pi_theta_step_traced_tw_ctx_inert_bitexact` | 不翻 | tw=None 路径同构域 bit-exact |
| 11 | `..._verdicts_normal_path_explicit_hold`（#201） | 不翻 | verdicts schema/组装零改；同构域 Hold 序列不变 |
| 12 | `..._verdicts_normal_path_typed_close`（#201） | 不翻 | 单腿域 typed 裁决一致 |
| 13 | `..._verdicts_p1_force_flat_all_risk_exit`（#201） | 不翻 | P1 分支零改 |
| 14 | `..._verdicts_p2_overlay_typed_and_hold`（#201） | 不翻 | TW P2 分支零改 |
| 15 | `t1_target_zero_assertion_fires_in_step`（#199） | 不翻 | 单腿平凡域 |
| 16 | `t1_target_zero_assertion_holds_with_multiple_core_legs`（#209） | 不翻 | 一类多腿两链全平同效（本票对照锁实证） |
| 17 | `t2_cross_level_confirmed_certificate_holds_l1_position_end_to_end` | 不翻 | 跨级不触发（判据同级过滤两链同款） |
| 18 | `t2_same_level_confirmed_certificate_matches_t1_exit_split` | 不翻 | 单腿同构域 |
| 19 | `t2_unconfirmed_same_level_signal_records_without_exit_decision` | 不翻 | 证书门对齐后两链同款（未确认 ⟹ record 域） |

## 3. 备忘 §3a 清单逐条核对（runner typed ledger / 端到端 / 回归 / 锁）

全部**不翻**。共同证据：runner 合成夹具（e1_bars、e_classification、px100 系）全部
单腿单候选或一类域（同构）；消费段零改；AccountOrder/typed ledger/verdicts 三轨形态
不变。

| # | 测试 | 结论 | 逐条理由 |
|---|---|---|---|
| 20 | `typed_ledger_reverse_close_root` | 不翻 | 单腿一类同构域 |
| 21 | `typed_ledger_reverse_reduce_core` | 不翻 | 单腿三类同构域 |
| 22 | `typed_ledger_shortdiff_close_overrides_trigger_class` | 不翻 | S 组装域零改 |
| 23 | `typed_trade_censored_hold_carries_exit_z` | 不翻 | censored 路径（窗口终点）零改 |
| 24 | `typed_ledger_censored_hold_at_window_end` | 不翻 | 同 23 |
| 25 | `typed_trade_carries_position_node_id` | 不翻 | 仓位节点链路零改 |
| 26 | `same_carrier_reentry_distinguished_by_generation` | 不翻 | generation 机制零改；#200 先平后开次序经 fold 变体原样保持 |
| 27 | `typed_trade_carries_exit_z_snapshot` | 不翻 | exit_z 装配零改 |
| 28 | `run_theta_v0_pi_loop_produces_trades_nonempty` | 不翻 | 单腿域 |
| 29 | `run_theta_v0_pi_prefix_classify_is_causal_no_lookahead` | 不翻 | 因果无前视：channel 判据只读当 bar 输入（同 fold 域） |
| 30 | determinism 两枚（`...deterministic...` 系） | 不翻 | channel 判据同确定性（theta_key 全序 + find 首个） |
| 31 | `run_theta_v0_pi_overlay_reconciles_and_bit_exact_net` | 不翻 | overlay/sep_legs 链路零改；同构域净额 bit-exact |
| 32 | `run_theta_v0_pi_risk_gate_force_flat_on_insolvent` | 不翻 | P1 零改 |
| 33 | TW 对账两枚（4724/4770 口径） | 不翻 | TW 分支零改 |
| 34 | `pan_div_dc_e_default_inactive_order_track_bitexact`（锁①） | 不翻 | pan_div 非活跃臂订单轨冻结；合成场景同构域 |
| 35 | `m6_cost_model_none_bit_exact_and_conserves`（锁②） | 不翻 | 成本模型 none 臂 bit-exact（同构域 + 消费段零改） |
| 36 | `opsem_dump_env_gated_bit_exact`（锁③） | 不翻 | dump 未增行（本票未触 dump 通道；#201 后 dump 零改维持） |

另：interp.rs 自测（备忘 §3b，interpret 本体 10+ 枚）——本体零改 ⟹ 不翻（42 绿含
新增 4）；channel.rs 自测（备忘 §3c）——烤料全 confirmed ⟹ 证书门恒真，22 枚零翻
（24 绿含新增 2）；shadow.rs 自测 7 枚零翻（夹具域不变；生产事实仍由 verdicts 单源）。

## 4. BTC train 16000 窗 typed 轨逐笔对照（opsem dump 新旧 diff）

方法：`OPSEM_DUMP_DIR` 门控 dump（新旧同机制、零代码改动对照面），`trades.jsonl`
26 笔逐笔按 `(level, ordinal, entry_bar)` 配对 diff；裁决级证据由两版 `voice_verdicts`
轨 bar 10355–10375 段对照提供（临时 #[ignore] dump 测试实跑，用后已删）。

### 4.1 分歧段（bar 10365，唯一行为分叉点；之前 16 笔逐字段全同）

bar 10365 持仓：L1/14（Core）、L1/18（Core，restore 祖先腿——无登记，不入 typed）、
L1/15、L2/2、L2/3、L0/86（ShortDiff 子腿，父=L1/18）。当 bar 一个 **L1 二类**反向候选。

- **旧 fold**：二类只关**首个**命中腿 ⟹ L1/14 关（CloseRoot）、L1/18 存活（至 10367
  被另一候选关，CloseRoot）；L0/86 父在存活，10367 被三类信号关（CloseShortDiff trig=3）。
- **新 channel**：L1/14、L1/18 **各自独立**裁 Exit(CloseRoot)（同一候选同为两者
  ≺_Θ 首个命中——「每声部每步一枚」对「每候选归桶」的票面明知结构差）⟹ 两腿同 bar
  关；L0/86 父 L1/18 离场 ⟹ §13 AncOK **连带剪除**（prune，typed=CloseShortDiff 归
  #198 口径；父驱动同 bar 连带离场）。
- 裁决级证据（verdicts 轨逐 bar 对照）：旧 `bar=10365 L1/18=Hold` / 新
  `bar=10365 L1/18=CloseRoot`；旧 `bar=10367 L0/86=CloseShortDiff, L1/18=CloseRoot` /
  新 bar 10366 起两腿均消失（L1/18 关闭、L0/86 剪除）。L1/18 为 restore 祖先腿
  （两版 typed 轨均无其 entry——「restore 恢复的祖先 carrier 腿不在 opened 列」），
  其关闭不进 typed ledger（runner「表中无登记 ⟹ 不入 ledger」消费口径两版同款）。

**判定：翻得对**（新口径证据，无需上报销）：① 结构差属票面 What-to-build 明知的
「多腿/多候选场景两链裁决结构不同，非 bit-exact」；② channel 裁决与 shadow 同输入
同裁（P2/P3 域 Match，§5）；③ 与 S7「级别内全平是必须」（#209，用户裁 A）方向一致
——一类已全平，channel 使二/三类多腿场景每腿独立裁决，与「级别为分账维度、卖出按
归属唯一记账」的会计层对准同向；④ 连带剪除是 §13 既有语义的自然结果（判据零改）。

### 4.2 下游连锁（分叉的传播，逐笔）

| 笔 | 变化 | 归因 |
|---|---|---|
| L0/86（entry 9940） | exit 10367→10365；信号关→prune | 父 L1/18 提前离场连带（§4.1）；typed 仍 CloseShortDiff（#198 口径不变） |
| L0/92（entry 10367，新） | 新增（ShortDiff 子，CloseShortDiff@10422 prune） | 分叉后 L0 slot 状态不同 ⟹ 开仓序列新分支 |
| L0/93（entry 10422，旧） | 消失（旧 Core，CloseRoot@11457 trig=2） | 旧分支开仓不再发生 ⟹ CloseRoot −1 |
| L0/98（entry 11178，新） | 新增（ShortDiff，CloseShortDiff@11958 trig=2） | 新分支开仓 ⟹ CloseShortDiff +1 |
| L0/99（entry 11457，旧） | 消失（旧 Core，ReduceCore@11679 trig=3） | 旧分支不再发生 |
| L0/104、L0/114、L0/123、L0/128、L0/130、L1/26、L1/28（7 笔） | 仅 units 微差（entry/exit/typed/trigger 全不变） | sizing 目标随活动集构成历史漂移（p_tilde 由 sep_legs 构成决定）——连锁，非独立裁决差 |
| 二类触发开仓（i_class=2） | 11 → 10 | 旧分支的 L0/93（二类开仓）不再发生；OpenShort 通道逐笔标注约束两版全成立 |
| 五枚举分布 | CloseRoot 9→8、CloseShortDiff 8→9、ReduceCore 7、RiskExit 0、Hold 2（总 26 不变） | 上述 ±1 的净效果；窗口末全平守恒 |
| 断言①②③（#199/#209 探针） | 评估=2/0/3、违例全 0（两版同） | 硬门在新口径下保持（channel 一类天然全平） |
| 开仓分布 | Core{0}×10、Core{1}×3、ShortDiff×10、Short×3 | 两版同形态（OpenShort=0 样本缺席声明沿用 #200） |

## 5. shadow 双链比对（验收③：P2/P3 域分歧清零或逐条解释）

新内核 BTC 全窗 shadow（42310 声部步，58 条分歧）：

- **P2/P3 域清零**：`exit_typed_mismatch=0`、`channel_exit_production_hold=0`——
  生产 C 组关闭判据 = channel `cert_close_trigger` 单源 ⟹ channel 裁
  Exit(CloseRoot|ReduceCore) 与生产 Closed(同 typed) 全 Match（结构性清零，非样本凑巧）。
- `channel_hold_production_exit=5`：逐条核对全部为 `Closed(CloseShortDiff)`（bar 2045/
  8843/9059/11958/15243 的 ShortDiff 腿）——**S 组/P4 域**（channel 对 entry_v==ShortDiff
  腿不置位 P2/P3，票面「P4 维持现状」= 散装 fold 承担）：已知缺口，非本票引入，
  留待 P4 接线票。
- 其余类别（均为 #196 已声明的语义缺口，与本票正交）：`channel_open_production_idle=9`
  （P6 开仓域，AncOK 准入门/消费语义差）、`channel_hold_production_open=10`（P6 域，
  ShortDiff 角色候选开仓语义差）、`channel_only=26`（P5 OpenShortDiff/P7 Record，
  生产落点不在 channel 域）、`production_silent_drop=8`（§13 结构剪除，channel 无此
  语义）。
- 基线对照（旧内核同窗）：`exit_typed_mismatch=0`、`channel_exit_production_hold=0`、
  `channel_hold_production_exit=5`——本窗基线 P2/P3 域输入全在同构域（无二/三类×
  同级多 Core 腿的 fold 关首个场景——bar 10365 正是首个且唯一该形态，channel 侧
  判据与 fold 结果恰同关 L1/14 首腿 ⟹ Match；分歧只显现在生产侧换内核后 L1/18 的
  命运）。两版分歧类别结构一致，佐证 58 条分歧全部归属既有缺口 taxonomy。

## 6. 三把 bit-exact 锁 + clippy + 重型套件口径

- 锁①订单轨（pan_div 非活跃）、锁②成本模型 none、锁③opsem dump（env 门控）：
  逐枚在 `cargo test --lib` 1828 内绿（锁③的 dump 未增行——本票未触 dump 通道）。
- clippy：改动四文件（channel/interp/coverage/runner）命中集与基线逐行一致
  （26=26，零新增；新函数采用 `sort_by_key`/`is_some_and` 无警告形态，旧函数同款
  旧形态保持零改）。
- 重型套件：按用户口径不跑（本票 = lib 全量 + BTC train 16000 窗三见证 + opsem
  新旧对照 + shadow 全窗对照）。

## 7. 遗留 / 未验证项

- P4 域（ShortDiff 腿关闭）维持散装 fold——`channel_hold_production_exit` 5 条缺口
  随 P4 接线票收口；P5/P6/P7 域 shadow 缺口 taxonomy 未变（#196 声明）。
- **混合域 S 组结局形态**（code-review Spec 轴注记，2026-07-24）：同级「C 组腿 + S 组
  ShortDiff 腿 + 单个二/三类反向候选」输入下，新链 external 预关 C 组腿后，fold 规则2
  的同一候选会继续关「首个剩余命中」= S 组腿；旧 fold 该候选关首个命中（可能是 C 组
  腿）后即消费。S 组**判据**未动（P4 不换 ✓），但 S 组腿结局在该形态下改变——属票面
  明知「多腿/多候选两链裁决结构不同，非 bit-exact」域（候选消费粒度差的另一面）。
  BTC 窗唯一分歧段（bar 10365）的 S 组腿实际走 §13 连带剪除而非此径（§4.1）；此形态
  在本窗未出现独立样本，更大样本的逐条核对随 #204 或 L2 收口按新基线程序覆盖。
- BTC 分歧段只此一处（本窗唯一「二/三类×同级多 Core 腿」输入）；其它品种/窗口未跑
  （用户口径不跑重型套件）——更大样本的分歧面随 #204（后续票）或 L2 收口再核。
- restore 祖先腿（L1/18）的关闭不进 typed ledger 是两版同款消费口径；其 AccountOrder
  侧（无登记 ⟹ 无开仓镜像 ⟹ 关闭不镜像）两版同，账户轨无悬挂（断言4/探针全绿）。
- fold 复制维护风险（code-review 两轴在案）：`interpret_with_external_closes` 与本体
  循环体逐分支同语义，规则演进须两处同步（interp.rs 本体 doc 已补 #202 并存声明）。
