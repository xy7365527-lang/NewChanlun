# 三阶段会计总体性重构 —— 信号/操作/会计三层统一设计

> 编排者裁决 2026-06-20。本文推翻 `recursive_t_architecture_v2.md` §8.7「每实例独立 cost_basis/phase」，
> 把三阶段会计从 per-instance 上移到根账本（TRoot）总体口径，镜像空头，统一增股数两层，载体通用化。
> 13-agent 工作流（原文溯源 + 四视角设计 + 三对抗验证）综合产物。**纯设计，未改代码。**

## 0. 命题与原文确证

### 0.1 编排者连续裁决（推翻 §8.7）

1. **三阶段是总体性的，不是 per-layer**：cost_basis/phase 在 TRoot 追踪，不在每个 TInstance。所有级别的短差利润汇聚降低**同一个**总体成本。phase 是全局状态。
2. **方向对称**：空头三阶段是多头的镜像。降成本/退本金/增股数对多空都适用。
3. **增股数两层**：持续增（各级别短差，EarningShares 阶段同金额→更低价买回更多 units，每次短差都发生）+ 总结算增（最高级别翻转=完全平仓+反手，全量反向建仓，新初始仓>旧初始仓）。
4. **标的通用**：现货/期货/加密永续/期权四载体，会计结构统一，载体不同（期权提供空头损失边界）。

### 0.2 原文确证（L0，最终权威=博文，逐字）

- **三阶段总体性**：第31课:24（=chan99/0033:21）「当成本为0以前，要把成本变为0；成本变成0以后，就要挣股票，直到股票见到历史性大顶（月线以上卖点）」。第31课:169 缠师答疑「成本为0前，只补进相同的数量，仓位不增加；成本为0后……股票才会越来越多」⟹ **阶段边界（成本穿0）是改写全部级别买卖规则的单一全局事件**。chan99/0033:11「级别的意义其实只有一个，基本只和买卖量有关……在持股成本变成负数前，仓位是一直不变的」⟹ **级别只决定『量』(units)，成本/仓位/阶段是整只持仓的总体属性**。
- **增股数两层**：chan99/0033:23「成本为0以后……利用每一个短差，上面抛了以后，都全部回补，这样股票就越来越多，而成本还是0」（持续增）+「等待一个超大级别的卖点，一次性把他砸死」「牛市结束前才把所有股票全部清仓」（总结算增）。
- **方向对称**：第26课:34「无论先买后卖与先卖后买，效果是一样的」「本ID的仓位一直不变（恒仓），卖点变少，买点回复原数量，绝对不加仓」。
- **空头立于不败=认沽期权**：第15课:976（实操五粮液认沽038004）「把剩下的成本是0了……只持有成本是0的仓位等待第二波……绝对立于不败之地」+ :956「该权证风险极大，最终要变成废纸」（损失=权利金有界）。
- **现状代码确证（L2）**：flat `t_engine.rs:105` `TStage` 已是 GLOBAL（`self.stage` + `core_cost_basis:209` + `core_long_units:288` 跨 ladder 汇聚）——**总体三阶段是 flat 的原始设计**；rec_engine.rs 改 per-instance（`RecStage` 每 TInstance:132 + `account_core_reduce` 取 slot:360 + 降本分母退化单实例:364）才是偏离 = §12 方向不对称根因。

### 0.3 地基原则：绩效=Σ|涨跌幅|，永远在市场有方向（零真空）

整个设计的地基是一条度量原则：**绩效 = Σ|涨跌幅|**——把每一段走势的绝对涨跌幅都吃到。
推论（贯穿三层）：
- **永远在市场，永远有方向**：走势向上就持多、向下就持空，**不存在空仓真空期**（空仓=丢失该段绩效）。这是 §9.3 `promote`（回调短头升格新核心，零真空）的存在论根据，也是「最高级别翻转=完全平仓+反手」（§4.2，不是平仓观望）的根据。
- **方向跟着走势走**：核心方向锚定走势方向（操作层 A2：方向由所骑走势节点定，非 BSP 推断），不是择时进出。
- **降成本是「在场」的副产品**：因为永远在场、永远来回短差，所以 Σ短差利润持续降低总体成本（三阶段的物质来源）。三阶段不是额外策略，是「永远在场吃绝对涨跌幅」的会计形式。

> 这条原则解释了为什么「50% 空头不回补」是致命的：不回补 = 该段下跌后的上涨绩效丢失 + 空头被轧（负绩效）。零真空要求**任何一段走势都不能漏**，多→空→多必须完整闭合。

## 1. 三层架构与唯一焊接接口

```
信号层(Signal)          操作层(Operation)              会计层(Accounting)
divergence.rs           rec_driver + TInstance 树      TRoot 总体 campaign
─────────────           ──────────────────────        ──────────────────
c段已修复,方向对称  →   消费 TrendNode 做 reduce/add → realized 经唯一入口汇聚
输出 BSP/走势节点        每级别只跟直接父子通信         总体 cost_basis/phase 状态机
(§12.1 严格镜像)        最高级别翻转=完全平仓+反手     成本穿0→退本金→增股数
```

**唯一焊接点（操作层→会计层，回答 Q1/Q5）**：所有 reduce（sink/recover/promote/clear/强平）产生的 realized **必经单一全局入口**：

```rust
TRoot::account_campaign(&mut self, role: ReduceRole, realized: f64, c: f64)
```

取代现状 `account_core_reduce(slot, realized, c)`（rec_engine.rs:360，删 slot 参数）。
操作层不读会计字段；会计层不碰拓扑。`rec_driver.rs`（:85-162 只读 node/direction/units/basis/child）+ `ffi.rs` **零改动**，改动面仅 `rec_engine.rs`（TRoot/TInstance 结构 + ~8 方法）。

> **对抗修正 #1（REFUTED→重设计）**：`role` 不能用 `direction` 判别核心 vs 短差腿（嵌套孙 T 可与核心同向）。
> 必须用**拓扑身份**：核心 = `root_slot` + spawn relabel 继承链（无 parent）；短差腿 = 任何有 parent 的 sink 子 T。
> 调用点已知 role：sink 减的是父（核心 spine 上则降成本）/recover 减的是子腿（leg_pnl）/promote 减的是旧核心（降成本入总结算）。

## 2. 字段重新划分（TInstance 瘦身 / TRoot 总体三阶段）

**依据**：chan99/0033:11「级别的意义只和买卖量有关」⟹ 实例只决定买卖『量』(units)，不决定成本/阶段。

### TInstance（纯仓位身份+拓扑，保留 8，删 5）
```
保留: node:TrendNode / direction:Polarity / units:f64(≥0,符号在direction禁sign位)
      / basis:f64(per实例进场均价,强平+逐笔pnl用,不可上移——每条腿basis独立)
      / parent / child / lifecycle / generation
删除: cost_basis(:131) / phase:RecStage(:132) / notional_in(:134) / withdrawn(:136) / earning(:138)
      ——这5个是总体属性,per-instance持有=§12方向不对称根因(降本只作用单slot,跨实例利润不汇聚)
```

### TRoot（新增 5 字段，镜像 flat t_engine.rs:203-221）
```
campaign_cost_basis:f64   // 总体核心有效成本/share,可<0,NaN=无campaign (= flat core_cost_basis:209)
campaign_phase:RecStage   // 单一全局状态机 CostReduction→CapitalRecovered→EarningShares (= flat stage:203)
campaign_notional_in:f64  // 总体投入本金K (= flat notional_in:205)
earning_cash:f64          // ③阶段总弹药,free子账 (= flat earning_cash:215)
campaign_direction:Polarity // 总体核心方向(=instances[root_slot].direction,翻转时更新)
复用: free / withdrawn_total (已有)
```

## 3. Q1：realized 如何流入总体 cost_basis

```rust
fn account_campaign(&mut self, role: ReduceRole, realized: f64, c: f64) {
    match role {
        ReduceRole::Core => match self.campaign_phase {  // 核心 spine 减仓(sink父/promote旧核心)
            CostReduction => {
                let rem = self.core_units_on_spine();  // ★ 跨实例汇聚核心 spine units(非单实例,非全同向)
                if rem > EPS && self.campaign_cost_basis.is_finite() {
                    self.campaign_cost_basis -= realized / rem;       // 降成本(方向无关:realized已按direction正确)
                    if self.campaign_cost_basis <= 0.0 {              // 穿0 → 退本金
                        self.campaign_phase = CapitalRecovered;
                        self.try_withdraw();                          // free→withdrawn,移本金到安全池
                    }
                }
            }
            CapitalRecovered => self.try_withdraw(),                  // 继续退至 notional_in 全收回 → EarningShares
            EarningShares => if realized > 0.0 {
                self.earning_cash = (self.earning_cash + realized).min(self.free.max(0.0)); // ③弹药
            }
        },
        ReduceRole::Leg => self.short_leg_pnl += realized,            // 短差腿(recover子T)单独算,不入降成本(点5)
    }
}
```

- **降本分母 `core_units_on_spine()`**（对抗修正 #1）= 汇聚 `root_slot` 实例 + spawn relabel 链同身份实例的 units，**排除任何有 parent 的 sink 子 T**。移植 flat `core_long_units:288` 的跨 ladder 汇聚，泛化为跨实例 spine 汇聚，修复 rec_engine.rs:364 分母退化为单实例。
- **方向无关**：`realized` 由 `rec_reduce` 按 direction 已算正确（Long=m(c−basis) / Short=m(basis−c)，:178-179），故 account_campaign 对多空核心**同一公式**降成本——这是 §12 不对称的根治（Short 不再落 short_leg_pnl 平铺）。
- **TW 中性不破**（对抗验证 #1 confirmed）：cost_basis/phase 是**虚拟会计量，从不进 TW 公式**（`total_wealth` 只读 units/direction + withdrawn_total，:268-284）。字段上移对 TW 守恒零影响。

**退本金结合买卖点（非纯阈值，编排者澄清）**：第31课:28「在股票达到 1 倍升幅附近找一个**大级别的卖点**出掉部分，把成本降为 0，原来投入的资金就全部收回来了」。`cost_basis≤0` 是退本金的**使能条件**，但执行须对齐一个**结构性卖点**（reduce 事件），不是「数值穿 0 就立刻 withdraw」的脱离市场结构的阈值平仓。落码含义：`try_withdraw` 由「跨 0 的那次 reduce（本就发生在卖点）」触发即自然对齐——但若未来加独立 withdraw 路径，必须门控在卖点，禁止纯价格阈值触发（与 §0.3「方向跟走势走」一致，杜绝择时化）。

## 4. 增股数两层（统一律 + Q2）

**统一律**：增股数两层是同一个全局 `campaign_phase==EarningShares` 标志的两种时间尺度表现，**不是两个机制**。
> 成本=0 后，任何把现金换成 units 的操作都用**同金额 cash/c_now** 而非同股数。

### 4.1 持续增（过程层，各级别短差）
recover（子 T 平空升回）时读**全局** `self.campaign_phase`（现状 rec_engine.rs:514 读 `instances[parent].phase` 必须改为读 `self.campaign_phase`）：
- `CostReduction/CapitalRecovered` → q = m_short（**同股数**，恒仓回复，第31课:169「补进相同数量」）；
- `EarningShares` → q = `earning_cash.min(free)/c`（**同金额**）。因 c_now<c_sink（回调终点<起点，§8.1），q>m ⟹ Σunits 单调增、cost_basis 钉 0。

弹药=全局 `earning_cash`（非 per-instance 份额），门=全局 phase。**不需等最高级别平仓，每个短差都在发生。**

### 4.2 总结算增（周期末层，最高级别翻转）= Q2
操作层 promote/flip（顶级方向反转，emergent_dir 确认）→ 会计层 `settle_and_increment()` 原子序列（GLOBAL，TW 中性）：
```
① Σ全树清仓 realized 经 account_campaign 累入(降成本/earning)
② 归还 withdrawn_total → free
③ free = K_old + Σ短差降成本 + short_leg_pnl + 增股数浮盈 = total_cash
④ 新核心 enter: campaign_notional_in=total_cash, u_new=total_cash/c_new > u_old (新初始仓>旧初始仓)
```
> **对抗修正 #3（PARTIAL REFUTE）**：现状 promote:612-614 已退还 withdrawn 到 free（house money 连本归，**非保留**）——故「立于不败跨翻转保留」表述需更正为「withdrawn 在总结算时归还、并入 total_cash 反向建仓」。且现状 promote:616-630 只 carry child 单实例 units，旧核心降成本成果进黑洞丢失——`u_new>u_old` 在 per-instance 下结构不可表达；GLOBAL 上移后才可表达。
>
> **有效域硬边界（L0，chanlun:21/169）**：成本转 0 **前**短差只降成本、**严格不增股数**（否则违恒仓 026:34「绝对不加仓」）。股数增长只在成本=0 后。

## 5. 空头三阶段镜像 + 四载体立于不败梯度（Q3）

**双层结构（对抗验证 #2 survived 并加固）**：
> **降成本机会（可达，载体无关）⊥ 立于不败终态（载体相关，裸空不可达）**

- **降成本机会**：方向对称在「制造降成本机会」层 L0 成立（026:34 先买后卖/先卖后买效果一样）。空头 cost_basis 语义 = **开空均价**；降成本 = 低买回/高卖空降低净支出（镜像多头高卖低买）。可达，载体无关。
- **立于不败终态**：现货空头亏损无界（c→∞），无对称的「零成本天花板」（无价格上限）⟹ **「绝对立于不败=零成本仓位」在现货空头数学上不可表示**。这与 §12 诊断 + [[project_t_short_leg_regime_function]] 的 L3 否定结果收敛：空头不是多头的完全对称镜像。

**四载体可达性梯度（禁无条件声明，formalization-validity-domain）**：

| 载体 | 多头立于不败 | 空头立于不败 | 机制 |
|------|------|------|------|
| 现货 | 可达（最坏归零，下限0） | **不可达**（c→∞ 无界，无零成本天花板） | 降成本到 cost_basis≤0 |
| 期货 | 可达 | 部分（保证金/强平线截断，但强平=被动出局≠立于不败） | margin + 轧空尾 |
| 加密永续 | 可达 | 部分（同期货 + funding 流，正费率持空收 funding 降成本） | margin + funding |
| 认沽期权(买方做空) | — | **可达**（损失界于权利金，015:956 变废纸=有限全损） | 期权天然损失上限 |

**载体抽象（零 workaround，编排者终裁 2026-06-20）**：三阶段会计对**所有方向、所有载体一律对称**——空头核心和多头核心走**完全相同**的降成本/退本金/增股数代码，**不加任何 `can_reach_earning` 之类的方向门控**（那是「声明空头不可达回避问题」的 workaround）。

> **删除原 `can_reach_earning` 设计**：之前提议「`can_reach_earning(Short)==false` 禁止空头进 EarningShares」是回避问题的 workaround。正确做法：让空头三阶段对称地跑，**现货空头无界尾部风险若发生，体现在强平数据里（诊断信号）**——不预先 gate。如果空头被强平，说明 recover 没做对（买点没及时回补），是要修的真问题，不是要 gate 的对象。

载体差异（未来扩展）可经一个无状态 trait 表达 mtm/realized/liquidation_bound/carry，默认 `SpotCarrier` = 现状硬编码（Long=+u·c/Short=−u·c）。但**载体 trait 不含任何方向可达性声明**——它只描述物理量（市值/pnl/强平线），不判定「能否立于不败」。认沽期权（损失界于权利金，015:956）是**未来可选载体**（提供空头损失上限），不是用来 gate 三阶段的判据。

**本轮落码不引入 CarrierModel trait**（现货/加密 BTC 用现状硬编码 SpotCarrier 语义即可）——trait 是未来多载体扩展，非本轮零 workaround 修复的一部分。

## 6. 逐仓/全仓 vs 总体三阶段（Q4）—— 正交两层，不矛盾

「全程逐仓 vs 三阶段总体」是把两层同名词误当同一对象。**层分离**：
- **逐仓/全仓 = 操作层风险边界算子**（读 per-instance basis）：问「这个实例仓位会不会被保证金机制平掉」，取决于实际成交均价 basis 与现价 c 的距离。
- **总体三阶段 = 会计层成本状态机**（读 root cost_basis/phase）。

**耦合单向**（根 phase → 强平模式选择器，不可反向）。依据 chan99/0033:9「唯一风险就是投入的钱不能换成更多的钱」（风险相对注入本金 K，非名义价位）：
```rust
fn liquidate_boundary(&mut self, c, bar):
  match self.campaign_phase {        // ← 会计层选模式
    EarningShares => 全仓(root.nav(c)≤0 连锁全平)   // 本金已退(withdrawn隔离),仅利润缓冲吸收回撤,不触本金
    _ => 逐仓(每实例独立 l.basis 判,SUB_LIQ_FACTOR)  // 本金在险,单实例亏损不侵蚀核心仓保证金
  }
```
现状 t_engine.rs:709-745 已是此架构。**「全程逐仓」旧裁决在本金在险期成立；进入 EarningShares 切全仓是总体 phase 的必然推论，非违反。** 空头降成本后**强平判据仍用裸 basis（per-instance）**，不用降成本后口径——强平是 operation 层算子，问物理成交价 vs 现价；立于不败（cost_basis≤0）改变的是**强平模式（逐→全）而非取消强平**（现价暴涨仍可触发空头强平，轧空尾在强平前即出局）。

## 7. Q1–Q5 接口总表

| 问题 | 答案 |
|------|------|
| Q1 realized→总体cost_basis | 唯一入口 `account_campaign(role,realized,c)`，role 由拓扑身份判（核心spine=降成本/腿=leg_pnl），降本分母=`core_units_on_spine()` 跨实例汇聚 |
| Q2 最高级别翻转→增股数总结算 | promote/flip→`settle_and_increment()`：全树清仓→归还withdrawn→total_cash→反向enter，u_new=total/c_new>u_old |
| Q3 空头立于不败 | 降成本机会可达(载体无关)⊥立于不败终态(载体相关)：现货裸空不可达/认沽可达/期货部分。`CarrierModel::can_reach_earning` 声明 |
| Q4 逐仓/全仓 vs 总体三阶段 | 正交两层，单向耦合：root.phase 选强平模式（本金在险逐仓/已退全仓），t_engine:709 现成架构 |
| Q5 per-instance vs 总体 接口 | TInstance 瘦身{node,direction,units,basis,parent,child,lifecycle,gen}；TRoot 持{campaign_cost_basis,phase,notional_in,earning_cash,direction}；焊接=account_campaign |

## 8. 落码改动面与开放矛盾

**改动面**（rec_engine.rs 局部，rec_driver/ffi 零改动）：
1. TInstance 删 5 字段；TRoot 加 5 字段。
2. `account_core_reduce(slot,...)` → `account_campaign(role,...)`（拓扑判 role + spine 汇聚分母）。
3. `core_units_on_spine()` 新方法。
4. recover:514 读 `self.campaign_phase`（非 instances[parent].phase）；q 用全局 earning_cash。
5. promote:616 改 `settle_and_increment()`（全树清算 total_cash 反向建仓，u_new>u_old）。
6. 强平模式选择器上移读 `self.campaign_phase`。
7. `CarrierModel` trait + `SpotCarrier` 默认（bit-exact 守卫）。
8. **`drain` 消融**：flat `t_engine.rs:574 drain`（子级持同父向遗留仓→反父向 BSP 减暴露 1/3）**不是独立操作，是三阶段缺失的 workaround**——它把「同父向仓位的减仓 realized」临时塞进 `account_reduce(jdir,…)` 自圆其说（:583）。总体三阶段落地后，任何 reduce 的 realized 都经唯一入口 `account_campaign(role,…)` 按 phase 分流（降成本/退本金/增股数），`drain` 退化为「核心 spine 上的一次 reduce（降成本）」或「腿 reduce（leg_pnl）」，无需独立分支。**drain 根因=三阶段没设计，非独立 bug**（删 drain，逻辑并入 account_campaign）。

**开放矛盾（须 escalate / 已 escalate，落码前不得自决）**：
- **§9.3 总结算反向腿 vs clear_root**：总结算增（§4.2 ④反向 enter）= 最高级别**反手做空**，与 `clear_root`（rec_engine:637「顶级=clear 不 flip，避 BTC −706%」）冲突 = §9.3 读法 A/B 未决（escalation `2026-06-20-t-macro-core-net-short-regime.md` + §11）。**未裁决前：总结算反向腿只对有 parent 的子级 flip 合法，顶级走 clear_root（不反手）**——即总结算增的「全量反手」在顶级被 §9.3 gated，会计结构（settle_and_increment）就绪但顶级反向方向待裁。
- **现货空头 EarningShares 不可达**：`can_reach_earning(Short)==false`（现货）须声明有效域，不可补丁伪装（formalization-validity-domain）。期权载体的操作对象（认沽自身 K 线 vs 标的走势节点）= [[project_put_option_short_earning]] R1 缺口，载体层未定。
- **SUB_LIQ_FACTOR=2.0 / MOBILE_FRAC=1/3** 无原文依据（§8.2/§8.3 开放轴），本重构不触及。

## 9. 完整洞察整合（本轮对话发生史）—— 三层 + 跨层

本节把整轮对话的所有关键洞察无遗漏地归位到三层架构，标注解决处与谱系，确保不丢失。

### 9.1 信号层（Layer 1，已解决）
| 洞察 | 内容 | 解决处/依据 |
|------|------|------|
| **c段=实现错误非定义冲突** | 贪心中枢扫描（center.rs:79 `i=last+1`）把离开段 c 吸收进下一中枢 → cut_away → 77.6% 趋势无 c 段 → 趋势背驰判不出 → type1 全塔衰减。**第27课「两个中枢不可能共用一个次级走势」**证明这是扫描实现错误（连接段归属），不是定义矛盾。修复 `i=last+2` → c段缺失 79%→0%、type1 250→1927、**level3 type1_buy 0→2 涌现** | c段 escalation `2026-06-20-t-c-segment-connection-attribution.md`；[[project_c_segment_fix_regime]] |
| **买卖信号方向对称** | `judge_trend_divergence` 全程 `t.direction` 参数化（创新高低/Type1Sell↔Type1Buy/守沿/ZG↔ZD），无 Long/Short 硬编码 | §12.1（divergence.rs:191-209/242-245） |

### 9.2 操作层（Layer 2）
| 洞察 | 内容 | 解决处/依据 |
|------|------|------|
| **绝对 ladder vs 相对级别的裂缝=病根** | flat 引擎用绝对 ladder 数组（固定级别槽），但走势级别是**相对涌现**的（r* 随数据涨落）。绝对/相对错配 ⟹ 空头绑绝对 ladder、平空落错 ladder、−89.7% 穿仓。这是 level 不匹配的根 | [[project_t_cross_level_coupling_falsified]]；v2 §0/§6 |
| **T 算子自相似递归解决 level 匹配** | 每级别一个 T 实例**骑走势节点（TrendNode）**而非绝对 ladder，父子按走势节点身份（start_bar）通信，级别涌现=spawn。从坐标层消除绝对/相对裂缝 | v2 §1-§3；[[project_recursive_t_architecture_v2]] |
| **promote 消除真空期** | 转折时回调短头（自 sink 已持新方向，t1）relabel 为新核心、旧核心清，**零真空**（绩效=Σ\|涨跌幅\| 要求，§0.3）。取代 flip（清+反向开仓）/clear（清+等待） | v2 §9.3 |
| **多→空 与 空→多 应对称** | 信号方向无关 ⟹ 卖点翻空与买点翻多应完全对称 | v2 §11 |
| **定义域对称但 base case 有效域不对称** | 缠论原文（37课条件4/29:52/27课区间套）+ T算子定义域（promote 方向无关）**对称**；最高级别 base case 有效域**不对称**=三重合取：**时间之矢**（做空反转已完成的过去结构/回补 construct 未涌现的未来结构）+ **无 return address**（顶级 parent=None，回补必为构造非返回）+ **regime 放大**（长牛回补载体 type1_buy@high 枯竭）。不对称是结构常量，破坏性是 regime 函数 | v2 §11.4-11.5 |

### 9.3 会计层（Layer 3，本文核心）
| 洞察 | 内容 | 解决处 |
|------|------|------|
| **三阶段会计只给多头是 50% 不回补根因** | account_reduce/account_core_reduce：Long→三阶段降成本立于不败；Short→short_leg_pnl 平铺。空头无降成本→永远逐仓可强平→牛市续涨中 recover 前被强平→不回补 | §12.3；本文 §3（镜像修复） |
| **三阶段是总体性的（根账本，非 per-instance）** | cost_basis/phase 上移 TRoot；级别只决定买卖量（chan99:11）。§8.7「每实例独立」被推翻 | 本文 §0-§3 |
| **reduce 的 realized 立刻计入降成本，短差腿 P&L 分开核算** | 每次 reduce 经 account_campaign：核心 spine reduce→立即降 cost_basis；短差腿（recover 子T）reduce→short_leg_pnl 单独算（点5）。role 由拓扑判 | 本文 §3 |
| **退本金结合买卖点，非纯阈值** | cost_basis≤0 是使能条件，执行对齐大级别卖点（第31课:28），禁脱离市场结构的阈值平仓 | 本文 §3（weave） |
| **增股数两层** | 持续增（各级别短差 EarningShares 同金额→更低价→更多 units，每次都发生）+ 总结算增（最高级别翻转全量平仓反手） | 本文 §4 |
| **最高级别翻转=完全平仓+反手=总结算时刻** | 一次性结算全级别累积降成本+增股数+做空利润，全量反向建仓，u_new>u_old | 本文 §4.2（顶级反手被 §9.3 escalation gated） |
| **次级别也能增股数** | EarningShares 阶段，各级别短差同金额 cash/c_now（c_now<c_sink）→更多 units，不需等最高级别平仓 | 本文 §4.1 |
| **三阶段与逐仓/全仓耦合** | 阶段一（CostReduction/CapitalRecovered，本金在险）逐仓保护核心本金；退本金后（EarningShares，本金已退 withdrawn 隔离）全仓——单层回撤被利润缓冲吸收，**本金风险被消解**，只损利润不触本金 | 本文 §6 |
| **期权=空头损失边界工具** | 现货裸空亏损无界→立于不败不可达；认沽期权损失=权利金有界→空头立于不败唯一可达载体（第15课:976/956） | 本文 §5 |
| **标的通用** | 现货/期货/加密永续/期权四载体，会计结构统一进 TRoot，载体差异进 CarrierModel trait | 本文 §5 |

### 9.4 跨层失效模式诊断
| 失效 | 三失效点跨三层 |
|------|------|
| **50% 空头不回补** | (a) **买点缺失**=信号层（type1_buy@high 被 c段缺失+regime 打稀疏，翻回多触发器不来）；(b) **level 不匹配**=操作层（绝对 ladder vs 相对级别裂缝，平空落错级别）；(c) **量不够**=会计层（短差 1/3 不足以翻核心 + 空头三阶段缺失→被强平）。三失效点分属三层，须三层协同修复 |
| **drain 根因=三阶段没设计** | drain（同父向减暴露 1/3）不是独立 bug，是三阶段缺失的 workaround；总体三阶段落地后并入 account_campaign（§8.8） |

### 9.5 三层接口闭环（一句话）
**信号层**产出方向对称的走势节点+买卖点 → **操作层**递归 T 实例骑走势节点、跟方向、promote 零真空、最高级别翻转完全平仓反手 → **会计层**所有 reduce 的 realized 经 `account_campaign` 汇聚总体 cost_basis、三阶段（降成本/退本金/增股数两层）、CarrierModel 载体差异。三层经 `view.nodes`（信号→操作）与 `account_campaign(role,realized,c)`（操作→会计）两个接口焊接，正交解耦。

## 10. 结果包六要素

- **结论**：三阶段会计上移到 TRoot 总体口径（cost_basis/phase/notional_in/earning_cash/direction），唯一焊接点 `account_campaign(role,realized,c)`（拓扑判 role，spine 汇聚降本分母），方向对称（多空核心同一降成本公式），增股数两层（持续增=各级别短差同金额/总结算增=最高级别翻转全量反手 u_new>u_old），四载体经 `CarrierModel` trait 统一（空头立于不败=载体函数，现货裸空不可达），逐仓/全仓由总体 phase 单向驱动。TInstance 瘦身为纯仓位身份。
- **定义依据**：第31课:24/169（三阶段总体+成本穿0全局门+成本0前同股数不增仓）；chan99/0033:11（级别只决定量）/:21/:23（增股数两层+总结算）；第26课:34（恒仓+方向对称）；第15课:976/956（认沽立于不败+权利金损失边界）；flat t_engine.rs:105/203-289（GLOBAL 蓝本）。
- **边界条件（结论翻转）**：(a) 若降本分母用 direction 而非拓扑 spine，嵌套孙 T 同向被误计入核心（对抗 #1，已修正用 spine）；(b) 现货空头若声称 EarningShares 可达=声明膨胀（须 can_reach_earning 声明）；(c) 总结算反向腿在顶级 = §9.3 未决，落码前顶级走 clear_root；(d) 上移是否修复 §12 不对称及载体收益差异 = L2/L3 待验，不可声称带来 alpha。
- **下游推论**：(1) §8.7「每实例独立 cost_basis/phase」被推翻，回归 flat GLOBAL 模型；(2) rec_engine.rs 局部重构（结构+8方法），rec_driver/ffi 零改动；(3) 落码后每步 NT 回测验证守恒+§12 不对称是否消除；(4) §9.3 escalation 解决前，总结算增的顶级反手 gated。
- **谱系引用**：推翻 §8.7（[[project_recursive_t_architecture_v2]] 三裁决之会计轴）；[[project_put_option_short_earning]]（认沽空头 earning 凸性载体消解，四载体立于不败的原文锚）；[[project_earning_shares_empty_domain]]（挣股数空头空有效域）；[[project_bidirectional_accounting]]（双账本极性分离+空头单相单律非对称定理）；[[project_t_short_leg_regime_function]]（做空腿亏损 L3）；§9.3 escalation（总结算反手 gated）；§11/§12（回补难+方向不对称诊断）。
- **影响声明**：本文 = `docs/three_phase_unified_design.md`，13-agent 工作流综合 + 三对抗验证，**未改任何代码**。落码含义：rec_engine TRoot/TInstance 重构 + account_campaign 焊接 + CarrierModel trait（下一步实装，每步 NT 验证）。推翻 v2 §8.7，精化 §4（三阶段）/§9.3（总结算反手 gated）/§12（不对称根治）。
- **影响声明（认识论等级）**：§1-§4 字段划分/account_campaign 公式/降本分母汇聚 = **L0**（第31课总体定义 + flat GLOBAL 蓝本代数推导，零数据依赖）；§5 空头立于不败可达性 = **L0 数学**（现货空头无界）+ **L0 文本**（第15课认沽）；§6 逐仓/全仓层分离 = **L0**（t_engine:709 现成架构）；上移是否修复 §12/带来载体收益 = **L2/L3 待验**（不得声称 alpha）。
