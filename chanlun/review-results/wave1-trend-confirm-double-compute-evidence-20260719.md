# wave-1 证据核验：trend_confirm_time 双算（2026-07-19）

**性质**：优化 wave-1「单源化」方向的关键事实核验——双算是否真实存在、输入是否逐参一致、规模如何计量。纯只读核验，零代码改动。

## 1. 双算事实（两处调用点，同一纯函数）

`trend_confirm_time`（`level_view.rs:673-684`）是**纯函数**（无全局态读写；p126_split 计数器为只增诊断，不影响返回值）。同一输入 ⟹ 同一输出。

| 调用点 | 位置 | 上下文 | 计数器 |
|---|---|---|---|
| **A（assemble 侧）** | `level_view.rs:1279` | `assemble_level_view` 判定 Trend 块 `CompletionStatus`（Completed/Pending）时调用 | `p126_split::n_asm_confirm` / `t_asm_confirm` |
| **B（provide 侧）** | `level_view.rs:874` | `provide_nest_candidate_events` 对过 `extreme` 门的 pair 产出 `NestCandidateEvent` 时调用 | `p126_split::n_tr_confirm` / `t_tr_confirm` |

## 2. 输入逐参对照（同一 (projection, pair, as_of)）

| 参数 | A（:1279） | B（:874） | 一致性 |
|---|---|---|---|
| `segments` | `&segments`（投影段序列） | `&segments`（同一投影段序列） | ✓ 同一 |
| `last` | `&projection.seeds[block.end_center].center` | `&projection.seeds[pair.id.block_end_center].center` | ✓ 同一 seed |
| `direction` | `block.dir.expect(...)` | `pair.id.direction` | ✓ 同一 pair |
| `side` | `Direction→Side` 同一映射 | 同 | ✓ |
| `seg_a` | `pair.seg_a` | `pair.seg_a` | ✓ 同一 |
| `c_start` | `pair.seg_c.0` | `pair.seg_c.0` | ✓ 同一 |
| `as_of` | `query.as_of` | `view.query.as_of` | ✓ 同一（provide 消费 assemble 产出的 view） |
| `hist/dif/close_src` | `material.hist/dif/close_src` | 同源传入 | ✓ 同一 |

**结论**：对同时经过 A（CompletionStatus 判定）与 B（事件产出）两条路径的 pair，`trend_confirm_time` 被**逐字同输入调用两次**——双算确凿，消除 B 侧重算不改任何语义（纯函数结果复用）。

## 3. 交集规模（单源化的真实收益域）

- A 侧调用域：Trend 块的 **CompletionStatus 判定**（特定末 pair）。
- B 侧调用域：**全部过 `extreme` 门的 Trend pair**。
- 双算收益域 = A∩B 交集（A 已算且 B 也要算的 pair）；B\A 部分（assemble 未判定的 pair）仍须 B 自算。
- **规模计量工具已就位**：`p126_split::n_asm_confirm`（A 侧次数）与 `n_tr_confirm`（B 侧次数）——下一次全量/截段重放的 stderr 计数可直接读出两侧调用规模与耗时占比（`t_asm_confirm` / `t_tr_confirm`），单源化收益 = `t_tr_confirm` 中属交集的部分。**实装前先用计数器定量交集，不拍收益数字（090）**。

## 4. 单源化候选形态（登记，不定稿）

- **形态 i（结果携带）**：`CompletionStatus::Completed` / `Pending` 的 evidence 变体携带 `confirm_t: Option<usize>`（A 侧已算结果），B 侧消费 view 时直接读——零新判据路径，语义逐项一致。
- **形态 ii（memo 键）**：以 `(projection 哨兵, pair.id, as_of)` 为键在 view 评估内 memo 一次——侵入更小，但引入键生命周期管理。
- **纪律**：两形态都属「生产源码计算顺序/缓存改动」，判定逻辑零改动；实装后 `cargo test --lib` 全绿 + 截段双跑 diff=0 为验收线（与 p123/p124 同协议）。

## 5. 关联证据

- p122 Phase 0：L1 级 view 税占 82%（16,763 views × 76.7ms@2M）；`trend_confirm_time` 是 view 内最重的单函数之一（T4/T3/T2/T5 四段扫描 + 渐进窗口）。
- p123 模块头「trend_confirm 游标驻留未实装」登记：函数**内部**的 per-pair 游标（env 包络/acc_hi/area/dif/hist 极值）跨评估驻留需改生产源码——与本双算是**两个独立的优化点**（函数间结果复用 vs 函数内游标驻留），可分别立项。
