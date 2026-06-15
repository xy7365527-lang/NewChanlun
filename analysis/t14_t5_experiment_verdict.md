# T14/T5 根翻转实验判决：必然性推论 vs 拼接有效域

> 任务（2026-06-15 编排者）："严格实装 4 条交易层推论（T1/T5/T14/A5），消除 unn
> '近似'"。最终裁决（编排者 msg2）："T14 根翻转是必然性推论（第21环），CL/BRN 的退化
> 是回测否定，不否定必然性。回测否定的是当前拼接方式在这两个标的上的有效域，不是 T14
> 本身。必然性累积不被经验否定——这是纲领。"
>
> **存放位置裁决**：T14/T5 实装提交到实验分支（`experiment/t14-t5-root-flip-necessity`），
> **不合入主引擎 unn**（main 保持 root 恒仓降成本形态——CL/BRN 正域 +132%/+271%）。
> 本文档（main 上）= 实验的持久发现记录。

## 0. 认识论分层（formalization-validity-domain.md）

| 层 | 命题 | 状态 |
|----|------|------|
| **L0** | T14 根翻转是第21环（卖点翻空买点翻多）的必然推论 | 必然（概念推导，不被经验否定，§0 纲领） |
| **L2** | in-place 翻转 + MtM 根空头会计守恒（Σunits=N_base ∧ NAV 不变） | 成立（11M bar prove 零 panic + 11 单测 + `prove_t14_root_flip`） |
| **L3** | T14 拼接在 CL/BRN 的回测 P&L | **否证**（CL +132%→−9%，BRN +271%→−17%）——**有效域读数，非必然性否定** |

**关键区分**：L3 否证缩小的是**拼接方式的有效域**（T14 在震荡 regime 不适用），**不是 T14 这条
必然性推论**。必然性累积——T14 作为环21 的推论，与 N1-N8 一样是 unn 的结构必然，只是其
**操作拼接（根级别翻转 vs 根恒仓降成本）的有效域是 regime 函数**（539号：清仓/翻转频率无普适最优）。

## 1. 实装内容（实验分支）

### T14 根多空对称翻转（环21）
- `unified_necessity.rs` C 块：单根 ∧ 反向 type1 背驰@source S≥E* ∧ prove_chain ⇒
  **in-place 翻转根极性**（长→空 / 空→长），不增删 voice（N1 单根保持），units 不变（A10 同股数 M=N）。
- 触发判据 = **type1 买卖点**（`sig.sell1[s]`/`sig.buy1[s]`，环21 卖点翻空买点翻多），
  **非根否定**（根否定→翻转 = stop-and-reverse = whipsaw 破产，三次试探全否，见模块删除史）。
- 根否定（破 027:25）⇒ **恒仓 noop**（不翻转、不回现金——A8 否定线先于保证金线，root 不踏空）。

### T14 根空头 MtM 会计（§8.4 有效域边界扩展）
- `isolated_fugue.rs` `nav()` + `close_voice` 根分支：**子空头**（parent=Some）= 内部负债
  父吸收 ⇒ 冻结 capital；**根空头**（parent=None）= 外部市场负债 ⇒ MtM = `capital − units×c`。
  根无父吸收 liability，冻结会造成"只赚不赔提款机"bug；MtM 使翻转/否定/EOD 同价 c 守恒且真实兑现 P&L。
- iso 永不创建空头根（根恒多）⇒ iso **bit-exact 不变**（12/12 测试守住）。
- **有效域边界（声明=能力，同 A7）**：MtM 根空头**不 spawn 降成本**（MtM 短父 spawn 长子破坏守恒
  +m×c，try_spawn 为冻结口径设计）⇒ 根空头是叶节点，多空对称在降成本层不完整。

### T5/A5 根级别涌现 = 会计重组（环20 + §6）
- `root_emergent_ladder`（dir-aware，iso 双向扩展）：根级别随走势涌现单调上爬（多沿 Up/空沿 Down）。
- C 块 in-place relabel `root.ladder = re`（纯会计无物理交易 ⇒ units/NAV 不变 ⇒ N8/A2 安全）。
- 出场资格级别由涌现 E* 决定（非入场固定）⇒ §6"只有最高涌现级别走势完美才清仓/翻转"的算子化。

## 2. 未实装（不可约矛盾，no-workaround）

| 推论 | 字面诉求 | 冲突 | 严格形式 |
|------|---------|------|---------|
| **T1** | F 不被 pending 门控 / 从 segment 建仓 | **⊥ N5**（segment 非势源，prove_chain 硬断言 s≥PENDING_LO；裸扫描入场 = 539 A′ 解耦踏空轴） | 保持 N5-gated F；"永远在场"由 T14 翻转 + 根否定恒仓承载 |
| **A5** | 旧根降为新父子 voice 的**树重组** | **⊥ N8**（husk Long 父致旧根 close 走子返还分支 ⇒ N·c 误记入 nav 盲字段 capital ⇒ NAV 翻倍 panic） | 严格形式 = T5 涌现 relabel（已实装），非树重组 |

## 3. L3 否证证据（有效域读数）

| 标的 | regime | HEAD（根恒仓+降成本） | T14（根翻转） | Δ | P1 |
|------|--------|------|------|------|-----|
| **CL** | 震荡正域 | +132.4% | **−9.1%** | **−141.5pp** | PASS→FAIL |
| **BRN** | 震荡正域 | +270.9% | **−17.5%** | **−288.4pp** | PASS→FAIL |
| spawns(降成本) | | CL 2049 / BRN 993 | CL 169 / BRN 40 | 坍塌 | |
| roots(翻转churn) | | CL 37 / BRN 12 | CL 145 / BRN 64 | 暴涨 | |

> 注：上述 L3 数字为含 negate_line 的早期翻转变体（翻转根携 027:25 否定线 ⇒ B 否定churn）。
> 提交变体（type1 翻转 + 根否定恒仓 noop）的 negate-churn 已消除，但**方向性踏空根因不变**——
> 震荡 regime 中 type1 顶翻空 ⇒ 价格反弹 ⇒ 翻转腿失血。L3 否证域 = 震荡 regime（CL/BRN）。

**机制根因**：CL/BRN 是震荡 regime，更高级别走势不持续 ⇒ `re`(涌现层)≈root_ladder ⇒ T5 涌现门
**不提供保护**（保护只在持续趋势 ES/BTC，re 爬高使翻转稀疏）⇒ T14 在每个震荡顶翻空 ⇒ 降成本
alpha 被摧毁。这是 539号 `constitutive_throughput_falsified` 的精确复现 + `signal_layer_duality`
（located 折叠进层选择丢失独立闸门）的根级别镜像。

## 4. 结果包六要素

1. **结论**：T14 根翻转（环21 必然性推论）L0-必然 + L2-守恒正确，但 L3 拼接在 CL/BRN 震荡正域
   否证。保留为实验分支；主引擎 unn 保持根恒仓降成本（CL/BRN 正域形态）。
2. **定义依据**：T14←环21（多空对称）；MtM 根空头←§8.4 有效域边界；T1⊥N5←环7（segment 非势源）+pcf
   坍缩谱系；A5⊥N8←§8.1/§8.4 守恒。
3. **边界条件（结论翻转）**：若在持续强趋势 regime（ES/BTC，re 爬高），T5 涌现门使翻转稀疏 ⇒ T14
   有效域成立甚至增益（bidir_s1s4 BTC+7368% 旁证）。有效域 = 趋势 regime；否证域 = 震荡 regime。
4. **下游推论**："消除 unn 近似"的前提（"~"是缺陷）被否证——4 条"~"是**正确的有效域边界**，
   `necessity_derivation.md §5` 的"有意识取舍"标注是对的。T14/T5 的价值是**显式化有效域边界**，
   不是替换主引擎。
5. **谱系引用**：`constitutive_throughput_falsified`（539 A′ 踏空）；`signal_layer_duality`（located
   折叠丢失独立闸门）；`formalization-validity-domain`（有效域<定义域：L2 代数成立≠L3 经验成立）；
   `nrf_v4_strict_accounting`（A8 否定线支配保证金线）。删除史：df752ea6e4（根否定翻转 whipsaw）→
   6faf4ec45a（T1⊥A8 扬弃）→ c6deae8780（根恒仓收敛）。
6. **影响声明**：实验分支改 `unified_necessity.rs`（+T14/T5/prove_t14_root_flip）、
   `isolated_fugue.rs`（nav MtM 根空头 + close_voice 极性完备，iso bit-exact 守住）、
   `positional.rs`（删 negate_hedges 观测态字段）。**主引擎 unn（main）不变**——CL/BRN 正域守住。

**认识论等级**：T14 必然性 = L0；守恒正确 = L2（11M bar prove 零 panic）；拼接 P&L = L3（震荡 regime 否证，趋势 regime 待验）。
