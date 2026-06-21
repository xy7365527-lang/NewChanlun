# BTC T 引擎跨版本演化 + 当前权威基线

> **目的**：消除「BTC T 引擎盈亏」这个高频引用数据的跨 session 不一致。本文档是该指标的**单一权威来源**。
>
> **核查日期**：2026-06-20（cargo 重跑确认）
> **当前 HEAD**：`06c9739f7e`（代码 = `288d704a1b` recover 全量了结 + `06c9739f7e` docs 落盘）
> **认识论等级**：基线数字 = L2（单标的真实数据 cargo 重跑）；机制账本 = L0（源码会计结构）；跨版本演化 = 历史记录

---

## 1. 当前权威基线（必引此表，勿引旧负值）

当前 main BTC `operation_self_replication` 引擎三模式真值
（`BT_SYMBOLS=BTC cargo test --release recursive_t::t_engine_run::t_engine_8x3 -- --ignored`，4 625 119 bar，BH = +1380.4%）：

| mode | strat | mdd | n_trades | final_nav |
|------|-------|-----|----------|-----------|
| **structural** | **+29.6%** | 17.8% | 4225 | 129628.63 |
| and | −29.1% | 55.1% | 4643 | — |
| or | +21.8% | 32.4% | 3398 | — |

三模式皆 **<BH**（+1380%）——确认滞后税 + 强牛踏空。

**per-level 已实现 PnL（structural）**：

| ladder | L0 | L1 | L2 | L3 | L4 | L5 |
|--------|-----|-----|-----|-----|-----|-----|
| PnL | **+41998** | +14611 | +22869 | **−32727** | −17123 | +0 |

赚在几何塔底（L0–L2 快短差），**深层 L3/L4 失血**（中间级别 1/9、1/27 量级短差在强趋势被 1x 逐仓强平级联）。

---

## 2. 关键陷阱：`engine` 字段是静态标签

`t_engine_*_trades.json` 的 `engine:"operation_self_replication"` 是**固定字符串标签，不随代码演化更新**。t_engine.rs 在 2026-06-19 一天内迭代了 5+ 个版本，但该字段从未变。

**判断某个 trades.json 是哪个版本，看 commit + `final_nav` + 文件 mtime，绝不看 `engine` 字段。**

---

## 3. 跨版本演化时间线（全部 2026-06-19，BTC structural）

| commit | 时间 | 版本特征 | BTC | 性质 | 记忆条目 |
|--------|------|---------|-----|------|---------|
| `dc8c95ed37` | 01:10 | ε 对称 sink/recover 几何塔 | **+664%** (<BH) | 早期赢家 | `project_t_operation_self_replication` |
| `c06db11942` | — | 假减仓跨级耦合（父级 units 不动） | +10.2% (OR) | 否证 | `project_t_cross_level_coupling_falsified` |
| `5f93000e11` | 16:24 | 正确双向耦合（父级真减仓 sink/recover） | **−89.7% 💥** | 真穿仓 (mdd>100%) | `project_t_cross_level_coupling_falsified` |
| `b590d4884f` | 17:18 | + 自下而上涌现升级（emergence-upgrade） | **−33%** (+57pp 救回) | 救穿仓 | `project_t_emergence_upgrade_ab` |
| `288d704a1b` | 19:29 | + recover 改全量了结 | **+29.6%** (−32.6→+29.6, +46.6pp) | **转正（当前）** | `project_t_recover_full_vs_quota` |
| `06c9739f7e` | — | docs 落盘判决（无代码改动） | +29.6% | HEAD | — |

**两步救穿仓**：涌现升级（+57pp）→ recover 全量（+46.6pp），把 −89.7% 真穿仓救回 +29.6% 正域。两者性质都是 **regime 方差/敞口缩减器，非普适 alpha**：救穿仓标的、杀震荡/趋势赢家、mdd 普降。

### 旧记忆负值的真实身份（非错误，是演化快照）

| 旧值 | 来源 | 真实身份 |
|------|------|---------|
| **−89.7% 💥** | `project_t_cross_level_coupling_falsified` | commit `5f93000e11`（同谱系，emergence + recover 修复**前**的穿仓中间版） |
| **−77.6%** | `project_t_short_close_level_mismatch` | **更早的、架构不同的引擎**（root-flip / E-sink / F门(328-349) / AND-整段0空头——当前 t_engine.rs **无这些术语和行号**） |

两份记忆的**机制洞察仍有效**（做空腿失血、平空级别错配、深层失血与当前 +29.6% 版的 L3/L4 失血同构），但**数字勿引为当前基线**。两份文件已加「版本时效」标注。

---

## 4. units 守恒账本（drain / recover / reduce 机制，源码逐位核验）

常数：`LAMBDA = 3.0`，`MOBILE_FRAC = 1/LAMBDA = 1/3`（`fugue_v3/mod.rs:90,94`）。
会计双腿（`fugue_v3/accounting.rs`）：`reduce_at(k,m)` 把 m 股变现金（`units −= m`，资金进/出 `free`，NAV 中性）；`add_at(k,m,dir)` 反之。
**单腿 = 销毁/变现；双腿(reduce + add) = 级间转移守恒。**

| 算子 | 代码 | sizing | 会计 | Σ\|units\| | units 去向 |
|------|------|--------|------|-----------|-----------|
| **sink** | `t_engine.rs:240-256` | 当前父级 1/3 (`mobile_quota(u_p)`) | reduce_at(P) **+ add_at(sub)** | **守恒** | 父级→子级（开反父向短差） |
| **recover** | `t_engine.rs:263-277` | **全量** (`m=u_sub`) | reduce_at(sub) **+ add_at(P)** | **守恒** | 子级→父级（全平升回，股数 1:1） |
| **drain** | `t_engine.rs:280-287` | 当前子级 1/3 | **仅 reduce_at(j)**，无 add_at | **减少 1/3** | 子级→idle `free`（**销毁暴露，不归还父级**） |
| **liq** | `t_engine.rs:347-358` | 全量 | 仅 reduce_at(k) | 减少 | 价格边界 1x 逐仓强平→现金 |
| **flip** | `t_engine.rs:330` (clear_all) | 全塔全量 | 仅 reduce_at | 减少 | 核心反向时全塔平到现金 |
| **enter** | `t_engine.rs:169-179` | 全部 `free/c` | 仅 add_at | 增加 | 现金→建仓（**全运行仅 4 次**） |

### drain 详解（单腿销毁，不归还父级）

```rust
fn drain(&mut self, j: usize, bar: i64, c: f64) {
    let u = self.layers[j].units;
    let m = mobile_quota(u);              // = u/3
    if !(m > 1e-12 && m.is_finite()) { return; }
    reduce_at(&mut self.layers, j, m, &mut self.free, c, bar, &mut self.res, "drain");
}
```

- **触发**（`route_bsp:302-308`）：子级 j 已持**同父向遗留仓**，又收到**反父向减仓 BSP** → drain（而 j 空仓/已持反向短差时走 sink）。
- **语义**：减暴露 1/3、不翻转、父级主导。函数签名**不含 parent 参数**，物理上不可能归还父级。
- **vs 强平**：drain 与 liq 是同会计形态（孤立 reduce_at），但 **drain 不是强平**——drain 由 BSP 信号(B 段)主动触发、取 1/3；liq 由价格边界(A 段)被动触发、取全量。

### recover 全量 vs σ-不变 1/3（有意分歧）

`t_engine.rs` 的 recover 取**全量**（`m=u_sub`，编排者 2026-06-19 方案②裁决：次级别走势"完成"=0/1 事件 ⟹ 全量了结），与 `fugue_v3::cycle::recover_chunk` 的 σ-不变 1/3 配额**有意分歧**，仅作用于 t_engine，不动 542 号 `prove_sigma_quota` 守卫。
> **源码缺陷待修**：`t_engine.rs:310` 行内注释和 `accounting.rs:24` 旁注仍写「recover 平 1/3」，与全量代码（`t_engine.rs:270`）矛盾，应删除（违反 no-patch-mentality 声明-实际一致性）。

---

## 5. 几何衰减 + 下跌段踏空（为什么 +29.6% 却跑输 BH）

全运行 `n_entries_by_ladder = [0,0,0,2,2,0,...]`——**8.8 年仅 4 笔真实建仓**（ladder3 两笔 + ladder4 两笔，全在 2017–2020、价格 $4k–8k）。其余 4221 笔全是 reduce(2374)/recover(1243)/drain(592)/flip(2)/liq(12)/eod(2) 内部级联 churn。

### units 几何衰减链（notional 验证，排除 price 假象）

| 时段 | 价格 | max units (BTC) | max notional |
|------|------|-----------------|--------------|
| 2019 | $8712 | 15.92 | **$119,172** |
| 2020 中 | $7280 | 2.80 | $23,339 |
| 2020 末 | **$7152** | 0.101 | **$1,038** ← 价格几乎不变，notional 塌 22.5× |
| 2021 | $58142 | 2.9e-5 | $1.75 |
| **下跌段** | $43539 | **1e-6** | **$0.05** |

价格几乎相同（$7280→$7152）时 notional 塌 22.5 倍，shares 衰减 ~1590 万倍 vs 价格只涨 3.4 倍——**衰减在 deployed notional，绝非 price 假象**。但 NAV 逐 bar 守恒（`prove_nav_neutral`），units→现金时资本进 idle `free` 池未离开系统（equity 恒 129628）——**衰减 = 去杠杆，不是资本蒸发**。

### 衰减根因链
1. **sink 反复取 1/3 下放**（守恒、可逆）——core 每次 ×2/3
2. **recover 窄门控漏接**（`t_engine.rs:311`：仅子级恰持反父向短差且收同父向 BSP 才升回）——大量下放回不到 core
3. **enter 极罕见**（仅 4 次，要求全塔清零）——flip 之间无 idle free 再部署路径
4. **drain(592)/liq(12) 单向漏**——把 Σ\|units\| 变 idle 现金（累计 drain 变现 $326,355，liq $1542），次要量级

### 下跌段 69k→15k 踏空（bar 2218375 $69000 → 2760270 $15514）
- 段内 short_exposure ≤ 0.0003% 满仓，units ≈ 1e-6 BTC，整段对盈亏贡献 **−0.00**
- equity：顶部 129628.60 → 底部 129628.63（暴跌全程只动 +0.03）
- 全部 +29.6% 收益来自 2017–2020 大仓位期；最高级别 lad7 的 88 个 type3 买点全 `used=False`，唯一 used 的是顶部 type1 relabel
- **根因 = units 供给侧饿死（踏空），不是方向判断错**（段内最久空头 7/8 leg 方向对，从 $43k 空到 $29k，但 dust size）

---

## 6. 复现方法

```bash
export PATH="$HOME/.cargo/bin:$PATH"
# BTC 三模式（structural/and/or），写 t_engine_BTC_{mode}_trades.json
BT_SYMBOLS=BTC BT_DUMP_TRADES=1 \
  cargo test --release --manifest-path rust/Cargo.toml \
  recursive_t::t_engine_run::t_engine_8x3 -- --ignored --nocapture

# 全 8 标的：去掉 BT_SYMBOLS
# emergence-upgrade 默认 ON；T_NO_EMERGENCE=1 关闭（OFF 复现 5f93000e11 负值）
```

**注意**：
- Cargo.toml 在 `rust/` 子目录，用 `--manifest-path rust/Cargo.toml`。
- uncommitted `t_engine_run.rs`(诊断函数) / `operate.rs`(fugue_v3) **不在 t_engine_8x3 引擎路径**，不影响此基线。
- `analysis/data_cache/t_engine_*` 文件 ON/OFF 共享文件名会互相覆盖，对比须用隔离目录。

---

## 7. 结果包

**结论**：当前 main（HEAD `06c9739f7e` / 代码 `288d704a1b`）BTC T 引擎 structural = **+29.6%**（cargo 重跑确认，final_nav 129628.63 / n_trades 4225 逐字吻合），and = −29.1% / or = +21.8%，皆 <BH +1380%。记忆中 −89.7%(`5f93000e11`) / −77.6%(更早异架构引擎) 是被取代的演化快照——机制洞察仍效，数字非当前基线。units 账本：sink/recover 级间守恒、drain 单腿销毁 1/3 到 idle 现金、enter 全运行仅 4 次。+29.6% 跑输 BH 的根因 = 几何衰减把仓位饿成尘埃 + 下跌段踏空（units 供给侧，非方向侧）。

**定义依据**：缠论第 65 课 `Move(k)≡Level-(k+1) 笔` + ε 对称几何塔（sink=σ⁻¹∘τ / recover=σ∘τ）；`MOBILE_FRAC=1/LAMBDA` σ-不变配额（542 号）；recover 全量=次级别走势"完成"0/1 事件（543 号方案②）。

**边界条件**：(1) 后续 commit 改 t_engine.rs/recover 配额/涌现门 → 须重跑刷新本基线；(2) emergence-upgrade 默认 ON，`T_NO_EMERGENCE=1` 关则复现 5f93 负值；(3) `engine` 字段是静态标签，版本识别靠 commit + final_nav。

**下游推论**：「BTC T 引擎盈亏」的任何引用都应指向本文档的 §1 基线表 + §3 演化表，而非散落在各 session 记忆里的单点数字。抗踏空修复应指向 §5 根因链（core 仓位 floor + recover 门控放宽 + idle free 再部署），而非单堵 drain。

**谱系引用**：`project_t_engine_btc_baseline`（记忆基线）、`project_t_recover_full_vs_quota`（543 号开放轴#1）、`project_t_emergence_upgrade_ab`、`project_t_cross_level_coupling_falsified`（539 号根翻空有效域）、`project_t_operation_self_replication`、`project_t_short_close_level_mismatch`。

**影响声明**：纯文档产出，未改引擎代码/定义。本文档与记忆条目 `project_t_engine_btc_baseline.md` 互为镜像（记忆=检索入口，本文档=完整论证）。重跑覆盖了 `t_engine_BTC_{structural,and,or}_trades.json`（与原文件 bit-exact，无数据损失）。
