# #1346 现行代码对照（基线 main `1f5cdf76`）

| 谓词/对象 | 生产位置（文件:行） | 与已裁是否一致 | 偏差 → 实施义务（具名票） |
|---|---|---|---|
| Dir | `rust/src/theta_v0/classifier/cand_predicate.rs:420-428`（`div_cand_fail` 条件 1）；Lean `NestingCertificate.lean:151-153` | 一致 | — |
| Comparable | `cand_predicate.rs:430-443`（条件 2，父末中枢为界 + `d3_prev_crossing_anchor` `:349-361`，#1265 全仓唯一取段原语）；Lean `:95-141` `Comparison` 四入口分派 | 一致（#1245 → #814 D-3 → #1231/#1265 链已把「最近同向前驱」私货退役；`classifier/mod.rs` 的同款派生经 grep `最近同向` 在当前 main 零命中） | — |
| Extreme | `cand_predicate.rs:445-452`；Lean `:147-149` | 一致（趋势/盘整两支；Type2/3 入口由 #1347 分派） | — |
| 元素谓词三条件独立可用 | **缺**：Rust 只有四条件 `div_cand`（`:384-386`）与 `div_cand_fail`（`:395`），Weak 在 `:454-470` 合在同一函数 | **偏差 I-1**：三条件未单独成函数；qujiantao §9 清单 46 已登记为 SPEC #847 项 | TB-02-Q #1439：拆 `div_cand_struct(input) -> Option<u8 ∈ {1,2,3}>`，Weak 作独立门 `G`；`econ_positive.rs` 的 `acc_*` 报告自陈「四条件」同改 |
| 生产真跑的三条件（承重） | `classifier/level_view/pan_provider.rs:248-266`（经 `backtest/admission.rs:853` ← `fill.rs:4935`） | 一致（qujiantao §6.2） | 登记不改 |
| 条件化（J ⊆ I(c)） | `classifier/nest.rs:992-996` `is_sub(child,&parent)`（排序后循环内过滤）；Lean `:232-234` `eligibleCandidates` | 一致 | — |
| 条件化（strategy 侧） | `strategy/nest.rs:99-101` `chosen()` 只吃 `cands` 不吃 J | **偏差 I-4**（PDF p5 反例；qujiantao §9 清单 49） | TB-02-Q：`chosen(J)` 签名条件化；或随 TB-10 旧入口退役（`interp.rs:580-586` 链恒单元素） |
| 集合非空门 Cand | Lean `:246-247`；Rust 无显式对象（隐含于「循环找到任一 rung」） | **偏差 I-6**：Rust 无 `C(J,t)` 集合对象与基数分区输出（CC-049） | TB-02-Q：物化 `conditioned_set{members, cardinality}` |
| Sel_Θ 序 | `strategy/nest.rs:55-59`、`classifier/nest.rs:63-67`、Lean `Strict/Nest.lean:117-120` | 三处逐字同向（end 最新→start 最新→idx 最小） | — |
| Sel_Θ 应用方向 | `classifier/nest.rs:978-985` `sort_by_key(sel_key)` **升序**（取 end 最早）；`:1250-1258` 按 parent (start,end) 升序 | **偏差 I-2**（#778 连带动作 1 未落） | TB-02-Q：改为按 `sel_order` 降序取首（或 `select_best` 于过滤后集合）；对拍锁：固定输入下 selected 与 Lean `selOrder` 极值一致 |
| Sel_Θ Lean 形态 | `NestingCertificate.lean:239-240` `SelTheta = head?`（对输入序敏感） | **偏差 I-5**：与 `Strict/Nest.lean` 的键序 `selOrder` 不同形 | Lean 跟进（TB-02-Q 或 #1225 后续）：改为 `eligibleCandidates` 上按 `selOrder` 取极值，或把「输入已按 selOrder 排序」写成定理前件 |
| selected 身份与归因 | `classifier/nest.rs:146` `selected_keys()`；S 层 `selected_from` 关系已声明（`s_session/s_tb02_contract.py:890`、`browser/tb01c-client.js:446`）但无生产者 | **偏差 I-3** | TB-02-Q：S 发布 `selected_from` 边与 `selector_version` |
| selected gate（G） | `admission.rs:500` `strict_chain_verdict`（严档）；力度门 `confirm_divergence_l`（`cand_predicate.rs:454-470` 内） | 严档一致；力度门位置错（见 I-1） | TB-02-Q + TB-04（#1362 下主动定位准入叶）：G 的 predicate_id 进 `production_selected_result` |
| 全候选只读观察轴 | 无 | **偏差 I-7**（CC-052 轴 2） | TB-02-Q：`readonly_all_candidate_result`，不进准入 |
| 只收 Closed | `admission.rs:500` 严档 Closed ∧ 零跳；`classifier/nest.rs` 装配读已判事件 | 一致 | — |
| 同版本回放 | 无 `selector_version` 字段 | **偏差 I-8**（AT-14） | TB-07 #1366 下叶：selector_version 进快照/AsKnown |
| R2 owner | `rust/src/economic_session/*`、`s_session/*`：无候选/Sel/gate 对象 | 语义不在 R2，owner 按 ADR 0027 归结构判定域 | TB-02-Q 为唯一实装入口；旧 `strategy/nest.rs`、`interp.rs:580-586` 归 TB-10 收编/退役 |

偏差 I-1～I-8 全部是**实施缺口**（已裁语义未落或落错方向），不是语义空缺。
