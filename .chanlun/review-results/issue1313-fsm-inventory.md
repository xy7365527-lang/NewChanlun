# #1313 状态机完全分类核验与方向对称化：盘点表 + MECE 锁登记

> 票：GitHub Issue #1313（#1278 ③列「自同构：状态空间方向对称」⑧列「锁」延伸；#1311 前置）。
> 性质：**盘点成文 + 方向对称化落码 + MECE 测试锁**。生产行为变更仅一处新使能位
> `MarketMode::Perp`（runner master FSM 短侧对称臂），`Stock` 在册路径零接触（O0≡P5）。
> 基线：`sandcastle/issue-1313`（== `origin/main` @ `fa5b614ccb`）。
> 日期：2026-08-30。

## 0. 一句话结论

全仓状态机共 **4 台**（LayerState 共享枚举 + runner master FSM + CenterBook 每中枢生命周期
+ spiral VoiceStatus），逐台核 MECE（互斥 = 同刻恰一态 / 完备 = 转移覆盖所有合法情形），
方向对称化补 **ArmedShort**（LayerState）与 **ARMED_SHORT/SHORT**（runner，`MarketMode::Perp`
门控），每台补状态穷尽 + 转移覆盖测试锁，入 #1294 锁网清单。

## 1. 盘点总表

| # | FSM | 载体 | 状态集 | 转移表 | MECE 判定 |
|---|---|---|---|---|---|
| 1 | LayerState（共享枚举） | `rust/src/trading/positional.rs:40` | `Flat / Armed{arm_bar} / ArmedShort{arm_bar} / Pending{confirm_bar} / Long{…} / Short{…} / Gated{…}`（7 态） | 见 §2（按引擎分列） | ✅（§2.1 互斥 + §2.2 完备） |
| 2 | runner master FSM | `rust/src/trading/runner.rs:94-104` | `FLAT(0) / ARMED(1) / LONG(2) / ARMED_SHORT(3) / SHORT(4)`（5 态） | 见 §3 | ✅（§3.2） |
| 3 | CenterBook 每中枢生命周期 | `rust/src/trading/center_book.rs:71` | `Formed / Extended / Dead / DeadDown / Frozen / PendingDeparture(窗口)` | 见 §4 | ✅（§4.2，含两处已声明例外） |
| 4 | spiral VoiceStatus | `rust/src/spiral/voice.rs:27` | `Active / PendingRecovery / Closed`（3 态） | 见 §5 | ✅（§5.2） |

**口径订正（对票面「现状」）**：票面称「`positional.rs:40` LayerState = Flat/Armed/Pending/Long，
无 Short；dual_voice/axiom_voice 各有带 Short 的 LayerState」。核查当前 HEAD 事实：

- 全仓**只有一个** `LayerState` 枚举（positional.rs:40），自 `89da391d1d`（2026-06-12）起
  即含 `Short`（fusion_btr 白名单层翻转断面承载位）与 `Gated`（fusion_btrg 消融臂）；dual_voice/
  axiom_voice/unified_voice/unified_osc/positional_fusion **共享**此枚举，无独立 LayerState。
- 票面的「引擎间状态空间不同构」缺口在 89da391d1d 已收敛为单一枚举；**本票残余缺口 =
  无 ArmedShort（短侧布防表达位）+ runner master FSM 无 SHORT/ARMED_SHORT + MECE 无锁**。
  本票按残余缺口收口，不重做已收敛部分。

## 2. LayerState（共享枚举）逐引擎转移表

### 2.1 互斥（MECE-互斥）

Rust `enum` 结构保证同一 `LayerState` 值**恰为一态**（编译器穷尽 + 零重叠构造）。测试锁：
`trading::positional::tests::layer_state_seven_variants_mutually_exclusive`
（positional.rs:1789）——七态判别式两两不同（`HashSet` 基数 = 7）。**锁形态** =
`layer_state_variant`（positional.rs:1776）的非穷尽 `match`：新增变体未表态即编译失败。

### 2.2 转移表（按引擎；完备 = 每个 (态, 合法输入) 组合有定义、无静默缺口）

**① positional 基座（`PolarityMode::Cycle45`，positional.rs `run_positional`）**
45课持币/持股二元循环 per-layer 形态，长侧专属：

| 现态 | 输入 | 次态 | 出处 |
|---|---|---|---|
| Flat | buy1@k | Armed | positional.rs（阶段2 Cycle45 臂） |
| Armed | 次级别 buy_any 确认 ∨ 超时 | Long（enter_or_defer）或 Pending（pool 不足） | 同上 |
| Armed | sell1@k | Flat（撤防，n_disarms+1） | 同上 |
| Pending | 卖点@k | Flat（取消，n_pending_cancels+1） | 同上 |
| Pending | 重试资金足 | Long | 同上 |
| Pending | 重试仍不足 | Pending | 同上 |
| Long | 卖点@k | Flat（出场，trade 落盘） | 阶段1 |
| Short / Gated / ArmedShort | 任意 | unreachable!（守卫：长侧模式无空侧入口） | 阶段2 |

**② Hold26（`PolarityMode::Hold26`，同函数）** 26课恒仓，confirmed 直接消费、无 ARMED：

| 现态 | 输入 | 次态 |
|---|---|---|
| Flat | buy_any@k | Long / Pending |
| Pending | 卖点@k | Flat（取消） |
| Pending | 重试 | Long / Pending |
| Long | 卖点@k | Flat（削减本层 slice） |

**③ Fusion（`positional_fusion.rs`，`short_mask` 双向条件轴）** 持有 Short/Gated：

| 现态 | 输入 | 次态 |
|---|---|---|
| Flat | buy_any@k ∨ nf_buy | Long / Pending |
| Pending | sell_any@k ∨ nf_sell | Flat（取消）；否则重试 Long / Pending |
| Long | 卖点削减 | Flat（削减）；白名单层 → Short（margin 锁定翻空）或 Gated（消融臂） |
| Short | 逐仓强平（equity≤0） | Flat（强平价 2×entry 记账） |
| Short | in_trend（MoveUp 强制平空） | Flat →（enter_or_defer 翻多）Long / Pending |
| Short | buy_any@k ∨ nf_buy（经 MoveDown 停回补 / R2 门） | Flat → 翻多 Long / Pending |
| Gated | in_trend ∨ buy_any@k（经门） | Flat → 翻多 Long / Pending |
| Armed / ArmedShort | 任意 | unreachable!（Fusion 无布防相位） |

**④ UnifiedVoice（`unified_voice.rs`，fusion_v）** 与 Fusion 空头书出口集同形（Flat/Pending/
Long/Short；Armed/ArmedShort unreachable），XZD 门/冻结门修饰入场臂。

**⑤ AxiomVoice（`axiom_voice.rs`，fusion_va）** 翻转落点恒零暴露（Coin）——Short 不可达：

| 现态 | 输入 | 次态 |
|---|---|---|
| Flat ∨ Gated | buy_any@k ∨ nf_buy（经 026:80 豁免域 / 相位 / 成本门） | Long / Pending（Gated 恢复计数） |
| Pending | sell_any@k ∨ nf_sell | Flat（取消）；否则重试 |
| Long | （出口集） | Flat / 其它 |
| Armed / ArmedShort / Short | 任意 | unreachable! |

**⑥ DualVoice（`dual_voice.rs`，fusion_vd/vn）** 多头书（LayerState）+ 空头书并立（非互斥）；
多头书持 Flat/Pending/Long/Short（Short 仅 dual_book=false 单书翻转架构），Armed/ArmedShort
unreachable。

**⑦ UnifiedOsc（`unified_osc.rs`）** osc 层只读 `LayerState::Long`（`if let Long`）——非独立
FSM，登记为消费方不列为状态机。

### 2.3 完备性判定

- 每台引擎的 `match layers[k]` 均为编译器强制穷尽（本票新增 ArmedShort 后，4 处过渡 match
  + 1 处基座 match 全部显式表态，编译器驱动补齐——axiom_voice:580 / dual_voice:727 /
  positional_fusion:1448 / unified_voice:633 / positional 基座 `(_, ArmedShort)` 臂）。
- 各引擎的 unreachable 臂**即完备性声明**：不可达态 = 该引擎结构上无入口（守卫），非缺口；
  短侧态（Short/Gated/ArmedShort）的可达性由各自模式的守卫（short_mask / Coin / 入口分派）界定。
- **补齐的缺口**：`ArmedShort` 变体（本票新增，见 §3 runner 侧的可达消费方 + MECE 锁锚点）。

## 3. runner master FSM（`Run.state`，u8 常量域）

### 3.1 状态集与转移表（#1313 方向对称化后）

| 现态 | 输入 | 次态 | 镜像关系 |
|---|---|---|---|
| FLAT | buy1@k（最高层） | ARMED | long 臂（在册） |
| FLAT | sell1@k（最高层，且无 buy1，`MarketMode::Perp`） | ARMED_SHORT | **短侧布防（新增）**，ARMED 镜像 |
| ARMED | 次级别 buy_any 确认 ∨ 超时 | LONG（open_position） | long 臂（在册） |
| ARMED | sell1@arm_ladder | FLAT（撤防） | long 臂（在册） |
| ARMED_SHORT | 次级别 sell_any 确认 ∨ 超时 | SHORT（open_short） | **短侧确认（新增）**，ARMED 镜像 |
| ARMED_SHORT | buy1@arm_ladder | FLAT（撤防） | **短侧撤防（新增）**，ARMED 镜像 |
| LONG | master 出场（sell1 等） | FLAT（close_position） | long 臂（在册） |
| SHORT | buy1@entry_ladder | FLAT（close_short，reason=short_cover） | **短侧平空（新增）**，LONG 出场镜像 |
| SHORT | eod（无买点） | FLAT（close_short，reason=short_eod） | **短侧收口（新增）** |

- 方向镜像约定（票面约束）：Short 态 = 持空仓，45课持币/持股二元循环的方向镜像，**不引入
  第三持仓类**；卖点对称语义 = B-1/B-2 的卖侧镜像（卖点布防开空 ↔ 买点布防开多；次级别
  卖确认 ↔ 次级别买确认；买点平空 ↔ 卖点平多）。
- 同 bar 买卖点并现：long 布防优先（保守先例，短侧仅无 long 布防时接管）。
- 短侧持仓会计：1x 全仓 `units = INITIAL_CAPITAL/entry_price`（long 全仓同量纲镜像），
  `pnl = units×(entry−exit)`，`pnl_pct` 落 TradeRec（`n_short_diffs=0`、`cost_basis_at_exit=0.0`
  ——短侧无 long 成本基准，sentinel 显式化）；无 voice 腿、无递归建仓（短侧 = 表达位最小
  二元循环，full 对称声部归下游设计票）。

### 3.2 MECE 判定

- **互斥**：`state` 为 5 个两两不同的 u8 常量（0..=4），`state_name`（runner.rs 测试模块）
  域外显式拒绝。锁：`fsm_symmetry_tests::master_fsm_five_states_partition`（runner.rs:1765）。
- **完备**：主循环 `if FLAT / else if ARMED / else if ARMED_SHORT / else if SHORT / else LONG`
  为五态全分派（无遗漏）；`MarketMode` 穷举 match（Stock/Perp）编译强制表态。
- **零接触守卫**：`Stock` 模式短侧不可达（`perp` 门）——锁：
  `fsm_symmetry_tests::stock_mode_never_arms_short`（runner.rs:1887）。

## 4. CenterBook 每中枢生命周期

### 4.1 状态集（谓词分类，非单一 enum）

| 状态 | 载体谓词 | 语义 |
|---|---|---|
| Formed | `last[ladder]` 置位 | 新中枢形成（cs 出现） |
| Extended | `last` 同 cs 边界更新 | 中枢延伸 |
| Dead | `dead[ladder]` 含 cs | confirmed Type3 终结 |
| DeadDown | `dead_down[ladder]` 含 cs | confirmed Sell3 向下终结（方向标注） |
| Frozen | `frozen[ladder] == Some(cs)` | hard_type3 Buy3 冻结 |
| PendingDeparture（窗口） | `pending_departure[ladder]` | candidate Type3 离开段未决（49课行52/68） |

### 4.2 MECE 判定

- **主互斥**：alive ⟺ `last.is_some() ∧ !is_dead`——每中枢恰为 alive / dead 之一（`alive()`
  center_book.rs:358 的单读法即本分类）。
- **叠加标注（非独立持仓态）**：`dead_down ⊆ dead`（方向标注）；`frozen` 为 Buy3 冻结标注，
  有一条在案单侧例外（先杀者为准对 `frozen` 不成立，`ingest` 的 parity 行为覆盖 cert 先杀，
  见模块头 #664 登记）；`pending_departure` 为窗口（与 alive/dead 正交，死/新中枢时清除）。
- 锁：`center_book::tests::center_lifecycle_mece_classification_lock`（center_book.rs:692）
  驱动 Formed→Extended→窗口置位→价格否定→Dead/DeadDown 全链并断言分类互斥。

## 5. spiral VoiceStatus

### 5.1 状态集与转移表

| 现态 | 输入 | 次态 |
|---|---|---|
| Active | units→0（spawn/翻转后父 units 归零） | PendingRecovery（`refresh_status`） |
| PendingRecovery | units>0（子回补回满） | Active（`refresh_status`） |
| 任意非 Closed | 后序 close | Closed（终态） |
| Closed | `refresh_status` | Closed（不改写） |

### 5.2 MECE 判定

三态判别式两两不同（互斥）；转移覆盖锁 + 终态单向性锁：
`spiral::voice::tests::voice_status_three_variants_mutually_exclusive`（voice.rs:217）+
`voice_status_transition_coverage_lock`（voice.rs:230）。

## 6. MECE 锁登记（入 #1294 锁网清单）

格式同 #1294 报告 §1（判据 × 锁位置 × 形态）：

| # | 判据 | 锁位置（测试名，file:line） | 形态 | 历史分歧/缺口来源 |
|---|---|---|---|---|
| M1 | LayerState 七态互斥（状态穷尽） | `trading::positional::tests::layer_state_seven_variants_mutually_exclusive`（positional.rs:1789） | 判别式 match 编译锁 + HashSet 基数断言 | 票面「引擎间状态空间不同构」（89da391d1d 已收敛，本锁钉死基数 7） |
| M2 | runner master 五态互斥 + 域外拒绝 | `trading::runner::fsm_symmetry_tests::master_fsm_five_states_partition`（runner.rs:1765） | u8 常量域 + state_name 断言 | 票面「runner FLAT/ARMED/LONG 无短侧」缺口 |
| M3 | 短侧卖出对称（布防/确认/平空/eod/撤防/盈亏方向） | `perp_short_arm_confirm_cover_cycle_positive_pnl`（:1780）/ `perp_short_loss_when_price_rises`（:1807）/ `perp_short_arm_disarms_on_buy1`（:1831）/ `perp_short_timeout_force_entry`（:1849）/ `perp_short_eod_close_at_last_close`（:1868）/ `perp_long_arm_priority_over_short_when_both_present`（:1906） | 合成 tape 驱动生产路径断言（非 `#[ignore]`） | #1313 ③列方向对称化 |
| M4 | Stock 零接触守卫（O0≡P5） | `stock_mode_never_arms_short`（runner.rs:1887） | 合成 tape + Stock 模式断言零 trade | 在册 parity 承重面（O0≡P5） |
| M5 | CenterBook 每中枢生命周期分类锁 | `center_lifecycle_mece_classification_lock`（center_book.rs:692） | 全链转移驱动 + alive/dead/dead_down/frozen 分类互斥断言 | 票面「CenterBook 生命周期」盘点项 |
| M6 | VoiceStatus 三态互斥 + 转移覆盖 + 终态单向 | `voice_status_three_variants_mutually_exclusive`（voice.rs:217）/ `voice_status_transition_coverage_lock`（voice.rs:230） | 判别式 + 转移驱动断言 | 票面「spiral 各版」盘点项 |

全部锁落 `cargo test --all-targets`（CI 在跑），非 `#[ignore]`，零外部数据依赖。

## 7. 验证证据

```text
# 环境（同 #1294 报告 §6）
export PYDIR=/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu
export LD_LIBRARY_PATH=$PYDIR/lib:$LD_LIBRARY_PATH LIBRARY_PATH=$PYDIR/lib

cd rust
cargo check --all-targets                    → Finished（零新增 error；本票触及 trading 文件零新增 warning）
cargo test --lib trading                     → 402 passed / 0 failed / 5 ignored
cargo test --lib spiral                      → 46 passed / 0 failed
cargo test --lib fsm_symmetry_tests          → 8 passed
cargo test --lib layer_state_seven_variants  → 1 passed
cargo test --lib center_lifecycle_mece       → 1 passed
cargo test --lib voice_status                → 2 passed
```

## 8. 收尾

- 生产行为变更仅 `MarketMode::Perp` 使能位（新增枚举变体）+ runner 短侧对称臂；`Stock`
  在册路径由 M4 锁守卫零接触。
- 短侧 full 对称声部（voice/recursive 腿的短侧镜像）不在本票（表达位最小二元循环），归下游
  设计票——如实声明，不假装完整。
- 数据文件零新增；无密钥。

*本报告只登记盘点、锁与证据，不代裁、不关票。*
