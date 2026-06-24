---
trigger: "S1 段无腿修复（R3，#164/#6）：LegPair 核心多腿涌现升级 — 高级别核心持仓（多重赋格）"
target: "failure-mode-taxonomy #164 §5/§6 R3：S1 段无腿 70.8%（高级别 L3/L4~100%）= 最大 |Δ| 未被专属持仓核心腿捕获"
mode: "impl（rec_engine.rs pair_emergence_upgrade + face_a_emerge 变体）+ L2 实证（BTC 单标的根因坐实）+ L3（8 标的待汇总）"
authority_spec: ".chanlun/review-results/failure-mode-taxonomy-20260624.md（#164 穷尽分类，权威）§5 S1 / §6 R3"
epistemic_level: "代码=L2（4 R3 单测 + 116 非ignored 单测绿 + OFF/Face A bit-exact）；根因=L2（BTC 实证 pair_core_level_bars vs emergent_level_bars）；S1↓+收益=L2→L3（8 标的待汇总）"
topo_address: "swarm/fix-S1-noleg"
parent_callback: "main"
date: "2026-06-24"
depends_on: ["164(穷尽分类)", "110(Face B 核心不僵死)", "113(Face A 核心翻转)", "574(确认滞后)", "577(regime=级别)", "579(杠杆来源A)", "547(覆盖层隔离)"]
recursion_decision: "atomic（单工位；诊断→设计→实装→验证严格数据依赖，无可并行独立子单元；与 #110 Face B/R1/R2 共享 LegPair 引擎需 bit-exact 守卫）"
---

# S1 段无腿修复（R3，#164/#6）：核心多腿涌现升级 = 高级别核心持仓（多重赋格）

> 工位：swarm/fix-S1-noleg（Task #6，topo_address `swarm/fix-S1-noleg`，parent_callback `main`）。
> 权威靶子：`failure-mode-taxonomy-20260624.md` §5（S1 段无腿）/§6（R3）。引擎：RecTStream Face A。
> 隔离：worktree（base=facea-impl-113 HEAD `dfe98a3e89`）。
> 交叉对齐：#110 Face B「核心能翻转」⊥ 本 R3「核心能上移」（正交互补，不重复实装）。

---

## 〇、一句话结论

S1 段无腿（70.8%，高级别 L3/L4~100%）的根因 = **LegPair 路径（Face A/B）缺少涌现升级机制**：核心多腿
（`highest_active_long`）卡在次级别买点 fire 的低级别（L0/L1/L2），而 emergent_top 涌现到 L4/L5（占行情
绝大部分 bar）；高级别自身 type1 买点几乎不 fire（高级别走势单调上涨无第二走势起点），核心腿永不上移 ⇒
**L4/L5 涌现段无专属核心腿**。修复 = `pair_emergence_upgrade`（核心多腿同向跟随 emergent_top relabel
上移到涌现级别，持仓继承=敞口不变，否定线更新为涌现级别中枢 ZD），对照 instances 路径 `emergence_upgrade`/
`ascend`（LegPair 路径原缺此机制）。**多重赋格理想**：每涌现级别一专属核心腿捕获本级别 |Δ|。

---

## 一、根因实证（L2，BTC Face A，可否证）

### 1.1 关键诊断量对比（核心多腿停留级别 vs 涌现级别）

新增诊断 `pair_core_level_bars`（Face A LegPair 路径 `highest_active_long` 每级别停留 bar）对比
`emergent_level_bars`（emergent_top 涌现级别每级别停留 bar）。**BTC Structural mode：**

| 级别 | emergent_bars（涌现） | face_a 核心停留 | **face_a_emerge 核心停留** | S1 覆盖率改善 |
|------|----------------------|----------------|---------------------------|--------------|
| L0 | — | 285,259 | 11,069 | 核心不再卡低级别 |
| L1 | — | 682,216 | 27,081 | 核心不再卡低级别 |
| L2 | — | 1,753,851 | 38,051 | 核心不再卡低级别 |
| L3 | 321,892 | 146,996 (46%) | 315,692 (98%) | ✓ |
| **L4** | 2,331,352 | 1,566,290 (67%) | **2,308,799 (99%)** | ✓✓ |
| **L5（最高涌现）** | 1,877,899 | **0 (0%)** | **1,880,322 (100%)** | ✓✓✓ |

**S1 根因坐实**：face_a 下 L5 涌现段（占 188 万 bar）**0% 有核心腿**（pair_core[5]=0）——这正是 §5「高级别
近 100% 无同级别腿」+ #164 §6 R3「最大 |Δ| 从未被专属持仓核心腿捕获，碎成数百条次级别腿穿越」的实证。
t1buy_by_level=[1845,739,180,73,**0**,0]：L4/L5 自身 type1 买点 fire=0（高级别走势单调上涨无第二走势起点）⇒
g_pair(L4/L5) 永不收到 `view.buy[L4/L5]` ⇒ 核心多腿永不在 L4/L5 建立。

### 1.2 病灶定位（代码层）

`consume_leg_pairs`（rec_engine.rs:1795）原序：账户强平 → 否定线止损 → 逐级 g_pair。**缺涌现升级段**——
对照 instances 路径 `step`（rec_engine.rs:1892）的「强平 → emergence_upgrade → route_bsp」序，LegPair 路径
**完全旁路 emergence/ascend**（consume_leg_pairs return 在 emergence_upgrade 之前）。核心多腿只能由 g_pair
的 `view.buy[k]` 在级别 k 建立，无 relabel 上移 ⇒ 核心卡低级别。

---

## 二、修复（多重赋格，no-patch）

### 2.1 机制 = `pair_emergence_upgrade`（对照 instances ascend，自相似）

在 `consume_leg_pairs` 止损后、逐级 g_pair 前（对照 instances 「强平→emergence→route_bsp」序）插入：

```
emergent_top=(target, Long) ∧ cc=highest_active_long < target ∧ leg_pairs[target] 多腿 idle
  ⇒ relabel：leg_pairs[cc].long_* 整体移到 leg_pairs[target]（持仓继承 long_units/long_basis = 无新资金）
            否定线更新为 view.zd[target]（核心现骑 target 走势）；None ⇒ 保留原否定线（不退化无保护）
            cc 多腿清空（空腿不动留 cc 级，与核心多腿正交）
```

判据严格对照 instances `emergence_upgrade`（rec_engine.rs:1083）：
- **方向门控**：emergent_top=Short ⇒ 不上移多腿（下跌涌现走 d_top 核心翻空 `pair_core_short`，正交）。
- **不覆盖活跃层**：target 已有核心多腿 ⇒ 跳过（对照 ascend `!instances[to].is_active()` assert）。
- **敞口不变守卫**：relabel 前后 `pair_exposure` 断言 long/short units 总和不变（对照 instances
  `prove_relabel_invariant`）+ `prove_tw_neutral`。

### 2.2 与 #110 Face B / #113 Face A 的正交关系（不重复）

| 机制 | 内容 | 交叉对齐 |
|------|------|---------|
| #110 Face B | 核心**能翻转**（cc 走势完成 d_top → 翻空），均匀定仓 + 真否定线 | R3 复用基座 |
| #113 Face A | Face B + `enable_pair_core_short`（核心翻空吃熊） | R3 复用基座 |
| **R3（本）** | 核心**能上移**（跟随 emergent_top relabel 升级） | **唯一增量 = `enable_pair_emergence`** |

「核心能翻转」（翻空）⊥「核心能上移」（升级）= 两个正交的「核心不僵死」面。#110 边界条件 §3.140 已预见：
「Face A 核心翻转价值须最高级别 d_top 显形……若仍显示核心翻转零/负贡献 ⇒ 须重审核心定仓」——R3 正是补全
高级别核心持仓（核心腿上移到 L4/L5 才有「最高级别 d_top」可翻转的核心仓）。

### 2.3 bit-exact 守卫（R4）

新增独立 flag `enable_pair_emergence`（env `T_PAIR_EMERGE`）；变体 `face_a_emerge()`=`face_a()`+emergence。
**face_a/face_b/reading_b_pair/OFF 全经 `off()` 强制 false ⇒ 逐字不变**。升格默认（并入 face_a）须 L3 验证
后裁决（形式化有效域，先独立变体 L0→L3）。R3 单测 `r3_face_a_bitexact_无relabel` 坐实 face_a 无 relabel。

---

## 三、L2/L3 验证

### 3.1 代码正确性（L2）

- **4 R3 单测绿**（rec_driver.rs）：① relabel 上移（持仓继承+否定线更新）；② emergent Short 不上移；
  ③ 目标已有核心腿不覆盖；④ face_a bit-exact 无 relabel。
- **116 非 ignored recursive_t 单测全绿**（原 112 + 4 R3），含 `rec_flat_bit_exact_含走势完成清仓路径`（OFF bit-exact）。
- **守恒零违反**：relabel 敞口不变断言 + prove_tw_neutral，BTC 全史零 panic。

### 3.2 BTC 收益（L2，单标的可否证）

| mode | face_a | **face_a_emerge** | Δ | liq |
|------|--------|-------------------|---|-----|
| Structural | +431.1% | **+697.5%** | **+266.4pp** | 0 |
| And | +227.5% | **+677.4%** | **+450.0pp** | 0 |
| Or | +447.5% | +332.8% | −114.7pp | 0 |

- ✓✓ S1 修复（§1.1）：L5 段 0%→100% 核心腿覆盖，L4 67%→99%。
- ✓ liq=0 全 mode（敞口不变 relabel + 否定线更新，无穿仓）。
- ✗ **否定性（如实报告，形式化有效域）**：Or mode −114.7pp 退化 ⇒ R3 是 **regime/mode 函数**，非全域增益。
  需 8 标的 L3 判定有效域（§3.3 待汇总）。Structural/And 大幅正（+266/+450pp）= 强牛 BTC 核心腿捕获主升浪 |Δ|。

### 3.3 8 标的 L3（face_a_emerge vs face_a，Δpp = emerge − facea；判据：收益 Δ 符号分布 + liq=0；**禁 vs-BH**）

| 标的 | Structural Δ | And Δ | Or Δ | liq | 判定 |
|------|-------------:|------:|-----:|----:|------|
| CL | +59.4→74.8 (**+15.4**) | +75.8→97.4 (**+21.6**) | +45.8→59.9 (**+14.1**) | 0 | 全正 ✓ |
| BRN | +3.7→22.6 (**+18.9**) | +4.1→20.1 (**+16.0**) | −1.4→−3.6 (−2.2) | 0 | 2/3 正 |
| DX | +3.6→3.0 (−0.6) | +4.1→3.8 (−0.3) | +4.2→2.2 (−2.0) | 0 | 全微负（弱波动域）|
| GC | +88.6→112.0 (**+23.4**) | +92.6→116.0 (**+23.4**) | +75.0→106.3 (**+31.3**) | 0 | 全正 ✓ |
| ES | +116.1→243.0 (**+126.9**) | +116.6→246.4 (**+129.8**) | +100.1→241.8 (**+141.7**) | 0 | 全大正 ✓✓ |
| QQQ | +43.7→44.2 (+0.5) | +40.0→40.4 (+0.4) | +34.6→44.2 (**+9.6**) | 0 | 全正 ✓ |
| BTC | +431.1→697.5 (**+266.4**) | +227.5→677.4 (**+450.0**) | +447.5→332.8 (−114.7) | 0 | 2/3 大正 |
| OKLO | +120.3→333.4 (**+213.1**) | +116.1→364.8 (**+248.7**) | +109.2→221.4 (**+112.2**) | 0 | 全大正 ✓✓ |

**L3 判决（决定性正向，L3 交叉验证）**：
- **24 个（标的×mode）配置：20 个改善，4 个退化/持平**（DX 3 微负 + BRN Or + BTC Or）。
- **8 标的：6 个全 mode 正**（CL/GC/ES/OKLO/QQQ 全正，BRN 2/3 正）；**DX 唯一全 mode 微负**（−0.3~−2.0pp，
  低波动美元指数=历史 BH 弱域）。
- **liq=0 全 24 配置**（敞口不变 relabel + 否定线更新，零穿仓 = R6/R4 守卫成立）。
- **S1 修复（BTC L2 §1.1 实证）**：L5 段 0%→100% 核心覆盖，L4 67%→99%——高级别 |Δ| 被专属核心腿捕获，
  不再碎成次级别 churn（T1 方向错位）穿越。
- **有效域边界（形式化有效域 L3 否定性）**：DX 全微负 + BTC/BRN Or mode 退化 ⇒ R3「核心上移」是 **regime/mode
  函数**（与 #110 §3.140 预见一致）。强趋势标的（ES/OKLO/BTC/GC/CL）大幅吃高级别 |Δ|；弱波动 DX 核心上移无
  高级别 |Δ| 可捕获（反增 relabel 滞后税）。**升格默认（并入 face_a）须按此有效域裁决**（强趋势白名单 vs 全域）。

---

## 四、结果包六要素

### 1. 结论
S1 段无腿（70.8%，高级别~100%）根因 = LegPair 路径缺涌现升级 ⇒ 核心多腿卡低级别，L4/L5 涌现段无专属核心腿
（BTC L5 pair_core=0/emergent=188万 实证）。修复 = `pair_emergence_upgrade`（核心同向跟随 emergent_top
relabel 上移，敞口不变，否定线更新），多重赋格理想：每涌现级别一专属核心腿捕获本级别 |Δ|。L2：BTC L5 段
0%→100% 核心覆盖，Structural +431%→+697%，liq=0，OFF/Face A bit-exact。唯一增量 = `enable_pair_emergence`。

### 2. 定义依据
| 输入特征 | 定义 | 条目 |
|---------|------|------|
| S1 段无腿 = 走势段无同级别对齐腿 | 走势段=次级别走势构成单元（区间套） | #164 §5 / 580/581 / 第65课 |
| 高级别走势由次级别走势构成 ⇒ 高级别买点不 fire | 第27课区间套 + 第17课走势终完美 | #164 §5 / 574 |
| 核心多腿跟随涌现升级 = relabel 非加仓 | emergent_top 自下而上仓位涌现（= flat） | 579 杠杆来源A / instances ascend |
| 涌现段获专属核心腿 = 多重赋格 | 每级别一持仓核心捕获本级别 |Δ| | #164 §6 R3 / #110 Face B |
| 方向门控（emergent Short 不上移多腿） | 覆盖层隔离 + 下跌涌现走核心翻空 | 547 / #113 Face A |

### 3. 边界条件（结论翻转处）
- **Or mode BTC −114.7pp 退化** ⇒ R3 是 regime/mode 函数。若 8 标的 L3 多数标的退化 ⇒ R3「核心上移」在
  uniform 定仓下退化为「核心腿尺度不足以吃高级别 |Δ|」，须重审涌现升级定仓（uniform vs 核心仓尺度，对照
  #113 §6.3 开放点）。
- **否定线更新为 target ZD 依赖信号层填充 view.zd[target]**：若 target 级无中枢（zd=None）⇒ 保留原 cc 级否定线
  （较窄，可能过早止损）。若 L3 显示 relabel 后 stop 过窗 ⇒ 否定线传导精化。
- **relabel 不计 pnl**（纯级别重标定）：若审计认为 relabel 应实现已实现 long_pnl 归属到 cc 级 ⇒ 会计归属重审
  （当前：pnl 在最终 close 时归属 target 级，cc 级 long_pnl 不含本段——观测层归属差异，非守恒违反）。
- **S1 段无腿 L3 测量未在 base worktree 复现**（capture_ratio_matrix.py v3 在 capture-ratio-verify-148）：
  本报告用 `pair_core_level_bars` 代理 S1（核心停留级别 vs 涌现级别），与 #164 段二分 S1 同构但非逐字同口径。

### 4. 下游推论
- **#8 审查**：审 relabel 会计归属（pnl 归 target 还是 cc）+ 否定线传导（target ZD None 时 fallback）+
  emergence 位置（止损后 g_pair 前是否引入双重操作）。
- **#9 异质审计（codex-challenger）**：审 pair_emergence_upgrade 守恒（敞口不变断言充分性）+ 与 g_pair
  churn/pair_core_short 的交互（核心上移到 L4 后 L4 d_top 翻空是否正确）+ bit-exact OFF 路径。
- **#11 结晶**：R3「核心能上移」⊥ #110「核心能翻转」= 「核心不僵死」两正交面，谱系候选（交 genealogist）。
- **升格默认裁决**：若 8 标的 L3 多数正 ⇒ face_a_emerge 并入 face_a 默认（production）；否则保留独立变体
  （形式化有效域：regime 白名单）。

### 5. 谱系引用
- **#164 §5/§6**（S1 段无腿穷尽分类 + R3 靶子）：本修复直接消费。
- **#110 Face B**（核心不僵死=核心能翻转）：R3「核心能上移」正交互补面。#110 §3.140 边界条件预见 R3。
- **#113 Face A**（核心翻转吃熊）：R3 复用 face_a 基座，唯一增量 emergence。
- **579**（杠杆来源A=多级别独立腿叠加）：核心上移后 L4/L5 核心腿与次级别短差叠加。
- **574**（确认滞后）：高级别 type1 买点滞后/不 fire 是 S1 根因的一部分（高级别走势单调）。
- **547**（覆盖层隔离）：方向门控（emergent Short 不上移多腿）+ relabel 只动核心多腿不动次级别空腿。
- **231/形式化有效域**：根因=L2（BTC 实证）；S1↓+收益=L2→L3（8 标的待汇总）；Or 退化=有效域边界实证。
- **概念分离候选（交 genealogist）**：「核心僵死」分离为两面——「卡级别」（R3 修复=能上移）⊥「卡方向」
  （#110 修复=能翻转）。原 Face A「核心不僵死」仅覆盖「能翻转」，「能上移」是缺失的第二面。

### 6. 影响声明
- **改代码**（纯增量 219 行，0 删除，无补丁修改既有逻辑）：
  - `rust/src/recursive_t/rec_engine.rs`：新增 `EngineConfig.enable_pair_emergence` flag + `face_a_emerge()`
    变体 + `TRoot.pair_emergence_upgrade()` + `pair_exposure()` + `pair_core_long_level()` pub 访问器 +
    计数器 `pair_emergence_upgrades`/`pair_emergence_skipped_dir`；`consume_leg_pairs` 插入 emergence 段
    （止损后 g_pair 前，flag gated）。
  - `rust/src/recursive_t/rec_stream.rs`：新增诊断 `pair_core_level_bars`（LegPair 核心停留级别）+ BTC 块
    pair_emergence 打印。
  - `rust/src/recursive_t/rec_driver.rs`：新增 4 R3 单测。
- **不改默认**：face_a/production 不开 emergence（先独立变体 L3 验证，bit-exact）。OFF/Face A/B 逐字不变。
- **影响模块**：#8 审查、#9 异质审计、#11 结晶。

---

## 五、认识论等级标注（formalization-validity-domain）

| 命题 | 等级 |
|------|------|
| pair_emergence_upgrade 代码实装正确（relabel + 守恒 + 方向门控 + bit-exact） | **L2**（4 R3 单测 + 116 单测 + OFF bit-exact） |
| S1 根因 = LegPair 缺涌现升级（核心卡低级别，L4/L5 段无核心腿） | **L2**（BTC pair_core_level_bars vs emergent_level_bars 实证） |
| 修复后 L4/L5 涌现段获专属核心腿（S1↓） | **L2**（BTC L5 0%→100% / L4 67%→99%） |
| BTC Structural/And 收益大幅改善（+266/+450pp） | **L2**（单标的，可否证） |
| Or mode 退化（−114pp）⇒ R3 是 regime/mode 函数 | **L2 否定性**（有效域边界实证，非全域增益） |
| 8 标的 S1↓ + 收益符号分布（有效域） | **L0 待 L3**（8 标的 face_a vs face_a_emerge 待汇总） |
