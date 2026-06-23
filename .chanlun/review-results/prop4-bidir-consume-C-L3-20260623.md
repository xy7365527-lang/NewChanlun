# 命题4 读法乙 双向（construct+consume）+ 开放轴C（严格逐级区间套）：诊断 + 实装 + L3

> 工位：prop4-bidir（命题4 读法乙双向 + consume 平空 + 开放轴 C）。日期：2026-06-23。
> 分支：`prop4-nest-readingB-20260623`。实装跑在**隔离 git worktree**（`/tmp/prop4-diag-wt`，基于已提交 HEAD），
> 绕开主树并发碰撞（主树 prop4-nest 同时有未提交脚手架，见 §0.1）。
> 认识论等级：**诊断 = L3**（8 标的真实数据）/ **实装 = L0**（源码 + 概念形式化）/ **L3 = L3**（8 标的 × 6 变体）。

---

## 0. 一句话判决

**consume 平空（557 顶层闸门的全级别双向对偶：持仓遇反向 type1∨2∨3 买卖点 → full cover + 做多）普适解 560 的整仓长持死扣穿仓（8/8 强平→0，持仓 30–900× 缩短），并在 range/震荡 regime 产出项目史上最大吃跌 alpha（CL +1429%=50×BH、BRN +178.6%，P1 2/8）；但收益是 regime 函数（trend regime churn 中性/失血 ≪ BH，539 不可约），且 range alpha magnitude 是零摩擦伪影。开放轴 C（严格逐级区间套）收益层否证（误判顶穿仓不减 + consume on 时完全 inert，仅 DX +4.2pp）。诊断（L3，20 腿）：缺平空 45%（100% 有 cover 信号）/ 误判顶 55% 对半分——验证编排者 consume spec（买卖点 cover+做多）优于脚手架（背驰段+1/3）。命题4 双向吃跌：range 成立、trend 仍 539-bound。**

---

## 0.1 主树并发碰撞声明（诚实标注，feedback_task_queue_owner_liveness）

本工位（prop4-bidir）启动时发现主树 `rust/src/recursive_t/{rec_engine,rec_stream}.rs` 已有 **268 行未提交脚手架**实装任务22（`enable_nest_consume`/`enable_nest_strict`/`nest_try_flip`/`nest_consume_step`/`level_diverge`/harness `prop4_consume_l3`），且监控发现 rec_stream.rs 在我读取期间被另一 agent **实时编辑**（mtime 在纯 `sleep` 期间变化；无 rustfmt hook ⇒ 并发 agent）。存活同级 agent：prop4-nest、recursive-bsp。

**两点冲突**：
1. **双重分派**：疑似 prop4-nest 并发做同一任务 → 已上报 team-lead 裁决，**不在主树编辑避免互相覆盖**。
2. **consume 解读分歧**：脚手架 consume = 「次级别**背驰段** → 平核心仓 **1/3**，不开反向腿」；编排者给本工位的 spec = 「type1∨2∨3 **买点** → **整仓 cover** + **做多**（557 顶层 all_sell 闸门的对偶）」，且编排者明确警告「注意是 type1∨2∨3，不只底背驰段=type1」。脚手架恰是编排者警告反对的解读。

**处理**：本工位在**隔离 worktree** 按编排者 spec 从头实装 + 跑 L3（完全不碰主树碰撞区），落盘结果交 team-lead 协调合并。诊断（§1）跑在 worktree 的**已提交 prop4-nest 单向 baseline**（与主树脚手架无关，可独立信任）。

---

## 1. Step1 诊断（L3）：两种穿仓根因二分（缺平空 vs 误判顶）

对 prop4-nest（读法乙单向，已提交 baseline）每条做空腿，逐笔诊断做空后走势（埋点：实际建仓 bar + 持仓期
min/max close ⇒ MFE/MAE；type1∨2∨3 买点 cover 信号计数）。二分判据：

- **MFE%**（favorable）= (entry − min_c)/entry：做空后价**下跌**幅度（做空方向对的证据）。
- **MAE%**（adverse）= (max_c − entry)/entry：做空后价**续涨**幅度（误判顶的证据）。
- **n_cover** = 持仓期 [entry, exit] 内 type1∨2∨3 **买点**数（cover 信号是否存在 ⇒ consume 能否平空）。
- 二分：**MFE% ≥ 5% ⇒ 缺平空**（价跌过、做空对、若仍亏=没锁）；**MFE% < 5% ⇒ 误判顶**（价几乎只涨）。

### 1.1 汇总（8 标的，20 条做空腿）

| 标的 | strat% | 空腿n | 缺平空n | 误判顶n | 缺平空pnl | 误判顶pnl | 缺平空有cover信号 | 缺持bar | 误持bar |
|------|-----|-----|-----|-----|------|------|------|------|------|
| CL   | −62.9 | 4 | 1 | 3 | −26211 | −7405 | 1/1 | 1209894 | 10832 |
| BRN  | −105.2 | 2 | 1 | 1 | −19042 | −87630 | 1/1 | 135018 | 177132 |
| DX   | +6.3 | 1 | 1 | 0 | +4922 | 0 | 1/1 | 551240 | 0 |
| GC   | −100.0 | 3 | 1 | 2 | −115640 | −11714 | 1/1 | 1807951 | 494974 |
| ES   | −100.2 | 5 | 1 | 4 | +7237 | −151281 | 1/1 | 18972 | 618562 |
| QQQ  | −100.0 | 1 | 0 | 1 | 0 | −100924 | 0/0 | 0 | 405312 |
| BTC  | −100.0 | 1 | 1 | 0 | −109514 | 0 | 1/1 | 132614 | 0 |
| OKLO | −103.1 | 3 | 3 | 0 | −216051 | 0 | 3/3 | 31973 | 0 |

**全标的合计**：
- **缺平空（做空对但缺平空）**：9 腿（45%），pnl = **−474,299**，**其中 100%（9/9）持仓期内有 type1∨2∨3 cover 信号**。
- **误判顶（价续涨）**：11 腿（55%），pnl = **−358,954**。

### 1.2 六个强牛穿仓标的（term ≈ +100% = 强平）的根因对半分

| 标的 | 杀手腿 | MFE | MAE | term | pnl | cover信号 | 归类 |
|------|------|-----|-----|------|-----|------|------|
| OKLO | #2 | **+33.3%** | +101.3% | +101.3% | −241495 | 70 | **缺平空**（跌33%后反转上穿）|
| BTC  | #0 | **+36.0%** | +100.0% | +100.0% | −109514 | 121 | **缺平空**（跌36%后反转上穿）|
| GC   | #2 | +9.7% | +100.0% | +100.0% | −115640 | 2105 | **缺平空**（跌~10%后上穿，2105信号）|
| ES   | #4 | +0.2% | +100.2% | +100.2% | −142833 | 1530 | **误判顶**（几乎不跌直冲）|
| QQQ  | #0 | +2.0% | +100.0% | +100.0% | −100924 | 601 | **误判顶**（几乎不跌直冲）|
| BRN  | #1 | +1.6% | +106.3% | +106.3% | −87630 | 234 | **误判顶**（几乎不跌直冲）|

**3 缺平空（OKLO/BTC/GC）/ 3 误判顶（ES/QQQ/BRN）**——对半分。

### 1.3 诊断结论（定下游设计权重）

1. **两根因都实质存在（45/55 by 腿数，57/43 by pnl）**——neither dominates ⇒ **consume 与 C 都需要**，非单一解。
2. **缺平空腿 100% 有 cover 信号**（9/9）⇒ consume 平空（type1∨2∨3 买点）确有平仓机会 = **验证 consume 假设**。最清晰：OKLO（MFE+33/41%）、BTC（MFE+36%）——做空对了一大段，cover 信号充足，但整仓骑到反转 → 穿仓。
3. **误判顶腿（55%）价直上** ⇒ 单纯 cover（脚手架的平 1/3 中性）无效（剩 2/3 仍亏，无多腿吃涨）；需 consume 的「**+做多**」翻多骑涨 + **C 减少误判顶武装**（逐级嵌套校验，做空错的顶大多 top 背驰段未逐级嵌套坐实）。⇒ **诊断验证编排者 spec（cover + 做多 dual of 557）优于脚手架（背驰段 + 1/3）**。

**边界条件（诊断结论翻转）**：θ_fav=5% 阈值。极值（OKLO/BTC +33~36% 缺平空；ES/QQQ/BRN +0.2~2% 误判顶）对阈值鲁棒；GC（+9.7%）/CL #1（+11.4%）是中段，阈值升到 10% 会把 GC 杀手腿重归为「中段」。45/55 是 5% 阈值的中心估计。

---

## 2. Step2 实装摘要（L0，file:line 指 worktree，待合并主树）

按编排者 spec 实装**双向（construct + consume）+ 开放轴 C**（隔离 worktree `/tmp/prop4-diag-wt/rust/src/recursive_t/`）：

### 2.1 construct（保留，读法乙单向）
`rec_engine.rs:nest_step` §3：大级别背驰段武装（`view.top_diverge`）+ a0 区间套定位最低级 type1 → clear_all + enter armed 方向。与 prop4-nest 单向逐字一致（OFF/ANCHOR bit-exact）。

### 2.2 consume（新增核心，557 顶层 all_buy/all_sell 闸门的对偶）
`rec_engine.rs:nest_step` §2（`enable_nest_consume`）：**持仓中遇反向买卖点（type1∨2∨3 = `view.buy[k]`/`view.sell[k]`）⇒ full cover + 反向 enter**。
- 持空 → 找**买点** → cover 平空 + **做多**；持多 → 找**卖点** → close 平多 + **做空**。
- 定位最低级别买卖点（自相似：最高级别 + 递归次级别，最低级 = 最接近 a0）。
- consume 是仓位**管理层**（独立于武装窗口）；触发后置 `nest_consumed=true` ⇒ construct 不在本窗口立即翻回（两触发源不互斗）；新背驰段窗口（极性变/top 反转）重置 consumed ⇒ construct 可再发起。
- **区分脚手架**：用**买卖点**（非背驰段）、**整仓 cover**（非 1/3）、**翻反向做多/做空**（非中性）——诊断 §1.3 验证此 spec。
- 计数：`n_nest_consumes`、`nest_consume_pnl`。

### 2.3 开放轴 C（严格逐级区间套）
`rec_engine.rs:nest_step` §3（`enable_nest_strict`）+ `rec_stream.rs` `level_div` 计算：construct 定位点 `loc` 须 `loc..=top` **逐级背驰段方向一致**（`view.level_diverge[k] == Some(armed)`），非仅 top 武装 + a0 最低定位。第27课区间套字面（「先找背驰段，次级别图找相应背驰段，反复到最低级别」）的形式化。逐级嵌套不贯通 ⇒ 不翻（减少误判顶）。

### 2.4 受控实验 + 诊断埋点
- `EngineConfig::{off,anchor,nest,nest_consume,nest_strict,nest_bidir}()` 六变体单进程对照。
- `nest_trades` 扩 8 元组（+min_c/max_c 持仓期极值）+ 强平腿记入 nest_trades（穿仓腿 MFE/MAE）+ `all_buy_bars`/`all_sell_bars`（cover 信号计数）。诊断埋点纯观测，不改 OFF 行为。
- harness `prop4_pierce_diagnosis`（诊断）+ `prop4_bidir_l3`（L3 六变体）。

**bit-exact**：consume/strict 全 flag 门控（OFF/ANCHOR/NEST 单向走原路径）。OKLO OFF +55.3%/ANCHOR −88.5%/NEST −103.1% 逐字复现 prop4-nest baseline。

---

## 3. Step3 L3 结果（8 标的 × 6 变体，PerfectionMode::Structural）

### 3.1 strat% 矩阵（* = 超 BH）

| 标的 | OFF | ANCHOR | NEST(单向) | +CONS | +C | +BIDIR | BH |
|------|-----|--------|-----|------|-----|------|-----|
| CL   | +20.3 | −62.6 | −62.9 | **+1429.2*** | −62.5 | **+1429.2*** | +28.2 |
| BRN  | −26.0 | −89.8 | −105.2 | **+178.6*** | −105.2 | **+178.6*** | +87.4 |
| DX   | −0.7 | −12.3 | +6.3* | +0.2 | **+10.5*** | +0.2 | +4.1 |
| GC   | −22.7 | +85.2 | −100.0 | −5.9 | −100.0 | −5.9 | +257.3 |
| ES   | −2.3 | +301.9 | −100.2 | −28.8 | −100.2 | −28.8 | +594.3 |
| QQQ  | −8.6 | −54.2 | −100.0 | +16.2 | −100.0 | +16.2 | +174.6 |
| BTC  | +26.0 | +1080.2 | −100.0 | −99.3 | −100.0 | −99.3 | +1380.4 |
| OKLO | +55.3 | −88.5 | −103.1 | −92.8 | −100.3 | −92.8 | +307.1 |

**P1（超 BH）：OFF=0 / ANCHOR=0 / NEST=1{DX} / +CONS=2{CL,BRN} / +C=1{DX} / +BIDIR=2{CL,BRN}。**

> **bit-exact 守住**：OFF/ANCHOR/NEST 列逐字复现 prop4-nest baseline（560号）——CL OFF+20.3/ANCHOR BTC+1080.2/NEST 全列与
> `prop4-nest-readingB-L3-20260623.md` 完全一致。consume/strict 全 flag 门控。

### 3.2 consume 验收（NEST 单向 vs +CONS）：解整仓长持死扣穿仓

| 标的 | NEST% | CONS% | consume数 | NEST持bar | CONS持bar | NEST_liq | CONS_liq |
|------|-----|-----|-----|------|------|-----|-----|
| CL   | −62.9 | +1429.2 | 5447 | 310598 | 1016 | 1 | **0** |
| BRN  | −105.2 | +178.6 | 2528 | 156075 | 997 | 2 | **0** |
| DX   | +6.3 | +0.2 | 2144 | 551240 | 972 | 0 | 0 |
| GC   | −100.0 | −5.9 | 5676 | 932633 | 990 | 1 | **0** |
| ES   | −100.2 | −28.8 | 4739 | 498644 | 1240 | 1 | **0** |
| QQQ  | −100.0 | +16.2 | 1054 | 405312 | 709 | 1 | **0** |
| BTC  | −100.0 | −99.3 | 5052 | 132614 | 940 | 1 | **0** |
| OKLO | −103.1 | −92.8 | 462 | 31973 | 942 | 2 | **0** |

**consume 普适解穿仓**：持仓 bar **30–900× 缩短**（GC 932633→990），强平 **全 8 标的 1–2 → 0（消除穿仓）**。

### 3.3 消融（Δstrat% vs NEST 单向）：consume 贡献 vs C 贡献

| 标的 | Δconsume | ΔC | Δbidir | NEST flip |
|------|------|-----|------|------|
| CL   | **+1492.1** | +0.5 | +1492.1 | 9 |
| BRN  | **+283.8** | +0.0 | +283.8 | 4 |
| DX   | −6.1 | **+4.2** | −6.1 | 4 |
| GC   | +94.2 | +0.0 | +94.2 | 5 |
| ES   | +71.4 | +0.0 | +71.4 | 20 |
| QQQ  | +116.2 | +0.0 | +116.2 | 2 |
| BTC  | +0.7 | +0.0 | +0.7 | 5 |
| OKLO | +10.2 | +2.8 | +10.2 | 7 |

- **consume 贡献主导**（Δ +0.7~+1492pp，7/8 正）；**C 贡献 ≈0**（仅 DX +4.2，其余 0.0/+0.5/+2.8）。
- **+BIDIR ≡ +CONS 逐字**（8/8 完全相等）⇒ **consume on 时 C 完全 inert**：consume 高频翻转使 construct（strict-C 作用处）几乎不 fire（NEST flip 降到 1），strict-C 仅作用 construct 路径，故被 consume subsume。

---

## 4. 判决

### 4.1 consume 平空是否解 prop4-nest「整仓长持死扣」穿仓？——**YES（机制普适，决定性）**
全 8 标的：强平 **1–2 → 0**（消除穿仓），持仓 bar **30–900× 缩短**。560号「整仓做空腿长持 1–2 年死扣失血 −87k~−242k → 穿仓」被 consume **结构性消除**——做空形成下跌走势出现 type1∨2∨3 买点即 cover+做多，不再骑到反转穿仓。**编排者核心判据1 = YES。**

### 4.2 C（严格逐级区间套）是否减少误判顶？——**NO（收益层否证）**
- NEST_C 在 4 个误判顶主力标的（GC/ES/QQQ/BTC）**仍 −100%**（误判顶穿仓**不减**）——strict-C 没拦住误判顶的做空（逐级嵌套校验下，误判顶仍有级别贯通而武装）。
- C 仅改善 DX construct（NEST +6.3 → NEST_C +10.5，超 BH）；其余标的 ΔC ≈ 0。
- consume on 时 C 完全 inert（+BIDIR ≡ +CONS）。
- **C 的诊断承诺（减 55% 误判顶）在收益层未兑现**。误判顶的真解是 consume 的「+做多」翻多（cover 错空 + 骑涨），非 C 的更严武装。**编排者核心判据2 = NO。**

### 4.3 收益符号 / 超 BH / 保持解 556
- **consume 收益 = regime 函数**（与本项目全部吃跌探索同构，第 N 例）：
  - **range/油（CL/BRN）**：consume 巨幅 super-BH（CL **+1429%=50×BH**、BRN +178.6%=2×BH，项目史上最大 range alpha）。双向买卖点翻转捕捉震荡。
  - **mild（QQQ +16.2 / GC −5.9 / ES −28.8）**：解穿仓到近 breakeven，但 ≪ BH（趋势——ANCHOR/BH 吃涨胜）。
  - **强牛（BTC −99.3）**：双向 churn 在不停牛市失血（每个空腿亏、每个多腿太短未捕捉趋势），勉强解穿仓。
  - **低波（DX +6.3→+0.2）**：consume churn 抹掉 NEST 的小 edge。
- **保持解 556**：construct 背驰段闸门不变 ⇒ 556 顶层真解冻不变（consume 正交于 556，作用买卖点非背驰段）。

### 4.4 命题4 双向是否吃跌成立 or 仍 539-bound？——**range 成立，trend 仍 539-bound**
- consume 把单向的「整仓长持死扣穿仓」改形态为「双向买卖点 churn」：**range regime 是 alpha 来源**（捕捉震荡，CL/BRN 巨幅 super-BH）；**trend regime 是 opportunity cost**（churn 中性/失血，错过趋势，≪ ANCHOR/BH）。
- **539（做空腿 regime 失血）被 consume 改形态但未消除**：从「整仓穿仓」（560）变为「双向 churn 中性」（不再灾难，但 trend 下仍不盈利）。539 的 regime 依赖**不可约**——这是跨 8 标的 + 多读法的第 N 次确认。
- 与 560 对照：560 单向 = 解 556 暴露 539（穿仓）；本工位双向 = **解 556 + 解 539 的灾难形态（穿仓）+ 在 range regime 真吃跌（震荡 alpha），但 trend regime 仍 539-bound**。这是吃跌探索的实质推进（首次 consume 普适解穿仓 + range 巨幅 alpha），但**非普适 alpha**（P1 2/8）。

### 4.5 ★关键诚实 caveat（认识论降级）
1. **zero 摩擦伪影（CRITICAL）**：CL +1429% 来自 **5447 次整仓翻转 @ zero transaction cost**。real cost（CL/BRN 油期 ~1.5–3bps/round-trip × 数千 flip）会大幅侵蚀 range alpha 的 magnitude。**机制（解穿仓 + 震荡捕捉）real（L3），但 range alpha 的 magnitude 是 zero-cost-inflated**（同 [[project_cl_1s_a0_verdict]]/[[project_h4_amp_admission_verdict]] 的零摩擦伪影；cost/maker 建模是开放轴）。
2. **机制归因（consume ≠ 命题4 区间套）**：consume on 时 construct（背驰段 + a0 区间套 = 命题4 proper）近乎 vestigial（flip 降到 1，consume 462–5676）。**range alpha 来自 consume 的全级别买卖点震荡捕捉（557 顶层闸门族的全级别对偶），非命题4 区间套结构**。命题4 区间套退化为 entry-timing 层（近 inert）。诚实命名：本结果是「**双向 type1∨2∨3 买卖点 mean-reversion（557 闸门族）**」，不是「命题4 读法乙区间套吃跌」。
3. **consume 定位级别 = 关键未穷举轴**：consume 在**最低级**买卖点翻转（max 自相似 = max churn）⇒ range 捕捉最强但 trend churn 最重（BTC −99.3 = 多腿太短未骑趋势）。最高级（557 literal）会少 flip ⇒ trend 失血降但 range alpha 可能降。级别是 547/558 级别错配问题的复现，未穷举。

---

## 5. 六要素结果包

1. **结论**：命题4 读法乙**双向**（construct 背驰段闸门保留 + consume = type1∨2∨3 买卖点 full cover+做多，557 顶层闸门全级别对偶 + 开放轴C 严格逐级区间套）忠实实装（隔离 worktree，OFF/NEST bit-exact）。L3 8 标的：**consume 普适解整仓长持死扣穿仓（liq 全 →0，持仓 30–900× 缩短）**；收益 = regime 函数，P1 2/8{CL +1429%、BRN +178.6% 巨幅 super-BH（range/油）}，trend regime 解穿仓但 ≪ BH。C 收益层否证（误判顶不减，consume on 时 inert）。诊断（L3，20 腿）：缺平空 45%（100% 有 cover 信号）/ 误判顶 55%。
2. **定义依据**：第27课区间套（先找背驰段→次级别相应背驰段→反复到 a0）= construct + 开放轴C 逐级嵌套；557 顶层 all_sell 闸门（type1∨2∨3，「不是方向是买卖点」）的对偶 = consume（持空遇买点 cover+做多）；560 读法乙单向（背驰段解冻 556 暴露 539）；539（做空腿 regime 失血）；本工位输入数据满足：每条做空腿的 MFE/MAE（持仓期价格极值）+ type1∨2∨3 cover 信号计数 ⇒ 缺平空/误判顶二分。
3. **边界条件（结论翻转处）**：
   - (a) **consume 定位级别**：最低级（本实装，max churn）→ range alpha 最强 / trend churn 最重。改最高级（557 literal）→ trend 失血降但 range alpha 可能降。**级别决定 regime 画像**（547/558 级别错配复现，未穷举）。
   - (b) **transaction cost**：zero cost（本实装）→ CL +1429%；real cost（数千 flip × bps）→ range alpha magnitude 大幅降。**符号（解穿仓 + range 正 / trend 负）鲁棒于 cost；magnitude 不鲁棒**。
   - (c) **θ_fav=5%**（诊断二分阈值）：极值（OKLO/BTC +33~36% 缺平空 / ES/QQQ/BRN +0.2~2% 误判顶）鲁棒；GC/CL 中段（+9.7/+11.4%）阈值升 10% 会重归类。
   - (d) **C 严格度**（逐级 vs 仅 top）：本实装逐级 loc..=top；收益层 ≈ no-op（除 DX）。
4. **下游推论**：
   - **吃跌核心难点从「顶层冻结」彻底转移到「做空腿 regime + 平空触发」**：560 证「解 556 暴露 539」；本工位证「consume 解 539 灾难形态（穿仓）+ range 真吃跌」，但 trend regime 539 不可约。⇒ **吃跌 alpha 的有效域 = range/震荡 regime（白名单 {CL, BRN}）**，与 ANCHOR 的吃涨有效域（强牛 {GC/ES/BTC}）**正交互补**（consume 吃震荡 / ANCHOR 吃趋势）。
   - **consume（557 全级别对偶）= 双向 mean-reversion 引擎**：range 捕捉震荡 = alpha；trend churn 中性 = opportunity cost。这是独立于命题4 区间套的机制（construct 近 inert）。
   - **ANCHOR + consume 组合开放轴**（557 §四正交开放轴的实证延伸）：ANCHOR 底仓吃涨（强牛 super-BH）+ consume 机动吃震荡（range super-BH）——两 regime 互补，从未组合测。
5. **谱系引用**：
   - **560**（读法乙单向）：本工位双向是其直接延伸——单向解 556 暴露 539 穿仓 → 双向 consume 解穿仓 + range 吃跌。
   - **557**（顶层 all_sell 闸门 type1∨2∨3，「不是方向是买卖点」）：consume = 其**全级别双向对偶**（顶层 all_buy 闸门 + 自相似次级别）。557 收益 PARTIAL（无超 BH）→ 本工位 consume 全级别双向 2/8 super-BH（range）——557 闸门族在 range regime 真出 alpha。
   - **539**（做空腿 regime 失血）：被 consume 改形态（穿仓→churn 中性）但 regime 依赖不可约（trend 仍负）= 第 N 次确认。
   - **556**（顶层冻结）：construct 不变 ⇒ 真解冻保持。
   - **547**（cascade 级别错配）/ **558**（构成≠操作等价）：consume 定位级别问题是其复现（最低级 churn vs 最高级 trend-capture）。
   - **231/formalization-validity-domain**：consume 机制有效域（解穿仓，普适）⊋ alpha 有效域（range，2/8）= 第 N 例。
   - **552**（ANCHOR 强牛吃涨）：与 consume 吃震荡正交互补（组合开放轴）。
   - memory：`project_cl_1s_a0_verdict`/`project_h4_amp_admission_verdict`（零摩擦伪影）、`project_t_short_leg_regime_function`、`project_c_segment_fix_regime`（机制成立 + 收益 regime 同构）。
6. **影响声明**：隔离 worktree 实装（`EngineConfig` consume/strict 变体 + `nest_step` consume/strict-C + `LevelView.level_diverge` + 诊断埋点 nest_trades 8 元组/all_buy_bars + harness `prop4_pierce_diagnosis`/`prop4_bidir_l3`）；**不触主树**（避 prop4-nest 脚手架并发碰撞，§0.1）；待 team-lead 协调合并（主树脚手架的 consume=背驰段+1/3 应替换为本 spec=买卖点+做多，诊断 §1.3 + L3 §4 验证后者）；不改任何已结算定义。

---

## 6. 认识论等级标注

- §1 诊断 = **L3**（8 标的真实数据，20 腿，含否定性——55% 误判顶 consume 难单纯 cover 解）。
- §2 实装 = **L0**（源码 + 概念形式化，第27课区间套 / 557 顶层闸门对偶逐字导出）。
- §3 L3 / §4 判决 = **L3**（8 标的 × 6 变体真实数据，含否定性——C 否证、trend regime 不盈利、P1 2/8）。
- §4.5 caveat：zero-cost magnitude = **L3 但零摩擦**（符号鲁棒、magnitude 待 cost 建模）；consume 定位级别 = **未穷举**（有效域边界，诚实标注）。

---

## 7. 建议谱系号方向

- **谱系 561**（命题4 读法乙双向 consume——解穿仓普适 + range 吃跌 alpha / trend 仍 539-bound）：
  - 结算方向：**consume（557 顶层闸门的全级别双向对偶：持仓遇反向 type1∨2∨3 买卖点 → full cover + 反向）普适解 560 的整仓长持死扣穿仓（liq 全→0），并在 range/震荡 regime 产出项目史上最大吃跌 alpha（CL +1429%=50×BH、BRN +178.6%）；但收益是 regime 函数（trend regime churn 中性/失血 ≪ BH，539 不可约），且 range alpha magnitude 是零摩擦伪影。C（严格逐级区间套）收益层否证（误判顶不减 + consume on 时 inert）。**
  - depends_on：560（读法乙单向）、557（顶层闸门，consume 是其全级别对偶）、539（做空腿 regime）。
  - related：556、547、558、231、552（ANCHOR 正交）。
- **开放轴**（未关闭）：(1) consume 定位级别（最低 vs 最高 = range/trend 画像调节）；(2) transaction cost / maker 建模（range alpha magnitude 真实性）；(3) ANCHOR + consume 组合（吃涨 ∥ 吃震荡正交，从未组合测）。
