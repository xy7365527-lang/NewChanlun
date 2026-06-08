# 双树法 K 线 PH 引擎 —— 对比单树法（close）+ MOS/OKLO 复跑

> 引擎：`src/newchan/a_dual_merge_tree.py`（T_high=OnlineMergeTree(-high) 追顶死亡史，T_low=OnlineMergeTree(low) 追底确认史）。
> 驱动：`analysis/dual_merge_tree_validation.py`｜输出 `analysis/data_cache/dual_merge_tree_compare.json`。
> 复跑：`cd <repo> && .venv/bin/python -u analysis/dual_merge_tree_validation.py`（**必须 `.venv/bin/python`**）。
> 单树对照：`a_settle_trigger.settle_triggers_from_prices(closes)`（喂 close，与历史 700 报告同口径）。
> 认识论：取负还原/方向翻转 **L0**；"low 阶梯比 close 更深 + 顶阶梯是信息增量" **L2**（真实数据可观测）；"笔候选=缠论笔"候选同构（需 TV 缠论指标 L2 对比，本次未连 TV，不编造）。

---

## 0. 一句话结论（先读）

**双树法相对单树（close）有两处实质信息增量：(1) T_low 喂 low 比 close 更深 → 底 settle 阶梯的 valley 更低、近端 gap 更小；(2) T_high 给出一整条"顶 settle 阶梯（high 跌破确认顶死）"——这是 close 单树结构上给不出的维度。三标的复跑：核心相位结论（MOS 下跌中 / 700 下跌低点 / OKLO 反弹中段）不被推翻，但被"顶/底 alive 比"这个双树独有的 regime 指纹进一步精化。**

---

## 1. 700 见顶后子窗口（n=157）：双树 vs 单树 ★核心对比★

| 维度 | 单树法（喂 close） | 双树法（喂 high/low） | 差异 |
|------|-------------------|----------------------|------|
| 近端底 settle | valley **439.0** → settle **441.4**（gap +14.2） | valley **432.0** → settle **433.6**（gap **+10.0**） | low 比 close **更深 7 点**，第一个向上确认**更近 4.2 点** |
| 这轮下跌全局底 | close 425.0（curP 252.5） | low **420.4**（reversal 249.6） | low 全局底比 close **更深 4.6 点** |
| 见顶定位 | close argmax **677.5** | high argmax **682.5** | 真实顶比 close 顶**高 5 点** |
| **顶 settle 阶梯** | **结构上给不出**（close-sublevel 看不见顶） | 当前反弹顶 **438.4**，high **跌破 432.8（还需跌 5.6）** 即确认这波反弹顶死；全局顶 682.5 不可跌破 settle（reversal 249.7） | **双树独有维度** |
| 顶/底 alive 分量 | 单树 close alive=14（仅底向） | 顶 alive=**2**，底 alive=**13** | 顶 alive 极少 = 仍是下跌结构 |
| 笔候选 | （单树无双向笔） | **58 条**（已确认 36） | 双树提供完整 zigzag |

**结论是否改变**：核心定位（**这轮下跌低点区 / 潜在一买候选区、反弹未启动、力度未确认**）**不被推翻**——顶 alive 仅 2、底主导腿仍 alive（reversal 249.6 未被否定）。但双树**精化**了三点：
1. **近端确认更近**：第一个向上确认从 close 口径 441.4（gap 14.2）修正为 low 口径 **433.6（gap 10.0）**——用 low 而非 close 度量"最近一段下跌腿"，反弹门槛更低。
2. **新增顶维度的实时风险**：当前这波小反弹的顶（438.4）**极脆弱**——high 只需回落 5.6 到 432.8 就确认这波反弹的顶死。这是 close 单树**给不出**的下行风险信号（监视：跌破 432.8 → 当前反弹结构破坏）。
3. **真实极值替代 close 极值**：整轮区间用真实 [420.4, 682.5] 替代 close 的 [425.0, 677.5]——幅度更大（下跌 -38.5% vs -36.9%）。

---

## 2. MOS（n=1255）：双树确认"仍在下跌途中"

| 项 | 双树读数 | 含义 |
|----|---------|------|
| 顶 alive / 底 alive | **2 / 25** | 顶几乎全灭、底持续堆积 = **强下跌结构** |
| 全局顶 | 79.28，reversal **57.59** | 大级别顶被否定需反向腿 persistence > 57.59（巨大，下跌远未结束） |
| 全局底 | low **20.89**，reversal 55.47 | 主导下跌腿 alive、不可反弹 settle |
| 近端顶 settle | 当前顶 24.5，跌破 **21.69**（还需跌 2.81）确认死 | 当前微反弹的顶随时可死 |
| 最近笔候选 | up bottom@1247=20.89 → top@1254=24.5（amp 3.61，**未确认**） | 当前一小段反弹未确认成笔 |

**结论不变**：MOS **仍在下跌途中**（与单树法 `MOS<700<OKLO` 相位排序一致）。双树新增确认：**顶结构几乎全灭（alive=2）**——这是单树 close 给不出的"下跌强度"直接读数。

---

## 3. OKLO（n=1229）：双树揭示"双边结构最活跃"（反弹中段）

| 项 | 双树读数 | 含义 |
|----|---------|------|
| 顶 alive / 底 alive | **19 / 15** | 三标的中**唯一顶 alive > 底 alive** = 双向结构活跃 |
| 单树 close alive | 14 | 双树揭示 **19 个存活顶**，顶结构远比 close 树丰富 |
| 近端顶 settle | 当前顶 70.978，跌破 **69.52**（还需跌 0.49）确认死 | 反弹中段顶部脆弱、临界 |
| 全局顶 / 全局底 | 顶 193.84（reversal 188.15）/ 底 5.35 | 超大区间，主导未否定 |
| 最近笔 | 双向交替密集 | 反弹中段震荡 |

**结论不变**：OKLO **反弹中段**。双树**精化**：OKLO 是三者中**顶/底最对称**（比值 1.27），印证"反弹中段、双向结构活跃"——这与 700（0.15）、MOS（0.08）形成清晰梯度。

---

## 4. 双树独有的 regime 指纹：顶/底 alive 比 ★新发现★

| 标的 | 顶 alive / 底 alive | 比值 | 相位（与单树一致） |
|------|--------------------|------|-------------------|
| MOS | 2 / 25 | **0.08** | 强下跌 |
| 700 | 2 / 13 | **0.15** | 下跌低点 |
| OKLO | 19 / 15 | **1.27** | 反弹中段 |

**"顶 alive / 底 alive" 比值随相位单调上升**（下跌→低点→反弹）。这是双树法**独有**的 regime 标量——单树 close 只有底向分量，**结构上无法构造**这个比值。比值 < 0.2 = 下跌主导（顶持续被杀）；比值 ≈ 1 = 双向均衡（反弹/区间）。

> 认识论：这是 **L2** 观测（3 标的真实数据可见的单调关系），**非 L3**——未做多时段/多标的交叉验证，不声称鲁棒。比值阈值（0.2 / 1.0）是 3 点启发，非统计标定。

---

## 5. 结果包六要素

1. **结论**：双树法（T_high=-high sublevel 追顶、T_low=low sublevel 追底）相对单树（close）有两处可观测信息增量：**(a)** T_low 喂 low 比 close 更深——700 子窗口近端底 valley 432.0（close 439.0）、settle 433.6（close 441.4，gap +10.0 vs +14.2）、全局底 420.4（close 425.0）；**(b)** T_high 给出 close 结构上给不出的**顶 settle 阶梯**（high 跌破确认顶死）——700 当前反弹顶 438.4 跌破 432.8（还需跌 5.6）即确认死。三标的复跑：MOS（顶/底 alive=2/25，下跌）、700（2/13，下跌低点）、OKLO（19/15，反弹中段）核心相位结论**不被推翻**，但新增"顶/底 alive 比"regime 指纹（0.08 < 0.15 < 1.27 随相位单调上升）。

2. **定义依据**：§10 因果 settle 判据（`a_online_persistence` L0 定理：合并配对因果确定 ⟺ 鞍点右侧出现 ≥ 屏障价）；本引擎在此之上做**确定性坐标变换**——T_high 喂 -high → -high 的 valley = high 的 peak = 顶，settle（-high 涨过屏障）= high 跌破屏障 = 顶被杀；persistence 取负不变 = 真实下跌幅度。T_low 喂 low，语义与单树 close 同向（反弹涨过确认底），但 low 比 close 深 → 底更低。笔候选 = 两棵树全部分量（settled+alive）极值的 alternation 折叠序列。输入满足：700/MOS/OKLO 真实日线 highs/lows（缓存），逐根 `update(high,low)`，`settle_triggers_high/low()` 在 finalize 前调。

3. **边界条件（结论翻转条件）**：
   - **顶/底 alive 比阈值**（0.2/1.0）是 3 标的启发，多标的可能落在区间内 → regime 判定模糊；需 L3 标定才能作硬阈值。
   - **settle 屏障可被未来改写**：alive 腿 settle 价是当前因果估计，未来创新极值会改写（§7.5 在线树代价）——700 跌破 420.4 → 底阶梯整体下移；high 创新高 → 顶全局上移。
   - **"笔候选 = 缠论笔" 未验证**：本次未连 TV-MCP，未与缠论指标逐笔对齐（llm-role-boundary：不编造缠论指标读数）；笔候选含未确认端点（start/end_confirmed 标记），过滤确认笔需 both confirmed。
   - **见顶定位敏感性**：700 子窗口用 close argmax(677.5@idx334)；若用 high argmax 起点/腿数可能微变（未扫描）。
   - **力度维度缺失**：双树仍是**幅度层**（H0 persistence ∈ ker(D)，239 号）——只给"创新极值/止跌确认"的必要条件，**不给 MACD 力度衰竭**（521 号：纯拓扑动量不存在）。700"潜在一买"仍不可升级为"确认一买"。

4. **下游推论**：(a) "这轮下跌是否结束"的监视器升级——除底阶梯（升破 433.6 起逐级确认）外，新增**顶阶梯下行风险**（700 跌破 432.8 → 当前反弹顶死，结构破坏）；(b) 顶/底 alive 比可作三标的 regime 横截面排序（替代/补充单树的 persistence 相位排序）；(c) 笔候选序列可喂给中枢/线段层（但需先与缠论指标 L2 对齐确认同构）；(d) TV 解锁后：用顶 settle 阶梯对齐缠论顶分型确认、底阶梯对齐底分型、笔候选对齐缠论笔——三重 L2 对比；(e) 双树是**幅度层**升级，力度判定仍归 MACD（pending-012/013 不变）。

5. **谱系引用**：§7.5（在线因果 merge tree alive/settled，本引擎只做坐标变换不改判据）、§10（因果 settle 判据 L0 定理）、§17.1/§17.3（形态学层 + 规则3 高级别 alive→假收敛，双树把它分裂为顶向/底向两条）、521 号（纯拓扑动量不存在 → 双树仍仅必要条件，一买充分性需 MACD）、239 号（H0≈幅度∈ker(D) → 双树是幅度层，不含力度）、231 号（有效域 L0-L3：取负还原 L0，"low 更深+顶阶梯增量" L2，"笔=缠论笔"候选未验证）。**未发生概念分离**（双树是 OnlineMergeTree 的组合用法，未改任何已结算定义；取负是数值变换，llm-role-boundary 的概率范式命名边界不适用）。本引擎是 `a_online_persistence`/`a_settle_trigger` 的**黑盒组合**，不修改其源码。

6. **影响声明**：新增 `src/newchan/a_dual_merge_tree.py`（DualMergeTree + DualComponent/DualSettleTrigger/StrokeCandidate/ContainmentPair）、`tests/test_dual_merge_tree.py`（15 个测试，全过——含取负还原证伪、顶跌破 settle 证伪、persistence 取负不变 property、T_low ≡ 单树引擎 property、笔交替/折叠、包含嵌套）、`analysis/dual_merge_tree_validation.py`（700 对比单树 + MOS/OKLO 复跑驱动）、`analysis/data_cache/dual_merge_tree_compare.json`（输出）、本报告。**未修改 `a_online_persistence.py`/`a_settle_trigger.py`**（黑盒调用）；**未引入新概念定义**（双树是组合用法 + 确定性坐标变换）；**未改任何已结算定义**。取负在输入施加、输出还原，用户看到的全部是真实价格。
