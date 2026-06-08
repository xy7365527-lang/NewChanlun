# M2 选股层 → M1 背驰定位器（E）端到端回测

工作周期（K4 regime）：60min 聚合期货，level-0 走势方向。
K4 时间线天数：758（2024-01-01 → 2026-06-04）。

## K4 regime / 极性分布（全时间线）

| regime | 天数 |
|--------|------|
| bull_equity | 352 |
| bull_commodity | 245 |
| neutral | 161 |

| 极性 S | 天数 |
|--------|------|
| -2 | 111 |
| -1 | 13 |
| +0 | 367 |
| +1 | 62 |
| +2 | 205 |

## 对比表

| 标的 | 版本 | 交易数 | 复利收益% | 胜率% | Sharpe | MaxDD% |
|------|------|--------|-----------|-------|--------|--------|
| QQQ | E（无选股，纯背驰） | 1 | -2.88 | 0.0 | 0.000 | -2.88 |
| QQQ | E+M2（K4选股驱动方向） | 0 | +0.00 | 0.0 | 0.000 | 0.00 |
| QQQ | Buy&Hold | — | -2.20 | — | — | — |
| OKLO | E（无选股，纯背驰） | 7 | +932.77 | 42.9 | 0.496 | -42.54 |
| OKLO | E+M2（K4选股驱动方向） | 6 | +128.92 | 16.7 | 0.327 | -43.23 |
| OKLO | Buy&Hold | — | +307.08 | — | — | — |

## 门控统计

| 标的 | bars | 起 | 止 | 交易日 | K4拦截日 | 掩码入场 |
|------|------|----|----|--------|----------|----------|
| QQQ | 21743 | 2026-03-02 | 2026-04-02 | 25 | 23 | 6 |
| OKLO | 447739 | 2024-05-10 | 2026-06-02 | 516 | 196 | 34 |

## 结果包六要素

**结论**：见上述对比表（E vs E+M2 vs Buy&Hold）。

**定义依据**：
- K4 配置 Γ=(σ(E/$),σ(Au/$),σ(Oil/$))，σ=RecursiveOrchestrator level-0 settled-move 方向（缠论走势方向，第31课盘整→FLAT）。
- 极性 S=polarity_index(Γ)；regime=regime_from_config（金/油相对方向→ω）。
- 方向表=selection_pool.apply_regime_adjustment（regime+S→置信度）；门控=long_allowed（LOW→拦截做多）。
- E=run_swing_trading(MODE_NONE)：底背驰买入、L2趋势顶背驰清仓，纯多头。

**边界条件**：
- QQQ 带时间戳 1min 仅约 1 个月（小样本，统计功效低）；OKLO 覆盖约 2 年。
- K4 工作周期=60min（日线在 move 层级全 FLAT，无法产生 regime）。
- 无前视：date X 仅用 date<X 的 regime（滞后一日）。
- σ_r 绑定为原油(CL)（用户 M2 规格），区别于 k4_scanner 的利率(TLT)。
- 门控仅作用于入场，不强制平仓；改 selection_pool 置信度逻辑会翻转结果。

**下游推论**：
- 若 E+M2 < E：确认门控在股票牛市削减暴露的既有否定性结论（project_divergence_gate_entry_exit_falsified / omega_regime_falsified）。
- 若 E+M2 > E：K4 金/油 regime 对股票方向有择时价值——但需 L3 多标的/多时段交叉验证才可声明。
- 管线本身贯通（各层对接无误）属 L1，与收益方向无关。

**谱系引用**：project_omega_regime_falsified、project_divergence_locator_entry_exit、project_divergence_gate_entry_exit_falsified、527号（config σ 走势方向态）。

**影响声明**：新建胶水层 src/newchan/strategy/k4_integration.py + 本回测脚本；不修改引擎/selection_pool/config_space。

**认识论等级**：管线连通 L1；收益对比 L2（单标的，可证伪；非 L3 交叉验证）。
