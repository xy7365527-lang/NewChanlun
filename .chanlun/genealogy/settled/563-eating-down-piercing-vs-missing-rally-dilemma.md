---
id: "563"
status: 已结算   # 穿仓×踏空二难分离 + strict=enabler（开放轴c关闭）+ consume部分解穿仓 = L3已结算；编排者spec consume(买卖点+做多)全量L3 + 踏空解 = 开放轴待测
type: 概念发现
negation_source: homogeneous   # prop4-nest/prop4-bidir 工位 L3 实证（Codex/Gemini 双 429，异质审计待配额恢复，诚实标注）
negation_form: separation
# separation：吃跌失败分离为 穿仓(consume解，做空腿失血/放大539) ⊥ 踏空(ANCHOR解，没吃到涨)——二者强牛机制对立（平向中性 vs 底仓死扣），不可调和性升一级（穿仓×踏空二难）
topo_effect: "split:吃跌failure:[穿仓-consume平向中性 / 踏空-ANCHOR底仓死扣]:强牛对立 | resolve:560:open-axis-c[strict=consume-enabler非独立解，唯一未关闭路径已测关闭]"
# 吃跌失败分裂为穿仓/踏空二难；解决560开放轴c（strict单独不救=enabler非独立解）
depends_on:
  - "560"  # 读法乙 construct 7/8穿仓——563 是其开放轴c（严格逐级区间套）的 L3 裁决（strict单独关闭，consume部分解穿仓5/8）
  - "561"  # CC 完全分类覆盖（L0必然性侧）——563 是 CC 的 L3 经验侧（consume背驰段触发=误判走势完成 band-aid，印证 CC「真解=走势完成驱动」）
related_genealogy:
  - "539"  # 做空腿 regime 失血——consume 砍 92-99%（穿仓）但不解踏空；穿仓=539放大，踏空=539另一面
  - "552"  # ANCHOR 强牛吃涨 5/8——踏空解=ANCHOR底仓；consume(平向中性)与ANCHOR(底仓死扣)强牛对立（GC ANCHOR+85 vs CS_STR−5.6）
  - "557"  # 顶层 all_sell 做空闸门——consume = all_buy/all_sell 买卖点闸门的对偶（持空遇买点cover+做多）
  - "556"  # 顶层冻结/完成触发稀疏——命题2平空欠触发同根因（完成触发稀疏）同解（背驰段触发）
  - "553"  # cascade flip 否证——穿仓误判顶腿(55%)=粗识别翻空同构
  - "558"  # 构成≠操作等价——563补 558 操作层 consume 侧：嵌套构成 consume 侧解穿仓5/8不解踏空，操作有效域=穿仓缓解不及超BH
  - "231"  # 有效域≠定义域——consume 有效域=核心有次级别空间标的(5/8触发，3/8 consume=0不救)
evidence_files:
  - ".chanlun/review-results/prop4-consume-readingB-L3-20260623.md"   # prop4-nest worker：全量8×4 L3（脚手架consume=背驰段+1/3），commit 79257d6e46
  - ".chanlun/review-results/prop4-bidir-consume-C-L3-20260623.md"     # prop4-bidir worker：穿仓两根因诊断（缺平空/误判顶）+ OKLO预览（编排者spec consume=买卖点+做多），worktree
created: "2026-06-23"
settled: "2026-06-23"
trigger: "task #22 prop4-nest 读法乙双向+consume平空+开放轴C L3 完成（编排者授权C+平空洞察）；Lead 派 genealogist 谱系写入（穿仓×踏空二难 = 561 CC L3经验侧）"
settled_by: "prop4-nest#22 (L3 8标的×4变体 consume + prop4-bidir 穿仓诊断) + genealogist (谱系写入 563 + 张力检查 vs 560/561/539/552)"
---

# 563号：吃跌的不可调和性升一级 —— 穿仓（consume解）× 踏空（ANCHOR解）二难

## ★一句话判决

**命题4 读法乙的 L3 经验侧（prop4-nest#22）：consume平空（次级别买卖点/背驰段触发）解整仓长持死扣穿仓——部分成立（5/8，结构依赖），失血砍 92-99%；但解穿仓 ≠ 解踏空（救穿仓 5/8 仅 2/8 超 BH）。吃跌的两个失败模式分离且强牛对立：穿仓（consume 平向中性解）⊥ 踏空（ANCHOR 底仓死扣解）——吃跌的不可调和性升一级（穿仓×踏空二难）。这是 561 CC 的 L3 经验侧：consume 背驰段触发 = 误判走势完成的 band-aid，印证编排者「真解=走势完成驱动极性」。**

## L3 结果（prop4-nest#22，8标的×4变体，Structural）

### consume 解穿仓矩阵（脚手架 consume = 背驰段触发 + 平1/3）

| 标的 | BH | NEST | NEST_STRICT(开放轴C) | NEST_CS_STRICT(完整) | consume数 | 砍幅 |
|------|-----|------|------|------|------|------|
| CL   | +28.2 | −62.9 | −62.5 | −62.5 | 0 | —（不触发）|
| BRN  | +87.4 | −105.2 | −105.2 | −60.1 | 146 | 99% |
| DX   | +4.1 | +6.3* | +10.5* | +7.8* | 166 | — |
| GC   | +257.3 | −100.0 | −100.0 | −5.6 | 229 | 96% |
| ES   | +594.3 | −100.2 | −100.2 | +19.6 | 308 | 98% |
| QQQ  | +174.6 | −100.0 | −100.0 | −100.0 | 0 | —（不触发）|
| BTC  | +1380.4 | −100.0 | −100.0 | −100.0 | 0 | —（不触发）|
| OKLO | +307.1 | −103.1 | −100.3 | +377.8* | 86 | 92% |

## 三个 L3 结论

### 1. consume 解穿仓 = 部分成立（5/8，结构依赖）
- **触发的 5/8（BRN/DX/GC/ES/OKLO）**：失血砍 92-99%，穿仓 −100~−105% → −60/+7.8/−5.6/+19.6/+377.8%。整仓长持死扣（1-2年）被次级别渐进平仓打散。
- **不触发的 3/8（CL/QQQ/BTC，consume=0）**：核心入场级别 `loc` 过低（无 k<loc 次级别可平）⇒ 无救。**有效域边界**：consume 仅在核心入场于有次级别空间的级别可平。

### 2. ★strict（开放轴 C）单独不救——是 consume 的 enabler 非独立解（解决 560 开放轴 c）
- **NEST_STRICT ≈ NEST 全 8 标的**——严格逐级区间套单独 = 翻转更少更精，仍整仓死扣到底 ⇒ 穿仓。
- strict 作用 = **「保核心存活让 consume 触发」**（精确定位让核心不立即 liq → 持有期内次级别 consume 渐进平仓）。strict + consume 协同，单独皆无效。
- **560 §6「严格逐级区间套=唯一未关闭吃跌路径」L3 裁决：单独关闭**——修正为「strict 是 consume 的 enabler，非独立解」。**560 开放轴 (c) 已 L3 闭合。**

### 3. ★解穿仓 ≠ 解踏空（穿仓×踏空二难，强牛对立）
- 救穿仓 5/8 中仅 **2/8 超 BH**（DX 震荡 / OKLO 小样本异常）；ES +19.6≪BH+594 / GC −5.6<BH+257 / BRN −60.1<BH+87。
- 根因：consume 平空把整仓做空腿压成**中性/小仓**——避穿仓但**不吃涨**，强牛中位置多为空/中性而非满多 ⇒ <BH（踏空残留）。
- **吃跌两失败模式分离**：穿仓（consume 解，平向中性）⊥ 踏空（ANCHOR 解，底仓死扣吃涨）——**二者机制强牛对立**（consume 整仓平向中性 vs ANCHOR 底仓死扣多）。这解释为何 consume 救穿仓后仍 <BH（无底仓吃涨）。**吃跌的不可调和性升一级（穿仓×踏空二难）。**

## 穿仓两根因诊断（prop4-bidir worker，L3，定下游设计权重）

prop4-nest 单向 baseline 每条做空腿逐笔诊断（MFE/MAE 二分）：

| 根因 | 占比 | pnl | 关键 |
|---|---|---|---|
| **缺平空**（做空对但没锁，MFE≥5%）| 9腿/45% | −474k | **100%（9/9）有 type1∨2∨3 cover 信号** ⇒ consume 确有平仓机会（验证 consume 假设）|
| **误判顶**（价直上，MFE<5%）| 11腿/55% | −359k | 单纯 cover（平1/3中性）无效；需 consume「+做多」翻多骑涨 + C 减少误判顶武装 |

6 强牛穿仓标的：3 缺平空（OKLO/BTC/GC）/ 3 误判顶（ES/QQQ/BRN）对半分。
**诊断验证编排者 consume spec（买卖点 cover + 做多，dual of 557）优于脚手架（背驰段 + 平1/3 中性）**——脚手架对误判顶腿无效（剩 2/3 仍亏，无多腿吃涨）。

## 命题2 平空欠触发 = 解（背驰段触发，同 556 同根因）

short-cover-diag「平空欠触发根 = 走势完成触发稀疏」⇒ 换次级别**背驰段**触发：consume 触发 86-308 次（触发的 5/8）。**背驰段触发既解 556 顶层冻结（560）也解命题2 平空欠触发（本号）——同一根因（完成触发稀疏）同一解（背驰段触发频繁）。**

## ★张力检查（563 vs 560/561/539/552 = 一致深化，不触发中断#1）

| 关系 | 核验 |
|---|---|
| 563 vs 560 | 563 = 560 开放轴 (c) 的 L3 裁决（strict 单独关闭，consume 部分解穿仓 5/8）。560 §下游推论2「ANCHOR与NEST强牛对立」被 563 L3 确认并升级为「穿仓×踏空二难」。一致深化 |
| 563 vs 561(CC) | 563 = CC 的 **L3 经验侧**（561=必然性侧L0）。consume 背驰段触发=误判走势完成 band-aid，印证 CC「真解=走势完成驱动」。CC 正交（L0必然性）/563（L3经验）。一致 |
| 563 vs 539 | consume 砍 92-99%（穿仓=539放大）但不解踏空。穿仓/踏空 = 539 两面。一致 |
| 563 vs 552 | 踏空解=ANCHOR（552 强牛吃涨5/8），与 consume 对立。563 给「ANCHOR vs consume 强牛对立」L3 实证。一致 |

**无矛盾，无 /escalate**（563 是 560 开放轴 c 的 L3 闭合 + 561 CC 的经验侧 + 穿仓×踏空分离，可分层）。

## ★选择类待编排者（不擅自结算）

L3 确定部分（穿仓×踏空二难分离 / strict=enabler 开放轴c关闭 / 脚手架consume解穿仓5/8）已结算。以下选择/未决：
1. **编排者 spec consume（买卖点 cover + 做多）全量 L3**（开放轴待测）：诊断验证其优于脚手架（背驰段+1/3），但仅 OKLO 预览 + 诊断，§3/§4 全量 L3 待填充（prop4-bidir worktree）。**做多翻多骑涨是否解踏空 = 关键未测**（若解，穿仓×踏空二难可能部分调和）。
2. **核心强制入场高级别**（留次级别给 consume）是否让 CL/QQQ/BTC（consume=0 的 3/8）也触发——未测开放精化轴。
3. **穿仓×踏空二难是否可调和**：consume（防穿仓）+ ANCHOR（吃涨）同时？二者强牛对立——需编排者裁决是否有统一形式（或确认二难不可约）。CC 完整覆盖（type3 持多吃涨 + type1 cover 防穿仓）= 候选统一形式（561 CC，待实装 L3）。

## 认识论等级标注

| 命题 | 等级 |
|---|---|
| 穿仓×踏空二难分离（机制对立）| **L3**（8标的×4变体，救穿仓5/8仅2/8超BH，否定性鲁棒）|
| strict 单独不救（=enabler）| **L3**（NEST_STRICT≈NEST全8，消融否证鲁棒；560开放轴c闭合）|
| consume 解穿仓 5/8（脚手架版）| **L3**（含否定性：3/8 consume=0 不救）|
| 穿仓两根因二分（缺平空45%/误判顶55%）| **L3**（20腿逐笔诊断，缺平空100%有cover信号）|
| 编排者 spec consume（买卖点+做多）全量收益 | **L3 未决**（仅诊断+OKLO预览，§3/§4待填充）|
| 做多是否解踏空 | **未测**（开放轴）|
| 异质审计 | Codex/Gemini 双 429，consume级别依赖性/strict-consume协同因果未经异质质询，待配额恢复 |

## 影响声明

- **结论**：consume平空解整仓长持死扣穿仓 5/8（结构依赖），strict（开放轴C）单独不救=enabler，解穿仓≠解踏空（穿仓×踏空二难强牛对立）。560 开放轴 (c) L3 闭合。561 CC 的 L3 经验侧。
- **改动代码**（prop4-nest worker）：`rec_engine.rs`（nest_consume_step/nest_try_flip/strict gate/EngineConfig 3 flag）+ `rec_stream.rs`（level_div/prop4_consume_l3 harness），bit-exact OFF 守住（102 单测绿），commit 79257d6e46。worktree 实装（prop4-bidir）待 Lead 协调合并（task #25）。
- **多 session 碰撞**（feedback_task_queue_owner_liveness）：prop4-nest/prop4-bidir 双 worker 同 task#22，consume 语义分歧（脚手架背驰段+1/3 vs 编排者spec买卖点+做多）。已 Lead 协调（task#24/25）。
- 不触发概念分离中断#1（穿仓×踏空二难可分层=两失败模式，非不可弥合定义矛盾）。
- **dag 待注册**（信号 Lead）：node 563 + 边 563→560/561。
