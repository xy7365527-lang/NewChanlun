# emergence ON vs OFF — reduce/recover 配对率诊断（认识论等级 L3：8 标的真实数据 A/B）

**数据**：`/tmp/t_off_v2`(T_NO_EMERGENCE=1) vs `/tmp/t_on_v2`(default)，同一份代码两次重跑，仅差消融门环境变量。
**脚本**：`analysis/t_engine_reduce_recover_pairing.py`、`analysis/run_emergence_ab.sh`。
**mode**：本判决用 `or`（用户关注的 CL +50%→+9% 即 or 模式）；structural/and 数据已同批生成可复查。

---

## 结论：用户假设被否证 —— 敞口坍缩的根因不是「recover 没及时归还」，而是「enter/flip 重建通道被 emergence 关闭」

用户假设：*涌现升级后低级别 BSP 变 sink（reduce 1/3），敞口降低是因为 reduce 后 recover 没及时归还。*

**数据否证「recover 归还不足」是主因**：emergence ON 的 sink/recover 配对率（次数+金额+缺口）多数 **优于** OFF，不是更差。

| 标的 | 版本 | sink/rec 次数配平 | 金额 升回/下放 | 缺口% | entries | flip 笔 | 毛敞口 | strat% |
|------|------|------------------|---------------|-------|---------|---------|--------|--------|
| CL   | OFF  | 87.4%            | 75.6%         | 24.4% | **25**  | **64**  | 419.2  | **+50.4** |
| CL   | ON   | 100.3%           | 84.7%         | 15.3% | **2**   | **1**   | 27.4   | **+7.9**  |
| OKLO | OFF  | 68.5%            | 58.9%         | 41.1% | 27      | 60      | 8440   | +90.3 |
| OKLO | ON   | 79.2%            | 73.8%         | 26.2% | 2       | 1       | 1123   | -19.3 |
| BRN  | OFF  | 106.3%           | 62.0%         | 38.0% | 29      | 66      | 508.9  | -122.0 |
| BRN  | ON   | 98.0%            | 10.7%         | 89.3% | 2       | 1       | 46.9   | -12.0 |
| ES   | OFF  | 90.9%            | 74.5%         | 25.5% | 26      | 52      | 16.8   | +2.8  |
| ES   | ON   | 120.1%           | 88.7%         | 11.3% | 5       | 13      | 18.2   | +28.1 |

注：CL ON 的金额配平率(84.7%)、缺口%(15.3%)、末端挂账(0) **全部优于** OFF(75.6%/24.4%/74.2)，但 strat 反而从 +50% 跌到 +8%。
⟹ 敞口坍缩 ≠ 配对失衡。

### 真因（决定性证据）：enter/flip 重建通道关闭

`enter` 用**全部 free** 建核心仓（`m=free/c`）；`flip`=`clear_all`+`enter`（全仓重建）。两者是核心仓「满血重建」的唯一入口。

emergence ON 的因果链（`t_engine.rs` step 阶段 A'）：
1. `emergence_upgrade` 在每次 BSP 路由**前**把核心仓 relabel 上移到涌现上界（L4→L7/L8，不等高级别 BSP）。
2. 核心升高后，`nearest_active_parent(j)` 对几乎所有低级别 BSP 都返回那个升上去的核心 ⟹ 它们从「核心级 enter/flip」退化为「子级 sink/recover/drain 短差」。
3. 高 ladder（L7/L8）的 BSP 极罕见（高级别走势稀疏）⟹ `flip` 几乎不再触发（CL/OKLO/BRN 从 60+ 笔 → 1 笔）。
4. 核心仓首次 `enter` 后**只能被 sink 单调侵蚀（每次 −1/3）**，`recover` 几何衰减只回输 1/3 的 1/3 ⟹ 净失血、无法满血重建 ⟹ 毛敞口坍缩 5–15×。

**比喻**：OFF 核心仓「反复满血复活」（flip 64 次全仓重建）→ 大敞口吃趋势；ON 核心仓「只能放血不能复活」（flip 1 次）→ 敞口萎缩到 1/15 → 吃不到 CL 油趋势 → +50%→+8%。

---

## 逐问回答

### 1. 每标的 reduce 次数 vs recover 次数配对率？
- **次数口径**（n_cycle_opens/closes）：ON 多数 ≥ OFF。CL 87.4%→100.3%，BTC 101.7%→100.4%，ES 90.9%→120.1%（recover 比 sink 还多）。
- **金额口径**（reduceΣu/recoverΣu）：ON/OFF 均 < 100%（几何衰减结构缺口）。CL ON 84.7% > OFF 75.6%。**唯一恶化是 BRN ON 仅 10.7%**（见第 4 问反常）。
- 结论：按配对率，ON **不比** OFF 差。配对率不是敞口坍缩的解释变量。

### 2. reduce→recover 间隔多少 bar（归还速度）？
recover trade 的 `(exit_bar−entry_bar)` p50，占总序列长度比：
| | CL | BTC | OKLO | ES | QQQ |
|--|----|----|------|----|----|
| OFF | 5.1% | 2.3% | 2.0% | 10.4% | 2.7% |
| ON  | 17.2% | 27.5% | 37.5% | 33.7% | 29.0% |

ON 归还**确实更慢**（p50 占比普涨 3–15×）。但这是「核心仓上移后短差挂账更久」的**伴随现象**，非主因——因为金额最终缺口(表4)ON 多数比 OFF 小。慢而仍归还，不是「不归还」。
（机制：`add_at` 加权 basis 不更新 entry_bar ⟹ 同一轮短差被反复部分 recover，entry_bar 锚最早下放点，gap 持续放大。）

### 3. 多少 reduce 没对应 recover（永久敞口损失）？
- **缺口%**（下放未经 recover 升回，被 drain/liq/flip 消解或末端挂账）：CL ON 15.3% < OFF 24.4%；OKLO ON 26.2% < OFF 41.1%。ON 缺口普遍**更小**。
- **末端挂账短差**：CL ON=0，OFF=74.2；OKLO ON=21.7，OFF=2893.9。ON 几乎无残留。
- 结论：**「1/3 永久没归还」在 ON 下并不成立**——ON 的下放量本身就小 10× 且归还更完整。永久敞口损失假设被否证。

### 4. CL 配对具体情况（+50%→+9%）
per-ladder（表5）：
- OFF：核心在 L4-L7 多次 `enter`（L4=11,L5=4,L6=6,L7=2），flip 64 次 → 反复全仓重建 → 毛敞口 419。
- ON：**仅 L3 `enter`=2，L4-L8 全为 0**；核心被 emergence 锁在 relabel 上移轨道，L4 sink=1856 次被持续下放，flip 仅 1 次。毛敞口 27（缩 15×）。
- 场景配平（表3b）：CL ON 父空场景 reduce,short→recover,long 配平 **100%**（6016→6016），父多场景仅 53%。CL 核心多空切换被 emergence 冻结在首仓方向附近。
- 直接损益（表3）：净敞口 OFF +9.7 → ON +2.3，多头敞口 214→15。CL 是上行 regime（bh+28%），ON 的多头敞口坍缩 14× → 吃不到趋势。

### 5. OFF 版有没有配对问题？
有，且**金额配平问题 OFF 更严重**（CL 75.6%、OKLO 58.9%、BRN 62.0% 都 < ON）。但 OFF 靠 `flip` 全仓重建（52-85 笔）持续补充敞口，**配对缺口被重建掩盖**——失血的同时满血复活。ON 去掉了重建，缺口才暴露为净敞口坍缩。
⟹ reduce/recover 几何衰减是 ON/OFF **共有**的结构属性；emergence 的净效应是去掉了对冲它的 enter/flip 重建。

### 关于 route_bsp/recover 触发条件「是不是有条件阻止了 recover」
读 `t_engine.rs:286-331` + `recover:256-271`：
- recover 触发 = 「同父向 BSP」∧「j 持反父向短差」∧「m=u_sub/3>1e-12」。反父向 BSP 走 sink/drain。
- **recover 的触发条件本身未被 emergence 改变**——ON/OFF 走同一 `route_bsp`。emergence 仅在 step 阶段 A'(`emergence_upgrade`)改变「谁是父级」，把更多 BSP 推入子级路由。
- 真正被 emergence「阻止」的不是 recover，是 **enter/flip**（核心 relabel 上移使低级别 BSP 不再命中 `nearest_active_parent==None` 的核心级分支）。
- 几何衰减（recover 每次仅平 1/3）是 ε 对称塔的**设计属性**，非 bug、非条件阻止。

---

## 结果包六要素

1. **结论**：用户假设（reduce 后 recover 没归还→永久敞口损失）被 L3 真实数据否证。敞口坍缩(毛敞口 5-15×)的根因是 `emergence_upgrade` 关闭了核心仓 enter/flip 全仓重建通道（entries↓10×、flip→~0），而非 sink/recover 配对失衡（ON 配对率多数优于 OFF）。

2. **定义依据**：
   - `enter`/`flip`=核心仓全 free 重建（`t_engine.rs:166-176,323-326`）；`sink`=父减 1/3 下放、`recover`=子平 1/3 升回（`accounting.rs reduce_at/add_at`，单 Layer 单一持仓加权 basis）。
   - emergence 语义：核心仓随 level 涌现 relabel 上移（第65课 Move(k)≡Level-(k+1) 笔，`t_engine.rs:203-234`）。
   - 输入数据特征满足定义：CL ON entries=2/flip=1（重建关闭）∧ sink=3190（子级 churn）∧ 毛敞口 27（坍缩）。

3. **边界条件（结论翻转条件）**：
   - 若某标的 emergence 不彻底压制 flip（如 **ES**：flip 仍 13 笔、毛敞口持平 18.2）→ 结论翻转为「emergence 通过去除过度做空腿改善收益」(+2.8→+28.1)。
   - 若 regime 为强反转/震荡（核心方向频繁错）→ 关闭重建反而救穿仓（BRN +110pp、BTC +58pp）。即 emergence = **regime 方差缩减器**，趋势 regime 杀赢家、穿仓 regime 救命。
   - 若涌现 level 触达 ladder≥MAX_LADDER 上界（当前数据 ≤L8 未触达）→ emergence_upgrade 静默跳过，退化为 OFF。

4. **下游推论**：
   - emergence ON 不是 alpha 引擎，是仓位规模压缩器（毛敞口统一缩 5-15×）⟹ 对 8 标的 P1(超 BH)从 OFF 的若干个降为 0（移除唯一超 BH 的 CL OR+50.4）。
   - 若要保留「自下而上涌现归属」又不杀趋势 alpha，需让 emergence 上移**不剥夺核心级重建权**——即 relabel 后仍允许更高/同级 BSP 触发 flip 全仓重建，而非把所有低级别 BSP 降为子级 churn。这是开放设计轴。

5. **谱系引用**：
   - [[project_t_emergence_upgrade_ab]]（本判决的 L3 数值精确复现：救穿仓{BRN+110/BTC+58/ES+25}杀赢家{OKLO−110/GC−35/CL−42/QQQ−13}；A/B=regime 方差缩减器非 alpha）。
   - [[project_t_short_close_level_mismatch]]（做空晚建早平、核心隔离）、[[project_t_operation_self_replication]]（ε 对称 sink/recover 几何塔=毛敞口几何衰减来源）。
   - 概念分离：本分析将「配对率(次数/金额/缺口)」与「重建通道(enter/flip)」分离——前者非解释变量、后者是。属新观测，未见既有谱系条目，建议结晶。

6. **影响声明**：新增 `analysis/t_engine_reduce_recover_pairing.py`、`analysis/run_emergence_ab.sh`、本报告。未改引擎代码（纯诊断）。`/tmp/t_off_v2`、`/tmp/t_on_v2` 为隔离 A/B 数据快照；`analysis/data_cache` 现存最后一次 ON run 覆盖值。
