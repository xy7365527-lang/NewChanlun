# M8 修复后重跑对照报告（Done when #6，2026-07-19）

**任务**：判定 L1/L2/Gap-1/暴露面实装修复是否足以解释三窗 net_r 全负。
**对照基线**：修复前 m8 首跑报告（p126-stage3-m8e2e-20260719.md）三窗数值 + trades.jsonl 518 行。

## 1. 三窗数值对照（修复前 vs 修复后）

| 窗 | 指标 | 修复前 | 修复后 | 变化 |
|---|---|---|---|---|
| p3fold | execR | −15,383,474 | −15,383,474 | **逐位一致（0）** |
| p3fold | MaxDD | 0.9406 | 0.9406 | 0 |
| p3fold | R | −15,232,144 | −15,232,144 | 0 |
| p3fold | LCB(R) | −18,771,457 | −18,771,457 | 0 |
| p3fold | stage | I(降成本) | I(降成本) | 0 |
| wf7 | execR | −20,995,480 | −20,995,480 | **0** |
| wf7 | MaxDD | 0.8966 | 0.8966 | 0 |
| wf7 | R | −20,835,598 | −20,835,598 | 0 |
| wf7 | LCB(R) | −23,871,127 | −23,871,127 | 0 |
| wf7 | stage | I(降成本) | I(降成本) | 0 |
| wf8 | execR | −17,134,632 | −17,134,632 | **0** |
| wf8 | MaxDD | 0.6490 | 0.6490 | 0 |
| wf8 | R | −16,844,347 | −16,844,347 | 0 |
| wf8 | LCB(R) | −21,097,485 | −21,097,485 | 0 |
| wf8 | stage | I(降成本) | I(降成本) | 0 |

**关键证据**：L1/L2/Gap-1/暴露面全部实装**不改语义、纯观测层**——三窗 6 指标×3 窗共 18 项数值**逐位一致（bit-exact 0 变化）**。这验证了：①修复没引入订单流变化；②修复没破坏既有测试网（cargo test 1737/0 全绿）；③三窗 net_r 全负**不是**这些字段名/观测补全层的问题。

## 2. trades.jsonl 字段拆分对照（518/518 笔）

| 维度 | 修复前 | 修复后 | 变化 | 判定 |
|---|---|---|---|---|
| 行数 | 518 | 518 | 0 | bit-exact |
| trade_id 集合 | 518 唯一 | 518 唯一 | 0 | bit-exact |
| 数值字段（entry_px/exit_px/entry_stop_dist/pnl_raw_unlevered/units/entry_bar/exit_bar） | — | — | **mismatch=0（全部 518×7 字段逐位一致）** | bit-exact |
| units=0.0 笔数 | 64 | 64 | 0 | bit-exact |
| **顶层键：`via_structural_prune`** | 存在（36.1% True） | **删除** | 字段名诚实化（090 修复：36.1% 来自 §13 AncOK silent_drops 而非 econ 门剪枝） | L1-P2 |
| **顶层键：`via_anc_ok_prune`** | 不存在 | **新增**（同 36.1% True） | 字段名诚实化（如实标注来源） | L1-P2 |
| **interpreter_at_entry 键：`gamma_count_chi_filtered`** | 存在（谎报：chi=None 下取值 1-5 暗示已过滤） | **删除** | 090 谎报修复 | L1-P1 |
| **interpreter_at_entry 键：`gamma_count_at_entry`** | 不存在 | **新增**（同值，如实标注为入场时未过滤数） | 字段名诚实化 | L1-P1 |
| **interpreter_at_entry 键：`chi_filter_active`** | 不存在 | **新增**（bool，chi.is_some() 如实标注 χ 是否真在过滤） | L1-P1 观测补全 | L1-P1 |
| **interpreter_at_entry 键：`b1_sizing_available`** | 不存在 | **新增**（518/518 全 true，照实否定「sizing 缺入口」前提） | L2-P4 观测补全 | L2-P4 |
| **voice_tree_at_entry 键：`depth`** | 不存在 | **新增**（分布 {0:207,1:146,2:101,3:44,4:20}，depth==nest_depth 518/518 全等） | L2-P5 观测补全 | L2-P5 |
| **voice_tree_at_entry 键：`w_depth`** | 不存在 | **新增**（518/518 零违例 == depth_weights[depth]，w_depth=0 ⟺ units=0 双向零违例） | L2-P6 60/30/10 生效可证 | L2-P6 |

## 3. 端到端判定

**修复是否足以解释 net_r 全负？否（不充分）。**

理由：
1. **L1/L2/Gap-1/暴露面全部修复是 bit-exact 观测层**——三窗数值逐位一致（0 变化），说明这些修复**消除了 090 谎报与观测盲区**（字段名诚实化、60/30/10 生效可证、sizing 缺口否定），但**不改变订单流**，因此**不改变 net_r**。
2. **net_r 全负的机制根因仍未触**：swarm L3 探针（l3-econ-gate-filter-rate-20260719.md）发现 **χ 在当前量纲下退化为方向门**（dir 键拒 45.2% 全部 Short、level 键拒 100% 含全盈利 L3）——χ 接入需要**量纲裁定**（L0 前置），不在本波修复范围。Nest/Xzd 准入同样**无独立鉴别依据**（负贡献与 Short/L1 共线），接入需 econ 门过滤率实测（L3）。
3. **signal 层 INCONCLUSIVE 独立并存**：即便修复全部实装偏离，signal 无 confirmed alpha 仍会传导为端到端 INCONCLUSIVE（措辞§5.3 不外推 max-full）。Gap-1 文本回填只是把写死的旧口径改为新口径（35 桶/Pass），**不改 signal 终判本身的 INCONCLUSIVE 定性**（n_eff 不充分，措辞§5.6）。

## 4. 修复的**真实**价值（照实登记）

- **090 谎报消除**：`gamma_count_chi_filtered`（暗示 χ 已过滤但 chi=None）、`via_structural_prune`（暗示结构门剪枝但实为 AncOK silent_drops）已改诚实名
- **60/30/10 生效可证**：`w_depth == depth_weights[depth]` 518/518 零违例 + `w_depth=0 ⟺ units=0` 双向零违例——赋格仓位机制**确实生效**，与「L1 淹没大级别」假设的**方向性证据**（depth=0 占 207/518=40%，depth=1 占 28%，depth=2 占 19%，depth≥3 占 12%）
- **sizing 缺口否定**：`b1_sizing_available` 518/518 全 true + `units=0.0` 64 笔 ⟺ `depth≥3` 严格双蕴含——units=0.0 是 spec:42 正确的零 sizing（depth_weights 仅 3 项，越界 w=0 保留现金），**不是 bug**
- **Gap-1 口径一致**：wverify_run.rs 写死文本已从 0704 旧口径（25 桶/INCONCLUSIVE）回填为关②后新口径（35 桶/Pass/两桶 LCB>0 Validated），grep "25 桶" 零命中

## 5. 剩余 gap（按优先级）

- **Gap-1（新）**：χ 量纲裁定（L0 前置）——当前量纲下 χ 退化为方向门，接入前先裁量纲（绝对额 unlevered PnL vs 收益率 vs 归一化）
- **Gap-2**：L3 econ 门过滤率实测（χ/Nest 接入后对 n_orders 与 net_r 的机制影响，需 walk-forward 实测，非离线探针可定）
- **Gap-3**：signal 层 INCONCLUSIVE 定案（n_eff 不充分，措辞§5.6；与新口径 Pass 并存需裁定）
- **Gap-4**：`theta_overlay.rs:80` 编译失败（feature-gated bin，L1-P3 实装导致，需后续授权工位一行修复，不进 cargo test --lib 编译面）

## 6. 结论

**M8 修复后重跑对照完成。L1/L2/Gap-1/暴露面实装是 bit-exact 观测层修复（不改语义），消除了 090 谎报与观测盲区，但不足以解释 net_r 全负**——根因在 χ 量纲退化（L0 前置）与 signal 层 INCONCLUSIVE（独立并存）。本 goal 六项判据中第 6 项完成。

**证据文件**：/tmp/m8_opsem_fixed.out（修复后 stdout）、/tmp/m8_opsem_fixed/trades.jsonl（518 行）、cargo test 1737/0 全绿输出、三窗 18 项数值逐位一致对照（本报告 §1）。
