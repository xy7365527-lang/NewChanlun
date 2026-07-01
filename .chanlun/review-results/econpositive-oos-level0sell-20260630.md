# 663 推论 OOS 验证：level0卖 train/holdout 切分 + 方向不对称（防 winner's curse）

**认识论等级**：L2（真实数据单标的单切分，可产否定性结果；非 L3 跨标的）。
**对象**：in-sample 最强正类 level0卖（level=0, δ=−1，in-sample actual_pnl=+3.47e4/349/36%）。
**复算**：`cargo test -p <crate> --release acc_level0sell_oos -- --ignored --nocapture`（确定性）。

## 数据 / 窗口
- BTC 全量 4613599 bar；**截断窗 [2017-08-17→2026-05-31]，bars=4613599**（最后 5000000 bar；OOM 边界=显式有效域，非全历史，同 l2_btc 纪律）。
- train_frac=0.6，切分 bar 索引=2768159（切分日 2022-11-27，半开区间 train=[0,2768159) / holdout=[2768159,4613599) 无重叠）。

## 窗净涨跌方向（首尾 close 总收益）
| 窗 | 净收益 | 方向 |
|---|---|---|
| 截断全窗 | +1628.85% | 涨 |
| train | +287.90% | 涨 |
| holdout | +345.57% | 涨 |
| 子窗1（前半） | +881.41% | 涨 |
| 子窗2（后半） | +76.00% | 涨 |

## 任务1：level0卖 train vs holdout（关键——正负决定 winner's curse 判定）
| 窗 | n | Σactual_pnl | n_act+/n（胜率） | Σab_rev |
|---|---|---|---|---|
| train | 2216 | 5.8651e4 | 785/2216 (35%) | 3.1304e5 |
| **holdout** | 1432 | **1.8363e4** | 522/1432 (36%) | 3.3883e5 |
| holdout 对照·买(δ+1) | 1688 | 1.2780e5 | 636/1688 (38%) | — |

**判定**：holdout level0卖 actual_pnl = 1.8363e4 > 0
→ **OOS 初步稳健**：holdout 卖仍正，+3.47e4 不是纯挑赢家产物。**但仅 BTC 单标的单切分 L2，非 L3**——holdout 窗净涨跌=+345.57%，若 holdout 仍跌段则方向效应未排除（见任务2）。

## 任务2：方向不对称（2 个不重叠子窗，结构性 vs 窗口效应）
| 子窗 | 净涨跌 | 卖(δ−1) actual_pnl | n卖 | 买(δ+1) actual_pnl | n买 | 卖>买? |
|---|---|---|---|---|---|---|
| 1（前半） | +881.41% | 1.0256e3 | 1814 | 1.6672e5 | 2053 | 否 |
| 2（后半） | +76.00% | 6.9442e4 | 1832 | 7.4050e4 | 2036 | 否 |

**判定**：卖优势在子窗间不一致（子窗1卖>买=false，子窗2=false）。
→ **窗口效应**：卖优势依赖特定子窗（很可能是下跌子窗做空），非结构性。否定性结果照实报。

## 结果包六要素
1. **结论**：holdout level0卖 actual_pnl=1.8363e4（OOS 初步稳健），方向不对称=窗口效应。
2. **定义依据**：level0卖=663 in-sample 最强正类（level=0/δ=−1/顶背驰卖空反转腿）；actual_pnl=δ(Pτout−Pτin)−Ce 真实成交口径（664-Q3）。
3. **边界条件**：holdout 窗净涨跌=+345.57%；若 holdout 为下跌段（做空天然赚），OOS 正不足以证 alpha（需跨涨跌子窗，见任务2）。train_frac=0.6 改变切分点结论可能翻转（单切分脆弱）。
4. **下游推论**：level0卖可作信号层 entry 候选，但须 L3 跨标的 + 涨段验证后才升基座（防方向效应）。
5. **谱系引用**：663 econpositive 推论；664 对象错配修复（δ=反转腿方向）；formalization-validity-domain（L2 有效域 < 定义域）；161（务实=把缺口留后面）。
6. **影响声明**：新增 Dataset::slice_bar_range（半开区间切片，复用 source_index 重置契约）+ acc_level0sell_oos 测试；不改 decompose_capturable_spread/TradeRecord/Order。
