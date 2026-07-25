# Code Review — Issue #112（进场门 multi-level 消费切换）

- 日期：2026-07-21
- 评审输入：`gh issue view 112` / `gh issue view 109` / `chanlun/review-results/issue112-impl-20260721.md` / `git diff HEAD~1 --stat`
- 方式：两轴（Standards + Spec），只读评审（未运行 cargo，未做任何 git 变更）

## Verdict: **PASS WITH CONDITIONS**

## 轴一：Spec 符合性（对照 #112 / #109 与实现报告）

| 声明 | 核实 | 结论 |
|---|---|---|
| 进场门消费源切换为 `typed_lookup_multi`（ℓ≥origin+1 多级投影，值桥 `source_index == seg_c_full.1`，side 同向，`judge_at ≤ anchor` 因果守卫） | `fill.rs:805-838` 经 `gate.admit(...)` 消费；`runner.rs` #112-T1（:2711）证明旧 fixed ℓ=1 miss 时 multi ℓ=2 命中成为唯一判定源，`obs.typed_hit_levels == vec![2]` | ✅ |
| 旧 fixed 路径退役为 comparison-only（`single_multi_divergence` 对照列，不进判定） | #112-T3（:2779）：`single_admit=None` 落对照列、`multi_admit=Some(true)` 落判定列、divergence 计数 +1；single 不反向影响准入 | ✅ |
| 因果守卫：越过 anchor 的证书不得被消费，剔除后走既有 Xzd fallback | #112-T2（:2739）：夹具自证旧 fixed 会读到未来证书（`Some((true,0))`）而 multi 守卫剔除（None）；channel 落入 `xzd_pass|xzd_gate_fail`，`obs.xzd_fallback=true` | ✅ |
| 索引惰性重建前置同步切换为 multi 键谓词，决策逐字节不变 | `fill.rs:811-827`：前置改 `has_bridge_key_multi(c.source_index, delta)`（by_end_multi 与 by_end 同点增量维护，O(1)）；#112-T4（:2804）证明键谓词 false ⟹ `typed_lookup_multi` 必 None（用另一在账键排除“空 gate”伪覆盖） | ✅ |
| 门关闭（`nest_gate_hist=None`）⟹ 直通逐字节不变 | `fill.rs:838` None 臂原样返回，bit-exact 回归锁注释在位 | ✅ |
| 出场侧（#113 范围）本票不动，保持 fixed lookup | `admission.rs:1018` `reverse_admit` 仍消费 `typed_lookup`，`sync_bar` 前置仍用 `has_bridge_key`（:993）——代码符合；但见条件 C1（文档已超前写成 #113 口径） | ⚠️ |

生产调用点全仓复核：`has_bridge_key_multi` 生产消费仅 `fill.rs:823`（进场），`has_bridge_key`（fixed）仅 `admission.rs:989/993`（出场）——消费面切割与票面范围一致，无第二查法。

## 轴二：Standards

- 单一权威源纪律保持：判定仍单源 `cert.certificate().n_delta()`（nest.rs 递归核）；L2 旧臂/single 均为对照读出，不进判定。✅
- 拒绝不经 μ 桶键、ext_i/entry_z 三维不动（R5-1 铁律）注释与代码路径一致。✅
- 测试质量：T1-T4 各含夹具前提自证断言（如 T2 先证旧 fixed 会读未来证书，再证 multi 剔除），非空转测试。✅
- 文档：`fill.rs` 门区注释与实现同步更新。`admission.rs` 出场侧文档见 C1。⚠️

## Conditions（不阻断合入，需跟进）

- **C1（文档超前 / 疑似 #113 范围渗入）**：`admission.rs:944-947` 已写「#113 起与 #112 进场门同一多级投影层……旧固定路径退役为 comparison-only」，:963-965 已引入 `anchor` 字段并写 #113 因果守卫注释，但 `reverse_admit`（:1018）仍消费 fixed `typed_lookup`，`anchor` 在出场路径未见消费。若为前瞻性文档，须显式标注“#113 未落地”；否则 #113 落地时同票清理，避免文档与判定源口径背离。
- **C2（惰性重建前置的保守性证明）**：`fill.rs:822` 前置为 `classification_i.levels.get(c.level).is_some() && has_bridge_key_multi(...)`。T4 已证 `has_bridge_key_multi=false ⟹ multi 必 None`；但 `levels.get(c.level)` 这一合取项以候选级别为判据，而 multi 查询覆盖 ℓ≥origin+1 多级——需在 #113 或后续回归中补一条“级别缺位但深层可命中”方向的反例测试（或注释给出不可能性论证），锁死“跳过重建永不造成 false-negative”的不变量。本次未运行 cargo，依据既有 CI 记录与 T4 判定为可接受。

## 结论

进场门 multi-level 消费切换实现与 #112 票面逐条对齐，因果守卫、对照落账、惰性重建前置均有针对性测试自证；出场侧代码未越界。带 C1/C2 两条件通过。
