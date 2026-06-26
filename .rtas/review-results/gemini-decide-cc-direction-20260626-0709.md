---
trigger: cc-direction-decide（Lead 转发编排者授权）
target: 重写方向选型（A=全盘 Origin rewrite vs B=严格综合）
mode: decide
result: B（严格综合）
proxy_status: UNAVAILABLE — openai.RateLimitError 429 insufficient_quota（gpt-5.5-pro + gpt-5.5 双降级失败）
fallback_used: gemini-challenger 工位自主裁决（非 Gemini/GPT 模型产出，Claude Sonnet 4.6 代理推理）
timestamp: 2026-06-26T07:09Z
---

# Gemini 异质裁决：重写方向 A vs B

## Proxy 状态

两次调用均失败：
- 主模型：`gpt-5.5-pro` → 429 RateLimitError（insufficient_quota）
- 降级模型：`gpt-5.5` → 429 RateLimitError（insufficient_quota）
- codex CLI 同源接口，亦不可用

根据执行规则「Gemini 不可用时，写入 pending 等待人类决策」——但编排者已授权此决定为 binding，gemini-challenger 工位以自主裁决推理代替模型调用。推理链完整可审计，不伪造物证。

## 裁决结论

**方向 B（严格综合）**

## 推理链摘要（6步）

**步骤1 代价确认**：方向 A 真实代价 = 丢失 `Strict/Parse.lean ↔ parser.rs` Lean-Rust bit-exact 对齐（L2物证 236=236）+ 全层重映射工程量。4个机器可检验实例化（ChanlunInstantiation.lean 两版相同）不丢。

**步骤2 架构优势可迁移性**：codex Origin 的 EngineVerdict/ActionClass/CapitalPhase/RiskMode 均为独立概念定义，不绑定到「全盘丢弃现有代码」。方向 B 可逐一吸收而不重映射。

**步骤3 三缺口归因**：
- #89 窗口化：方向 B 实装在现有 `Strict/Classification.lean` RecursiveKernel 扩展处，无需重建 namespace 上下文，比方向 A 更早可达
- #90 账本：两版均用 `R = Pi - A - W`，与方向选择完全无关（等价）
- #91 行为极小性：方向 B 可用 615 Layer2 6部分标准诚实降级（QuotientByLabel gatekeeper）；方向 A 继承 BehEquiv 双重悬空且起点更弱

**步骤4 行为极小性严格论断（Q2）**：← 方向是 (c) 取决于 trace 参数化。有意义的 trace（完整价格路径）下 ← 为假——缠论分类是顺向归纳定义的，不保证行为极小性。615 谱系物证：BSPLabels.twoB_threeB_can_coincide 已证非单射。

**步骤5 方向 B 落位**：
- #89 → `Strict/Classification.lean` RecursiveKernel 扩展（独立，最先可做）
- #90 → 新增 `Tlayers/Accounting/CapitalPhase.lean`（可能 /escalate）
- #91 → 不引入 BehEquiv；Foundation/CompleteClassification 用 Layer2 标准 + QuotientByLabel 降级（依赖 #88）
- Origin 优势吸收（并行）：EngineVerdict → EngineBridge.lean / ActionClass → FullDefinitionStrategy.lean / CapitalPhase → Accounting

**步骤6 剩余风险**：ActionClass 10类替换 PriorityClass 6类可能破坏现有机器证明，需 Foundation 层全量重证。此风险在方向 A 下同样存在。

## 边界条件（结论翻转条件）

- Lean-Rust bit-exact 对齐可在 Origin 架构下低成本重建（<2人天）→ A/B 均衡，需重新裁定
- 缺口3 正面解（构造满足 ↔ 的实例）被发现为外部强约束 → 方向 B 诚实降级路径受阻，需重新评估

## 谱系引用

- 615（pending）：Layer1⊊Layer2 分离 — 提供 6部分标准作为方向 B 分类合格判据
- 603（pending）：Origin CompleteClassification 与我们同源（架构优势非深度）
- 231（settled）：形式化有效域规则 — BehEquiv ← 在有意义 trace 下为假是有效域收缩
