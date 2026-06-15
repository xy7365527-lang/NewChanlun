# unn T1⊥A8 扬弃验证：观测态（Observation State）

> 引擎：`rust/src/trading/unified_necessity.rs`（mode="unn"）。承接 df752ea6e4（字面 T1
> 永远在场内 in-place 翻转）的经验否证。编排者裁决："实装 T1⊥A8 的扬弃——观测态"。

## 1. 结论

字面 T1（永远持仓、否定→in-place 翻转不回现金）经 df752ea6e4 实装后 CL−56.7%/BRN−136.6%
**破产**（whipsaw stop-and-reverse）。本次实装其**辩证扬弃**（Aufhebung，089号）——观测态：

| 标的 | 字面 T1（df752ea6e4） | 观测态扬弃（本次） | 改善 |
|------|----------------------|-------------------|------|
| CL | strat **−56.7%** / MDD −103.8（破产） | strat **−9.1%** / MDD −50.7 | +47.6pp，脱离破产 |
| BRN | strat **−136.6%** / MDD −153.3（破产） | strat **−17.5%** / MDD −32.0 | +119.1pp，脱离破产 |

两标的均脱离破产（strat 远高于 −100%，MDD 减半以上），任务判据"比字面 T1 好得多"达成。

## 2. 扬弃的三环节（否定 / 保留 / 提升）

| 环节 | 内容 | 代码 |
|------|------|------|
| **否定** | T1 不是字面永远持仓——根否定 ⇒ `close_voice` 回现金（A8 资本保全：否定线=停损，先于保证金线）。删除 B 规则 in-place 翻转。 | B 段 `is_root ⇒ close_voice("negate_observe") + observing=true` |
| **保留** | 引擎不离场——观测态下信号层照常更新（nest 窗口/cascade located/BSP 检测全跑），step 不 early-return（A→F + N1-N8 prove 照常）。 | step 无观测态 early-return；`prove_t1_aufheben` 判据① |
| **提升** | 在场 = 跟踪走势 + 在买卖点操作（含回现金的观测态）。观测态遇下一个 F-eligible confirmed BSP（buy1@located 链顶）⇒ 同 bar 重新建仓。 | F 段 `if self.observing { reentry + observing=false }` |

**覆盖映射分歧层**（任务点5）：T1⊥A8 处覆盖空间从一层（持仓）分裂为两层（持仓 ∥ 观测）。
观测态 = 后否定的现金等待态（≠ 初始未入场态——`observing` 标记区分两者）。

## 3. C 规则的 T14 保留（相位区分：否定停损 vs 走势完美翻转）

字面 T1 的破产死因是 **B 规则否定线 stop-and-reverse**（强趋势中根被迫无避险做空累积亏损），
**非** C 规则的 type1 背驰翻转。两者相位区分：

- **B 否定（停损相）**：否定线被破 = 走势otherwise否定 ⇒ 回现金（观测态，A8）。
- **C type1 背驰（走势完美相）**：§6"十年1-2次"顶/底背驰 ⇒ in-place 长↔空翻转（T14 保留）。

实证（exit_reasons）：`flip_short`=10(CL)/4(BRN) 是保留的 C-T14 翻转；`negflip_short/negflip_long`
（字面 B 翻转腿）= **0**（已删除）。

## 4. 验收标准：必然性运行时证明（L2，零 panic = PASS）

引擎内 prove 函数 violation = panic（137号 make-decision-observable）。新增第 9 个：
**`prove_t1_aufheben`**（观测态运行时证明，两支判据）：
- ① 引擎不停（保留）：观测态不 early-return——结构性保证，到达断言即证。
- ② 即时重建（提升）：若 bar 起处观测态且本 bar F-eligible（buy_source ∧ buy1@s ∧ units>0），
  则 bar 末必已离开观测态（重建仓）。violation = "有 BSP 不建仓"（A8 退化为永久空仓）。

B 规则变更全标的生效（任何根否定→现金），故全 8 标的（~26.9M bar）重新 L2 验证：

| 标的 | bars | N1-N8+T1⊥A8 prove | N1 max_children | N4 floor_stops | 验收 |
|------|------|-------------------|-----------------|----------------|------|
| OKLO | 447K | PASS（零 panic） | 3 | 0 | ✓ |
| QQQ | 728K | PASS（零 panic） | 1 | 0 | ✓ |
| BRN | 2.4M | PASS（零 panic） | 3 | 0 | ✓ |
| DX | 2.0M | PASS（零 panic） | 0 | 0 | ✓ |
| ES | 5.6M | PASS（零 panic） | 3 | 0 | ✓ |
| GC | 5.5M | PASS（零 panic） | 2 | 0 | ✓ |
| CL | 5.5M | PASS（零 panic） | 4 | 0 | ✓ |
| BTC | 4.6M | PASS（零 panic） | 5 | 0 | ✓ |

**全 8 标的 ~26.9M bar 零 panic + 零 floor_stops** ⇒ N1-N8 + 新增 `prove_t1_aufheben`
在全 bar 成立（L2 必然性验证）。新增观测态判据未在任何 bar 触发"有 BSP 不建仓"违反。

## 5. 观测态机制实证（L2，真实数据）

| 标的 | negate_observe（根否定→现金） | recover（子回补） | flip_short（C-T14 保留） | negate（子否定） | root_entries |
|------|------------------------------|------------------|-------------------------|-----------------|--------------|
| CL | 145 | 127 | 10 | 35 | 145 |
| BRN | 64 | 30 | 4 | 10 | 64 |

- **negate_observe = root_entries**（145/145，64/64）：每次入场最终被否定回现金（CL/BRN 是震荡/
  下行 regime，长仓反复被否定）。观测态使每次否定 = 有界损失（现金避险），故总损失受控
  （−9.1%/−17.5%）而非字面 T1 的无界 whipsaw 累积（破产）。
- **flip_short > 0**：C-T14 type1 背驰翻转保留并触发（10/4 次）——空头侧捕获仍在。
- **zero negflip_***：字面 B 翻转已删除。

## 6. 全 8 标的有效域读数（L3，非验收标准）

| 标的 | unn strat% | BH% | MDD% | negate_observe | flip_short(C-T14) | maxkids | P1 |
|------|-----------|-----|------|----------------|-------------------|---------|----|
| OKLO | +14.4 | +307.1 | −24.7 | 19 | 2 | 3 | ✗ |
| QQQ | +26.6 | +174.6 | −8.4 | 26 | 2 | 1 | ✗ |
| BRN | −17.5 | +87.4 | −32.0 | 64 | 4 | 3 | ✗ |
| DX | −4.1 | +4.1 | −7.1 | 70 | 8 | 0 | ✗ |
| ES | −3.6 | +594.3 | −32.4 | 83 | 8 | 3 | ✗ |
| GC | +160.5 | +257.3 | −21.5 | 127 | 17 | 2 | ✗ |
| CL | −9.1 | +28.2 | −50.7 | 145 | 10 | 4 | ✗ |
| BTC | +72.8 | +1380.4 | −76.5 | 41 | 5 | 5 | ✗ |

**读数（非结论）**：
- **观测态全标的激活**：negate_observe ∈ [19, 145]——每标的均有根否定→现金的观测态转移。
  `flip_short`（C-T14 type1 背驰翻转）全标的 > 0（保留有效）。零 `negflip_*`（字面 T1 删净）。
- **核心成果（CL/BRN 脱离破产）**：CL −56.7%→−9.1%、BRN −136.6%→−17.5%——观测态以
  现金避险替代无界 whipsaw，两破产标的均回到有界损失。
- **MDD 全标的有界**：所有标的 MDD > −77%（vs 字面 T1 的 −103.8/−153.3 击穿 −100%）。
- **P1 = 0/8**（全弱于 BH）：观测态以"踏空换资本保全"——强牛标的（OKLO/QQQ/ES/BTC 等
  BH 极高）的暴露不足。**P1 非验收标准**（§7 边界）；P1=0/8 缩小了"永远在场 vs 观测态"
  的有效域边界——观测态的相对优势在震荡/下行 regime（破产避免），强单边牛 regime 是
  开放轴（regime 门控，§8）。

## 7. 边界条件（结论翻转条件）

- **观测态有效域**：观测态优于字面 T1 的有效域 = **非单边强牛 regime**。强单边牛市中，
  否定→现金会错失反弹（踏空），字面 T1 的"永远在场"反而可能（罕见地）不输——但其无界
  whipsaw 风险使期望破产。观测态以"踏空换资本保全"，在震荡/下行 regime（CL/BRN）净优。
- **P1（vs BH）非验收标准**（编排者框架）：必然性 prove（零 panic）是验收；P1 是 L3 有效域
  读数。CL/BRN P1=✗（弱于 BH）不否定 8 条必然性——清仓/重建频率的 regime 依赖是开放轴。
- **再入场 N5 门控**：观测态重建仓走 F 路径（located_buy 链顶 ≥ move(L1) ∧ buy1@s），
  **非裸入场**——裸入场 ⊥ N5（segment 非势源，539号 A′ 解耦踏空轴）。任务"不需 pending_locate"
  指不需 C 式翻转链（根已在场的清仓/翻转），而非绕过 F 的 N5 入场门。

## 8. 下游推论

- 字面 T1（永远在场）经验否证后，观测态成为 unn 的根级别出场默认形态。
- 开放轴（承接 df752ea6e4 commit message）：**regime 门控**观测态 vs 永远在场——
  单边强牛 regime 下可否定观测态、改回某种在场形态（需 regime 判别量，正交于本次结构修复）。
- C-T14 翻转的有效域（type1 背驰"十年1-2次"）独立于观测态——两者相位区分后可分别精化。

## 9. 谱系引用

- **df752ea6e4**：字面 T1 永远在场内 in-place 翻转——T1⊥A8 概念张力的发现（commit message
  已记录"否定→现金退出不是缺陷，是资本保全必然"）。
- **b5355deb08**：T14/T5/A5（C 规则 type1 背驰翻转，本次保留）。
- **089号**：扬弃 Aufhebung（否定+保留+提升）——本次实装的方法论依据。
- **539号 constitutive_throughput_falsified**：A′ 解耦踏空轴——F 保持 N5 门控的依据。
- **027:25**：否定线（缠论原文，A8 资本保全=否定线先于保证金线）。
- **形式化有效域规则**：本验证 L2（CL/BRN ~8M bar，全 8 标的填充后 ~25M bar）——
  否定性结果（P1✗）缩小有效域边界，比确认性结果信息增量更高。

## 10. 影响声明

- **改动**：`rust/src/trading/unified_necessity.rs`（B 规则 in-place 翻转 → 观测态；新增
  `prove_t1_aufheben`；F 规则观测态→持仓态转移；模块文档；观测态字段 + 计数）；
  `rust/src/trading/positional.rs`（PositionalResult 加 4 个观测态读数字段）；
  `rust/src/lib.rs`（marshal 观测态字段到 Python）。
- **不改**：BSP 引擎、会计原语（close_voice/settle/nav 逐字复用）、C 规则 T14 翻转、
  N1-N8 prove（持仓态全保持 PASS）。
- **单元测试**：`t1_root_negate_flips_in_place`（字面翻转）→ `t1_aufheben_negate_observes_then_reenters`
  （观测态扬弃）；其余 10 个 unn 测试不变全过（365 lib 测试全过）。
