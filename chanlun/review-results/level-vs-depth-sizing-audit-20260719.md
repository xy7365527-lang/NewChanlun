# 级别 vs 深度：仓位比例语义审计（60/30/10 是 depth 权重，不是 level 权重）

- 日期：2026-07-19
- 数据：`/tmp/m8_opsem_fixed/trades.jsonl`（518 笔）
- 代码基线：worktree `/tmp/kimi-nest-mainline`（分支 kimi-nest-mainline-20260717），只读审计，未改任何代码

## 0. 裁定（TL;DR）

**不是一回事。`depth_weights=[0.60,0.30,0.10]` 是赋格嵌套深度（parent 链长度）的资金帽权重，不是塔级 L1/L2/L3 的级别权重；两者语义独立，实测证伪任何"depth≈level"的同构假设。**

- 根声部（depth=0）可以在**任何级别**开仓——trades.jsonl 中 L0/L1/L2/L3 四个级别均存在 depth=0 的根，且 L3 的根与 L0 的根拿同一份 w=0.60 资金帽。
- 实测名义敞口按级别分布为 L0 77.95% / L1 10.94% / L2 1.62% / L3 9.49%——与 60/30/10 **无任何对应关系**。60/30/10 约束的是同一嵌套树内根/子/孙的资金配比，不约束级别分布。
- 现行机制**只有 depth-based sizing + 全局总量帽**，没有任何 level-based 的独立仓位上限（聚合层）。这是设计缺口（§4）。

## 1. 语义区分（代码锚）

### 1.1 depth = 嵌套树 parent 链长度（真父子，非级别差）

- `coverage.rs:1557-1567` `element_depth`：「元素的真嵌套深度（沿 parent 链长度，根=0；**铁律：真父子，非级别差**）」——depth 由 parent 索引链计数，与 level 差无关。
- `coverage.rs:1428-1429`（leg_target doc）：「`depth` = 元素在嵌套树的深度（根=0，沿 parent 链长度）——**真嵌套深度**（铁律：来自 parent 链，非级别差）」。

### 1.2 depth_weights 的定义与消费链

- 定义：`config.rs:135-137`（`max_depth`/`depth_weights` 字段），默认值 `config.rs:155-156`（`max_depth: 3`，`depth_weights: [0.60,0.30,0.10]`）；规格锚 `reference-theta-v0.md:42`：「资金帽：深度权重 `w=[0.60,0.30,0.10]`，未用部分保留现金不重分配。[设计选择,默认值]」——注意标注是 **[设计选择]**，非缠论教义可导。
- 取权重：`voice.rs:180-194` `depth_weight(depth, config)`——按 depth 索引查表，越界返 0.0（未用部分留现金，不重分配）。
- 消费点①（plan_orders sizing）：`mod.rs:371` `w_depth: voice::depth_weight(d.depth, &config.voice)` 注入 `SizingInput`；`risk.rs:121`（字段语义「声部深度资金权重」）、`risk.rs:148-175`/`risk.rs:207-210` sizing 项2 名义上限 `floor(w_depth·γ·NAV/(entry·tick_size))`。
- 消费点②（coverage 腿目标）：`coverage.rs:1424-1428`（doc：单位数 `s_e = base_units × w_depth × w_dir`，`w_depth` 对齐 `voice::depth_weight`）；`coverage.rs:1524`（leg_target）与 `coverage.rs:1548`（leg_target_two_segment）：`w = depth_weight(depth, config) * dir_weight(...) * w_grade(...)`。
- 测试锚：`coverage.rs:3681-3699` `leg_target_side_and_depth_weighted_units`——根 depth 0 → units=1000×0.60=600，子 depth 1 → 1000×0.30=300，逐值钉死深度索引语义。
- 边界：`voice.rs:196-202` `within_max_depth`（depth < max_depth 才开声部）；`mod.rs:261-264` 超限深度跳过。

### 1.3 level 的独立通道（不进 w_depth）

- `level` 字段：`mod.rs:176`（`VoiceDecision.level`）；trades 记录中 `certificate.level`。
- level 进 sizing 的**唯一**现存通道：`mod.rs:360-361` `config.sizing_profile.resolve(d.level, side_key, &config.risk)` 按 (level, side) 取 (ρ, Γ, gap_buffer) override——这是 PDF《全互斥定义策略》§3 的 ρ_{ℓ,δ,r}/Γ_{ℓ,δ,r} **状态函数**（`risk.rs:133-134`），属**单笔标量 override**，不是级别聚合上限。默认空 profile（`config.rs:367-368` 冻结断言 `entries.is_empty()`）⟹ 全部退化为 risk 标量（`config.rs:285`），冻结 v0 下 level 对 sizing **零影响**。
- level 的其它用途：冲突排序键（`mod.rs:294/309`，`reference-theta-v0.md:54`「高 level 先于低 level」）——只定订单次序，不动仓位。

### 1.4 根声部可以在任何级别开仓

- `voice.rs:5`：「根 = 当前最高有效决策级别 L*；最多 `config.voice.max_depth` 层（L*, L*-1, L*-2）」（对齐 `reference-theta-v0.md:40`）。L* 是**运行时的**最高有效级别——不同 bar 的 L* 不同，L1 信号触发时 L*=L1，此时 depth=0 的根就是 L1 声部。
- `voice.rs:93-95`（★诚实）：「v0 recognize 产**单声部决策**（depth 0，独立根）——每个买卖点是 §5 的独立根」；coverage 路径（多根嵌套）同样按 parent 链定 depth。根方向 `root_side` 由信号定（`voice.rs:74-77`）。

## 2. 实测分布（trades.jsonl，518 笔）

口径：`units`（开仓腿目标单位数，= base_units × w_depth × w_dir × w_grade，runner.rs:2237/2359 注明可核验）；级别取 `certificate.level`，深度取 `voice_tree_at_entry.depth`。一致性核验：**depth == nest_depth 518/518 全等**；**certificate.level == voice_id.level 518/518 全等**；w_depth 与权重表 [0.60,0.30,0.10] **0 失配**（含 depth≥3 越界返 0 的 64 笔，全部 units=0，不持仓）。

### 2.1 名义敞口按级别分布（与 60/30/10 无关）

| certificate.level | units 合计 | 名义占比 | 笔数 | 平均 units | 最大 units |
|---|---|---|---|---|---|
| L0 | 64501.8 | **77.95%** | 434 | 148.6 | 652.4 |
| L1 | 9052.2 | 10.94% | 54 | 167.6 | 652.0 |
| L2 | 1339.0 | 1.62% | 7 | 191.3 | 251.7 |
| L3 | 7855.9 | 9.49% | 23 | 341.6 | 506.7 |
| 合计 | 82748.9 | 100% | 518 | | |

级别分布完全由**信号出现频率**决定（L0 信号多 ⟹ L0 敞口占 78%），机制上没有任何力把它推向某个级别配比。

### 2.2 (level, depth) 交叉分布——depth ≠ level 的直接证据

| level | depth=0 | depth=1 | depth=2 | depth≥3 |
|---|---|---|---|---|
| L0 | 46554.5 (56.26%, n=177) | 14094.2 (17.03%, n=106) | 3853.1 (4.66%, n=92) | 0.0 (n=59) |
| L1 | 5132.7 (6.20%, n=13) | 3608.1 (4.36%, n=28) | 311.4 (0.38%, n=8) | 0.0 (n=5) |
| L2 | 305.1 (0.37%, n=2) | 1002.7 (1.21%, n=4) | 31.2 (0.04%, n=1) | — |
| L3 | **7171.7 (8.67%, n=15)** | 684.2 (0.83%, n=8) | — | — |

关键行：**L3 depth=0 共 15 笔、7171.7 units（占全样本 8.67%）**——最高塔级 L3 的根声部拿的是 depth=0 的 0.60 资金帽，与 L0 根同权。若 depth 与 level 是同一件事，L3 行 depth=0 列应为空；实测相反。depth 与 level 是两个独立坐标：(level, depth) 交叉表非对角。

## 3. 机制设计对照：现存通道里有没有 level-based sizing？

| 通道 | 位置 | 是 level-based？ | 是聚合上限？ |
|---|---|---|---|
| w_depth（depth_weights） | `voice.rs:188-194` → `mod.rs:371` / `coverage.rs:1524` | 否（depth 索引） | 否（单腿权重） |
| ρ_{ℓ,δ,r}/Γ_{ℓ,δ,r} sizing_profile | `config.rs:255-285`（`SizingEntry{level,side,rho,gamma,gap_buffer}`）→ `mod.rs:360-361` | **是**（按 (level,side) override） | 否（**单笔**标量：ρ 进风险预算项、Γ 进该笔名义上限项） |
| 全局名义/毛帽 γ̄·U_ℓ | `risk.rs:560-573`；`coverage.rs:1679` `apply_gross_cap`；毛闸门 `runner.rs:3306-3316` | 否（全账户一个标量） | 是（全局，不分级） |
| U_ℓ 协变资本单位 | `risk.rs:111`（「方案A协变：nav 在 runner 层即是 U_ℓ（按级别缩放注入）」） | **名义上是** | — |
| U_ℓ 实装 | `runner.rs:442`/`runner.rs:1279` `base_units = equity_nav / px` | **否**——实装为 NAV/px 单一标量，**无 a_k 级别缩放因子**；「按级别缩放」是设计意图注释（`coverage.rs:2480`/`coverage.rs:2645`），实装未接 | — |

结论：**设计里只有 depth-based sizing + 单笔 (level,side) override + 全局总量帽；没有 level-based 的独立聚合仓位上限。** 设计文档（`reference-theta-v0.md:39-47`）的资金帽条款也只定义了深度权重，未定义级别权重或级别上限；60/30/10 的标注是 [设计选择,默认值]，无教义锚（博文/编纂版均无此配比，属 Θ 设计层自由参数）。

## 4. 设计缺口（照实否定）

1. **级别无独立仓位上限。** 各级别敞口占比是信号频率的被动结果（§2.1：L0 78% / L3 9.5%），没有任何机制阻止"某级别独大"或保证"高级别配重"。若机制设计的意图是塔级越高仓位越重（或反之、或任何显式级别配比），现行代码**不实现该意图**。
2. **depth 权重被误读为级别权重的风险。** 二者名字相邻（spec:40「最多 3 层 L*,L*-1,L*-2」把 depth 层写成级别记号 L*,L*-1,L*-2——这是**同一棵树内**根的级别与其子级别的相对记号，不是塔级 L1/L2/L3 绝对坐标），但 spec:40 的写法极易被读成"级别权重"。本审计的实测交叉表（§2.2）证伪此读法。
3. **sizing_profile 的 Γ_{ℓ,δ,r} 是假聚合解。** 它能按级别压低**单笔**名义（`config.rs:391-393` 示例 L2 多 Γ=1.2 / L2 空 Γ=0.8），但同级别多笔并发时总敞口无界（除全局 γ̄·U_ℓ）。
4. **U_ℓ 级别协变未接线。** 文档承诺（`risk.rs:111`、`coverage.rs:2645`）"runner 按级别注入 U_{ℓ+k}(S_k x)=a_k U_ℓ(x)"，实装 `runner.rs:1279` 只有一个 base_units=NAV/px。声明=能力：当前能力 = 单一 U_ℓ，级别协变是未实装的设计项。

## 5. 修复设计（级别权重如何进入 sizing；仅设计，不改代码）

三个方案按侵入度升序，均须走 change request，且默认配置必须 bit-exact 冻结 v0（对齐 prereg §6.2 / 646 号裁决先例）。

- **方案A：level 权重表进乘法因子（最小侵入，仿 w_grade 模式）。** 仿照 `w_grade`（`coverage.rs:1489-1495`，G 轴因子化、默认 [1.0,1.0] 保 bit-exact）引入 `level_weights: Vec<f64>`（默认全 1.0），sizing 变为 `s_e = base_units × w_depth(depth) × w_level(level) × w_dir × w_grade`；plan_orders 路径在 `mod.rs:365-376` 的 `SizingInput` 加同名字段。性质：按级别缩放**单笔**名义，满足"级别配比"的权重语义；仍非聚合上限。
- **方案B：聚合级别毛帽（真"各级别独立仓位上限"，仿 G7）。** 仿照 `apply_gross_cap`（`coverage.rs:1679`，逐根子树 KKT water-filling）按 level 分组：`Σ_{e: level(e)=ℓ} |s_e| ≤ Γ̄_ℓ · U_ℓ`，超限时逐级别组 KKT 投影（组内保 depth_weights 比率——与 G7「子树内保 κ/depth_weights 比率」同一投影逻辑，proofs-full-strategy-20260703.md:360）。性质：把级别从"单笔标量"提升为 K_Θ 约束（strict §12 17 项约束族新增一项）；Θ 参数 Γ̄_ℓ 标注 [设计选择,L3经验待标定]。工作量最大，但是唯一回答"各级别是否应有独立仓位上限"的正面方案。
- **方案C：接线 U_ℓ 级别协变（orthodox 路线）。** 把 `runner.rs:1279` 的 base_units 改为按级别注入 `a_k` 缩放（`U_{ℓ+k} = a_k U_ℓ`），让 coverage.rs:2645 的既有注释成为实装。性质：直接落在协变框架内（S_k Θ = Θ 强形式），但改变所有级别的绝对名义标度，对冻结 v0 的 bit-exact 冲击最大，且 a_k 取值无教义锚，属最重设计决策。
- **组合建议**：若目标只是"级别配比可表达"，方案A 足够且最贴合 646 号裁决确立的"因子化 + 默认 1.0 bit-exact"先例；若目标是"级别风险隔离"（防某级别独大），须方案B。方案A+B 可叠加（A 管意图配比，B 管硬上限）。方案C 独立成项，与 A/B 正交。

## 6. 引用汇总

- 代码锚：`rust/src/theta_v0/strategy/voice.rs:5,180-202`；`rust/src/theta_v0/strategy/coverage.rs:1424-1428,1489-1495,1513-1567,1679,3681-3699`；`rust/src/theta_v0/strategy/mod.rs:166-177,261-264,360-377`；`rust/src/theta_v0/strategy/risk.rs:111,121,148-175,207-210,560-573`；`rust/src/theta_v0/config.rs:135-137,155-156,255-285,367-368`；`rust/src/theta_v0/backtest/runner.rs:442,1279,3306-3316`。
- 文档锚：`reference-theta-v0.md:39-47`（§Θ_voice/:42 深度权重，§Θ_risk/:47 sizing 公式）；`docs/formal-chain/proofs-full-strategy-20260703.md:360`（G7 投影先例）。
- 教义锚：无——60/30/10 与任何级别配比均为 [设计选择,默认值]，博文/编纂版（docs/chanlun/text/blog/INDEX.md 权威链）无对应资金配比条款；缠论教义只给结构（买卖点/级别），资金分配是 Θ 设计层自由参数。
- 数据锚：`/tmp/m8_opsem_fixed/trades.jsonl`（518 笔；字段 `certificate.level`、`certificate.nest_depth`、`voice_tree_at_entry.depth`、`voice_tree_at_entry.w_depth`、`units`）。
