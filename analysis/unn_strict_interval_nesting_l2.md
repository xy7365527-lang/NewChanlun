# unn located 区间套递归到底（时序）—— 修复 + L2 实证

> 任务（编排者 2026-06-15，两轮）：
> 轮1：按 `buysellpoint_necessity_vs_canon.md`（L0）修复 located 偏离原文三处。
> 轮2 修正：**不是"低三级"而是递归到底（成本门终止），每级 type1**；并查清"为什么 located
> 从未武装"（域空是矛盾——信号层 S11 保证买卖点首尾相连）+ 跑 1s a0 数据。
>
> 认识论等级：实装 L0；prove 零 panic / bit-exact = L1；8 标的 + 1s 回测 = L2/L3。

---

## 1. 结论

located 严格形式 = **区间套递归到成本门终止（theta<friction），每级 type1 背驰确认**
（每级 type1 = 该级走势完美 = 该级 voice 可闭合，**不跳级**——否则中间级 voice 会计状态未确认）。
单 bar 实现架构上不可能（相邻 ladder type1 同 bar 共现率 ~1%），故实装为**时序 frontier
累积器**。**域空已消除**——8 标的全部入场交易，prove N1-N8 零 panic，T14 根翻转可达，
QQQ 全量 +166.9%（BH +174.6%，MDD −22.8% 优于 BH −25.6%）。

## 2. 定义依据 + 实装（unn 专属，BSP 引擎与共享 rec_sub_evidence/nrf 不改）

| 判据 | 原文 | 实装 |
|------|------|------|
| 递归到底 | 第64课:65"低三级以上"=最低要求；严格=递归到成本门 | `cost_gate_open(f)=theta(f)≥SUB_COST_K×friction`；frontier 下探直到门关 |
| 逐级 type1 | 第29课:396"都要下次级别以下找第一类" | `has_type1`（只认 type1，type2/3 不算）；frontier 每级必有 type1 才降 |
| 后续走势 | 第29课:52/54 反弹后次级别走势验证 | `compress_bar < confirm_bar` 严格（`prove_chain` release-active assert）|
| 时序累积 | 区间套递归"定位到一个时间、价格的点"=跨 bar 收敛 | `Pending.frontier` 跨 bar 累积（type1 逐级在后续 bar 出现）|
| C/E 区分 | §9 type1→清仓/翻转；type2/3→降成本 | confirm@floor 同一 fire 两路：confirm_*（C/F）+ nf_*（E）；sig.sell1/buy1[source] 区分 |

frontier 机制：候选@k 武装 → frontier=k−1；每 bar 若 `cost_gate_open(frontier) ∧
has_type1(frontier)` 则 frontier−1（区间套递归，不跳级）；当 `cost_gate 关闭`（递归到底
floor）∧ `since<bar` ⇒ confirm。floor 处候选即定位点（无更细 cost-effective 子结构可 refine）。

## 3. 域空根因（debug：递归链在哪断）—— 两层

### 3a. 时序错位（单 bar 实现不可行）

type1 在相邻 ladder **几乎从不同 bar 共现**（QQQ 200K）：

| 检查 | 结果 |
|------|------|
| type1@move L1(3) 与 type1@segment(2) 同 bar 共现 | **1 / 99** |
| segment type1 总 1110，同 bar move L1 也 type1 | **1** |

⇒ 单 bar"每级 type1"链必然域空。区间套定位到点本就**跨 bar 逐级收敛**（S11 首尾相连
是**时序流**性质，非同 bar）⇒ located 必须时序累积（frontier）。

### 3b. 成本门 floor = 数据决定（frontier<sub 守卫错误）

中枢中位振幅 vs 摩擦门 0.2%（=SUB_COST_K 2.0 × SUB_FRICTION_RT 0.001）：

| ladder | QQQ 1min | CL 1s |
|--------|---------:|------:|
| segment(2) | 0.105% ✗ | 0.037% ✗ |
| move L1(3) | 0.240% ✓ | 0.099% ✗ |
| recL2(4) | 0.453% ✓ | 0.211% ✓ |

候选 ~95% 在 move L1(3)。上一版守卫"至少降一级（frontier<sub）"要求候选下探到子级——
但 QQQ 1min 的 move L1 子级 segment 在成本门**之下**（0.105%<0.2%），CL 1s 连 move L1
自身都在门下。候选恰在成本门 floor ⇒ 被守卫全拒 ⇒ 域空。**移除守卫 = 真正递归到底**：
floor 处候选凭自身 type1 确认（C/F 由 sig.sell1/buy1[source] 门控），不强制下探。

## 4. 有效域读数（L2/L3）

### 4a. 1min（250K bar/标的，prove 零 panic）

| 标的 | 入场 | 翻转 | spawn | trades | strat% | BH% |
|------|----:|----:|------:|------:|-------:|----:|
| QQQ(728K NT) | 1 | 0 | 19 | 20 | **+166.9** | +174.6（MDD −22.8 vs −25.6）|
| OKLO | 1 | 0 | 27 | 28 | +225.1 | +328.6 |
| BTC | 1 | 0 | 32 | 33 | **+155.3** | +74.8（超 BH）|
| CL | 1 | 2 | 41 | 44 | +12.4 | +39.4 |
| GC | 1 | 2 | 0 | 3 | +6.7 | +15.3 |

恒仓（T1）：入场一次后持有 + 降成本，无主动清仓（翻转/强平为被动/T14）。域空消除——
全标的交易。QQQ bit-exact（NT 流式==批量）PASS。

### 4b. 1s a0（700K bar，编排者指令）

| 标的 | 入场层 | 翻转 | spawn | strat% | BH% |
|------|-------|----:|------:|-------:|----:|
| CL 1s | move L1(3) | 2 | 0 | −11.3 | +7.1 |
| ES 1s | move L1(3) | 3 | 0 | −3.8 | +4.1 |

1s 假设（"中间层级更多→递归到底更多层确认"）**部分否证**：1s finer resolution ⇒ 每级
振幅更小（segment 0.037%/move L1 0.099%），相对**固定**摩擦 0.2% ⇒ 成本门 floor **上移**，
非下移。候选仍在 floor（move L1）⇒ 入场@move L1（floor 处自身 type1 确认），降成本 spawn=0
（无 cost-open 子级可降），多翻转少降成本 ⇒ 这些切片负 alpha。

## 5. 边界条件

1. **摩擦门 0.2% 是绝对阈值**：成本门 floor = f(数据振幅/摩擦)。1min QQQ floor=move L1，
   CL 1s floor=recL2。降低 SUB_FRICTION_RT 会下移 floor（更多级可降成本）——未改（在册值）。
2. **入场恒在成本门 floor 层**：候选 95% 在 floor，故入场层≈floor。更高层（recL2+）入场需
   该层 type1 候选（罕见）。
3. **1s 负 alpha 是 regime/切片**：700K 1s ≈数周；多翻转少降成本的形态在这些切片负，全年/
   全标的待全量复验（未跑全量 5.96M）。

## 6. 下游推论

1. **域空 = located 单 bar + frontier<sub 双重 bug**，非定义矛盾——编排者"S11 首尾相连⇒
   链应存在"正确（链是时序的）。修复后链总能找到（floor 处确认）。
2. **C/E 耦合修复 E-interference**：上一版 loose nf 在多 bar located 构建期抢先 E spawn 碎裂
   根 ⇒ C 翻转被多 voice 阻断（T14 不可达）。耦合后 confirm@floor 同 bar fire 两路，C（type1）
   在 E 前互斥 ⇒ T14 可达（CL/GC 翻转）。
3. **成本门 floor 是操作分辨率**：1min floor=move L1 ⇒ 笔/线段级结构在 1min 下不可操作
   （振幅<摩擦）。这是 521号 PH settle 用途边界的会计层对应。

## 7. 谱系引用

- `buysellpoint_necessity_vs_canon.md`（L0）：本实装定义依据。其边界条件1（ladder↔级别映射）
  + 第7节张力（已结算裁决 vs 第64课）在本轮由"递归到底+成本门"消解——成本门是数据决定的
  floor，非固定级数。
- `nested_fugue.rs:135-148`：共享 rec_sub_evidence 不变（nrf v4 327 测试守卫）；unn 改用
  时序 frontier（本文件）。
- `project_pcf_1s_a0_source_collapse` / `project_cl_1s_a0_verdict`：1s a0 先例（秒级=观测分辨率，
  本轮成本门 floor 上移给出会计层根因）。

## 8. 影响声明

- 改 `rust/src/trading/unified_necessity.rs`：`cost_gate_open`+`has_type1` helper；`Pending.frontier`
  时序累积器（删 fired_nf）；confirm@floor 耦合 nf/located 两路；`cascade_arm`/`prove_chain`
  时序 `<`；13 unn 测试时序链重写。
- 守恒律 violation=panic：prove N1-N8 零 panic（8 标的 1min + CL/ES 1s）；S11 located 流零
  violation；bit-exact NT 流式==批量。
- commit：`4b01ceebd1`（代码）+ 本文件。BSP 引擎与共享 rec_sub_evidence/nrf v4 零改动。
