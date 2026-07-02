# 实装报告：BspCandType 新增 StructBreak 第四类纤维（task #62）

**执行者**：ws-sbimpl | **日期**：2026-07-02 | **裁决来源**：codex 终局裁决A `.chanlun/review-results/codex-decide-20260702-204130-d07f.md`

## ① 结论

`rust/src/theta_v0/backtest/econ_positive.rs` 五处施工级改动，逐字按 codex 终局裁决A 实装：

1. **`BspCandType` 枚举**（econ_positive.rs:584-593）新增 `StructBreak` 变体：`{Type1, Type2, Type3, StructBreak}`。
2. **`bsp_cand_type`**（:595-618）前置判别：`if bits.class_index() == 0 { return BspCandType::StructBreak; }`，之后才走原有 buy1/buy2/else 三分派。
3. **`cand_delta`** match（:683-692）新增 `BspCandType::StructBreak => false`。
4. **`cand_delta_base_gate`** match（:701-711）新增 `BspCandType::StructBreak => false`。
5. **`build_gate_certificate`** match（:1163-1177，行号因前四处插入而偏移，原裁决文标注 :1154）新增 `BspCandType::StructBreak => None`——门拒，不新增 `GateCertificate` 通道，不复用 Type3 的 `descend_type1_anchor_depth`/Nest/Xzd。

新增护栏测试两条：
- `bsp_cand_type_structbreak_zero_bits_dispatch`：六 bit 全零（双侧）⟹ `StructBreak`；buy3/sell3 真三类置位仍 ⟹ `Type3`（回归保护）。
- `build_gate_certificate_structbreak_zero_bits_always_none`：同一 lvl==0 fixture 下，零 bit 恒 `None`（即使 lvl==0 是 Type2/3 的免门场景也不误纳）；对照组真 Type3（buy3=true）同 fixture 下仍正常 `Some`（未受影响）。

## ② 定义依据

- **P2-R2**（`signal.rs:577-580` 注释）：破中枢结构候选全部进样本（消选择偏差），未背驰确认时 bits 全零 + `struct_break_dir` 方向旁路。
- **`judge_third`**（`signal.rs:358-391`）：真三类候选产生路径，几何前提=未破核心区间（`retest.price > c.zg` 严格不触及），置 buy3/sell3。
- **`candidate_dir`**（`interp.rs:220-236`）+ 护栏测试（`interp.rs:1059-1074`，逐字断言"六 bit 全零 ⟹ class_index=0"）：`struct_break_dir` 仅在六 bit 严格全零时启用旁路，不进桶键——`class_index()==0` 是精确的 StructBreak 判别边界，与既有护栏测试完全对齐，非新假设。
- **673 号先例**（已结算）：统一分支混入几何前提互斥的对象群时，正确解法=拆分谓词/分支而非下游打补丁。本次是该原则的第二次实例应用。

## ③ 边界条件（何时翻转）

- 若文档原文明确规定 T3 是"残差候选池"而非三类买卖点，本次分拆失去正当性——但当前无此原文依据。
- 若未来给 `StructBreak` 定义了独立的确认语义/形式化证书，则应新增 `GateCertificate::StructBreak` 通道，而非维持恒 `None`。
- 若发现六 bit 全零不仅来自 P2-R2 的 `struct_break_dir`，还有其他无方向噪声来源，`class_index()==0` 单一判别条件不再充分，需要把 `struct_break_dir` 或完整 `BspPoint` 传入分类函数。

## ④ 下游推论

- **W-VERIFY（task #13）**：不受直接影响——alpha 分桶键 `MuClass.i_class = bits.class_index()`，与 `BspCandType`（econ_positive.rs 内部门控分派枚举）是两个不同层的概念，零 bit 候选在分桶层本就识别为 `bsp_class=0`，与真三类（`bsp_class()=3`）分桶层本就不混。本次收紧只影响 `econ_positive.rs` 内部的门控分派（Nest/Xzd/拒），不改变 W-VERIFY 桶键完备性。task #13 仍应在本次实装后重跑作为收敛前的最终校验。
- **350K BTC dx 重跑结果**（见下节）：本窗口 gate-pass 信号集变化量=0——收紧是概念/架构层面的修正（消除 T3 纤维语义混淆），非本窗口信号集收窄。

## ⑤ 谱系引用

- 新增 `.chanlun/genealogy/pending/679-bspcandtype-structbreak-fourth-fiber-zero-bit-gate-tightening.md`（status: 已结算，type: concept-separation），依赖 673（先例）+ 615（partition-proof 首次核实此互斥）。
- `dag.yaml` 同步：新增节点 `679` + 两条 `depends_on` 边（679→673, 679→615）。YAML 校验通过（648 节点、1679 条 depends_on 边）。

## ⑥ 影响声明

**代码改动**：`rust/src/theta_v0/backtest/econ_positive.rs`——
- `BspCandType` 枚举 + `bsp_cand_type` + `cand_delta` + `cand_delta_base_gate` + `build_gate_certificate`（五处施工点）。
- 诊断新增：`acc_classification_level_hole_dx` 新增 `n_zerobit_gate_pass` 计数器 + 报告行（供未来重测复查，不改变既有断言/逻辑）。
- 新增两条单测护栏。

**350K BTC dx 重测（真实数据，如实报告）**：

用 `acc_classification_level_hole_dx`（`ECON_L2_MAX_BARS=350000`，`--release --ignored --nocapture`）测量：

| 状态 | n_zerobit_gate_pass | n_gate_pass_total | n_xzd_pass | sig_post_sum |
|---|---|---|---|---|
| before（临时禁用 StructBreak 前置判别，还原旧 else=>Type3 语义） | 0 | 892 | 30 | 892 |
| after（恢复 StructBreak 判别，即当前实装） | 0 | 892 | 30 | 892 |

**变化量 = 0**。350K BTC 实测窗内，零 bit 破中枢候选此前也从未真正走通 Nest/Xzd 门通过——本次收紧对该窗口已产出的 gate-pass 信号集**无经验影响**，价值是概念/架构层面（消除 T3 纤维语义的对象混淆，闭合 673 先例的残余实例）。有效域声明：仅覆盖 350K 截断子窗，非全量 461 万 bar 结论；W-VERIFY 全量重测时应一并复查此计数器是否仍恒为 0（`formalization-validity-domain` 规则：如实报告，即使为 0 也不隐藏）。

**测试验证**：
- `cargo test --release econ_positive`：33 passed, 0 failed, 7 ignored。
- `cargo test --release --lib`：1396 passed, 0 failed, 100 ignored。
- `econ_positive.rs` 并发协调：开工前记录基线 diff（32 行，对应 #13/ws-wverify 的 xzd 命中率哨兵段 `assert!`→`eprintln!` 改动），本次五处改动+两条测试+一处诊断计数器均在不重叠区域（580-620/691/711/1163/1554-1610/3201/3345/3465/3697行区间），最终 diff 中该哨兵段内容与基线逐字一致，未被覆盖或冲突。
