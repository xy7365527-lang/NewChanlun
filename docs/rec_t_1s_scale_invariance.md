# recursive_t prove 守卫尺度不变性 1s 深度验证

**日期**：2026-06-22
**入口**：`rust/src/recursive_t/rec_stream.rs::scale_invariance_1s`（`#[ignore]`）+ `backtest_run.rs::{load_clean_ohlc_window, load_clean_ohlc_window_ns}`
**跑法**：`CHANLUN_DATA_DIR=<data_cache> BT_1S_SYMBOLS={CL|BTC|ES|BRN} cargo test --release scale_invariance_1s -- --ignored --nocapture`
**认识论标注（更新 2026-06-22 L3）**：4 panic 守卫零 fire 已从 L2（CL+BTC 各 1 窗）升级为 **L3**——**4 标的（CL/BTC/ES/BRN）× 多窗口（每标的 3 个 disjoint 窗）** 全部零 fire。**L4 涌现并验证守卫成立**（CL/BRN/ES 三标的长窗 `highest_active=4`，4 panic 守卫在 L4 仍零 fire）⟹ **方向1 的 L3 有效域上界被突破**。§4 为 L3 增量，§1–§3 保留 L2 原始记录（谱系：L2→L3 生成史）。

---

## 0. 核心概念（编排者 2026-06-22 确立）

缠论级别递归 ≅ **多尺度滤波器组（小波 MRA）**。prove 守卫 = 滤波器组不变量的可执行形式。**1s = 奈奎斯特观测分辨率（非操作床位）**——提高采样率让滤波器组分辨更细尺度（多出深层），prove 守卫应在新层继续成立。本验证是该等价在**尺度维度**的首次实证（8 标的 L3 是**标的维度**，两者正交互补）。

> ⚠ 严守区分：1s 作**操作床位**（短差交易）= 零摩擦伪影已否证（[[project_cl_1s_a0_verdict]]）。本验证仅在观测分辨率维度，**未评估 1s 收益/alpha**。

---

## 1. 结论

**prove 守卫尺度不变性在 1s 更深滤波器层成立（L2，CL+BTC 双 regime 确认）。**

| 验证项 | CL 1s（577,388 bar, 油） | BTC 1s（1,209,600 bar, 加密） | 判定 |
|---|---|---|---|
| 4 panic 守卫零 fire | 进程零 panic（446 ops） | 进程零 panic（719 ops） | ✅ 尺度不变 |
| `sink_descends`/`sigma_quota`/`relabel_invariant` | 全程成立 | 全程成立 | ✅ |
| `bsp_triggers_operation`（no_trigger） | 0 | 0 | ✅ |
| `radial_scaling`（f∝λ⁻ᵏ）on sink_by_level | 0 违反层 `[105,43,42,0..]` | 0 违反层 `[258,111,26,0..]` | ✅ 更宽 k 几何递减 |
| `radial_scaling` on type1bsp_by_level | 0 违反层 `[403,154,73,0..]` | 0 违反层 `[574,249,90,0..]` | ✅ |

## 2. 滤波器组深度对照（小波 MRA 预言：高采样率 → 多分解层）

| 标的 | 1s 最深活跃层 | 1min 同窗最深 | Δ | 1s 新增通带（中枢） |
|---|---|---|---|---|
| CL（2025-04 窗，20.1×） | **L3** | L2 | **+1** | L3 通带 1646→0（仅 1s）、L2 通带 10364→16（648×） |
| BTC（2026-05-29..06-11 窗，76.4×） | **L3** | L2 | **+1** | 1s 多出 L2(17413)+L3(1216)，1min 全 0 |

提高采样率确实多出一层深层滤波器，prove 守卫在这些新层继续成立——与 [[project_cl_1s_a0_verdict]]「1s 真实多一层（lid4 涌现/lid3 中枢 3→33）」一致。

## 3. 结果包六要素

1. **结论**：滤波器比喻的尺度不变性有效域从 1min 扩展到 1s 成立（CL+BTC，L2）；`radial_scaling`/`sigma_quota`/`sink_descends`/`relabel_invariant` 在更细尺度均保持。
2. **定义依据**：`scale_invariance_1s` 测试 + `load_clean_ohlc_window`（ISO 日期切片，与 `load_clean_ohlc` 逐字相同清洗）；4 panic 守卫接入 `rec_engine.rs` 热路径（违反即 panic 终止，跑完 = 零 fire 的可执行证明）。RecStream::push_bar 是 bar-无关的，1s bar 直接喂。
3. **边界条件（守卫 fire = 滤波器自相似有效域边界）**：守卫会 fire 的尺度形态 = `sub≥parent`（区间套上浮）/ `m≠units/3`（级别依赖配额）/ ascend 改敞口 / 无触发源操作——CL/BTC 1s 三层塔均未出现。**当前塔深止于 L3，L4+ 尚未验证**（层数取决于行情反复次数非 bar 数，[[project_recursive_level_emergence]]）⟹ 有效域上界 = L3。
4. **下游推论**：4 panic 守卫是**尺度鲁棒的结构断言**（跨 1min/1s 零 fire 一致），与观测计数守卫（dir_mismatch/sink≠recover，regime 依赖）形成对比。完整 L3 尺度交叉验证需多标的×多窗口 1s（当前 CL/BTC 各 1 窗）。
5. **谱系引用**：[[project_cl_1s_a0_verdict]]（秒级=观测分辨率非操作床位）、`docs/prove_guards_migration.md`、546号（滤波器组等价确立）、[[project_four_prove_guards_l2]]（观测守卫 1min 读数，1s 同向）。
6. **影响声明**：改 2 文件纯 test/loader（`rec_stream.rs` 新测试 +223、`backtest_run.rs` 新窗口加载器 +87），**未改生产路径**，prove_guards/rec_engine 引擎逻辑零改动，bit-exact 保持。`load_clean_ohlc_window` 通用工具入主引擎为多标的 1s L3 铺路。

---

## 4. L3 交叉验证 + L4 触发（2026-06-22 增量）

§1–§3 是 **L2**（CL+BTC 各 1 窗）。本节扩展三维度，证据等级升 **L3**，并**突破 L3 有效域上界**（L4 涌现且守卫成立）。

### 4.1 三维度扩展

| 维度 | 方向1 L2 | 本节 L3 |
|---|---|---|
| 标的（宽度） | CL, BTC | **+ ES, BRN**（共 4 标的） |
| 窗口（同标的稳定性） | 各 1 窗 | **每标的 ≥3 个 disjoint 日期窗** |
| 塔深（L4 触发） | 止于 L3 | **CL/BRN/ES 长窗涌现 L4，守卫成立** |

### 4.2 L3 交叉验证矩阵（标的 × 窗口；4 panic 守卫零 fire = 进程未 panic）

| 标的 | 窗口 | bars | 最深活跃层 | 4 panic 守卫 | radial_scaling 违反层(sink/bsp) | no_trigger |
|---|---|---|---|---|---|---|
| **CL** | 1mo 全月（2025-04） | 577,388 | L3 | 零 fire | 0 / 0 | 0 |
| CL | 窗A 04-01..04-10 | 278,735 | L3 | 零 fire | **1** / 0 | 0 |
| CL | 窗B 04-21..04-30 | 187,146 | L3 | 零 fire | 0 / 0 | 0 |
| CL | 1y L4探测 2024-06-02..08-30 | 1,474,646 | **L4** ★ | 零 fire | 0 / 0 | 0 |
| **BTC** | 2w 全（2026-05-29..06-11） | 1,209,600 | L3 | 零 fire | 0 / 0 | 0 |
| BTC | 窗A 05-29..06-04 | 604,800 | L3 | 零 fire | 0 / 0 | 0 |
| BTC | 窗B 06-05..06-11 | 604,800 | L3 | 零 fire | 0 / 0 | 0 |
| **BRN** | 窗A 2024-06-02..06-30 | 377,115 | L3 | 零 fire | 0 / 0 | 0 |
| BRN | 窗B 2025-05-01..05-30 | 417,998 | L3 | 零 fire | **1** / 0 | 0 |
| BRN | 1y L4探测 2024-06-02..08-30 | 1,292,122 | **L4** ★ | 零 fire | 0 / 0 | 0 |
| **ES** | 窗A 2025-06-12..06-30 | 539,505 | **L4** ★ | 零 fire | **1** / 0 | 0 |
| ES | 窗B 2026-05-12..05-30 | 685,817 | **L4** ★ | 零 fire | 0 / 0 | 0 |
| ES | 1y L4探测 2025-06-12..09-10 | 2,508,266 | **L4** ★ | 零 fire | **1** / 0 | 0 |

**总计 13 个（标的×窗口）单元，4 panic 守卫全部零 fire。** L4 涌现于 3/4 标的（CL/BRN/ES）。

### 4.3 关键区分：panic 守卫 vs observation-count 守卫（务必不混淆）

`radial_scaling`（T50）与 `adjacent_same_dir`（T24）是 **observation-count 守卫（L2 regime 依赖，非 panic）**——`prove_guards.rs` 已显式标注「强趋势 regime 下高级别涌现频繁可局部违反（有效域读数）」。因此：

- 上表 `radial_scaling 违反层=1` 的 3 个单元（CL 窗A、BRN 窗B、ES 窗A/probe）**不是有效域边界破裂**，而是 **regime 读数**——某层 sink 计数 > 上一层（如 ES 窗A `sink_by_level=[114,28,19,24]`，L3=24 > L2=19）。这与方向1 已观察的 regime 依赖一致（dir_mismatch 跨窗 5%–66% 波动）。
- **4 panic 守卫（`sink_descends`/`sigma_quota`/`relabel_invariant`/`bsp_triggers_operation`）才是结构断言**——它们在全部 13 单元零 fire（含 4 个 L4 单元），**包括 L4 新增的第 4 层**。

### 4.4 L4 涌现机制（[[project_recursive_level_emergence]] 印证）

层数取决于**行情反复次数**非 bar 数：
- BTC 2 周（1.2M bar）止于 L3——加密 2 周行情反复不足。
- ES 即使 28 天窗（540k–686k bar）也涌现 L4——股指日内反复最密（`emrg=5` ops，最高）。
- CL/BRN 需 ~3 月窗（1.3–1.5M bar）才涌现 L4——油品反复频率介于两者之间。

L4 单元的滤波器通带形态（如 ES probe `[6932852,894322,119353,13534]`）保持几何递减，第 4 层（13534 走势组）prove 守卫零 fire ⟹ **滤波器自相似在第 4 层继续成立**。

### 4.5 L3 结果包六要素

1. **结论**：prove 守卫尺度不变性 **L2→L3 成立**——4 标的 × 多窗口（13 单元）4 panic 守卫零 fire；**L4 涌现且守卫成立**（CL/BRN/ES），方向1 的 L3 有效域上界被突破，新上界 = **L4**。
2. **定义依据**：复用 `scale_invariance_1s`（`run_one_scale`/`report_scale`）；新增 `load_clean_ohlc_window_ns`（databento `timestamps_ns` 切窗，与 `load_clean_ohlc` 逐字相同两遍清洗）。标的×窗口见 §4.2 全列。
3. **边界条件（守卫 fire = 有效域边界）**：4 panic 守卫的 fire 形态（`sub≥parent`/`m≠u·MOBILE_FRAC`/ascend 改敞口/无触发源操作）在 4 标的×多窗口×L4 均**未出现**。新上界 = **L4**（L5+ 未触发——本批窗口塔深止于 L4；ES 全量年（11.76M bar）/更长窗或可触发 L5，待验证）。observation-count 守卫（radial_scaling/adjacent_same_dir）的局部违反是 regime 读数非边界。
4. **下游推论**：4 panic 守卫是**尺度鲁棒 + 标的鲁棒 + 窗口鲁棒 + 塔深鲁棒（≥L4）**的结构断言——滤波器组不变量在缠论级别递归的可观测尺度全程成立。与 observation-count 守卫（regime 依赖）的对比在 L3 进一步强化：结构断言零 fire / 观测读数随 regime 波动。
5. **谱系引用**：[[project_cl_1s_a0_verdict]]（秒级=观测分辨率非操作床位）、[[project_recursive_level_emergence]]（层数~反复次数非 bar 数，印证 L4 触发条件）、`docs/prove_guards_migration.md`、546号（滤波器组等价 + 僵尸核心死锁）、[[project_four_prove_guards_l2]]（observation 守卫 1min 读数）。方向1 L2（commit 646c92bd0e）是本 L3 的直接前驱（L2→L3 生成史）。
6. **影响声明**：改 2 文件——`backtest_run.rs`（新增 `load_clean_ohlc_window_ns` + `iso_day_to_unix_ns` 无依赖 civil→ns 算术 + 1 个非 ignored 单元测试 `iso_day_to_unix_ns_锚点与边界`）、`rec_stream.rs`（`scale_invariance_1s` 扩展为 4 标的×多窗口×L4 探测 + L3 汇总矩阵）。**未改生产路径**，prove_guards/rec_engine 引擎逻辑零改动，bit-exact 保持，89→90 个 recursive_t 单元测试绿。`load_clean_ohlc_window_ns` 建议入主引擎（databento `timestamps_ns` schema 的通用时间窗切片工具）。
