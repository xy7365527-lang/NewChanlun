# codex 裁定：A4→A5 力度签名 SubMovePower 分量处置（q3 D-1 残余）

- 日期：2026-07-03（文件名沿 Task #149 指定 20260704）
- 裁定源：codex exec（read-only sandbox，model_reasoning_effort=low；首次默认档 10min 超时，重试成功）
- 提问包：/tmp/codex-a4-prompt.md（证据=原文 §5 p6 𝒜_ℓ 全族 + divergence.rs/signal.rs 现状 + f92a/beta-bucket-design 前判例 + no-patch/231号铁律）
- 同轮已实装（非本裁定范围）：TV=Σ|P_{t+1}−P_t| 真接入 ForceFeatures/force_state，ForceStateA4 升名 ForceStateA5。

## 裁定原文（verbatim）

裁定：选 (b) 登记诚实缺口。

严格形式：对 ℓ 级走势段 `s`，`SubMovePower_ℓ(s)=Σ m_{ℓ-1}(u)`，其中 `u ∈ descend_leveled(s)` 且 `dir(u)=dir(s)`；基例 ℓ=0 无次级别，取 0 或不纳入完整族；递归步要求 `m_{ℓ-1}` 已是下一级同一力度定义的已结算值，而不是临时用 MACD/TV 代替。

理由：当前 `force_features(hist,dif,closes,start,end,dir)` 是段坐标纯函数，只能生产 MACD/DIF/振幅/速度/TV。`SubMovePower` 需要 `LeveledMove.sub_moves` 与逐级 `m_{ℓ-1}`，数据在递归塔层，不在 `signal.rs` 一类候选构造口。现在加字段只能造死字段或假 proxy，违 no-patch；称完整 `ForceState` 也违 231。`ForceStateA5` 后缀 + doc 标注缺递归分量 + 证明义务登记，是当前严格形式。

独立工位前置与验收：

1. 前置：A/C 段必须可映射到对应 `LeveledMove`，不是仅有 `Segment` 坐标。
2. 前置：定义并实现 `m_ℓ` 基例与递归步，方向过滤、空 descend、级别终止明确。
3. 验收：`ForceFeatures` 增 `sub_move_power` 且无 `Option` 死字段、无 0 占位冒充已算。
4. 验收：`signal.rs` 力度管线从递归塔透传 A/C 的 `LeveledMove`，测试覆盖 L0 基例、L1 三子段求和、方向过滤、A/C 支配态翻转/变 Incomparable。
5. 验收：通过后才允许 `ForceStateA5 → ForceState`。
