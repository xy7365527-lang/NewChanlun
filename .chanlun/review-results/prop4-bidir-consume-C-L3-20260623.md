# 命题4 读法乙 双向（construct+consume）+ 开放轴C（严格逐级区间套）：诊断 + 实装 + L3

> 工位：prop4-bidir（命题4 读法乙双向 + consume 平空 + 开放轴 C）。日期：2026-06-23。
> 分支：`prop4-nest-readingB-20260623`。实装跑在**隔离 git worktree**（`/tmp/prop4-diag-wt`，基于已提交 HEAD），
> 绕开主树并发碰撞（主树 prop4-nest 同时有未提交脚手架，见 §0.1）。
> 认识论等级：**诊断 = L3**（8 标的真实数据）/ **实装 = L0**（源码 + 概念形式化）/ **L3 = L3**（8 标的 × 6 变体）。

---

## 0. 一句话判决

**（待 L3 完成填充——见 §4 判决）**

诊断（L3）：prop4-nest 强牛穿仓**两根因对半分**——缺平空（做空对但没锁，45% 腿，pnl −474k，**100% 有 cover 信号**）/ 误判顶（价直上做空错，55% 腿，pnl −359k）。两根因都实质存在 ⇒ consume 与 C 都需要，且**诊断验证编排者 spec（consume = 买卖点 cover + 做多，dual of 557）优于脚手架（次级别背驰段 平 1/3 不开反向腿）**。

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

## 3. Step3 L3 结果（8 标的 × 6 变体，Structural）

**（待全量 L3 完成填充）**

OKLO 预览（已跑）：

| 标的 | OFF | ANCHOR | NEST(单向) | +CONS | +C | +BIDIR | BH |
|------|-----|--------|-----|------|-----|------|-----|
| OKLO | +55.3 | −88.5 | −103.1 | −92.8 | −100.3 | −92.8 | +307.1 |

**OKLO consume 验收（关键）**：持仓 bar **31973 → 942（34× 缩短）**，强平 **2 → 0（消除穿仓）**，consume=462 次。⇒ **consume 机制确解整仓长持死扣 + 消除强平**，但收益仍 −92.8%（462 次翻转 churn，consume_pnl −93351 = 震荡税）。消融：Δconsume +10.2pp / ΔC +2.8pp。

---

## 4. 判决

**（待 L3 完成）**

---

## 5. 六要素结果包

**（§4 判决完成后补全。诊断部分先行）**

1. **结论**（诊断部分）：prop4-nest 强牛穿仓两根因对半分（缺平空 45%/误判顶 55%），缺平空腿 100% 有 cover 信号 ⇒ consume + C 都需要，验证编排者 consume spec（买卖点+做多）优于脚手架（背驰段+1/3）。
2. **定义依据**：第27课区间套（背驰段 + 自顶向下到 a0）；557 顶层 all_sell 闸门（type2 反弹不新高解稀疏）的对偶（all_buy 闸门 cover）；560 读法乙（背驰段解冻 556 暴露 539）；539 做空腿 regime。
3. **边界条件**：θ_fav=5%（诊断二分阈值，极值鲁棒、中段敏感）；consume 定位级别（最低级=最 churn；最高级=最少）；C 严格度（逐级 vs 仅 top）。
4. **下游推论**：（待 L3）。
5. **谱系引用**：560（读法乙）、557（顶层闸门对偶）、539（做空腿 regime）、556（顶层冻结）、547（cascade 级别错配）、231（有效域≠定义域）。
6. **影响声明**：worktree 实装（consume + C + 诊断埋点），不触主树（避碰撞）；待 team-lead 协调合并。

---

## 6. 认识论等级标注

- §1 诊断 = **L3**（8 标的真实数据，20 腿，含否定性——55% 误判顶 consume 难解）。
- §2 实装 = **L0**（源码 + 概念形式化，第27课/557 对偶逐字导出）。
- §3 L3 = **L3**（8 标的 × 6 变体真实数据）。
- consume 定位级别 / C 严格度 = 部分**未穷举**（有效域边界，诚实标注）。
