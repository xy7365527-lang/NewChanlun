"""不变量定义 — 29 条运行时规则

I1-I5：笔层不变量，由 InvariantChecker 执行检查。
I6-I10：线段层不变量（MVP-B1），由 SegmentInvariantChecker 执行检查。
I11-I15：中枢层不变量（MVP-C0），由 ZhongshuInvariantChecker 执行检查。
I16-I17：系统级不变量（PR-C0.5），跨层 diff 身份规范。
I18-I22：走势类型层不变量（MVP-D0），由 MoveInvariantChecker 执行检查。
I23-I29：买卖点层不变量（MVP-E0），由 BspInvariantChecker 执行检查。

I1: SETTLED_OVERWRITE — settled stroke 不可被覆盖
    同一 (i0, i1, direction) 的 stroke_settled 只能出现一次，
    除非先收到对应的 stroke_invalidated。

I2: TIME_BACKWARD — 事件时间戳必须单调非递减
    同 TF 内，相邻 bar 的 bar_ts 不允许回退。

I3: DUPLICATE_SETTLE — diff 不产生重复 settle
    同一 (i0, i1, direction) 不能被 settle 两次而中间无 invalidate。
    （与 I1 检测同一违规但从 diff 视角表述）

I4: TYPE_MISMATCH — candidate/settled 类型边界不混淆
    confirmed=True 的 Stroke 必须对应 settled 事件，
    confirmed=False 的 Stroke 必须对应 candidate 事件。
    （此规则由 bi_differ 逻辑保证，checker 做事后验证）

I5: REPLAY_DETERMINISM — 回放确定性
    同输入 → 同 event_id + 同 payload + 同顺序。
    （此规则由 test_replay_determinism 和 test_event_immutability 验证，
     checker 不做实时检查）

I6: PENDING_DIRECT_SETTLE — pending_break 不得直接 settle
    SegmentSettleV1 之前（同 segment_id 同批次内），
    必须先有对应的 SegmentBreakPendingV1。

I7: SETTLE_ANCHOR — 结算锚 old_end=k-1, new_start=k
    SegmentSettleV1 的 s1 必须等于 break_at_stroke - 1，
    new_segment_s0 必须等于 break_at_stroke。

I8: GAP_NEEDS_SEQ2 — gap 必须经历第二序列分型
    gap_class="gap" 的 SegmentSettleV1 须由 v1 算法内部
    的第二特征序列分型确认。（由算法保证，checker 做审计）

I9: INVALIDATE_IDEMPOTENT — invalidate 传播幂等
    同一 (s0, s1, direction) 不能被 invalidate 两次
    而中间无 settle。

I10: SEGMENT_REPLAY_DETERMINISM — 线段回放确定性
    同输入 → 同 segment event_id + payload + 顺序。
    （由测试覆盖，checker 不做实时检查）

I16: IDENTITY_PRESERVING_NO_INVALIDATE — 身份保持更新不得产生 invalidate
    若 diff 前后列表同位元素具有相同身份键（segment: (s0, direction),
    zhongshu: (zd, zg, seg_start)），diff 不得为旧元素产生 Invalidate 事件，
    只允许产生升级/更新事件。（由 diff 算法保证，由测试覆盖）

I17: INVALIDATE_IS_TERMINAL — invalidate 是终态
    同一 identity key 的实体一旦被 invalidate，不得再出现后续
    Candidate/Settle/BreakPending 事件。由 checker 运行时检查。

I18: MOVE_MIN_CENTER — move 至少包含 1 个中枢
    MoveCandidateV1.zs_count >= 1。

I19: MOVE_CANDIDATE_BEFORE_SETTLE — settle 前必有 candidate
    MoveSettleV1 之前（同 move_id 同批次内或历史中），
    必须先有对应的 MoveCandidateV1。

I20: MOVE_PARENTS_TRACEABLE — 中枢索引可回溯
    zs_end >= zs_start 且 zs_count >= 1。

I21: MOVE_INVALIDATE_TERMINAL — invalidate 是终态（I17 在 Move 层的应用）
    同一 identity key (seg_start) 一旦被 invalidate，不得再出现后续事件。

I22: MOVE_REPLAY_DETERMINISM — 走势类型回放确定性
    同输入 → 同 move event_id + payload + 顺序。
    （由测试覆盖，checker 不做实时检查）

I23: BSP_TYPE_CONSTRAINT — 买卖点类型约束
    type1/type2 的 divergence_key 不可为 None（关联趋势背驰）。
    type3 的 divergence_key 必须为 None。

I24: BSP_CANDIDATE_BEFORE_CONFIRM — confirm 前必有 candidate
    BuySellPointConfirmV1 之前（同 bsp_id），
    必须先有对应的 BuySellPointCandidateV1。

I25: BSP_CONFIRM_BEFORE_SETTLE — settle 前必有 confirm
    BuySellPointSettleV1 之前（同 bsp_id），
    必须先有对应的 BuySellPointConfirmV1。

I26: BSP_MUTUAL_EXCLUSION — 买卖点互斥
    同 level_id 下，不存在同时活跃的 (type1, seg_idx=S)
    和 (type2, seg_idx=S) 或 (type1, seg_idx=S) 和 (type3, seg_idx=S)。
    允许 (type2, seg_idx=S) 和 (type3, seg_idx=S) 共存（2B+3B 重合）。

I27: BSP_INVALIDATE_TERMINAL — invalidate 是终态
    同身份键 (seg_idx, kind, side, level_id) 一旦被 invalidate，
    不得再出现后续 Candidate/Confirm/Settle 事件。

I28: BSP_REPLAY_DETERMINISM — 买卖点回放确定性
    同输入 → 同 bsp event_id + payload + 顺序。
    （由测试覆盖，checker 不做实时检查）

I29: BSP_PRICE_MONOTONE — 买卖点价格约束
    type1 buy price ≤ 关联中枢 ZD；type3 buy price > 关联中枢 ZG。
    （待确认时机结算后启用）
"""

# ── 笔层不变量 code ──
I1_SETTLED_OVERWRITE = "I1_SETTLED_OVERWRITE"
I2_TIME_BACKWARD = "I2_TIME_BACKWARD"
I3_DUPLICATE_SETTLE = "I3_DUPLICATE_SETTLE"
I4_TYPE_MISMATCH = "I4_TYPE_MISMATCH"
I5_REPLAY_DETERMINISM = "I5_REPLAY_DETERMINISM"

# ── 线段层不变量 code（MVP-B1）──
I6_PENDING_DIRECT_SETTLE = "I6_PENDING_DIRECT_SETTLE"
I7_SETTLE_ANCHOR = "I7_SETTLE_ANCHOR"
I8_GAP_NEEDS_SEQ2 = "I8_GAP_NEEDS_SEQ2"
I9_INVALIDATE_IDEMPOTENT = "I9_INVALIDATE_IDEMPOTENT"
I10_SEGMENT_REPLAY_DETERMINISM = "I10_SEGMENT_REPLAY_DETERMINISM"

# ── 中枢层不变量 code（MVP-C0）──
I11_ZHONGSHU_OVERLAP = "I11_ZHONGSHU_OVERLAP"
I12_CANDIDATE_BEFORE_SETTLE = "I12_CANDIDATE_BEFORE_SETTLE"
I13_PARENTS_TRACEABLE = "I13_PARENTS_TRACEABLE"
I14_ZHONGSHU_INVALIDATE_IDEMPOTENT = "I14_ZHONGSHU_INVALIDATE_IDEMPOTENT"
I15_ZHONGSHU_REPLAY_DETERMINISM = "I15_ZHONGSHU_REPLAY_DETERMINISM"

# ── 系统级不变量 code（PR-C0.5）──
I16_IDENTITY_PRESERVING_NO_INVALIDATE = "I16_IDENTITY_PRESERVING_NO_INVALIDATE"
I17_INVALIDATE_IS_TERMINAL = "I17_INVALIDATE_IS_TERMINAL"

# ── 走势类型层不变量 code（MVP-D0）──
I18_MOVE_MIN_CENTER = "I18_MOVE_MIN_CENTER"
I19_MOVE_CANDIDATE_BEFORE_SETTLE = "I19_MOVE_CANDIDATE_BEFORE_SETTLE"
I20_MOVE_PARENTS_TRACEABLE = "I20_MOVE_PARENTS_TRACEABLE"
I21_MOVE_INVALIDATE_TERMINAL = "I21_MOVE_INVALIDATE_TERMINAL"
I22_MOVE_REPLAY_DETERMINISM = "I22_MOVE_REPLAY_DETERMINISM"

# ── 买卖点层不变量 code（MVP-E0）──
I23_BSP_TYPE_CONSTRAINT = "I23_BSP_TYPE_CONSTRAINT"
I24_BSP_CANDIDATE_BEFORE_CONFIRM = "I24_BSP_CANDIDATE_BEFORE_CONFIRM"
I25_BSP_CONFIRM_BEFORE_SETTLE = "I25_BSP_CONFIRM_BEFORE_SETTLE"
I26_BSP_MUTUAL_EXCLUSION = "I26_BSP_MUTUAL_EXCLUSION"
I27_BSP_INVALIDATE_TERMINAL = "I27_BSP_INVALIDATE_TERMINAL"
I28_BSP_REPLAY_DETERMINISM = "I28_BSP_REPLAY_DETERMINISM"
I29_BSP_PRICE_MONOTONE = "I29_BSP_PRICE_MONOTONE"

# code → 描述映射
INVARIANT_DESCRIPTIONS: dict[str, str] = {
    I1_SETTLED_OVERWRITE: "settled stroke 被覆盖（无先行 invalidate）",
    I2_TIME_BACKWARD: "事件时间戳回退（bar_ts 非递减违规）",
    I3_DUPLICATE_SETTLE: "同一笔被重复 settle",
    I4_TYPE_MISMATCH: "confirmed 状态与事件类型不匹配",
    I5_REPLAY_DETERMINISM: "回放事件序列不确定",
    I6_PENDING_DIRECT_SETTLE: "segment settle 前无 pending_break",
    I7_SETTLE_ANCHOR: "结算锚违规（old_end != k-1 或 new_start != k）",
    I8_GAP_NEEDS_SEQ2: "gap 类 settle 未经第二序列分型确认",
    I9_INVALIDATE_IDEMPOTENT: "segment invalidate 非幂等（重复否定）",
    I10_SEGMENT_REPLAY_DETERMINISM: "线段回放事件序列不确定",
    I11_ZHONGSHU_OVERLAP: "中枢 candidate 的 ZG <= ZD（重叠不成立）",
    I12_CANDIDATE_BEFORE_SETTLE: "中枢 settle 前无对应 candidate",
    I13_PARENTS_TRACEABLE: "中枢事件的 seg_start/seg_end 无法回溯到已确认段",
    I14_ZHONGSHU_INVALIDATE_IDEMPOTENT: "中枢 invalidate 非幂等（重复否定）",
    I15_ZHONGSHU_REPLAY_DETERMINISM: "中枢回放事件序列不确定",
    I16_IDENTITY_PRESERVING_NO_INVALIDATE: "身份保持更新产生了 invalidate（应为升级/更新事件）",
    I17_INVALIDATE_IS_TERMINAL: "invalidate 后出现同身份事件（invalidate 应为终态）",
    I18_MOVE_MIN_CENTER: "Move candidate 中枢数量 < 1",
    I19_MOVE_CANDIDATE_BEFORE_SETTLE: "Move settle 前无对应 candidate",
    I20_MOVE_PARENTS_TRACEABLE: "Move 事件的 zs_start/zs_end 无法回溯（zs_end < zs_start 或 zs_count < 1）",
    I21_MOVE_INVALIDATE_TERMINAL: "Move invalidate 后出现同身份事件（应为终态）",
    I22_MOVE_REPLAY_DETERMINISM: "Move 回放事件序列不确定",
    I23_BSP_TYPE_CONSTRAINT: "买卖点类型约束违规（type1/2 缺 divergence_key 或 type3 带 divergence_key）",
    I24_BSP_CANDIDATE_BEFORE_CONFIRM: "买卖点 confirm 前无对应 candidate",
    I25_BSP_CONFIRM_BEFORE_SETTLE: "买卖点 settle 前无对应 confirm",
    I26_BSP_MUTUAL_EXCLUSION: "买卖点互斥违规（同 seg_idx 出现 type1+type2 或 type1+type3）",
    I27_BSP_INVALIDATE_TERMINAL: "买卖点 invalidate 后出现同身份事件（应为终态）",
    I28_BSP_REPLAY_DETERMINISM: "买卖点回放事件序列不确定",
    I29_BSP_PRICE_MONOTONE: "买卖点价格约束违规（type1 buy > ZD 或 type3 buy ≤ ZG）",
}
