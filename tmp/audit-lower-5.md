# 下层5条定义审计报告

**审计者**: audit-lower
**日期**: 2026-02-23
**方法**: 定义文件全文 vs 实现文件逐行交叉验证

---

## 1. 包含关系 baohan (v1.3)

### 一致性: ⚠️ 部分偏差

**定义约束清单**:
1. 包含关系判定: `K1.high >= K2.high AND K1.low <= K2.low`（或反之）
2. 合并方向由最近一对无包含关系的相邻K线决定
3. 向上合并: `new_high = max(h1, h2)`, `new_low = max(l1, l2)`
4. 向下合并: `new_high = min(h1, h2)`, `new_low = min(l1, l2)`
5. OHLC 工程处理: `open = 左K.open`, `close = 右K.close`
6. 初始方向 None → 默认按 UP 合并
7. 方向判定——定义文件记录两种: 原文单条件 `gn >= gn-1 → UP` / 代码双条件 `high_i > high_{i-1} AND low_i > low_{i-1} → UP`
8. 先左后右递推，完成后序列无任何相邻包含关系
9. 空输入 → 空返回；单根K线 → 原样返回
10. 输出 `merged_to_raw: list[tuple[int, int]]`

**代码实现验证** (`a_inclusion.py`):

| # | 约束 | 代码位置 | 状态 |
|---|------|---------|------|
| 1 | 包含判定 `>=`/`<=` | L33-35: `last_h >= curr_h and last_l <= curr_l` | ✅ 一致 |
| 2 | 方向由无包含对决定 | L48-51: else 分支更新 dir_state | ✅ 一致 |
| 3 | 向上合并 max/max | L40-41 | ✅ 一致 |
| 4 | 向下合并 min/min | L43-44 | ✅ 一致 |
| 5 | OHLC 处理 | L23(open=opens[0]), L45(close=closes[i]), L52(open=opens[i]) | ✅ 一致 |
| 6 | 初始方向 None → UP | L26: `dir_state = None`, L38: `effective_up = dir_state != "DOWN"` — None != "DOWN" → True → UP | ✅ 一致 |
| 7 | 方向判定双条件 | L48-51: `curr_h > last_h and curr_l > last_l` → UP; 严格不等 `>`/`<` | ⚠️ 见下 |
| 8 | 先左后右递推 | L28: `for i in range(1, n)` 顺序遍历 | ✅ 一致 |
| 9 | 边界: 空/单根 | L82-85 | ✅ 一致 |
| 10 | merged_to_raw 输出 | L61-63 | ✅ 一致 |

**未声明行为**:
无。代码逻辑与定义文件精确对应。

**语义偏差**:
1. **方向判定等号问题** — 定义文件 §"合并方向" 明确记录: 原文用 `>=`/`<=`，代码用 `>`/`<`。代码 L48 `curr_h > last_h` 使用严格不等式。定义文件将此列为 "未结算问题 #2"，已知差异，属于已记录的开放问题——**不是遗漏，是有意识的待决项**。
2. **单条件 vs 双条件** — 定义文件 §"未结算问题 #1" 明确记录两者在非包含前提下数学等价但概念路径不同。代码使用双条件，定义文件已声明此为开放问题。

**测试覆盖**: 10/10 关键约束已覆盖
- `test_inclusion.py`: 29个测试覆盖包含判定、方向切换、向上/向下合并、链式合并、dir=None 默认UP、空输入、单根、OHLC 惯例、raw_map 完整性/单调性/覆盖性、无残留包含断言集成。

---

## 2. 分型 fenxing (v1.0)

### 一致性: ✅ 完全一致

**定义约束清单**:
1. 顶分型双条件: `K[i].high > K[i-1].high AND K[i].high > K[i+1].high AND K[i].low > K[i-1].low AND K[i].low > K[i+1].low`
2. 底分型双条件: `K[i].low < K[i-1].low AND K[i].low < K[i+1].low AND K[i].high < K[i-1].high AND K[i].high < K[i+1].high`
3. 极值价: 顶分型 = K[i].high；底分型 = K[i].low
4. 必须在包含处理后序列上判定
5. 严格大于/小于（不含等号）
6. 首尾K线不可能是分型
7. 至少需要3根MergedBar
8. 数据结构: Fractal(idx, kind, price) frozen dataclass
9. 不做去重/过滤——留给笔构造步骤

**代码实现验证** (`a_fractal.py`):

| # | 约束 | 代码位置 | 状态 |
|---|------|---------|------|
| 1 | 顶分型双条件 | L42-45: `h_curr > h_prev and h_curr > h_next and l_curr > l_prev and l_curr > l_next` | ✅ 精确一致 |
| 2 | 底分型双条件 | L47-50: `l_curr < l_prev and l_curr < l_next and h_curr < h_prev and h_curr < h_next` | ✅ 精确一致 |
| 3 | 极值价 | L46: `price=float(h_curr)` (top); L51: `price=float(l_curr)` (bottom) | ✅ 一致 |
| 4 | 在merged序列上 | 函数签名 `fractals_from_merged(df_merged)` 明确前置条件 | ✅ 一致 |
| 5 | 严格不等式 | `>` 和 `<`，不含 `>=`/`<=` | ✅ 一致 |
| 6 | 首尾不成分型 | L65: `for i in range(1, n - 1)` | ✅ 一致 |
| 7 | 至少3根 | L58-59: `if n < 3: return []` | ✅ 一致 |
| 8 | Fractal dataclass | L17-33: `@dataclass(frozen=True, slots=True)` with idx/kind/price | ✅ 一致 |
| 9 | 不做去重 | 函数只返回所有满足条件的分型，无过滤逻辑 | ✅ 一致 |

**缺口列表**: 无

**未声明行为**: 无

**语义偏差**: 无

**测试覆盖**: 9/9 关键约束已覆盖
- `test_fractal.py`: 12个测试覆盖顶分型、底分型、首尾排除、少于3根、严格单调无分型、多分型排序、断言集成（正/反例）。

---

## 3. 笔 bi (v1.4)

### 一致性: ✅ 完全一致

**定义约束清单**:
1. 上升笔: 底→顶（direction="up"）; 下降笔: 顶→底（direction="down"）
2. 条件1: 异性分型（同性不成笔）
3. 条件2-旧笔: merged gap >= 4
4. 条件2-新笔: merged gap >= 2 AND raw gap >= 3（原始K线计数，不考虑包含关系）
5. 条件3: 方向有效性 — up: `cand.price > start.price`; down: `cand.price < start.price`
6. 分型去重: 同类相邻保留更极端者
7. 强制顶底交替
8. 锁定规则: 已成笔终点不可被替换；遇同类更极端分型延伸当前笔
9. 最后一笔 confirmed=False
10. 连续性: `strokes[i].i1 == strokes[i+1].i0`
11. Stroke 数据结构: (i0, i1, direction, high, low, p0, p1, confirmed) frozen
12. 新笔实现路径: `merged_to_raw[cand.idx][0] - merged_to_raw[start.idx][1] - 1 >= 3`

**代码实现验证** (`a_stroke.py`):

| # | 约束 | 代码位置 | 状态 |
|---|------|---------|------|
| 1 | 方向语义 | L164-166: `_validate_direction` bottom+top→"up", 否则→"down" | ✅ 一致 |
| 2 | 异性分型 | L227-233: `if cand.kind == start.kind` → 走同类处理路径 | ✅ 一致 |
| 3 | 旧笔 gap>=4 | L217: `min_gap = 4 if mode in ("wide", "new")`, L157: `return merged_gap >= min_gap` | ✅ 一致 |
| 4 | 新笔双条件 | L154-156: `merged_gap >= 2 and raw_gap >= 3`; raw_gap计算= `merged_to_raw[cand.idx][0] - merged_to_raw[start.idx][1] - 1` | ✅ 精确匹配定义 |
| 5 | 方向有效性 | L164-166: `cand.price > start.price` (up), `cand.price < start.price` (down) | ✅ 一致 |
| 6 | 分型去重 | L62-84: `dedupe_fractals` — top保留更高price, bottom保留更低price | ✅ 一致 |
| 7 | 强制交替 | L91-110: `enforce_alternation` | ✅ 一致 |
| 8 | 锁定/延伸 | L225-233: `locked` flag + `_extend_prev_stroke` | ✅ 一致 |
| 9 | 最后一笔 confirmed=False | L187-197: `_mark_last_unconfirmed` | ✅ 一致 |
| 10 | 连续性 | L244-246: `i = j` 后 `j += 1` — start 自然传递保证首尾相连 | ✅ 一致（测试验证） |
| 11 | Stroke dataclass | L23-55: `@dataclass(frozen=True, slots=True)` | ✅ 一致 |
| 12 | 新笔实现路径 | L155: 精确匹配定义中的公式 | ✅ 一致 |

**缺口列表**: 无

**未声明行为**:
1. `mode="new"` 但未传 `merged_to_raw` 时回退到旧笔逻辑 (L216: `use_new_bi = mode == "new" and merged_to_raw is not None`)。定义文件未明确声明此回退行为，但不影响正确性，测试已覆盖。

**语义偏差**: 无

**测试覆盖**: 12/12 关键约束已覆盖
- `test_stroke.py`: 约30个测试覆盖去重、交替、宽笔gap=4/3、严笔gap=5/4、confirmed语义（1/2/3笔）、方向交替、连续性、p0/p1正确性、新笔模式（旧笔拒绝但新笔接受、新笔也拒绝insufficient raw gap、条件1共用K线拒绝、无merged_to_raw回退）、断言集成。

---

## 4. 线段 xianduan (v1.3)

### 一致性: ⚠️ 部分偏差

**定义约束清单**:
1. 至少三笔组成，前三笔必须有重叠部分
2. 线段被破坏 ↔ 被另一线段破坏（充要条件）
3. 方向与起始笔一致; 笔数为奇数
4. 硬约束: 顶高于底
5. 特征序列: 向上段取反向笔(X序列)，向下段取反向笔(S序列)
6. 标准特征序列: 对特征序列做包含处理
7. 第一种情况(无缺口): 笔破坏 → 需发展为线段破坏
8. 第二种情况(有缺口): 从分型中心开始构建第二特征序列，出分型即终结
9. 第67课: 第二特征序列不再分第一/二种情况
10. 第71课: 包含关系限于同一特征序列内
11. 结算锚: 新段前三笔必须重叠
12. Segment 数据结构含 break_evidence
13. 最后一段 confirmed=False

**代码实现验证** (`a_segment_v1.py`):

| # | 约束 | 代码位置 | 状态 |
|---|------|---------|------|
| 1 | 至少三笔+前三笔重叠 | L33-46: `_three_stroke_overlap` + `_find_overlap_start` | ✅ 一致 |
| 2 | 线段破坏充要条件 | L296-329: `_try_trigger_segment` 中结算锚验证（新段也必须三笔重叠） | ✅ 一致 |
| 3 | 方向与起始笔一致 | L413: `seg_dir = strokes[seg_start].direction` | ✅ 一致 |
| 3b | 笔数奇数 | 未显式检查 | ⚠️ 见下 |
| 4 | 顶高于底硬约束 | 未显式检查 | ⚠️ 见下 |
| 5 | 特征序列构造 | L420: `opposite = "down" if seg_dir == "up" else "up"` 取反向笔 | ✅ 一致 |
| 6 | 标准特征序列(包含处理) | L196-225: `_FeatureSeqState.append` 做包含处理 | ✅ 一致 |
| 7 | 第一种(无缺口) | L148-164: `_is_fractal_and_gap` 返回 `(is_fractal, has_gap)`; has_gap=False 时直接触发 | ✅ 一致 |
| 8 | 第二种(有缺口)+第二特征序列 | L228-251: `_second_seq_has_fractal` 从分型中心后收集同向笔，独立包含处理 | ✅ 一致(v1.2修复) |
| 9 | 第二特征序列不分一/二种情况 | L251: `_has_any_fractal(elements)` 检查任意分型即可 | ✅ 一致 |
| 10 | 第71课包含作用域 | L435: `feat.reset(seg_dir)` 每段重置特征序列 → 隐式隔离 | ✅ 一致(隐式规避) |
| 11 | 结算锚 | L318-320: `_three_stroke_overlap(strokes[k], strokes[k+1], strokes[k+2])` | ✅ 一致 |
| 12 | break_evidence | L324-328: `BreakEvidence(trigger_stroke_k, fractal_abc, gap_type)` | ✅ 一致 |
| 13 | 最后一段 confirmed=False | L369-376: `_ensure_last_unconfirmed` | ✅ 一致 |

**缺口列表**:
1. **[定义 §3b: 笔数奇数]** → 代码中未显式验证线段包含笔数为奇数。定义文件(第77课)明确"线段中包含笔的数目，都是单数的"。代码通过特征序列分型触发断段，间接保证了奇数性（反向笔构成特征序列，分型需要3个元素 → 至少5笔 → 段终点在同向笔 → 奇数），但没有显式 assert。影响评估: **低风险**——逻辑上隐式保证，但缺少显式防护。
2. **[定义 §4: 顶高于底硬约束]** → 代码中未对产出的线段做"顶高于底"验证。定义文件(第78课)明确"顶肯定要高于底"是定义的一部分。影响评估: **中等风险**——特征序列法正确执行时理论上不会产生顶低于底的线段，但缺少显式防护/断言。

**未声明行为**:
1. `TAIL_WINDOW = 7` (L171) — 尾窗扫描大小限制。定义文件未提及此工程参数。这是一个性能优化选择，限制分型检测只在最近7个元素内进行。**需要确认**: 如果特征序列超过7个元素且分型出现在更早位置，是否会被遗漏？
2. `_skip_until_stroke` 机制 (L180, L188-194) — 当触发被拒绝后标记跳过，防止反复触发同一分型。这是工程防护，定义层无对应。不影响正确性。
3. `_finalize_last_segment` (L332-366) 中的回退逻辑: 当剩余笔数不足 `min_seg_strokes` 时，将剩余笔合并到前一段。定义文件未声明此行为。

**语义偏差**:
1. **第一种情况的处理简化** — 定义文件说"第一种情况(无缺口): 笔破坏 → 需发展为线段破坏"。代码中第一种情况(`has_gap=False`)直接在 `scan_trigger` 返回后由 `_try_trigger_segment` 的结算锚验证（新段前三笔重叠）来间接判定是否"发展为线段破坏"。这是正确的等价实现，但概念路径与原文有差异。

**测试覆盖**: 11/13 关键约束已覆盖
- `test_segment_v1.py`: 约20个测试覆盖无触发单段、顶分型断段、底分型断段、confirmed语义、端点契约、边界条件（少于3笔、无重叠起点）
- `test_segment_gap_classification.py`: 16个测试覆盖缺口分类
- `test_segment_tail_window_scan.py`: 6个测试覆盖尾窗扫描
- `test_segment_v0_vs_v1_comparative.py`: 10个测试覆盖两口径对比
- 未覆盖: 笔数奇数性显式验证、顶高于底显式验证

---

## 5. 中枢 zhongshu (v1.3)

### 一致性: ✅ 完全一致

**定义约束清单**:
1. 至少三个连续已确认线段的价格区间重叠: `ZG > ZD` 严格成立
2. `ZD = max(seg_i.low)`, `ZG = min(seg_i.high)` (前三段)
3. 固定区间: 初始三段确定 [ZD, ZG] 后不变
4. 延伸判定(中心定理一): `[dn, gn] ∩ [ZD, ZG] ≠ ∅` → 弱不等式 `sj.high >= zd and sj.low <= zg`
5. 突破判定: `dn > ZG` 或 `gn < ZD` → 严格不等式
6. 续进: 突破后从 `break_seg_idx - 2` 开始扫描
7. 只处理 confirmed=True 的 Segment
8. 波动区间: GG = max(所有段high), DD = min(所有段low)
9. 突破方向: `breaker.low > zg` → "up"; `breaker.high < zd` → "down"
10. Zhongshu 数据结构含 zd/zg/seg_start/seg_end/seg_count/settled/break_seg/break_direction/gg/dd
11. 笔不裁决: 输入必须是线段，非笔

**代码实现验证** (`a_zhongshu_v1.py`):

| # | 约束 | 代码位置 | 状态 |
|---|------|---------|------|
| 1 | 三段重叠 ZG > ZD | L117: `if zg <= zd: i += 1; continue` | ✅ 严格不等 |
| 2 | ZD/ZG 计算 | L114-115: `zd = max(s1.low, s2.low, s3.low)`, `zg = min(s1.high, s2.high, s3.high)` | ✅ 一致 |
| 3 | 固定区间 | L69-87: `_extend_zhongshu` 只判重叠不改 zd/zg | ✅ 一致 |
| 4 | 延伸弱不等式 | L80: `sj.high >= zd and sj.low <= zg` | ✅ 一致(v1.1修复) |
| 5 | 突破严格不等 | L92: `breaker.low > zg` (up); L94: `breaker.high < zd` (down) | ✅ 一致(v1.1修复) |
| 6 | 续进 | L135: `i = max(break_seg_idx - 2, seg_end_idx)` | ✅ 一致 |
| 7 | 只处理 confirmed | L104: `confirmed = [s for s in segments if s.confirmed]` | ✅ 一致 |
| 8 | GG/DD 波动区间 | L74-75 初始, L83-84 延伸时更新 | ✅ 一致 |
| 9 | 突破方向 | L90-96: `_break_direction` | ✅ 一致 |
| 10 | 数据结构 | L22-64: `@dataclass(frozen=True, slots=True)` 含全部字段 | ✅ 一致 |
| 11 | 笔不裁决 | 函数签名 `zhongshu_from_segments(segments: list[Segment])` | ✅ 一致 |

**缺口列表**: 无

**未声明行为**: 无

**语义偏差**: 无

**测试覆盖**: 11/11 关键约束已覆盖
- `test_zhongshu_overlap_golden.py`: 约30个测试覆盖三段重叠、无重叠、ZG==ZD精确相切、延伸(第四段)、边界精确触及(5个弱不等式验证)、突破+事件序列、突破方向(up/down)、续进形成新中枢、candidate在settle之前(I12)、ZhongshuEngine端到端、未确认段忽略、前端定位字段
- `test_zhongshu_invariants.py`: 13个测试覆盖 I11/I12/I13/I14/I17 不变量正反例
- `test_zhongshu_determinism.py`, `test_zhongshu_invalidate_propagation.py`, `test_zhongshu_identity_skip.py`, `test_zhongshu_level.py`: 额外覆盖

---

## 汇总

| 定义 | 版本 | 一致性 | 缺口数 | 未声明行为 | 语义偏差 | 测试覆盖 |
|------|------|--------|--------|-----------|---------|---------|
| 包含关系 baohan | v1.3 | ⚠️ 部分偏差 | 0 | 0 | 2 (均已记录为未结算问题) | 10/10 |
| 分型 fenxing | v1.0 | ✅ 完全一致 | 0 | 0 | 0 | 9/9 |
| 笔 bi | v1.4 | ✅ 完全一致 | 0 | 1 (回退行为) | 0 | 12/12 |
| 线段 xianduan | v1.3 | ⚠️ 部分偏差 | 2 | 3 | 1 | 11/13 |
| 中枢 zhongshu | v1.3 | ✅ 完全一致 | 0 | 0 | 0 | 11/11 |

### 需要关注的项

**P1 (应修复)**:
1. **线段: 顶高于底硬约束缺失** — 第78课明确"顶肯定要高于底"是定义的一部分。建议在 `_make_segment` 或 `_emit_segment` 中添加 assert/检查。

**P2 (建议补充)**:
2. **线段: 笔数奇数性未显式验证** — 逻辑隐式保证但缺少显式防护。建议在 segment 断言函数中添加。
3. **线段: TAIL_WINDOW=7 的充分性** — 需确认此窗口是否足够覆盖所有合法分型位置。

**P3 (已知开放项，非本次审计范围)**:
4. **包含关系: 等号问题** — 定义文件已记录为未结算问题 #2。
5. **包含关系: 单条件 vs 双条件** — 定义文件已记录为未结算问题 #1。

### 谱系违规检查

| 检查项 | 违规? | 说明 |
|--------|-------|------|
| 005b (非对象否定) | ❌ 无 | 未发现超时/阈值硬编码作为否定来源 |
| 068 (偏序集/浮点阈值) | ❌ 无 | 所有比较使用 `>`/`<`/`>=`/`<=`，无 epsilon 近似 |
| 006 (级别=递归层级) | ❌ 无 | 无时间周期替代级别定义的代码 |
| TAIL_WINDOW=7 | ⚠️ 存疑 | 这是一个数值常量限制扫描范围，虽非阈值判断，但可能影响正确性——需进一步验证 |
