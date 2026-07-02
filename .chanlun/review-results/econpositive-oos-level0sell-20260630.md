# 663 推论 OOS 验证：level0卖 train/holdout 切分 + 方向不对称（防 winner's curse）

**认识论等级**：L2（真实数据单标的单切分，可产否定性结果；非 L3 跨标的）。
**对象**：in-sample 最强正类 level0卖（level=0, δ=−1，in-sample actual_pnl=+3.47e4/349/36%）。
**复算**：`cargo test -p <crate> --release acc_level0sell_oos -- --ignored --nocapture`（确定性）。

## 数据 / 窗口
- BTC 全量 4613599 bar；**截断窗 [2025-11-04→2026-05-31]，bars=300000**（最后 300000 bar；OOM 边界=显式有效域，非全历史，同 l2_btc 纪律）。
- train_frac=0.6，切分 bar 索引=180000（切分日 2026-03-09，半开区间 train=[0,180000) / holdout=[180000,300000) 无重叠）。

## 窗净涨跌方向（首尾 close 总收益）
| 窗 | 净收益 | 方向 |
|---|---|---|
| 截断全窗 | -28.78% | 跌 |
| train | -33.43% | 跌 |
| holdout | +6.98% | 涨 |
| 子窗1（前半） | -34.35% | 跌 |
| 子窗2（后半） | +8.51% | 涨 |

## 任务1：level0卖 train vs holdout（关键——正负决定 winner's curse 判定）
| 窗 | n | Σactual_pnl | n_act+/n（胜率） | Σab_rev |
|---|---|---|---|---|
| train | 219 | 4.7671e4 | 80/219 (37%) | 1.0546e5 |
| **holdout** | 128 | **9.0023e3** | 46/128 (36%) | 3.3310e4 |
| holdout 对照·买(δ+1) | 164 | -5.7072e4 | 46/164 (28%) | — |

**判定**：holdout level0卖 actual_pnl = 9.0023e3 > 0
→ **OOS 初步稳健**：holdout 卖仍正，+3.47e4 不是纯挑赢家产物。**但仅 BTC 单标的单切分 L2，非 L3**——holdout 窗净涨跌=+6.98%，若 holdout 仍跌段则方向效应未排除（见任务2）。

## 任务2：方向不对称（2 个不重叠子窗，结构性 vs 窗口效应）
| 子窗 | 净涨跌 | 卖(δ−1) actual_pnl | n卖 | 买(δ+1) actual_pnl | n买 | 卖>买? |
|---|---|---|---|---|---|---|
| 1（前半） | -34.35% | 3.9538e4 | 177 | -6.0105e4 | 163 | 是 |
| 2（后半） | +8.51% | 1.8076e4 | 168 | -5.3798e4 | 198 | 是 |

**判定**：两个不重叠子窗 level0卖均优于买。
→ **结构性（跨涨跌都卖优）**：子窗1/子窗2 净涨跌符号相反（一涨一跌）卖仍都优 ⟹ 卖优势非单纯下跌段做空产物，是结构性方向不对称。L2 单标的。

## 结果包六要素
1. **结论**：holdout level0卖 actual_pnl=9.0023e3（OOS 初步稳健），方向不对称=结构性候选。
2. **定义依据**：level0卖=663 in-sample 最强正类（level=0/δ=−1/顶背驰卖空反转腿）；actual_pnl=δ(Pτout−Pτin)−Ce 真实成交口径（664-Q3）。
3. **边界条件**：holdout 窗净涨跌=+6.98%；若 holdout 为下跌段（做空天然赚），OOS 正不足以证 alpha（需跨涨跌子窗，见任务2）。train_frac=0.6 改变切分点结论可能翻转（单切分脆弱）。
4. **下游推论**：level0卖可作信号层 entry 候选，但须 L3 跨标的 + 涨段验证后才升基座（防方向效应）。
5. **谱系引用**：663 econpositive 推论；664 对象错配修复（δ=反转腿方向）；formalization-validity-domain（L2 有效域 < 定义域）；161（务实=把缺口留后面）。
6. **影响声明**：新增 Dataset::slice_bar_range（半开区间切片，复用 source_index 重置契约）+ acc_level0sell_oos 测试；不改 decompose_capturable_spread/TradeRecord/Order。
