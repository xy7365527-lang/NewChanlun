# 裁定4 补记：独立第二次 Codex 裁定 vs 已落盘裁定4 的收敛/分歧点

- **工位**：ws-codexq2（team-lead 直接指派"裁定4"，指派时表述为"至今无人认领"）。
- **发现**：开工前未查文件系统，产出后核对发现 `.chanlun/review-results/codex-q1-spec-rulings-20260703.md` 的"裁定4"节**已被另一并行工位（很可能是同一个 ws-codexq1spec 或其延续）用独立的 Codex decide() 调用完成并落盘**（`codex-decide-20260703-032539-a034.md`，日志时间戳早于我这轮）——与本次任务分配时"至今无人认领"的描述不符，是本 session 第二次撞车（第一次见 #122 主三项裁定；memory `feedback_task_queue_owner_liveness.md` 第五例已记录同类模式，本次是第六例）。
- **处理**：不覆盖已落盘的裁定4 正文（避免销毁并行工位的合法产出），本文件作补记/交叉引用，供 team-lead/ws-g5interp 决定是否需要调和。
- **我的独立裁定**：`.chanlun/review-results/codex-decide-20260703-033335-8cd9.md`（context-file: 见本次调用记录，独立构建，未参考对方产出）。

## 1. 收敛点（两次独立 Codex 裁定一致）

- P1（强平）不能实现成"候选归属某个桶"，必须能在候选集为空时也强制清仓——两次裁定都明确指出这一点。
- 拒绝"分层 output-等价"（只扩 `mutex.rs` oracle 做 property test，不改真实订单流/账本流）——两次都判定这是装饰性 oracle，不构成真实统一。
- P2（CloseOverlay）必须产生真实订单/close 效果，不能继续留在与生产 π 订单流 disjoint 的 `closed_loop` 玩具引擎里。
- 三者（P1/TW 统一、G4 typed exit、G7 毛头寸声部级约束）耦合于同一个更大的解释器/仓位接口重构，可分阶段落地。

## 2. 我这轮核验到但已落盘裁定4正文未提及的具体代码缺陷（建议补入正文，不依赖分歧解决）

**幽灵腿（ghost leg）具体反例**——`coverage.rs:1710-1739`（`coverage_step_from_buckets`）：

```rust
for c in &buckets.open {
    let idx = candidate_start + c.gamma_index;
    if idx < work.len() && !raw.contains(&idx) {
        raw.push(idx);   // ← 无条件推入，不检查 KThetaRiskGate
    }
    // ...父链恢复逻辑同样无条件...
}
```

`buckets.open` 由 `interp::interpret`（不知道风险门）产生，本函数无条件把它折进 `raw`→`next_idx`→`next_active`（成为下一 bar 的 `prev_active`）。**这个过程完全不检查 `force_flat`**。具体反例：某 bar `force_flat=true`，同 bar 恰有新候选命中 P8 Open Root（`interpret` 不知道 force_flat，正常放入 `buckets.open`）——则 `next_active`（下一 bar 的 `prev_active`）**会包含这个"新开"的腿**，即使同一 bar `p_star=pi_theta_position(...,gate)` 被 force_flat clamp 为 0（实际订单是 Close/Hold，这个腿从未真实建仓）。这个分叉跨 bar 传播：下一 bar 这个幽灵腿继续参与 `reverse_signal` 关闭匹配、`strategy_target_legs` 深度加权仓位计算、子声部 `AncOK` 祖先闭合判定（若它是某 ShortDiff 子声部的父 carrier，子声部会因这个从未真实建仓的父在场而被误判祖先齐全，错误准入）。

这不是"两种架构选一种"层面的分歧——**无论最终选哪种统一方案，这个具体的无条件 push 都必须被堵住**，否则 P1 强平在任何实装形式下都无法真正清空持仓簿记状态。建议实装工位（#124/#133）无论采纳哪个方向都先修这一点。

## 3. 真实分歧点（两次独立 Codex 裁定不一致，需要人工/第三方仲裁，我不擅自选边）

**分歧A：`interp::interpret` 的 Rust 函数签名是否需要真正扩展**

- 已落盘裁定4：解释器接口整体升级为 `I_Θ(ctx: RiskState+TwState+Active/LegBook, gamma) -> {buckets, order_effect, tw_event, exit_kind}`——语言比较抽象，未指名是否要求 `interp::interpret` 这个具体 Rust 函数改签名。
- 我的裁定：明确要求 `interp::interpret(gamma, active) -> Buckets` **保持签名不变**，风险/TW 投影发生在 `interpret()` 与 `coverage_step_from_buckets` 之间的组合层——理由是 `interpret()` 现有唯一性证明/shadow-fold 测试绑定的数学对象是 `ℛ_Θ(Γ,A)`，不应擅自扩大这个已证明对象的定义域（231号：不应偷换已证明对象）。

  这一点实际分歧可能较小（已落盘裁定4没有明确到 Rust 函数级别，我的裁定更精确到实现细节），但**实装时必须二选一**：#124 G5-impl 工位如果直接照抽象的 `I_Θ(ctx,gamma)` 字面实装，很可能真的会改 `interp::interpret` 的签名——这会破坏现有 `interp.rs:940-999` 的证明文档（唯一性 ∃!、theta_key 相关注释）而不自知。建议 #124 落地前明确选边。

**分歧B：P3/P4（TW 无订单事件）是否应该"消耗本步解释器裁决"（阻塞/推迟当步普通候选处理）**

- 已落盘裁定4：P3/P4 成立时"应当消耗本步解释器裁决"（即普通候选的开/平/记录动作这一步被屏蔽或推迟到记录桶），不能与普通候选处理并行不冲突地各走各的。
- 我的裁定：P3/P4 是**纯只读输出通道**——只读 `TwState`/`RiskPolicy`/`RiskMode` 产生 `TwEvent`，**不影响** `D`（close 桶）/`O`（open 桶）本身。

  这是一个真实的行为分歧，不是术语差异：按已落盘裁定4，当 TW 处于"可退本金"（P3）或"可增股"（P4）状态时，当步的正规买卖点候选处理会被屏蔽/推迟；按我的裁定，两者完全独立，TW 相变和候选处理各自正常进行，互不干扰。**这个分歧直接影响实装行为，不能含糊过去**——建议：这个具体问题追加一次针对性的 Codex/Gemini 第二意见 tie-break（明确只问这一点，附两次裁定的具体分歧文本），或由 team-lead 依据 PDF §7 优先级链的字面语义直接裁定（PDF §7 P3/P4 排在 P5-P10 之前，若严格按"固定优先级+互斥化 C_j=P_j∧⋀_{k<j}¬P_k"字面语义，P3/P4 成立时确实应该屏蔽 P5-P10——这似乎支持已落盘裁定4 的"消耗当步裁决"读法，而非我的"纯只读通道"读法；我这轮的裁定在这一点上可能是我自己论证不够严格处，值得指出而非隐藏）。

## 4. 建议处理

1. `coverage.rs:1710-1739` 幽灵腿缺陷（§2）：建议直接补入已落盘裁定4正文（无争议，任何架构方向都需要），不需要额外裁定。
2. 分歧A（interpret 签名）：#124 实装前明确选边，倾向保留现有签名+组合层投影（更小改动、不破坏现有证明），除非有理由认为必须真正扩展底层函数。
3. 分歧B（P3/P4 是否屏蔽当步候选）：**倾向修正为"消耗本步解释器裁决"（已落盘裁定4的读法）**——重新核对 PDF §7 优先级链字面语义（`C_j=P_j∧⋀_{k<j}¬P_k`，P3/P4 在 P5-P10 之前）后，我认为我这轮裁定在这一点上对"只读不影响 D/O"的论证不足，已落盘裁定4 更贴合 PDF 字面互斥化结构。此处不擅自改已落盘正文，留给 team-lead/#124 实装工位定夺。

## 影响声明

本文件为补记/交叉引用，不修改已落盘 `codex-q1-spec-rulings-20260703.md` 正文。核验到的幽灵腿具体代码缺陷（`coverage.rs:1710-1739`）建议后续由 #124/#133 实装工位处理时一并纳入测试覆盖（无论最终选哪个架构方向）。
