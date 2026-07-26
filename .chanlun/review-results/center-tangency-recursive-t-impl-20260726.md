# #322 实施报告：recursive_t 延伸谓词改弱（第四实现对齐全域）

- 日期：2026-07-26
- 票：#322（parent #59；上游 #246 全域线、#290 裁定 A、#314 Python 侧落码、#321/#323 成立边界从严）
- 流程：本仓 `/implement` + `/tdd`（先红后绿）。**code-review 步骤未做**——按编排者指令改由独立影子评审执行（禁自评）。
- 纪律：只改 `rust/src/recursive_t/center.rs` 一个源文件 + 本报告；既有脏文件（`CLAUDE.md`、`rust/src/bin/p107_level_calib.rs`、`.claude/`、`.agents/` 等）未动未 add；**#337 在途未提交改动**（`theta_v0/classifier/center_lifecycle.rs`、`theta_v0/backtest/opsem_dump.rs`）未动——见 §5 编译隔离说明。未回关 issue、未发评论。

---

## 0. 结论摘要

| 项 | 结果 |
|---|---|
| 消费面判定 | **生产可达（研究/回测层活跃），非退役** ⟹ 走「切弱对齐」，不走声明作废 |
| 谓词改动 | 1 个（`center.rs:20` `overlaps`），`< / >` → `<= / >=`；行为改动 **1 行** |
| 不改的两处严格 | `detect_type3` 离开判据、`same_direction_step` 外缘分离判据（脱离/分离在定理一即严格，现状已符合）；`find_centers` 成立条件 `zg > zd`（#321/#323 裁定从严，与弱延伸并存是裁定结果） |
| TDD | 3 条新用例，2 红 → 绿（相切上沿/下沿），第 3 条（真脱离不延伸）**本就绿**——作为反向锁固定「改弱不越界」 |
| 测试读数 | `cargo test --lib` **1913 passed / 0 failed / 139 ignored**（基线 1910 + 3 新用例；既有用例**零翻动**） |
| 四把锁 | **4/4 绿**——但**覆盖为零**，零翻动不构成安全证据（§4.2） |
| 靶向验证（切片，不重放） | OKLO：L0 相切吸收 4 次、中枢 421(弱) vs 422(严)；BRN：L0 84 次 + L1 3 次、中枢 L0 2181 vs 2204 / L1 300 vs 302。相切率 OKLO 0.14% / BRN 0.51%（单元级），与 #314 Python 侧 0.21%/0.37% 同量级 |
| 慢锁 | 未跑（§6 清单），留重型窗口 |
| 未判定项 | 全塔真实重跑后的下游位移（本票只做切片对照，见 §3.3 有效域声明） |

---

## 1. 消费面盘点（验收项①）

**问题**：`recursive_t` 链是否已实质退役？若退役则登记声明作废，否则切弱。

**判定：活链，生产可达。** 调用图（`grep` 全仓，排除 `.claude/worktrees/`）：

```
center::find_centers          ← 唯一调用点 operator::apply_t（center.rs 无其他消费者）
operator::apply_t             ← mod::iterate（塔驱动器）
                              ← rec_stream::level_segments / 中枢 dump（诊断）
mod::iterate                  ← ffi::run_recursive_t          → PyO3 导出（lib.rs:2442）
                              ← ffi::PyTFugueStream::push_bar  → PyO3 导出（lib.rs:2445-2446）
                              ← ffi::run_t_fugue               → PyO3 导出
                              ← stream::TFugueStreamCore（:225/:384，T Fugue 流式回测内核）
                              ← t_engine_run / backtest_run 诊断与回测测试
Python 侧消费                 ← trading_system/backtest_t_fugue.py（走 run_t_fugue）
                              ← analysis/t_vs_v3_comparison.py:65（走 run_recursive_t）
```

`find_centers` 位于 T Fugue 流式回测内核的每 bar 重跑路径上（`ffi.rs:235`：`push_bar` 内部 `process_bar → iterate → …`），不是死代码。

**与 theta_v0 classifier 的关系：完全解耦。** `theta_v0::classifier::recursive_tower` / `theta_v0::classifier::center`（#323 改的那个 `center.rs`）与 `recursive_t` **互不引用**——两套独立中枢实现，前者是生产 BSP 塔链（四把锁覆盖的那条），后者是 T 递归算子 standalone 实现（`mod.rs` 模块头明示「不依赖 v3 nucleus」）。因此：

- 本票改动**不触碰** #337 正在改的 `theta_v0/classifier` 一带；
- 反过来，四把锁（走 theta_v0 生产链）对本票改动的覆盖为 **0**（§4.2）。

**结论**：非退役 ⟹ 按票面第一分支「切弱对齐全域」。

---

## 2. 改动（验收项②）

### 2.1 谓词

| 文件:行 | 函数 | 旧 | 新 |
|---|---|---|---|
| `rust/src/recursive_t/center.rs:20`（改后 :39） | `overlaps` | `unit.low < zg && unit.high > zd` | `unit.low <= zg && unit.high >= zd` |

原文锚（照 #314 同款登记，写入 `overlaps` doc）：中心定理一 `docs/chanlun/text/blog/020-第20课.md:56`——「走势中枢的延伸等价于任意区间[dn，gn]与[ZD，ZG]有重叠。换言之，若有Zn，使得dn>ZG或gn<ZD，则必然产生高级别的走势中枢或趋势及延续。」**脱离条件用严格不等 ⟹ 端点相等不构成脱离 ⟹ 仍属有重叠 ⟹ 延伸。**

对齐对象登记：Lean `Origin.CenterStates.CenterExtension`（弱）/`CenterBroken`（严格）、v1 `a_zhongshu_v1._extend_zhongshu:104`、v0 `a_center_v0._has_overlap`（#314 已弱）、`a_level_fsm_newchan.overlap`（#314 已弱）、#246 全域口径。改弱后**四实现同口径**（票面「第四实现分歧」消解）。

### 2.2 明确不改的三处（写入注释，防后人误改）

| 位置 | 谓词 | 保持 | 理由 |
|---|---|---|---|
| `detect_type3` | `leave.low > c.high` / `leave.high < c.low` | 严格 | 定理一「dn>ZG 或 gn<ZD」——**脱离**本就严格；相切应判「未脱离」，现状已符合 |
| `same_direction_step` | `next.dd > prev.gg` / `next.gg < prev.dd` | 严格 | 定理二外缘**分离**判据；相切 → `None`（不是趋势的一步）= 视为重叠，与「相切=重合」一致 |
| `find_centers` 成立条件 | `if zg > zd` | 严格 | #321 裁定 / #323「三实现统一从严」（`ZG==ZD` 单点核心不成立）。**成立严格 + 延伸弱并存是裁定结果，非遗漏**——已写入 `find_centers` doc 步骤 2 |

---

## 3. 先红后绿证据（验收项③）

**seam**：公共函数 `find_centers`（既有测试已在此 seam）。TDD 一个 vertical slice：先写 3 条用例跑红 → 改谓词 → 跑绿。

### 3.1 红（改谓词前，`cargo test --lib recursive_t::center`）

```
running 7 tests
test recursive_t::center::tests::不重叠不成中枢 ... ok
test recursive_t::center::tests::中枢向后延伸吸收重叠单元 ... ok
test recursive_t::center::tests::三笔重叠成中枢 ... ok
test recursive_t::center::tests::真脱离段不延伸中枢 ... ok
test recursive_t::center::tests::两个不重叠中枢_上涨同向 ... ok
test recursive_t::center::tests::下沿相切段延伸中枢 ... FAILED
test recursive_t::center::tests::上沿相切段延伸中枢 ... FAILED

---- 下沿相切段延伸中枢 stdout ----
assertion `left == right` failed
  left: [0, 1, 2]
 right: [0, 1, 2, 3]
---- 上沿相切段延伸中枢 stdout ----
assertion `left == right` failed
  left: [0, 1, 2]
 right: [0, 1, 2, 3]

test result: FAILED. 5 passed; 2 failed
```

即旧口径把相切段判为「已脱离 ⟹ 中枢在此终结」，正是定理一方向的反面。第 3 条 `真脱离段不延伸中枢`（`low=20.1 > ZG=20`）**红阶段即绿**——它是反向锁：证明改弱后仍不吞真脱离段（改后复跑仍绿，见 §3.2）。

### 3.2 绿（改 `overlaps` 后）

```
running 8 tests
test recursive_t::center::tests::靶向_相切延伸口径逐级对照 ... ignored
test recursive_t::center::tests::不重叠不成中枢 ... ok
test recursive_t::center::tests::真脱离段不延伸中枢 ... ok
test recursive_t::center::tests::中枢向后延伸吸收重叠单元 ... ok
test recursive_t::center::tests::三笔重叠成中枢 ... ok
test recursive_t::center::tests::下沿相切段延伸中枢 ... ok
test recursive_t::center::tests::上沿相切段延伸中枢 ... ok
test recursive_t::center::tests::两个不重叠中枢_上涨同向 ... ok

test result: ok. 7 passed; 0 failed; 1 ignored
```

用例构造（核心区间恒为 `ZD=12, ZG=20`）：
- `上沿相切段延伸中枢`：第 4 段 `[20,30]`，`low == ZG` → `units [0,1,2] → [0,1,2,3]`、`gg 22 → 30`
- `下沿相切段延伸中枢`：第 4 段 `[2,12]`，`high == ZD` → `units [0,1,2] → [0,1,2,3]`、`dd 10 → 2`
- `真脱离段不延伸中枢`：第 4 段 `[20.1,30]` → 恒 `[0,1,2]`（改弱前后皆绿）

### 3.3 靶向验证（切片对照，**不重放**）

新增 `#[ignore]` 探针 `靶向_相切延伸口径逐级对照`（`center.rs` tests 内，含旧严格口径复刻 `find_centers_strict_legacy`）。方法：跑一次生产 `iterate` 拿全塔，取**每级别的实际输入单元序列**（L0 = a₀，Lk = L(k−1) 的 `next_units`），在同一序列上分别跑弱口径与严格口径的 `find_centers` 并逐字段比较。

跑法：`BT_SYMBOLS=OKLO cargo test --release --lib recursive_t::center::tests::靶向 -- --ignored --nocapture`

```
[OKLO] bars=343282  a0=2843
  L0: units= 2843 中枢 弱= 421 严= 422 相切吸收判定=4 集合差异=true
  L1: units=  395 中枢 弱=  58 严=  58 相切吸收判定=0 集合差异=false
  L2: units=   50 中枢 弱=   8 严=   8 相切吸收判定=0 集合差异=false
  L3: units=    6 中枢 弱=   1 严=   1 相切吸收判定=0 集合差异=false
合计：相切吸收判定=4 次，级别集合差异=1 层

[BRN] bars=2434075  a0=16466
  L0: units=16466 中枢 弱=2181 严=2204 相切吸收判定=84 集合差异=true
  L1: units= 2060 中枢 弱= 300 严= 302 相切吸收判定=3 集合差异=true
  L2: units=  268 中枢 弱=  39 严=  39 相切吸收判定=0 集合差异=false
  L3: units=   34 中枢 弱=    6 严=   6 相切吸收判定=0 集合差异=false
  L4: units=    5 中枢 弱=   1 严=   1 相切吸收判定=0 集合差异=false
合计：相切吸收判定=87 次，级别集合差异=2 层
```

读数：相切在真实数据里是**低频但非零**事件（单元级 OKLO 4/2843 = 0.14%、BRN 84/16466 = 0.51%，与 #314 Python 中枢层 0.21%/0.37% 同量级）。方向一致——弱口径中枢**更少**（相切段被吸收 ⟹ 中枢更长 ⟹ `i = last+2` 跳过位置后移 ⟹ 后续中枢起点被吞）：OKLO L0 −1、BRN L0 −23 / L1 −2。

**★有效域声明（限定）**：本对照是**逐级切片**，两套谓词跑在**同一份（弱口径产出的）输入序列**上。真实全量重跑时，L0 口径变化会改变 `next_units`，上层输入随之漂移——因此「L1+ 集合差异=false」**只说明「在同一输入上两谓词同解」，不等于「上层塔不受影响」**。上层真实位移属全量重跑范畴，本票按验收要求不做（见 §7 未判定项）。

---

## 4. 锁与测试读数（验收项④）

### 4.1 `cargo test --lib`

```
test result: ok. 1913 passed; 0 failed; 139 ignored; 0 measured; 0 filtered out
```

基线 1910 + 3 条新用例 = 1913；ignored 138 + 1 探针 = 139。**既有用例零翻动**（0 failed，且新增前后既有 4 条 center 用例逐条绿）。

### 4.2 四把锁（`--ignored`，BTC 数据）

```
test theta_v0::backtest::runner::tests::btc_prune_leg_exit_type_matches_account_identity ... ok
test theta_v0::backtest::runner::tests::btc_type2_open_short_channel_witness ... ok
test theta_v0::backtest::runner::tests::btc_type2_residual_correction_witness ... ok
test theta_v0::backtest::l3_delta_r_alpha::tests::typed_ledger_btc_smoke ... ok
test result: ok. 4 passed; 0 failed（76.24s）
```

**「零翻动」的诚实标注（沿用 #323 Critical-2 / #331 / #336 先例）**：四把锁全部走 `theta_v0` 生产链（`backtest::runner` / `l3_delta_r_alpha`），而 §1 已证 `theta_v0` 与 `recursive_t` **零引用关系**——四把锁对本票改动路径的覆盖为 **0**。**零翻动是零覆盖的结果，不构成安全证据。** 真正覆盖本次改动的是：(1) §3 的 3 条 TDD 用例（含反向锁）；(2) §3.3 两标的真实数据切片对照（相切事件被实际触发 4 / 87 次，行为方向与定理一一致）。

---

## 5. 编译隔离说明（090 照实）

**主工作区当前无法编译 `--lib`**：#337 的在途未提交改动处于半成品态——

```
error[E0063]: missing field `revived` in initializer of `ChainConsumed`
  --> src/theta_v0/classifier/center_lifecycle.rs:1007 / :1026
error[E0004]: non-exhaustive patterns: `CenterLifecycleEvent::Superseded` not covered
  --> src/theta_v0/backtest/opsem_dump.rs:721
```

按纪律（不动他人在途文件、不 `git stash`），本票全部测试读数在**基于 HEAD `902cc3ff47` 的临时 worktree**（`/tmp/nc322`，独立 `CARGO_TARGET_DIR=/tmp/nc322-target`，`analysis/data_cache` 软链主仓）中取得，worktree 内容 = HEAD + 本票 `center.rs` 单文件改动，**无其他脏文件**（`git status` 仅 `M rust/src/recursive_t/center.rs`）。验证完毕后 worktree 已移除。

**影响声明**：因此本票读数**不含** #337 在途改动，是「HEAD + 本票」的干净读数；#337 落地后若与本票同文件冲突——不会，两票文件面无交集（§1 已证 theta_v0 ⊥ recursive_t）。

---

## 6. 慢锁清单（未跑，留重型窗口）

- `recursive_t::backtest_run::*`（8 标的全量 T 回测，`--ignored`）
- `recursive_t::t_engine_run::*`（T Fugue 三模式矩阵 + 结构 dump，`--ignored`）
- 其余 135 条 `--ignored`（多为 BTC/多标的重型回测与 profile）

**这三类中 `recursive_t::*` 的重型回测是真正会被本票改动移动读数的锁**——本票按验收要求「慢锁不跑」，其历史读数（T 引擎 BTC/8 标的基线）在本改动后**口径已变、须重跑方可继续引用**。

---

## 7. 未判定项

1. **全塔真实重跑后的下游位移**：§3.3 只做同输入切片对照；L0 中枢边界位移（OKLO −1 / BRN −23）会经 `encapsulate` 传导到上层单元，真实塔的 L1+ 变化、BSP 计数、T Fugue 净值均未测（属重型窗口）。
2. **既有 `recursive_t` 回测结论的过期范围**：本票未逐条清点哪些历史报告引用了旧口径 T 塔读数（记忆项「T 引擎 BTC 权威基线」在列）。建议随重型窗口一并处理。
3. **Lean 侧对 `recursive_t` 的 parity**：`recursive_t` 无 Lean parity fixture（与 theta_v0 不同），本次对齐是**读码对齐**而非机器锁对齐——口径日后若再漂移，无自动告警。
