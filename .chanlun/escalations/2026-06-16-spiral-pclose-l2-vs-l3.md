# 矛盾上浮：P-close（Δr=−1）的存在方式——「L2 表达力 re-description」⊥ 任务期待的「L3 alpha 操作机制」

> 上浮人：螺旋引擎 v2 验证工位（2026-06-16，Step 6+7 真实数据验证）
> 触发：执行任务 Part 3「v2 核心改进是 H¹ 闭合……BTC E spawn 子空头是否不再全亏 wins>0？
> 161 次强平是否减少/消失」。8 标的全量真流式实测后，发现任务前提与已验证设计**结构性互斥**。
> 分类（四分法）：**选择**（P-close 的目标存在方式 = L2 re-description 还是 L3 操作机制，
> 需价值判断）+ **语法记录**（closure.rs 文档声明「向心下沉到次级别 φ归零」实际实现为
> 类型化/计数，非会计落点改变——此读法此前未显式化）。
> 等级：**L3**（8 标的真实数据 bit-exact 全 PASS，强平/空头胜负逐位等）。
> 关联报告：`analysis/spiral_v2_verification_verdict.md`。

---

## 1. 停下来：为什么这不是可自决的定理或行动

任务 Part 3 显式声明「P-close 是 v2 的**核心改进**」并预期它**改善** BTC spawn 子空头胜负、
**减少**强平。但已验证设计（架构 §2 + `mod.rs` §2）的目标是 spiral **bit-exact 复现 unn**。
两个目标互斥（见 §2）。要么：

- (A) 接受 P-close = L2 表达力 re-description（现状，bit-exact，设计文档一致），验证完成；
- (B) 重设计 closure.rs 使 Δr=−1 **真改操作落点**（子声部闭合落到 r=k−1 而非 unn 原级别）
  ⟹ 破 bit-exact ⟹ 试 L3 alpha。

(A) vs (B) 是**价值判断**（spiral 应该是什么），不是从已结算原则的逻辑必然推论。我不能
单方面重设计引擎（破坏架构 §2 已声明的 bit-exact 目标），也不能伪造 Part 3 的差异
（`no-workaround.md`）。故**不可自决**。

---

## 2. 精确描述矛盾：bit-exact ⟺ P-close 零增量（两者不能同真）

### 2.1 实测（L3，8 标的全量真流式）

| 标的 | strat Δ(spiral−unn) | 强平 spiral=unn | 空头 wins spiral=unn | bit-exact |
|------|------|------|------|------|
| BTC | 0.0 | 89=89（Δ0） | 9=9 | ✅ |
| CL | 0.0 | 0=0 | 4=4 | ✅ |
| ES | 0.0 | 22=22 | 3=3 | ✅ |
| OKLO | 0.0 | 36=36 | 0=0 | ✅ |
| QQQ/GC/DX/BRN | 0.0 | 全 Δ0 | 全等 | ✅ |

**任务 Part 3 预期全否证**：BTC 强平未减少（89=89），子空头 wins 未增（9=9，pnl −206757 逐位等）。

### 2.2 为何必然（定理）

| 命题 | 来源 |
|------|------|
| spiral 复用 unn `CenterBook`/`DepthRef` 成本门 + 会计落点 | 架构 §2.4，`project_spiral_engine_v2_bitexact` |
| closure.rs `Δr=−1` = unn 既有跨级别闭合（N5⊥N7）的群论 re-description，非新机制 | closure.rs §6.1 + 实测 |
| `cross_level_closures` 是 spiral 唯一新增可观测量，**只观测不干预** | ffi.rs result_to_dict |

∴ bit-exact（已 8/8 验证）⟺ P-close 零 P&L 增量。**接受 A 则 Part3 恒 0；要 B 的 alpha 则
bit-exact（Part1）必失败**。

### 2.3 语法记录：closure.rs 文档 vs 实现的读法差

closure.rs §6.1 文字：「势耗尽**不在开仓级别 r=k 原地闭合**……而是**向心下沉到次级别 r=k−1**
（φ 归零）」。字面读 = 会计落点应在 r=k−1。但实现 bit-exact ⟹ 落点 = unn 原级别。

调和读法（本工位推断，待裁决）：unn 的 E spawn/D 回补**已经**是「向心下沉」行为，
Δr=−1 是给它的**群论命名**（扬弃 089：否定+保留落点+提升存在方式）。即文档描述的是
**已被 unn 实现的运动**的群论形式，不是要求一个**新的**落点。`p_close_descend_level(k)=k−1`
是这个运动的**坐标读出**，不是**重定向指令**。

---

## 3. 接受 A 会怎样 / 接受 B 会怎样

- **接受 A（L2 re-description）**：验证**完成**。spiral 的价值 = 表达力 + 可证性（群关系 panic
  守卫 + A∉M 编译期显形），不是 alpha。unn 永久保留为 bit-exact 基线。BTC/CL/BRN 的 P1 fail
  是 unn 机制的有效域读数（539 regime 正交），spiral 完整继承，P-close **不是修复路径**。
  与既有谱系 `project_spiral_engine_v2_bitexact`「表达力 L2 非 alpha」一致。

- **接受 B（L3 操作机制）**：需改 closure.rs 使子声部闭合的**会计落点**真正下沉到 r=k−1
  （独立于 unn 的 CenterBook 落点）。这破坏 Part1 bit-exact（预期且**应该**破坏——bit-exact 是
  A 的判据，不是 B 的）。届时 Part3 才可能产生非 0 差异，但需**重新预注册判据**（L3 alpha 是否
  改善 BTC spawn/强平）并接受可能的否定性结果（`formalization-validity-domain.md`：否定比确认
  更有价值）。这是**新设计周期**，非本次验收对象。

---

## 4. 请编排者裁决

P-close（Δr=−1）的目标存在方式：

1. **A**：L2 表达力 re-description（现状，验证完成，与设计文档/谱系一致）；或
2. **B**：重设计为 L3 发散操作机制（破 bit-exact，试 alpha，新周期 + 新预注册）。

本工位倾向 **A**（架构 §2 已声明 bit-exact 为目标，谱系已结算「表达力 L2 非 alpha」，
Part3 的 alpha 期待与该设计互斥而非该设计的缺陷）。但若编排者本意是 B（任务 Part3 措辞
「核心改进」「wins>0」「强平减少」指向 alpha），则需启动新设计周期——本工位不擅自重设计。
