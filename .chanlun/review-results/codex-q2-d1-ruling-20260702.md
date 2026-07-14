# codex 全权终裁：D1 收窄版设计 + Q2 双零消费者去留（full-strategy-pi p2 前置）

- **工位**：ws-codexq2（goal g-full-strategy-pi，p2 前置裁定，codex-p2 审计的续裁）
- **日期**：2026-07-02
- **审计对象**：Q1 考古后收窄的 D1 命题（`.chanlun/review-results/full-strategy-pi-conformance-20260702.md` §Q1）+ Q2（`mutex.rs::mutex_class` 8 谓词 vs `closed_loop/mutex_interp.rs::choose_action` 9 谓词双零消费者去留）
- **Codex 完整交互**：`.chanlun/review-results/codex-decide-20260702-235449-5f01.md`（自动持久化，含完整 prompt+response）
- **裁定结论**：**D1 收窄版 well-defined（有条件）+ Q2 明确裁定（删 choose_action，留 mutex_class）**

---

## 1. 结论

### D1（逐候选级等价命题）

**(a) well-definedness**：命题良定义，但前序设计草案（`.chanlun/review-results/full-strategy-pi-conformance-20260702.md` §p2 设计 D1）里提议的桥接函数签名 `predicates_of(candidate, working, active) -> Predicates` **不完整**——缺少 `interpret` 规则3/4 实际依赖的 fold 内状态 `opened: Vec<(level, dir)>`（`interp.rs:947,991`，本 fold 内已开过的 slot）。修正签名为：

```rust
predicates_of(candidate: &Candidate, working: &[(ActiveLeg, bool)], opened_slots: &[(u32, VoiceSide)], risk_gate: ...) -> mutex::Predicates
```

补上 `opened_slots` 后，投影对任意 (candidate, 当前 fold 状态) 是确定性函数——可以构造双射。但存在**一处概念坍缩**：`mutex_class` 的 `P7 same_dir_record_add`（字面语义"记录**或加仓**"）在当前 `interp::interpret` 里没有对应的"加仓"动作——`interp` 只有 `record` 桶，没有区分"记录"和"加仓"两种子语义。桥接只能把 P7 降级为"slot 已占用则 record"，不能声称验证了"加仓"这半句。

**(b) property test 方案**：不能只对拍最终三桶输出，必须用 **shadow fold**——按 `theta_key` 排序候选后，对每个候选在处理前用同一份 `working`/`opened_slots` 快照算出 `Predicates`，跑 `mutex_class` 得 `Cj`，把 `Cj` 映射到桶（`桥接表：C1→close(风险)/C2,C3→close/C4,C5,C6→open/C7,C0→record`），再执行 `interp` 同构的一步 fold，断言桶一致。生成域覆盖：多 level（含跨级）、Long/Short/Flat、`bsp_class∈{1,2,3,u8::MAX}`、能触发 buy/sell/双侧/无侧的 `BspBits`、18 类 `OperationRole`、以下活动集组合——空 active、同级同向、同级反向、双向腿共存、跨 level 腿、以及 fold 内已 opened 的 slot。**D1 范围内先固定 `risk_liquidate=false`**（P1/风险强平门当前由 `KThetaRiskGate.force_flat` 独立兜，不在 `interpret` 逐候选 fold 内表达，不在本次收窄范围——见下方边界条件）。

**(c) 残留裁定点（多谓词命中）**：测试要分两层。第一层（**必须通过**）：桶级一致——`C2/C3→close`，`C4/C5/C6→open`，`C7/C0→record`（P8 hold 在真实候选上基本不可达，因为候选本身若无类/无方向已在规则1被 record，若有类必落入规则2/3/4之一）。第二层（**可选，诊断性**）：精确 `Cj` 索引一致。若桶一致但精确 `Cj` 不一致——说明 `mutex_class` 只能充当"三桶级 oracle"，不能充当"子动作级 oracle"（如区分"记录"vs"加仓"），这不是 bug，是二者粒度不同的诚实标注。若**桶级本身**不一致，才是真矛盾（D1 桥接设计错误，或 `interp` 与 PDF 谓词优先级真分叉），此时走 `/escalate`，不 workaround。

**(d) 权威归属**：分叉时的裁定顺序——**PDF/项目已采纳的领域语义（若有更高权威文本）> 生产 `interp::interpret` 的候选集 fold 语义 > `mutex_class` 的组合逻辑镜像**。`mutex_class` 只有在 `predicates_of` 桥接被证明忠实（(a)(b)(c) 通过）时才具备 oracle 资格；它本身零生产消费者、从未被真实数据检验，不能单凭"是 PDF 字面谓词镜像"就自动推翻已在 wverify alpha 线上产出经验结果的生产代码。反之，生产代码的既有经验表现也不能反过来豁免它接受语义正确性检验——两者是互补关系，不是"胜负"关系：`mutex_class` 检验**语义**正确性，wverify alpha 检验**经验**有效性，二者独立维度。

### Q2（双零消费者去留）

**明确裁定**：删除 `closed_loop/mutex_interp.rs::choose_action`（及其 9 谓词 `ActionClass`/`c_holds`），保留 `strategy/mutex.rs::mutex_class` 作为 D1 property test 的唯一对拍参照。

**理由**：
1. `choose_action` 的 9 谓词对齐一份 Lean 原型（`FullDefinitionStrategy.lean`），比 PDF §5 举例的 8 谓词更细（多出 `Deleverage`/`ExecutionRepair` 两类），且模块自身注释已承认"9 谓词 ↔ alpha2 §5 的 8 谓词语义映射...不强行 1:1"——即它对齐的不是 D1 要检验的同一个输入空间（生产 `interp` 目前只有 close/open/record 三桶，没有去杠杆、执行修复这类细分动作）。用它做 D1 oracle 会引入 D1(a) 同样的范畴错误。
2. `no-patch-mentality`（090号语法规则）禁止补丁式重复实装并存。若同时保留两个零消费者谓词解释器，日后测试失败时无法判断哪一个代表项目规范语义——两个"权威镜像"并存本身就是矛盾源。
3. `mutex_class` 的 8 谓词直接对齐 PDF《缠论的全互斥定义策略》Part B 四的字面谓词列表（本仓库的第一权威依据），且其粒度（三桶级：close/open/record）与生产 `interp` 的输出粒度一致，是唯一在同一输入空间上可比较的候选。

**若需保留 Lean 一致性验证**：`choose_action` 删除前，若团队认为 Lean 原型侧的内部一致性仍有独立价值，只能**降级**为"仅验证 Lean 原型自身内部一致性的 test-only fixture"，且必须在模块文档中明示"与生产解释器 `interp::interpret` 无关，不作为 D1 桥接对象"——不能以任何形式暗示它是 D1 等价证明的参照。本裁定倾向直接删除（更严格，避免"降级但没人读文档"的腐化路径），删或留由实施工位按此原则二选一执行，不需要再次上浮。

---

## 2. 定义依据

- `.chanlun/review-results/full-strategy-pi-conformance-20260702.md` §Q1（本裁定直接依赖的原文考古结论）：Part B 四=候选级裁决，五/十三=组合级持仓向量优化，分层非矛盾——已裁定，本次不重新论证。
- `strategy/mutex.rs` 模块文档（1-46行）：8 谓词 P1..P8 逐字对齐 `买卖点alpha2.pdf` Doc2 §5 / Doc3 §6。
- `closed_loop/mutex_interp.rs` 模块文档（1-47行）：9 谓词自认"忠实镜像 Lean"而非 PDF §5 字面举例，且自陈"不强行 1:1"。
- `strategy/interp.rs::interpret`（940-999行）+ `theta_key`（859-877行）：生产 fold 的实际状态依赖（`working`/`opened`），是 D1(a) 签名修正的依据。
- no-patch-mentality（090号语法规则）：Q2 删除裁定的直接依据。

## 3. 边界条件

- **D1 范围边界**：本次裁定的 D1 property test **不覆盖 P1（风险强平）**——`interpret` 逐候选 fold 内没有表达风险强平（由 `KThetaRiskGate.force_flat` 在更上层的 K_Θ 收窄中独立处理，不在候选级 fold 内）。若后续要把 P1 也纳入逐候选级等价检验，需要先设计"风险门如何投影进候选级 fold"这一额外桥接，不在本裁定范围内，需重新裁定。
- **判定翻转条件**：若 property test 跑出"桶级本身不一致"（而非仅精确 Cj 不一致），本裁定的"D1 well-defined"判断不因此翻转——那不是 well-definedness 问题，是暴露了真矛盾（interp 语义与 PDF 谓词优先级分叉），走 `/escalate`。
- **Q2 翻转条件**：若团队后续正式把生产策略接入 Lean 9 谓词的细分动作模型（去杠杆/执行修复作为独立可交易动作），或 PDF/项目规范文档明确把 9 谓词设为新 canonical，则 Q2 判定翻转，`choose_action` 可重新评估留用。当前无此计划，判定维持删除。

## 4. 下游推论

- p2 实装工位可基于本裁定推进 D1：新增 `strategy/mutex.rs::bridge_from_interp`（或类似命名）实现修正后的 `predicates_of` 签名（含 `opened_slots` 参数）+ shadow-fold property test（覆盖 §1(b) 生成域），断言桶级一致（§1(c) 第一层）。
- `closed_loop/mutex_interp.rs` 删除是独立于 D1 实装的清理动作，可并行执行（无数据依赖）。
- P7 命名建议（Codex 附带风险提示）：`same_dir_record_add` 现有命名暗示"加仓"语义，但生产侧无对应动作，建议实装时一并改名为更诚实的 `same_slot_record`（或保留原名但在文档补充"生产侧无加仓子动作，仅记录"的诚实声明）——非阻塞，实装工位酌情处理。
- P1（风险强平的候选级投影）留作后续独立裁定项，不阻塞本次 D1/Q2 收口。

## 5. 谱系引用

- `.chanlun/review-results/codex-p2-design-ruling-20260702.md`：本裁定的直接上游（D1 原方案范畴错误的发现）。
- `.chanlun/review-results/full-strategy-pi-conformance-20260702.md` §Q1：本裁定依赖的原文考古结论（多候选并行合规、D1 收窄依据）。
- no-patch-mentality（090号语法规则）：Q2 删除裁定的谱系依据。
- formalization-validity-domain（231号）：D1(d) 权威归属推理中"经验有效性≠语义正确性，两者独立维度"的谱系依据（有效域≠定义域的同构应用——`mutex_class` 语义正确不代表验证了经验有效，wverify alpha 经验有效不代表验证了语义正确）。

## 6. 影响声明

- 本工位纯只读审计 + Codex CLI 调用，未修改任何生产代码。
- 产出：本裁定文件 + Codex 完整交互记录（`.chanlun/review-results/codex-decide-20260702-235449-5f01.md`）。
- 下游消费：p2 D1 实装工位（可据本裁定的 `predicates_of` 修正签名 + shadow-fold 测试方案直接实装）；Q2 清理工位（删除 `closed_loop/mutex_interp.rs::choose_action` 及其测试，或按诚实降级路径处理，二选一由实施工位执行，不需要再上浮）。

---

```yaml
---stance-declaration---
verdict: settled
review_target: full-strategy-pi p2 D1收窄版 + Q2双零消费者去留
stances:
  d1_narrowed_equivalence_well_defined: confirmed_with_condition
  d1_predicates_of_signature_needs_opened_slots: confirmed
  d1_p7_same_dir_record_add_semantic_collapse: confirmed_honest_gap
  d1_property_test_shadow_fold_required: confirmed
  d1_bucket_level_vs_exact_cj_level_distinction: confirmed
  d1_p1_risk_liquidate_out_of_scope: confirmed
  d1_authority_order_pdf_gt_interp_gt_mutex_class: confirmed
  q2_delete_choose_action: confirmed
  q2_keep_mutex_class_as_sole_oracle: confirmed
  q2_9_predicate_vs_8_predicate_divergence_source: lean_finer_grain_not_1to1_with_pdf
escalation_needed: []
concessions:
  - "若团队要保留 choose_action 做 Lean 内部一致性 fixture，需明示文档声明与生产解释器无关——实施工位可在删除/降级间二选一，不需再上浮"
---end-stance---
```
