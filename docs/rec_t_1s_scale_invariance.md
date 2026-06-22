# recursive_t prove 守卫尺度不变性 1s 深度验证

**日期**：2026-06-22
**入口**：`rust/src/recursive_t/rec_stream.rs::scale_invariance_1s`（`#[ignore]`）+ `backtest_run.rs::load_clean_ohlc_window`
**跑法**：`CHANLUN_DATA_DIR=<data_cache> BT_1S_SYMBOLS={CL|BTC} cargo test --release scale_invariance_1s -- --ignored --nocapture`
**认识论标注**：4 panic 守卫零 fire = **L2**（CL+BTC 双标的 1s 双 regime）；滤波器深度 +1 = L2；L4+ 尺度不变性 **未验证**（有效域上界 = L3）。

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
