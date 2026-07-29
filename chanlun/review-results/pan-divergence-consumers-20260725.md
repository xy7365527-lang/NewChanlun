# 盘整背驰在生产路径上的消费通道核查

- 日期：2026-07-25
- 执行：codex exec（gpt-5.6-sol，read-only，effort=high）
- 落盘：主控 session 代写（codex 处于只读沙箱，写入被拒；本文内容为其核查结论，锚点未改动）
- 触发：编排者质疑「我们的实现不管盘整背驰？你确定？」

## 一句话结论

**「实现不管盘整背驰」作为整体陈述是错的。**

准确表述：

> 盘整背驰被排除在**同级 B1/S1** 和 **`Cand^δ` / `N^δ` 严格区间套链**之外；`pan_div_diag` 字段只有诊断用途。与此同时，实现保留独立的 `PanDivCert` 生产通道，用于 Nest/XZD 承接统计，并在**默认关闭**的 `center_oscillation` 开关开启时，驱动受优先级与 KΘ 风控约束的震荡 ShortDiff 精确开平仓。

主控注：先前对话中我把「不进区间套链」表述成了「实现不管盘整背驰」，是过度概括。编排者质疑成立。

## 1. `judge_pan_div` 的全部调用点

| 位置 | 用途 |
|---|---|
| `signal.rs:1333` | **生产分类路径**——证书进入 `pan_divs`，最终写入 `LevelState.pan_div`（`signal.rs:1124-1127`；`classifier/mod.rs:427-467, 2044-2128`） |
| `recursive_tower.rs:2093` | 非趋势盘整支线——转换为 `CandDeltaEvent{cand_delta:false, pan_div_diag:true}`（`:2095-2118`） |
| `recursive_tower.rs:2146` | 趋势分支内保留的布尔诊断调用，只写事件字段（`:2173-2193`） |
| `signal.rs:2578` / `:2607` | 测试（正例 / 反例） |

**未能判定**：`recursive_tower.rs:2146` 在异常重复中枢键下是否可达——缺少 `(end_index, zd, zg)` 全局唯一性不变量。

## 2. `pan_div_diag` 的读取者

| 位置 | 用途 |
|---|---|
| `recursive_tower.rs:2207` | 事件确定性排序键 |
| `strict_nest_check.rs:546, 656` | 每级诊断计数，`:1712-1720` 输出报告 |
| `recursive_tower.rs:2282, 2314` | 回归测试断言 |

**没有业务门读取它。** `N^δ` 装配只认 `cand_delta`：基例拒绝 `nest.rs:1135-1138`、攀升跳过 `:1198-1203`、批量基例过滤 `:1261-1269, 1289-1301`。

⟹ 就 `pan_div_diag` 这个**字段**而言，「纯诊断」的说法成立。

## 3. `Cand^δ` 之外的消费通道（关键）

盘整背驰另有一条独立生产线，走的是 `PanDivCert` / `LevelState.pan_div`，**不经 `pan_div_diag`**：

| 环节 | 位置 | 行为 |
|---|---|---|
| **进场** | `fill.rs:898-925` | `LevelState.pan_div` 经 Nest/XZD 门；通过后可路由为**反父方向 P9 `OpenShortDiff`**（`oscillation.rs:611-647, 681-725`） |
| **出场** | `oscillation.rs:591-609, 727-747` | 后续盘背若恢复母腿方向，关闭同 `parent + center + oscillation_id` 的精确 ShortDiff 子腿（**P7 `CloseShortDiff`**） |
| **风控** | `fill.rs:962-1030` | 标准 BSP / P1–P4 优先，可压制盘背订单；候选受 KΘ 容量、不得穿零反向加仓、仓位 clamp 约束。**盘背不定义也不修改 `RiskMode`** |
| **声部** | `fill.rs:880-897`；`backtest/pan_div.rs:51-59` | 读取真实 live parent 并维护独立 `OscillationBook` 子腿；**不进入**普通 `ActiveLeg/StepTrace` 声部生命周期（`fill.rs:940-1052`） |
| **统计诊断** | `econ_positive.rs:430-485, 4746-4775` | 形成 `PanDivConsolidation` RawSignal 与独立 μ̂ / PnL 桶；另有 census 与重放对拍 |

## 4. 但整条订单轨默认关闭

`center_oscillation.enabled = false`（`oscillation.rs:19-28`；`fill.rs:437-451`；`config.rs:369`）——**订单消费惰性关闭**。

⟹ 在默认配置下，盘整背驰确实**不产生任何交易行为**；但这是**开关关闭**的结果，不是「实现没有这条通道」。二者区别重大：前者可以打开，后者需要新建。

## 5. 对 map #250 的意义

1. **「96% 盘整事件被挡在链外 ⟹ 策略看不到盘整」这个推论要收窄**：链外不等于系统外。盘整背驰有独立通道（P9 开 / P7 平），只是默认关闭。
2. **但对 `N^δ` 区间套链而言，排除是彻底的**——`cand_delta=false` 在四处被过滤，无任何回旋。所以 [SPEC #258](https://github.com/xy7365527-lang/NewChanlun/issues/258) 要判的「`Cand^δ` 窄化」问题不受本核查影响，仍然成立。
3. **新增一个可裁项**：`center_oscillation` 开关默认关闭这件事本身是否该重议——它与「盘整背驰不入链」是两条独立的排除，此前被混为一谈。

## 6. 原始产物

codex 完整输出：`/tmp/codex-pan-div-consumers.md`（本次会话产物，未入库）
