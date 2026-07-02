# Codex 异质对审：P2 方案A（#31 工位产出）

> 工位：#31 [p2-plan-review]（codex-challenger，异质代理）。2026-07-01。
> 模式：review（代码审查 + 语义分叉裁定）。
> 对象：`.chanlun/review-results/p2-plan-20260701.md`（#28 P2-plan 产出）。
> 权威依据：`/Users/silencehan/Downloads/目前的缺口.pdf` §6 方案A + Rust 代码逐行。

---

## Codex 原始输出（逐字）

```
受限说明：当前 workspace 是空目录，codebase-memory-mcp 调用被取消；以下审查基于你给出的逐行片段和因果链，不能声称已实际打开仓库逐行核验。

- 问题：candidate_dir 回退条件如果用 root_sel(...) == Flat 判断，会误把"多空 bit 同时触发导致
  Flat/冲突"的旧候选交给 struct_break_dir 改向。
  位置：interp.rs:206-211
  严重性：致命
  修复建议：回退必须写成严格零 bit：!bits.conf_plus() && !bits.conf_minus() && struct_break_dir.is_some()
  不能用 root_sel 的结果代表"无六 bit 方向"。

- 问题：R2 不碰 class_index() 的推导成立，但只在 struct_break_dir 完全不进入 BspBits、MuClass、
  class_index()、bucket key 的前提下成立。
  位置：types.rs:229、mu_estimator.rs:241
  严重性：建议
  修复建议：新增测试断言同一输入下六 bit 和 class_index() 完全不变；另测零 bit 破中枢候选只改变
  Candidate.dir，不改变 bits.class_index()。

- 问题：Weak_Θ 若被宣称为第34课 canonical 背驰定义，则冻结 buy1 为纯 MACD 面积不是定义忠实实现；
  它只是"legacy MACD-area 第一类买卖点 bit"。
  位置：signal.rs:309-318、bsp.rs:57,83
  严重性：重要
  修复建议：二选一：要么明确文档化 buy1/sell1 是 bit-exact 冻结的 MACD-area proxy，完整 Weak_Θ
  只在 selector 层裁决；要么承认若 buy1 必须代表完整 canonical 背驰，就与"不动 μ-key"硬约束真冲突。
  裁定：工程上 R2 可接受，但语义上必须降格命名/注释，不能继续说 buy1 等于完整第34课背驰。

- 问题：GOLDEN 翻转不是唯一诚实方案，但在"当前 FNV 对 {pts:?}"语义下，新增 Debug 字段必然翻转。
  位置：signal.rs:1660 附近、signal.rs:1673
  严重性：建议
  修复建议：诚实方案 A：更新 GOLDEN，并说明因 Debug 结构扩展。诚实方案 B：把测试改名为 legacy
  canonical digest，显式只序列化旧字段，再新增 struct_break_dir 单独 golden/断言。禁止通过自定义
  Debug 隐藏字段来假装未变。

- 问题：改点清单遗漏风险集中在多入口和组装路径。
  位置：interp.rs:308、extract_signals_with_hist_features、extract_signals_with_hist
  严重性：重要
  修复建议：assemble_gamma_with_tower 必须同步走同一个方向解析函数；两个 hist 入口若构造/返回
  BspPoint，也必须保留 struct_break_dir。否则生产主路径修好了，hist/tower 路径仍会把零 bit 破中枢
  候选判成 Flat。

- 问题：R2 只能消除 C<A MACD 门造成的选择偏差，不能宣称 Γ_struct 全量进样本。
  位置：signal.rs:293-300
  严重性：重要
  修复建议：声明收窄为"几何门通过且 A/C 可配对的破中枢候选，不再因 MACD C≥A 被预删"。如果
  Γ_struct 包含无 A/C 配对或未过几何门的结构破位，R2 仍过滤它们。

StructBreakFeature 处置：删除比接通 .1 更小、更干净，但前提是 Candidate 的 force features 在所有
Candidate 构造路径都有真实填充和测试覆盖；否则只是把死 sidecar 换成新死字段。

---stance-declaration---
verdict: conditional
stances:
  weak_theta_layering: needs_work
  r2_class_index_safety: accept_with_guard
  candidate_dir_fallback: reject_if_root_sel_flat_guard
  golden_digest_handling: needs_explicit_test_semantics
  struct_break_feature_removal: accept_if_candidate_features_are_populated
  missing_entrypoints: needs_work
  selection_bias_claim: needs_scope_narrowing
---end-stance---
```

---

## 审计代理判定（异质质询，不背书）

### 质询1：candidate_dir 回退误触发否定 — **成立（有效否定）**

**Codex 否定**：方案 §2 描述「六bit方向为 Flat 且 struct_break_dir=Some(s) ⟹ dir=s」，
若用 `root_sel(bits) == Flat` 作条件，会误把 buy1=true,sell1=true 的双侧候选（root_sel(1,1)=Flat，
见 voice.rs:270）走到 struct_break_dir 回退，改变其方向。

**核验**（voice.rs:265-271 逐行）：
```rust
pub fn root_sel(cands: RootCandidates) -> VoiceSide {
    match (cands.long_trigger, cands.short_trigger) {
        (true, false) => VoiceSide::Long,
        (false, true) => VoiceSide::Short,
        (false, false) => VoiceSide::Flat, // 真零bit
        (true, true) => VoiceSide::Flat,   // 双侧冲突，root_sel 也返回 Flat
    }
}
```

(1,1) 情形即 buy1=true AND sell1=true，interp.rs:1045 的测试已明确标注「(1,1) → root_sel=Flat」。
如果回退条件写成 `if candidate_dir(bits) == Flat && struct_break_dir.is_some()`，就会把
(buy1=1,sell1=1) 这类有六 bit 方向但冲突的候选也交给 struct_break_dir 改向——这确实是错误。

**正确回退条件**应是 Codex 建议的严格零 bit 检查：
`!bits.conf_plus() && !bits.conf_minus() && struct_break_dir.is_some()`

（conf_plus=false ⟺ buy1=buy2=buy3=false；conf_minus=false ⟺ sell1=sell2=sell3=false）

**判定**：否定成立。方案 §2 的「六bit方向为 Flat」表述是模糊的——Flat 有两种来源（零bit vs 双侧冲突），
需精确写为「严格零bit（conf_plus=false ∧ conf_minus=false）」。这是 impl 阶段的实装精确性要求，
**不翻转方案 R2 整体架构**，但 #32 impl 必须用严格零bit条件。

---

### 质询2：R2 不碰 class_index 的推导链 — **否定部分成立（建议升格）**

Codex 判「推导成立，但需测试守护」。代码中 `class_index()` 只读六 bit（types.rs:229），
struct_break_dir 作为 BspPoint 的新字段不参与 `class_index()` 计算——这是 L0 代数事实。

Codex 的否定是「需要测试守护」，而非推导链本身错误。**评判**：推导链成立。Codex 的建议
（新增测试断言）合理但不是阻塞性否定——这是 impl 阶段的测试要求，记录为 #32 改点。

---

### 质询3：Weak_Θ 语义问题 — **否定成立，但不触发 escalate**

Codex 指出「buy1 冻结为 MACD 面积但注释声明为第34课背驰 = 声明膨胀」。

**核验**：当前 signal.rs:107/311/318 注释写「仅背驰确认才置第一类端点语义」、「buy1/sell1 保严格
趋势背驰语义」。若第34课 canonical 背驰已被 PDF §34课定义为词典序（Weak_LEX），则用纯 MACD 面积
判 buy1 = 声明代码不具备的能力（no-patch-mentality §5「声明膨胀」）。

**方案已自我发现此问题**（§5 open point，§8结论中的「gate冻结/feature升级」精确分层），且诚实标注
认识论等级。PDF §6方案A（Q6裁决）明确说「P2目标是让宽候选进样本，MACD作feature不作gate」——这本身
就是把 buy1/sell1 的语义限定为「MACD-area 第一类」（不是完整力度缠论第一类），然后通过 selector
层的 Weak_Θ 力度门来逼近完整 canonical 背驰。

**判定**：Codex 否定成立，但方案已提出正确解——注释/命名降格（buy1 明确注释为 MACD-area-gate，
非 canonical Weak_Θ），完整背驰语义在 selector 层实现。**不触发 escalate**，记录为 #32 改点
（注释修正）。

---

### 质询4：GOLDEN 重算处置 — **否定部分成立，方案A更优**

Codex 提出两种诚实方案：A（更新 GOLDEN + 说明）、B（新增独立断言）。

方案文档（§5 bit-exact风险面第(1)项）已要求「GOLDEN 重算 + 逐 case 对拍证六 bit 不变」，
与 Codex 方案A 一致。Codex 补充的方案B（保留旧 GOLDEN + 分离新字段断言）也合理，
但增加测试复杂度。

**判定**：方案文档的处置是正确的（方案A），Codex 否定不成立（方案已覆盖）。Codex 方案B
是可选的增强，非必须。

---

### 质询5：改点清单遗漏（assemble_gamma_with_tower 等入口）— **否定成立**

Codex 指出 `assemble_gamma_with_tower`（interp.rs:308）需要同步，两个 hist 入口也需要保留
struct_break_dir。

**核验**：
- `extract_signals_with_hist`（signal.rs:550）是 `extract_signals_with_hist_features(...).0`
  的包装，若 BspPoint 结构增加 struct_break_dir 字段，这两个入口自然携带（BspPoint 结构改了，
  所有返回 BspPoint 的函数自动携带新字段）。**无需额外修改入口函数签名**，只需确保
  `judge_first_cached` 正确填充新字段。

- `assemble_gamma_with_tower`（interp.rs:308）：其内部调用 `candidate_dir(bits)` 同路径，
  若方案 §3 的「interp.rs candidate_dir 层增回退分支」是修改 candidate_dir 函数本身，
  则 assemble_gamma_with_tower 会自动受益，**无遗漏**。但如果回退分支是在 assemble_gamma
  内单独写而非 candidate_dir 函数内，则 assemble_gamma_with_tower 会遗漏。

  **关键细节**：方案 §7 改点3「interp.rs:206-211 candidate_dir：增回退分支」——改点写在
  candidate_dir 函数里，而非 assemble_gamma 调用点。这样 assemble_gamma_with_tower（interp.rs
  :308 及后续函数）也调用 candidate_dir，自动获得回退逻辑。但需确认 BspPoint 的 struct_break_dir
  字段在 assemble_gamma_with_tower 的 BspPoint 来源（signal.rs extract_signals 路径）里已填充。

**判定**：Codex 否定部分成立。改点3必须修 candidate_dir 函数（非调用点），以覆盖所有入口。
方案改点表述已指向 interp.rs:206-211 的函数本身，方向正确，但遗漏说明此设计选择的覆盖理由。
记录为 #32 impl 注意项：candidate_dir 函数级修改，不是调用点修改。

---

### 质询6：选择偏差消除有效域收窄 — **否定成立，方案文档需补声明**

Codex 指出「R2 只消除 C<A MACD 门的选择偏差，不能宣称 Γ_struct 全量进样本」。

**核验**：signal.rs:293-300 的几何门（trend_dir匹配 ∧ 破最后中枢 ∧ A/C 可配对）仍然存在。
Γ_MACD ⊊ Γ_geometric ⊊ Γ_struct（完整结构破位集）。R2 把 Γ_MACD 扩展到 Γ_geometric，但
仍不等于 Γ_struct。方案 §8 边界条件(a) 提到此，但 §8.6 影响声明表述为「消选择偏差」，
可能被解读为宣称 Γ_struct 全入样本。

**判定**：否定成立。需要在 #32 impl 的注释中精确声明有效域：「消除 MACD-C≥A 对几何门通过候选
的预删（扩展 Γ_geometric 进样本），不声明 Γ_struct 全入样本」。

---

## 分叉裁定汇总（供 #32 impl 执行）

### 分叉1 裁定：R2 采纳（含实装精确要求）

- R1 排除：定理，违 class_index 硬约束。
- **R2 采纳**，但 impl 精确要求：
  - candidate_dir 回退条件必须写为 `!bits.conf_plus() && !bits.conf_minus() && struct_break_dir.is_some()`，禁止用 `root_sel == Flat`
  - candidate_dir 改点在函数体内（interp.rs:206-211），覆盖所有调用者
  - buy1/sell1 注释必须降格：明确标注为「MACD-area proxy，非 canonical Weak_Θ」

### 分叉2 裁定：selector 独立 feature 通道（维持方案原判）

Codex 未否定分叉2。力度进 selector 层独立 feature 通道（不碰 MuClass）的裁定维持。

---

## 综合 verdict

**verdict: conditional-pass**（条件通过，需在 impl 阶段补入三个精确要求后可进 #32 实装）

**三个必须补入的精确要求**：
1. **candidate_dir 回退条件精确化**（Codex 致命否定）：严格零bit检查，不用 root_sel==Flat
2. **选择偏差声明收窄**（有效域诚实）：明确声明 R2 扩展 Γ_geometric，不声明 Γ_struct
3. **buy1 注释降格**（声明-能力一致）：buy1 注释明确为 MACD-area-gate proxy

**不触发 escalate** 的原因：无 R1 vs R2 的真矛盾（R1 已被硬约束唯一排除），无 Weak_Θ 下沉 buy1 的
语义冲突（PDF §6方案A 明确指示「力度作特征」的分层），Codex 的「致命」否定均是 impl 实装精确性
问题，不是方案架构上的不可消解矛盾。

---

## 边界条件（本裁定翻转条件）

- 若 #32 impl 阶段确认「所有 Flat 候选都是真零bit，不存在 buy1=sell1=1 的双侧候选」→ Codex 质询1
  否定可以降级（但仍建议用严格条件，防御性编程）
- 若 PDF 中有更清晰的证据表明「buy1/sell1 bit 本身必须代表 canonical Weak_Θ 背驰」→ 触发真矛盾
  escalate（当前无此证据，PDF §6 明确 MACD 从 gate 降为 feature）

---

## 影响声明

- 落盘文件：`.chanlun/review-results/codex-p2-plan-review-20260701.md`（本文件）
- 未改任何 src/
- 下游：#32 impl 必须纳入三个精确要求；#34 codex 代码异质审计在 impl 后执行
