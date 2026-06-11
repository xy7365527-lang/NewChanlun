# 引擎 BSP 缺口诊断：递归层 type1 缺失 + type2 恒空的根因

> 调研对象：为什么引擎在 recursive 层（ladder≥4）不产出 confirmed type1 buy？
> 为什么 type2 完全缺失？——"吃到每一笔"的最大阻塞。
> 数据：OKLO 447,739 bar（1min, databento）。认识论等级：**L2**（单标的真实数据，
> 根因机制为结构性证明 L0 + L2 实证双重确认）。
> 探针（只读，未改引擎）：`analysis/_bsp_gap_probe.py` / `_bsp_gap_probe2.py` /
> `_bsp_gap_probe3.py`；原始数据 `analysis/data_cache/_bsp_gap_probe.json`。

---

## 0. 结论（TL;DR）

两个问题是**同一个结构缺陷的两个表现**：走势（move）的边界语义使趋势背驰的
C 段（离开最后中枢的走势段）在关键位置**结构性为空**。

| 问题 | 根因 | 性质 |
|------|------|------|
| ladder≥4 无 type1 | 递归层 move 的 `seg_end = last_zs.comp_end`，C 段恒空（`c_start > c_end` 100% 命中），趋势背驰在递归层**不可能被构造** | 代码级语义缺陷（level-1 与递归层语义不一致） |
| type2 完全缺失 | type1 锚 100% 钉在段数组末段（pending move 尾锚）→ `find_next` 越界；settle 后离开段被下一中枢吸收 → C 段变空 → type1 消失。**type1 在 settled 结构上的存在窗口 = ∅** → type2 的源恒空 | 定义级缺陷（背驰 C 段被 move 边界截断） |

附带发现（严重性不低于两个主问题）：**buy1[2] 的 2559 次触发全部是
"生长中 C 段"瞬态信号**——用完整 A 段对比尚未走完的 C 段，C 段短时力度必然小，
背驰必然成立；C 段走完后 1120/1121 的背驰消失。当前 buy1 掩码的信号质量
需要重新审视（见 §4）。

---

## 1. 问题1诊断：递归层为什么不产出 confirmed type1

### 1.1 实证：L4 的 58 个事件全部是 type3

OKLO 447K 全程事件流聚合（探针1）：

```
ladder4: type3 buy  confirmed=False:18 / True:14
         type3 sell confirmed=False:15 / True:11     合计 58，type1/type2 = 0
ladder5: type3 buy  confirmed=False:1  / True:2      合计 3，type1/type2 = 0
```

buy1 掩码只认 confirmed type1 buy → ladder≥4 恒 False → entry≤3 →
tranche 区间空定义域。**不是"产生了被过滤"，是 type1 根本没被构造。**

### 1.2 排除法：背驰前提全部满足，背驰本身为零

L4 终态 15 个 move **全部**是 `trend` 且 `zs_count≥2`（趋势背驰的 move 级前提
15/15 满足），但背驰数 = **0**。失败点在 `detect_trend_divergence` 内部。

### 1.3 根因：C 段结构性为空（16/16 逐个验证）

`rust/src/divergence.rs:340-344`：

```rust
let c_start = zs_last.seg_end as i64 + 1;
let c_end = mv.seg_end;
if c_start > c_end || ... { return None; }
```

而递归层 move 构造 `rust/src/level.rs:258`（docstring 第12行明示）：

> `moves_from_level_zhongshus` **无 num_segments 调整**——末组 seg_end 直接取
> last_zs.comp_end（不像 level-1 的 next_seg_start-1 / num_segments-1 扩展）

即递归层 **每个 move 的 `seg_end ≡ 最后一个中枢的 comp_end`** → 永远
`c_start = comp_end+1 > c_end = comp_end` → 趋势背驰恒 None → type1 恒不存在
→ type2（以 type1 为源）恒不存在。58 个事件全是 type3 的原因：type3 不需要背驰。

探针2逐 move 验证：L4 15 个 + L5 1 个 trend move，**16/16 C 段为空**。

### 1.4 这是 Python 原始设计的缺陷，不是 Rust 移植 bug

- Python 原版 `src/newchan/a_zhongshu_level.py:240`：`seg_end=last_zs.comp_end`（相同）
- Python 区间套模块 `src/newchan/a_nested_divergence.py:157-159`：
  `c_start = zs_last.comp_end + 1; c_end = move.seg_end; if c_start > c_end: return None`
  ——**区间套递归定位撞的是同一堵墙**：递归层从大到小逐级找背驰段的链条在
  第一级就断了（大级别背驰从未存在）。

对照 level-1 的 `rust/src/moves.rs:115-121`：非末 move `seg_end = next_seg_start-1`、
末 move `= num_segments-1`——move 延伸出最后中枢，C 段非空，所以 ladder3 有
272 个 confirmed type1 buy。**递归层与 level-1 的 move 边界语义不一致**是缺陷本体。

---

## 2. 问题2诊断：type2 为什么完全缺失

### 2.1 实证：检测逻辑无 bug，但输入恒为空

`_detect_type2`（`src/newchan/a_buysellpoint_v1.py:252-283` ≡
`rust/src/buysellpoint.rs:267-321`）逻辑忠实于第17/21课：type1 后第一个
rebound（反向段）再第一个 callback（同向段），confirmed = 回调不创新极值。
**逐位移植正确**（37 项差分测试守卫）。死因全在上游：

OKLO 全程（探针1/2）：

| 观测 | 数值 |
|------|------|
| ladder2 瞬态 confirmed type1 buy 事件 | 2,559 |
| ladder2 type1 锚距末段距离=0 的占比 | **5581/5581 = 100%** |
| ladder2 终态在列 type1 | **1 个**（29,695 段中） |
| ladder2 type2 事件（含 candidate） | **0** |
| ladder3 终态 type1 / type2 | 2 / 2（type2 仅源于存活 type1） |

### 2.2 双重机制

**机制一（pending 期）：尾锚**。pending move 的 `seg_end = num_segments-1`，
趋势背驰 `c_end = mv.seg_end` → `t1.seg_idx = 末段索引`。
`find_next_seg_by_direction(segs, seg_idx+1, …)` 从数组末端之后开始找 → 恒 None
→ type2 不可构造。新笔到来时 pending move 延伸，type1 锚跟着滑到新末段
（所以同一轮背驰反复 fire 出 2559 个"新"事件），锚永远不会有后续段。

**机制二（settled 后）：离开段被下一中枢吸收**。探针3失败原因分布：
1121 个 settled trend move 中 **c_empty = 1120**（fc≥fa = 0！）。笔中枢层相邻
中枢首尾相接（下一组首中枢 `seg_start ≤ zs_last.seg_end+1`），settled move 的
`seg_end = next_seg_start-1 < c_start` → C 段在 settle 瞬间归零 → 背驰消失 →
type1 从列表蒸发。**type2 需要 type1 锚固定后再来 ≥2 段，但 type1 的存在窗口
与"锚后有段"的窗口交集为空。** 这不是 1min 数据的偶然，是
`zhongshu_from_strokes`（中枢吸收式延伸）+ move 边界语义的必然。

ladder3（线段中枢）中枢间有间隙，所以有 2 个 type1 存活并产出 2 个 type2
——交叉印证机制二。

### 2.3 与第20课原文的差距

知识库 §10.1（源自第17/20课，未回溯原始博文）：
> 第二类买点：第一类买点后，次级别上涨结束，再次下跌的那个次级别走势的结束点

代码以"本级别段"作"次级别走势"的代理（段是本级别走势的次级别构件），结构同构、
可接受。**真正的差距不在 type2 检测，在 type1 的存在论**：缠论的第一类买点是
走势转折的**事件**（事后在结构上固定于趋势极值）；引擎的 type1 是终态结构快照的
属性，而该快照在 settle 后不再包含背驰段。买卖点定律一（"第二类买卖点由次级别
第一类买卖点构成"）要求 type1 锚持久存在——当前实现不满足。

---

## 3. 附带发现：buy1[2]/buy1[3] 是"生长中 C 段"瞬态信号

2559 个 confirmed type1 buy 事件全部产生于 pending move 的生长期：A 段完整、
C 段尚在生长。C 段短 → 振幅×持续时间小 → `force_c < force_a` 几乎必然成立 →
背驰"成立"。C 段走完后重新评估，1120/1121 不再是背驰。

含义：当前 buy1 掩码触发的"背驰"大多数是 C 段不完整的伪背驰（C 段每延长一笔
就再 fire 一次）。这解释了在册结论"type1 BSP 仅 level-1 太细（41笔）"之外的
另一面：**数量多的层（ladder2）质量可疑**。修复 B（§5）顺带修正此 artifact。

---

## 4. 修复方案（分级，均未实施——按 no-workaround 规则待确认）

### 修复A：递归层 move 边界对齐 level-1 语义（问题1，代码级）

`rust/src/level.rs::moves_from_level_zhongshus` + Python 镜像
`src/newchan/a_zhongshu_level.py::moves_from_level_zhongshus`：
非末 move `seg_end = 下一组首中枢 comp_start - 1`；末 move `= num_components - 1`
（与 `moves.rs:115-121` level-1 语义逐字一致）。

- **性质**：消除同一概念（走势边界）在两层的不一致——属于"删除并用正确逻辑
  替换"，不是补丁。
- **影响面**：递归层 divergence（解锁）、type1/type2（解锁）、`a_nested_divergence`
  区间套（同时解锁）、BSP `MoveLookup` 的 settled 归属（更多组件被覆盖，type3
  的 settled 字段更准确）。move 身份键 `(direction, seg_start)`、settled 时点、
  D3 方向行、级别涌现判定（zs/move 计数）均不变。move high/low 语义不变
  （level-1 同样不含离开段极值，口径一致）。
- **模拟验证（探针3 Sim1，OKLO 终态）**：L4 立即出现 2 个趋势背驰
  （力度比 0.289 / 0.553，均 confirmed）→ **+2 confirmed type1 sell、
  +2 confirmed type2 sell**。buy 侧终态为 0（6 个下跌 move 扩展后力度不衰竭，
  被正确否定——没背驰就是没背驰）；事件级（pending 生长期）buy fire 需修复后
  逐 bar 重测。L5 数据量不足（仅 1 个 pending move），无变化。
- **代价**：递归层在册回测口径漂移（所有消费 ladder≥4 事件的结果需重跑）。

### 修复B：背驰 C 段免 move 边界截断（问题2，定义级，需上浮）

两个候选，可组合：

**B1（事件源化 type2）**：type1 confirmed 事件 fire 时冻结锚
`(seg_idx, price)`，type2 检测消费冻结锚而非终态快照。
- 优点：直接满足"type2 由 type1 事件构成"（买卖点定律一的事件读法）。
- 代价：打破 `buysellpoints_from_level` 纯函数快照契约（增量≡全量的 bit-exact
  差分测试框架需为 type2 增加事件态例外）。架构改动最大。
- 注意：B1 不修正 §3 的伪背驰 artifact（瞬态 fire 照旧）。

**B2（C 段越界极值定义）**：settled trend move 的 C 段 =
`[zs_last.seg_end+1, 趋势极值段]`，极值段允许落在下一 move 首中枢覆盖区内
（即不被 `mv.seg_end` 截断；down→区间内最低 low 段，up→最高 high 段）。
- 优点：**保持纯函数**（仍是结构的确定性函数），type1 锚固定在趋势极值段
  （即缠论语义上的转折点），type2 经现有 `_detect_type2` 代码自然解锁，零改动。
  同时修正 §3 artifact 的 settled 侧（背驰以完整 C 段评估）。
- **模拟验证（探针3 Sim2-B2，OKLO ladder2 终态）**：1121 个 settled trend move 中
  **type1 存活 384（confirmed 373：buy 187 + sell 186）、type2 解锁 384
  （confirmed 360，其中源 type1 buy confirmed 的 177）**。
  对照原定义：type1 存活 1、type2 = 0。
- 代价：背驰定义变更 → ladder2/3 全部 BSP 流漂移 → 在册回测全部需重新评估。
  pending 期的瞬态 fire 行为不变（流式快照本性）。
- **这是定义层修正，触发矛盾上浮条件**：现行"C 段=离开段∩move 内"与缠论
  "背驰段终于转折点"两种理解不可同时成立，需编排者裁决（建议 B2，理由：
  缠师第24课背驰段以转折点为终点，move 边界是工程分区不是概念边界）。

### 不建议的方向

- 调 `TYPE1_CONFIRM_RATIO` 阈值：根因是 C 段为空/被截断，调阈值是补丁思维。
- 给 type2 单独造检测旁路而不修 type1 存在论：声明膨胀（type2 名实不符）。

---

## 5. 对"吃到每一笔"的影响评估

### entry / tranche 空间（修复A）

| 状态 | buy1 可达 ladder | entry 上限 | tranche 区间 (2, entry-1] |
|------|----------------|-----------|--------------------------|
| 现状 | {2,3} | 3 | (2,2] = ∅（空定义域） |
| 修复A 后 | {2,3,4}（L4 通道打通，OKLO 终态 sell 侧已证；buy 侧事件级待测） | 4 | (2,3]：1 层 |
| 修复A + 更长历史/更多标的 | {2,3,4,5} 可能 | 5 | (2,4]：2 层 |

OKLO 447K（约2.4年）下 L5 只孕育出 1 个 pending move——L5 信号在此数据量下
本就稀薄，修复后主要增量在 L4。递归建仓的定义域从空集变为非空。

### 六种买卖点完备性（修复B）

现状四种在用（type1 buy/sell、type3 buy/sell），type2 buy/sell 缺位。
B2 模拟（仅 ladder2、仅终态、仅 OKLO）：**+360 个 confirmed type2**
（约 1/80 笔一个），其中源于 confirmed type1 buy 的 177 个——这正是
"1买后回调不创新低"的标准加仓/第二入场点，是当前交易层完全没有的机会类别。
同时 type1 自身从"瞬态闪烁"变为"settled 结构上 373 个持久 confirmed 锚"，
区间套（`a_nested_divergence`）随之获得可用的大级别背驰段输入。

### 量化注意

以上数字是**终态快照口径**（修复后结构的一次性重算），不是逐 bar 事件流回测。
事件级的信号时序、与交易层 FSM 的耦合效果，需修复落地后用
`compute_organic_signals` 全程重跑才能给出（L2→L3 需多标的）。

---

## 6. 结果包六要素

1. **结论**：递归层 type1 缺失的根因 = `moves_from_level_zhongshus` 的
   `seg_end = last_zs.comp_end` 使趋势背驰 C 段恒空（16/16 实证）；type2 恒空的
   根因 = type1 锚 100% 尾锚（pending 期）+ 离开段被下一中枢吸收（settled 后
   c_empty 1120/1121），type1 在 settled 结构上存在窗口为空集 → type2 源恒空。
   `_detect_type2` 本身无 bug。
2. **定义依据**：趋势背驰（`divergence.rs:322-363`，第24课 T2/T4 框架的振幅
   fallback）；type1 confirmed = `force_c/force_a ≤ 0.9`（candidate-fix）；
   type2 = 第17/21课回调不创新极值（`a_buysellpoint_v1.py:217-283`）；
   买卖点定律一（知识库 §10.2）；level-1 move 边界 `moves.rs:115-121` vs
   递归层 `level.rs:258`。
3. **边界条件**：①若改用 MACD 背驰路径（`enable_macd`），机制一/二仍成立
   （C 段空与力度度量无关），但 §3 的伪背驰比例会变。②若某标的笔中枢间
   存在系统性间隙（中枢不首尾相接），机制二弱化——ladder3 的 2 个存活 type1
   即此情形；在中枢稀疏的标的上 type2 可能偶发。③B2 模拟数字依赖振幅力度
   度量与 0.9 阈值；换度量则 373/360 会变，但"0 → 数百"的量级跃迁来自
   C 段从空变非空，对度量不敏感。④L5 结论受数据量限制（1 个 move），
   不可外推。
4. **下游推论**：①修复A 同时解锁 `a_nested_divergence` 区间套递归定位
   （其 C 段空判同源）；②buy1[2] 的 2559 次触发是生长中 C 段瞬态信号，
   所有以 buy1 掩码为入场门的在册回测（区间套入场、有机赋格 master 触发）
   的信号质量解释需要修订；③修复 B2 落地则 ladder2/3 的 BSP 流整体漂移，
   全部在册 BSP 消费方回测失效需重跑；④"L4 有 58 个事件"对交易层而言
   全是 type3——若交易层想在修复前利用 L4 信息，type3 是唯一通道。
5. **谱系引用**：candidate-fix / confirmed-fix（`a_buysellpoint_v1.py`
   docstring 发生史：confirmed 与 Move.settled 解耦）；区间套配对修复 P1-P7
   （bsp_events 级别隔离架构）；在册结论"引擎不发 type2"（complete_fugue_v2，
   本诊断给出其根因）；005b 对象否定对象（走势完成由 Move 内部机制否定——
   本诊断显示 move 边界语义反向污染了背驰对象的定义域）。
   **不确定**是否存在编号谱系直接记录"递归层 move seg_end 语义"的决策；
   `level.rs` docstring 第12行显示这是有意为之的移植忠实性选择，原始动机
   未见谱系记录——修复 A 落地前应补谱系考古。
6. **影响声明**：本产出为只读诊断，未改动任何引擎代码/定义。新增文件：
   本文档 + 3 个探针脚本（`analysis/_bsp_gap_probe{,2,3}.py`）+
   探针数据（`analysis/data_cache/_bsp_gap_probe.json`）。
   修复 A/B 均未实施：A 是代码级（可直接做，但口径漂移需重跑递归层在册回测），
   B 是定义级（触发矛盾上浮，需编排者裁决 B1/B2/组合）。
