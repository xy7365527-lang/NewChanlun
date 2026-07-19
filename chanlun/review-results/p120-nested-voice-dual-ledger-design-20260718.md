# p120 施工图：嵌套子声部 + fill 双账本负仓（关⑤）——recognize 嵌套产出设计 + q⁺/q⁻ 分腿账本改造

日期：2026-07-18 ｜ 性质：**施工图（只设计不实装）**——零代码改动、零数据重放、零 git mutation、主仓零写入、零 cargo 调用（p116 后台重放不受干扰）
工位：主线阶段 2 **关⑤（嵌套子声部 + fill 双账本负仓）**
输入在案：`docs/theta-v0-grammar-audit.md`（#1/#5/预留面）；`docs/bsp_consumption_redesign.md`（C1 裁决/sink-recover 齿轮/多空双开=多声部赋格/§9.1 净额翻转根因/§9.7 G1-G5）；`docs/recursive_fugue_necessity_proof.md`（T16/T21/T45、四步 1-cycle、RF-cyc −2）；`.chanlun/specs/2026-06-28-complete-mutex-classification-pdf-extract.md`（M13/M14 P^sep、级内互斥级间并发）；`chanlun/escalate/bsp-terminal-endorsement-ruling-20260718.md`（关①S2/S4、关②037:20、关③T1、串行序）；`chanlun/escalate/nest-leftover-rulings-u1-u3-20260718.md`（U2 c 确立时点=t*）
教义引用格式 `0XX:段号` = 主仓 `docs/chanlun/text/blog/0XX-第X课.md` 行号，全部于 2026-07-18 直读原文逐条核对（§9 自检清单）
代码坐标格式 `文件:行号` = worktree `/tmp/kimi-nest-mainline` 2026-07-18 现行状态直读核对

---

## 0. 结论速览

| 必答 | 一句话结论 |
|---|---|
| 1 recognize 嵌套产出 | 新增 `recognize_nested`（strategy/mod.rs）：候选源换 `interp::assemble_gamma_with_tower`（真 ShortDiff 角色，候选序 bit-exact 不变）+ 真活动集喂 `interpret`；**角色门**：`role.v==ShortDiff ∧ 活父声部 ∧ 父侧=−候选方向 ∧ depth+1<max_depth` ⟹ 产 depth>0 子决策，`root_side` **继承树根**（非 `cand.dir`）⟹ `voice_side` 代数自动兑现 σ_child=−σ_parent；其余候选走现行 depth=0 根决策（`build_decision` 零改）。形态=**单脊柱赋格树**（depth 索引账户结构零改动）；多孩子分叉树列 v1 边界外。 |
| 2 级联关闭 | `exit.rs:121` 的 `parent_invalid: false`（恒假占位）实义化：depth>0 声部 `parent_invalid = 父槽空仓 ∨ 父 exit_pending`；调用方新增 cascade 发射（父退出 ⟹ 全部更深 held 强制退出，**最深优先**），ConflictKey 退出序已有 depth 终局键承载。 |
| 3 fill 双账本 | 新文件 `backtest/dual_ledger.rs`：`DualLedger{cash, q_long≥0, q_short≥0, cost_long, cost_short}`（M14 `P^sep=∏(R≥0 e⁺⊕R≥0 e⁻)` 的账户层兑现）+ `apply_fill_dual`（**分腿不先净额**：开空不触多腿=M13 父仓保持；平仓只作用指定腿；realized 只在平仓腿产，保 A' 结算源）+ `LegOrder` 腿标记包装（**types.rs `Order` 零改**）。旧 `apply_order`/`apply_fill` 本体零改。 |
| 4 bit-exact 冲突面 | **不碰主干**：classifier 全树、净额 fill 路径、coverage/overlay/interp/risk/voice/types/Lean 零触碰；与关①②③在途实装面（signal.rs 判据链/nest.rs/level_view.rs/divergence.rs/p92 bin）**零文件交集**。bit-exact 论证=构造性（旧路径零改）+ 双账本**嵌入恒等**（同订单流下 cash/equity 逐笔相同、兼容腿标记下 realized 逐笔相同——现行净语义是双账本子集）。 |
| 5 单测 | 26 个测试分五组（§6）：A 嵌套产出 6 / B plan_orders 嵌套 4 / C 级联关闭 4 / D 双账本 8 / E 集成 4；含 σ 交替代数锁、不先净额锁、嵌入恒等逐字节锁、M13 父仓保持锁、G5 双层记账同数锁。 |
| 6 输入假设 | 接口/对象/账本语义**零依赖**关①②（内容无关设计）；嵌套**触发信号总体与一切产量读数**依赖关①（单调加宽）→关②（纯收缩）串行链落地后的最终 BSP 账本——在途账本上的量测无效；U2「双开入场不得早于 t*」由 runner 因果前缀分类构造性满足，不加额外门（§2）。 |
| 7 与 coverage 路径关系 | coverage M29 引擎已产嵌套 ShortDiff 腿但执行净额（`net_target_units` 自声明「完整毛分账本执行须 hedging 账户」coverage.rs:1624）；本关 DualLedger 是 recognize 路径先建的 hedging 账户，coverage 接 hedge 执行=同 substrate 后续关，本关不接（防范围蔓延）。 |

---

## 1. 现状证据（文件:行号，全部直读核对）

### 1.1 审计锚点的时效订正（090：声明须与现行能力一致）

`docs/theta-v0-grammar-audit.md`（2026-06-28）是本关的立项依据，但其代码坐标已部分漂移，先行订正（直读核对）：

| 审计条目 | 审计原文坐标 | 现行实况（worktree 直读） |
|---|---|---|
| #5「recognize 产单声部独立根 depth=0（mod.rs:426-448）」「v0 classifier 不产嵌套子声部（mod.rs:431）」 | 指 classifier 域 | **仍成立但坐标漂移**：recognize 在 `strategy/mod.rs:454`（非 classifier/mod.rs）；`depth: 0, // §5 独立根` 在 `strategy/mod.rs:558`；「不跨级别赋格翻转」注释在 `:482-483`。任务书「rust/src/theta_v0/classifier/mod.rs（recognize 现仅产 depth=0 独立根）」的归属亦按此订正。 |
| #1「fill 引擎 `apply_order`（runner.rs:637）`Sell` 分支调 `close_long`」「units 物理上 ≥0（runner.rs:573）」 | runner.rs:573/637 | **已修复（stale）**：`apply_order`（runner.rs:3122）/`apply_fill`（runner.rs:3167）已是**方向中性有符号账本**（`units` 正多/负空/0 空仓，runner.rs:969 注释），「删 v0 退化的 long-only `close_long`，no-patch 重写」（`:3107-3108`）。**但账本仍是净额**——`先平后开`（`:3110-3112`）：任一订单先平反向持仓再开新仓，多空双腿无法在账户层共存。本关的 fill 改造是**第二层**（净额→双账本），不是重复修 #1。 |
| #5「`risk.rs::parent_cap`/`signed_notional` 已为多声部预留，但无消费者」 | risk.rs | **半 stale**：`parent_cap`（risk.rs:257）**有**消费者（`strategy/mod.rs:348` `build_open_order` 的 depth>0 分支），但 recognize 恒产 depth=0 ⟹ 生产**不可达**（恒走 `:345-346` 根分支 `root_parent_cap`）；`signed_notional`/`VoiceNotional`/`gross_notional`/`net_notional`/`leverage_metrics`/`leverage_ok`（risk.rs:441-547，美元空间 Lean 镜像）**无生产消费者**；其 units 空间等价物 `gross_units_cap`/`gross_units_ok`（risk.rs:563-569+）**已被 coverage.rs:1687/1690 消费**（KKT 毛闸门）。 |

### 1.2 recognize 现状：depth=0 独立根（嵌套产出的缺位点）

- **产出点**：`strategy/mod.rs:454-510` `recognize`——`interp::assemble_gamma`（interp.rs:275，**扁平**：候选元素 `parent=None` 去根化 ⟹ V 恒 Ambient，interp.rs:276-277 注释）→ 按时刻分组 → `interp::interpret(&gamma_x, &[])`（**活动集恒空**，mod.rs:493）→ `build_decision`（mod.rs:521）逐候选产 `VoiceDecision { depth: 0, root_side: cand.dir, .. }`（mod.rs:557-571）。
- **缺位三件事**：(a) `depth` 恒 0（mod.rs:558）——「每个买卖点 = §5 独立根（parent=none，不跨级别赋格翻转）」（mod.rs:482-483；同义注释 mod.rs:157-158、voice.rs:93-95）；(b) 候选角色不可用——扁平 gamma 的 V 恒 Ambient，**ShortDiff 角色根本不产生**（真角色须 `assemble_gamma_with_tower`，interp.rs:451，其注释「V 真出 FollowParent/ShortDiff，非扁平 assemble_gamma 的恒 Ambient」interp.rs:438-439）；(c) 无父失效关闭——`exit.rs:121` `parent_invalid: false`（恒假占位，其注释「root 声部无父 ⟹ 恒 false……子声部的父失效判定属嵌套树扩展，v0 未触发」exit.rs:72-73）。
- **消费链（嵌套就绪度）**：`plan_orders`（mod.rs:250）已 depth 感知——`within_max_depth` 门（:260）、`voice_side(d.root_side, d.depth)` 绝对方向（:268）、`account.qty_at(d.depth)` 持仓（:269）、depth>0 时 `parent_cap(qty_at(d.depth-1))`（:348）；`AccountState.voice_qty: Vec<u32>` 按 depth 索引（mod.rs:128-137）；`HeldVoice` 台账按 depth 索引（exit.rs:37-42，`held.get_mut(d.depth)` exit.rs:60）；runner `plan_and_fill_mtm` 维护 `voice_qty`/`held` per depth（runner.rs:2712/2715）+ `apply_voice_fill` 按 fill 两段真实成交更新（runner.rs:3009-3018）。**即：消费链的 depth>0 通道全部焊好，缺的只是产出端 depth>0 决策与 fill 层双账本。**

### 1.3 fill 现状：净额有符号账本 + 先平后开（双账本的改造点）

- **账本态**：`(cash, units: f64 有符号, entry_cost, trade_pnls)`（runner.rs:2707-2710）。
- **成交语义**：`apply_order`（runner.rs:3122）把 action 映到 `(delta, close_only)`（:3139-3152）→ `apply_fill`（:3167）两段：段1 **平反向**（`units·delta<0` ⟹ 先平 min(qty,|units|)，realized 入 trade_pnls，:3202-3230）；段2 **开新仓**（余量按 delta 开，加权成本基，:3232-3267）。成本对称（多=买入均价含费/空=卖出净收扣费，:3114-3117）；现金约束（开多需 cash≥含费成本，开空无预付、保证金未建模——诚实声明 :3119-3121）。
- **净额效应**：持多时 Sell = 先平多再（余量）开空 ⟹ **多空双腿账户层不可共存**——这正是 `bsp_consumption_redesign.md` §9.1 裁定的「净额翻转是空头不赚根因」（M13 语义：短差=父仓保持+次级别反向双开，净额账户物理上不可表达）。
- **下游口径（双账本必须对账的既有消费者）**：`cum_price_pnl += units·Δpx`（runner.rs:1090）；TW 成本划转 `basis_now=(units.abs()·entry_cost.abs())`（:1137）；窗口终点强平（:1758-1774）；`track_position_transition` 以 units 跨 0 配对交易（:2945）；`FillOutcome{closed,opened,rejected,realized,fee}`（:3281-3290）——A'（codex GAP3 裁定清单③）：`realized` 只在平仓段产，是 TW Realize 唯一合法资金源（:3137-3138/:3178-3184 注释）。

### 1.4 预留面盘点（已就位、本关激活）

| 预留 | 位置 | 现状 | 本关处置 |
|---|---|---|---|
| `voice::voice_side(root_side, depth)` σ_root·(−1)^depth | voice.rs:98 | 完整定义，depth>0 分支生产不可达 | 方案 A 激活（σ_child=−σ_parent 代数核心） |
| `voice::depth_weight`（w=[0.60,0.30,0.10]） | voice.rs:188 | 同上 | 子声部 sizing 项2 权重 |
| `voice::within_max_depth`（默认 3 层） | voice.rs:200 | plan_orders:260 已消费 | 子深度边界门 |
| `risk::parent_cap(parent_qty)`=floor(parent·β) | risk.rs:257 | 有消费者（mod.rs:348）但生产不可达 | 方案 A 激活（父子 β 约束首次生产可达） |
| `risk::gross_units_cap`/`gross_units_ok` | risk.rs:563/:572 | coverage.rs:1687 消费中 | 方案 A 复用为嵌套毛闸门（同一 predicate 单源，不私写比较） |
| `risk::VoiceNotional`/`signed_notional`/毛净聚合 | risk.rs:441-547 | 美元空间无生产消费者 | 方案 B 双账本的毛/净敞口读数接此（识别为合法首位生产消费者） |
| cascade_close 语义（父关⟹后代全关） | risk.rs:358-361 注释（引 Origin.VoiceTree）；exit.rs:121 恒假占位 | 纸面有、运行时假 | 方案 A §3.5 实义化 |
| `interp::assemble_gamma_with_tower`（真 ShortDiff 角色） | interp.rs:451 | coverage 路径消费中；recognize 未用 | 方案 A 候选源（角色单源，禁角色 fork） |
| `interp::interpret` 活动集参数 | recognize 内 mod.rs:493 恒传 `&[]` | 𝒟_x 恒空（mod.rs:503-506 debug_assert） | `recognize_nested` 传真活动集（𝒟_x 反向关闭生效） |

### 1.5 邻接已实装机制（设计定位关系，防重造/防口径 fork）

- **coverage M29 引擎**（coverage.rs:5-11）：已产**真嵌套**元素树（`CoverageElement.depth/parent` 只来自 `RMove::Compose.subs` 真父子，coverage.rs:23-29 铁律——`recognize` v1 旧 bug「`depth=l_star-level_idx` 级别差伪造」经 codex 裁决+L2 OKLO 坐实后明文禁止重蹈）与 ShortDiff 反向子声部腿（`leg_target`，coverage.rs:1424-1436）；但执行仍**净额**（`net_target_units` 毛→净降维，coverage.rs:1614-1633，:1624「完整毛分账本执行须 hedging 账户（v0 净额）」）。⟹ 本关 DualLedger 与该 hedging 账户同构，coverage 接线列后续关。
- **M5 overlay**（overlay_state.rs:1-43）：hedge-mode 逐声部 P^sep 簿 + ΔN 订单 + pnl_v 归因，**只读旁路**（runner.rs:1432-1444，不改净额 fill）——它是双账本语义的**影子排练**（PDF §11 线性对账 Q⁺ΔP−Q⁻ΔP=(Q⁺−Q⁻)ΔP 已在其注释坐实，overlay_state.rs:22），本关把同语义转正为 recognize 路径真账本；overlay 本体零改。
- **pan_div ShortDiff 子腿**（runner.rs:1380-1381/1422-1429）：#82 中枢震荡域已有 `signed_live_child_units()` 子腿投影叠净目标的先例——证明「子腿+父目标合成」的工程形态已在产；其域是盘整震荡（DC-E 门），与本关的趋势 ShortDiff 子声部**正交不合并**。
- **recognize 路径的活消费方**：`nautilus/strategy.rs:223`（recognize_current）+`:168/:179`（plan_orders 退出/开仓两批）+`:144-164`（held per-depth 退出循环）；`StrategyFamily::pi`（mod.rs:623）；`run_theta_v0`（runner.rs:291→`plan_and_fill_mtm`）；tests/theta_v0_perf_profile.rs:155。⟹ recognize 不是死代码，本关改动须保全部既有调用方 bit-exact（旧函数零改，新函数并列）。

### 1.6 教义锚（课号:段号=行号，2026-07-18 直读主仓原文逐条核对）

- **093:22**（先卖后买）：「如果你震荡操作水平比较好，就利用（1，0）后必然出现的震荡进行短差操作，由于都是先卖后买……」
- **025:126**（四步短差）：「操作上，一定不能等什么确认，而是有卖点就卖，有买点就买，当然是根据相应的级别。即使你是中线持有，5分钟、30分钟背驰，也可以先卖部分出去，下来再回补，这样就机动灵活了。」
- **026:80**（单边上扬禁短差 + 1/3 份额）：「短线是让你把成本将下来，而且确保持有的安全性，除了日线的单边上扬走势，短线必须坚持。但仓位可以控制，例如用其中的1/3……」
- **029:396**（次级别一类点定位）：「所有买点，归根结底都是第一类买点，要找第二、三类，其精确的，都要下次级别以下找第一类。」
- **029:52**（一卖镜像）：「……而第一类卖点的情况是一样的，只是方向相反。」
- **029:58**（三买持有协议）：「出现就继续持有，否则就可以抛出……」
- **024:18**（背驰-买卖点定理）：「任一背驰都必然制造某级别的买卖点，任一级别的买卖点都必然源自某级别走势的背驰。」
- **027:38**（区间套嵌套）：「……而这背驰段，一定在季度线的背驰段里，而且区间比之小……」
- **016:114**（没有趋势没有背驰）：「记住，没有趋势没有背驰……背驰是两个趋势之间比较才有意义，和盘整里比较是没用的。」
- **017:60/66/70**（一买二买/定律一/趋势转折定律）：「任何级别的第二类买卖点都由次级别相应走势的第一类买点构成」等。
- **031:30**（0 成本股票 + 超大级别卖点清仓）：「一旦股票再次启动，你就拥有比底部还多的但成本为0的股票……直到牛市结束前，才把所有股票全部清仓。」
- **020:40**（级别延续定理一）：「在更大级别缠中说禅走势中枢产生前，该级别走势类型将延续。」
- **065:182**（递归形式）：「两者在递归的形式上是一样的，都是an=f(an-1)……」

---

## 2. 输入假设（关①关②最终验收依赖声明，090 强制）

本设计分**零依赖层**与**依赖层**两类，如实切分：

**零依赖层（内容无关，不随关①②验收结果翻转）**：
- 全部接口/对象定义（`recognize_nested`/`DualLedger`/`apply_fill_dual`/`LegOrder`/cascade 发射）——它们消费「`classification.levels[ℓ].bsp` 里有什么点什么 bits」的**抽象**，不依赖任何具体坐标集。
- 账本语义（分腿不先净额/父仓保持/级联关闭/realized 只出平仓腿）——纯结构性质，L0。
- bit-exact 嵌入恒等（§4.4）——对任意固定订单流成立，与 BSP 账本无关。

**依赖层（验收量测与信号总体，随关①②落地移动）**：
1. **嵌套触发的候选总体**：子声部的触发源 = 各级别 type1/2/3 BSP（经角色门筛 ShortDiff）。关① S2/S4（`signal.rs:1237/:1239/:1241` anchors→anchors_self，级别≥1 第一类**单调加宽**）与关② 037:20（`judge_first_cached` 加破 b 包络极值合取，**纯收缩**）共同改变该总体的最终成员。⟹ **本关一切验收量测（嵌套声部计数/级别分布/双开存活率/recover 率）必须在关①→关②串行链（T4 裁决序）验收完成后的最终 BSP 账本上跑**；在途账本上的任何量测读数无效、不得引用。
2. **关①单调性假设**：关①验收线（b）承诺「级别≥1 第一类新增点可枚举、既有点不消失」（T2 误杀=0 构造性证明）。本设计假设该单调加宽成立——若关①验收翻案，本关嵌套触发总体随之重估（接口不变）。
3. **关②纯收缩假设**：关②裁决 1 承诺「保留点逐字段不变，只删除破核心未破 b 极值候选」（含零 bit `struct_break` 候选域收缩，`struct_break_dir` 语义精确化）。本设计的触发只消费**非零 bit** 候选（方向 Flat 已归 𝒦，mod.rs:529 注释）⟹ 关②的零-bit 域收缩对本关透明；非零 bit 点的收缩直接减少嵌套触发数——如实依赖。
4. **t* 因果假设（U2 裁决 4）**：「双开等任何以该事件为依据的入场不得早于 t*」。recognize 路径的 BSP 来自 runner [A] 前缀因果重分类（`runner.rs:1172` 注释链：≤i 数据因果分类）——延迟案 BSP 在其 t* 确认后才出现在 `classification_i.levels[ℓ].bsp` ⟹ 双开入场不早于 t* **由因果流构造性满足**，本设计不加额外 t* 门（加了=双重口径）。
5. **关③ T1 正交**：终端背书查法移位（nest 事件层，`nest.rs`）不触碰 `classification.levels[ℓ].bsp` 内容（T4 验收线 b：关③ diff=0，账本只读查法变更）⟹ 与本关零交集，无假设。
6. **调度假设**：p116 后台重放完成前本关不实装任何代码（纪律）；本关实装窗口排在关①②③ 验收之后（串行纪律的自然延伸，避免 BSP 账本二次移动使嵌套量测作废）。

---

## 3. 方案 A：recognize 嵌套子声部产出设计（depth>0、σ_child=−σ_parent、级联关闭）

### 3.1 总形态：单脊柱赋格树（最小侵入，账户结构零改动）

声部树 = 一条**脊柱链**：根（depth 0）→ 子（depth 1）→ 孙（depth 2），`depth < config.voice.max_depth`（默认 3，voice.rs:200）。选单脊柱的理由：账户/台账现状全部是 **depth 索引单槽**（`AccountState.voice_qty: Vec<u32>` mod.rs:130；`held: Vec<Option<HeldVoice>>` exit.rs:37+runner.rs:2715）——单脊柱下**零结构改动**；父身份结构性蕴含（depth d 的父=depth d−1），**不需要给 `VoiceDecision` 加 parent 字段**（避开 ~15 处构造点的冲突面）。FULL 八一般树 T=(V,p) 的多孩子分叉（同 depth 多声部）需 VoiceId 键控账户——如实列 **v1 边界外**（§7 遗留 L3），不伪造 generality（090）。

教义/形式依据：T16（每级别独立完整形态学，necessity_derivation.md:403）+ T21（并发=级别同时性，:517）+ T45（操盘结构周期，:938）+ 四步 1-cycle（recursive_fugue_necessity_proof.md §3，配对 −2 非平凡）+ FULL 八（σ_v=σ_r(−1)^{d(v)}）+ M13（短差=父仓保持+次级别反向双开）+ M12（角色互斥穷尽 Σ1=1）+ 093:22/025:126（先卖后买四步）。

### 3.2 产出规则（角色门 + 活父判据）

**新函数**（strategy/mod.rs 新增，与 `recognize` 并列；`recognize` 本体零改）：

```rust
pub fn recognize_nested(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],     // classify_with_tower 第二返回值（classifier/mod.rs:490，Classification bit-identical）
    active: &[ActiveLeg],               // 调用方 held 台账的活动腿投影（见下适配）
    bars: &[Bar],
    config: &ThetaConfig,
) -> Vec<VoiceDecision>
```

逐时刻 x（source_index 分组，同 recognize 的 moments 循环骨架）：
1. `gamma = interp::assemble_gamma_with_tower(classification, tower)`（interp.rs:451）——候选序与扁平 `assemble_gamma` **bit-exact 相同**（其契约注释 interp.rs:441-442），唯一差异 = `role` 从真父子塔派生（V 真出 FollowParent/ShortDiff）。**角色单源**（coverage::operation_role，coverage.rs:1342）——禁在 recognize 侧另写角色判据（090/单一来源纪律）。
2. `buckets = interp::interpret(&gamma_x, active)`——传真活动集（`recognize` 恒传 `&[]`，mod.rs:493）；𝒟_x 反向关闭桶自此非空（§3.5 级联之外的**常规反向关闭**由 interpret 规则2 产，exit=true 决策）。
3. 对 `buckets.open` 每候选 c（`cand.dir != Flat`）：
   - **子声部门**（四合取）：`c.role.v == Vertical::ShortDiff`（δ_g=−σ_{p(g)}，coverage.rs:933）∧ **活父存在**（`active` 中 depth_p 槽有腿，side_p 非 Flat）∧ **方向对偶**：`cand.dir == flip(side_p)`（候选绝对方向=父侧翻转——ShortDiff 语义 `Side(e)=−σ_{α_e}` M27 的运行时校验）∧ **深度余量**：`depth_p+1 < max_depth` ∧ **附着一致**：候选的真父容器身份=父腿 carrier（`cand` 经 `coverage_elements_with_tower` 附着链的 `parent_id` == 父腿 `ActiveLeg.id`；`ActiveLeg.parent_id/op_parent` 字段已承载此链，interp.rs:126-138）。
     ⟹ 产**子决策**：`build_child_decision(cand, point, depth_p+1, root_side=父腿.root_side, ..)`——`depth=depth_p+1`，**`root_side` 继承树根方向（不取 `cand.dir`，理由见 §3.3）**，`exit=false`。
   - 否则 ⟹ 走现行 `build_decision`（depth=0 独立根，mod.rs:521，**零改**）。
4. `buckets.close`（活动腿遇反向候选）⟹ `exit=true` 决策（复用入场快照，同 exit.rs:132-137 形态）。

**级内互斥、级间并发**（M12/T21）：同一时刻多候选经 `interpret` 的 ≺_Θ 全序 + fold 唯一化（interp.rs:19-22 ∃! 三依据）——级内互斥由解释器承载；父子声部持仓并发由双账本（§4）承载；冲突排序沿用 `ConflictKey`（退出先/高 level 先/类序/(ts,src)/depth 终局键，mod.rs:290-296/:305-311），depth 键使父子订单全序确定。

### 3.3 σ_child=−σ_parent 的代数兑现（root_side 继承是关键设计点）

`voice_side(root_side, depth) = root_side·(−1)^depth`（voice.rs:98）。设树根方向 R、父深度 d_p：
- side(parent) = voice_side(R, d_p) = R·(−1)^{d_p}
- side(child) = voice_side(R, d_p+1) = R·(−1)^{d_p+1} = −side(parent) = flip(side(parent)) ✓

**不变量一致性**：§3.2 门要求 `cand.dir == flip(side_p)` ⟹ 子决策的绝对方向 side(child) = flip(side_p) = cand.dir——**赋格交替与候选信号方向代数一致**（一致是门的**判据**，不是假设：方向不对偶的候选根本不开子）。M11「父级多头的短差=次级别做空/父级空头的短差=次级别做多」由此逐例成立。

**为什么 `root_side` 不能取 `cand.dir`**：若子决策 `root_side=cand.dir=flip(R·(−1)^{d_p})`，则 voice_side(cand.dir, d_p+1) = cand.dir·(−1)^{d_p+1} ≠ cand.dir（d_p+1≥1 时多翻一次）——绝对方向错。继承树根 R 使 `voice_side` 的 (−1)^depth 恰好落在候选方向上。故 `build_child_decision` 与 `build_decision` 的唯一字段差异 = `root_side`（继承）与 `depth`（>0）；`stop_in/entry/signal_index/bsp/level` 同法从 `BspPoint` 零重算读出（mod.rs:532-555 同骨架）。

### 3.4 活动腿投影适配（held 台账 → `&[ActiveLeg]`）

`recognize_nested` 的 `active` 参数由调用方从 held 台账投影：每 depth 槽的活声部 → `ActiveLeg { level: d.level, dir: voice_side(d.root_side, d.depth), source_index: d.signal_index, lambda: 入场λ, id: carrier ElementId, parent_id: depth>0 ? Some(父 carrier) : None, is_boundary_root: depth==0, op_parent: … }`。carrier 身份：开仓时从候选附着的元素 `id` 冻结（`CoverageElement.id` 跨 bar 稳定，coverage.rs:172-175）——需在 `HeldVoice` 侧增补 `carrier: ElementId` 一个字段（exit.rs:37，唯一结构改动点；`record_held_voice` 同步，exit.rs:49）。**备选**（零字段改动）：身份用 `(depth, signal_index)` 复合键——但 depth 索引下 signal_index 已唯一，实则**可不增字段**，附着一致判据改用 `(parent.depth, parent.signal_index)` 对候选的 `(父容器 level, 父容器 ρ)` 匹配。两案在 §7 列 D1 待裁（倾向零字段案，冲突面更小）。

### 3.5 级联关闭（exit.rs:121 实义化 + cascade 发射）

**两处改动**：
1. `exit_decision_for`（exit.rs:85）签名加 `parent_invalid: bool`（或新增包装函数 `exit_decision_for_nested(hv, depth, bar, i, groups, equity_now, parent_state)`）：`CloseTriggers.parent_invalid`（exit.rs:121）从恒 `false` 改为**实参**——对 depth>0 声部，调用方按 `held[depth-1].is_none() ∨ held[depth-1].exit_pending` 计算；depth=0 恒 `false`（根无父，现行语义保留）。§9 closePred 四析取（X = ¬ParentValid ∨ χ^{σ_p} ∨ Stop ∨ RiskClose，exit.rs:18）自此四项皆实。
2. **cascade 发射**（调用方：runner `plan_and_fill_mtm_dual` 与 nautilus `on_bar` 的退出循环）：当 depth d 的 `exit_decision_for` 触发，对所有 `j>d` 且 `held[j].is_some()` 强制产退出决策（`exit=true`，复用入场快照），**最深优先**（j 降序）——对齐 M16 AncOK「父关则子关」与 §20 先平后开执行序；ConflictKey 退出批内 depth 终局键保证订单序与决策序一一对应（runner.rs:2742-2746 已有同形配对论证）。`exit_pending` 标记抑制 fill 前重复触发（exit.rs:33-35 机制 + nautilus/strategy.rs:149-151 同形检查沿用）。

**现金序**：双账本下子腿平仓收/付现金独立成腿（§4.3），最深优先使子腿现金先到位、父腿后平——与现行「退出先于开仓」（spec:54，mod.rs:282）正交叠加。

### 3.6 sizing 与风控消费链（parent_cap 首次生产可达 + 毛闸门）

- `plan_orders`/`build_open_order` **本体零改**：depth>0 决策到达后，`:348` 的 `risk::parent_cap(account.qty_at(d.depth-1), &config.risk)` 自然激活=floor(父持仓·β)；父空仓 ⟹ cap=0 ⟹ qty=0 ⟹ None（risk.rs:255-256/:218-220）——与 §3.2 活父门**双保险**。
- **毛敞口闸门**（新增，声明为 [设计选择,Θ_risk]）：`plan_orders` 汇总后（或 `plan_and_fill_mtm_dual` 每 bar），校验 Σ_v q_v ≤ γ̄·base_units——判定调 `risk::gross_units_ok`（risk.rs:572，coverage.rs:1687 同一 predicate **单源**，不私写比较）；超限 ⟹ **拒当步边际子开仓**（fail-closed，边际腿=当步最新子决策；不缩放既有腿——缩放规则属另一裁定）。依据：strict §11 line 359「净约束不能替代毛约束」（risk.rs:540-541 注释已锚）；多声部双开使毛敞口真实化是嵌套启用的**必然伴生**（audit #5 预留面原文即指此）。
- 配置不动：`β=0.5`（risk.rs:997-999 测试锚）、`depth_weights=[0.60,0.30,0.10]`（voice.rs:12）、`max_depth=3`——全部沿用现 config；C2 待裁（1/3 机动份额 f=1/λ vs β=0.5，bsp_consumption_redesign §9.7 C2）与 M15 同单位数（κ=1）两套 sizing 哲学的张力**登记不裁**（§7 L4），本设计不改任何默认值。

### 3.7 四步 1-cycle 在双账本上的形态（M13 父仓保持）

recursive_fugue_necessity_proof §3 的四步 γ：①平多@k → ②开空@k−1 → ③平空@k−1 → ④做多@k（配对 ⟨φ_c,γ⟩=−2≠0，非平凡性的充要根因=穿 ε=−1）。在 **M13 + 双账本**下：
- ①④**恒等化**（父仓保持 q_{α,t+}=q_{α,t}，M13/M27）——父多腿在双账本上不穿净额翻转，父腿生命周期独立于子腿；
- ②=子 ShortDiff 开仓（§3.2 门）；③=子腿 §9 closePred（反向信号 χ^{σ_p}：持空子遇买侧候选）或止损或级联平仓；
- 循环净效果 = 子空腿 realized PnL = **父降成本金额**（G5 双层记账同数，bsp_consumption_redesign §9.7 G5；T37 child.P&L≡parent.cost_reduction）——一个数字的两个会计身份，由 TW `TwEvent::ShortDiff` 通道结算（ledger.rs TW 六构造子之一）。
- RF-cyc 的「穿 ε=−1」在双账本上 = 子空腿的**存在本身**（不是父腿的翻转）；单边上扬 H¹ 幅度→0（026:80，RF-NR2）= 激活有效域——**引擎结构产出，激活度是 L2 读数**，本设计不预装 regime 门（§7 L5）。

---

## 4. 方案 B：fill 双账本负仓改造（q⁺/q⁻ 分腿不先净额）

### 4.1 新对象 `DualLedger`（M14 `P^sep` 的账户层兑现）

**新文件 `rust/src/theta_v0/backtest/dual_ledger.rs`**（runner.rs 已 6515 行，独立文件减冲突面）：

```rust
/// 双账本（hedge-mode 分腿头寸簿，M14 `P^sep=∏_{v}(R≥0 e⁺_v ⊕ R≥0 e⁻_v)`）：
/// 多腿/空腿为两个独立坐标（q⁺≥0, q⁻≥0），不先净额抵消（M14/M30）。
pub struct DualLedger {
    pub cash: f64,
    pub q_long: f64,    // ≥0：多腿手数（M14 e⁺ 坐标）
    pub q_short: f64,   // ≥0：空腿手数（M14 e⁻ 坐标）
    pub cost_long: f64, // 多腿每单位含费成本基（买入均价含买入费，同 apply_fill 多侧口径）
    pub cost_short: f64,// 空腿每单位含费成本基（卖出均价扣卖出费净收，同 apply_fill 空侧口径）
}
impl DualLedger {
    pub fn net_units(&self) -> f64 { self.q_long - self.q_short }   // Net(p)=Σ(q⁺−q⁻)，M14 另行定义的净额映射
    pub fn gross_units(&self) -> f64 { self.q_long + self.q_short } // 毛敞口（strict §11 G_t）
    pub fn equity(&self, px: f64) -> f64 { self.cash + self.net_units() * px } // 估值用净投影
}
```

### 4.2 订单腿标记 `LegOrder`（types.rs `Order` 零改）

```rust
/// 腿标记订单（strategy 层包装，不改 types.rs Order 的冲突面）：
/// leg=作用腿（Long=多腿/Short=空腿）；close=true 平仓腿 / false 开仓腿。
pub struct LegOrder { pub order: Order, pub leg: VoiceSide, pub close: bool }
```

产出侧：`plan_orders_dual`（新包装）——对每个 decision 先算 `side=voice_side(d.root_side,d.depth)`（mod.rs:268 同式），开仓决策 ⟹ `LegOrder{leg:side, close:false}`（Buy=开多腿/Sell=开空腿）；退出决策 ⟹ `LegOrder{leg:side, close:true}`。leg 由 decision 单一决定（voice_side 单源），订单↔decision 配对沿用 runner.rs:2747 的 zip 模式（ConflictKey 排序确定性保证一一对应）。**歧义消解**：现行 `StrictAction::Close` 不带方向（apply_order 按持仓符号推导，runner.rs:3134-3136 注释）——LegOrder 显式携腿，消除该推导。

### 4.3 `apply_fill_dual` 成交规则（分腿，不先净额）

| 订单 | 规则 | 现金 | 成本基 | realized |
|---|---|---|---|---|
| open Long qty | q_long += qty | cash −= qty·px·(1+fee)（现金不足 ⟹ rejected，同现行 need_cash，runner.rs:3239-3251） | cost_long 加权平均（同 :3260-3261 式） | 0 |
| open Short qty | q_short += qty（**q_long 不动**——M13 父仓保持，不先净额） | cash += qty·px·(1−fee)（无预付，保证金未建模声明沿用 :3119-3121） | cost_short 加权平均 | 0 |
| close Long qty | q_long −= min(qty,q_long)（余量 rejected，**不借机开反向**——close_only 语义保留 :3135-3136） | cash += 平量·px·(1−fee) | 全平 ⟹ cost_long=0（同 :3226-3228） | += (px·(1−fee) − cost_long)·平量 |
| close Short qty | q_short −= min(qty,q_short) | cash −= 平量·px·(1+fee) | 全平 ⟹ cost_short=0 | += (cost_short − px·(1+fee))·平量 |

- **PnL 公式与现行逐字同构**（runner.rs:3210-3216 的统一式按腿拆分），成本对称保留。
- **A' 保留**：`FillOutcomeDual { closed_long, closed_short, opened_long, opened_short, rejected, realized, fee }`——`realized` 只在平仓腿产，TW Realize 唯一合法资金源的结算边界不变（runner.rs:3178-3184 口径按腿延伸）。
- **费用口径**：fee = 成交名义×fee_rate 逐段累计（同 :3209/:3252），RDecomposition 守恒断言的独立测得保留。

### 4.4 bit-exact 嵌入恒等（核心论证：现行净语义是双账本子集）

**定理（嵌入恒等，构造性）**：对任一固定订单流，存在腿标记方案（**兼容腿标记**）使 DualLedger 逐笔产出与现行 `apply_fill` 的 (cash, equity, realized, fee) 完全相同：
1. **现金流逐笔恒等**：Buy/Sell 的 cash 变动公式两账本同形（Buy: −qty·px·(1+fee)；Sell: +qty·px·(1−fee)——与腿无关，runner.rs:3208/:3237 同式）。
2. **估值恒等**：双账本 equity = cash + (q_long−q_short)·px；净账本 equity = cash + units·px。兼容腿标记下不变量 `q_long−q_short == units` 逐笔保持（开多 +q/开空 −q/平多 −q/平空 +q，与 units += delta·qty 同转移）。
3. **realized 恒等（兼容腿标记的定义）**：把现行「先平后开」翻译为两腿操作——`Sell qty while units=+N` ⟺ `close Long min(qty,N)` + `open Short (qty−min(qty,N))`；`Buy qty while units=−N` ⟺ `close Short min(qty,N)` + `open Long 余量`。此时段1 平仓腿的 realized 公式 == 现行段1 公式（:3210-3216 逐字），段2 开仓腿不产 realized（:3181「段 2 开新仓不产 PnL」同）。
4. **费用恒等**：平仓段+开仓段费用分腿累计 == 现行两段的合计（同名义×同费率）。

**推论（回归锁）**：现行 `plan_and_fill_mtm` 的任意历史交易序列可在 `plan_and_fill_mtm_dual` 上以兼容腿标记重放，全部账本数字**逐字节相同**——这是单测 D8 的对拍对象（§6）。**真正分叉只出现在嵌套声部启用后**（新订单形态「持多腿时开空腿」在旧账本不存在对应物）——那是本关的**新能力**，不是旧数字漂移。M30 有效域声明同步落地：分账本腿级收益（G^sep>0，M24）≠ 净额账户收益命题；equity 估值仍取净投影（§4.1），两腿浮盈亏在净值上抵消（M30 原文语义）。

### 4.5 下游口径对账（既有消费者的逐一安置）

| 下游 | 现行（净账本） | 双账本口径 | bit-exact？ |
|---|---|---|---|
| `cum_price_pnl`（M6 R 分解，runner.rs:1090） | Σ units·Δpx | Σ (q_long−q_short)·Δpx | **恒等**（PDF §11 线性，overlay_state.rs:22 已锚） |
| TW 成本划转（:1137） | \|units\|·\|entry_cost\| | 分腿：q_long·cost_long + q_short·cost_short（在险市值分腿和） | 兼容腿标记下相等；双开共存时**语义差**（声明：双开两腿都在险=分腿和，是 M14 语义的忠实口径，非旧 bug） |
| 窗口终点强平（:1758） | units≠0 强平 | 双腿各自强平（两笔 forced，合成现金相同） | 恒等（线性） |
| trade 配对（:2945） | units 跨 0 | 每腿 entry_bar 配对（(depth,leg) 键） | 兼容腿标记下同一交易列表 |
| TW Realize（A'） | fill.realized | FillOutcomeDual.realized（平仓腿和） | 恒等（§4.4-3） |
| maint_margin/funding/borrow（risk.rs:646/:809/:819/:825；runner.rs:1118-1120 喂 net_notional_usd=\|units\|·px） | 以 \|N\| 为基 | v1 沿用 \|net_units\|·px | 嵌套未启用时恒等；**双腿共存时为近似口径（诚实声明）**——per-leg 精化（borrow 基=空腿名义、funding 基=毛额）列遗留 L6，涉成本数字变更须独立裁定 |
| 毛/净杠杆读数 | （无生产消费者） | gross=(q_long+q_short)·px / net=\|q_long−q_short\|·px 喂 `risk::leverage_metrics`（risk.rs:509）——其**首位生产消费者** | 新读数（不污染旧数字） |

### 4.6 接线：`plan_and_fill_mtm_dual` + 新入口（旧路径零改）

- runner.rs 新增 `plan_and_fill_mtm_dual`（与 `plan_and_fill_mtm` 并列，~250 行）：账本态换 `DualLedger`；`apply_order` 调用点换 `apply_fill_dual`；`apply_voice_fill` 语义不变（voice_qty[depth]=手数——单脊柱下 depth↔腿 1:1，side 由 held 台账定）；退出循环换 cascade 版（§3.5）；开仓循环喂 `recognize_nested`（§3.2）；`k_theta` 毛闸门（§3.6）。
- 新入口 `run_theta_v0_dual`（与 `run_theta_v0` 并列；`run_theta_v0` 签名/行为零改 ⟹ 全部现有 bin/测试 bit-exact）。
- nautilus/strategy.rs：`recognize_current`（:220-224）切 `classify_with_tower`（Classification **bit-identical**，classifier/mod.rs:482 契约）+ `recognize_nested`；退出循环（:144-164）接 cascade。**venue 侧 hedge-mode 账户前提**在 §7 L7 声明（净额 venue 上 q⁺/q⁻ 不可共存——双账本的 venue 适配是部署裁定，非本关）。

---

## 5. bit-exact 冲突面

### 5.1 触及文件清单（设计，实装时逐 hunk 核对）

| 文件 | 改动 | 性质 |
|---|---|---|
| `rust/src/theta_v0/strategy/mod.rs` | +`recognize_nested`（~120 行）、+`build_child_decision`（~60 行）、+`plan_orders_dual`/`LegOrder`（~50 行）、+毛闸门调用（~15 行）、测试区 +10 | **纯新增**；`recognize`/`plan_orders`/`build_decision` 本体零改 |
| `rust/src/theta_v0/strategy/exit.rs` | `exit_decision_for` +`parent_invalid` 实参（签名 1 处 + :121 实义化）或新增 nested 包装；+cascade 辅助（~40 行）；测试 +4 | 签名改 1 处（两调用方同步）；`:121` 恒假占位消除 |
| `rust/src/theta_v0/backtest/dual_ledger.rs` | **新文件**（DualLedger + apply_fill_dual + FillOutcomeDual + 8 单测，~400 行） | 新建 |
| `rust/src/theta_v0/backtest/runner.rs` | +`plan_and_fill_mtm_dual`（~250 行）、+`run_theta_v0_dual`（~30 行）、+mod 注册 1 行、测试 +4 | **纯新增**；`apply_order`/`apply_fill`/`plan_and_fill_mtm`/`apply_voice_fill`/`simulate_fills` 本体零改 |
| `rust/src/theta_v0/backtest/mod.rs` | +`pub mod dual_ledger;` 1 行 | 注册 |
| `rust/src/theta_v0/nautilus/strategy.rs` | `recognize_current` 切 `classify_with_tower`+`recognize_nested`（~10 行）；退出循环接 cascade（~15 行） | 接线；Classification bit-identical |
| （D1 备选才触及）`strategy/exit.rs` `HeldVoice` +`carrier` 字段 | 视 §3.4 D1 裁定 | 倾向零字段案 |

### 5.2 不碰清单（主干零触碰，无需升级裁定）

- **classifier/\* 全部**：BSP/塔/中枢/笔/线段/级别主干——`P92_BIT_EXACT` 五维构造性不受影响。
- **净额 fill 路径**：`apply_order`（runner.rs:3122）/`apply_fill`（:3167）/`FillOutcome`（:3281）/`plan_and_fill_mtm`（:2679）/`run_theta_v0`（:274）及其全部现有臂（metrics/equity/trade_pnls/TW/R 分解）。
- **coverage.rs / overlay_state.rs / interp.rs / persistent.rs / oscillation.rs / pan_div.rs**：复用只读（`assemble_gamma_with_tower`/`operation_role`/`gross_units_ok` 调用，不改其本体）。
- **risk.rs sizing/parent_cap/leverage 公式、voice.rs 全部**：复用只读。
- **types.rs**：`Order` 不加字段（LegOrder 在 strategy 层包装）。
- **strategy/ledger.rs**（R/TW 账本）、**config.rs**（不加新参数——双账本以入口函数区分，零 config 改）。
- **formal/ Lean 全部**（rust 领先登记，同 T3 L1 先例）。
- **关①②③ 在途实装面**：signal.rs 判据链 / nest.rs 终端查法 / level_view.rs / divergence.rs / recursive_tower.rs / p92 bin / p116 规则书——**零文件交集**（本关文件清单与之无一重合）。

### 5.3 bit-exact 论证

1. **构造性**：旧函数/旧入口/旧账本全部零改（§5.1 纯新增列）⟹ 现有全部臂（`cargo test --lib`、L3 bins、perf profile、P92/P116 探针）数字不动。
2. **嵌入恒等**（§4.4）：双账本含现行净语义为子集，回归锁以逐字节对拍实装（单测 D8）。
3. **Classification 同源**：`classify_with_tower` 与 `classify` 共用 `classify_impl` 单一来源，返回值 bit-identical（classifier/mod.rs:482 契约注释）——nautilus 切换无分类层漂移。
4. **验收线（供裁定）**：`cargo test --lib` 既有测试零**非授权**变红；`P92_BIT_EXACT` 五维 diff=0（classifier 未碰，构造保证+重放复验）；`run_theta_v0` 既有 L3 基线逐标的 diff=0（旧路径零改）；新路径读数单独建账（不与旧基线混排）。

---

## 6. 单测计划（测试名 + 断言 + 数据构造）

**A 组：recognize 嵌套产出（strategy/mod.rs 测试区，6 个）**
- A1 `nested_shortdiff_child_depth1_side_flipped`：构造 Long 根活父（depth 0，held 投影）+ 其真子容器上的 sell1 候选（role.v=ShortDiff，塔夹具同 coverage.rs 测试）⟹ 断言产 1 决策：`depth==1`、`root_side==Long`（继承）、`voice_side(root,1)==Short==cand.dir`（σ_child=−σ_parent 代数锁）。
- A2 `nested_ambient_candidate_stays_root`：Ambient/First 候选（无真父或非 ShortDiff）⟹ depth=0 根决策，与现行 `recognize` 输出逐字段相等（回归锁）。
- A3 `nested_direction_mismatch_no_child`：ShortDiff 角色但 cand.dir≠flip(父侧)（如父 Short、候选 sell）⟹ 不产子（落 depth=0 根域或不产——按角色门四合取逐项断言）。
- A4 `nested_max_depth_boundary`：父在 depth=max_depth−1 ⟹ 不产孙（within_max_depth 门）。
- A5 `nested_no_live_parent_no_child`：无活动集 ⟹ 输出==`recognize`（单帧等价，active=∅ 坍缩锁）。
- A6 `nested_deterministic_replay`：同输入两跑 ⟹ 决策 Vec 逐字节同（≺_Θ 全序确定性）。

**B 组：plan_orders 嵌套（strategy/mod.rs 测试区，4 个）**
- B1 `plan_child_parent_cap_binds`：父持仓 100（voice_qty[0]=100），子决策 depth=1 ⟹ sizing 项3=floor(100·0.5)=50 生效（qty≤50；risk.rs:971 既有单测的链路版）。
- B2 `plan_child_blocked_when_parent_flat`：voice_qty[0]=0 + 子决策 ⟹ parent_cap=0 ⟹ qty=0 ⟹ 无订单（risk.rs:981 链路版；与 A5 双保险）。
- B3 `plan_child_weight_and_direction`：子订单 action=Sell（Short 腿），w_depth=0.30 进项2（depth_weight[1]）。
- B4 `plan_mixed_conflict_order_stable`：父退出+子开仓同 bar ⟹ 退出先（ConflictKey.exit_first）且全序确定（两输入序对拍同输出，mod.rs:848-849 既有对拍模式延伸）。

**C 组：级联关闭（strategy/exit.rs 测试区，4 个）**
- C1 `cascade_parent_exit_closes_descendants_deepest_first`：held[0..3] 全活，depth 0 触发退出 ⟹ 产 3 决策，序 [depth2, depth1, depth0]。
- C2 `parent_invalid_fires_when_parent_slot_empty`：held[1] 活、held[0] 空 ⟹ depth1 的 X=true（parent_invalid 单项触发，stop/reverse/risk 全假对照）。
- C3 `cascade_exit_pending_suppressed`：父 exit_pending=true 时子不重复产退出（fill 前抑制）。
- C4 `cascade_root_only_regression`：仅 depth0 活 ⟹ 行为与现行逐字节同（parent_invalid=false 路径回归锁）。

**D 组：双账本（backtest/dual_ledger.rs 测试区，8 个）**
- D1 `dual_open_short_keeps_long_leg`（不先净额锁/M13 父仓保持）：q_long=10 时 open Short 6 ⟹ q_long==10 不变、q_short==6、cash 增量=6·px·(1−fee)、equity==cash+(10−6)·px。
- D2 `dual_coexist_net_zero_gross_nonzero`（M14 坐标非零/Net=0）：q_long=q_short=N ⟹ net==0、gross==2N、cash==nav−N·px(1+fee)+N·px(1−fee)。
- D3 `dual_close_long_realizes_per_leg`：两段加权成本基后 close Long ⟹ realized==(px·(1−fee)−cost_long)·平量（与 apply_fill 多侧公式逐字同构）。
- D4 `dual_close_short_realizes_per_leg`：镜像空侧 realized==(cost_short−px·(1+fee))·平量。
- D5 `dual_close_clamp_no_reverse`：close qty>腿持仓 ⟹ 平到 0、余量 rejected、不开反向（close_only 保留）。
- D6 `dual_cash_constraint_long_open`：现金不足 ⟹ open Long rejected（need_cash 同现行）。
- D7 `dual_equity_linearity_coexisting`：双腿共存下 equity==cash+net·px（PDF §11 线性对账）。
- D8 `dual_bitexact_embedding_vs_apply_fill`（**核心回归锁**）：随机/脚本化净语义订单流（Buy/Sell/Add/Reduce/Close 序列）经兼容腿标记重放 vs 现行 `apply_fill`——cash/equity/realized/fee/trade_pnls **逐字节相等**（§4.4 四恒等逐项断言）。

**E 组：集成（runner.rs 测试区，4 个）**
- E1 `e2e_four_step_cycle_dual_ledger`：合成 bars（上行→顶背驰 sell1→回调→底背驰 buy1→续上行）：根 Long 开 → 子 Short 开（父腿不动断言）→ 子买侧信号平（realized>0，价格下跌验证 G^sep>0 腿级语义）→ 父腿仍在 → 终点全平 ⟹ 现金守恒（cash_final==nav+Σrealized−Σfee）。
- E2 `e2e_g5_double_entry_same_number`：E1 中子空腿 realized == TW ShortDiff 事件的父降成本金额（G5 同数锁，TW 接线到位后启用；未接线前标记 ignore 并注释）。
- E3 `e2e_cascade_parent_stop_closes_child_first`：父止损触发时子活 ⟹ 交易列表子平先于父平、零残留持仓。
- E4 `e2e_nested_disabled_bitexact`：nested 关（无 ShortDiff 触发数据）⟹ `plan_and_fill_mtm_dual` 输出==`plan_and_fill_mtm`（兼容嵌入的链路级对拍）。

---

## 7. 风险与边界

1. **激活 regime 门不预装（RF-NR2，026:80）**：「每个次级别买卖点都必须操作」已被否证为非必然——激活是 regime-gated 的 H¹ 幅度（单边上扬 H¹→0）。本设计引擎层**结构产出**（信号决定），不设 regime 门；嵌套产量的有效性是关①②后 L2 量测，不预承诺（090）。若后续裁定加门，唯一合法挂载点 = `recognize_nested` 产出层（单点，禁散布）。
2. **现货/做空可行性**：q_short 是**真空头**（期货/永续域）；现货标的的「机动份额暂驻」读法（RF §3 边界条件、015:976 认沽语义）下 q_short 须 gate=0——部署层开关，本关引擎不区分标的（标的无特化纪律），声明有效域。
3. **保证金/借券未建模**（现行声明沿用 runner.rs:3119-3121）：双腿共存使毛敞口真实化，\|N\| 基 accrual 在共存时是近似（§4.5 表）——per-leg 精化列遗留 L6（涉成本数字，须独立裁定，不得顺手改）。
4. **单脊柱边界**：每 depth 一声部；FULL 八一般树 T=(V,p) 多孩子分叉需 VoiceId 键控账户（voice_qty/held 从 Vec-by-depth 升 HashMap-by-VoiceId）——v1 不覆盖，遗留 L3。
5. **G4 先验（bsp_consumption_redesign §9.7 G4）**：僵尸空头三路径 + recover 率级别衰减（rL2+ recover 率实测 0%）——嵌套子声部的高层短差有**结构性套牢先验**；验收读数必须 per-level 报告 recover 率与双开存活时长（不准只报合并数）。
6. **G1 摩擦地板**：低级别短差可能在摩擦地板下（segment 级 0.02-0.19% < 摩擦）——per-level P&L 须扣摩擦报告；最低可操作级别由摩擦地板决定，不无限下探。
7. **审计漂移再声明**：本关 fill 改造是净额→双账本（第二层），不是重复修 audit #1（已修，§1.1）；嵌套产出是 recognize 路径的补齐，与 coverage M29 引擎的既有嵌套腿是**两引擎同构物**——角色/元素身份单源复用（§3.2）防口径 fork；coverage 接 hedge 执行列后续关，本关不接（防范围蔓延）。
8. **遗留登记（本关不裁）**：L1 Lean 漂移（rust 双账本/嵌套领先 Origin.VoiceTree/SubVoiceOpenClose，同 T3 L1 先例登记 formal-chain）；L2 关①②翻案时的嵌套触发总体重估；L3 多孩子分叉账户；L4 β=0.5 vs f=1/3（C2）与 M15 同单位数 sizing 张力；L5 regime 激活门裁定；L6 per-leg accrual 精化；L7 venue hedge-mode 部署适配；D1（§3.4）carrier 身份字段两案。

---

## 8. 涉及文件清单（汇总）

**新增（2）**：`rust/src/theta_v0/backtest/dual_ledger.rs`；本文档。
**修改（5，全部纯新增区/接线，旧本体零改）**：`rust/src/theta_v0/strategy/mod.rs`、`rust/src/theta_v0/strategy/exit.rs`、`rust/src/theta_v0/backtest/runner.rs`、`rust/src/theta_v0/backtest/mod.rs`（1 行注册）、`rust/src/theta_v0/nautilus/strategy.rs`。
**只读复用（0 改）**：`strategy/{voice,risk,interp,coverage,overlay_state,persistent,ledger}.rs`、`classifier/*`、`types.rs`、`config.rs`。
**不碰**：关①②③ 全部在途实装面、`formal/`、主仓一切。

---

## 9. 090 纪律自检

- 教义引用全部于 2026-07-18 直读主仓 `docs/chanlun/text/blog/` 原文逐条核对：093:22 / 025:126 / 026:80 / 029:52 / 029:58 / 029:396 / 024:18 / 027:38 / 016:114 / 017:60 / 017:66 / 017:70 / 031:30 / 020:40 / 065:182（课号:段号=文件行号）。
- 代码坐标全部直读 worktree 现行状态核对：recognize=`strategy/mod.rs:454`（任务书「classifier/mod.rs」归属已订正，§1.1）；audit 三处 stale 锚点已订正（§1.1 表）；exit.rs:121 / runner.rs:3122/:3167/:969 / risk.rs:257/:441-547/:563-569 / interp.rs:275/:451 / coverage.rs:23-29/:1342/:1424-1436/:1614-1633 / voice.rs:98/:188/:200 / mod.rs:250/:268/:348/:521/:558 等逐点复核。
- 声明与实际能力一致：审计 #1 已修不重复邀功；`parent_cap`「有消费者但生产不可达」如实区分（§1.1）；双开产量/alpha 零预承诺（§7-1）；嵌套触发总体对关①②最终账本的依赖如实声明（§2）；单脊柱/现货/保证金/accrual 近似四处边界如实上浮（§7）。
- v3 硬禁令遵守：零概率/统计推断作决策基础；零回测验证策略（历史数据仅证代码正确性——D8/E4 对拍即此性质）；零有效市场假设；无新增依赖（全部复用 crate 内既有模块）。
- 零代码改动、零 git mutation、零 cargo 调用（p116 不受扰）、主仓零写入；新增文件仅本文（worktree 内）。

---

## 签字位

- [x] A 嵌套产出设计（recognize_nested + 角色门 + root_side 继承 + 单脊柱）——施工图落盘
- [x] 级联关闭设计（exit.rs:121 实义化 + cascade 最深优先）——施工图落盘
- [x] B 双账本设计（DualLedger + apply_fill_dual + LegOrder + 嵌入恒等）——施工图落盘
- [x] bit-exact 冲突面（主干零触碰 + 与关①②③零文件交集）——落盘
- [x] 单测 26 个（A6/B4/C4/D8/E4）——落盘
- [x] 输入假设（关①②依赖分层 + U2 t* 构造性满足）——落盘
- [ ] 编排者裁定（空位：D1 两案、串行排期、是否另立 coverage hedge 接线关）
