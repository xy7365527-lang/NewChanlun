# type1 背驰发射缺口根因诊断——公理"每反转必有type1"的 36% 可修 + 64% 证伪

> 编排者修正方向(2026-06-13)："44-49%覆盖率不是递归太严,是信号层bug。走势终完美是公理→
> 每笔端点必有次级别type1。先诊断缺失原因,不要绕过。"
> 认识论等级：**L3**(OKLO/CL/BTC 三标的真实数据 + 逐move bit-exact复刻 + 反事实)。
> 方法：穷尽诊断 Workflow(wf_570bb768，7 agents，4 假设 + 对抗验证 + 综合反事实)。

---

## 1. 结论

OKLO L1(ladder3) 105 个走势方向反转(笔端点)中仅 47(44.8%) 有 confirmed type1，54 个
**完全无 type1**(只有 type3)。逐 move bit-exact 复刻 `detect_trend_divergence` 定位根因：
**54/55 缺失反转的背驰根本没发射**(no_div_emitted)，卡在 `divergence.rs:315` 的 T2 判据
`force_c < force_a`——这些反转 **force_c ≥ force_a**(C段力度≥A段，中位比 3.91，最大 101.26，
全部 ≥1.148)，按背驰定义(后段力度衰竭)不构成背驰。

**逐项排除任务列举的四个怀疑**：

| 怀疑 | 实测 | 判决 |
|------|------|------|
| (a) 背驰阈值(TYPE1_CONFIRM_RATIO=0.9)太严 | 只欠确认 4 个边界 candidate；54 个连 div 都没发射 | ✗ 非主因 |
| (b) 中枢没 settle | 55/55 满足≥2 settled 中枢；settle门消融零差异(no-op) | ✗ |
| (c) 走势类型判定 | 缺失反转完成 move 100% kind=trend∧zs_count≥2 | ✗ |
| (d) C 段缺陷(H1 C段空/B2回退) | C段空=0，B2回退触发 0/55，H1 彻底否证 | ✗ |
| (e) 部分本就该是 type3 | **主因** | ✓ |

**根因分解(综合反事实，L2/L3)**——分母 55 个 T2_FAIL：

- **~36% (20/55) = 可修引擎缺陷(力度代理 duration 不对称)**：`force=(high-low)×duration`
  (`divergence.rs:195`)。A 段因 `trend_a_segment_range`(296-300) 相邻中枢首尾相接**坍缩为
  单段**(105/105，中位 76bar)；C 段被 B2(`next_settled_zs_seg_end`+`trend_extreme_seg`，
  418-424) **延展到下一 settled 中枢**(中位 4段 273bar，A/C 时长比 0.292)。短 A 段的
  force_a 被长 C 段的 duration 系数碾压。**反事实(去 duration)：这 20 个 amp_c<amp_a，本应
  是背驰**——力度代理 bug 压住了真背驰。
- **~64% (35/55) = 公理过强(强趋势末段加速，真实价格事实)**：去 duration 用纯振幅，这 35
  个**仍 amp_c 严格 > amp_a**——强趋势末段(C段)几何上就走得更远(加速冲顶/破底)，**无任何
  力竭**。按背驰定义不是背驰，引擎标 type3 **正确**。**任何力度代理(振幅/面积/MACD)都救不了**。
- **H4(笔端点多检) = 0%**：move 边界都是真中枢级反转(`greedy_group` 按中枢方向关系封组)；
  反事实合并同向 move 使 type1 从 60 降到 46(更差)，H4 方向证伪。

---

## 2. 定义依据

- **背驰定义**(缠论)：某级别走势中，后一同向推动段力度 < 前一同向推动段力度 = 力度衰竭 =
  背驰。引擎实现 `check_three_dim`(divergence.rs:315) T2=`force_c<force_a`；无 MACD 时 T6/T7
  (DIF峰/HIST峰衰竭)恒 False(dif/hist 峰值恒 0)，退化为纯 T2。
- **升跌完备性定理**：任何向上/向下必从三类买卖点**之一**开始并结束(type1∨type2∨type3)。
  type1=趋势背驰(走势完美/结束)；type3=中枢突破(新走势开始)。**同一反转点 = 旧趋势完美
  结束(无 type1，因无力竭) ∧ 新趋势中枢突破(type3)** = 两种合法标注。
- **数据**：OKLO 447738 bars + CL 5.5M + BTC 4.6M，`RecursiveOrchestrator`，逐 move bit-exact
  复刻 `detect_trend_divergence`(replicate PASS=65==engine divs=65 守卫通过)。

---

## 3. 公理的正确形式：type1 ∨ type3（升跌完备性）

公理"每笔端点必有次级别 type1"**~64% 被价格事实证伪**(强趋势加速无力竭)。但"每笔端点
必有某买卖点"成立——正确形式 = **type1 ∨ type3**：

| 标的 | type1∨type3 覆盖 | 残留(nothing) |
|------|-----------------|--------------|
| OKLO | 96.2%(101/105) | 4 |
| CL | 96.2% | ~3.8% |
| BTC | 95.4% | ~4.6% |

残留 ~4% 是"nothing"(反转处既无 type1 也无 type3)，需单独审计(可能是 candidate 未确认
或最高/最低层稀疏)。**type1∨type3 ≈ 96% 接近完备**，与 540号/537号的升跌完备性(任意类型
BSP 覆盖)一致——这里精化为 type1∨type3(排除 type2 回测点不标记反转)。

---

## 4. 边界条件 / 修复轴（按可达性排序）

1. **【可修，引擎层，救~36%】力度代理去 duration**：`divergence.rs:186-195` compute_force
   fallback 去掉 `×duration` 或改单位时长振幅。预期 OKLO type1 覆盖 45%→~55%。**概念层
   改动(改背驰判据含义)，按 formalization-validity-domain 须标 L2/L3 跨标的验证发射率，
   不可 L0 自证；bit-exact 破坏，影响所有在册回测**。
2. **【未测开放轴】A 段定义**：当前 A 段=最后两中枢间隙(坍缩单段)。若 A 应是**趋势首推动段**
   (move 起点到首中枢)，amp_a 会更大，可能救回部分 64%。**这是 bug-vs-公理的判别关键，
   需源头审计原文背驰 A/C 段定义**(缠师背驰比的是首末推动还是相邻晚段？)。
3. **【可修，确认门，救4个】TYPE1_CONFIRM_RATIO 0.9→1.0**：收回 4 个 candidate(div 已发射)，
   对 54 个 no-div 零作用。
4. **【未否证开放轴】接通 MACD 三维度**：`Some(MacdCtx)`+`enable_macd_divergence`，force 从
   振幅切换为面积/DIF峰/HIST峰，T6/T7 激活。但强趋势加速冲顶时 MACD 动能往往也创新高，
   预期仍 <100%。
5. **【不可修，公理放宽】~64% 残留**：强趋势末段几何更猛是真实价格事实，正确归属 type3。
   公理放宽为 type1∨type3(覆盖 96%)。

**不应改动**(实证证明无 bug)：`trend_a_segment_range` 坍缩(c_empty=0)、B2 越界(回退 0/55)、
`moves.rs` 分段(H4=0%)、`buysellpoint.rs` detect_type1(div 发射则必产 type1，无丢弃分支)。

---

## 5. 下游推论

1. **依赖 type1 作笔端点信号的策略在 ~55% 反转处无 type1**，被迫退化到 type3(滞后，新走势
   已开始)。这与 RNF 清仓踏空/regime 二难、v5"located 折叠丢失次级别确认独立闸门"吻合——
   清仓绑 type1，覆盖天花板 ~45-55%。
2. **NRF/区间套用 confirmed type1 作次级别确认闸门，覆盖天花板 ~45-55%**。区间套递归到 a0
   要 100% 覆盖，必须 type1∨type3(或修 duration 提到 ~55% 仍不足 100%)。
3. **力度度量是有效域瓶颈**(非 settle 门、非确认比值门)。"背驰只振幅维度 MACD 不可达"
   (memory)在此定量化为发射率 ~45%。

---

## 6. 谱系引用

- **project_bsp_gap_root_cause / project_c_segment_fix_b2**：本诊断**否证** H1(C段空)——B2 修复
  后 C 段不空(c_empty=0)，问题相反是 C 段被 B2 **过度延展**致 force_c 膨胀。
- **project_signal_layer_duality**("背驰只振幅维度 MACD 不可达")：本诊断定量化为发射率 ~45%
  跨标的恒定。
- **540号/537号(升跌完备性)**：本诊断精化升跌完备形式为 type1∨type3(96%)。
- **533号(结构滞后)**：type1 覆盖天花板 ~45% = 确认层在强趋势结构性受限的定量形式。

---

## 7. 影响声明

**新增**：`analysis/type1_emission_gap_diagnosis.md`(本报告)；诊断脚本探索性(未入库)。
**引擎/交易层零改动**——本工作是纯诊断(回答"为什么缺失")。

**对认识的影响**：
1. 公理"每反转必有 type1"**~64% 被价格事实证伪**(强趋势加速)，正确形式 = type1∨type3(96%)。
2. **~36% 是真引擎缺陷**(force=振幅×duration 的时长不对称)，去 duration 可救 type1 45%→~55%。
3. 我之前提议"放松到 sell_any"：36% 方向对(duration bug 真实)，但把 64% 也当 bug 是错的——
   那 64% 是真实价格事实，type1∨type3 才是正确操作形式。

**待裁决(定义层，需编排者 + 源头审计)**：
- (A) 修 duration(救 36%，bit-exact 破坏全回测) — 做不做？
- (B) 源头审计原文背驰 A/C 段定义(判别 64% 是 A段定义 bug 还是真公理过强) — 做不做？
- (C) 接受 type1∨type3 作操作形式(覆盖 96%，不改引擎) — 区间套/清仓判据改 type1→type1∨type3？

---

*L3 | OKLO/CL/BTC + 逐move bit-exact + 反事实 | 缺失type1=36%可修duration bug + 64%公理过强(强趋势加速) | 正确形式type1∨type3覆盖96%*
