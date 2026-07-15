# 互斥证明体系的盘整缺口回溯审计（证伪优先）

日期：2026-07-14

审计对象：`docs/formal-chain/proofs-full-strategy-20260703.md`（全文 405 行）

口径：只读；未修改旧证明、生产代码或测试

认识论规则：`.chanlun/genealogy/settled/231-formalization-validity-domain.md`

## 结论摘要

1. **§A 按现文不成立**：假设 5 写成成本函数 `𝔠_Θ`，证明链却用它推出状态分类 `z_t` 唯一；这是原论域内即可构造反例的真矛盾，须 escalate。
2. 扩域后，`dir=None`、盘整背驰及离开—回抽未判决窗口没有进入旧 `z_t/Γ_t/P_1..P_10` 的同一全函数链；若拒绝默认吞并，`∃!O_{t+1}` 的复合全函数证明断裂。
3. 若把这些状态统一投影为无候选/`C_0=Hold`，运行时仍可唯一出单，但证明的是旧策略的保守总化，不是盘整协议的语义完备性。
4. **§B 本体不失效**：`Σ_j 1[C_j]=1` 对任意布尔谓词向量仍是 L0 恒等式；扩域暴露的是谓词语义不完备，而非互斥化代数错误。
5. 区间边界开平、盘整背驰减仓、三类点协议切换均不在当前 P1..P10 的字面动作语义中；盘整背驰证书甚至明确“不入谓词”，可被 `Record/Hold` 静默吸收。
6. **T1 按题述被否证**：在线三分缺 `Pending`；即使 3 类回抽已确认，仅凭 `[ZD,ZG]` 也不能区分新生与扩展，后者必须读后继已完成中枢的 `GG/DD`。
7. **T2 的排他不变量可证，切换语义不可证**：固定优先级可保证恰一协议活跃，但“3 类确认 ⇒ 趋势”被“3 类后可能扩展成更大级别盘整”直接反例否定。
8. **T3 仅修正版可证**：任何趋势形成前必经过“单中枢 Active 前缀”；不能把该前缀冒充一个已经 Completed 的盘整走势对象。

## 假设提取表

### 1. §A 明列的 11 条假设

| # | 文档中的数学职责 | 行号锚 | 本审计抽出的隐含域前提 |
|---|---|---|---|
| 1 | `T_t` 全定义且 `T_t^inc=T_t^full` | `proofs-full-strategy-20260703.md:51-61` | “结构塔可计算”被当成“所有消费语义也已定”；未确认 frontier 只有禁止复用，没有成为策略状态的 `Pending` 分支。 |
| 2 | `Γ_t` 有限 | `:65-69` | 候选宇宙等同于全部需处理事件；`Γ=∅` 被默认为“没有动作需要”，而非“协议事件没有候选编码”。 |
| 3 | `N^δ_{ℓ↓e}` 对任意合法输入全定义 | `:73-81` | `δ` 已给定且可方向化；定义只覆盖 `ℓ≥e`，未覆盖 `δ=None` 或离开—回抽尚未完成的资格态。 |
| 4 | `I_γ` 类型全定义；六态 partition 穷尽 | `:85-90` | `{B1,B2,B3,S1,S2,S3,StructBreak}` 被当成全部策略事件；“六态位置标签穷尽”被提升为“盘整/协议语义穷尽”。 |
| 5 | 文档写作“`𝔠_Θ` 是函数（成本函数）” | `:94-98` | 证明结构却要求“状态分类器 `C_Θ:x↦z` 是函数”（`:41`）；现文没有这条假设。 |
| 6 | `I_Θ` 使用固定优先级，等价于 P1..Pm 互斥化 | `:102-107` | 每个需要执行的动作都已先被某个 `P_j` 表达；固定优先级只解决冲突，不负责发现缺失动作。 |
| 7 | `AncOK` 是函数 | `:111-118` | 活动集只有持仓腿生命周期；协议态生命周期不在 `A_t` 或 `AncOK` 中。 |
| 8 | `TStage` 与 TW 事件全定义 | `:122-129` | 资金三阶段被当成全部离散状态机；另一个“盘整/趋势协议状态机”不存在。 |
| 9 | `K_Θ(x_t)≠∅` 且有限 | `:133-140` | 已有动作能先生成目标持仓；若协议动作未编码，`0` 仍让集合非空并掩盖语义缺失。 |
| 10 | `LexArgmin` 有固定平局规则 | `:144-151` | 目标函数只需在已有动作/目标持仓上唯一化；不会补出未进入可行集的盘整动作。 |
| 11 | `Schedule_Θ` 是函数，零差额为 Hold | `:155-159` | 策略输出被压成订单 `O`；“无订单但协议切换”的状态事件不在输出类型中。 |

### 2. 方向二值、已完美、分类全域性的隐含假设

| 编号 | 隐含假设 | 证据与行号 | 证伪意义 |
|---|---|---|---|
| DIR-1 | 策略方向轴 `δ` 是二值并且先于分类给定 | `N^δ` 的布尔值域见 `proofs...:73-80`；原 PDF `推导完全互斥分类.pdf` p1 明写 `Σ={+1,-1}`；现行操作角色仍写 `δ_g∈{+1,-1}`、水平穷尽依赖二值，见 `coverage.rs:875-891`。 | `MoveBlock.dir=None` 不是该方向轴的成员；不能在不扩值域/分支的情况下声称同一分类全域。 |
| DIR-2 | 盘整的无方向可安全降成“无候选/Flat/Ambient” | 旧文把 `trend_dir=None` 解释为不产一类（`proofs...:90`）；当前 `MoveBlock` 明定 `Consolidation=None`（`decompose.rs:43-60`），D3 裁定同样要求三态（`d3-direction-ruling-20260714.md:9-18`）。 | “不产趋势一类”是正确的；“因此没有任何盘整动作”不是推论。 |
| DIR-3 | 更高/父方向的 `None` 只影响统计分桶，不影响操作语义 | 旧证明承认 `sigma_higher:None`（`proofs...:285-290`）；现行 `MuClass` 多个字段允许 `None`，但核心 `delta:i8` 仍二值（`mu_estimator.rs:57-116`）。 | 状态容器能存 `None` 不等于解释器已定义 `None` 下的盘整协议。 |
| SETTLED-1 | `Γ_t` 的生产宇宙只有 Settled 候选 | 旧文自己承认 Live 候选无生产者（`proofs...:303-306`）；现行字段说明仍写候选恒 Settled、Live 无生产者（`mu_estimator.rs:117-126`）。 | 离开完成但回抽未完成时没有可表达的策略候选；这是在线状态，不是非法输入。 |
| SETTLED-2 | “走势终完美”可被当成当下已经完成 | 3 类点要求离开、回试均为完成次级别走势（`.chanlun/definitions/maimai.md:132-142`），正式确认在回试 `Move.settled`（`:245-273`）。 | “终将完成”是未来存在性，不提供当下分类值；未判窗口必须显式 `Pending`。 |
| SETTLED-3 | sealed frontier 之外不需要策略态 | 假设 1 只说 frontier 不得复用（`proofs...:51-61`），总证明却立即假定每步全函数（`:39-43`）。 | 禁止偷看未来是正确约束，但不能用“忽略 frontier”替代在线总化证明。 |
| TOTAL-1 | `∀x_t` 的量词覆盖原始行情状态且每链路均全定义 | 定理及复合论证见 `proofs...:33-43`。 | 一旦 `x_t` 包含 `dir=None`/`PendingRetrace`，必须给出对应的 `z/Γ/P` 像；否则全称量词过宽。 |
| TOTAL-2 | 六态位置 partition 等价于全部策略语义 partition | `proofs...:85-90`。 | 六态只解决走势位置/买卖点编码，不含协议 mode、回抽资格、盘整边界动作。 |
| TOTAL-3 | P1..P10 穷尽动作需要 | 定理把任意谓词向量变成唯一 `C_j`（`proofs...:171-196`）；契约动作清单见 `mutex.rs:6-17`。 | 该定理只穷尽 `{0,1}^{10}`，不证明“每个盘整动作需求都使某个 P 为真”。 |
| TOTAL-4 | `C_0=Hold` 是语义中性兜底 | `proofs...:175-183`、`mutex.rs:75-83`。 | 对“等待更多证据”可中性；对“已确认盘整背驰但减仓动作无谓词”则是静默漏动作。 |
| TOTAL-5 | 候选三桶守恒可推出市场状态动作完备 | 守恒只在 `Γ` 论域上，见 `proofs...:236-243`。 | `PanDivCert` 不进入 `Γ/BspPoint` 时，三桶再完美也审计不到它。 |
| TOTAL-6 | 残余 z 维缺口不影响全函数性 | 文档称缺口不影响 `∃!`（`proofs...:163-167`），同时承认 Live/CostBucket 等缺口（`:262-273,303-306`）。 | 对旧订单函数可能不影响决定性；对“完整扩域策略”则不能继续宣称完整状态 `z`。 |

PDF 对照结果：`pdftotext -layout` 成功抽取 `缠论的全互斥定义策略.pdf`、`缠论的全互斥定义策略2.pdf`、`推导完全互斥分类.pdf`、`完整的策略.pdf`。前两版与最终版均保留二值交易方向；`完整的策略.pdf` §6 的 `z` 含 `δ` 但没有协议 mode/离开—回抽 Pending，§7 的 P1..P10 与当前 `mutex.rs:6-17` 一致。故本审计不需要以 Markdown 代替无法读取的 PDF；PDF 抽取未失败。

## 扩域判定表

扩展状态空间取题设三项的直积：`dir∈{多,空,None}`、盘整默认/中枢震荡协议、离开段完成而回抽尚未裁决的在线窗口。判定中的“破口”指原陈述不能在该扩域上保持同一语义；“需改陈述”指原函数仍可算，但必须收窄定义域或扩输出类型才能继续进入复合证明。

| 假设 | 判定 | 理由；破口时的最小反例 `x` | 等级 |
|---|---|---|---|
| 1. `T_t` 全定义且 bit-exact | **需改陈述** | parser 对 raw history 仍可全定义；但 C2 已区分 `WindowUnit` 与消费层 `CompletedMove`。取 `x_A`：中枢已完成、离开已完成、回抽尚未完成；塔可有 tail，走势/协议像仍是 Pending。应改为“塔事实全定义 + `CompletedMove ⊎ Pending(reason)` 投影全定义”，不能把前者代替后者。 | L0 |
| 2. `Γ_t` 有限 | **成立** | 扩增有限类事件仍有限，空集也有限。注意：有限性不蕴含事件覆盖。 | L0 |
| 3. `N^δ_{ℓ↓e}` 全定义 | **破口** | `x_N`：仅一个已完成中枢，`MoveBlock.kind=Consolidation, dir=None`，且回抽未完成。旧值域只给已定 `δ` 的 `{0,1}`；`N^{None}`/资格 Pending 未定义。若硬置 false，会把“尚不可判”与“已判失败”合并。 | L0 反例 |
| 4. `I_γ` 类型全定义 | **破口** | `x_P`：单中枢盘整，存在已确认 `PanDivCert`，但没有 B1/S1 或 2/3 类 bit。当前契约明确 PanDiv“不产 BspPoint/不置 six-bit”（`signal.rs:525-531`），因此旧 `I_γ` 无像；若期望“盘整背驰减仓”，动作类别也无像。 | L0 反例 |
| 5. `𝔠_Θ` 成本函数 | **破口** | 即使在原论域，确定费用不能推出 `z_t` 唯一。最小模型：同一 `x` 有两个合法分类 `z^a≠z^b`，费用函数、其余函数均确定，两个分类分别导向不同订单；11 条现文假设仍未排除此模型。详见 `escalate`。 | L0 反模型 |
| 6. `I_Θ` 固定优先级 | **需改陈述** | 对现有 P 向量仍是函数；但 `x_P` 的盘整减仓、`x_A` 的 Pending、协议切换不在 P 中。应改为“固定优先级 + 动作语义覆盖义务 `required_action(x) ⇒ ∨P_j(x)`”。 | L0 |
| 7. `AncOK` 是函数 | **成立** | 作为持仓腿集合运算不受三态方向影响。它不负责协议 mode；不得据此宣称协议状态已闭合。 | L0 |
| 8. `TStage/TWEvent` 全定义 | **需改陈述** | 资金阶段机仍全定义，但与盘整/趋势协议机正交。状态应是两机的积，或显式加入 `ProtocolEvent`；TW 全定义不能推出协议全定义。 | L0 |
| 9. `K_Θ` 非空有限 | **成立** | lot 网格与 `0` 见证仍成立。反面是：`0` 会让错误的 Hold 也满足非空，不能作为盘整动作覆盖证据。 | L0 |
| 10. `LexArgmin` 固定平局 | **成立** | 给定有限非空候选和固定终局键，唯一选择仍成立；缺失动作在进入 `K_Θ` 前已经丢失。 | L0 |
| 11. `Schedule_Θ` 是函数 | **需改陈述** | 订单映射仍确定；协议切换可能 `qty=0` 但改变下一时刻政策。扩域的真正输出至少需包含 `(O_{t+1}, ProtocolEvent_{t+1})`，否则订单唯一不能推出状态转移唯一。 | L0 |

## (a)/(b) 失效登记

### (a) 定理本体失效：§A 的复合全函数链

#### A-0：原论域内的断链

§A 的证明结构写“由 5 ⇒ 状态 `z_t` 唯一”（`proofs...:39-43`），但假设 5 实际只断言交易成本 `𝔠_Θ` 确定（`:94-98`）。最终 PDF `完整的策略.pdf` §13 的第 5 条是分类器 `C_Θ` 为函数，不是成本函数；Markdown 整合时发生了符号/职责错换。

因此现文的 11 假设不蕴含 `z_t` 唯一。上表的双分类模型就是反例：成本相同并不排除两个分类与两个订单。该错误不依赖 `dir=None`，故是**原论域内真矛盾**，不是本次扩域才产生的缺口。

#### A-1：扩域后的严格断链

取在线状态：

```text
x_A:
  已完成中枢 B: [ZD,ZG]=[100,110], [DD,GG]=[90,120]
  已完成向上离开段 L: 区间 [111,130]
  回抽走势 R: 已开始但未 Completed
  MoveBlock.dir = None（当前仍只有单中枢/盘整语义）
```

在不偷看未来的前提下：

1. `T_t` 可保存 raw/tail；
2. 但 `N^δ` 没有 `δ=None/Pending` 像；
3. `I_γ` 没有“离开已成、回抽待决”类型；
4. `z_t` 没有协议 mode/回抽资格状态；
5. 因而严格的扩域分类链是部分函数，不能复合推出 `∃!O_{t+1}`。

若实现选择把 `x_A` 投影为 `Γ=∅→C_0→Hold`，则程序决定性仍在，但其数学对象变成“旧函数经遗忘投影后的总化”。它只证明**有唯一订单**，不证明**扩域语义已完整分类**。这正是 231 号“定义域 ≠ 有效域”的本案形式。

#### 建议 diff（文档义务；不实装）

```diff
- 假设 5：𝔠_Θ 是函数（成本函数）
+ 假设 5a：C_Θ : X → Z 是全函数（状态分类器；扩展 X/Z 的定义域和值域）
+ 假设 5b：𝔠_Θ 是成本函数
+ z_t 显式包含 MoveBlock.dir∈{Up,Down,None}、
+ ProtocolMode、Departure/Retrace 的 Pending/Completed 资格态

- ∀x_t, ∃! O_{t+1}=π_Θ(x_t)
+ ∀x_t∈X, ∃!(O_{t+1},ProtocolEvent_{t+1})=π_Θ(x_t)
+ 前提：事实投影、候选分类、协议事件和动作谓词均为全函数
```

### (b) 真空真：§B 恒成立但策略语义不完备

`C_1=P_1`、`C_j=P_j∧⋀_{k<j}¬P_k`、`C_0=⋀¬P_j` 的证明只需要一个布尔向量。扩域不会破坏这条 L0 恒等式。当前 `mutex_class` 也只是取最小 true 索引（`mutex.rs:133-150`），并且该模块明确是测试 oracle、非生产消费者（`:60-66`）。

真正缺的是另一个命题：

```text
对每个扩域状态 x，若缠论/协议要求动作 a，
则至少有一个 P_j(x) 表达 a，且最小命中类 C_j 的动作语义等于 a。
```

旧 §B 没有证明这个命题。逐动作核对如下：

| 扩域所需动作/事件 | P1..P10 最接近项 | 只读核对 | 结论 |
|---|---|---|---|
| 中枢区间边界开仓 | P8 OpenRoot / P9 OpenShortDiff | P8/P9 的门是“已有买卖点候选 + slot 空 + 角色”，契约 `mutex.rs:14-15,278-282`；没有 `price` 相对 `[ZD,ZG]` 的边界谓词。 | **不在语义内**。无 Bsp 候选时全 false→C0；有无类结构候选时 P10 Record。 |
| 回到中枢/边界平仓 | P5 CloseRoot / P6 ReduceCore / P7 CloseShortDiff | 三者要求反向 Bsp 候选，typed 由一二三类决定（`mutex.rs:11-13,252-267`）；没有“回到 `[ZD,ZG]`”本身的 close predicate。 | **不在语义内**。回中枢但无反向买卖点时不会触发 P5-P7。 |
| 盘整背驰减仓 | P6 ReduceCore 看似接近 | P6 仅“三类反向⇒减核心”。`PanDivCert` 明确不置 six-bit、不产 BspPoint（`signal.rs:525-531`），且 `Cand^δ` 装配明确“盘整背驰不入谓词”（`nest.rs:380-390`）；生产新确认投影还写明 PanDiv 无消费者（`runner.rs:728-740`）。 | **确定缺口**。当前最多进入统计承接或被丢弃，不会变成 P6。 |
| 盘整背驰产生的“类一类买卖点” | P5/P8/P9 的一类开平看似接近 | 仓库定义明确盘整背驰不产生标准同级第一类，只在超大级别退化语境保留“类第一类”边界（`.chanlun/definitions/maimai.md:295-316`）；当前 `PanDivCert` 刻意不置 B1/S1 bit（`signal.rs:525-531`）。 | **没有独立谓词/typed action**。不得冒充 P5/P8/P9 的标准一类；若要交易，须先裁定独立类别及承接动作。 |
| 三类点作为“盘整→趋势”协议切换事件 | P6/P8/P9 | 三类只影响已有持仓的 ReduceCore 或开仓角色；`Predicates` 与 `StepTrace` 无 `ProtocolMode/ProtocolEvent` 字段，仓库 `theta_v0` 也无该状态机。 | **不在语义内**。即使三类触发了订单，也没有协议切换证据。 |
| 趋势背驰/回中枢后“趋势→盘整” | P5 CloseRoot 或 C0 | P5 是持仓关闭，不是协议 mode 迁移；无反向买卖点时 C0 Hold。 | **不在语义内**。订单 Hold 与协议回落被混同。 |
| 盘整默认/初始态 | C0 Hold / Ambient | C0 只表示无 P 命中；Ambient 只表示父容器无方向（`coverage.rs:903-927`）。两者都不是持久协议状态。 | **语义缺失**。默认协议不能从“什么都没触发”反推。 |

因此这里的准确裁决是：**§B 定理成立，§B 的“全定义”命名发生有效域膨胀**。它全定义的是 `P→C` 的组合映射，不是 `市场扩域状态→完整策略动作`。

#### 建议 diff（契约义务；不实装）

```diff
  P1..P10 保留现有优先级
+ P_center_boundary_open / P_center_reentry_close（若裁定为独立动作）
+ P_pan_div_reduce
+ P_protocol_to_trend
+ P_protocol_to_consolidation
+ ProtocolMode ∈ {Consolidation, Trend(Up), Trend(Down)}
+ property: required_action_covered_by_some_predicate
+ property: c0_only_when_no_required_action
```

是否把这些事件新增为独立 P，还是先生成 typed Candidate 再复用 P5..P10，是实现设计选择；但无论选哪条路，都必须增加“动作需求覆盖”证明，不能只重跑 `2^m` 互斥穷举。

## T1/T2/T3 证明或反例

### T1 中枢后演化三分定理

#### 先构造反例

**反例 T1-a（在线不穷尽）**：采用 `x_A`。回抽未完成时，不能确认中枢终结，也没有后继已完成中枢可比较。把它判延伸会被稍后“不回中枢”推翻；判新生/扩展则偷看未来。因此 `延伸/扩展/新生` 在在线状态空间上少了 `Pending`，不是 MECE。

**反例 T1-b（`[ZD,ZG]` 信息不足）**：固定前中枢与同一组已完成离开—回抽：

```text
B:       [ZD,ZG]=[100,110], [DD,GG]=[90,120]
离开 L:  向上至 140
回抽 R:  低点 115 > ZG=110（第三类买点几何已确认）

后继一 C_N: [ZD',ZG']=[130,135], [DD',GG']=[125,140]
              DD'=125 > GG=120  ⇒ 新生/上涨延续

后继二 C_X: [ZD',ZG']=[112,118], [DD',GG']=[105,135]
              ZD'>ZG 且 DD'≤GG ⇒ 外缘重叠，扩展成高级别中枢
```

两条历史在 `B+L+R` 截止时对 `[ZD,ZG]` 的位置关系完全相同，却有不同后继。故“裁决谓词只取离开段回抽与 `[ZD,ZG]` 的位置关系”不能判新生/扩展。

#### 裁决

**T1 按题述 FALSIFIED（L0）**。失败有两层：在线域缺 Pending；settled 域还缺后继中枢 `GG/DD`。

#### 可证的收窄版

在“后继同级别中枢已经完成，且当前对象已离开延伸域”的 settled 域上，三分可证：

1. 若所有后续 `Z_n` 均满足 `[d_n,g_n]∩[ZD,ZG]≠∅`，由中心定理一为延伸（`.chanlun/definitions/zhongshu.md:144-163`）。
2. 否则等待后继同级别中枢完成。若 `后DD>前GG`，为上涨新生；若 `后GG<前DD`，为下跌新生（`:165-176`）。两式不可能同时成立，因为每个中枢均有 `DD≤GG`。
3. 两个新生式都不成立，而两个同级别核心区间已分离时，外包络必重叠；由中心定理二的充要式为扩展（`:178-191,321-330`）。
4. 三类两两互斥：延伸尚未产生独立后继中枢；上涨/下跌新生互斥；新生要求外包络严格分离，扩展要求外包络重叠。

这证明的是**settled 后验三分**，不是在线任意时刻三分。在线正确值域至少是 `Pending / 延伸 / 新生 / 扩展`。

```diff
- T1：任意时刻，延伸/扩展/新生 MECE；只读离开回抽相对 [ZD,ZG]
+ T1a（在线）：Pending ⊎ Extension ⊎ Newborn ⊎ Expansion
+ T1b（settled）：Extension / Newborn / Expansion MECE
+ [ZD,ZG] 只裁延伸终结与第三类几何；
+ Newborn/Expansion 必须读后继已完成中枢的 GG/DD（central-ggdd-v1）
```

建议看守：`center_development_online_has_pending`、`same_retest_can_lead_to_newborn_or_expansion`、`settled_center_development_mece_central_ggdd_v1`。

### T2 协议互斥不变量

#### 先构造反例

题述转移“第三类点确认 ⇒ 趋势态”不成立。反例就是 T1-b 的 `C_X` 分支：第三类买点已确认，但后续形成的是高级别中枢扩展。仓库定义也明写“第三类买卖点后不必然是趋势，可能是更大级别盘整”（`.chanlun/definitions/maimai.md:132-142`）。因此第三类点最多是**协议切换候选/待决事件**，不能独自证明 `Trend`。

“趋势背驰 ⇒ 盘整态”同样只能作为保守操作协议，不是走势类型定义定理：背驰保证回拉/走势完成，但后继可以是盘整或反向趋势；旧定义只给“背驰→完成”而非“后继必为盘整”（`.chanlun/definitions/zoushi.md:232-248`）。

#### 可证部分：排他不变量

若把语义 guard 修正为：

- 初始 `ProtocolMode=Consolidation`；
- 只有 `central-ggdd-v1` 确认 `UpContinuation/DownContinuation`（第二个同向已完成中枢）才进入相应 Trend 协议；
- `LevelExpansion`、已确认回中枢或趋势完成事件进入/保持 Consolidation；
- 多事件同 bar 时用与 §B 相同的固定优先级互斥化；无事件时保持原 mode；

则“任意时刻恰一协议活跃”可严格证明：

1. 基例：初始值只有 `Consolidation`，故恰一。
2. 归纳步：事件谓词经 `C_j=P_j∧⋀_{k<j}¬P_k` 后恰一类命中；对应转移只写一个 mode；`C_0` 保留归纳假设中的唯一 mode。
3. 因此由时间归纳，所有 `t` 上 mode 都存在且唯一。证明只依赖枚举值域与固定优先级，是 L0。

**裁决**：T2 的**排他不变量成立（修正 guard 后 L0 已证）**；题述的具体切换语义被反例否定。其经验收益/减回撤不在本证明域，仍为 L2 未测。

```diff
- third_point_confirmed => ProtocolMode::Trend
+ third_point_confirmed => PendingProtocolSwitch
+ central_ggdd_v1 in {UpContinuation,DownContinuation} => ProtocolMode::Trend(dir)
+ LevelExpansion | confirmed_reentry | trend_completed => ProtocolMode::Consolidation
+ protocol event predicates use the same fixed-priority mutualization
```

建议看守：`protocol_mode_total_exclusive`、`type3_expansion_does_not_enter_trend`、`central_ggdd_newborn_enters_trend`、`simultaneous_protocol_events_take_fixed_priority`。

### T3 趋势前缀定理

#### 先构造反例

若“盘整前缀”指一个**已完成的盘整走势对象**，T3 为假。最小上涨趋势只需两个依次上移的中枢 `C_1,C_2`。`C_1` 形成后，该前缀可以继续生长为同一个趋势；它不必先冻结成独立 Completed Consolidation。盘整的仓库定义本身要求“某完成的走势类型只包含一个中枢”（`.chanlun/definitions/zoushi.md:103-124`）。

#### 可证的精确版本

**T3'：任何趋势首次被确认之前，必有一个“恰一已完成中枢、走势块仍 Active、`dir=None`”的因果前缀。**

证明：

1. 趋势定义要求至少两个依次同向中枢；一个中枢只能是盘整形前缀（`.chanlun/definitions/zoushi.md:106-123`）。
2. 取趋势首次可确认的时刻；第二个同向中枢在该时刻才 Completed。其严格前一时刻最多只有第一个已完成中枢，否则趋势会更早确认。
3. 一个中枢的块由现行分解器构造成 `MoveKind::Consolidation, dir=None`，随后链尾标 Active（`decompose.rs:128-140`）；后继关系为 `Up/DownContinuation` 时才成为 `Trend,Some(dir)`（`:54-72`）。
4. 中枢本身要求至少三个已完成次级别走势类型（`.chanlun/definitions/zhongshu.md:132-140`）；同一先一枢、后二枢的时序可递归应用于每一级别。
5. 故每条趋势形成路径都经过单中枢 Active 前缀，但不必经过 Completed Consolidation 对象。∎

**裁决**：题述若把“盘整”解释为协议默认/Active 单中枢前缀，则 T3' 为 **L0 已证**；若解释为 Completed 盘整对象，则由上述反例否定。

```diff
- 趋势 = 已完成盘整 + 中枢新生
+ 趋势首次确认路径 = 单中枢 Active 前缀 + 第二个同向已完成中枢的 central-ggdd-v1 新生确认
+ 禁止把 Active 前缀登记为 Completed Consolidation
```

建议看守：`trend_first_confirmation_has_single_center_active_prefix`、`active_prefix_is_not_completed_consolidation`、`recursive_trend_prefix_preserved_per_level`。

## OPEN 义务

| ID | OPEN 义务 | 为什么当前不能冒充已证 | 关闭条件 / 建议看守点 | 等级 |
|---|---|---|---|---|
| O-1 | 在线中枢发展四态（含 Pending）的规范状态与转移 | T1 已证伪三态在线穷尽；目前只有 settled `classify_relation`。 | 在形式链中定义 Pending 原因枚举；测试 `center_development_online_has_pending`。 | L0 OPEN |
| O-2 | `firstRetrace` 身份与失败后的重启/消费规则 | D7 仍待裁；当前证据只证明价格几何，不证明 firstness/identity（`c2-pending-rulings-material-20260714.md:69-80`）。 | D1 projection + D2 A/C hook 版本化后，裁定 `(center,departure)` 身份；测试 `first_retrace_consumed_or_restarted_explicitly`。 | L0 语义 OPEN |
| O-3 | 盘整/趋势协议状态机的权威 guard | “3 类⇒趋势”已被否；“趋势背驰⇒盘整”只是保守政策。 | 用户/权威文档裁定 guard 与同 bar 优先级；随后证明 mode 全定义。 | L0 语义 OPEN |
| O-4 | 区间边界开平的精确动作契约 | 当前原文锚只给中枢/3 类几何，没有给仓位大小、开平对象与优先级。 | 给出 typed action、持仓对象、与 P1..P10 的映射；测试 `center_boundary_action_covered`。 | L0 设计 OPEN |
| O-5 | 盘整背驰减仓的动作量与承接层 | `PanDivCert` 现为统计承接，不是生产动作；“减多少/减哪腿”未裁。 | 明确 `PanDivCert→ReduceCore/CloseShortDiff/Record` 规则及优先级；测试 `pan_div_required_action_not_c0`。 | L0 设计 OPEN；收益 L2 |
| O-6 | 扩域 `z_t` schema 与分类全函数证明 | 当前 `MuClass.delta` 二值，缺 ProtocolMode/Pending；可存部分 None 不等于全域分类。 | 新 schema、`classify_ext_total` property、旧投影的显式版本号。 | L0 OPEN |
| O-7 | 动作语义完备定理 | 现有 `2^10` 只证 P→C；没有证 required action→P。 | 新 property `required_action_covered_by_some_predicate` 与 `c0_only_when_no_required_action`。 | L0 OPEN / L1 看守 |
| O-8 | `π_Θ` 输出类型是否包含无订单协议事件 | 只返回 Order 会丢失 mode 切换。 | 裁定输出积类型或把 mode 纳入下一状态转移；测试 qty=0 的协议事件仍可见。 | L0 OPEN |
| O-9 | T3 的“Active 前缀”形式证明落点 | 数学证明已给，但 Lean/契约尚未见该定理。 | 建议在 `Origin.CenterStates`/走势分解侧加 `trend_has_single_center_active_prefix`，不改 Completed 定义。 | L0 已证、机器锚 OPEN |

## escalate

### E-1 BLOCKER：假设 5 的符号/职责错换使 §A 现文在原论域内无效

**冲突证据**：

- 证明链：`proofs-full-strategy-20260703.md:41` 写“由 5 ⇒ 状态 `z_t` 唯一”；
- 假设正文：`:94-98` 把 5 定义为交易费用函数 `𝔠_Θ`；
- 原 PDF：`完整的策略.pdf` §13 第 5 条是分类函数 `C_Θ`；§6 也先定义完整状态 `z`；
- 所以不是“成本函数也能顺带分类”，而是整合文档把两个不同符号/职责混写。

**最小反模型**：令 `X={x}`、`Z={z_a,z_b}`，不对 `x` 的分类施加函数性；令费用、AncOK、TW、K、LexArgmin、Schedule 全部为确定函数。令 `z_a` 导向 Buy、`z_b` 导向 Hold。现文假设 5 成立，但 `z_t` 与订单均不唯一。因此 `11 assumptions ⇒ ∃!O` 为假。

**升级理由**：这是定理前提缺失，不是行号漂移或生产可达性问题；继续只补盘整维度会在错误基底上扩证。

```diff
- 假设 5：𝔠_Θ 是函数（成本函数）
+ 假设 5：C_Θ 是状态分类全函数（给出定义域和值域）
+ 假设 5a：𝔠_Θ 是成本函数（若后续 J_x 证明需要，独立列出）
```

### E-2 CONCERN：文档同时声称“完整 §6 z 已闭合”与承认状态轴缺失

`proofs...:163-167` 声称唯一性有效域已扩至“完整 §6 z”，但 `:262-273,303-306` 同时保留 TStage/ηBucket/CostBucket/Live 轴缺口（其中部分后来已在代码补上，Live/Cost 仍有诚实声明）。这不会单独否定旧订单函数的决定性，却否定“完整状态已覆盖”的措辞。应把“决定性闭合”与“语义 schema 完备”分开登记。

## 认识论等级汇总

| 结论/物证 | 等级 | 有效域与禁止外推 | 建议机器看守（仅建议） |
|---|---|---|---|
| §A 假设 5 反模型 | **L0 否证** | 原论域即可成立；不依赖行情。 | 文档 lint：证明链引用的每个假设输出类型必须匹配。 |
| dir 三态与盘整无方向 | **L0 定义/已裁** | 走势类型语义；段几何方向仍可二值，不能混同。 | `decompose.rs:43-60`；`consolidation_dir_is_none`。 |
| `x_A` 在线断链 | **L0 反例** | 证明旧 schema 未覆盖 Pending；不声称真实触发频率。 | `center_development_online_has_pending`。 |
| PanDiv 不入 P/生产动作 | **L0 静态物证** | 当前 HEAD 代码路径；未来接线后须重审。 | `signal.rs:525-531`、`nest.rs:380-390`、`runner.rs:728-740`；`pan_div_required_action_not_c0`。 |
| §B `Σ1[C_j]=1` | **L0 恒等式** | 只覆盖给定布尔向量。 | 既有 `mutex.rs` 2^10 穷举是 **L1 管线看守**。 |
| P1..P10 动作语义不完备 | **L0 契约审计** | 当前契约与题设扩域；不等于实盘策略无效。 | `required_action_covered_by_some_predicate`、`c0_only_when_no_required_action`。 |
| T1 题述反例 | **L0 否证** | 在线域及 `[ZD,ZG]` 信息充分性。 | `same_retest_can_lead_to_newborn_or_expansion`。 |
| T1 settled 收窄版 | **L0 已证** | 仅后继同级别中枢 Completed；新生/扩展用 GG/DD。 | `center.rs:203-214`；`settled_center_development_mece_central_ggdd_v1`。 |
| T2 排他不变量 | **L0 条件证明** | 以 mode 枚举、总转移、固定优先级为前提；具体 guard 仍 OPEN。 | `protocol_mode_total_exclusive`。 |
| “3 类确认⇒趋势” | **L0 否证** | 第三类后扩展反例；不否定将其作为候选事件。 | `type3_expansion_does_not_enter_trend`。 |
| T3 Active 前缀 | **L0 已证** | 只证单中枢 Active 前缀，不证 Completed 盘整对象。 | `trend_first_confirmation_has_single_center_active_prefix`。 |
| 当前生产中的出现频率、PnL、减回撤 | **L2 未执行** | 本任务全程只读，无真实数据重放；禁止从 L0/L1 外推。 | 后续预注册真实数据测试，另行报告。 |

总认识论裁决：本审计新增的是 **L0 否证、L0 收窄证明与静态契约物证**；现有 `2^10`/property tests 至多是 **L1 管线看守**；没有产生任何 L2 市场有效性结论。
