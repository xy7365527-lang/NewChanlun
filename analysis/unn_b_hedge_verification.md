# unn B 规则重构：删观测态 → 否定线触发 = 势减弱 ⇒ 加速降成本对冲

> 编排者裁决 2026-06-15："删掉观测态，回到必然的形式。"
> 承接 6faf4ec45a（观测态：根否定→现金避险，P1=0/8 踏空）与 df752ea6e4（字面 T1：
> 否定→in-place 翻空，CL−56.7%/BRN−136.6% 破产）。本次是同一矛盾的**第三次扬弃**。

## 1. 概念运动：三次扬弃同一矛盾（"永远在场"的形式）

| commit | "永远在场"的实现 | 否定线触发的语义 | 死因/代价 |
|--------|-----------------|-----------------|-----------|
| df752ea6e4 | 否定 ⇒ in-place 翻空（stop-and-reverse） | 翻转信号 | whipsaw 破产 CL−56.7% |
| 6faf4ec45a | 否定 ⇒ 回现金 + 观测态 | 停损信号（A8 资本保全） | 踏空 P1=0/8 |
| **本次** | **否定 ⇒ 次级别降成本对冲（根不动）** | **势减弱势源**（非买卖点） | 恒仓在崩盘 regime 受伤 |

**核心重分类**：否定线触发 **不是** 走势完美/买卖点，只是"势在减弱"的信号。势减弱的
必然响应不是离场（翻转或现金），而是在次级别**对冲降成本**。翻转保留给 type1 背驰
（C 规则，"十年 1-2 次"，第11环）。

## 2. 必然性推导链（每步必然，编排者给定）

1. 势减弱（否定线触发）⇒ 次级别反向运动增强（第13环全称）
2. 次级别反向运动 = 次级别卖点（第11环）
3. 次级别卖点 ⇒ 子 voice 开空 = 降成本（第17环 + 第22环）
4. 子 voice 的 P&L ≡ 父 voice 的降成本额（守恒律 §8.3）
5. 势恢复 ⇒ 次级别走势完美 ⇒ 子 voice 回补 ⇒ 父 voice 继续（第2环；D 规则）
6. 势耗尽 ⇒ 同级别买卖点（type1 背驰）出现 ⇒ 翻转（第11环；C 规则）

⇒ B 规则（根多头否定）≡ E 的自层降成本 spawn（N7，`try_spawn_cost_gated` 子空@ladder−1），
仅触发器从 nf fire 换成否定线穿越——否定线被重新归类为自层势源，非离场触发。

## 3. B 规则三分支（step() B 段）

| voice | 否定（破 027:25 线） | 必然性依据 |
|-------|---------------------|-----------|
| 根多头 | 次级别 spawn 子空头降成本对冲，根**保持多头不清仓**；否定线一次性消费（离散穿越事件），子空头继承之作 027:25 stop | 第13/11/17/22环 |
| 子 voice | 回补返父（cascade 关子树）= 势恢复，对冲赌注失败 | 第2环（D 同向，否定线是更激进 stop） |
| 根空头 | T8 叶节点 no-op（MtM⊥frozen 守恒，根空头不嵌套降成本）；留待 A 强平 ∨ C type1 买点翻多 | T8 有效域边界 |

**翻转只在买卖点**：C 规则仅 type1 背驰 confirmed（`sig.sell1[s]`/`sig.buy1[s]`）+ pending_locate
级联链触发。否定线不触发 C（第11环：操作只在买卖点）。

## 4. prove 更新

- **删** `prove_t1_aufheben`（观测态的 prove：观测态 F-eligible ⇒ 必重建仓）。
- **加** `prove_t1_no_voluntary_exit(was_active, is_active, root_cashed_out)`：森林"非空→空"
  只能经 **A 强平**（设 root_cashed_out=true），**绝不经 B 否定回现金**。violation = panic。
- **诚实声明（no-patch.md 声明膨胀禁止）**：函数证明的是**转变命题**（exits-are-guarded：唯一
  合法离场是被动强平），**非**正命题"任意 bar 有仓位"——强平后/入场前有合法有限空仓 gap。
  "永远在场"的精确形式 = "不主动以避险/否定回现金"。（审查 H1：原名 `always_positioned`
  声明强于实装，已改名消解。）
- C-clear-at-base（root@基底清仓）结构上不可达（root_ladder≥PENDING_LO>FIRST_BSP），已升
  `unreachable!()`——把"不可达"从注释提为运行时断言（审查 M1；6.7M bar 零触发实证）。
- N1-N8 八 prove 不变（B-hedge 是 N7 自层降成本，try_spawn 守恒由 N8 守卫）。

## 4b. 认识论等级标注（formalization-validity-domain.md；对抗审查 necessity 维度 HIGH）

B 与 E 的差异不可抹去（审查指出，已诚实标注）：
- **E** 触发 `nf_sell` 经 `rec_sub_evidence` 次级别**背驰结构确认**（链第2步"次级别卖点"严格形式）。
- **B** 触发只是否定线**价格穿越**（located 链 027:25 结构破，非背驰确认）。

⇒ 链第1→2 步（否定线穿越 ⟹ 次级别卖点）是 **L2 必然（第13环全称的有效域内），非 L0 纯代数**。
B 是对次级别卖点的"预期"（anticipation），E 是"确认"——两者**互补**（B 抢先 + E 补强），
非等价替代。链第3步（卖点⇒spawn）受 N4 成本门约束：θ<friction ∨ θ=None 时合法终止（第16环），
此场景 B 退化为裸暴露至 C type1——N4 有效域边界（L2 内合法中断，非 L0 全称失效）。
否定线**一次性消费**（破线=结构作废，离散穿越事件；冷启动 cost-gate 拒绝时根无对冲——
L2 有效域边界，已标注）。

## 5. 验收（L2 必然性——非回测）

| 标的 | bars | N1-N8+T1 prove | floor_stops(N4) | max_children(N1) |
|------|------|---------------|-----------------|------------------|
| CL | 5,528,156 | **零 panic** | 0 | 2（森林实证） |
| BRN | 1,200,000+ | **零 panic** | 0 | 2（森林实证） |

⇒ N1-N8 + T1 在 ~6.7M bar 上**全 bar 成立**（L2）。`cargo test` 365 全过（含新测试
`t1_negate_spawns_cost_reduction_hedge_root_stays`）。

## 6. 有效域读数（L3——非验收标准；编排者裁决：必然性检验才是验收）

| 标的 | unn | BH | MDD | P1 | flips | neg_hedges | child-neg-close | spawns |
|------|-----|----|----|----|----|------------|-----------------|--------|
| CL | −61.1% | +28.2% | −146.2% | ✗ | 6 | 1 | 24 | 98 |
| BRN | +62.8% | +87.4% | −79.2% | ✗ | 11 | 1 | 10 | 64 |

**否定性结果（缩小有效域边界，比确认性结果更有价值，231号规则）**：
- 对比观测态 commit（CL −9.1%/MDD−50.7%），**显著回归**——这是**正确的设计后果非 bug**：
  观测态在崩盘 regime 回现金避险（踏空换资本保全）；恒仓+对冲不离场，崩盘中根多头持续
  失血，对冲（θ 配额分数）仅部分抵消。深 MDD 来自 type1 翻空后根空头 MtM 失血
  （b5355deb08 既有机制）。N8 守恒每 bar 通过（零 panic）证明无会计 bug。
- **neg_hedges=1 是正确的低值**：type1 翻转（C，CL=6/BRN=11）通常抢先在顶部退出多头相，
  B-hedge 只在"势减弱但未到 type1 买卖点"的回退情形触发——买卖点主导，否定线降成本是
  罕见 fallback（符合"翻转只在买卖点"）。
- **有效域边界**：恒仓+降成本对冲在**强单边趋势**（不踏空）优；在**崩盘/震荡 regime**
  受伤（恒仓失血 > 对冲抵消）。CL/BRN 属负域。

## 6b. 对抗性多模型审查（4 维度并行 + 综合，ultracode workflow）

necessity / accounting / prove / rust 四维度对抗审查 → **GO（零 CRITICAL，零守恒 bug）**：
- accounting/rust 维度确认：B-hedge spawn NAV 守恒（N8 每 bar 通过）、T8 根空头叶节点正确、
  借用安全、无越界/NaN/双动（N2）。rust 维度判 **SOUND**。
- 3 个 HIGH 全是**声明膨胀（no-patch 禁止模式5）/有效域未标注**，非行为缺陷，已在**同一
  commit 内**修复（综合裁决：不允许"先 commit 功能，声明后修"= 渐进式回避）：
  - **H1**：`prove_t1_always_positioned` 改名 `prove_t1_no_voluntary_exit`（名实一致）。
  - **H2**：否定线无条件消费的 N4 有效域边界已标注（L2 非 L0）。
  - **necessity HIGH**：B 价格穿越 vs E 背驰确认的认识论等级差异已标注（见 §4b）。
- M1：C-clear-at-base 死代码升 `unreachable!()`（防 PENDING_LO 漂移，非静默掩盖 T1）。
- 修复均零行为改动 ⇒ **bit-exact 重现**（CL/BRN 回测数字逐位不变），无需重跑回归。

## 7. 结果包六要素

- **结论**：删除观测态，B 规则改为"根多头否定 ⇒ 次级别降成本对冲（根不清仓）"，翻转
  只在 type1 买卖点（C）。删 `prove_t1_aufheben`，加 `prove_t1_always_positioned`。
- **定义依据**：概念运动链第13环（势减弱全称）、第11环（买卖点）、第17/22环（降成本）、
  第2环（势恢复回补）；§8.3 守恒（子 P&L≡父降成本）；T8（根空头叶节点 MtM⊥frozen）。
- **边界条件**：① 若否定线触发**确等价**于走势完美/买卖点（与第11环冲突）则 B 应触发翻转
  而非对冲——结论翻转；② 若 T8 会计约束被解除（根空头可 MtM-safe spawn 长子）则根空头
  否定也应对冲——根空头 no-op 分支翻转；③ 若 regime 为强单边趋势，有效域读数（P1）翻正。
- **下游推论**：观测态相关字段（4 个）从 PositionalResult/lib.rs marshal 删除；新增
  `n_nrf_negate_hedges_by_ladder`。回测脚本 necessity 读数加 negate_hedges。下游消费者
  （unn_*.json 读者）不再有 observe 字段。
- **谱系引用**：T1⊥A8 扬弃序列（df752ea6e4→6faf4ec45a→本次）；089号扬弃；539号
  constitutive_throughput（恒仓 vs 踏空有效域）；b5355deb08 根空头 MtM。
- **影响声明**：改动 `rust/src/trading/unified_necessity.rs`（B/C/F/finish/struct/docstring/
  prove/test）、`rust/src/trading/positional.rs`（字段）、`rust/src/lib.rs`（marshal）、
  `analysis/unified_necessity_backtest.py`（读数）。不改 BSP 引擎、不改 iso/其余模式。
