# #925 前置勘察：`λ` / `f = 1/λ` 的全部实装、生产可达性、扫描存否、回填影响面

- 日期：2026-08-07
- 票：[#925](https://github.com/xy7365527-lang/NewChanlun/issues/925)（wayfinder:grilling，map #787，ADR 0017 连带处置 (b)）
- 性质：**AFK 只读探针**。未改任何 `.rs`。全部承重点已逐条打开确认（非 grep 命中数）。
- 基线：worktree HEAD 已 `git reset --hard main` 至 `a4f5d458f6`（初始 HEAD 为 `19b4015927`，属 `origin/main` 祖传线，已作废）。

---

## 0. 一句话结论

**主控「两处 `SUB_SPAWN_FRAC` 会静默劈叉」的判断成立**，但**范围低估了**：同一概念 `f = 1/λ` 在本仓有 **4 处独立实装、2 个互斥的 λ 值（2 与 3）**，四处**互不引用**；而其中**没有一处**落在当前 NT 生产路径上。

| # | 常量 | 文件:行 | 值 | 表达 | λ |
|---|---|---|---|---|---|
| 1 | `SUB_SPAWN_FRAC` | `rust/src/spiral/params.rs:37` | 0.5 | `1.0 / LAMBDA`（参数化，`:32` `LAMBDA=2.0`） | 2 |
| 2 | `SUB_SPAWN_FRAC` | `rust/src/trading/positional_fusion.rs:104` | 0.5 | `0.5`（**硬编码**，doc `:98` 却写「值 = 1/λ」） | 2（隐含） |
| 3 | `MOBILE_FRAC` | `rust/src/fugue_v3/mod.rs:115` | 1/3 | `1.0 / LAMBDA`（参数化，`:111` `LAMBDA=3.0`） | 3 |
| 4 | `MOBILE_FRAC` | `rust/src/recursive_t/rec_engine.rs:69` | 1/3 | `1.0 / 3.0`（**硬编码**，模块内私有，遮蔽 #3） | 3（隐含） |

穷举依据：`grep -rn "\bLAMBDA\b" rust/` 全仓 **4 命中**（`spiral/params.rs:32,:37`、`fugue_v3/mod.rs:111,:115`）；`grep -rn "SUB_SPAWN_FRAC" rust/` 全仓 **19 命中**（含 doc/测试）；`grep -rn "MOBILE_FRAC" rust/` 覆盖 `fugue_v3/`、`recursive_t/`。覆盖目录 = `rust/` 全树。

> 同名异物排除（已逐条打开）：`rust/src/theta_v0/backtest/capture_oos.rs:49-51` 的 `LAMBDA_LO/HI/Q` 是**成交价滑点分位参数**（0.05%~1.0%），与尺度比无关；`rust/src/bin/p112_*.rs:370` / `p113_*.rs:184,207` 的 `lambda_c` / `lambda_a` 是**走势 C/A 段起点索引**，与尺度比无关。

---

## Q1：两套 `SUB_SPAWN_FRAC` 各自的生产可达性

### 前置：本仓「生产」有三个互不相同的所指

1. **NT 实盘/回测策略**（`trading_system/strategy/rec_t_strategy.py:45` `newchan_rust.RecTStream(config.mode)`）→ 驱动 `recursive_t::rec_engine` → 用实装 **#4**（`rec_engine.rs:69` 私有 `MOBILE_FRAC=1/3`）。
2. **π（`theta_v0`）**——`rust/src/trading/mod.rs:53-54` 与 `rust/src/spiral/mod.rs`（GUARD-ROLE 块）逐字：「theta_v0（π，唯一现役引擎）对本目录生产引用 = 0」。已复核：`grep -rn "LAMBDA|SPAWN_FRAC|MOBILE_FRAC" rust/src/theta_v0/` **零命中**（`capture_oos.rs` 三处是同名异物，见上）。π 与本票四处实装**完全无关**。
3. **落盘读数的产出路径**——python/PyO3 出口跑回测并把 JSON 写进仓。**这一条才是 #925 该关心的**。

### 判定 A：`spiral/` 那一套（实装 #1）

**可达，且已产出在册落盘读数。**

链路（逐环打开确认）：
`rust/src/spiral/params.rs:37` → `accounting.rs:20` `use super::params::{…, SUB_SPAWN_FRAC}` → `accounting.rs:202-203` `sigma_invariant_quota` → `accounting.rs:241` `let m_quota = sigma_invariant_quota(p_units)`（在 `try_spawn_cost_gated` 内，`:215`）→ `engine.rs:27` import、`engine.rs:323` 唯一调用点（E 降成本分支）→ `ffi.rs` `PySpiralStream` / `run_spiral` → `rust/src/lib.rs:2689-2690` pymodule 注册 → python。

python 消费者 **2 处**（`grep -rln "SpiralStream" .`，覆盖全仓）：
- `trading_system/backtest_spiral_stream.py:148` `nr.SpiralStream(floor_ladder=FLOOR)`
- `trading_system/compare_spiral_unn.py:60` 同上

**落盘读数在仓**：`trading_system/data_cache/spiral_stream_{BTC,BRN,CL,DX,ES,GC,OKLO,QQQ}.json`（8 标的）。BTC 那份实测含配额敏感字段：`spiral.spawns_by_ladder = [0,0,9650,24,2,0,…]`、`strat_pct = 38.7`、`mdd_pct = -49.8`、`n_trades = 9677`、`max_children = 1001`。`spawns` 非零 ⟹ `try_spawn_cost_gated` 的 `Some(_)` 分支**确实触发过 9676 次**，`SUB_SPAWN_FRAC` 是这些数字的直接输入。

**⚠️ 名分标注的关键订正**：`rust/src/spiral/mod.rs` 的 GUARD-ROLE 块把本族判「现役」，理由是 (1) 两处 python 调用 + (2) `fugue_v3` 对 `spiral::signal` 的编译期硬依赖。**理由 (2) 与 `SUB_SPAWN_FRAC` 无关**——`fugue_v3` 只 `use crate::spiral::signal::{…}`（`axis.rs:22` / `observe.rs:17` / `morphology.rs:17` / `engine.rs:19-20` / `operate.rs:23`），**从不 use `spiral::params` 或 `spiral::accounting`**。故实装 #1 的可达性**只靠那两处 python 冷调用**（`git log -1` 均 2026-06-21，之后未再跑）。

另有在册登记与此一致：`.chanlun/implementation-index-20260723.md:144` 逐字「`rust/src/spiral/accounting.rs:196-291` sigma_invariant_quota/try_spawn_cost_gated（f=1/λ 级别无关配额 spawn）——**仅 PyO3 出口**」。

### 判定 B：`trading/positional_fusion.rs` + `trading/unified_necessity.rs` 那一套（实装 #2）

**可达，且 python 调用面远宽于 spiral。**

链路：
`positional_fusion.rs:104` `pub(crate) const SUB_SPAWN_FRAC: f64 = 0.5` → `unified_necessity.rs:195` `use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT, SUB_SPAWN_FRAC}` → `:994-995` `sigma_invariant_quota` + `:1064` `let m_quota = p_units * SUB_SPAWN_FRAC`（在 `try_spawn_cost_gated` 内，`:1023`）→ `:1642` 唯一生产调用点 → `run_unified_necessity` → `positional.rs:1052` mode 分派（`PolarityMode::UnifiedNecessity`）→ `run_positional_rust` / `UnnStream`（`lib.rs:2480-2490,2687`）→ python。

**`positional_fusion::SUB_SPAWN_FRAC` 的消费者只有 1 个模块**（`unified_necessity`）——`grep -rn "SUB_SPAWN_FRAC" rust/` 全仓核验，`trading/` 内除 `positional_fusion.rs` 定义处外仅 `unified_necessity.rs` 5 处命中（`:195` import、`:993` doc、`:995`、`:1013` doc、`:1052/:1057` 注释、`:1064`、`:2531/:2540/:2548/:2551` 测试）。

python 消费者（`grep -rn '"unn"' --include=*.py .`，覆盖全仓）：`trading_system/backtest_unn.py:179`、`backtest_unn_stream.py:154`、`analysis/unified_necessity_backtest.py:120`、`analysis/investigate_qqq_liq.py:29`、`analysis/unn_1s_a0_runner.py`、`analysis/run_unn_es_1s.py`、`analysis/unn_d_recover_pairing_check.py:59`、`analysis/verify_unn_stream.py:96` —— **已查到 8 个脚本**。`rust/src/trading/mod.rs:38-40` 另登记「`analysis/` 下有 60+ 处真实非测试调用」覆盖全部 mode（本探针未逐条复核那 60+ 的 mode 归属，只复核了 `mode="unn"` 的 8 处）。

**落盘读数在仓**：`trading_system/data_cache/unn_stream_{BTC,BRN,CL,DX,ES,GC,OKLO,QQQ}.json` + `unn_nt_{CL,ES,OKLO,QQQ}.json`。BTC 那份：`strat_pct = 12.0`、`mdd_pct = -6.5`、`n_trades = 114`、`nrf_counters.spawns = [0,0,89,24,0,…]`。同样是配额敏感读数。

### 判定 C：两套是什么关系

**同一算法的两代实现，v2（spiral）对 v1（unn）做 bit-exact 对照，两代都未退役。**

承重：
- `rust/src/spiral/mod.rs:14-16`（模块头 §「与现有 unn 引擎的关系」）逐字：「v2 **不删除** `trading::unified_necessity`（保留为 bit-exact 对照基线，直到 v2 验收通过）。v2 复用信号层契约 + 会计语义」。
- `rust/src/spiral/accounting.rs:213-215` doc 逐字：「**bit-exact 前提（Step 6）**：成本门 θ 来源 = unn 的 `DepthRef`（复用 pub 类型，不改现有引擎）——与 unn `try_spawn_cost_gated` 同一 `theta(sub, None, q, min_obs)` 调用，使 E spawn 决策与 unn 逐 bar 一致（差异隔离到群作用路由层）」。
- 对账脚本 `trading_system/compare_spiral_unn.py:1`（「逐 bar bit-exact 对账（架构 §2.4 判据2）」）、`:88`（「trade 表逐行比对（bit-exact = 完全相等）」）、`:110`（PASS / MISMATCH 打印）。

**未查到任何 `#[deprecated]` 标记或退役声明**覆盖这两族——两族的 GUARD-ROLE 块（`spiral/mod.rs`、`trading/mod.rs`）都明写「名分：**现役**」「零删除，零移入 `legacy/`」。

### 判定 D：哪一套的 `SUB_SPAWN_FRAC` 真正影响到会落盘的读数

**两套都影响，且影响的是不同的落盘文件；但两套影响的都不是当前 NT 生产路径的读数。**

- 实装 #1（spiral）→ `data_cache/spiral_stream_*.json`（8 标的），最后落盘 commit `cc017e4e61`（2026-06-21）。
- 实装 #2（unn）→ `data_cache/unn_stream_*.json` / `unn_nt_*.json`（8+4），最后落盘 commit `e2a78b0dd7`（2026-06-16 11:42，即 542 落地当天）。
- 实装 #4（`rec_engine.rs:69`）→ NT 生产策略 `rec_t_strategy.py` 的读数，以及 `analysis/capture_ratio_matrix.py:84` / `t3_exit_trigger_diag.py:35` 的 `RecTStream` 读数。**#925 若只回填 `spiral/params.rs:32` 的 `LAMBDA`，生产读数一动不动。**

**⟹ 对 #925 的直接含义**：`SUB_SPAWN_FRAC` 回填**不会改变任何当前生产读数**，只会改变两族对照引擎的历史落盘读数。真正吃 λ 的生产常数是 `MOBILE_FRAC`（λ=3），而它**不在 542 的裁决范围内**（542 的 `source` 字段只点 `positional_fusion.rs` 与 `unified_necessity.rs`）。

---

## Q2：`LAMBDA` 的全部消费者

**`rust/src/spiral/params.rs:32` 的 `LAMBDA` 在全仓只有 1 个消费者：同文件 `:37`。**

检索式与覆盖目录：
```bash
grep -rn "\bLAMBDA\b" rust/                 # 全仓 4 命中，见 §0 表
grep -rn "params::LAMBDA\|use super::params" rust/src/spiral/   # 5 命中，无一含 LAMBDA
grep -rn "params::" rust/src/ | grep -v "^rust/src/spiral/params.rs"   # 6 命中，无一含 LAMBDA
```
`spiral::params` 的跨文件 import 逐条列出（全部打开确认）：
- `accounting.rs:20` → `{SUB_COST_K, SUB_COST_MIN_OBS, SUB_COST_Q, SUB_FRICTION_RT, SUB_SPAWN_FRAC}`
- `signal.rs:22` → `{DEPTH_REF_WINDOW, FIRST_BSP_LADDER, PENDING_LO}`
- `prove.rs:27` → `{FIRST_BSP_LADDER, PENDING_LO}`；`prove.rs:200` 函数内 → `SUB_SPAWN_FRAC`
- `engine.rs:28` → 多常量（不含 `LAMBDA`）；`engine.rs:147` → `params::SUB_LIQ_FACTOR`

`LAMBDA` 亦未经 PyO3 导出（`grep -rn "SPAWN_FRAC\|spawn_frac" rust/src/lib.rs` 零命中），也无环境变量覆盖（`grep -rn "env::var" rust/src/trading/positional_fusion.rs rust/src/spiral/` 零命中）。

**⟹ 改 `spiral/params.rs:32` 的连带面 = 恰好 `spiral::params::SUB_SPAWN_FRAC` 一个符号**，再顺链到 `spiral/accounting.rs` 与 `spiral/prove.rs`。`trading/positional_fusion.rs:104` **不会跟着动**——两处零引用关系（`positional_fusion.rs` 的 use 列表 `:68-82` 逐行确认，无 `crate::spiral`）。**主控判断成立。**

同理，`fugue_v3/mod.rs:111` 的 `LAMBDA` 只有 1 个消费者（`:115` `MOBILE_FRAC`）；而 `MOBILE_FRAC` 有跨模块消费者（`fugue_v3/accounting.rs:25,83`、`fugue_v3/prove.rs:26,42`、`recursive_t/prove_guards.rs:31,277`）。**注意 `recursive_t/rec_engine.rs:69` 自己定义了同名私有 `MOBILE_FRAC=1.0/3.0` 遮蔽了 `fugue_v3::MOBILE_FRAC`，但同文件 `:65` 又 `use crate::fugue_v3::SUB_LIQ_FACTOR`** —— 即 `rec_engine` 算配额用私有的，而它调用的守卫 `prove_guards::prove_sigma_quota`（`prove_guards.rs:277`）重算 canonical 用的是 `fugue_v3::MOBILE_FRAC`。**这是与主控指出的同一形态的第二处劈叉**，目前两值都是 1/3 所以不显形；改 `fugue_v3/mod.rs:111` 会让 `rec_engine` 的每次 sink/drain **运行时 panic**（`prove_guards.rs:279-281`）。

---

## Q3：542 自陈的「全 8 标的回测扫描」做过没有

**结论：在下列已查到的范围内，没有做过。**

542 的原话（`.chanlun/genealogy/settled/542-spawn-allocation-sigma-invariant.md:4`）：「值 1/λ 因 λ（级别尺度比 A₅）是涌现量（T50 Δt(k)∝λᵏ）未独立测，作 leverage_triad **唯一自由度回测扫描**」；实装侧同义句在 `rust/src/trading/positional_fusion.rs:100-103`：「待全8标的回测扫描 + T50 涌现 λ 测量精化为 f=1/λ_measured」。

### 检索式与结果（逐条）

| 检索 | 覆盖 | 结果 |
|---|---|---|
| `grep -rln "SUB_SPAWN_FRAC" . --exclude-dir=.git --exclude-dir=target` | 全仓 | 15 文件，全部打开确认：6 个 `.rs` + 谱系/上浮/ADR/架构文档。**无一是扫描结果** |
| `grep -rln "leverage_triad" . --exclude-dir=.git --exclude-dir=target` | 全仓 | 12 文件，均为定义/诊断/裁决文本，**无扫描结果** |
| `ls rust/src/bin/` + `grep -rln "SPAWN_FRAC\|MOBILE_FRAC\|LAMBDA\|lambda" rust/src/bin/` | 50 个 bin | 5 命中，逐条打开：全部是 `lambda_c`/`lambda_a`（走势段起点索引）**同名异物**。**无扫描 bin** |
| `git log --oneline --all --grep="SPAWN_FRAC"` | 全 ref | 3 commit：`346ac05068`（542 实装）/ `e2a78b0dd7`（守卫）/ `5a58a3d0ee`（谱系落盘）。**全是落地，非扫描** |
| `git log --oneline --all --grep="leverage_triad\|配额扫描\|f 扫描\|λ 扫描"` | 全 ref | 仅 `346ac05068`。**无扫描 commit** |
| `gh issue list --search "SUB_SPAWN_FRAC" --state all --limit 30` | tracker 含 closed | 仅 #925 本票 |
| `gh issue list --search "leverage_triad" --state all --limit 30` | 同上 | 仅 #925 |
| `gh issue list --search "配额" --state all --limit 30` | 同上 | 30 条，无一是 f 扫描 |
| `grep -rn "env::var" rust/src/trading/positional_fusion.rs rust/src/spiral/` | — | 零命中 ⟹ **无参数化入口，扫描在技术上需改源码重编译** |

### 与之最接近但**不是**它的两件事（防误引）

1. **`.chanlun/escalations/2026-06-19-1922-t-recover-quota-full-vs-sigma-invariant.md`** —— 有一张 8 标的 × 3 模式的 L3 受控对照表（GC +50.7pp / BTC +46.6pp / OKLO +31.1pp / QQQ +2.0 / DX +0.9 / CL −2.1 / BRN −10.0 / ES −19.6），判决「全量 recover = regime 方差缩减器，非普适 alpha」。**但它的自变量是 `t_engine.recover` 的「1/3 → 全量」二值切换，不是 `SUB_SPAWN_FRAC` 的 f 值扫描**，且作用对象是 `recursive_t::t_engine`（实装 #4 族），**与 542 点名的 `positional_fusion`/`unified_necessity` 不同族**（该上浮文件自陈：裁决「仅改 t_engine，不碰 operate.rs/542 守卫」）。
2. **ADR 0016 裁定二**（`docs/adr/0016-*.md`，由 #835 落盘）—— 否决固定倍率 `Q(ℓ)=Q₀·f^ℓ`，四条理由之一即引上述 8 标的实测。**它否决的是「按级别几何递减发钱」这个形状，不是「f 该取多少」的扫描**。

### 反面证据（扫描未做的正面痕迹）

- `docs/spiral_engine_v2_architecture.md:49` 至今写着「`λ=2`（SUB_SPAWN_FRAC=0.5）| **L2 待测** | A₅ 二分递归建模默认，待 T50 涌现 λ 测量精化」。
- 同文件 `:619` 参数审计表：「SUB_SPAWN_FRAC | 0.5 | ⚠ **形式✅/值❌** | L0 形式 + L2 值」。
- `.chanlun/review-results/issue844-multi-fugue-asset-census-20260801.md:238` 逐字：「(b) 已实装但 λ 值 `542` **自陈「未独立测」**」。
- ADR 0016 裁定二第 2 条逐字：「值 `f=1/3` 是 C 档——代码里两个互斥 λ 并存（`spiral/params.rs:32` `LAMBDA=2.0` vs `fugue_v3/mod.rs:111` `LAMBDA=3.0`）」。

**⟹ 三份独立在册文档（架构表、census、ADR）在 542 落地后 1.5 个月仍把 λ 记为「待测/值❌」，与「扫描未做」一致。**

---

## Q4：改 `SUB_SPAWN_FRAC` 的影响面清单

设回填目标为 `f = 1/5.96 ≈ 0.168`（#907 时间比）或 `f = 1/3.10 ≈ 0.32`（#915 容量比）。

### 4.1 配额计算的全部调用点（穷举，检索式 `grep -rn "prove_theta_sigma_invariant\|sigma_invariant_quota" rust/src/`，覆盖 `rust/src/` 全树）

| 调用点 | 性质 |
|---|---|
| `rust/src/spiral/accounting.rs:202-203` `sigma_invariant_quota` | 定义 |
| `rust/src/spiral/accounting.rs:241` | **生产调用**（`try_spawn_cost_gated` 内） |
| `rust/src/spiral/prove.rs:199-207` `prove_theta_sigma_invariant` | 守卫定义 |
| `rust/src/spiral/accounting.rs:244` | 守卫调用 |
| `rust/src/spiral/prove.rs:647` / `:654` | **单测（硬编码值，见 4.3）** |
| `rust/src/trading/unified_necessity.rs:994-996` | 定义 |
| `rust/src/trading/unified_necessity.rs:1064` | **生产调用**（`try_spawn_cost_gated:1023` 内，被 `:1642` 调用） |
| `rust/src/trading/unified_necessity.rs:1008-1017` | 守卫定义 |
| `rust/src/trading/unified_necessity.rs:1068` | 守卫调用 |
| `rust/src/trading/unified_necessity.rs:2534-2540` / `:2551-2552` | 单测 |

另有同形态的第三/四族守卫（**不吃 `SUB_SPAWN_FRAC`，吃 `MOBILE_FRAC`**，改 `SUB_SPAWN_FRAC` 不动它们）：`rust/src/fugue_v3/prove.rs:38-46 prove_sigma_quota`、`rust/src/recursive_t/prove_guards.rs:269-283 prove_sigma_quota`。

### 4.2 运行时守卫会不会 panic

**不会。两族守卫对 `SUB_SPAWN_FRAC` 的值都是"齐动"的，改值不触发 panic。**

- `unified_necessity.rs:1064` 算 `m_quota = p_units * SUB_SPAWN_FRAC`；`:1009` canonical = `sigma_invariant_quota(p_units)` = `p_units * SUB_SPAWN_FRAC`。**同一常数，恒等**。
- `spiral/accounting.rs:241` 更直接：`m_quota = sigma_invariant_quota(p_units)`，`:244` 守卫再调 `p_units * SUB_SPAWN_FRAC`。

**⚠️ 附带发现（不属票面，但影响对守卫强度的判断）**：`spiral/accounting.rs:242-243` 的注释逐字写「**独立内联表达** ⇒ 漂移回 sub-依赖分配即 panic」，但 `:241` 的实际代码是**直接调用 canonical 函数 `sigma_invariant_quota`**，不是独立内联表达。unn 侧（`:1005` doc + `:1064` 代码）名副其实（写的是 inline `p_units * SUB_SPAWN_FRAC`）。spiral 侧这条注释与代码不符 —— 该守卫在 spiral 里是**真重言**，不能检出任何漂移。此项与 #928（注释与事实不符）同类，建议单开 debt 票，本探针不裁。

### 4.3 会因此变红的测试（穷举，`cargo` 未跑，纯读源判定）

| 测试 | 位置 | 判定 |
|---|---|---|
| `theta_sigma_invariant_holds_on_canonical_quota` | `rust/src/spiral/prove.rs:645-648` | **★ 必红**。`prove_theta_sigma_invariant(50.0, 100.0, 3, 0)` **硬编码 50/100 = 0.5**。f→0.168 时 canonical=16.8，\|50−16.8\|=33.2 ≫ 容差 `1e-9×100` ⟹ assert 失败。**这是唯一一处会因回填而无条件变红的测试** |
| `theta_sigma_invariance_fires_on_level_dependent_quota` | `rust/src/spiral/prove.rs:650-655` | 条件红：`should_panic`，传 30.0/100.0。f=0.168/0.32 时仍 panic ⟹ 绿。**但若 f 恰被设为 0.3 则变红** |
| `theta_sigma_invariant_holds_for_constant_quota` | `rust/src/trading/unified_necessity.rs:2529-2541` | 绿。用 `sigma_invariant_quota(p_units)` 派生，且 `:2540` 断言也是相对式 |
| `theta_sigma_invariance_fires_on_level_dependent_quota` | `rust/src/trading/unified_necessity.rs:2543-2553` | 绿（同上，除非 f 设为 0.3） |
| `cost_gate_constants_match_organic_defaults` | `positional_fusion.rs` 内（doc `:84-86` 点名） | 绿——只守 `SUB_COST_K`/`SUB_FRICTION_RT`，**不守 `SUB_SPAWN_FRAC`** |

**fixture / 快照测试：不受影响。** `rust/tests/` 全部 11 个集成测试逐个查名与内容归属：`theta_v0_*`（8 个，π 族）、`econ_oddeven_diagnosis.rs`、`issue533_p123_byte_guardrail.rs`、`nest_isolation_guard.rs` —— **无一触达 spiral/unn 配额**。`rust/tests/fixtures/theta_v0_parity.json` 与 `theta_v0_center_parity.json` 由 Lean 导出、属 π 族，与本票无关，`scripts/check_fixture_drift.py` 不会因此变红。

**跨引擎一致性守卫：不存在。** 已查：无任何 Rust 测试断言 `spiral::params::SUB_SPAWN_FRAC == positional_fusion::SUB_SPAWN_FRAC`（`grep -rn "SUB_SPAWN_FRAC" rust/` 19 命中全部打开确认）。**这正是「静默劈叉」成立的机制：编译期与测试期都没有任何东西会发现两值不等。**

### 4.4 会失效的落盘读数与在册引用

**A. 直接失效（数值由 `SUB_SPAWN_FRAC` 直接决定，回填即全部过期）**

| 读数 | 落盘 commit / 日期 | 说明 |
|---|---|---|
| `trading_system/data_cache/spiral_stream_{BTC,BRN,CL,DX,ES,GC,OKLO,QQQ}.json`（8 份） | `cc017e4e61`，2026-06-21 | `strat_pct` / `mdd_pct` / `n_trades` / `spawns_by_ladder` / `max_children` 全部配额敏感 |
| `trading_system/data_cache/unn_stream_{同 8 标的}.json`（8 份） | `e2a78b0dd7`，2026-06-16（542 落地当天） | 同上 |
| `trading_system/data_cache/unn_nt_{CL,ES,OKLO,QQQ}.json`（4 份） | 同批 | 同上 |

**B. 在册文本引用（引了 A 的数字或引了 f=0.5 这个值）**

| 文件 | 关系 |
|---|---|
| `.chanlun/genealogy/settled/542-spawn-allocation-sigma-invariant.md` | **裁决本体**。`:4` 值判据 / `:9` settlement / `:10` source / `:61-62` 双角色表。回填 = 修订本文件 `epistemological_level` 的「值 L2」一栏 |
| `docs/spiral_engine_v2_architecture.md:49` / `:368` / `:619` / `:632` | 参数审计表三处 + 结构层清单，均钉「λ=2 / 0.5 / 形式✅值❌」 |
| `docs/necessity_derivation.md:127` / `:458` / `:495` / `:554` / `:586` / `:722` | 环 15/16/20/22 的 T₁₈ 裁决行与 prove 登记 |
| `docs/dialectical_exhaustion.md:201` / `:322` / `:666` / `:1058` | 会计配额 f 行 |
| `docs/adr/0017-*.md:279` / `:339` | T-11 条目已登记「走上浮（连带处置 (b)）」+ 已点名两份同名常数 |
| `docs/adr/0016-*.md:60` | 裁定二第 2 条已登记两个互斥 λ |
| `.chanlun/escalations/2026-06-15-theta-allocation-non-necessity.md` | 上浮本体 |
| `.chanlun/review-results/issue844-multi-fugue-asset-census-20260801.md:126` / `:238` | R6 比例三套互斥的 (b) 项 |
| `.chanlun/implementation-index-20260723.md:144` | 「仅 PyO3 出口」登记 |
| `analysis/spiral_solution_to_underperformance.md:49` / `:101` / `:134` / `:190` | 引 f=1/λ 零参数论证 |
| `analysis/unn_theta_allocation_necessity_diagnosis.md` §3 | σ-等变诊断（542 的证明基础） |
| `analysis/spiral_v2_verification_verdict.md` / `analysis/unn_latest_behavior_analysis.md` / `analysis/unn_1s_a0_verdict.md` | 引 A 类读数的判决文本，最后动于 `cc017e4e61`（2026-06-21） |
| `formal/Tlayers/Operational.lean` | 含 `leverage_triad`（本探针未打开核验其是否含 f 的数值，**标注为未查清**） |

**C. 明确不受影响（已复核，非「查不到就写不受影响」）**

- **π（`theta_v0`）全部读数** —— 依据：`grep -rn "LAMBDA|SPAWN_FRAC|MOBILE_FRAC" rust/src/theta_v0/` 零命中（同名异物已排除）+ `trading/mod.rs:53-54` 与 `spiral/mod.rs` 两处 GUARD-ROLE 逐字「π 对本目录/本族生产引用 = 0，π 完全自包含」。
- **NT 生产策略 `rec_t_strategy.py` / `RecTStream` 读数** —— 依据：`rec_engine.rs:69` 用自己的私有 `MOBILE_FRAC=1.0/3.0`，与 `SUB_SPAWN_FRAC` 零引用关系（`rec_engine.rs:65` 唯一从 `fugue_v3` import 的是 `SUB_LIQ_FACTOR`）。
- **`isolated_fugue`（iso）路径** —— 542 自陈（`:81`）「iso 不调 `try_spawn_cost_gated`（其 spawn 路径独立，`isolated_fugue.rs:407` 自有 θ 配额）」。本探针**未独立复核 `isolated_fugue.rs:407`**，此项照搬 542 自陈，标注为**未复核**。

### 4.5 若一并回填 `MOBILE_FRAC`（λ=3 那两处）的额外影响 —— 仅提示，不属票面

- `rust/src/fugue_v3/prove.rs:156` `prove_sigma_quota(30.0 * MOBILE_FRAC, 30.0, 4, 0)` 与 `rust/src/recursive_t/prove_guards.rs:442` 同形——**都是派生式，改值仍绿**。
- **但 `rec_engine.rs` 会运行时 panic**：它用私有 `MOBILE_FRAC=1/3` 算 `m`，守卫 `prove_guards.rs:277` 用 `fugue_v3::MOBILE_FRAC` 重算 canonical。只改 `fugue_v3/mod.rs:111` ⟹ 两者不等 ⟹ `prove_guards.rs:279` assert 失败 ⟹ **NT 生产策略每次 sink/drain 直接 panic**。承重：`rec_engine.rs:1700` / `:1810` 两处 `prove_sigma_quota` 调用点注释「移植守卫（L0，从 spiral/fugue_v3）」。
- 这条影响的是**生产**，而 `SUB_SPAWN_FRAC` 那两处影响的**不是**。若 #925 的裁决要落到生产，落点在 `MOBILE_FRAC` 不在 `SUB_SPAWN_FRAC`。

---

## 5. 对主控判断的裁定

> 「回填 λ 时若只改 `LAMBDA`，两处会静默劈叉。」

**成立。** 三条独立承重：

1. `positional_fusion.rs:104` 是字面量 `0.5`，其 use 列表（`:68-82`，逐行确认）不含任何 `crate::spiral` 项 ⟹ 改 `spiral/params.rs:32` 对它零效果。
2. 全仓无任何测试或断言把两个 `SUB_SPAWN_FRAC` 挂钩（`grep -rn "SUB_SPAWN_FRAC" rust/` 19 命中全部打开确认）⟹ 编译期、测试期都不报警。
3. 劈叉的唯一显形处 `trading_system/compare_spiral_unn.py`（bit-exact 对账）**不在 CI 里**（`.github/workflows/ci.yml` 的 job 为 π 族 + `fixture-drift`），只在人手动跑时才会打 `❌ MISMATCH` ⟹ 名副其实的**静默**。

**订正 / 扩充三点**：
- **范围低估**：同一概念有 4 处实装、2 个互斥 λ；`fugue_v3` ↔ `rec_engine` 之间存在同形态的第二处劈叉，且那一处**会 panic 而非静默**。
- **票面未提但已在册**：ADR 0017 `:339` 已逐字登记「全仓另有第二份同名常数 `rust/src/spiral/params.rs:37`（与 `positional_fusion.rs` 的那份互不引用）」；ADR 0016 `:60` 已登记两个互斥 λ。**主控的发现与在册记录一致，不是新发现，但票面 #925 确实没引这两处。**
- **优先级订正**：`SUB_SPAWN_FRAC` 的劈叉后果是「两份历史对照读数不再可比」；`MOBILE_FRAC` 的劈叉后果是「生产引擎 panic」。#925 若只盯 `SUB_SPAWN_FRAC` 会漏掉后者。

---

## 6. 没查到 / 不确定（照实登记，090）

1. **`formal/Tlayers/Operational.lean` 里的 `leverage_triad` 是否含 f 的数值** —— 只做了 `grep -l` 命中，**未打开文件核验**。若 Lean 侧钉了 0.5，回填会连带 fixture 漂移，本报告未覆盖此风险。
2. **`isolated_fugue.rs:407` 自有 θ 配额、不受 542 影响** —— 照搬 542 自陈（`:81`），**未独立打开该行复核**。
3. **`rust/src/trading/mod.rs:38-40` 登记的「`analysis/` 下 60+ 处非测试调用」** —— 本探针只复核了其中 `mode="unn"` 的 8 个脚本，**未逐条核验其余调用的 mode 归属**，故无法断言「unn 路径的 python 消费者恰好 8 个」，只能说**已查到 8 个**。
4. **未跑 `cargo test`** —— §4.3 的红/绿判定是纯读源推断（对 `spiral/prove.rs:647` 而言推断链很短：常数 50.0 对 canonical=100×f，f≠0.5 即失败），**未实测验证**。
5. **两族落盘 JSON 是否被更晚的报告重新引用过** —— 只查到最后一次改动 commit（`cc017e4e61` / `e2a78b0dd7`），**未穷举所有引用这些数字的下游文本**，§4.4-B 表是已查到的，不声明穷尽。
6. **`λ` 该不该回填、回填成 5.96 还是 3.10** —— 本票是探针，**不裁**。但一条事实供裁决参考：#907 的 5.96 是**时间间隔比**、#915 的 3.10 是**容量比**，而 542 定义的 λ 是「级别的时间尺度比」（`spiral/params.rs:30` 逐字「尺度比 λ（A₅ 二分递归建模默认）」，配 T50 `Δt(k)∝λᵏ`）——**按定义只有 #907 的 5.96 同量纲**；`fugue_v3/mod.rs:112` 的 λ=3 依据则是原文 `026:80`「用其中的 1/3」，**是仓位比例不是时间比**，与 542 的 λ 本就不是同一个量。这三个数字**是不是同一个 λ，本身就是 #925 该裁的第一件事**。
