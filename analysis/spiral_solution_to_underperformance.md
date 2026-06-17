# 螺旋覆盖空间对 BH-underperformance 三问题的解：五方向裁决报告

> 综合裁决工位（2026-06-16）。整合方向 A–E 的推导 + 每方向四棱镜对抗验证。
> 方向 A–D 由工作流（5 方向独立推导 × 4 棱镜对抗验证）产出；**方向 E 的专职 agent 因 API 500 中断，由 Lead 直接从源码 + 8 标的实测数据补全（§5）**。
> 源文件逐字核验：`docs/necessity_derivation.md`、`docs/spiral_exhaustive_enumeration.md`、
> `docs/covering_space_confirm_verification.md`、`analysis/unn_latest_behavior_analysis.md`、
> `trading_system/data_cache/unn_stream_*.json`（8 标的实测 liq 计数）、
> `.chanlun/escalations/2026-06-15-theta-allocation-non-necessity.md`、
> `.chanlun/escalations/2026-06-16-confirmed-root-raw-asymmetry.md`。
>
> **认识论等级**：每个结论标注 L0（纯代数/定义）/ L1（合成数据）/ L2（单标的真实）/ L3（交叉验证）。

---

## 0. 总裁决（TL;DR）

| 方向 | 标题 | 最终裁决 | 认识论等级 | 必然性 vs 有效域 |
|------|------|---------|-----------|-----------------|
| A | spawn 的 r 下界（弧长约束） | **negative**（否定性结果，有信息增量） | L3 | validity_domain_reading |
| B | 多树叠加（per-BSP 建树） | **rejected** | L0 | 不适用（与 A₃/N1/T42 矛盾） |
| C | ε 翻转的 r 约束（势能门槛） | **rejected** | L0 | validity_domain_reading |
| D | 确认滞后的补偿（分批/预期建仓） | **rejected** | L0 | validity_domain_reading |
| E | 遗漏推论扫描 | **negative / escalate**（找到 T₄₁ 但被 L3 否证） | L3 | necessity_violation（声明层） |

**严格解决方案数 = 0。** 五个方向无一是 strict（严格解）或 partial（部分解）。
方向 A/E 是有正信息增量的产出：A 证明零参数标的无关 r_min 在成本门层不可能（否定性）；**E 发现 58 条里直接指向"子空头全亏"的推论 T₄₁/T₄₆ 被 161 次实测强平 L3 否证，须 /escalate**（唯一必然性违反候选，非严格解）。

**三问题的必然性诊断**：
1. **天花板（root ≤ BH）= 有效域读数（C1）**，不修。是 A₃（恒仓）+ T₂₈（最高级别入场）+ T₄₉（向心确认滞后）三条必然性的联合不动点；BH-underperformance 是强牛 regime（非盈利 regime）的有效域边界，不是可补偿缺陷。
2. **子空头全亏（BTC 89 笔 wins=0）= 有效域读数（C1）**，不修。是 T₂₅（父多必子空，L0）的方向撞上 ambient 价格单边上行（价格 ⊥ 螺旋，C2）；539 号已结算「子空头/翻空有效域 ⊂ 非上行 regime」。
3. **units 稀释（max_children=80）= 必然性后果（C1）**，不修。是 T₃₄（Σunits=N_base 守恒）+ T₁₈（spawn 配额 f=1/λ，σ-不变 L0）的几何强制；主升浪暴露缩水是守恒律 + spawn 机制的派生量，非缺陷。

**唯一必然性违反候选（高亮，须单独处理）**：**T₄₁ 零强平** 声明为「无条件 L0」（编排者 2026-06-15 Phase-4 裁决，`necessity_derivation.md:1259-1262`），但 8 标的实测 liq 计数（BTC=89 / OKLO=36 / ES=22 / BRN=14，共 161 次实际强平，`unn_stream_*.json:exit_reasons.liq`）**经验否证**该无条件声明。详见 §5（方向 E，机制）+ §6.4（escalation 分类）。

---

## 1. 方向 A：spawn 的 r 下界（弧长约束）

**最终裁决：negative（否定性结果，有正信息增量）｜L3｜validity_domain_reading**

四棱镜（zero-param 已运行，high confidence）与推导一致，且 zero-param 棱镜的唯一裂隙**强化而非翻转**结论。

### 必然性依据（推论编号）
- **T₈ / T₂₈**：势源下界 `PENDING_LO = FIRST_BSP_LADDER + 1 = move(L1) = 3`；segment（ladder=2）是第一个有势但非势源（势未经中枢检验）。这是**唯一零参数结构 r_min**（level=圈数=结构常数，L0）。源码 `unified_necessity.rs:263, 625`（segment 非势源），`necessity_derivation.md:625-630`。
- **T₅₆**（角向-径向 holonomy）：confirm fire（角向圈 φ=0）只承载于 k≥PENDING_LO；segment 层出现 confirm fire 即 panic。L3 验证：8 标的 `fire_sell[2]=fire_buy[2]=0` 全零（零 panic）⟹ `nf_*[segment]≡None` ⟹ E 永不从 segment fire。**这修正了原候选链「E 可下探到 segment spawn_seg」的措辞**——E 是 move(L1) 父 fire 后 spawn 入 segment 子，非「segment 父 spawn」。
- **T₁₉**（成本门 = 递归终止）：`theta(sub) < friction` 即拒 spawn。这是决定 spawn_seg 成败的**真正 r_min 机制**。θ=振幅是 L2 经验量——`necessity_derivation.md:490-492` 证明「θ<f ⟹ 势在操作语义下消失」，比较对象是振幅 θ vs 摩擦 f；λ^k 恒正无法表达「势<friction」⟹ **纯结构 θ 不能判定势消失，零参数成本门被 T₁₉ 封死**。
- **T₂₀ / T₄₃**：N7 降成本用 voice 自层 nf 不查链；spawn_seg 走 `nf_*[move(L1)]` 逐层路，与级联 located 链分离 ⟹ 禁 spawn_seg 不推翻 N7、不破 N5⊥N7 张力解。
- **T₅₀**：径向标度律 f(k)∝λ^{−k} 解释 spawn_seg 在最内势源层 move(L1) 频繁是标度律必然。
- **T₁₈**：spawn 配额 f=1/λ 已升 σ-不变零参数（theta-allocation 裁决，`SUB_SPAWN_FRAC`，`positional_fusion.rs:102`），由 `prove_theta_sigma_invariant` 运行时 panic 守卫——**但配额零参数 ≠ 成本门零参数，二者范畴分离**。

### 推导链
1. ladder 映射逐字核验：bi=1, segment=FIRST_BSP_LADDER=2, move(L1)=PENDING_LO=3（`types.rs:26-30,40-47`）。
2. 唯一零参数结构 r_min（PENDING_LO=move(L1)）**已实装且最大化强制**（T₅₆ 守卫 + L3 fire[2]=0），但它**不阻止 spawn_seg**——spawn 入 segment 的父在 move(L1)=PENDING_LO，已满足 source≥PENDING_LO（合法 fire）。
3. 禁 spawn_seg 须把允许 spawn 的父级别下界从 move(L1) 抬高，或在 move(L1) 父处加门。前者改 T₈/T₂₈ 结构定义（否定 move(L1) 是势源，违缠师第 67/71 课线段=次级别走势本体）；后者就是成本门 θ 振幅门（标的依赖）。**故无「从推论推出的零参数 segment-spawn 禁令」。**
4. 成本门 r_min 标的依赖性（C2）：L3 数据 `cost_rej[2]` = QQQ5/DX24/GC62/CL31/ES58/BRN98 vs BTC0/OKLO0。BTC/OKLO 振幅大 theta 恒过 ⟹ spawns[2]=89/36 ⟹ 踏空（fail）；QQQ/DX 振幅小 theta 恒拒 ⟹ spawns[2]=0 ⟹ 纯 root 持有 ⟹ PASS。判别量 = 各标的 segment 中枢振幅相对 friction = ambient/regime 量。

### 是否经验参数
**非零参数（zero_parameter=false）。** 成本门 r_min 由 `depth_ref.theta()` 决定，θ(k)=最近 window 个中枢相对振幅 (ZG−ZD)/c 的 q 分位（`depth_ref.rs:61,88-110`），是价格振幅。价格是螺旋嵌入坐标（螺旋⊥价格，C2），不在 D∞ 三坐标 {φ,r,ε} 中。

**zero-param 棱镜补强（裁决文本应吸收）**：成本门含**四个独立经验/建模常数，超出 a0 继承**——`SUB_FRICTION_RT=0.001`（A₄ 只给 f>0 不给 0.001 值）、`SUB_COST_K=2.0`、`SUB_COST_Q=0.5`（P50 分位）、`SUB_COST_MIN_OBS=10`/`WINDOW=50`（`config.rs:671,674`，全局默认无 per-asset override）。这比裁决原文承认的「a0 继承」更参数化 ⟹ zero_parameter=false **更牢固**。但这些是**诚实经验依赖**（标的依赖来自数据振幅分布，不来自调出的标的常数），符合 C2「没有把回测调出的数伪装成结构常数」的合格标准。

**关键无偷换**：引擎严格分离两个 θ 角色——配额角色B（m=p_units×1/λ，σ-不变 L0，「势∝r」正确停在 r 的定义）vs 成本门角色A（`depth_ref.theta`，价格振幅，代码注释 `positional_fusion.rs:995-996` 明示「成本门仍用经验θ不变」）。**没有把振幅伪装成势∝r 结构常数。**

### 边界条件（结论翻转条件）
1. 若证明 move(L1) 自身不应是势源（PENDING_LO 应抬到 move(L1)+1）⟹ 零参数结构 r_min 可上移、spawn_seg 可结构禁止——但须改 T₈/T₂₈ 并与缠师第 67/71 课线段定义冲突（**C4 违反，须 /escalate 而非自决**）。
2. 若证明成本门 θ 可用纯结构量（圈数 λ^k）判定势存在性 ⟹ 成本门 r_min 可零参数化——但 T₁₉ 已封死（λ^k 恒正使 N4 终止失效，违 T₁₉）。
3. 若 regime 转震荡/下行，spawn_seg 子空头从负贡献转正（539 号；OKLO commit/short +7204、BRN move/short +7938）⟹「spawn_seg>0 必 fail」翻转——证明 fail 是 regime 函数（C1）非必然性违反。
4. 若实装 regime 门控（上行时抑制 spawn_seg）后 P1 仍不改善 ⟹「spawn_seg 是踏空源」被 L3 否证。

### 代码影响
**无引擎代码改动**（C1 有效域读数不修必然性）。零参数结构 r_min 已实装（`unified_necessity.rs` PENDING_LO 常数 + prove_t56 segment 无角向圈守卫 + self_level_counter_fire 层边界 nf_*[segment]≡None）。成本门 r_min 的标的依赖性是编排者 2026-06-16 已裁决保留的经验 θ（`escalations/2026-06-15-theta-allocation-non-necessity.md`），不改。**开放轴（非本方向裁决）**：E spawn 前加 root 涌现层方向门（上行抑制），属 regime 门控新轴，需独立 L3 验证，且引入 regime 判别可能违 C2（除非方向门用纯结构 located 链方向，零参数）。

### 缠论原文权威（C4）
无冲突，且**原文（线段=次级别走势本体，势源=move(L1)）正是禁止零参数 segment-spawn-ban 的权威依据**。T₈ 势源下界依据缠师第 67/71/77/78 课线段划分（编纂版遗漏已补录，`.chanlun/genealogy/settled/002-source-incompleteness.md`）+ 第 17 课级别完全分类——抬高 PENDING_LO = 否定 move(L1) 是势源 = 违原文。

---

## 2. 方向 B：多树叠加（per-BSP 建树）

**最终裁决：rejected｜L0｜不适用（与已结算公理/不变量矛盾）**

四棱镜（necessity / zero-param / chanlun-authority / behavior-falsify 全运行，high confidence）一致拒。

### 必然性依据（推论编号）
- **T₂₇**（永远在场）：当下点 (r_now,φ_now,ε_now) 在螺旋上 measure-1 恒存在，**唯一**（`necessity_derivation.md:607-620`）⟹ 一个 root，拒多树空间并存。
- **T₄₂**（孤儿不可能 + 森林单根不变量）：「同时至多一个 active root」（`:873-884`）。引擎 `prove_n1_forest`（`unified_necessity.rs:409`）每 bar 硬断言 `roots<=1`，多 root 立即 panic。**多树 = 多 active root = 直接违反已结算不变量。**
- **A₃**（恒仓公理，C4 决定性）：第 26 课「绝对不加仓，一开始就买够」。多树 = 回调买点叠加多头 = 加仓 = 明文禁止。
- **T₂₄ / T₁₄**（吃到每一笔的正确机制）：单根 in-place 时序手性翻转（ε 在每个 φ=0 翻转）+ 赋格子声部方向交替。`prove_t14_root_flip` 守 M=N 单根 dir 反转（翻转后断言 `roots==1`，`:356-359`）。这是**时序遍历**（temporal traversal）非空间并存（spatial coexistence），已实装（L2，25M bar 零 panic）。
- **T₃₄ / T₃₉**：Σ active voice.units = N_base 守恒；T₃₉（`:819-827`）证递归链任意深度 Σ(全链)=N_base。
- **T₅₅**：第二独立螺旋 D∞×D∞ 是比价/选股（M2）层，是「另一层」而非同标的多树——方向 B 不属于此。

### 推导链
1. **核心拒绝链（载重，2 支硬支柱）**：
   - **N1/T₄₂ 单根**：多树要求拆除 `roots<=1` 守卫。方向 B 要**拆除**已结算生成元/不变量，而非从 58 条推出——正是 **C3 禁止的反向工程**（无新公理、无新 D∞ 生成元）。
   - **A₃ 恒仓**：四条原文逐字命中——chan99/0047（26 课）「绝对不加仓，一开始就买够」+「级别只和买卖量有关」；blog/067:254/304/312（67 课）「绝对不增仓…现在还买是脑子水太多」。回调买点叠多头=加仓=明文禁止。
2. **吃到每一笔已由现架构满足**：T₂₄/T₂₇/T₁₄ 时序翻转（`:556-620`，编排者 2026-06-15 裁决「多空对称=时序分段交替」）。方向 B 的动机已被满足 ⟹ 多树非必需。
3. **「不同级别独立 root」无原文依据且反被原文否定**：第 26 课「级别只和买卖量有关」——级别只决定每次操作的「量」，不产生独立持仓树；降成本是同一批 N 单位流转（T₃₄/T₃₉），非多棵独立 N-base 树。

### 棱镜修正（框架，不改裁决）
- **necessity 棱镜**：「三重独立拒」是修辞虚高，实为 **2 支硬支柱（N1/T₄₂ + A₃）+ 1 支稻草人**。步骤1（基点无关性=π₁ 同构）经 grep 全 docs/ 语料零命中（「基点无关/basepoint/π₁/多基点」），covering doc §6 只讲「当下点唯一」，从未论证 π₁ 基点无关性——此步是工位自造对手，**非载重**。rejected 仅靠 N1/T₄₂ + A₃ 即成立。
- **zero-param 棱镜**：裁决把 param 违反错放在「units 守恒」（被 T₃₉ 部分消解，次要）。**真正的 zero-param 死因**：单根 SUB_SPAWN_FRAC 切分由 T₁₈/T₄₈ σ-不变 + T₅₉ 自相似强制 f=1/λ（零自由度）；而 B 的「每个 buy 新建第二棵 root」**没有任何覆盖空间结构强制「第二棵树在此」**——measure-1 当下点唯一，不存在第二个 measure-1 主体。故 B 必须引入「何时新建第二棵 root」准入判据，此判据无 L0 螺旋锚点 ⟹ regime/标的特定经验阈值（违 C2）。
- **chanlun-authority 棱镜**：被原文**过度确定**（over-determined）。第 33 课多义性（多树倡导者最可能援引的反方文本）逐字核验：是同一段走势的多重**释义/读法**，不是多个**同时持仓**；且第 33 课「真正的路只有一条」+ T₅₂ 把多读法塌缩为单一区间套规范固定 ⟹ **第 33 课独立否证方向 B**。区间套（第 27/28/30 课）=单个转折点逐级精确化→唯一的一笔，非多入场叠加。
- **behavior-falsify 棱镜**：T₄₉ 实装（`docs:1368-1369`）confirm fire 62→377（合法必然性改进，8 prove 零 panic），但「回测 P1 未升…fire 升但 P1 崩」——连合法必然性改进都抬不动天花板 ⟹ 天花板是 regime 读数。历史加仓行为已实测=bug（`project_costreduction_moneyprinter_bug`），非 alpha。

### 是否经验参数 / 代码影响
**非零参数。** 突破天花板必须引入「何时新建第二棵树/每棵分多少 units」叠加准入判据=regime/标的特定经验参数（违 C2）。**若误采纳须拆除 N1/T₄₂/N8 守卫，是反向工程，不应触碰任何 .rs。**

### 缠论原文权威（C4）
**直接冲突（C4 拒）。** 第 26 课「绝对不加仓」+ 第 67 课「绝对不增仓…脑子水太多」明文。这正是缠师反复斥责的「最坏习惯…股票不断上涨就不断加仓」。

---

## 3. 方向 C：ε 翻转的 r 约束（势能门槛）

**最终裁决：rejected｜L0｜validity_domain_reading**

chanlun-authority 棱镜（high confidence）一致拒；逐字直读 084/086/026/030 一级权威博文，C4 链经得起对抗且被原文主动否决。

### 必然性依据（推论编号）
- **T₅₇**（手性-角向反演 τhτ⁻¹=h⁻¹，`necessity_derivation.md:2209-2223`）：做空走势 = 做多走势的角向镜像；τ 是 R₂ 对合（等距同构）。推论(c)：「做空引擎不需独立形态学——空头 BSP = 多头 BSP 的镜像」。镜像把买点流映为卖点流，但**级别坐标 r 在 τ 下不变**（τ 只翻手性 ε 与绕向 h↦h⁻¹，不平移 r）⟹ 空头门槛 = 多头门槛 = 成本门 T₁₉（方向无关）。**镜像对称禁止非对称 r 门槛。**
- **T₂₅**（多空嵌套）：父多→子空→孙多方向沿级别交替（L0 必然，`spiral_exhaustive_enumeration.md:171`）。子空在 k−1 内圈（**更低 r**）涌现，与「空头需更高 r」相反；用 r 门槛/root-ε 抑制子空 = 否定 T₂₅。
- **T₁₈**：操作量 m=p_units×θ（力量分量），方向 τ 同胚不进入 ⟹ 成本门 T₁₉ 是唯一门槛，方向无关 ⟹ 无独立空头门槛。
- **T₃₆**（earning 非对称定位）：空头 earning L0 不可构造（莫比乌斯单值障碍 P4）在 {ε}×ℝ₊ 载体，**不传导到 {r} 级别塔**；earning 非对称 ≠ spawn 门槛非对称。
- **T₄₁**：否定线（破极值，结构 φ-信号）处理空头风险有界（任意 r 同构），不需 r 门槛。
- **T₃₀**：root_emergent_ladder 多头爬Up/空头爬Down（τστ⁻¹=σ⁻¹）是结构信号；但用它门控子空违 T₂₅。

### 推导链
1. T₅₇ τ 等距对合 ⟹ 镜像把 r 保持不变（顶↔底，买↔卖，但级别不变）⟹ 空头门槛=多头门槛。
2. **C3 穷尽性**：D∞ 的 {r,ε} 细胞（`spiral_exhaustive_enumeration.md:168-174`）逐格逐字核验仅含 T₂₅（级别交替）+ T₄₀（M=N）+ T₅₁（莫比乌斯丛）+ T₃₀（内嵌 rε），**无「非对称 r-floor」不变量**；N=54+4=58 双路径核对无空槽 ⟹ 引入非对称门槛 = 新增 D∞ 关系（违穷尽性）。
3. **C1 regime**：子空头全亏是 τ-轨道在 ambient 价格的 regime 读数（539 号有效域），非必然性违反——必然性（T₂₅ 父多子空，T₅₇ 镜像对称）正确，亏损是强牛 regime（价格⊥螺旋，C2）。
4. **C2**：任何「空头需 r≥k* 而多头不需」必引入经验阈值 k*（标的特定/regime 依赖）⟹ 零参数不可达。「势∝r」对多空同样赋 r，τ 镜像下 r 不变。

### 是否经验参数 / 代码影响
**非零参数（被拒结果）。** 非对称 r 门槛必引入经验阈值 k*。**若误采纳会改 try_spawn_cost_gated 加 r 门槛/root-ε 门控**，但 (a) 违 T₂₅，(b) 违 C2，(c) 等价于 `constitutive_throughput_falsified`（settle 门/A′/R6′ 三形态 L3 全否证 < v4）。正确处理：保持成本门 T₁₉ 纯方向无关终止（QQQ/DX 因 cost_rejects[2]全拒退化纯 root 持有 PASS，是成本门的零参数自然结果）。

### 边界条件
1. 若证明莫比乌斯丛实为平凡丛（圆柱）⟹ τστ⁻¹=σ，多空完全对称——但 asymmetry doc 已用 ℝ₊ 载体（P2 空头朝 c→∞ 无界）否证平凡丛，此路封闭。
2. 若 {r,ε} 细胞存在第三条独立不变量 ⟹ 非对称 r 门槛成合法槽位——但 N=58 已双路径核对（54+4），{r,ε}=2 满，无空槽。
3. 若缠师原文出现「做空必须更高级别确认」字面定义 ⟹ C4 翻转——当前 84/86 课仅有风险/载体不对称（P2），无级别门槛。

### 缠论原文权威（C4）
**被原文主动否决（强于「相容」）。** 84:316 逐字「任何的空头头寸最终都是死路一条」——做空非对称定位在载体风险/逼空维（{ε}×ℝ₊），非级别 r 门槛。84:36/86:31/86:33 理论层多空对称 + 实操层载体非对称（风险成倍/逼空死路），无一处级别门槛。26:34「无论先买后卖与先卖后买，效果是一样的」（多空操作对称）+「级别…基本只和买卖量有关」（级别≡买卖量，不绑方向门槛）。**缠师从未表述「做空需要更高级别/更大势能门槛」。**

---

## 4. 方向 D：确认滞后的补偿（分批/预期建仓）

**最终裁决：rejected｜L0｜validity_domain_reading**

四棱镜（necessity / zero-param / chanlun-authority / behavior-falsify 全运行，high confidence）一致拒。

### 必然性依据（推论编号）
- **A₃**（恒仓公理，C4 一级权威）：第 26 课「绝对不加仓，一开始就买够」直接否决分批建仓（逐步堆到 N = 在前若干买点持仓<N、用后续买点继续买高 = 加仓）。`necessity_derivation.md:53-54`。
- **T₄₉**（confirm 向心回溯 = 前向几何错误）：confirm 一个外圈 candidate@K 需检验内圈 type1，由展开恒等式 Δt∝λ^{K-1}(λ−1)>0，内圈 type1 必然在过去（`covering_space §7.3` 逐字裁定读法A「几何错误」；ES 1s 418/418=100% type1 在过去）。**预期建仓 = 前向 confirm = 几何错误。**
- **T₂₈**：从最高级别进入捕获最大势=晚但大，必然选最高级别（最晚）。否决「降级到低级别求更早入场」。
- **T₂₉**（先势后定位）：compress_bar≤confirm_bar≤bar，时序不可颠倒。「不等 confirm 就入场」= 先定位后有势，违此时序。
- **T₃₈**（双向重定基，`:800-816`）：「恒仓」=不主动加仓，N_base 在 φ=0 双向重定基（earning N+Δ / shortfall N−δ）——故严格禁止条款是 **A₃∧T₃₈ 的合取**（一次性买够 N ∧ 持仓中不追买），非单条 A₃。
- **T₃₀**：根级别涌现=会计重组（A₃ 禁加仓 ⟹「更晚但大」通过 root_emergent_ladder 上移已实现，不需物理加仓）。

### 推导链
1. **A₃ 腿**：分批建仓（堆到 N）= 加仓，被「绝对不加仓，一开始就买够」字面禁止。
2. **T₄₉ 腿**：预期建仓 = 读法A（前向）= 几何错误（type1 在内圈过去，前向够不着，实测 0% 在未来）。
3. **无必然性兼容的更早入场**：T₂₈ 强制最高级别（晚但大）；唯一「更早」机制是降级（早但小），但违 T₂₈。第 30 课 line150「买点都是在跌的时候形成的…如果第一买点没买，就等第二、三」+ 知识库§11.5「小转大…等待第二类买卖点确认」——小转大要求**更晚**（确认型操作点），非抢跑。第 26 课 line36「一般都是二类或三类买点」=刻意比一买更晚骗庄给货。

### 棱镜修正（不改裁决）
- **necessity 棱镜**：两处归因松动均由裁决**已引用**的推论补救——(1) 分批禁止精确形式是 A₃∧T₃₈ 非裸 A₃；(2)「预期建仓」两读，真·前向 confirm 由 T₄₉ 禁，「无 confirm 入场」由 T₂₉（先势后定位）禁。无新公理/新生成元。
- **zero-param 棱镜**：epistemic_level 应从裸 L0 精化为「L0 定性腿（A₅-独立）+ 螺旋公理条件性」——T₄₉ 腿条件于「走势-级别图是对数螺旋」建模公理（λ 恒定性是 L2 可证伪点，`covering_space §7.4/§1.3`）；但 A₃ 腿**无条件 L0**（独立禁分批）⟹ 双腿至少一腿无条件成立 ⟹ rejected 鲁棒。
- **behavior-falsify 棱镜（最强补强）**：天花板 BH-gap 主因**不是晚入场**。`analysis §2.1` 坐实 root long 腿本身≈BH（QQQ r=1.01/DX r=1.46 即上限），gap 主体是 segment 子空头失血（BTC −206757 几乎抵消 root +211973）+ units 稀释。⟹ **方向 D 瞄准的器官（入场时机）根本不是 BH-gap 产生处**——即便更早入场也无法系统超越 BH（强牛 BH 本身是天花板，gap 在下游 E/稀释）。这比 A₃ 的 L0 否决多一层 C1 经验否证。
- **chanlun-authority 棱镜（对抗加固）**：第 26 课 line80/line447 的「1/3 或 1/4」是**短差/降成本操作的量**（E spawn 降成本域），不是建仓分批。全文阅读后 C4 坐实「一次性买够 N」无例外。

### 是否经验参数 / 代码影响
**裁决本身零参数**（纯由 A₃+T₂₈+T₄₉ 推出）；方向 D**若实装**才引入批次数/每批比例/预期提前量经验参数（违 C2）。**无代码改动**——F 根入场（`unified_necessity.rs:713`，向心 confirm 后满仓开多 N）+ T14 D 回补（买点恢复到 N 封顶）已正确实装 A₃+T₂₈+T₄₉。若误采纳须把 helix_centripetal_confirm 改回前向窗口（违 T₄₉，重现 `project_recl2_confirm_breakpoint_temporal` 418/418 时序错配 + 失控强平）。

### 缠论原文权威（C4）
**直接冲突（C4 否决）。** 第 26 课「绝对不加仓，一开始就买够」+「买点的时候又回复原来的数量」（封顶=N）。第 30/29 课 + 知识库§11.5 全部指向「等待确认」（更晚）而非「预期/分批抢跑」（更早）。无任何与方向 D 相容的原文。

---

## 5. 方向 E：遗漏推论扫描（直接指向问题的推论）

**最终裁决：negative / escalate（找到直接相关推论 T₄₁/T₄₆，但被 L3 否证；"修复"被 T₄₁ ⊥ 第11环 矛盾阻断，须 /escalate）｜L3｜necessity_violation（声明层）**

> **工位说明（诚实）**：本方向的专职推导 agent 在工作流中因 API 500 中断（`derive:E failed`），本节由 Lead 直接从源码（`unified_necessity.rs:58-103`）+ 8 标的实测 `exit_reasons` 核验补全。结论与综合工位从数据独立推出的 T₄₁ 矛盾（§6.4）**收敛**——本节给出**机制**（子空头/D-回补/否定线删除路径），综合工位给出**文档内分类冲突**（Phase-1 vs Phase-4）。

### 必然性依据（推论编号）
- **T₄₁ 零强平 + T₄₆ 零破产**：这是 58 条里**直接指向"子空头全亏"问题**的推论。声称无条件 L0（否定线先于保证金线 / A₀+T49 ⟹ 空头必被 C 在买点消费）。
- **A₀（走势终完美）**：T₄₁ 证明的根——"空头押注的下跌走势必然完美 ⟹ 买点必现 ⟹ 消费空头"。
- **T₄₃（N5⊥N7）**：E 降成本用 voice **自层 nf fire**，**不查势源链**——E 可在尚未确认为势源的 candidate 上 spawn 子空头。
- **T₁₅（破极值=走势否定）+ 第11环（操作只在买卖点）**：027:25 否定线作为**操作触发器**已被编排者删除（`unified_necessity.rs:58-103`，2026-06-15"否定线是经验性补丁，违反操作只在买卖点，直接删除"），保留仅在 candidate 检测侧（势源有效性，区间套确认）。
- **T₄₄（递归深度有限）**：depth≤root.ladder−bi——**只约束深度，不约束广度**；`try_spawn_cost_gated` 无广度上限（max_children 只被观测不被约束），配额 f=1/λ 只让每次 spawn 几何收缩，不限同时活跃子数。
- **T₅₅-b（手性独立）**：标的强于大盘 ε_R=+ 才介入——但属 M2 选股层（T₅₅-e 层分离，不接入 M1 持仓）。

### 推导链：T₄₁ 零强平被 161 次实测强平直接否证

**8 标的实测出场原因（`trading_system/data_cache/unn_stream_*.json:exit_reasons`，当前 worktree 快照）：**

| 标的 | liq（A 强平） | recover（D 回补） | cascade | flip | max_children | P1 |
|------|------|------|------|------|------|------|
| **BTC** | **89** | 26 | 0 | 0 | 80 | fail r=0.10 |
| OKLO | 36 | 0 | 1 | 0 | 36 | fail |
| ES | 22 | 2 | 0 | 2+2 | 22 | fail |
| BRN | 14 | 11 | 0 | 2+2 | 15 | fail |
| CL | **0** | 4 | 10 | 0 | 11 | fail |
| GC | **0** | 8 | 0 | 1+1 | 2 | fail r=0.88 |
| QQQ | **0** | 3 | 0 | 0 | 1 | **PASS** |
| DX | **0** | 2 | 0 | 0 | 1 | **PASS** |

1. **T₄₁ 声明零强平，实测 161 次强平**（BTC89+OKLO36+ES22+BRN14）⟹ T₄₁"无条件 L0"被 L3 直接否证。
2. **机制（T₄₁"C 必先消费空头"为何不覆盖子空头）**：T₄₁ 的"C 在买点 in-place 翻多消费空头"是 **root 空头**路径（C 规则在源层 type1 买点翻转）。但**子空头是 E-spawn（N7 自层 nf，T₄₃）**，其出场是 **D 回补**（在子自层=segment 的 type1 买点）或 **A 强平**——**不经 C/root 向心 confirm**（T49 修的是 root confirm，不覆盖子空头）。
3. **强牛中子空头无买卖点出场**：子空头押注的 segment 下跌走势，在强牛中是随后上涨**否定（破极值）**的回调——它从未完美为真势源（A₀ 只对真·走势保证"完美⟹买点"，对被否定的 candidate 不保证）。⟹ segment 底背驰 type1 买点不来 ⟹ D 回补不触发 ⟹ 子空头持有到 capital 耗尽 ⟹ A 强平。
4. **否定线删除是放大器**：027:25 否定线（破极值即关闭子空头，bounded shortfall）作为操作触发器被删（第11环）⟹ 子空头被否定时**没有 bounded-loss 出场**，只能等 liq。`exit_reasons` 坐实：BTC 89 子空头里 89 liq（77% 出场）vs 仅 26 recover。
5. **regime 分布坐实条件性**：liq 集中在强牛/spawn_seg>0 标的（BTC/OKLO/ES/BRN）；震荡标的 CL/QQQ/GC/DX 零 liq（子空头经 recover/cascade 干净出场）⟹ 强平**条件于 regime**（空头被推入逆向趋势），非无条件不可达。

### 两层结构（关键诚实区分）
- **基线层（有效域，C1，不修）**：子空头在上行 regime 净负——**即使全部干净出场也亏**。GC 0 liq、8 全 recover，仍 r=0.88（子空头净负把 root 多头拖到 BH 之下）。这是 T₂₅（父多必子空）撞强牛 regime（539 号），必然性正确，不修。
- **放大层（必然性违反，须 escalate）**：当子空头无买卖点出场（强牛）时 liq，把基线的"温和净负"放大为"灾难"（BTC −206757 vs GC 温和 r=0.88）。这一层**违反 T₄₁ 零强平声明**——是声明层的必然性违反，不是有效域读数。

### 是否经验参数
**恢复否定线作为子空头 bounded 出场是零参数**（破极值=该 candidate 定义的结构极值，非经验阈值，T₁₅）。**但它不是"严格解"**——因为它**与第11环（操作只在买卖点）矛盾**：否定（破极值）不是买卖点，用它触发 voice 操作正是编排者 2026-06-15 删除的"经验性补丁"。⟹ **恢复否定线 ⊥ 第11环**，是真矛盾，不可自决。

### T₄₁ 矛盾的精确形式（须 /escalate）
> **T₄₁（零强平，要求子空头在 liq 前出场）⊥ {第11环（操作只在买卖点，否定线已删）+ T₄₃（N7 让 E 在否定-prone 自层 candidate 上 spawn 子空头）}**——在强牛 regime，三者联合不可满足（子空头无买卖点出场 → liq）。

这与综合工位独立发现的**文档内部矛盾**互补：`necessity_derivation.md` 自身 Phase-1（`:1226`，T₄₁=条件 regime 残余 P10'）与 Phase-4（`:1259-1262`，T₄₁=无条件 L0）对 T₄₁ 的分类自相矛盾。本方向给出**为何**条件成立的机制（子空头/D-回补/否定线删除路径），综合工位给出**文档内**的分类冲突。二者收敛：**T₄₁ 是条件定理（regime），不是无条件 L0。**

### 代码影响
**无改动（须先 /escalate 解矛盾）**。可能的解（待编排者裁决，均涉及定义层，不可自决）：
1. **恢复否定线作子空头 bounded 出场**——解 T₄₁ 违反，但违第11环（须重裁"否定是否算操作触发器"）。
2. **E spawn 前要求子空头 candidate 经向心 confirm 升为真势源（≥PENDING_LO）**——解 T₄₃ 的"自层 spawn 否定-prone candidate"，但违 N7/T₂₀（降成本用自层不查链）= 退回 pcf 引擎踏空死因。
3. **接受 T₄₁ 为条件定理（regime）**——把 §11 审计表 Phase-1 与汇总 Phase-4 统一为"条件 regime"，清除声明膨胀，但放弃"零强平无条件 L0"。

### 缠论原文权威（C4）
否定线源自缠师第 27 课否定线——它**是**缠论概念（破极值=走势否定）。编排者删它的依据是"操作只在买卖点"（第 11 课买卖点完备性）。**两条缠论原文（第27课否定线 vs 第11课买卖点完备）在子空头出场上张力**——这正是须 escalate 的语法记录：子空头的合法出场是"买点（D 回补）"还是"否定（破极值）"？缠师第 27 课"否定线"是否构成对**子空头**的操作触发器，原文未显式裁定（编纂版遗漏，须回溯博文，C4）。

---

## 6. 总裁决：三问题的必然性 vs 有效域诊断

**严格解决方案数 = 0。** 四个方向无一突破三问题——因为三问题**主要不是可修复的实装 gap，而是必然性后果 / 有效域读数（C1）**。

### 6.1 天花板（root ≤ BH）= 有效域读数（C1，必然性正确不修）
- **必然性根**：A₃（一次性 N，无多笔叠加）∧ T₂₈（最高级别入场=最晚）∧ T₄₉（向心回溯滞后 Δt∝λ^{K-1}(λ-1)>0 不可消除）。
- **诊断**：BH-underperformance 是强牛 regime（非盈利 regime）的有效域边界。`analysis §2.1` L3 坐实 root long 腿≈BH（QQQ r=1.01/DX r=1.46 即上限），且 T₄₉ confirm fire 62→377 的合法必然性改进抬不动 P1——**连必然性驱动的改进都不抬天花板 ⟹ 天花板是 regime 读数非缺陷**。
- **方向 B（多树）/方向 D（更早入场）都瞄准此问题，都被拒**：B 要求加仓（违 A₃），D 瞄准的器官（入场时机）根本不是 gap 产生处（gap 在 E/稀释）。

### 6.2 子空头全亏（BTC 89 笔 wins=0）= 有效域读数（C1，必然性正确不修）
- **必然性根**：T₂₅（父多必子空，L0）的方向撞上 ambient 价格单边上行（价格⊥螺旋，C2）。
- **诊断**：539 号已结算「子空头/翻空有效域 ⊂ 非上行 regime」。L3 判别量 = root 涌现到的 ambient 价格方向（强牛 vs 震荡/下行）：油链 CL/BRN 震荡 regime 子空头/翻空净赚（BRN move/short +7938，26 胜）。
- **方向 A（r 下界）/方向 C（ε 翻转 r 门槛）都瞄准此问题，都被拒/否定**：A 证明零参数 r_min 在成本门层不可能（C2），C 证明非对称 r 门槛违 T₂₅/T₅₇/C3。

### 6.3 units 稀释（max_children=80）= 必然性后果（C1，必然性正确不修）
- **必然性根**：T₃₄（Σunits=N_base 守恒）+ T₁₈（spawn 配额 f=1/λ，σ-不变 L0，`prove_theta_sigma_invariant` 守卫）。
- **诊断**：主升浪暴露缩水是守恒律 + spawn 机制的**派生量**，非缺陷。稀释深度 = max_children（BTC 80 > OKLO 36）= 振幅恒过成本门的标的中 spawn 数（C2 ambient 量）。

### 6.4 唯一必然性违反候选（高亮）：T₄₁ 零强平 vs 161 次实测强平
**这是唯一不能归为「有效域读数」的项，须单独 /escalate。**

- **声明（Phase-4，编排者 2026-06-15，`necessity_derivation.md:1259-1262`）**：T₄₁ 零强平 = **无条件 L0**——「A₀ 走势终完美 ⟹ 空头押注的下跌走势必然完美 ⟹ 买点必现 ⟹ C 在买点 in-place 翻多消费空头 ⟹ 空头必然先在买点被 C 消费，不依赖 regime；历史强平是前向 confirm 时序 bug（已由 T49 修复）的产物，非 regime 必然」。
- **经验（L3，8 标的 `unn_stream_*.json:exit_reasons.liq`）**：BTC liq=89 / OKLO liq=36 / ES liq=22 / BRN liq=14，**共 161 次实际强平（当前 worktree 快照，非 pre-T49-fix 陈旧数据）**。`.chanlun/escalations/2026-06-16-confirmed-root-raw-asymmetry.md` 进一步记录 D-fix 后 BTC liq **0→216**（config 依赖）。
- **内部矛盾**：同一文档 §11 审计表（Phase-1，`:1226`）把 T₄₁ 标为「**永久拓扑残余（条件定理）…条件于 MAM_est≥MAM_actual（regime）（asymmetry P10'）…强平兜底 A 不可无条件消除**」。Phase-1（条件 regime）与 Phase-4（无条件 L0）**不可同时成立**。
- **裁决方向**：8 标的 liq 计数的 **regime 分布**（强牛/spawn_seg>0 标的 BTC/OKLO/ES/BRN 有 liq；震荡标的 CL/QQQ/GC/DX 零 liq）**经验支持 Phase-1（条件 regime/P10'）**——强平当且仅当空头被推入逆向趋势（强牛），正是 `条件于 MAM_est≥MAM_actual`。**Phase-4 的「无条件零强平 L0」声明被 L3 否证**（声明膨胀，090 号；声明了代码不具备的能力——代码 161 次强平 fire）。
- **四分法分类**：**真矛盾（定义冲突）**——修复需改 T₄₁ 的含义/有效域（从「无条件 L0」降回「条件 regime 残余」），属 `no-workaround.md`/`testing-override.md`「修复需改定义含义=定义冲突，停下来上浮」。**不可自决**（已有编排者 Phase-4 裁决，撤销它须编排者价值判断）。
- **注意（C1 边界的诚实区分）**：161 次强平**本身**是 regime 行为（C1 有效域读数，与子空头全亏同根 539 号），**这不是必然性违反**；违反的是 **T₄₁ 的声明等级**——把一个 regime-conditional 的现象声明为 unconditional L0 必然性。即：**现象是有效域读数（C1 正确），声明是膨胀（C1 误用）**。须 /escalate 的是「T₄₁ 应分类为条件定理（Phase-1/P10'）还是无条件 L0（Phase-4）」这条已被编排者裁决但被 L3 数据否证的语法记录。

---

## 7. 结果包六要素

### 1. 结论
五方向（A/B/C/D/E）最终裁决：A=negative（否定性结果）、B/C/D=rejected、E=negative/escalate。**严格解 0 个。** 三问题（天花板/子空头/稀释）主要是必然性后果 / 有效域读数（C1），不被 L3 回测否定，不修引擎。**方向 E 发现 58 条里直接指向"子空头全亏"的推论 T₄₁/T₄₆（零强平/零破产），其"无条件 L0"声明被 161 次实测强平 L3 否证 = 唯一必然性违反候选，须 /escalate**（不是严格解——"修复"被 T₄₁⊥第11环 真矛盾阻断）。

### 2. 定义依据
- 方向 A 否定依据：T₈/T₂₈（唯一零参数结构 r_min=PENDING_LO=move(L1)）+ T₁₉（成本门 θ=振幅 L2 经验量，λ^k 不能判定势消失）+ T₅₆（segment 无角向圈，L3 fire[2]=0）。
- 方向 B 拒绝依据：T₂₇/T₄₂（单根不变量 measure-1 当下点）+ A₃（恒仓，第 26 课）+ T₃₃（多读法塌缩）。
- 方向 C 拒绝依据：T₅₇（τ 等距镜像，r 不变）+ T₂₅（子空在更低 r）+ {r,ε} 细胞穷尽（无非对称 r-floor 槽位，N=58 双路径）。
- 方向 D 拒绝依据：A₃∧T₃₈（恒仓+不追买）+ T₄₉/T₂₉（前向 confirm 几何错误/先势后定位）+ T₂₈（最高级别=最晚）。
- 方向 E / T₄₁ 违反依据：T₄₁/T₄₆（零强平/零破产，直接指向问题）+ T₄₃（N7 E 自层 spawn 否定-prone candidate）+ 第11环否定线删除（`unified_necessity.rs:58-103`）；`necessity_derivation.md:1226`（Phase-1 条件 regime）vs `:1259-1262`（Phase-4 无条件 L0）内部矛盾 + `unn_stream_*.json:exit_reasons.liq`（8 标的 161 实测强平）。

### 3. 边界条件
- 三问题诊断翻转：若 regime 转震荡/下行（油链 CL/BRN 已证）⟹ 子空头/翻空转正、稀释从拖累转贡献——证明三问题是 regime 函数（C1）非必然性违反。
- T₄₁ 裁决翻转：若证明当前 161 次强平全部来自残留实装 bug（如 confirmed_root raw 残余，`escalation 2026-06-16`）且修复后 liq→0 跨所有 regime ⟹ Phase-4「无条件零强平」恢复；若修复后强牛标的仍有 liq ⟹ Phase-1「条件 regime 残余」确立。
- 方向 A→strict 翻转：仅当 PENDING_LO 抬高（改 T₈/T₂₈，C4 风险）或 θ 纯结构化（T₁₉ 封死），两路均不可达。

### 4. 下游推论
- 三问题不修 ⟹ 引擎当前架构（单根 in-place 翻转 + 成本门 + σ-不变配额）是**必然性正确形式**，BH-underperformance 在强牛标的是有效域边界，不是工程债。
- T₄₁ /escalate ⟹ 触发 §11 审计表（Phase-1）与汇总（Phase-4）的一致性修复——二者对 T₄₁/T₄₆ 的分类必须统一（条件定理 vs 无条件 L0），且 `prove_n8`/NAV 守卫的声明等级须与实测 161 强平对齐（声明膨胀清除）。
- 开放轴（独立 L3 验证，非本工位裁决）：E spawn 前 root 涌现层方向门（上行抑制），但引入 regime 判别可能违 C2（除非用纯结构 located 链方向）。

### 5. 谱系引用
- **539 号**：子空头/翻空有效域 ⊂ 非上行 regime（方向 A/C 的 C1 根据）。
- **`project_t14_t5_root_flip_necessity`**：必然性累积不被 L3 否定，L3=有效域读数（同范式）。
- **`project_constitutive_throughput_falsified`**：settle 门/A′/R6′ 三形态 L3 全否证 < v4（方向 C 误采纳的等价前车）。
- **`project_costreduction_moneyprinter_bug`**：加仓暴增=bug 非 alpha（方向 B/D 加仓机制的前车）。
- **`project_recl2_confirm_breakpoint_temporal`**：418/418 前向 confirm 时序错配致失控强平（方向 D 误采纳的等价前车 + T₄₁ 历史强平归因）。
- **`escalation 2026-06-15-theta-allocation-non-necessity`**：成本门角色A 保留经验 θ（方向 A 的 C2 裁决根据）。
- **`escalation 2026-06-16-confirmed-root-raw-asymmetry`**：D-fix 后 BTC liq 0→216（T₄₁ 违反的 L3 证据 + raw 残余）。
- **语法记录候选（须 genealogist 开节点）**：T₄₁ 「无条件 L0」vs「条件 regime 残余」的分类——Phase-1/Phase-4 矛盾 + L3 否证。信号层 segment/move 双重性（`project_signal_layer_duality`）已标待结晶。

### 6. 影响声明
- **改动**：新建 `analysis/spiral_solution_to_underperformance.md`（本报告）。无引擎 .rs 改动（五方向全拒/否定/escalate，C1 不修必然性；方向 E 的 T₄₁ 修复须先解矛盾再裁决）。
- **影响模块/定义**：触发 T₄₁/T₄₆ 分类一致性 /escalate（`necessity_derivation.md §11` 审计表 vs 汇总）；标注 `SUB_FRICTION_RT/SUB_COST_K/SUB_COST_Q/SUB_COST_MIN_OBS` 四常数的经验依赖（方向 A 的 C2 完整论证，未改其值）。
- **未改动**：`unified_necessity.rs`/`depth_ref.rs`/`positional_fusion.rs`/`config.rs` 全部保持现状。

---

## 附：认识论等级总表

| 结论 | 等级 | 信息增量 | 理由 |
|------|------|---------|------|
| 方向 A 零参数 r_min 在成本门层不可能 | L0（推导）+ L3（8 标的 cost_rej 分布坐实） | 正（否定性，缩小有效域） | C2 必然违反 + T₁₉ 封死 |
| 方向 B 多树违 N1/T₄₂/A₃ | L0 | 零（同义反复，从已结算不变量推出） | C3 反向工程 + C4 加仓禁令 |
| 方向 C 非对称 r 门槛违 T₅₇/T₂₅/C3 | L0 | 零（{r,ε} 细胞穷尽枚举推出） | C3 无空槽 + C4 无级别门槛 |
| 方向 D 分批/预期违 A₃∧T₃₈/T₄₉ | L0 | 零（从公理推出） | C4 恒仓 + T₄₉ 几何错误 |
| 方向 E 找到 T₄₁/T₄₆ 但被实测强平否证 | L3（8 标的 exit_reasons.liq） | 高（否定性 + 矛盾揭示） | T₄₁⊥第11环机制 + Phase-1/4 分类冲突 |
| 三问题=有效域读数（C1） | L3（8 标的 liq/regime 分布） | 高（否证鲁棒性） | 强牛 vs 震荡 regime 二分 |
| **T₄₁ 无条件零强平被否证** | **L3（161 实测强平）** | **高（否定性，否证无条件声明）** | **Phase-1/Phase-4 矛盾 + 声明膨胀** |
