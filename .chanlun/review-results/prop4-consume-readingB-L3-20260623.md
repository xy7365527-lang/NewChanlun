# 命题4 读法乙双向 consume平空 + 严格逐级区间套（开放轴C）L3（任务22）

> 工位：prop4-nest（任务22，编排者授权 C+平空洞察）。日期：2026-06-23。
> 分支：`prop4-nest-readingB-20260623`，commit `79257d6e46`。
> 前置：本文件接 `prop4-nest-readingB-L3-20260623.md`（NEST 基线：背驰段真解冻 556 但整仓长持死扣穿仓 7/8）。
> 认识论等级：**L0 实装 / L3 实证（8 标的 × 4 变体）**。
> ⚠️ 任务22 多 session 碰撞：另一 session 并发编辑 rec_engine.rs（MFE/MAE 诊断），已回退；本工位在碰撞窗口 commit 锁定。

---

## 0. 一句话判决

**次级别买点平空（consume侧，背驰段触发）解整仓长持死扣穿仓——部分成立（5/8）**：consume 触发时做空腿失血砍 92–99%，强牛穿仓 −100% → −60~+377%（GC −100→−5.6 / ES −100→+19.6 / OKLO −103→+377 / BRN −105→−60）；但 3/8（CL/QQQ/BTC）consume 不触发（核心入场级别过低无次级别可平）⇒ 无救。**严格逐级区间套（开放轴C）单独不救（NEST_STRICT ≈ NEST 全 8），其作用是「保核心存活让 consume 得以触发」——strict 与 consume 协同，单独皆无效。** 解穿仓 ≠ 解踏空：5/8 救穿仓后仍 <BH（仅 DX/OKLO 超 BH）。

---

## 1. 实装摘要（L0，file:line）

接 NEST 基线（背驰段闸门 + a0 区间套整仓翻转），加三件（任务22）：

### 1.1 consume平空（`rec_engine.rs:nest_consume_step`）
- 核心仓持有期间，**次级别（k<核心级别）反核心向背驰段定位** ⇒ 平核心仓配额 1/3（`quota`，缠论 T 算子 construct 对偶 consume 侧）。底背驰段（次级别底）⇒ 平空（核心 Short 减仓）；顶背驰段 ⇒ 平多（核心 Long 减仓）。**平空不开反向腿**（区分 sink）。
- 触发源 = 次级别**背驰段**（非走势完成）⇒ 解命题2「平空欠触发」（short-cover-diag 根因 = 走势完成触发稀疏；换背驰段触发频繁）。
- `nest_step` 重构：`nest_try_flip`（主翻转）+ 未翻转时 `nest_consume_step`（consume）。

### 1.2 严格逐级区间套（开放轴C，`nest_try_flip` strict gate）
- 主翻转定位点 `loc` 须 `loc..=top` **逐级背驰段一致**（`view.level_diverge[k] == Some(armed)` for all k）——非仅 top 武装 + a0 定位。这是 NEST 基线报告 §6 标注的「唯一未关闭吃跌路径」的实装。

### 1.3 多重赋格 per-level 背驰段（`LevelView.level_diverge[k]`）
- `rec_stream.rs`：每级别当前走势 `trend_diverging_segment` → 操作极性数组（顶背驰段→Short / 底背驰段→Long）。consume + strict 共用。

### 1.4 受控对照 + bit-exact
- `EngineConfig::{nest_strict, nest_consume, nest_consume_strict}()`；harness `prop4_consume_l3`（OFF/NEST/NEST_STRICT/NEST_CS_STRICT）。bit-exact OFF 守住（flag 门控，102 单测绿）。

---

## 2. L3 结果表（8 标的 × 4 变体，Structural）

### 2.1 strat% 矩阵（* = 超 BH）

| 标的 | BH | OFF | NEST | NEST_STRICT（开放轴C） | NEST_CS_STRICT（完整形态） | consume数 |
|------|-----|-----|------|------|------|------|
| CL   | +28.2 | +20.3 | −62.9 | −62.5 | −62.5 | 0 |
| BRN  | +87.4 | −26.0 | −105.2 | −105.2 | −60.1 | 146 |
| DX   | +4.1  | −0.7  | +6.3* | +10.5* | +7.8* | 166 |
| GC   | +257.3 | −22.7 | −100.0 | −100.0 | −5.6 | 229 |
| ES   | +594.3 | −2.3  | −100.2 | −100.2 | +19.6 | 308 |
| QQQ  | +174.6 | −8.6  | −100.0 | −100.0 | −100.0 | 0 |
| BTC  | +1380.4 | +26.0 | −100.0 | −100.0 | −100.0 | 0 |
| OKLO | +307.1 | +55.3 | −103.1 | −100.3 | +377.8* | 86 |

### 2.2 做空腿失血（short_leg_pnl）NEST → NEST_CS_STRICT

| 标的 | NEST | NEST_STRICT | NEST_CS_STRICT | 砍幅（consume 触发时） |
|------|------|------|------|------|
| CL   | +0 | +0 | +0 | —（consume=0） |
| BRN  | −87630 | −87630 | **−907** | **99%** |
| DX   | +0 | +0 | +155 | — |
| GC   | −115640 | −115640 | **−5113** | **96%** |
| ES   | −142833 | −138065 | **−2563** | **98%** |
| QQQ  | −100924 | −100924 | −100924 | 0%（consume=0） |
| BTC  | −109514 | −109514 | −109514 | 0%（consume=0） |
| OKLO | −241495 | −497091 | **−19076** | **92%** |

---

## 3. 判决

### 3.1 ★consume平空 解整仓长持死扣穿仓？——**部分成立（5/8，结构依赖）**

- **consume 触发（5/8：BRN/DX/GC/ES/OKLO）**：做空腿失血砍 **92–99%**，整仓长持死扣（NEST 基线 1–2 年死扣）被次级别背驰段定位**渐进平仓**打散 ⇒ 穿仓 −100~−105% → **−60.1 / +7.8 / −5.6 / +19.6 / +377.8%**。**L3 问题「次级别买点平空缩短持仓减强牛穿仓」对这 5/8 答 YES。**
- **consume 不触发（3/8：CL/QQQ/BTC，consume=0）**：无救，仍 −62.5/−100/−100%。根因 = 核心入场级别 `loc` 过低（无 k<loc 的次级别可平）/ 持仓期不与次级别反核心向背驰段对齐。**有效域边界**：consume 仅在核心入场于有次级别空间的级别时可平。

### 3.2 ★strict（开放轴C）单独不救——其作用是「保核心存活让 consume 触发」

- **NEST_STRICT ≈ NEST 全 8 标的**（CL −62.5≈−62.9 / BRN −105.2 同 / GC −100 同 / ES −100.2 同 / BTC −100 同 / OKLO −100.3≈−103.1 / DX +10.5≈+6.3）。**严格逐级区间套单独 = 仅翻转更少更精，仍整仓死扣到底 ⇒ 穿仓。** 我先前子集印象「strict 逆转 OKLO +377」**是错误归因**——OKLO NEST_STRICT 实为 −100.3%（穿仓），+377.8% 来自 **consume**（NEST_CS_STRICT，consume=86）。
- **strict + consume 协同（neither alone works）**：
  - strict 单独：核心存活但整仓持到底 → 穿仓。
  - consume 单独（无 strict，子集实测 OKLO consume=0）：核心在松散定位下早穿仓（liq），无核心可平 → consume 不触发 → 穿仓。
  - strict + consume：strict 精确定位让核心**存活更久**（不立即 liq）→ 持有期内次级别背驰段触发 consume **渐进平仓** → 救穿仓。

### 3.3 解穿仓 ≠ 解踏空（强牛仍 <BH）

- 救穿仓的 5/8 中，仅 **2/8 超 BH**（DX +7.8>+4.1 震荡 / OKLO +377.8>+307.1 小样本异常）。ES +19.6 ≪ BH +594 / GC −5.6 < BH +257 / BRN −60.1 < BH +87。
- 根因：consume 平空把整仓做空腿压成中性/小仓，**避免穿仓但不吃涨**——强牛中位置多为空/中性而非满多，故 <BH（踏空残留）。**consume 解的是 539 放大（穿仓），不是踏空（吃涨）。**

### 3.4 命题2 平空欠触发 = 解（背驰段触发）

- short-cover-diag 诊断「平空欠触发根 = 走势完成触发稀疏」⇒ 换次级别**背驰段**触发：consume 触发 86–308 次（触发的 5/8）vs OFF recover 欠触发。**背驰段触发既解 556 顶层冻结（NEST 基线报告）也解命题2 平空欠触发（本报告）——同一根因（完成触发稀疏）同一解（背驰段触发频繁）。**

---

## 4. 六要素结果包

1. **结论**：consume平空（次级别背驰段触发，strict 保核心存活）解整仓长持死扣穿仓 **5/8**（失血砍 92–99%，−100% → −60~+377%），3/8 consume=0 不救。strict（开放轴C）单独不救——协同 enabler。解穿仓 ≠ 解踏空（2/8 超 BH）。命题2 平空欠触发由背驰段触发解。

2. **定义依据**：第27课区间套逐级嵌套（开放轴C）；T 算子 construct 对偶 consume 侧（次级别平仓）；命题2 平空（short-cover-diag）；第37/24课背驰段（`trend_diverging_segment`）。

3. **边界条件（翻转处）**：
   - (a) consume 触发依赖核心入场级别有次级别空间——若强制核心入场于高级别（留次级别），CL/QQQ/BTC 或可触发 consume。**未测**（开放精化轴）。
   - (b) consume 配额 1/3 固定——若全平（cover-to-flat）或自适应配额，穿仓砍幅/踏空权衡可能变。**未测**。
   - (c) OKLO +377.8% 小样本（3243 重跑 5 翻转 86 consume），高方差，可能异常——不可单独作 robust 证据。
   - (d) BTC（极端牛 +1380%）consume=0 不救——最极端牛市是 consume 有效域外。

4. **下游推论**：
   - 开放轴C（严格逐级区间套）**单独不是吃跌解**（先前 NEST 基线报告 §6 标注的「唯一未关闭路径」**已测，单独关闭**）——但作为 consume 的 enabler 有结构价值。
   - 真正缩短死扣的是 **consume平空（次级别背驰段渐进平仓）**，不是 strict。
   - consume 解 539 放大（穿仓）但不解踏空——**吃跌的两个失败模式（穿仓/踏空）需不同机制**：consume 解穿仓，吃涨需底仓（ANCHOR）。**consume（整仓平向中性）与 ANCHOR（底仓死扣吃涨）在强牛对立**——这解释为何 consume 救穿仓后仍 <BH（无底仓吃涨）。

5. **谱系引用**：
   - **556**（顶层冻结）：背驰段触发解（NEST 基线报告）。
   - **命题2 平空欠触发**（short-cover-diag）：背驰段触发解（本报告，consume 86–308 次）。
   - **539**（做空腿失血）：consume 砍 92–99%（穿仓）但不解踏空。
   - **552 ANCHOR**：与 consume 对立（底仓吃涨 vs 平向中性）——强牛 ANCHOR>consume（GC ANCHOR+85 vs CS_STR−5.6）。
   - **558 pending**：本报告补充 NEST 基线——嵌套构成操作层 consume 侧部分解穿仓但不解踏空，操作层有效域 = 穿仓缓解（5/8），不及超 BH（2/8）。
   - **231/formalization-validity-domain**：consume 有效域 = 核心有次级别空间的标的（5/8）。
   - memory：`project_t_short_leg_regime_function`、`project_t_recover_full_vs_quota`、`project_deadlock_dual_open_target`。

6. **影响声明**：
   - 改动 `rec_engine.rs`（nest_consume_step/nest_try_flip/strict gate/EngineConfig 3 flag/level_diverge）+ `rec_stream.rs`（level_div 计算/prop4_consume_l3 harness）。
   - bit-exact OFF 守住（flag 门控，102 单测绿）。commit `79257d6e46`。
   - ⚠️ 多 session 碰撞：另一 session 并发编辑 rec_engine.rs（MFE/MAE），已回退；本工位碰撞窗口 commit 锁定。建议编排者裁定任务22 单一 owner。

---

## 5. 认识论等级

- §1 实装 = **L0**。
- §2 strat%/short_pnl 矩阵 = **L3**（8 标的，含否定性：3/8 consume=0 不救 + 5/8 救穿仓仍多数 <BH）。
- §3.1/3.2 consume vs strict 归因 = **L3**（4 变体消融，strict 单独 ≈ NEST 全 8 = strict 单独无效的否证鲁棒）。
- §3.3 解穿仓≠解踏空 = **L3**（救穿仓 5/8 仅 2/8 超 BH）。
- 异质审计：Codex/Gemini 今日双 429（NEST 基线报告 §7 同），本报告两断言（consume 触发的级别依赖性 / strict-consume 协同因果）**未经异质质询**，待配额恢复。

---

## 6. 建议谱系方向

- **开放轴C 单独关闭**：严格逐级区间套不是吃跌解（NEST_STRICT ≈ NEST）。NEST 基线报告 §6 的「唯一未关闭路径」修正为「strict 是 consume 的 enabler，非独立解」。
- **新发现**：consume平空（次级别背驰段渐进平仓）解整仓穿仓 5/8 + 解命题2 平空欠触发——但不解踏空（无底仓吃涨）。
- **吃跌两失败模式分离**：穿仓（consume 解）⊥ 踏空（需 ANCHOR 底仓）；二者机制对立（平向中性 vs 底仓死扣）。建议谱系：**嵌套构成 consume 侧解穿仓不解踏空，吃跌需同时 consume（防穿仓）+ ANCHOR（吃涨）但二者强牛对立 ⇒ 吃跌的不可调和性升一级**（穿仓×踏空二难）。depends_on 556/539/552/命题2/558。
- **开放精化轴（未测）**：核心强制入场高级别（留次级别给 consume）是否让 CL/QQQ/BTC 也触发 consume。
