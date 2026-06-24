# TC 瞬时 churn 频率不可约 — #170/R4 否定性结果（L3 否证 #164「频率可约」声明）

**工位**：swarm/fix-TC-churn（Task #7/R4，失败模式修复子DAG）
**日期**：2026-06-24 ｜ **引擎**：`RecTStream` = Face A（默认开启，dfe98a3e89）＋ #149/#154 instrumentation ＋ #170 触发源/否定线诊断（observation-only，bit-exact 已验证）
**隔离**：worktree `fix-TC-churn`（base=facea-impl-113 HEAD `dfe98a3e89`）
**认识论**：诊断 = **L3**（真实数据每笔，可证伪，3 标的）；不可约性结论 = **L0 定理**（574 已结算谱系的直接推论）

> **核心结论（否定性）**：#164 分类报告声明 TC「频率可约——1-bar collapse 信号携零走势，应被过滤（信号反转下一 bar 的不应交易）」。
> **L3 穷尽否证此声明的机制前提与可实现性**：TC = 574 确认滞后不可约在**开仓侧**的精确体现，
> **在不引入 look-ahead 的前提下无合法引擎修复**。本结果**修正 #164 §4 TC 可约性声明**，不实装任何 gate（no-patch）。

---

## 0. 修复任务的原始假设（#164 §4 TC 节）

> TC 瞬时 churn：对齐入场但开平在 ≤1 bar 内 collapse（hold≤1），covered_Δ ≈ floor。
> 缠论根因：确认滞后（574）collapse——买卖点确认窗口内**信号反转**，开平重合于相邻 bar。
> 可约性：per-trade 不可约（成交价口径）；**频率可约——1-bar collapse 信号携零走势，应被过滤（信号反转下一 bar 的不应交易）**。

R4 修复方向（任务 #7）：**1-bar collapse 信号过滤（确认滞后 collapse 的开仓抑制）**，区分可约（频率）vs 不可约（574 成交价口径下界）。

---

## 1. L3 实证链（3 标的：OKLO/BTC/CL，bit-exact 引擎）

### 1.1 TC 平仓触发源拆解（否证「信号反转 churn」机制前提）

instrument：每笔平仓记录触发源（0=买卖点反向平 g_pair / 1=否定线止损 pair_stop_loss_step / 2=NAV强平/finish）。

| 标的 | n_TC | reason 0（信号反转平） | reason 1（否定线止损） | reason 2（边界） |
|------|-----:|----------:|----------:|----------:|
| OKLO | 36 | **0 (0.0%)** | **36 (100%)** | 0 |
| BTC | 309 | **0 (0.0%)** | **309 (100%)** | 0 |
| CL | 425 | **0 (0.0%)** | **425 (100%)** | 0 |

**否证**：TC ≠「信号反转 churn」（reason 0 命中 0%）。TC = **100% 否定线止损 collapse**。
#164 假设的因果机制（"反向信号携零走势"）**在引擎中不存在**——没有一笔 TC 是反向买卖点平的。

### 1.2 否定线穿透深度（TC 是浅穿，非深破）

| 标的 | 穿透 median | 浅穿 <5bps | 深穿 >50bps |
|------|----------:|----------:|----------:|
| OKLO | 6.22bps | 38.9% | 8.3% |
| BTC | 4.69bps | 54.0% | 0.3% |
| CL | 2.05bps | 72.5% | 0.2% |

下一 bar 仅**噪声级微穿**否定线（中位 2-6bps）即触发全量止损 ⇒ 开仓时已无安全空间。

### 1.3 开仓安全垫为负（TC 开仓即在否定线之下）

安全垫 margin =（多腿）(entry_px − ZD)/entry_px。margin>0=开仓在 ZD 上方有空间；margin<0=开仓已破 ZD。

| 标的 | TC margin median | T2/T3 对照 margin median |
|------|----------:|----------:|
| OKLO | **−893bps** | +645bps |
| BTC | **−257bps** | +182bps |
| CL | **−191bps** | +151bps |

**TC 笔开仓价 100% 已在否定线 ZD 之下**（开仓 bar 盘中 low<ZD 100% 命中）。
这是 574 §1.1 的精确显形：买点确认（需走势完成后信息）滞后于价格，**确认时价格已穿越中枢下沿**。
时序一致性已核验：`view.zd[k]` 取本次重跑末中枢 ZD，`view.buy[bl]` 取本次重跑 fresh BSP，
开仓价 c=当前 close——同一重跑、同一 tree，非陈旧 ⇒ "开仓在 ZD 下" 是真实结构，**非时序 bug**。

### 1.4 频率不可约的决定性否证（无开仓时 gate 能分离 TC 与盈利腿）

固定全集 `margin<0 & hold=1`（开仓状态完全同质：100% 开仓 bar low<ZD），按盈亏分：

| 标的 | win 笔 | loss 笔 | win/loss 破ZD深度 median 差 |
|------|------:|------:|----------:|
| OKLO | 93 | 94 | 73.69bps |
| BTC | 630 | 735 | 19.46bps |
| CL | 725 | 972 | 5.92bps |

**同一开仓状态（margin<0）下，盈利腿与亏损腿数量相当**，所有开仓时可知特征（margin / 开仓 bar low vs ZD / 破ZD深度）**无判别力**。
margin<0 全集净 pnl 接近零（BTC −696 / CL +753 / OKLO +1746），**margin<0 不是亏损源**。

`margin<0 & win` 含大量真实盈利腿（BTC +16958 / CL +13718）⇒ 任何"拒开 margin<0"的 gate 同时杀等量盈利腿。

**唯一区分 win/loss 的是下一 bar 价格方向**（win=反弹后信号平 / loss=继续跌触止损）= **未来信息 = look-ahead = 574 §1.1 延异非法**。

---

## 2. 与 574 谱系的对接（不可约性 = 已结算定理推论）

574（买卖点=确认滞后形式化解，已结算）§1.1：**操作顶/底 = 预测未来 = 延异非法**；§四：确认滞后不可约 ⟺ 走势完成不可预测 ⟺ σ=h²³失败。

本结果是 574 在 **TC/开仓侧** 的新 L3 确证：
- TC = 买点在"价格已回调至否定线下"才确认 fire（确认滞后开仓侧）。
- 分离 TC 亏损笔需要"下一 bar 价格方向"= 走势完成后信息 = **574 §1.1 延异非法**。
- ∴「TC 频率可约」⟺「存在不依赖未来信息的提前判别」⟺「确认滞后可约」——**与 574 已结算的不可约性矛盾**。

按四分法：「TC 频率是否可约」= **定理**（574 推论，自动结算），非选择。#164 §4 的「频率可约」声明 = 对 574 的 **over-claim**（574 §下游推论："任何提前定位平空的方案 = over-claim = 与本号矛盾 = 语法不合法"）。

---

## 3. 为什么不实装任何 gate（no-patch）

候选 gate 与否决理由：

| 候选 | 机制 | 否决（L3/谱系依据） |
|------|------|------|
| 抑制反向平仓（min-hold） | hold≤1 时不响应反向信号 | TC 100% 是止损非反向信号（§1.1）⇒ 拦不到 TC；且抑制止损=放任结构破坏（027:25 否定线支配） |
| margin<0 拒开 | 开仓破 ZD 则不开 | 杀等量盈利腿（§1.4），净 pnl≈0；margin<0 非亏损源 |
| 深度阈值拒开 | 破 ZD >X bps 拒开 | win/loss 深度无判别（§1.4，median 差 6-74bps 同量级） |
| look-ahead 过滤 | 预知下一 bar 反转则不开 | 574 §1.1 延异非法（预测未来） |

**no-patch 第6条（妥协方案禁止）**：为达成任务 KPI「TC 计数下降」而实装杀盈利腿/越界 maker/look-ahead 的 gate = 把矛盾留到下游 = 语法不合法。诚实的产出 = 否定性结果。

---

## 结果包六要素

### 1. 结论
TC 瞬时 churn 的「频率可约」声明（#164 §4）被 L3 穷尽否证。TC = 100% 否定线止损 collapse（非信号反转 churn），开仓价 100% 已在否定线 ZD 之下（确认滞后开仓侧），**在不引入 look-ahead 的前提下无开仓时 gate 能分离 TC 亏损笔与等量盈利腿**。TC 频率与 per-trade **均不可约**（= 574 确认滞后不可约在 TC 层的精确体现）。**无引擎修复实装。**

### 2. 定义依据
- TC 定义（#164）：对齐入场 hold=xb−eb≤CHURN_HOLD_BARS(=1)，covered≈floor。
- 否定线 ZD（53.1/第17课区间套否定线）：跌破中枢下沿=结构破坏止损（`pair_stop_loss_step`，Face B `enable_uniform_sizing`）。
- 确认滞后不可约（574 已结算，L3 #69 八标的）：背驰需"完成之后信息"⇒ 顶/底不可操作 ⇒ 提前操作=延异非法。
- r≤0⟺pnl≤0（#164 §0 covered>0 代数事实）。

### 3. 边界条件（结论翻转）
- 若找到**不依赖未来信息**的开仓时判据，使其在 `margin<0 & hold=1` 全集上分离 win/loss（§1.4 当前所有可知特征 median 差 5-74bps 同量级，零判别）⇒ TC 频率转为可约。当前 3 标的 L3 全否证。
- 若 `CHURN_HOLD_BARS` 改大（如 ≤3）⇒ TC↔T2 边界移动，但触发源（止损）不变，不可约性不变。
- 若引擎否定线机制改变（删 `enable_uniform_sizing` ⇒ ZD=None ⇒ 无止损路径）⇒ TC 不再是止损 collapse，但 Face B/A 的杠杆穿仓保护（liq=0）同时失效（§384 注："删 geom_tower 恒仓必须同步接真否定线否则来源A 杠杆穿仓"）⇒ 非局部可改。

### 4. 下游推论
- (a) R4 不向修复子DAG 贡献引擎改动；R·review（#8）/H·audit（#9）审查对象 = 本否定性结果的 L3 证据链与 574 对接，非代码 diff。
- (b) **修正 #164 §4 TC 可约性表**：「频率可约」→「频率不可约（574 开仓侧，L3 否证）」。下游 C 结晶（#11）应纳入此修正。
- (c) 四可约靶子（#164 §6）实际为**三**：R1（T1 方向，可约）/R2（T3 leak，574 floor 之上可约）/R3（S1 段无腿，可约）；**R4（TC）与 T2/T0 同列为 574 floor 不可约**。
- (d) 与 574 §5.1 vs 539 张力同构：TC 笔的"covered≈floor 的 realized loss"是**最小确认滞后 >0 在止损侧的显形**（574 下界本身），不是引擎缺陷。

### 5. 谱系引用
- **574**（买卖点=确认滞后形式化解，已结算）：本结果 = 574 §1.1（提前操作=延异非法）+ §四（确认滞后不可约）在 TC/开仓侧的 L3 新确证。**核心依据。**
- **563**（吃跌穿仓×踏空二难）：TC = 同一确认滞后两端的止损侧显形。
- **539**（清仓 regime 不可约）：TC 净 pnl 的 regime 差（OKLO/CL 正、BTC 负）= 574§5.1 认识论下界(L0)⊥操作盈利性(L3 regime)。
- **#164**（失败模式穷尽分类）：本结果**修正**其 §4 TC「频率可约」声明（over-claim，与 574 矛盾）。
- **谱系不确定声明**：本结果是否应升格为独立谱系节点（"TC=574 开仓侧不可约确证 + #164 修正"），留 C 结晶（#11）判定；当前作为 574 的 L3 下游确证记录。

### 6. 影响声明
- **不改引擎决策**：仅新增 observation-only 诊断字段（`leg_close_reasons` / `leg_entry_stops`，平行于 #149 `leg_trades`），final_nav bit-exact（OKLO 220328.4808626345 = 参照引擎）。
- **新增诊断工具**：`analysis/tc_churn_diagnostic.py` / `tc_churn_reason.py` / `tc_churn_stoploss_probe.py` / `tc_churn_safety_margin.py` / `tc_churn_discriminant.py` / `tc_churn_exhaust.py`。
- **新增报告**：本文件。
- **修正**：#164 §4 TC 可约性声明（频率可约→不可约）。
- **依赖载体**：cherry-pick #149（4dc4ee3175）+ #154（77f8adf69c）instrumentation 到 base 以支持 leg_trades 逐笔账本。

---

## 复现

```bash
cd /Users/silencehan/Projects/NewChanlun/.claude/worktrees/fix-TC-churn
cd rust && ../.venv/bin/maturin develop --release && cd ..
.venv/bin/python analysis/tc_churn_reason.py OKLO BTC CL        # §1.1 触发源（100% 止损）
.venv/bin/python analysis/tc_churn_safety_margin.py OKLO BTC CL # §1.3 margin<0
.venv/bin/python analysis/tc_churn_exhaust.py OKLO BTC CL       # §1.4 频率不可约否证
```
