# quality-guard 合规扫描 — prop4-consume + 561 协变/CC prove（2026-06-23）

> 工位：quality-guard（结构工位，常设）。topo_address: swarm/structural/quality-guard。
> 扫描对象：本 session 三份核心产出 + prop4-nest commit 变更（rec_engine.rs / rec_stream.rs）。
> 认识论等级：本报告 = **L0 技术审计**（对照已结算规则/谱系，无数据推断）。
> 裁决：**全部合规，无中断#1，无需退回。** 2 项低severity观察（非违规）。

---

## 0. 一句话判决

三份产出**六要素齐全 + 认识论等级标注合规（231号）+ 谱系引用充分**；代码变更**全 flag 门控、bit-exact 契约结构成立、无补丁思维/无声明膨胀/无死代码**。两份 prove 报告**自身即守 formalization-validity-domain**（主动标注"声称'539 完全非市场规律'=声明膨胀"并自我约束）——是有效域规则的正面范例。

---

## 1. 结果包六要素扫描（result-package.md 强制）

### 1.1 `prop4-consume-readingB-L3-20260623.md` — ✅ 完整（六要素 + L0/L3）

| # | 要素 | 位置 | 核验 |
|---|------|------|------|
| 1 | 结论 | §0 + §4.1 | ✅ consume平空解5/8（92–99%砍幅），3/8不救，strict单独无效 |
| 2 | 定义依据 | §4.2 | ✅ 第27课区间套/T算子construct对偶/命题2/第37/24课背驰段（`trend_diverging_segment`） |
| 3 | 边界条件 | §4.3 (a–d) | ✅ 4条翻转条件，含"未测"诚实标注（核心入场级别/配额1/3/OKLO小样本/极端牛有效域外） |
| 4 | 下游推论 | §4.4 | ✅ 开放轴C单独不是吃跌解 / consume vs ANCHOR强牛对立 |
| 5 | 谱系引用 | §4.5 | ✅ 556/命题2/539/552 ANCHOR/558 pending/231 + 3条 memory |
| 6 | 影响声明 | §4.6 | ✅ rec_engine.rs/rec_stream.rs + bit-exact OFF守住 + commit `79257d6e46` + 多session碰撞声明 |

**认识论等级（231号）**：§5 显式 L0(§1实装)/L3(§2/3矩阵+消融)。标题行 line 6 "L0 实装 / L3 实证（8标的×4变体）"。
**有效域膨胀检查**：✅ **无**。§3.3 "解穿仓 ≠ 解踏空"（5/8救穿仓仅2/8超BH），§5 标注 Codex/Gemini 双429 → 两断言"**未经异质质询**，待配额恢复"——否定性结果（3/8 consume=0不救）显式保留，未在 L3 后膨胀声称"已验证"。**模范诚实。**

### 1.2 `polarity-level-covariance-prove-20260623.md`（CC v3 主prove）— ✅ 完整

| # | 要素 | 位置 | 核验 |
|---|------|------|------|
| 1 | 结论 | §五.1 | ✅ 协变=D∞ L0必然推论，定位P2（非裸srs=r⁻¹）；CC主prove + N9降推论 |
| 2 | 定义依据 | §五.2 | ✅ D∞/P2/NR-3/四类分类/orbit doc§1.1/§3.2/§4 + 第17/21/19/30课 + dir_to_polarity(rec_engine.rs:152) |
| 3 | 边界条件 | §五.3 | ✅ 3条翻转条件（srs单独蕴含协变=反例否证 / 539残余税L3 / sink代理失配） |
| 4 | 下游推论 | §五.4 | ✅ N9统一四开放轴 / 删if-else硬编码regime / N7⊥N8⊥N9正交 |
| 5 | 谱系引用 | §五.5 | ✅ 能指碰撞σ/545/544/231 + **显式声明"检索 settled/ 未见独立 polarity covariance 节点，本文首次结晶"** |
| 6 | 影响声明 | §五.6 | ✅ 不改动代码/定义，仅设计；影响模块（若实装）列明 |

**认识论等级（231号）**：§六 + §九.7 双表，逐命题 L0/L2/L3 标注。
**有效域膨胀检查**：✅ **无，且主动防御**。§3.4 逐字："声称'补 N9 ⟹ 539 完全是架构产物非市场规律'= **声明膨胀**（把有效域=定义域）"；(b1)确认滞后标 L0不可约、(b2)净亏标 L3未决；§九.6 "完全 Ω ⟹ 539 消失 = L3未决，不声称"。§八 codex 异质质询 confirmed（三证伪路径全失败）。**这是 formalization-validity-domain 规则的正面执行范例。**

### 1.3 `561-polarity-level-covariance-necessity.md`（settled 谱系）— ✅ 格式合规

**谱系文件格式检查（清单#1）**：
- `status: 已结算` ✅（且精确分层："CC L0核心已结算；N9实装/CC实装/539(b2)L3=开放轴待授权"）
- 前置谱系 `depends_on: [541, 545]` ✅
- 关联谱系 `related_genealogy: [539,560,553,558,537,556,555,552,527,544,231]` ✅
- 完整 frontmatter：id/type/negation_source/negation_form/topo_effect/l0_source/provides_l0_backing_for/unn_axis/created/settled/updated/trigger/settled_by/evidence_file 全present ✅
- 正文含张力检查（CC vs 539/553/560/558/537）→ 明判"无矛盾，无 /escalate"；认识论等级表；影响声明；选择类待编排者标注 ✅

---

## 2. 代码违规扫描（no-patch-mentality / no-workaround / coding-style）

**对象**：`rec_engine.rs`（nest_consume_step / nest_try_flip / nest_step / strict gate / EngineConfig 3 flag）+ `rec_stream.rs`（level_div 计算 + harness）。

### 2.1 补丁思维（no-patch-mentality）— ✅ 无违规

- **flag 门控非补丁**：`enable_nest_consume`/`enable_nest_strict` 是**受控对照变体**（消融实验隔离贡献），非"在错误代码上加 workaround 让它大致工作"。OFF 基线逐字保留（`off()` 显式置 false），bit-exact 契约（line 61–62）结构成立（`nest_step` 仅在 `enable_nest` 的 A^NEST 分支调用，OFF 路径不触达）。
- **无渐进式回避 / 无兼容性垫片**：未见"先保留旧逻辑加新分支"的 fallback 残留。consume 是 `nest_step` 主翻转未触发时的对偶操作（construct↔consume），结构对称非边界打补丁。
- **无 TODO 遗留**：未见 `TODO/FIXME` 绕过标记。

### 2.2 声明膨胀（no-patch-mentality #5）— ✅ 无违规，且主动标注有效域边界

- `nest_step` doc（line 1029–1031）**逐字主动声明**："**有效域边界（不声明膨胀）**：区间套中间级别逐级背驰段未逐层校验……严格逐级嵌套未实装，记为有效域边界。" → 代码注释**诚实声明能力边界**，不多声明。
- consume doc（line 1132–1138）准确描述"平空不开反向腿（区分 sink）"——声明与实现一致（line 1167 `rec_reduce` 仅减仓，无 `rec_add` 反向腿）。

### 2.3 死代码（coding-style）— ✅ 无违规

- 诊断计数器（`flip_log`/`nest_trades`/`nest_diag`/`n_nest_consumes`/`nest_consume_pnl`/sink_by_level 等）= **137号 make-observable 模式**（合法可观测产出，被 harness §2.2 矩阵消费），非死代码。
- `level_div` 在 rec_stream.rs:329–346 每次重跑计算并注入 `view.level_diverge`，OFF/ANCHOR 不消费 → 见 §2.5 观察（非死代码，是 nest 启用时的必需输入）。

### 2.4 immutability（coding-style CRITICAL）— ⚠️ 不判为违规（有效域不覆盖）

`rec_engine.rs` 全程 `&mut self` 状态机突变（`self.instances[k]`/`self.free`）。**不判为违规**，理由（formalization-validity-domain 思路）：
- 该 commit **未引入新突变模式**——整个 rec_engine 既有架构一致建于可变引擎态（flat t_engine 对照 bit-exact 的前提）。
- immutability 规则的**有效域** = 并发/共享可变态的隐藏副作用；本引擎是**单线程顺序逐bar回放**，每bar克隆 instances 数组（百万次）= 性能荒谬且破坏 bit-exact 对照。规则定义域不覆盖单线程顺序引擎态。
- 透明声明：此处**不静默忽略规则**，而是判定规则有效域不及，故不 flag。

### 2.5 与 561 prove 诊断的一致性核验（关键）

561 报告**自我诊断** `nest_try_flip`（背驰段武装整仓翻转，line 1089+1119）= CC "过覆盖"/N9 违反特例，`route_bsp`（line 956 二元 is_buy 路由）= Ω 坍缩完全分类。**这不构成需我新报的代码违规**：
- CC/N9 prove **未实装**（561 status 明示"设计，编排者指示不实装，待授权"）——代码先于 prove 存在，是命题4 读法乙**探索性实验**（testing-override.md 生成态例外允许）。
- 报告**未声称代码正确**——L3 报告诚实报穿仓 7/8（NEST 基线）/ 5/8（consume）。代码与报告诊断**一致**（报告诊断代码，非掩盖）。
- 561 §张力检查已判"CC 与 539/553/560/558 一致深化，无矛盾，无 /escalate"——**概念分离已由 genealogist 以 aufhebung 结算**，我不重复触发中断#1。

---

## 3. 低severity观察（非违规，供 prop4-nest 工位参考，不阻塞）

### O1. strict gate 空区间真空通过（鲁棒性边界）
`nest_try_flip` line 1090–1095：`top_lvl = (0..MAX_LEVEL).rev().find(level_diverge.is_some())`，`(loc..=tl).all(...)`。若 `loc > tl`（定位 type1 级别高于任何背驰段级别），`(loc..=tl)` 为空区间 → `.all()` 真空返回 true → strict gate 退化为 no-op 放行。当前无害（报告实测 NEST_STRICT ≈ NEST 全8，strict 几乎无效应），但严格逐级嵌套语义下"空区间=贯通"是隐式假设，建议显式断言 `loc <= tl` 或注释该边界。**非违规**（不涉及已结算原则，是逻辑边界精化）。

### O2. level_div 全程计算、OFF 不消费（计算冗余）
rec_stream.rs:329–346 每次重跑计算 `level_div` 并 `view.level_diverge = level_div`，OFF/ANCHOR 路径不调用 `nest_step` → 计算结果未被消费。bit-exact 行为不受影响（仅 view 字段赋值），但 OFF 基线承担 nest 专属计算开销。**非违规**（nest 启用时必需），可选优化：将 level_div 计算移入 `enable_nest` 守卫内。

---

## 4. 六要素（本审计报告，技术性产出简化版 + 谱系引用）

1. **结论**：三份产出六要素齐全、认识论等级标注合规（231号）、谱系引用充分；prop4-nest 代码变更无补丁思维/无声明膨胀/无死代码，bit-exact 契约结构成立。两份 prove 报告自身守 formalization-validity-domain（主动防声明膨胀）。**全部合规，无退回。**
2. **边界条件（翻转处）**：若 (a) 任一报告在 L0/L1 后声称"已验证"而未标等级，或 (b) 代码引入未声明的能力膨胀/掩盖矛盾的 workaround，或 (c) 561 张力检查实为不可调和矛盾被误判为"一致深化"——则合规判决翻转。当前证据：(a) 否（等级齐全）、(b) 否（注释诚实+flag门控）、(c) 否（genealogist 已 aufhebung 结算，CC subsume 539/553/560 可分层）。
3. **谱系引用**：561（CC/N9，本次扫描对象）；231/formalization-validity-domain（有效域规则，两 prove 报告的核验基准）；090（声明膨胀禁止来源）；137（make-observable，诊断计数器合法性依据）；558（TYPE-COLLAPSE，561 一般化对象）。涉及概念分离领域（极性协变/Ω坍缩/做空腿regime）的产出均已引用对应谱系。
4. **影响声明**：本报告不改动任何代码/定义/谱系；新增 `.chanlun/review-results/quality-guard-20260623.md`；产出 = 合规清单 + 2 项低severity观察（O1 strict 空区间 / O2 level_div 冗余），转 prop4-nest 工位参考，不阻塞 commit。

---

## 5. 中断判定

- **中断#1（概念分离信号）**：❌ 不触发。561 已由 genealogist 完成张力检查（CC vs 539/553/560/558 = 一致深化），概念分离以 aufhebung 结算，无新的不可调和矛盾从代码违规中浮现。
- **退回修正**：❌ 无。三份产出合规，无六要素缺失、无锚定缺失、无有效域膨胀。

**结论：合规放行。** 2 项低severity观察供工位参考（非阻塞）。
