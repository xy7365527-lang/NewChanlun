# 亏损诊断：两轮 deep-research 结合报告（为什么亏钱）

- **日期**：2026-07-05
- **合成**：workflow synthesis 成功版（w1um7h30u，95/109 agent）+ Lead 合成补全
- **输入**：①博文层 deep-research（wf_182ff790-eef，5 confirmed）②formal-chain 层 deep-research（wf_a0f591ec-34d，synthesis 成功 5 merged findings）③formal-criteria-20260705.md（15 条硬判据清单）④M8 终报告 maxfull-e2e-round1（四层因果链）
- **认识论**：全部 L0/L1（PDF 亲读 + 代码静态核对）；L2/L3 转引既有跑批

---

## §1 一行答案

**亏钱不是单一原因，是「信号侧假背驰大量产生 + 退出侧风险/反向机制残缺」的叠加**——entry 侧在盘整语境下系统性产生 formal-chain 判据不允许的假信号，exit 侧因 RiskClose 退化 + 同级别反向 exit 不触发导致这些假信号被持有到破产才退出。两条链都与 M8 终报告的「signal 无 edge → exec 负 → treasury 不推进」因果链完全吻合，给出了**具体机制**。

---

## §2 两轮 confirmed claims 合并表（21 条，按亏损相关性排序）

### 🔴 第一档：退出机制残缺（退出过晚的直接根因）

| ID | 层 | claim | 票 | 代码锚点 | bug? |
|---|---|---|---|---|---|
| F5 | formal | RiskClose 退化为 equity≤0 单一判据（maint_margin/buffer/liq_flag 全 0 占位），§11 风险可行集 μ_t 五态只 discharge Insolvent | 3-0 | exit.rs:111-117 | **是 bug**（实装缺口） |
| F4 | formal | schedule_adapter 只派 ShortDiff 一类，SameReverse/SubFollow 两类操作角色从不产生动作 | 1-1 | transition.rs:654 | **疑似 bug**（whipsaw 无法分离） |
| F6 | formal | 二类反向点无独立 exit 枚举，折叠进 CloseRoot(P5) | 1-1 | interp.rs:218-256 | 待裁（g4-impl 边界） |
| B5-博文 | 博文 | 前视偏差第三类警示：bug/理论边界之外应增设「回测假设缺陷」类，优先排查 entry 时机 | 3-0 | — | 开放问题（需机器核验） |

### 🟡 第二档：信号判据弱于 formal-chain（假背驰/假买卖点源）

| ID | 层 | claim | 票 | 代码锚点 | bug? |
|---|---|---|---|---|---|
| B2-博文 | 博文 | 趋势背驰硬前提：a+A+b+B+c 中 A、B 必须**同级别中枢**，否则最多盘整背驰 | 3-0 | 第37课原文 | 判据基准 |
| F11 | formal | div_cand 无 τ=Trend 门，趋势 vs 震荡不区分 | 2-1 | cand_predicate.rs:107-151 | **疑似 bug**（盘整背驰误判趋势背驰） |
| F10 | formal | Comparable 条件2 只 rfind 同向段，无同级别中枢存在性检查 | 1-1 | cand_predicate.rs:133-135 | **疑似 bug**（与 B2-博文 直接冲突） |
| F15 | formal | 默认背驰口径 DivergenceGauge::MacdArea 退化为单 MACD 面积，formal-chain 要求支配序 force measure | 1-1 | divergence.rs | **疑似 bug**（假一类买卖点直接源） |
| F13 | formal | MACD 面积非力度充分统计量，单一面积比较在两种情况下误分类 | 3-0 | cand_predicate.rs:147-150 | 有效域边界（非 bug，但支撑 F15） |
| F14 | formal | lvl==0 Type2/3 信号免锚定门（定律一下沉在最细粒度被绕过） | 3-0 | econ_positive.rs | **疑似 bug**（结构松弛） |
| B1-博文 | 博文 | 三类买卖点硬几何条件：回踩终点不入中枢区间/不破 ZG-ZD | 3-0 | 第53课原文 | 判据基准 |

### 🟢 第三档：实装忠实（否定性确认——非 bug）

| ID | 层 | claim | 票 | 结论 |
|---|---|---|---|---|
| F1 | formal | 六谓词全定义含因果性要求（ex-ante 可测性判据） | 3-0 | 判据成立（支撑 B5-博文 前视偏差核验） |
| F2 | formal | 区间套 J_child⊆J_parent 硬条件 | 3-0 | 实装正确 |
| F3 | formal | ShortDiff 父仓保持 Δs_α=0, s_g=s_α | 3-0 | 实装正确 |
| F7 | formal | G4(#134) 仅统计层接线，生产订单流未 typed | 3-0 | 既有已知（g4-impl 在案） |
| F8 | formal | §16 X^cover 覆盖域：K_Θ 约束把 p̃ 推出时风险投影静默偏离 | 3-0 | 实装忠实，但暴露资本约束压制 |
| F9 | formal | §13 假设7：K_Θ 非空是全定义性前提 | 2-1 | 实装缺口检查点（连接 treasury） |
| F12 | formal | §4 condition 1/2（同级别同向+同上级语境）实装正确 | 3-0 | 否定性确认（背驰候选门非 bug） |
| F16 | formal | 力度衰减 MACD/EMA 规则标 [L3 待标定] v0 占位 | 2-1 | 有效域未标定（非 bug） |
| B3-博文 | 博文 | divergence 定位法：相同输入同输出为等价，反向 trace 追写入来源 | 3-0 | 方法论（供逐笔重放） |
| B4-博文 | 博文 | 利润最大定理：入场锚定确定操作级别 | 3-0 | 判据基准 |

---

## §3 亏损因果链（两轮结合 + M8 终报告对齐）

```
M8 终报告因果链：signal 无 edge → exec 净 R 全负 → treasury 不推进 → total LCB≤0
                          ↑                    ↑
              ┌───────────┴───────┐   ┌────────┴─────────┐
              │ 信号侧假背驰源    │   │ 退出侧机制残缺   │
              │ (entry 假信号)    │   │ (假信号持有过久) │
              └───────────────────┘   └──────────────────┘
```

### 信号侧（为什么 signal 无 edge）
1. **趋势门缺失**（F11 + B2-博文）：div_cand 不区分趋势/盘整，盘整语境下产生 formal-chain 判据不允许的"趋势背驰"信号
2. **Comparable 中枢前提缺失**（F10 + B2-博文）：只找同向段不查同级别中枢存在性
3. **力度判据脆弱**（F15 + F13）：默认单 MACD 面积，formal-chain 要求支配序 force measure → 假一类买卖点
4. **下级别锚定绕过**（F14）：lvl==0 Type2/3 免门，定律一下沉在最细粒度失效
5. **前视偏差未核验**（B5-博文 + F1）：因果性判据成立但未机器核验——开放问题

### 退出侧（为什么 exec 净 R 全负）
1. **风险退出残废**（F5）：RiskClose 只剩破产判据，保证金/缓冲主动减仓不存在 → 退出过晚
2. **同级别反向 exit 不触发**（F4 + F6）：schedule_adapter 只派 ShortDiff，SameReverse 从不动作 → whipsaw 无法被同级别反向分离
3. **资本约束静默压制**（F8 + F9）：K_Θ 把 p̃ 推出时 entry 被合法压制不报错 → treasury count=0 上游

### 理论边界（修了也不一定赚——非 bug）
- F12（§4 condition 1/2 实装正确）+ F16（力度衰减 L3 待标定）+ F13（MACD 非充分统计量）：部分判据实装忠实但经验有效性未在 L2/L3 验证

---

## §4 可行动 vs 不可行动

### 可行动（实装 bug，定理/行动类——蜂群可修）
- **修 F5**：实装 maint_margin/buffer/liq_flag 真实账户层输入（或显式标注 A10 waiver 域扩展），RiskClose 五态完整
- **修 F11/F10**：div_cand 加 τ=Trend 门 + Comparable 加同级别中枢存在性检查（对齐 B2-博文 第37课前提）
- **修 F15**：默认背驰口径改 ThetaDom/支配序（force measure），MacdArea 降为可选
- **修 F4**：schedule_adapter 派发 SameReverse（同级别反向平仓）
- **修 F14**：lvl==0 Type2/3 恢复锚定门（或显式声明有效域排除 lvl==0）

### 不可行动（理论边界/选择类——编排者裁决）
- F16 力度衰减 L3 标定（需真实数据经验校准）
- F6 二类反向 exit 枚举（g4-impl 边界，待裁）
- B5-博文 前视偏差核验（需逐笔重放基建——opsem workflow 的 instrument 阶段）

### 需逐笔验证（四个开放问题的可检验形式）
1. **bug 命中率**：2256 笔 entry 中，违反 F2/F11/F10/B1-博文 的占比多少？
2. **前视偏差**（B5-博文 + F1）：entry 信号是否消费 bar close 后信息？反向 trace 核验
3. **whipsaw 分离**（F4/F5）：满足全部判据仍亏的部分，exit 后走势是否系统性 whipsaw？
4. **treasury 上游**（F8/F9）：p̃∉K_Θ 的 bar 占比？是否是 count=0 根因？

---

## §5 结果包六要素

1. **结论**：亏损=信号侧假背驰（F11/F10/F15/F14）+ 退出侧机制残缺（F5/F4）叠加；两轮 deep-research 21 个 confirmed claims 与 M8 因果链一致收敛。
2. **定义依据**：博文层（第37/49/53课）+ formal-chain（买卖点.pdf §4/§9/§13/§15/§16、关于背驰.pdf §4、完整的策略.pdf §11）+ 代码锚点（exit.rs/transition.rs/cand_predicate.rs/divergence.rs/interp.rs）。
3. **边界条件**：①修 F5/F11/F10/F15 后若 signal 层仍无 alpha=理论边界（F12/F16 生效）；②前视偏差（B5-博文）若确诊=第三类根因，翻转整个诊断；③F4/F6 若经 codex 裁定为合法设计非 bug，退出侧诊断弱化。
4. **下游推论**：①F5/F11/F10/F15/F14 五条 bug 候选可立修复 goal；②逐笔重放（opsem workflow）是验证 bug 命中率的必要基建；③修复后须新预注册重跑（135 冻结纪律）。
5. **谱系引用**：090（严格性/声明膨胀）、161（否定性照实）、231（有效域<定义域）、135（冻结先于跑数）、679/692/696（结算先例——本诊断属研究产出非谱系结算）。
6. **影响声明**：新增本文件（纯研究合成，零代码改动）。两轮 deep-research workflow 的原始 claims 数据在各自 task output 文件。本报告不改任何 settled 谱系；F5/F11/F10/F15/F14 若经逐笔验证确认=真 bug，由 genealogist 新立 pending 谱系条目。

---

## §1.1 synthesis 权威结论修正（w1um7h30u 成功版）

workflow synthesis 成功后产出 5 条 merged findings，其核心结论修正了上文 §1/§4 的"bug 候选"标注：

**「亏损来自 layered spec-acknowledged v0 simplifications，不是 implementation bugs」**

修正要点（对照上文 §4「可行动」节）：
- F11/F10（div_cand 无趋势门/Comparable 无中枢检查）：econ_positive.rs:698-699 明确"含盘整背驰"是**代码有意混入**，Type1 主路径 signal.rs:822-829 有 center_trend_gate 预滤——**spec 承认的有效域边界，非 bug**
- F15（默认单 MACD 口径）：divergence.rs:252-265 自标「★诚实 gap」，config.rs:278-280 是 bit-exact 预注册边界——**spec 承认风险源，非 bug**
- F5（RiskClose 退化 equity≤0）：v0 可计算 Insolvent 的诚实占位——**A10 waiver 域内的已知简化**
- F14（lvl==0 Type2/3 免门）：结构性松弛但 econ_positive 代码注释自述——**有效域边界**

**真正的诊断方向**（synthesis 指出）：
1. Type2/3 占 97.8% 交易却跳过 per-rung divergence verification——这是**结构性有效域收窄**，不是 bug
2. 89.6% 交易在 level 0，定律一下沉在最细粒度绕过——同上
3. K_Θ 静默 clamp（p̃∉K_Θ 时）——对应 GAP3 count=0 上游
4. G4 typed-exit 层分离：μ pipeline 消费 typed labels 但不驱动订单流——**typed exit 诊断可能不反映实际订单行为**

修正后的可行动项（替换 §4）：
- **非"修 bug"，而是"L2/L3 经验校准 + 有效域扩展"**：默认背驰口径切 ThetaDom（F15）、lvl==0 免门的有效域显式声明或恢复锚定（F14）、K_Θ 三阶段资本约束实装（F8/F9）
- **逐笔重放仍是必要的**——但目的是测 bug 命中率（验证 spec 简化是否真的导致了假信号占比高），不是"找 bug 修 bug"
