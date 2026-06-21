---
date: 2026-06-21
status: 待裁决（重新激活——多重赋格落码后 §8.1 真实重现 10 次，两机制：5 多级别现金流耦合 + 5 C3 亏损短差。但仅占 1.7% recover，主因是做空腿牛市亏损 regime）
type: 概念层选择上浮（§8.1 同股数回补 free 不足——多级别现金流耦合 + 亏损短差，三读法待裁）
triggers: 多重赋格落码（所有级别并行消费 BSP，sink 0→592）→ §8.1 panic（首次 free=23665 < need=27823，盈利短差耦合）
relates: ['§8.1', 'C1', 'C3', '§9.8', '§9.9', '架构v2-§8-fail-loud']
sources: ['第31课', '第26课:34', '第37课', '第29课:58']
refs_memory: ['project_recursive_t_architecture_v2', 'project_t_short_leg_regime_function', 'project_zombie_short_diagnosis', 'feedback_bidirectional_always_in', 'project_earning_shares_empty_domain', 'project_t_multiscale_independent_filters', 'project_t_cross_level_coupling_falsified', 'project_t_operation_self_replication']
---

## 更新 2（2026-06-21，多重赋格落码后 §8.1 真实重现）

编排者裁决「所有级别并行消费 BSP（多重赋格），删 want gate + 单链下钻」落码后（扁平重构：`instance.child` 单链 → `TRoot.level_short[k]` 每级别短差），**sink 0→592 真正消费 BSP**（L0:319/L1:272/L2:1）。BTC 全量诊断（diag 兜底跑完）：

- **§8.1 free 不足 = 10 次**（592 recover 的 1.7%），**两个机制**：
  - **5 次 = 多级别现金流耦合（盈利短差 c2<c1 但 free 不足）**——非 C3。首次：lvl=0, core_units=18.79, Σshort=0, free=23665, need=27823, realized=+207（**盈利**）。根因：恒仓核心持仓占用财富（core 价值 ~97389 在持仓），free 现金不足以再买入回补。这是 [[project_t_cross_level_coupling_falsified]]「单一池放大」/[[project_t_operation_self_replication]]「几何塔 1x 强平级联」的显形。
  - **5 次 = C3 原版（亏损短差 c2>c1）**——卖点失败。
- 这两类合计 10 次（次要）。**主因是做空腿牛市系统性亏损**：short_leg_pnl=−21057，亏损率 55.7%（330 亏/262 盈），把纯持多 +1423% 拖到 +30.5%（< BH +1380%）。印证 [[project_t_short_leg_regime_function]] L3「做空腿=亏损唯一来源」。

**三读法（C3 扩展，待编排者裁）**：(A) 同金额回补（free 不足时回补可用部分，核心恒仓被侵蚀；diag 兜底即此）/ (B) 杠杆（free 可负）/ (C) 修订 §8.1 假设。**新增维度**：5 次是盈利短差耦合（非卖点失败），所以不只是"卖点会失败"问题，还有"恒仓单一 free 池 + 多级别几何塔 → 现金流动性不足"的结构问题。

**但 §8.1 是次要**（1.7%）。主矛盾是做空腿 regime（牛市亏损），需多标的 L3 确认（CL/DX 震荡 regime 可能盈利，[[project_t_operation_self_replication]] 正域）。

---

## 更新 1（已撤回 FALSIFIED → 已被更新 2 取代）

⚠️ 更新 1 的"C3 falsified"基于 sink=0（P3b 单链下钻未消费 BSP）。多重赋格落码后 sink=592 真正消费，§8.1 真实重现（见更新 2）。故 C3 **重新激活**，但带新机制（多级别耦合）。原 falsified 节存档于下。

## ⚠️ 撤回（2026-06-21，编排者质疑后重做诊断）

编排者质疑 C3 前提：「panic 只在做空亏了时触发。先查 panic 那一刻具体发生了什么，不要急着改 §8.1 或接受'卖点会失败'。」

**重建 P3b（按 summary 描述的 BSP 触发 reconcile）+ 三模式（Structural/And/Or）诊断脚手架跑全量 BTC（4.6M bar），结果推翻 C3 前提**：

| 模式 | strat | sink | recover | §8.1 panic | short_leg_pnl |
|------|-------|------|---------|-----------|---------------|
| Structural | +1423.82% | **0** | 0 | **0** | 0 |
| And | +1423.82% | **0** | 0 | **0** | 0 |
| Or | +1423.82% | **0** | 0 | **0** | 0 |

**C3 falsified**：三模式 §8.1 panic 全为 0——亏损短差 panic **不重现**。summary 记录的 `free=18522<need=19394` 是 P3b 迭代中某个 sink 触发条件不同的中间版本（未提交、已丢失）的偶发；当前忠实重建 sink=0，根本到不了 §8.1。

**真因（远比 §8.1 重要）= sink 完全不触发，BSP 卖点零消费**（type1_sell 产出 2927/2621/3615，全部落空）：
- **时序滞后（want 方向不符=9962/10764/9950，最大头）**：type1_sell（背驰确认）滞后于回调——卖点 fire 时 extract_chain 投影的回调走势已 `completed=true`，无"未完成回调"可骑次级别空头（[[project_interval_nesting_forward]] 确认滞后 / [[project_recl2_confirm_breakpoint_temporal]] 时序错配同构）。
- **级别×深度错配 + 自锁死（want=None=6151/2631/6162）**：reconcile 单链下钻，sink 成功才下钻；sink 在最高级别就失败 → 链永不下钻（depth 恒 0，my_level 恒 core_level）→ 只检查最高级别 type1_sell（L4 仅 30/1/38 个）→ 低级别大量 BSP（L0:1692/L1:792…金字塔）**永不被触及**。回调链深度分布 len=1 占~50%（牛市顺势链浅）。
- **根因**：重建的 P3b 的 sink 仍用 extract_chain 的"当前回调走势节点（want）"作做空载体 gate = **仍是走势结构驱动**，没有真正"消费 BSP"。C1 北极星要求 sink 直接由 BSP（卖点级别+价位即载体）触发，与单链下钻 + 走势节点 gate 结构冲突。

**结论**：C1 落地的真正障碍是 **sink 载体表示（BSP vs 走势节点）+ 单链 vs 多级别金字塔 + 卖点确认滞后**，不是 §8.1。§8.1 fail-loud 未被证伪（从未到达）。下一步见正文报告 + 待编排者定方向（BSP 直接触发删 want gate / N 独立级别引擎 [[project_t_multiscale_independent_filters]]）。

---

## 【以下为已证伪的原报告，存档】

## 选择报告

### 背景

C1 裁决（2026-06-20）：核心持多骑牛永不翻空，**所有卖点=短差**（reduce 本层 1/3 + 次级别开空），删 flip/promote/Z₂。
- **P3a**（走势触发 sink/recover）已落码验证：BTC NT 真账本 **+1423.44% > BH +1380.38%，首次超 BH**（核心 99.9% 持多 / 0% 空）。但 sink/recover 仅 5 次走势触发 = 基本持多骑牛 buy-and-hold，**齿轮/多重赋格未激活**。
- **P3b**（BSP 触发 sink/recover，激活全级别短差，C1 北极星"消费 BSP"的真正实现）落码后 **panic**：
  `free 不足以同股数多头回补（§8.1）：free=18522.56 < need=19394.80, c=4573`（rec_engine.rs:523）。

### 事实部分（L0 数学 + L2 触发）

**同股数恒仓回补的现金约束**（设核心恒仓 ⟹ free_before_sink≈0）：
- sink：核心在 c1 卖 m 股（free += m·c1），次级别在 c1 开空 m 股（free += m·c1）⟹ free += 2m·c1。
- recover：次级别在 c2 平空 m 股（free −= m·c2），核心在 c2 回补 m 股（free −= m·c2）⟹ free −= 2m·c2。
- 核心回补瞬间需 free ≥ m·c2 ⟹ 约束化简为 **free_before_sink ≥ 2m(c2 − c1)**。
- **c2 > c1（亏损短差：type1_buy 回补价高于 type1_sell sink 价）且 free_before≈0 ⟹ 必 panic**。

**为什么 P3a 不触发、P3b 触发**：P3a 走势触发的 5 次短差恰好 c2<c1（盈利）；P3b BSP 触发激活更多短差，**命中 c2>c1 的亏损短差**。

**c2>c1 的缠论本质**：type1_sell（一类卖点=背驰确认）**会失败**——背驰确认不是 100% 准确（[[project_t_short_leg_regime_function]] 跨 8 标的 L3「空头腿=亏损唯一来源」/ [[project_zombie_short_diagnosis]] recover 率级别衰减）。卖点失败 ⟹ 价格继续涨 ⟹ 次级别空头浮亏 ⟹ type1_buy 在更高位 fire ⟹ 恒仓回补现金不足。

### 矛盾的本质（概念层）

**§8.1 fail-loud 的原始裁决**（[[project_recursive_t_architecture_v2]] §8）：
> 「同股数：卖 N@P1 收 N×P1，回调终点**必低于起点**(P2<P1)否则非回调=结构判断有误。free 不足=结构检测 bug，fail-loud panic，不 cap、不接受残量。」

这条裁决**预设了卖点不会失败**（回调终点必低于起点）。但 C1 北极星要求**消费 BSP**，而 BSP 消费立即暴露：缠论卖点**会**失败（背驰确认的经验本质）。⟹ **§8.1 的 fail-loud 假设被 C1 的 BSP 消费证伪**。

三条要求在"假卖点亏损短差"上**不可同时满足**：
| 要求 | 来源 | 在亏损短差上 |
|------|------|------------|
| (R1) 核心恒仓 units 不减 | C1 核心持多骑牛 | 要求回补 m 股 |
| (R2) 资金守恒 free≥0 无凭空现金 | 双层记账（[[feedback_recursive_nested_fugue]]） | 亏损短差释放现金 < 2m·c1 占用 |
| (R3) 同股数回补 | §8.1 | 恒仓下 free≈0 必不足 |

R1∧R2∧R3 ⟹ free_before ≥ 2m(c2−c1) ≥ 0，在 c2>c1 时与 free_before≈0 矛盾。**fail-loud 假装这是 bug 而 panic，但它是真实的亏损短差**——panic 吞掉了"卖点失败"这个真实信号。

### 三方论证

**读法 A（同金额回补，放弃严格 R3）**
- 亏损短差平空（如实记账亏损）+ 用**释放的现金**回补核心（同金额，回补更少 units）。核心 units 被该笔短差亏损侵蚀。
- 依据：[[feedback_bidirectional_always_in]] 资金守恒；第31课「同股数」用于①降成本②退本金（核心**盈利时**操作），不覆盖短差亏损回补；亏损必须落地（绩效=Σ|涨跌幅|）。
- 代价：放弃严格"核心 units 恒定"——但"恒仓"重定义为**方向恒定（持多骑牛）**而非 units 绝对不变。亏损侵蚀 units 是诚实会计。
- 谱系警示：[[project_earning_shares_empty_domain]]「同金额降成本短差本身净负值」——同金额口径在 1min 实测净负，慎用。

**读法 B（同股数 + free 可负=杠杆/借根池，放弃严格 R2）**
- 核心恒仓 units 维持，亏损现金缺口从根池借（free 可负=杠杆）。
- 依据：恒仓骑牛严格保留（R1）；[[project_notional_leverage_research]] 名义敞口杠杆框架。
- 代价：引入杠杆（free<0），违背"无杠杆/逐仓"假设；亏损短差被杠杆放大（[[project_t_cross_level_coupling_falsified]] 单一池放大 −89.7% 穿仓前车）。

**读法 C（§8.1 假设修订——卖点失败是合法状态，非 bug）**
- fail-loud 的"结构检测 bug"前提错误：c2>c1 不是 bug 而是缠论卖点失败的合法显形。删 panic，改为 A 或 B 的会计处理 + 记录"假卖点"统计量（regime 观测）。
- 依据：缠论买卖点本质会失败（背驰≠100%）；§8.1 假设是架构 v2 在"未消费 BSP"时代的产物，C1 消费 BSP 后该假设失效。
- 注：读法 C 不是独立第三选项，而是"承认 §8.1 假设须改"——改后仍需在 A/B 间选会计口径。

### 不可弥合点

读法 A 牺牲核心 units 严格恒定（恒仓→方向恒定），读法 B 牺牲无杠杆（free≥0→可负）。两者都要求**先修订 §8.1 fail-loud 假设**（读法 C）——而 §8.1 是编排者已结算裁决（[[project_recursive_t_architecture_v2]] §8），修订须编排者裁决。且触及第31课"同股数"是否覆盖"短差亏损回补"的原文解读（同股数原文用于降成本/退本金=盈利侧操作，短差亏损回补是否同口径＝原文留白）。

**不可由定义自决**：§8.1 修订 = 改已结算裁决的适用范围（testing-override 规则：修复需改定义边界 ⟹ 定义冲突，停下上浮，非实现错误）。

### 推荐（待编排者裁决）

**倾向读法 A + C**（修订 §8.1 假设 + 同金额回补）：
1. P3a 已证 +1423% > BH 稳定基座**已提交**（cc017e4e/28f691d），上浮期间引擎处于验证态，无回退风险；
2. 读法 A 符合资金守恒（R2 不破）+ 双层记账谱系，亏损如实落地；
3. 读法 B 的杠杆放大有 [[project_t_cross_level_coupling_falsified]] −89.7% 穿仓前车；
4. "恒仓"重定义为方向恒定（C1 核心持多骑牛的本意是**永不翻空**，非 units 绝对冻结）与 C1 原意一致。

但 [[project_earning_shares_empty_domain]] 警示同金额口径 1min 实测净负 ⟹ 读法 A 可能把"假卖点亏损"变成系统性失血，须 L3 回测验证。且第31课"同股数"覆盖范围的原文解读须编排者确认 ⟹ 上浮。

**注**：本 escalation 是 [[project_recursive_t_architecture_v2]] §8.1 fail-loud 裁决的有效域质询——编排者 §11.6-3 已预警"recover 守卫硬编码多头语义须按 direction 镜像"，C3 是其延伸：不仅镜像方向，还要处理**卖点失败**这个 §8.1 假设未覆盖的合法状态。
