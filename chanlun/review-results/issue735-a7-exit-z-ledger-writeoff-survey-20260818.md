# #735 交付报告：L69 A7 判据「裁量分离」挂账——语义查明 + 核销路径建议

日期：2026-08-18 ｜ 票：#735（map #695，L69）｜ 立案出处：#696 浮出 + #699 补登（2026-07-29）
性质：**只查不裁**——语义查明 + 核销路径建议呈编排者；裁定后落地。本票不改生产代码。

## 结论摘要（TL;DR）

- **挂账语义**：A7（Task #165，《完整的策略.pdf》§6 z「两次快照」+ §9 typed exit）的「裁量分离」=
  **生产/消费两半分离**的 team-lead 令——生产半（`exit_z` 账本列 + `exit_z_of` 单源构造）已交付在役、
  构造被迫唯一（定理，非裁量）；消费半（μ 估计器按 `(entry_z, exit_z, exit_type)` 分桶语义）自 2026-07-03
  起显式「待 codex 裁决」，从未实装。
- **核销路径建议：死账核销（消费侧），生产侧保留。** 消费半的唯一消费者（μ 门侧）已随 χ 线实测否决裁定
  （2026-07-28）整族撤销、「不再开语义终裁」⟹「待 codex 裁决」的对象不复存在。
- **裁定后落地（供编排者裁定，本票不动）**：① 两处「待 codex 裁决」注释对齐 χ 线撤销裁定（L30 同款最小面）；
  ② 行号订正（票面 `selector.rs:299` → 现树 `:313`）；③ 总账 L69 状态「无家可归」→「死账确认」。

---

## 1. 挂账语义查明

### 1.1 挂账对象与来源

- **A7 = Task #165**（《完整的策略.pdf》§6 z「两次快照」+ §9 typed exit），非 GitHub Issue 号。
  引入提交 `4eba6d8fb9`（2026-07-03，Claude Fable 5）：「TypedTrade 增 exit_z 列（同 MuClass 两次快照，
  结构维入场冻结/账本态三元刷新到出场 bar），selector::exit_z_of 单源构造，五出场点全接」，
  提交信息明写「schema v2 显式版本标记 + B30 prereg 联动声明（μ 样本 schema 未变，消费侧分桶待 codex 裁）」。
- **结果包**：`8d9ce3f9bb` → `.chanlun/review-results/a7-exitmu-20260704.md`（六要素 + PDF §6/§9↔代码行映射表；
  已被 #504 记账层清收归档出仓，内容 git 史可见）。其 §5 把消费侧裁量的三方案明列在案：
  ①三元组独立桶 ②exit_type 作条件维 ③exit_z 账本态差分作特征——「本工位不自决，只生产 exit_z 账本列……
  待 codex decide」。
- **L69 立案**：`stale-tickets-audit-20260729.md` §2 清单 3.3 浮出「A7 判据至今『裁量分离』挂账在代码注释」、
  §8.3 登记「挂账有代码内登记、无票（census 漏登）」；#699 在 map #695 补登 L69
  （状态=无家可归，触发=未定）。**L69 不在 `debt-ledger-census-20260729.md` 正本 68 笔内**，其登记锚 =
  map #695 评论（#699 resolution）。

### 1.2 生产半（已交付、在役、非欠账）

| 件 | 落点 | 状态 |
|---|---|---|
| `exit_z` 账本列 | `rust/src/theta_v0/backtest/ledger.rs:74`（schema v3 增列，现 v4） | 在役 |
| `exit_z_of` 单源构造 | `rust/src/theta_v0/backtest/selector.rs:318` | 在役 |
| 五出场点接线 | `fill.rs:5752/5839/5880/5920`（四出场）+ `:6399`（censored）+ `open_ledger.rs:372` | 在役 |
| 透传测试 | `runner_tests.rs:3936`（结构同构快照）+ `:4009`（censored 也携） | 在役 |
| 观测面 | `opsem_dump.rs:828-831`（只读 `exit_z.t_stage` 入 dump） | 在役 |

生产半的构造**被迫唯一**（`selector.rs:297-311` 文档「定理，非裁量」）：一笔持仓生命期内是同一结构实体，
结构维（level/δ/i_class/…/origin_level）入场冻结不重采样（PDF §6「σ_higher: 入场时上级方向」按定义入场值），
唯一逐 bar 变化的是账本态三元 `{t_stage, eta_bucket, risk_mode}`——出场处重分类结构维需伪造不存在的出场候选
（多数 typed exit 无触发候选）= 声明膨胀（231号），故承继入场是唯一诚实构造。**该半无裁量、无挂账。**

### 1.3 消费半（未实装、待裁决、现确认死亡）

- 消费侧 = μ 估计器按 `(entry_z, exit_z, exit_type)` 分桶的语义（`ledger.rs:71-72` / `selector.rs:313-315`）。
- **从未实装**：`build_mu_from_bars`（`l3_delta_r_alpha.rs:262-326`）只读 `t.entry_z`
  （`:291` 取 δ、`:294` 作 `MuObservation.class`），不读 `exit_z`；`exit_type` 仅诊断切片透传
  （`:321` 注释「不进桶键/门控/裁决基」）。全仓 `exit_z` 无任何 μ 侧读点。
- 分离理由（team-lead 令）：分桶语义是**设计裁量**（多合理方案，需价值判断），故只生产 `exit_z` 入
  TypedTrade、不静默改 μ estimand（B30 prereg 联动声明，`ledger.rs:117-122`）。

---

## 2. 死因查明（本次调查的关键增量）

**χ 线实测否决裁定**（`chanlun/escalate/chi-line-falsification-ruling-20260728.md` §1①，2026-07-28，
编排者直接拍板，入 main `558104a0ff`）：

> χ 线——以历史样本 μ 估计为准入依据的统计门族——从本系统目标中撤销：不接生产、不进 Destination、
> **不再开语义终裁**（z_α/treat_empty/θ 全部核销）。

- μ 估计器门侧（μ/LCB/`mu_shrink`/`oos_gated_drop`）随族撤销：`mu_estimator.rs:1-8` 模块头显式
  「不接生产、不进 Destination、不再开语义终裁。诊断件保留（供 #61/#71 历史否证证据链追溯，禁删）」；
  `prob-inference-disposition-registry-20260728.md` A1 同款登记。
- 消费半的「待 codex 裁决」对象 = μ 门侧分桶语义 ⟹ **对象随 χ 线撤销消亡**，挂账失去触发条件。
- 关键区分（防误伤生产半）：χ 撤销族谱（A1-A8/B1-B6/C1）**不含** `TypedTrade`/`ledger.rs`/`exit_z_of`——
  `exit_z` 生产半是账本层载体 + 观测面，非推断核，**不在撤销范围、继续在役**。`selector.rs` 虽整体在
  A2（`chi_t`/`filter_gamma*` 诊断保留），但其内 `exit_z_of` 是生产函数（fill 循环调用），不受影响。

---

## 3. 核销路径评估（三选一）

| 路径 | 判定 | 理由 |
|---|---|---|
| **裁定落码** | 死路 | μ 门侧已撤销、无处落码；「不再开语义终裁」明令封死该门侧任何后续语义裁决 |
| **登记正式挂账** | 无触发可挂 | 唯一触发「codex 裁决后接入 μ 分桶键」永不触发；χ 线 §5 复议（「条件期望在新桶键/新样本量级下可估」的实测证据）若成立，属**新裁定 + 新 prereg**（`ledger.rs:121-122` 既有声明已覆盖），非本账续命 |
| **死账核销** | **推荐** | 消费半随消费者（μ 门侧）一并死亡；生产半已交付在役、本就不欠。注释「待 codex 裁决」为 stale 文案，裁定后对齐即可 |

**推荐定性**：L69 死账。半句拆账——生产半「已交付保留」（非欠账），消费半「随 χ 线撤销失效」（死账）。

---

## 4. 裁定后落地清单（供编排者裁定；本票不落地、不改码）

1. **注释对齐（L30 同款最小面，纯 doc 注释零行为变化）**：
   - `ledger.rs:71-72`「消费侧未定……待 codex 裁决」→ 改为：μ 门侧已随 χ 线撤销（2026-07-28）、不再开语义
     终裁；`exit_z` 保留为出场侧完备性账本载体 + opsem_dump 观测；任何未来接入消费须新开 prereg
     （复用 `TYPED_TRADE_SCHEMA_VERSION` 既有 B30 联动声明）。
   - `selector.rs:313-315`「待 codex 裁决（no-unnecessary-escalation 选择类）」→ 同款对齐。
2. **行号订正**：票面 `selector.rs:299` → 现树 `selector.rs:313`（rustfmt #700 等漂移，对齐 #785 存量漂移
   普查「漂移只来自路径/函数名/行号物理挪动」认定）。
3. **总账状态翻转**：L69「无家可归」→「死账确认」，随裁定在 map #695 或台账登记（生产半保留一句备注）。

---

## 5. 证据锚一览（090 照实）

- `4eba6d8fb9`（2026-07-03）：A7 生产半引入，提交信息「消费侧分桶待 codex 裁」。
- `8d9ce3f9bb`（2026-07-03）：A7 结果包 `a7-exitmu-20260704.md`（#504 归档出仓，git 史可见），§5 三方案。
- `chanlun/escalate/chi-line-falsification-ruling-20260728.md` §1①/§4/§5：χ 线撤销 + 门侧「不再开语义终裁」
  + 复议通道。
- `chanlun/review-results/prob-inference-disposition-registry-20260728.md` A1-A8/B1-B6/C1：撤销族谱（无
  TypedTrade/exit_z_of）。
- `rust/src/theta_v0/backtest/mu_estimator.rs:1-8`：门侧撤销标注。
- `rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:262-326`：`build_mu_from_bars` 只读 `entry_z`。
- `rust/src/theta_v0/backtest/ledger.rs:62-74/117-122`、`selector.rs:297-318`：挂账注释（现树行号）。
- `stale-tickets-audit-20260729.md` §2 清单 3.3 + §8.3；map #695 评论（#699 补登 L69）。
