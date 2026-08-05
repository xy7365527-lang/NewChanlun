# #813 线段侧（G-1～G-5）考据报告

**性质**：考据报告，**不含裁定**。裁定由人做。
**基线**：本地 `main` = `88f6f3c09b`（分支 `research/i813-segment` 由此切出）。
**日期**：2026-08-05。
**范围**：只查 #813 票面「线段」侧五组（G-1～G-5）。笔侧 S-1～S-4 由并行子代理处理，本报告不碰。
**只读**：未改任何 `.rs`/`.py`/`.lean`；未改 `.chanlun/definitions/xianduan.md`。

## 0. 基线归正过程（#909 第十四次实证）

开工时 `git log --oneline -1` = `19b4015927`（「谱系484号——multi_tf架构审计」），且
`rust/src/theta_v0/` 与 `formal/` **均不存在** ⟹ 落在祖传线。归正：`git fetch --all` →
`git checkout -B research/i813-segment 88f6f3c09b`。归正后 `formal/Origin/`、
`rust/src/theta_v0/parser/` 均在。**本仓 worktree 初始 HEAD 落祖传线的现象再次复现，
子代理开工必须先核。**

---

## 1. 票面前提核实汇总（先说错在哪）

| # | 票面断言 | 核实结果 |
|---|---|---|
| 1 | `a_segment_v0.py:147` = 三笔重叠法 | **行号偏 1**。`:147` 是空行；`def segments_from_strokes_v0` 在 **`:148`**，三笔重叠主体在 **`:157–166`**。 |
| 2 | `SegmentConstruction.lean:63` = 遇第一根反向笔即切 | **行号对**（`:63` = `def scanSegEnd`）。但**语义描述不完整**：它不是「即切」而是「消费到含第一根反向笔」（`:66` `then acc + 1`），且**在生产笔流上恒等于 2**（见 §2.3）。 |
| 3 | `Pipeline.lean:95` 的 Lean 端到端管线用的就是第三种 | **结论对，行号错**。`:95` 是 `import Origin.SegmentConstruction`，不是管线本体。管线本体 = `Pipeline.lean:136–137` `def pipelineSegments (inp) := segmentsOf inp.strokes`。**实质成立**（见 §2.4）。 |
| 4 | 「406 段 vs 参考 237 段（70% 过分段）」 | **可溯源**，但票面**用错了地方**。见 §3——它测的是**已废弃的旧批处理实现 vs Python v1 参考语义**，单标的（OKLO）单切片（2000 笔），**且已修复**。不是 G-1 三范式的量级差证据。 |
| 5 | G-1「特征序列增量状态机（3 处）」 | 见 §2.2 穷举：**独立实现 3 处**（Python v1 / rust 旧引擎 / rust theta_v0），计数**对**。 |
| 6 | G-2「方向性（4 处）」 | 见 §4 穷举：**方向性载体 4 处**（Python `_merge_feature_bar`、Python `_apply_inclusion`、`feature_seq.rs::apply_inclusion`、`rust/src/segment.rs::apply_inclusion`）+ Lean `mergeInclusion` 1 处 = **5 处**（若把 Lean 算进去）。票面 4 处**在「非 Lean」口径下对**，口径未写明。 |
| 7 | G-2 `segment.rs:196` 自称「中性实现」 | **函数在 `:196`，「中性实现」字样在 `:195`**（doc 注释）。 |
| 8 | G-3 `SegmentAutoConstruct.lean:177`／`:174` 自承「代理」 | **两个行号都对**。`:174` 逐字：「`rest`（分型后剩余特征序列）作为 `revSeq` 代理」；`:177` = `def extractSegEndData`。 |
| 9 | G-3「同向笔新建、任意分型即确认（3 处）」 | 见 §5 穷举：**独立实现 3 处**（`feature_seq.rs:229`、`rust/src/segment.rs:416`、`a_segment_v1.py` `_second_seq_has_fractal`）；`second_kind.rs:103` 是**转发壳**不计。计数**对**。 |
| 10 | G-4「两档并存，默认 strict」 | **对**，但票面把它标「灰区」**低估了后果**：仓内在案读数 `tests/test_rust_segment_equivalence.py:11` 逐字写「**BZ 全年 strict 28 段、optimized 159 段**」——两档差 **≈5.7 倍**。见 §6。另：**Rust theta_v0 侧 `ExtendMode::Optimized` 无生产构造点**（见 §6 可达性）。 |
| 11 | G-5「`MAX_SECOND_SEQ_SCAN=50` ↔ 无限」+「窗口 7→∞ 输出不变」 | **票面把两个不同的窗口混成了一个**。`MAX_SECOND_SEQ_SCAN=50`（第二特征序列扫描窗）与 `TAIL_WINDOW=7`（首特征序列尾窗）是两个常数、两条独立分歧。「7→∞ 输出不变」这句实测**是关于 TAIL_WINDOW 的**（`rust/src/theta_v0/parser/segment.rs:76–78`），不是关于 50 的。且**两档默认相反**：theta_v0 里 `second_seq_scan_window` 默认 **0=无限**，而 `TAIL_WINDOW` **硬编码 7 且不进 config**。见 §7。 |
| 12 | G-1「形式化层跑的不是生产那套」（措辞） | **结论成立但票面措辞过宽，须收窄**。`formal/Phase2/Claim10_SegmentV1.lean` **有** P2（特征序列 v1）的形式化（谓词层），只是**与端到端管线没有连线**（独立 lake root，`lakefile.toml:20`；`Origin/Pipeline.lean` 零 import `Phase2`）。准确病名 = **形式化层裂成两半**：能跑的那条（Origin 管线）判据不对，判据对的那条（Phase2）跑不起来。见 §11.1。 |
| 13 | G-1 的分歧只有「段体多长」 | **票面漏了一条现役分歧**：前三笔重叠的**相切边界**——P1（`a_segment_v0.py:162`）用严格 `<`，P2（`theta_v0/parser/segment.rs:140`）用 `<=`，Lean 两个模块自己也不一致（`Claim10:471-474` 标为「**未结算的口径细节**」，且原文**未明确表态**）。#246/#277/#288 那次相切口径切换**没有覆盖 `a_segment_v0.py`**。见 §11.3。 |
| 14 | G-3 的「对偶 vs 任意分型」只在 Lean Origin 出现 | **票面漏了一处源头**：`Claim10_SegmentV1.lean` 的溯源段**自己就把两种说法并列写了**（`:46` 写「须第二特征序列出**反向分型**」、`:47` 写「**只要有分型即可**」），两句相邻且都挂第67课名下。见 §11.4。 |

---

## 2. G-1 线段的构造范式

### 2.1 P1 — 三笔重叠法（Python v0）

`src/newchan/a_segment_v0.py:148–166`（逐字）：

```python
148:def segments_from_strokes_v0(strokes: list[Stroke]) -> list[Segment]:
149:    """v0 线段构造：三笔交集重叠 -> 生成线段。"""
...
157:    while j <= n - 3:
158:        s1, s2, s3 = strokes[j], strokes[j + 1], strokes[j + 2]
159:        overlap_low = max(s1.low, s2.low, s3.low)
160:        overlap_high = min(s1.high, s2.high, s3.high)
161:
162:        if overlap_low < overlap_high:
163:            segments.append(_build_segment_from_strokes(strokes, j))
164:            j += 2
165:        else:
166:            j += 1
```

段体固定 3 笔（`:130-132` `s1=j+2`、`i0=s1.i0`、`i1=s3.i1`），方向 = 第一笔方向
（`:133 direction=s1.direction`）。命中后 `j += 2` ⟹ **相邻两段共享一根笔**。

### 2.2 P2 — 特征序列增量状态机（穷举：3 处独立实现）

**穷举方法**：`grep -rn "特征序列" --include="*.rs" --include="*.py" --include="*.lean" .`
（46 个文件命中，见附录 A 计数表）→ 逐个打开筛「构造线段的主循环」→ 交叉核
`grep -rn "_apply_inclusion|_merge_feature_bar|mergeInclusion"` +
`grep -rn "segments_from_strokes_v1|divide_segments"`。覆盖目录：`rust/src`、`src/newchan`、
`formal`、`scripts`、`tests`、`analysis`。

1. **Python 正本** `src/newchan/a_segment_v1.py:564` `segments_from_strokes_v1`
   （状态机 `_FeatureSeqState` 在 `:269–433` 区段，`TAIL_WINDOW=7` 在 `:288`，
   `MAX_SECOND_SEQ_SCAN=50` 在 `:367`）。
2. **Rust 旧引擎** `rust/src/segment.rs:728` `segments_from_strokes_v1`
   （+ `:768` `segments_from_strokes_v1_into` 增量版；`TAIL_WINDOW: i64 = 7` 在 `:410`、
   `MAX_SECOND_SEQ_SCAN: usize = 50` 在 `:413`）。
3. **Rust theta_v0** `rust/src/theta_v0/parser/segment.rs:391` `divide_segments_with_tail`
   （+ `:459` `divide_segments` 薄封装；状态机 `FeatureSeqState` 在
   `rust/src/theta_v0/parser/feature_seq.rs:296`）。

`rust/src/theta_v0/parser/segment.rs:374` 自述定位（逐字）：
「★bit-exact 移植 Python `a_segment_v1.segments_from_strokes_v1`（编排者裁定 v1 唯一口径，
37 测试）。L1（对参考 spec 忠实，认证 harness `analysis/segment_refsem_cert.py`），**非**
对 Origin Lean bit-exact——动态划分算法在 Origin 有意不形式化」。

⟹ **P2 的三处是「同一口径的三份移植」，不是三套分歧判据**。跨语言逐位等价有 golden 测试
`tests/test_rust_segment_equivalence.py`（L1 合成 + L2 BZ Brent 真实 1min）。

### 2.3 P3 — Lean `segmentsOf`（★关键推论：在生产笔流上恒切 2 笔）

`formal/Origin/SegmentConstruction.lean` 逐字：

```lean
56:def strokeReverses (segDir : Direction) (s : Stroke) : Bool :=
57:  s.direction == segDir.flip
...
63:def scanSegEnd (segDir : Direction) : List Stroke → Nat → Nat
64:  | [], acc => acc
65:  | t :: ts, acc =>
66:      if strokeReverses segDir t then acc + 1
67:      else scanSegEnd segDir ts (acc + 1)
...
77:def nextSegmentEnd : List Stroke → Nat
78:  | [] => 0
79:  | [_] => 1
80:  | s :: rest => scanSegEnd s.direction rest 1
...
165:def segmentsOf (strokes : List Stroke) : List Segment :=
...
169:      let k := nextSegmentEnd (s :: rest)
170:      let consumed := (s :: rest).take k
171:      let seg := segmentOfStrokes consumed
172:      seg :: segmentsOf ((s :: rest).drop k)
```

**推论（`[新缠论:推论]`，推理链如下）**：

1. `formal/Origin/ChanlunElements.lean:35–41` 的 `structure Stroke` **只有五个字段**
   （direction/startIndex/endIndex/startPrice/endPrice），**无相邻笔方向交替的不变量**。
2. 生产侧笔流**严格交替**：`rust/src/theta_v0/parser/stroke.rs:30` 「结果是顶/底**严格交替**
   的分型序列（相邻必异类）」，`:70` 「方向：底→顶=Up，顶→底=Down」，并有 property 测试锁
   `stroke.rs:457–459` `property_adjacent_strokes_alternate_direction`。
3. 交替 ⟹ 对任意 `s :: t :: _`，`t.direction = s.direction.flip` ⟹
   `strokeReverses s.direction t = true` ⟹ `scanSegEnd s.direction (t::ts) 1 = 2`。
4. ⟹ **`nextSegmentEnd` 在生产笔流上恒 = 2**（列长 ≥2 时），`segmentsOf` **把笔流每 2 笔切一段**。

该文件自己的计算见证印证此点 —— `:213` 逐字注释：「三根笔：上(10→20) 下(20→15) 上(15→25)。
第一段上，**第二根反转处切**」。

**⟹ P3 在生产输入域上不是「一种线段判据」，而是「每两笔一段」的定长切分。**
`:239–246` 的 still-MISSING-A′ 已诚实标明它「**不**等于 SegmentFeatureSeq.lean 的完整判据驱动
切分」，`:245` 逐字「**本文件不声称**切出的线段真对应缠论权威标注」。

### 2.4 Lean 端到端管线用的是哪一套（★票面第 3 条独立核，不看注释自述）

**逐条比对**（不采信注释）：

- `formal/Origin/Pipeline.lean:95` = `import Origin.SegmentConstruction`（票面指的是这行 ⟹ **不是管线本体**）。
- `Pipeline.lean:136–137`（逐字）：
  ```lean
  136:def pipelineSegments (inp : OriginInput) : List Segment :=
  137:  segmentsOf inp.strokes
  ```
- `segmentsOf` 的唯一定义点 = `SegmentConstruction.lean:165`（`grep -rn "def segmentsOf" formal/` 唯一命中）。
- `Pipeline.lean:82–84` 的依赖声明逐字：「★**不** import #122 在改的 SegmentFeatureComplete /
  #124 的 ForceInterface（避冲突；**用构造层不用判据层完整版**）」。

⟹ **票面这条实质成立**：Lean 端到端管线走 `SegmentConstruction.segmentsOf`（= P3），
**没有**接 `SegmentFeatureSeq`/`SegmentFeatureComplete` 的完整判据，也**没有**接
`SegmentAutoConstruct` 的完整判据扫描。**形式化层跑的确实不是生产那套线段定义**，
而且按 §2.3，它跑的是**每两笔一段**。

**★额外查出一处内部矛盾（票面未提）**：`rust/src/theta_v0/parser/segment.rs:384` 声称
confirmed 段「对齐 `Origin.SegmentConstruction.segmentsOf` 输出」，而**同一 doc 注释的
`:376–377` 明写「非对 Origin Lean bit-exact」**。按 §2.3 的推论，`:384` 那句「对齐」**为假**
（P2 段长可变且 ≥3 笔，P3 恒 2 笔）。这是同一段注释内的自相矛盾，建议随本票订正。

### 2.5 ★三套构造范式的接受集关系（推出来的，不照抄票面）

**接受集定义**：固定输入笔列 `S`（严格交替，默认 config），一个实现的接受集 =
它产出的「段 = 笔下标闭区间 `[a,b]`」的集合。

**约束读出（点名到行）**：

| | 段体笔数 | 保证它的那一行 |
|---|---|---|
| P1 | **恒 3** | `a_segment_v0.py:158`（取 `strokes[j..j+2]`）+ `:130` `s1=j+2` |
| P2 | **≥3**（默认） | `rust/.../segment.rs:436` `end_stroke - seg_start < min_seg.saturating_sub(1)` ⟹ 拒；`min_seg = config.seg_min_strokes`，默认 3（`config.rs:64`） |
| P3 | **恒 2**（生产输入域） | §2.3 推论，锚 `SegmentConstruction.lean:57/66` + `stroke.rs:457` |

⟹ **接受集关系（不是「三种不同」这种无信息量的说法）**：

- **P1 ∩ P3 = ∅**、**P2 ∩ P3 = ∅**（默认 `seg_min_strokes=3`）——**P3 与另两套完全不相交**。
  一个段不可能同时是 2 笔与 ≥3 笔。
- **P1 与 P2 互不包含且交非空**（proper overlap）：
  - `P1 \ P2 ≠ ∅`：三笔重叠成立但特征序列无分型确认的位置，P1 出段、P2 不出。
  - `P2 \ P1 ≠ ∅`：P2 能出 4/5/7 笔段，P1 恒 3 笔，永不覆盖。
  - `P1 ∩ P2 ≠ ∅`：三笔段两边都认的位置。
- **不是 #815 那种严格包含链**；**也不是票面暗示的对称「互不包含」**——结构是
  「**一支孤立（P3）＋两支交叉（P1/P2）**」。
- **边界条件（结论翻转点）**：若 `seg_min_strokes` 被设为 2，`P2 ∩ P3` 可非空；
  若输入笔列**不**严格交替（Lean 类型允许，生产不产生），P3 的段长可 >2，
  与 P1/P2 的不相交结论翻转。

**落在差集里的具体输入（手算样例）**——五根交替笔：

| 笔 | 方向 | 起→止 | [low, high] |
|---|---|---|---|
| s0 | up | 10→20 | [10,20] |
| s1 | down | 20→15 | [15,20] |
| s2 | up | 15→25 | [15,25] |
| s3 | down | 25→18 | [18,25] |
| s4 | up | 18→30 | [18,30] |

- **P1**：j=0 → `overlap_low=max(10,15,15)=15`，`overlap_high=min(20,20,25)=20`，`15<20` ✓
  ⟹ 出段 `[0,2]`，`j=2`；j=2 → `max(15,18,18)=18 < min(25,25,30)=25` ✓ ⟹ 出段 `[2,4]`，`j=4` 退出。
  **P1 = { [0,2], [2,4] }**（2 段，**都标 up，且共享 s2**）。
- **P3**：`nextSegmentEnd = 2` ⟹ 出 `[0,1]`；drop 2 ⟹ `nextSegmentEnd=2` ⟹ 出 `[2,3]`；
  drop 2 ⟹ `[s4]` 单元素 ⟹ `nextSegmentEnd=1` ⟹ 出 `[4,4]`。
  **P3 = { [0,1], [2,3], [4,4] }**（3 段，**全标 up**，方向 = 首笔方向）。
- ⟹ `[0,2] ∈ P1 \ P3`、`[0,1] ∈ P3 \ P1`，**交集空**。手算见证成立。
- **顺带查出的两个共病**：本样例上 P1 与 P3 **都产出连续同向段**（全 up），
  违反「线段必上下交替」。这不在票面九组里，但两支都踩，值得单列。

---

## 3. ★「406 段 vs 参考 237 段（70% 过分段）」溯源

**溯源结果：可溯源，出处两处，但票面用错了论证位置。**

出处一 —— `rust/src/theta_v0/parser/segment.rs:379–381`（逐字）：

```
379:/// ★为何增量（cc-refsem-harness 归因，#84）：旧批处理「找首个分型即断段」对参考认证失败
380:/// （406 段 vs 参考 237 段，70% 过分段）。真因是缺第71课「假设转折点」逻辑（包含时先试不合并
381:/// 看是否触发分型）——批处理无状态，无法表达此动态过程。本函数用 `FeatureSeqState` 状态机修复。
```

出处二 —— `analysis/segment_refsem_cert.py:7–8`（逐字）：

```
 7:本 harness 在真实数据切片（OKLO 2000 笔）上做差分认证，并隔离三个候选分歧变量的贡献，
 8:定位 theta_v0=406 段 vs 参考=237 段（70% 过分段）的真因。
```

**读数口径（五项，按纪律逐项写清）**：

1. **点估计还是下界**：**单次点估计**（段数直接计数），无区间、无方法名。
2. **几个独立窗**：**1 个**。单标的 **OKLO**，单切片 **2000 笔**（不是 2000 bar）。
   `analysis/segment_refsem_cert.py:7` 是唯一口径来源。
3. **每窗还是合计**：单窗即合计（只有一窗）。
4. **覆盖多少总体**：**未知/未声明**。2000 笔切片相对 OKLO 全历史的占比、
   相对多标的池的占比，出处均未写。
5. **问的是哪一级**：**level 0**（笔 → 线段），本级别，无递归上钻。

**⟹ 三条限定，票面沿用时都丢了**：

- **(a) 比较的两边不是 G-1 的三范式**。406 = **theta_v0 的旧批处理线段实现**（已废弃）；
  237 = **Python `a_segment_v1.segments_from_strokes_v1` 参考语义**（= P2 的 Python 正本）。
  ⟹ 这是「**P2 的一个 buggy 移植 vs P2 的正本**」，**不是**
  「三笔重叠 vs 特征序列 vs 遇反向笔即切」。**票面把它当 G-1 三范式的量级差证据 = 误用。**
- **(b) 已修复**。`segment.rs:381` 逐字「本函数用 `FeatureSeqState` 状态机修复」。
  当前 `divide_segments_with_tail` **不是**那个 406 的实现。⟹ 这个读数**不描述现役任何一套**。
- **(c) 「参考」不是缠论权威标注**。`analysis/segment_refsem_cert.py:3–5` 逐字：
  「教义（codex ReferenceSemantics 裁决）：引擎不是定义。参考语义 Spec_seg = Python …」
  ⟹ 参考 = Python 实现，不是原文。`:13` 逐字「报差分计数（X 段 theta_v0 vs Y 段参考），
  **不声称策略盈利**（那是 L2 回测）」。

**顺带溯到 237 的第二次出现**（同一份 harness，另一个实验）——
`rust/src/theta_v0/parser/segment.rs:76–78`（逐字）：

```
76:/// ★L2 实测（OKLO 2000 笔，analysis/segment_refsem_cert.py）：窗口从 7 扩到 ∞ 输出不变
77:/// （237→237），证 7 在目标数据域非约束性（第二特征序列分型若有效必在窗口内）。保留 7 以
78:/// bit-exact 对齐参考默认；改值 = 改 Θ。这是单数据集 L2 观察，**非** L0 结构证明（不裸剥）。
```

⟹ **237 = 同一 OKLO 2000 笔切片上的参考段数**，两个读数同源同切片。这也定位了
G-5「窗口 7→∞ 输出不变」的出处（见 §7）。

**未复现声明**：本报告**未**重跑 `analysis/segment_refsem_cert.py`（需 OKLO 数据集，
只读工序不跑回测）。上述为**出处考据**，非复现验收。

### 3.1 ★读数的三条补充限定（第二次交付补入，比 §3 更硬）

1. **出处 harness 已被仓内自己判退役**。`analysis/_attrib_segment_divergence.py:17–19` 逐字：
   「## ⚠退役登记（#317 MED-1；#302 影子评审，chanlun/review-results/shadow-review-288-20260726.md）
   **本 harness 已退役：不得作为任何口径的参考基线引用，下方数字是历史记录。**」
   `.chanlun/review-results/issue789-multi-impl-census-20260730.md:191–194` 逐字：
   「`analysis/segment_refsem_cert.py`（线段参考语义认证 harness）**事实上已停用**：最近提交
   2026-06-26，依赖的 `_xcheck_oklo_strokes.json` 被 gitignore、快照内不存在，不在 pytest 收集路径。」
2. **本仓无可执行路径能重算出这两个数字**。`analysis/data_cache/_xcheck_oklo_strokes.json`
   工作树不存在，`git log --all --` 该路径**无任何历史**（从未入库）。
   ⟹ **读数可溯源到行，但不可复现。**
3. **70% vs 71% 仓内自相矛盾**：`(406−237)/237 = 71.3%`。`segment_refsem_cert.py:8` 与
   `feature_seq.rs:20` 写 70%；`_attrib_segment_divergence.py:3` 与产出它的 commit
   `ad87acc657`（2026-06-26）的 message 写 71%。票面沿用的是 70% 这一支。
4. **与关票落点文档自相矛盾**：`.chanlun/definitions/xianduan.md:199` 逐字
   「**已验证**：v0贪心段数是v1的2-3倍」——2–3 倍（+100%~+200%）与 406/237≈1.71 倍（+71%）
   **不是同一量级**，且引用的是另一个（仓内可跑的）测试
   `tests/test_segment_v0_vs_v1_comparative.py`。**票面未披露此矛盾。**

**⟹ 对 G-1 的净效果**：这个读数**不能**用来支持「三范式量级差」，它既不测三范式、
又出自已退役且不可复现的 harness、又与正本文档另一处读数打架。**建议裁定时不引用它**；
若要一个「v0 vs v1 量级差」的数字，仓内可跑的是
`tests/test_segment_v0_vs_v1_comparative.py`（合成数据，10/10 通过，口径「2–3 倍」）。

### 3.2 `feature_seq.rs:18–22` 另有一条被票面丢掉的归因（★重要）

`rust/src/theta_v0/parser/feature_seq.rs:18–22` 逐字（经溯源子代理引出）：

```
★为何重写（cc-refsem-harness 归因，#84）：旧 `segment.rs::divide_segments` 用「无状态
批处理找首个分型即断段」，对参考语义认证**失败**（真实数据 406 段 vs 参考 237 段，70%
过分段，前 237 段仅 2 段 bit-exact 一致）。归因证伪「inclusion 是根因」（外包络→方向性
仅 Δ=2 段，1.2%）——真因是**算法范式整体差异**……
```

⟹ **这条直接回答 G-2**：把包含处理从**外包络换成方向性**，在 OKLO 2000 笔上
**只差 Δ=2 段（1.2%）**。**票面把 G-2 列为一组独立分歧时，没有引这个已在案的量级读数。**
（同样的限定：单标的单切片、点估计、harness 已退役不可复现。）

---

## 4. G-2 特征序列包含处理是不是方向性的

### 4.1 载体穷举

**穷举方法**：`grep -rn "_apply_inclusion|_merge_feature_bar|mergeInclusion" --include="*.py"
--include="*.lean" --include="*.rs" .`（排除 `rust/target`）+
`grep -rn "特征序列" --include="*.rs" --include="*.py" --include="*.lean" .` 逐文件筛
+ 对每个命中点打开确认合并公式。覆盖目录：`rust/src`、`src/newchan`、`formal`、`tests`、
`scripts`、`analysis`、`topological-computation`。

**方向性合并（5 处载体，其中非 Lean 4 处）**：

| # | 位置 | 合并公式 | 方向由谁定 |
|---|---|---|---|
| 1 | `src/newchan/a_feature_sequence.py:79–96` `_merge_feature_bar` | 上：`max/max`（`:92-93`）；下：`min/min`（`:95-96`） | 局部 `dir_state` |
| 2 | `src/newchan/a_segment_v1.py:201` `_apply_inclusion` | 同上 | 局部 `dir_state` |
| 3 | `rust/src/theta_v0/parser/feature_seq.rs:186–206` `apply_inclusion` | 上 `max/max`（`:201-202`）；下 `min/min`（`:204-205`） | 局部 `dir_state`（`:352-356` 风格更新，见 `:207-217`） |
| 4 | `rust/src/segment.rs:327–359` `apply_inclusion`（f64 旧引擎） | 上 `max/max`（`:342-343`）；下 `min/min`（`:345-346`） | 局部 `dir_state`（`:352-356` 更新） |
| 5 | `formal/Origin/SegmentFeatureSeq.lean:153–167` `mergeInclusion` | `processUp` → `[较大 low, 较大 high]`；否则 `[较小 low, 较小 high]` | **全局固定 = 线段方向**（`SegmentAutoConstruct.lean:87` `buildFeatureSeqAux (segDir == Direction.up) [] elems`） |

⟹ **票面「方向性（4 处）」在「不算 Lean」的口径下正确；口径未写明。含 Lean 是 5 处。**

**中性外包络（1 处载体）**：`rust/src/theta_v0/parser/segment.rs:196–209`
`process_feature_inclusion`，逐字：

```rust
196:pub(super) fn process_feature_inclusion(elements: &[Interval]) -> Vec<Interval> {
...
200:            Some(prev) if prev.contains(&e) || e.contains(prev) => {
201:                // 包含 → 合并为外包络并区间（第67课:20 当 K 线做包含处理）。
202:                prev.lo = prev.lo.min(e.lo);
203:                prev.hi = prev.hi.max(e.hi);
```

「中性实现」字样在 **`:195`**（doc 注释，非 `:196`）：「简化为：对相邻包含元素，合并为
`[min(lo), max(hi)]` 的并区间——这是「元素当 K 线包含处理」的**中性实现**（保留外包络），
与 `Origin.SegmentFeatureSeq.Contains` 判定配套。」

### 4.2 ★「中性实现」自述当线索：与方向性差多少（不恒等，给差异条件）

**不恒等，且仓内已有在案裁定与反例**。`rust/src/theta_v0/parser/second_kind.rs:90–95`
逐字（这是本报告最强的一条在案证据）：

```
★BUG-08 修复：包含处理复用 `feature_seq::second_seq_has_fractal`（bit-exact 对齐 Python
`_apply_inclusion` 的方向性合并：向上取 max/max、向下取 min/min，初始 dir_state 同
Python `seg_dir==up ⟹ DOWN` 语义）。原实现复用 `segment::process_feature_inclusion`
的中性外包络 `[min(lo),max(hi)]`——那是 Origin 静态层的配套实现，与本文件宣称的
Python bit-exact 交叉验证不是同一算法（`[10,20]` 含 `[12,18]` 向上应得 `[12,20]`，
外包络仍得 `[10,20]`，会增删后续分型）。
```

⟹ **仓内自己已经认定二者不是同一算法，并已改掉一处误用（BUG-08）。**
`[10,20]` 含 `[12,18]`：向上 `[12,20]`、向下 `[10,18]`、外包络 `[10,20]` —— **三值互不相同**。

**差异条件（我推的，比在案反例更强一层）**：外包络与方向性合并的差异**不只是单元素高低值
改变，还会改变标准特征序列的元素个数**，因而**改变分型的存在性**。手算样例（向上线段，
特征序列元素依次给出）：

| 元素 | [lo, hi] |
|---|---|
| e0 | [10, 20] |
| e1 | [12, 18] |
| e2 | [11, 19] |
| e3 | [5, 13] |

- **方向性（up: max/max）**：e0⊃e1 ⟹ 合并 `[max(10,12), max(20,18)]=[12,20]`。
  e2=[11,19] 与 [12,20] **互不包含**（12>11、19<20）⟹ push。e3 与 [11,19] 互不包含 ⟹ push。
  **标准序列 = [12,20], [11,19], [5,13]（3 个元素）** ⟹ 可判分型（此例中恰无分型，但**门槛已过**）。
- **中性外包络**：e0⊃e1 ⟹ `[min(10,12), max(20,18)]=[10,20]`（**等于 e0，未变**）。
  e2=[11,19] 被 [10,20] **包含** ⟹ 再合并仍得 [10,20]。e3 ⟹ push。
  **标准序列 = [10,20], [5,13]（2 个元素）** ⟹ `find_feature_fractal`（`segment.rs:221-223`
  `if std_feat.len() < 3 { return None }`）**直接返回无分型**。

⟹ **差异条件**：当前元素被前一元素包含、且外包络吞并后仍能继续吞并后续元素时，
外包络会**少产出元素**，进而**抹掉本该存在的分型判定机会**。这是结构性差异，**不恒等**。

### 4.3 与 `parser/inclusion.rs` 地基的关系（票面点明要先读它）

`rust/src/theta_v0/parser/inclusion.rs:49–55` 的 K 线合并逐字：

```rust
49:fn merge(acc: &Bar, b: &Bar, dir: MergeDir) -> Bar {
50:    let (high, low) = match dir {
52:        Direction::Up => (acc.high.max(b.high), acc.low.max(b.low)),
54:        Direction::Down => (acc.high.min(b.high), acc.low.min(b.low)),
```

⟹ **K 线层的包含处理是 `max/max` / `min/min`，与特征序列层的方向性实现（§4.1 的 1–4）
公式一致**；**外包络 `[min lo, max hi]` 与 K 线层的两种合并都不同**。
⟹ 「元素当 K 线做包含处理」这句原文（`067:20`【正文】）如果照字面执行，**得到的是方向性合并，
不是外包络**。`segment.rs:195` 的「中性实现」自述在这一点上**与它自己引的原文出处相悖**。

**★ 但方向性还分两种，票面完全没提**：
- **局部方向**（Python/Rust，§4.1 的 1–4）：`dir_state` 初值按段方向定，随后**每遇一对
  非包含且严格单调的元素就更新**（`rust/src/segment.rs:352–356`）。
- **全局方向**（Lean，§4.1 的 5）：`processUp` 由**线段方向一次定死**，
  `buildFeatureSeqAux`（`SegmentAutoConstruct.lean:69–81`）全程不更新。

⟹ **「方向性」不是一个选项，是两个**。第一次出现「非包含且严格单调向下」的相邻对之后，
Lean 与 Python/Rust 就分岔。**这是 G-2 里第三条分歧线，票面漏了。**

### 4.4 可达性（本条依 Notes N-1 预授权判定）

**中性外包络 `process_feature_inclusion` 在生产路径上不被消费。**

- 唯一消费者 = `rust/src/theta_v0/parser/segment.rs:287`
  （在 `analyze_termination`，`:281` 定义）。
- **`analyze_termination` 无生产调用方**。穷举：`grep -rn "analyze_termination" rust/ --include="*.rs"`
  = **7 处命中，全部在 `rust/src/theta_v0/parser/segment.rs` 内**（`:281` 定义 + `:283/:289/:299/
  :305-306/:311/:318-321` 函数体 + `:948`/`:959` 两个单元测试）。
- **未再导出**：`rust/src/lib.rs`、`rust/src/theta_v0/mod.rs`、`rust/src/theta_v0/parser/mod.rs`
  三处均**零命中** `analyze_termination` / `SegmentTermination`。
- 生产线段入口 = `divide_segments_with_tail`（`:391`）→ 走
  `FeatureSeqState`（`feature_seq.rs:296`，方向性 `apply_inclusion`），**不经** `analyze_termination`。

⟹ **G-2 的「中性 vs 方向性」在现役生产输出上不产生任何差异**——不是「输入产生不出来」，
而是**中性那条臂的载体本身不在生产调用图里**（测试可达、生产不可达）。
**保证它的那一行**：`segment.rs:287` 是 `process_feature_inclusion` 的唯一调用点，
而其宿主 `analyze_termination`（`:281`）在 `rust/` 内无外部调用方（上述穷举）。

**⚠️ 三条不能省的限定**：
1. `analyze_termination` 是 `pub`（`:281`），crate 外可调用；`process_feature_inclusion` 是
   `pub(super)`（`:196`），只 parser 模块内可见。**「不可达」是「当前仓内无调用方」，
   不是「语言层不可达」。**
2. `analyze_termination` 的 doc（`:275`）声称它对齐 `Origin.SegmentFeatureComplete` 静态层
   ⟹ 它是**对拍/契约锚**用途。删它 = 删掉与 Lean 静态层的唯一对应物。
3. **在案已修过一次误用**（BUG-08，`second_kind.rs:90–95`）：`second_kind_confirmed` 曾经
   调过 `process_feature_inclusion`，那时它**是**生产可达的。⟹ 「不可达」是**修复后的现状**，
   不是**设计保证**，没有任何断言/测试锁住「生产不得调它」。

---

## 5. G-3 第二特征序列是什么

### 5.1 「代理」自承：代理什么、谁是被代理的那个

`formal/Origin/SegmentAutoConstruct.lean:172–178` 逐字：

```lean
172:/--
173:  从候选笔列提取完整段尾数据（用于布尔版完整判据）。
174:  向上线段找顶分型，向下线段找底分型。`rest`（分型后剩余特征序列）作为 `revSeq` 代理。
175:  返回 none 表示当前笔列中找不到分型（未达确认）。
176:-/
177:def extractSegEndData (segDir : Direction) (startPrice endPrice : Tick)
178:    (strokes : List Stroke) : Option SegEndData :=
179:  let featureSeq := buildFeatureSeq segDir strokes
```

**代理项 = `revSeq`**（`SegEndData` 的字段，喂 `segmentEndUpB/segmentEndDownB` 的第二特征序列位）。
**代理物 = `r.rest`** = **第一特征序列**在分型之后剩下的元素（`scanTopFractal`/`scanBottomFractal`
返回的 `rest` 字段，`:98`）。
**被代理的那个 = 真正的第二特征序列**（原文 `067:38`【正文】「从该分型最高点开始的**向下一笔
开始的序列的特征序列**」）。

### 5.2 两条序列的元素来源与确认判据（逐条核）

| | Lean（`SegmentAutoConstruct`） | 生产（Rust theta_v0 / rust 旧引擎 / Python v1） |
|---|---|---|
| **元素来源** | `r.rest` ⊂ `buildFeatureSeq segDir strokes`，而 `extractFeatureStrokes`（`:40–45`）取 `s.direction == segDir.flip` ⟹ **反向笔**（与第一特征序列同源，只是分型之后的尾巴） | `feature_seq.rs:249` `if sk.direction != seg_dir { i += 1; continue; }` ⟹ **只取 `seg_dir` 同向笔**；起点 `:246` `i = from_stroke_idx + 1` |
| **是否新建序列** | **否**——复用第一特征序列的剩余段，不重做包含处理（`r.rest` 已是标准序列的尾部） | **是**——`:235` `let mut elements = Vec::new()`，`:237–240` `dir_state` 初值**与主序列相反**（`Up ⟹ Some(Down)`），`:257` 重做方向性包含处理 |
| **确认判据** | `segmentEndUpB`（`:142-143`）：`if hasGapB e1 e2 then fractalInSeqB false revSeq else true` ⟹ 向上段只找 **底**分型（`wantTop=false`），向下段只找**顶**分型（`:147`）⟹ **只找对偶分型** | `has_any_fractal`（`feature_seq.rs:164–181`）：`:173` 顶分型 `b_h > a_h && b_h > c_h` **或** `:176` 底分型 `b_l < a_l && b_l < c_l` ⟹ **任意分型**；提前终止版 `:266–269` 同 |

⟹ **票面「根本不是同一条序列」成立，且比票面说的更强**：

1. **元素来源不相交**。第一特征序列取 `segDir.flip` 笔，第二特征序列取 `segDir` 笔；
   在严格交替笔流上这两个集合**互补且不交**。
   ⟹ Lean 的 `revSeq`（反向笔的尾巴）与生产的第二特征序列（同向笔）**没有一个元素重合**。
2. **不是「同一条序列的两种表述」**，是**两条不同笔集合上的两条不同序列**。
3. **确认判据也不同**：对偶分型 ⊊ 任意分型 ⟹ **Lean 的接受集是生产接受集的子集**
   （固定同一 `revSeq` 时；但因元素来源本就不同，实际是两个不可比的判据）。

### 5.3 ★原文站哪一边（两条都有【正文】级依据，方向明确）

**元素来源 —— 原文站「同向笔」**：`067-第67课.md:38`【正文】逐字：

> 「第二种情况：特征序列的顶分型中，第一和第二元素间存在特征序列的缺口，如果从该分型最高点
> 开始的**向下一笔开始的序列的特征序列**出现底分型，那么该线段在该顶分型的高点处结束……」

推理链：原线段向上 ⟹ 第一特征序列 = 向下笔。从分型最高点开始「向下一笔开始的序列」
= 一段新的**向下走势**；该向下走势的特征序列 = **向上笔** = 与原线段**同向**。
⟹ **生产的「取 seg_dir 同向笔」是原文可导的；Lean 的「取剩余反向元素」不是。**
（`second_kind.rs:85–86` 也是这么引的，逐字：「取 **seg_dir 同向笔**（= 新反向段的特征序列元素，
第67课"从分型极值点开始的反向一笔开始的序列的特征序列"）」。）

**确认判据 —— 原文站「任意分型」**：`067-第67课.md:46`【正文】逐字：

> 「强调，在第二种情况下，后一特征序列不一定封闭前一特征序列相应的缺口，而且，第二个序列中的
> 分型，不分第一二种情况，**只要有分型就可以**。」

⟹ **「只要有分型就可以」= 任意分型 ⟹ 生产侧对，Lean 的「只找对偶分型」与原文不符。**
`077-第77课.md:74–78`【正文】给了为什么可以简化的完整论证（收敛倒推）。

**追加原文（G-3 相关，同向笔序列的包含处理）**：`078-第78课.md:54`【正文】逐字：

> 「另外，一定要注意，对于第二种情况的第二特征序列的分型判断，**必须严格按照包含关系的处理来**，
> 这里不存在第一种情况中的假设分界点两边不能进行包含关系处理的要求。……而在第二种情况的第二
> 特征序列中，其**方向是和原线段一致**，包含关系的出现，就意味着原线段的能量充足……」

⟹ **「其方向是和原线段一致」直接、逐字坐实第二特征序列元素 = 同向笔。**
⟹ **同时坐实第二特征序列必须做包含处理**（生产做了，`feature_seq.rs:257`；
Lean 的 `r.rest` 是第一序列已做过包含处理的尾巴，**没有对同向笔重做**）。

**⟹ G-3 的名分结论（考据层，不裁）**：生产侧（同向笔 + 任意分型 + 重做包含处理）
**三项全部有【正文】级原文依据**；Lean 侧（剩余反向元素 + 只找对偶分型）
**未查到任何原文依据**，且其自身注释 `:174` 已自承是「代理」。

### 5.4 计数核实与可达性

**「同向笔新建、任意分型即确认」独立实现穷举 = 3 处**（票面「3 处」**对**）：

| # | 位置 | 备注 |
|---|---|---|
| 1 | `rust/src/theta_v0/parser/feature_seq.rs:229` `second_seq_has_fractal` | 现役生产 |
| 2 | `rust/src/segment.rs:416` `second_seq_has_fractal`（f64 旧引擎） | 现役（orchestrator 路径） |
| 3 | `src/newchan/a_segment_v1.py` `_second_seq_has_fractal`（:354–384，由 `feature_seq.rs:219` 逐字引出行号） | 参考正本 |

`rust/src/theta_v0/parser/second_kind.rs:103` `second_kind_confirmed` **是转发壳不计**
（`:111` `second_seq_has_fractal(strokes, seg_dir, apex_stroke_offset, 0)`）。
**穷举方法**：`grep -rn "second_seq_has_fractal|_second_seq_has_fractal|second_kind_confirmed"`
覆盖 `rust/src`、`src/newchan`、`tests`、`analysis`、`scripts`。

**可达性（本条依 Notes N-1 预授权判定）**：
- 生产侧 3 处**全部可达**：`feature_seq.rs:464`（在 `scan_trigger`，被
  `segment.rs:429` `feat.scan_trigger(strokes)` 调用，而 `divide_segments_with_tail` 是
  theta_v0 线段层唯一入口）；`rust/src/segment.rs:593`（在 `segments_from_strokes_v1`，被
  `orchestrator.rs:860`／`:857` 调用）；Python `a_segment_v1` 被
  `src/newchan/ab_bridge_newchan.py:106`（默认 `segment_algo="v1"`，`:228`）调用。
- Lean 侧 `extractSegEndData`：**不在 Rust/Python 生产调用图里**（Lean 独立）。
  ⚠️**但它也不在 Lean 端到端管线里**——`Pipeline.lean` 走 `segmentsOf`（§2.4），
  **不经 `SegmentAutoConstruct`**。穷举：`grep -rn "SegmentAutoConstruct" formal/` 确认
  `Pipeline.lean` 未 import 它。⟹ **Lean 的「完整判据」这一支两头都没接**：
  既不在生产上，也不在 Lean 自己的端到端管线上。**这条票面没说。**

---

## 6. G-4 缺口封闭降级要不要

### 6.1 载体

| 侧 | 位置 | 逐字 |
|---|---|---|
| Rust theta_v0（枚举） | `rust/src/theta_v0/parser/feature_seq.rs:278–285` | 「/// 延续模式（Python `extend_mode`）。」`Strict`：「67课严格延续：缺口被 c 封闭 → 按第一种情况处理（段更易终结）。」`Optimized`：「优化延续：任何缺口都走第二种情况（段更易延续）。」 |
| Rust theta_v0（判据体） | `feature_seq.rs:452–461` | `// 67课严格延续：缺口被 c 封闭 → 第一种情况（Python :414-422）。` / `if has_gap && self.extend_mode == ExtendMode::Strict {` / `Direction::Up => c_l <= a_h,` / `Direction::Down => c_h >= a_l,` / `if gap_closed_by_c { has_gap = false; }` |
| Rust 旧引擎 | `rust/src/segment.rs:582–587` | `let gap_closed_by_c = if seg_direction == Direction::Up {` … `if gap_closed_by_c {` |
| Python 正本 | `src/newchan/a_segment_v1.py:431–436` | `if has_gap and self._extend_mode == "strict":` / `gap_closed_by_c = c_l <= a_h`（`:433`）/ `gap_closed_by_c = c_h >= a_l`（`:435`）/ `if gap_closed_by_c:`（`:436`） |
| Python 开关 | `a_segment_v1.py:567` + `:573–575` | `extend_mode: Literal["strict", "optimized"] = "strict",` ；`"strict" — 67课严格延续：缺口被 c 封闭时按第一种情况处理（段更容易终结）` / `"optimized" — 优化延续：任何缺口都走第二种情况（段更容易延续）` |
| Lean | `formal/Origin/SegmentFeatureComplete.lean:477` | 「但有缺口封闭即切"（非缠论标准），第一支可去，但那不是第67课判据——当前严格三支合取。」 |

### 6.2 ★两档的后果远大于「灰区」（票面低估）

`tests/test_rust_segment_equivalence.py:9–12` 逐字：

> 「本 golden 契约在**新口径**下继续成立，相切边界不再 bit-exact 对齐 2026-07-26 前旧口径的
> 历史输出/基线（实测段端点零变化，仅 `break_evidence.gap_type` 标签级翻转：
> **BZ 全年 strict 28 段、optimized 159 段**，second→none）。」

**口径五项（按纪律）**：点估计；**1 个窗**（BZ 全年）；单窗即合计；覆盖率未声明；level 0。
**⚠️ 这句话的上下文是在讲相切边界改口径，28/159 是顺带给的两档段数对比，
不是专为 G-4 做的实验。** 但两档 **28 vs 159 ≈ 5.7 倍**，量级远大于 G-2 的 Δ=2 段（1.2%）。

另有专门测试锁两档差异：`tests/test_segment_gap_classification.py:401` 逐字
「"""strict 模式应比 optimized 多断出段。"""」（`:404` `segments_from_strokes_v1(strokes, extend_mode="optimized")`）、
`:418–421` `test_optimized_continues_past_gap`。
跨语言两档都跑：`tests/test_rust_segment_equivalence.py:156`／`:226`
`@pytest.mark.parametrize("extend_mode", ["strict", "optimized"])`。

### 6.3 ★原文站哪一边

**strict 有原文依据，但在【答疑】层不在【正文】层**：`067-第67课.md:196`【答疑，界外】逐字：

> 「麻烦的是第二种情况，在那种情况下，并不是任何三笔都能构成破坏，就算最终特征序列元素间的
> 缺口被封闭了。注意，在第二种情况下，**即使封闭，肯定不是被第一个给封闭的，因为这样就变成
> 第一种情况了**。」

推理链：「被第一个封闭 ⟹ 变成第一种情况」= strict 的判据本体（缺口被紧邻的下一个元素 `c`
封闭 ⟹ `has_gap = false` ⟹ 走第一种情况）。⟹ **strict 与这句【答疑】同构。**
`067-第67课.md:170`【答疑，界外】另一句同向：「这里是第一种情况，也就是特征序列缺口被第一笔就
封闭的情况，没必须探讨第二段特征序列分型的问题，那是第二种情况考虑的问题。」

**optimized（任何缺口都走第二种情况）—— 未查到任何原文依据**，且与上引【答疑】直接相悖。

**⚠️ 一条不能省的口径警告**：`067-第67课.md:46`【正文】说的是
「后一特征序列**不一定封闭**前一特征序列相应的缺口」——**这句管的是「第二特征序列要不要
封闭原缺口」（答案：不要），不是「原缺口被 c 封闭时要不要降级」。** 两件事同用「封闭」二字
极易混淆。G-4 问的是后者，其依据只在【答疑】层（`067:170`／`:196`）。
**⟹ 若裁「strict 唯一」，名分只能标到【067课答疑】级，标不到【正文】级。**

### 6.4 可达性（本条依 Notes N-1 预授权判定）

**默认构造那条路径逐条查**（票面点名要查默认，且在案有 env/scripts 覆写陷阱）：

| 侧 | 默认值 | 保证它的那一行 | 有无覆写 |
|---|---|---|---|
| Rust theta_v0 | **Strict**，且 `Optimized` **无生产构造点** | `segment.rs:408` `ExtendMode::Strict`（在 `divide_segments_with_tail`）、`:699` `ExtendMode::Strict`（在 `IncrSegments::from_full`）；穷举 `grep -rn "ExtendMode" rust/src` = 仅 `:280/:282/:284` 定义、`:306/:322` 字段/参数、`:453` 比较、`segment.rs:35` import、`segment.rs:408/:699` 构造、`feature_seq.rs:622/:629/:638` 三个**单元测试** | **`ExtendMode` 不进 `ParseConfig`**（`config.rs:39–58` 只有 `new_stroke_min_gap`/`seg_min_strokes`/`second_seq_scan_window` 三个字段）⟹ **无 config、无 env**。`grep -n "env::var" rust/src/theta_v0/config.rs` = 零命中 |
| Rust 旧引擎 | **strict** | `orchestrator.rs:860` `segments_from_strokes_v1(strokes, 3, true)`、`:857` `..., true, ss, sd)`；第三参 = `extend_mode_strict: bool`（`rust/src/segment.rs:731`）⟹ `true` = strict | 无 config/env |
| Python | **"strict"** | `a_segment_v1.py:567` 默认参数；调用方 `ab_bridge_newchan.py:106` `segments_from_strokes_v1(strokes)` **不传该参数** ⟹ 取默认 | `grep -rn "extend_mode" .github/ scripts/` = **零命中**（已核，含 `scripts/` 与 `.github/workflows/`）；`optimized` 的非测试命中只有 `analysis/_xcheck_secondkind_v1.py:46`（诊断脚本，非生产） |

⟹ **G-4 的 `optimized` 臂在 Rust theta_v0 上是「枚举变体存在但无生产构造点」（等价于死分支）；
在 Python 上是「可传参但无生产传参点」。** 两侧都**不是**默认。
**没有 env、没有 config、`scripts/` 与 `.github/` 无覆写**（已按在案陷阱清单逐一核过）。
⟹ **G-4 的两档在现役输出上不产生差异**，但**枚举/参数仍在**，且有测试锁着两档都能跑
（`test_segment_gap_classification.py:404`、`test_rust_segment_equivalence.py:156/:226`）
⟹ **裁「删 optimized」会翻掉这几个测试。**

### 6.5 与「D-5」的口径一致性要求

**⚠️ 本报告未拿到 D-5 的票面正文**（D-5 属另一张概念票，`gh issue view 813` 只写「与 D-5 同型」）。
可以确定的**同型结构**是：**「两档配置并存、默认那档有原文依据、另一档无原文依据、
且另一档在生产上无构造点」**。⟹ 两票口径须一致的**具体内容**是：

1. **无原文依据的那一档，处置动作要一样**（同删／同留为诊断开关／同标「非缠论口径」）；
2. **名分层级要一样**——G-4 的 strict 只到【067课答疑】级（§6.3），若 D-5 的默认档只到
   【推论】或【设计选择】级，两票不能一个标【正文】一个标【答疑】；
3. **「灰区」判定标准要一样**——本报告认为 G-4 **不该继续标灰区**：它的两档实测差 5.7 倍
   （§6.2），且 optimized 与【答疑】原文直接相悖（§6.3）。若 D-5 用同一标准，也应重标。

**⟹ 这一条必须等 D-5 票面到手才能收口，本报告只给结构对齐点，不替 D-5 定。**

---

## 7. G-5 第二特征序列扫描窗口

### 7.1 ★票面把两个不同的窗口混成了一组（最要紧的一处前提错误）

仓内有**两个**独立的窗口常数，票面 G-5 把它们的载体与实测句**交叉搭错了**：

| | 常数 | 管什么 | theta_v0 现值 | Python 现值 | rust 旧引擎现值 |
|---|---|---|---|---|---|
| **W1** | `MAX_SECOND_SEQ_SCAN` / `second_seq_scan_window` | **第二**特征序列扫描窗 | **0 = 无限**（`config.rs:65`，`ParseConfig::default`） | **50**（`a_segment_v1.py:367`） | **50**（`rust/src/segment.rs:413`） |
| **W2** | `TAIL_WINDOW` | **第一**（主）特征序列的尾窗扫描起点下界 | **7，硬编码，不进 config**（`rust/src/theta_v0/parser/segment.rs:79`，消费点 `:409`/`:700`） | **7**（`a_segment_v1.py:288`，消费点 `:413` `start = max(1, self.last_checked, n - self.TAIL_WINDOW)`） | **7**（`rust/src/segment.rs:410`，消费点 `:556-557`） |

**票面 G-5 写「`MAX_SECOND_SEQ_SCAN=50` ↔ 无限」并附实测「窗口 7→∞ 输出不变」。**
但那句实测**是关于 W2 的**，不是 W1。原文出处 `rust/src/theta_v0/parser/segment.rs:74–78` 逐字：

```
74:/// 尾窗大小（对齐 Python `_FeatureSeqState.TAIL_WINDOW=7`）。
75:///
76:/// ★L2 实测（OKLO 2000 笔，analysis/segment_refsem_cert.py）：窗口从 7 扩到 ∞ 输出不变
77:/// （237→237），证 7 在目标数据域非约束性（第二特征序列分型若有效必在窗口内）。保留 7 以
78:/// bit-exact 对齐参考默认；改值 = 改 Θ。这是单数据集 L2 观察，**非** L0 结构证明（不裸剥）。
```

W1 侧另有一句**独立**的同型实测，票面没引 —— `rust/src/theta_v0/config.rs:52–56` 逐字：

```
52:    /// 第二特征序列扫描窗口（0 = 无限全扫，bit-exact 优先）。
54:    /// ★`[设计选择,默认值]`（second_kind.rs 模块头声明）：Python 的 `MAX_SECOND_SEQ_SCAN=50`
55:    /// 是 O(n²) 性能妥协，**非缠论可导**。本字段 default 0（无限）——L2 实测（OKLO 2000 笔）
56:    /// 窗口扩到 ∞ 输出不变，证 50 在目标数据域非约束性（单数据集 L2，不裸剥常数）。
```

⟹ **订正后的 G-5 应是两组**：
- **G-5a（W1，第二序列窗）**：`50`（Python / rust 旧引擎）↔ `0=无限`（theta_v0 默认）。
  **两侧默认真的不同 ⟹ 真分歧。**
- **G-5b（W2，主序列尾窗）**：三侧**都是 7**，**无分歧**。
  「7→∞ 输出不变」是**为「保留 7」辩护的实测**，不是两档并存的证据。

**⟹ 票面 G-5 的「↔ 无限」这条对立，把 W1 的两档与 W2 的实测混编了。**

### 7.2 「7→∞ 输出不变（237→237）」的口径（五项）

1. **点估计还是下界**：点估计（段数计数相等），**无等价性证明**。
2. **几个独立窗**：**1 个**（OKLO 2000 笔）。
3. **每窗还是合计**：单窗。
4. **覆盖多少总体**：未声明。
5. **哪一级**：level 0。

`segment.rs:78` 自己已诚实标：「这是单数据集 L2 观察，**非** L0 结构证明（不裸剥）」。
**票面「（090）单点实测一致，未证等价」的标注是准确的**，这一条票面没有过度声称。
**但**：§3.1 已查出该 harness 已退役且数据文件从未入库 ⟹ **这条实测同样不可复现**，
票面没标这一层。

### 7.3 W1 的等价性另有一个**仓内可跑**的锚（票面没引，比 OKLO 那条强）

`rust/src/theta_v0/parser/feature_seq.rs:691` 有 property 测试
`property_second_seq_early_terminate_equals_full_scan`（`:660–663` doc 逐字
「★bit-exact 等价性验证（#93 性能优化的等价锚）：提前终止版 `second_seq_has_fractal`
…」），`:653` `second_seq_window_unlimited_vs_bounded` 直接对比 `scan_window=0` 与 `=50`
（`:656–657` `assert!(!second_seq_has_fractal(&strokes, Direction::Up, 0, 0));`
`assert!(!second_seq_has_fractal(&strokes, Direction::Up, 0, 50));`）。
另有 `#[ignore]` 的标度+等价 harness `rust/src/theta_v0/parser/profile.rs:104`
`diag_segment_window_effect`（`:111` `let windows = [0u32, 50, 200];`，`:142` `eqs[wi] = segs == base_segs;`，
数据 `:107` `es_1m_databento_10y.json`，规模 `:110` 最大 128000 bar）。

⟹ **要给 W1 一个能跑的等价性读数，路子是 `profile.rs:104`（ES 1m 10y，多规模），
不是已退役的 OKLO harness。** 但它 `#[ignore]`，且**只测 0/50/200，不测 7**。
⟹ **「7→∞」这个具体对比，目前仓内没有可跑的复现路径。**

### 7.4 可达性（本条依 Notes N-1 预授权判定）

- **W1 = 0（无限）是 theta_v0 生产默认**：`config.rs:60–67` `impl Default for ParseConfig`
  的 `second_seq_scan_window: 0`（`:65`）；穷举 `grep -rn "ParseConfig {" rust/src --include="*.rs"`
  = 4 处，其中**非默认构造只有 2 处且都非生产**：`stroke.rs:394`（测试辅助 `fn cfg`）、
  `profile.rs:131`（`#[ignore]` 诊断）。⟹ **生产恒取 default**。
  **无 env 覆写**：`grep -n "env::var" rust/src/theta_v0/config.rs` 零命中。
  **`scripts/` 与 `.github/` 零命中** `second_seq_scan_window`（已核）。
- **W1 = 50 在 rust 旧引擎与 Python 上生产可达**：
  `rust/src/segment.rs:593` 硬传 `MAX_SECOND_SEQ_SCAN`（无参数化），被
  `orchestrator.rs:860` 消费；`orchestrator.rs:215`、`segment_layers.rs:540`／`:838`
  另把它当 checkpoint 稳定性判据的 margin（`trigger_k + 1 + MAX_SECOND_SEQ_SCAN > n_strokes`）。
  Python `a_segment_v1.py:367` 类属性，`src/newchan/core/recursion/segment_engine.py:26`
  `_SECOND_SEQ_MARGIN: int = _FeatureSeqState.MAX_SECOND_SEQ_SCAN` 再取一次。
- ⟹ **G-5a 不是伪分歧**：`50` 与 `无限` **两侧都在生产路径上**（不同引擎），
  且 `50` 还额外承担了 orchestrator 的 checkpoint margin 语义
  （`orchestrator.rs:110` 逐字「段内任一候选缺口分型的第二特征序列扫描窗口
  （≤MARGIN=`MAX_SECOND_SEQ_SCAN`）」）⟹ **改 50 会连带动 checkpoint 稳定性判据**。
- **无测试锁住 W1 的取值本身**（`grep MAX_SECOND_SEQ_SCAN`／`second_seq_scan_window` 在
  `rust/tests/`、`tests/` 零命中）。

---

## 8. 原文考据（线段侧，全量检索）

**穷举方法**：语料 `docs/chanlun/text/blog/`（缠师博文 110 课 + 答疑，**最终权威**）与
`docs/chanlun/text/chan99/`（编纂版，已知遗漏 67/71/77/78 课）。检索式
`grep -rn "第二特征序列|三笔重叠|重叠|线段划分|特征序列|缺口" docs/chanlun/text/blog/`
（覆盖 001–110 全部文件）+ `… docs/chanlun/text/chan99/` 全目录；命中课逐课精读
（065/066/067/069/070/071/076/077/078/080/081）。
**引文核对两步全过**（① `↑正文` 分界行号；② 行首/行内「（注：」「（娇注：」标记）。

### 8.1 线段的构造判据

| 引文 | 出处 | 判定 |
|---|---|---|
| 「线段至少有三笔，线段无非有两种，从向上一笔开始的，和从向下一笔开始的。」 | `065-第65课.md:42` | 【正文】（42 < 界 56） |
| 「线段有一个最基本的前提，就是线段的前三笔，必须有重叠的部分……线段至少有三笔，但并不是连续的三笔就一定构成线段，这三笔必须有重叠的部分。」 | `065-第65课.md:48` | 【正文】 |
| 「**缠中说禅线段分解定理**：线段被破坏，当且仅当至少被有重叠部分的连续三笔的其中一笔破坏。而只要构成有重叠部分的前三笔，那么必然会形成一线段，换言之，线段破坏的充要条件，就是被另一个线段破坏。」 | `065-第65课.md:50` | 【正文】 |
| 「线段划分的最基本原则，就是线段必须至少有三笔……至于两笔为什么不能构成线段，这理由更简单，因为两笔，那么线段的两段的分型的性质肯定是一样的……由此可见，线段中包含笔的数目，都是**单数**的。」 | `077-第77课.md:62` | 【正文】（62 < 界 92） |
| 「而且，线段开始的那三笔，必须有重合，开始三笔没有重合的，是构不成线段的。」 | `077-第77课.md:64` | 【正文】 |

**★「三笔重叠」这四字连写，原文零命中**。缠师一律写「（前）三笔必须有**重叠/重合的部分**」。
「三笔重叠」是编纂版/仓内的简化提法（`chan99/0009-第七节 线 段.md:5`
「线段：至少由三笔组成，而且前三笔必须有重叠的部分。」）。

**★ 与 G-1 三范式的对照（考据结论，不裁）**：

- **「线段中包含笔的数目，都是单数的」（`077:62`【正文】）** ⟹
  **P3（Lean，恒 2 笔）与原文直接冲突**（2 是双数）。这是本报告为 G-1 找到的**最硬的原文否证**。
- **「至少三笔」+「前三笔必须重叠」** ⟹ P1（恒 3 笔）满足下界但**把「至少」读成了「恰好」**；
  P2（≥3，可变）与原文一致。
- **P1 还有第二处原文冲突**：`077:62`【正文】「一个完整线段的两段的分型不可能是同性质的」
  ⟹ 线段必上下交替。§2.5 手算样例里 **P1 与 P3 都产出连续同向段**，两者都违反此句。

### 8.2 特征序列（G-2 相关）

| 引文 | 出处 | 判定 |
|---|---|---|
| 「以向上笔开始的线段，可以用笔的序列表示：S1X1S2X2…SnXn。容易证明，任何Si与Si+1之间，一定有重合区间。而考察序列X1X2…Xn，该序列中，Xi与Xi+1之间并不一定有重合区间，因此，这序列更能代表线段的性质。」 | `067-第67课.md:16` | 【正文】（16 < 界 62） |
| 「**定义**：序列X1X2…Xn成为以向上笔开始线段的特征序列；序列S1S2…Sn成为以向下笔开始线段的特征序列。特征序列两相邻元素间没有重合区间，称为该序列的一个**缺口**。」 | `067-第67课.md:18` | 【正文】 |
| 「关于特征序列，把每一元素看成是一K线，那么，如同一般K线图中找分型的方法，也存在所谓的包含关系，也可以对此进行**非包含处理**。经过非包含处理的特征序列，成为**标准特征序列**。」 | `067-第67课.md:20` | 【正文】（**同一行末尾另有「（娇：这里的描述有欠缺……）」，那部分标【娇注】非正文本体**） |
| 「特征序列的元素包含关系，首先的前提是这元素都在一特征序列里，如果两个不同的特征序列之间的元素，讨论包含关系是没意义的。」 | `071-第71课.md:18` | 【正文】（18 < 界 46） |
| 「显然，特征序列的元素的方向，和其对应的段的方向是刚好**相反**的……因此，根本也不可能存在包含的可能。」 | `071-第71课.md:20` | 【正文】 |
| 「在这假设的转折点前后那两元素，是**不存在**包含关系的……但假设的转折点后的顶分型的元素，**是可以**应用包含关系的。」 | `071-第71课.md:42` | 【正文】 |
| 「对于第二种情况的第二特征序列的分型判断，**必须严格按照包含关系的处理来**，这里不存在第一种情况中的假设分界点两边不能进行包含关系处理的要求。为什么？因为在第一种情况中，如果分界点两边出现特征序列的包含关系，那证明对原线段转折的力度特别大，那当然不能用包含关系破坏这种力度的呈现。而在第二种情况的第二特征序列中，其**方向是和原线段一致**，包含关系的出现，就意味着原线段的能量充足……」 | `078-第78课.md:54` | 【正文】（54 < 界 64） |

**★ G-2 的原文考据结论（不裁）**：

1. 「元素当一K线做非包含处理」（`067:20`【正文】）⟹ 照字面执行 = **K 线层那套方向性合并**
   （§4.3），**不是外包络**。
2. 「包含处理是不是方向性的」**原文没有正面用「方向」二字回答合并公式**，但
   `067:20` 把它归约到「一般K线图找分型的方法」⟹ 方向性合并是**推论级**（`[新缠论:推论]`，
   推理链 = `067:20` 归约 + `rust/.../inclusion.rs:49-55` 的 K 线层实装公式 + 62/65 课 K 线
   包含处理原文）。**外包络在原文中查不到依据。**
3. **原文确有一处「包含处理的适用范围随位置而变」的方向性论述**：`071:42` +`078:54`
   ——「假设分界点两侧禁包含处理」vs「第二特征序列必须做包含处理」。
   **这是原文层面真正的「方向性」内容，票面 G-2 完全没有涉及。**
   ⚠️**这条对生产代码是个未核的开口**：`feature_seq.rs` 的 `append`（`:370`）doc 声称实装了
   71 课「假设转折点」（`:365-369`），但**「分界点两侧禁包含」这一条是否落实、
   落在哪一行，本报告未逐行核完**——列为待查。

### 8.3 第二特征序列（G-3 相关）

**「第二特征序列」原文确凿存在**。穷举：`grep -rn "第二特征序列" docs/chanlun/text/blog/`
= **9 处，分布 067／070／076／077(×2)／078(×2)／080 共 6 课**；
`grep -rn "第二特征序列" docs/chanlun/text/chan99/` = **0 处**（印证编纂版遗漏 77/78 课）。

| 引文 | 出处 | 判定 |
|---|---|---|
| 「第二种情况：特征序列的顶分型中，第一和第二元素间存在特征序列的缺口，如果从该分型最高点开始的**向下一笔开始的序列的特征序列**出现底分型，那么该线段在该顶分型的高点处结束……」 | `067-第67课.md:38` | 【正文】 |
| 「强调，在第二种情况下，后一特征序列**不一定封闭**前一特征序列相应的缺口，而且，第二个序列中的分型，不分第一二种情况，**只要有分型就可以**。」 | `067-第67课.md:46` | 【正文】 |
| 「各位肯定注意，在第二种情况下特别强调，第二特征序列，其实就是对应着线段C对线段B的破坏，不再分第一、二种情况了。这，其实是一个**简化的方法**。」 | `077-第77课.md:74` | 【正文】 |
| 「（收敛倒推论证：）如果我们坚持线段的最终破坏回补特征序列缺口情况……否则，这些线段的区间就会无限缩小，最后就会形成一个点，这显然是不可能的……」 | `077-第77课.md:76` | 【正文】 |
| 「正因为这样，所以在第二种情况中的第二特征序列判断中，就不再分第一、二种情况了……」 | `077-第77课.md:78` | 【正文】 |
| 「（第二特征序列）其方向是和原线段**一致**」 | `078-第78课.md:54` | 【正文】 |
| 「其实，如果第二特征序列没有三个元素，就根本不存在出现分段中第二种情况的可能。」 | `067-第67课.md:152`（同句另见 `070-第70课.md:96`） | 【答疑，界外】 |
| 「如果对第二种情况，找不到第二特征序列，那就是原来的线段没被破坏，就这么简单。请一切从定义出发。」 | `076-第76课.md:114` | 【答疑，界外】 |

⟹ 见 §5.3：**生产侧三项（同向笔 / 任意分型 / 重做包含处理）全部有【正文】级依据；
Lean 侧两项（剩余反向元素 / 只找对偶分型）零原文依据。**

**顺带核实一条生产实装与【答疑】的对应**：`067:152`【答疑】「第二特征序列没有三个元素 ⟹
不存在第二种情况」↔ `feature_seq.rs:261` `if n >= 3 {`（不足三元素不判分型）+
`has_any_fractal` `:166-168` `if n < 3 { return false; }`。**对得上。**

### 8.4 缺口封闭 / 两种情况（G-4 相关）

| 引文 | 出处 | 判定 |
|---|---|---|
| 「在标准特征序列里，构成分型的三个相邻元素，**只有两种可能**：」 | `067-第67课.md:24` | 【正文】 |
| 「**第一种情况**：特征序列的顶分型中，第一和第二元素间**不存在**特征序列的缺口，那么该线段在该顶分型的高点处结束……」 | `067-第67课.md:26,28` | 【正文】（**`:28` 末尾另有「（娇注：这里描述不全面，后期有补充。）」标【娇注】**） |
| 「**第二种情况**：……第一和第二元素间**存在**特征序列的缺口，如果从该分型最高点开始的向下一笔开始的序列的特征序列出现底分型，那么该线段在该顶分型的高点处结束……」 | `067-第67课.md:36,38` | 【正文】 |
| 「（注：第二种情况属于第三线段成立倒推第二线段成立的判断）」 | `067-第67课.md:44` | 【注，整行为括号注，**非正文本体**】 |
| 「上面两种情况，就给出**所有**线段划分的标准。显然，出现特征序列的分型，是线段结束的前提条件。」 | `067-第67课.md:48` | 【正文】 |
| 「这里是第一种情况，也就是特征序列缺口**被第一笔就封闭**的情况，没必须探讨第二段特征序列分型的问题，那是第二种情况考虑的问题。」 | `067-第67课.md:170` | 【答疑，界外】 |
| 「注意，在第二种情况下，**即使封闭，肯定不是被第一个给封闭的，因为这样就变成第一种情况了**。」 | `067-第67课.md:196` | 【答疑，界外】 |

**★「线段划分标准的两种（分类分歧）」—— 查不到。** 穷举
`grep -rn "线段划分" docs/chanlun/text/blog/`（全 110 课）：命中课 067/070/071/076/077/078/080，
**全部是同一套标准内部依「缺口有无」的两个分支**，**未查到任何两套互相竞争的线段划分标准**。
⟹ 若仓内某处主张「线段划分有两派标准」，**没有原文依据**。

### 8.5 《市场天道》交叉检索

**落点**：`docs/chanlun/text/tiandao/市场天道.pdf`（Git LFS，179MB），OCR 全文
`docs/chanlun/text/tiandao/ocr/ch00-front.md` ~ `ch13.md`；`INDEX.md` 注明**正文页 = PDF 页 − 18**。
**去空格检索**（`re.sub(r'\s+','',text)` 后 `.count()`，覆盖全部 14 个 `ch*.md`）：

| 词 | 命中 |
|---|---|
| 特征序列 | 仅 `ch04.md` **66 处**，其余章节 0 |
| 线段划分 | `ch03.md` 2、`ch04.md` **9**，其余 0 |
| 缺口 | `ch00-front` 2、`ch04` 44、`ch05` 4、`ch06` 4、`ch08` 1、`ch10` 3、`ch11` 58 |
| **三笔重叠** | **全书 0 处** |
| **第二特征序列** | **全书 0 处** |

**实质判断：《市场天道》在线段这一节上没有独立表述，是转述第67课。**
`ch04.md:2154` 去空格后逐字：「同时，在《教你炒股票第67课:线段的划分标准〈特征序列)》一文中，缠……」
随后 `ch04.md:2157–2226` 大段复述第67课「第一种情况／第二种情况」定义句，与
`067-第67课.md:24–46` 逐字或近逐字一致。

⟹ **G-1～G-5 五组，《市场天道》均无独立于缠论的判据表述。**
（页码：特征序列定义主体大致落 PDF p168–p174 ⟹ 正文页约 150–156；
**此为按 OCR 块序推算的近似值，非逐页精确核验**，不作精确断言。）

---

## 9. 形态清单（候选 + 代价；不含推荐，裁定由人做）

### 通则：哪几条挂在 S-1 上

| 组 | 是否挂 S-1 | 挂法 |
|---|---|---|
| **G-1** | **挂** | 三套构造范式全部**吃笔列**。S-1 定的是「至少 N 根独立 K」判据 ⟹ 定出的**笔集合**不同，则 §2.5 的接受集关系（尤其「P3 恒 2 笔」这一推论所依赖的**严格交替**前提）需重核：`merged_gap≥2 ∧ raw_gap≥3` / `source_index 之差 >3` / `noSharedBar ∧ barsBetween ≥ 3` 三套若产出**不同笔数与不同端点**，则 G-1 的「406/237」类段数对比与 §2.5 手算样例都要在 S-1 定案后重算。**但「P3 恒 2 笔」本身不挂 S-1**——它只依赖「相邻笔方向严格交替」，而三套 S-1 判据**都是在交替分型序列上配对**（`stroke.rs:30/:70`）⟹ 交替性不受 S-1 选择影响。 |
| **G-3** | **挂** | 第二特征序列的元素来源是「`from_stroke_idx` 之后的 seg_dir 同向笔」（`feature_seq.rs:246-252`）⟹ **笔集合变 ⟹ 第二特征序列的元素变 ⟹ 分型有无变 ⟹ SecondKind 确认与否变**。G-3 的**判据选择**（同向笔 vs 剩余反向元素、任意 vs 对偶分型）**不挂 S-1**；但**任何量化读数**（确认率、段数）挂。 |
| **G-2** | **不挂**（判据选择层）；量化读数挂 | 方向性 vs 外包络是**合并公式**之争，与笔集合无关。但「Δ=2 段 / 1.2%」这类读数挂 S-1。 |
| **G-4** | **不挂**（判据选择层）；量化读数挂 | strict/optimized 是**缺口降级规则**之争。但「BZ 全年 28 vs 159 段」挂 S-1。 |
| **G-5** | **不挂**（判据选择层）；量化读数挂 | 窗口大小之争。但「237→237」「窗口等价」类读数挂 S-1（且 §3.1 已证不可复现）。 |

### G-1 候选形态

| 候选 | 改哪些代码/Lean | 翻掉哪些在案读数 | 名分能标到哪一级 |
|---|---|---|---|
| **A. 定 P2（特征序列状态机）为唯一正本，P1 降参考、P3 标「非线段判据」** | 不必改实装（P2 已是三侧生产默认）；须改 `.chanlun/definitions/xianduan.md`（补 theta_v0 与 Lean 两侧，见 §10）；须改 `segment.rs:384` 那句假的「对齐 segmentsOf」（§2.4）；Lean 侧须给 `SegmentConstruction.lean` 与 `Pipeline.lean` 加显式声明「本管线的线段不是缠论线段判据」 | 翻掉 `segment.rs:384`「对齐 `Origin.SegmentConstruction.segmentsOf`」这句声明；**不翻**任何段数读数（P2 本来就是现役） | **【正文】级**：`065:48`+`077:62`（至少三笔、前三笔重叠、笔数单数）+ `067:16-48`（特征序列全套）。**这是五组里唯一能标到【正文】的** |
| **B. 只裁 P3 出局（把 Lean 端到端管线的线段段改接完整判据）** | `formal/Origin/Pipeline.lean:137` 改接完整判据；须实现 `SegmentConstruction` 的 still-MISSING-A′（`:239-246` 已列四步）；`SegmentAutoConstruct` 的 `revSeq` 代理须一并改（G-3 候选 B） | 翻掉 `SegmentConstruction.lean` 的终止性证明基础（`nextSegmentEnd_pos` 依赖「消费 ≥1」，`:249-252` 已警告「若完整判据允许消费 0 根，终止性翻转」）⟹ **须重证终止性** | 同 A（原文依据同一批） |
| **C. 留 P1 作 `segment_algo="v0"` 诊断开关，显式标「非缠论线段」** | 只改文档 + 在 `a_segment_v0.py:148` 加声明；`server.py:425` 的 query 默认已是 v1 | 翻掉 `xianduan.md:199` 那句「v0贪心段数是v1的2-3倍」的**定位**（从「已验证结论」降为「诊断口径对比」） | 【设计选择】级 |
| **D. 全删 P1** | 删 `a_segment_v0.py:148-180`；但 **`Segment` dataclass 也在该文件**，被 **30+ 个模块与测试 import**（穷举 `grep -rn "from newchan.a_segment_v0 import"` = 30 个文件）⟹ 须先把类型搬走 | 翻掉 `tests/test_segment_v0.py`（约 15 个断言）、`tests/test_segment_v0_vs_v1_comparative.py`（10/10）、`tests/test_real_data_e2e.py:211-228`、`tests/test_new_bi_pipeline.py:332-333`、`tests/test_ab_bridge_overlay.py:208` | — |

### G-2 候选形态

| 候选 | 改哪些代码 | 翻掉哪些在案读数 | 名分 |
|---|---|---|---|
| **A. 定「方向性（局部 dir_state）」为正本，删中性外包络** | 删 `segment.rs:196-209` + 其唯一消费者 `analyze_termination`（`:281-323`）+ 两个测试（`:948`/`:959`）+ `:920` `process_feature_inclusion` 单测 | 翻掉「与 `Origin.SegmentFeatureComplete` 静态层对齐」这条契约（`segment.rs:275`）——**删掉后 Lean 静态层在 Rust 侧没有对应物**；翻掉「Δ=2 段 / 1.2%」的复现路径（本已不可复现） | **【推论】级**（`067:20` 归约到 K 线包含处理 + `inclusion.rs:49-55` 实装 + 62/65 课）。**注意：`[新缠论:推论]`，须附推理链，不能标【正文】** |
| **B. 定方向性为正本，但保留外包络作 Lean 对拍锚（改名 + 显式标「仅对拍，禁生产」）** | 只加声明 + 加一条断言/测试锁「生产路径不得调 `process_feature_inclusion`」（**当前无此锁**，§4.4 限定 3） | 不翻读数 | 同 A |
| **C. 先裁「局部方向 vs 全局方向」（§4.3 那条票面漏的第三线）再裁 A/B** | Lean `SegmentAutoConstruct.lean:87` 的 `processUp` 固定 ⟹ 若改局部，须重写 `buildFeatureSeqAux`（`:69-81`）并重证终止性 | 翻掉 Lean 侧包含处理与生产的「配套」自述 | 未查到原文对「方向由全局还是局部定」的表述 ⟹ 只能标【设计选择】或【推论】 |

### G-3 候选形态

| 候选 | 改哪些代码/Lean | 翻掉哪些在案读数 | 名分 |
|---|---|---|---|
| **A. 定生产侧（同向笔 + 任意分型 + 重做包含处理）为正本，Lean 的 `revSeq` 代理标「已知不忠实」** | 只改文档 + 在 `SegmentAutoConstruct.lean:174` 把「代理」升级为「**已知不等价**，still-MISSING」 | 不翻读数（生产已是这套） | **【正文】级**：`067:38`（向下一笔开始的序列的特征序列）+ `067:46`（只要有分型就可以）+ `078:54`（方向和原线段一致 / 必须按包含关系处理）。**三项全【正文】** |
| **B. 修 Lean：`revSeq` 真接同向笔第二特征序列** | 新增 `extractSecondFeatureSeq`（取 `segDir` 同向笔 + `buildFeatureSeqAux` 重做包含）；`extractSegEndData`（`:177`）改用它；`segmentEndUpB`（`:143`）的 `fractalInSeqB false` 改为「任意分型」`fractalInSeqB true \|\| fractalInSeqB false` | 翻掉 `SegmentAutoConstruct` 的现有引理链（凡引用 `revSeq = r.rest` 的证明）；须重证终止性（新增递归） | 同 A |
| **C. 两条序列都保留，显式命名区分（第一序列尾部 vs 第二特征序列）** | 加类型/命名 + 声明 | 不翻 | 【设计选择】级——**但「只找对偶分型」与 `067:46`【正文】相悖，这一支保不住** |

### G-4 候选形态

| 候选 | 改哪些代码 | 翻掉哪些在案读数 | 名分 |
|---|---|---|---|
| **A. 定 strict 唯一，删 optimized** | 删 `feature_seq.rs:284` `Optimized` 变体 + `:453` 的 `&& self.extend_mode == ExtendMode::Strict` 条件（变无条件）+ `extend_mode` 字段（`:306`/`:322`）；Python 删 `a_segment_v1.py:567` 参数 + `:431` 条件；rust 旧引擎删 `segments_from_strokes_v1` 的 `extend_mode_strict` 参数（`segment.rs:731`/`:772`）⟹ **连带改 `orchestrator.rs:857/:860` 与 `segment_layers.rs:643/:807` 四个调用点** | 翻掉 `tests/test_segment_gap_classification.py:401`（strict 应比 optimized 多断段）、`:418-421`（optimized 不断段）、`tests/test_rust_segment_equivalence.py:156`／`:226` 两个 parametrize；翻掉 `:11` 那条「BZ 全年 strict 28 段、optimized 159 段」读数的**可复现性**（optimized 臂消失后无法再测） | **【067课答疑】级**（`067:170`/`:196`）。**标不到【正文】**——§6.3 已说明 `067:46`【正文】管的是另一件事 |
| **B. 保留 optimized，显式标「非缠论口径，仅诊断」+ 加锁「生产不得构造」** | 加声明 + 加断言/测试；`analysis/_xcheck_secondkind_v1.py:46` 保留 | 不翻读数 | 同 A（strict 侧名分不变） |
| **C. 维持现状（不裁，继续标灰区）** | 无 | 无 | — **但 §6.2 的 5.7 倍差与 §6.3 的原文相悖两条，说明「灰区」这个定性本身待复议** |

**G-4 与 D-5 的口径一致性要求**：见 §6.5 三条（处置动作同型／名分层级同型／「灰区」判定标准同型）。
**⚠️ 本报告未拿到 D-5 票面正文 ⟹ 这一条给的是结构对齐点，不是结论。**

### G-5 候选形态（按 §7.1 订正后拆成 G-5a / G-5b）

| 候选 | 改哪些代码 | 翻掉哪些在案读数 | 名分 |
|---|---|---|---|
| **G-5a-A. 统一为「无限」（theta_v0 现状），rust 旧引擎与 Python 的 50 去掉** | `rust/src/segment.rs:413` 删常数 + `:593` 传 0；**但 `orchestrator.rs:215`、`segment_layers.rs:540`/`:838` 用它当 checkpoint margin**（`trigger_k+1+MARGIN>n_strokes`）⟹ **无限化后这个 margin 无定义，checkpoint 稳定性判据须重设计**（`orchestrator.rs:110` 是该判据的说明）；Python `a_segment_v1.py:367` 改 + `segment_engine.py:26` 连带改 | 翻掉「50 在目标数据域非约束性」这条读数的用途（它本是为**保留** 50 辩护；无限化则不再需要它）；**不翻** golden 跨语言等价（若 Python 同步改） | **【设计选择,默认值】级**——`second_kind.rs:39` 自述「非缠论可导」，原文零依据 |
| **G-5a-B. 统一为 50** | `config.rs:65` 改 `second_seq_scan_window: 50` | 翻掉 theta_v0 的「bit-exact 优先于性能」承诺（`second_kind.rs:99-100` 逐字「默认全扫描（无 TAIL_WINDOW/MAX_SCAN 截断）——bit-exact 优先于性能」）；翻掉 `second_kind.rs:111` 的 `scan_window=0` 硬编码（`:109-110` 自述「与 feature_seq::second_seq_has_fractal 完全同构（scan_window=0 = 无限全扫）」） | 同上 |
| **G-5a-C. 保持两侧不同，显式登记为「引擎级设计选择差异」** | 只加声明 | 不翻 | 同上 |
| **G-5b（W2=TAIL_WINDOW）：确认无分歧，撤出本票** | 无（三侧都是 7） | 无 | — |

**G-5 的公共前置**：无论选哪支，**须先把票面 G-5 拆成 W1／W2 两条**（§7.1），
并**把「窗口 7→∞ 输出不变」这句实测重挂到 W2 名下**，同时补标「出处 harness 已退役、
不可复现」（§3.1）。**仓内可跑的窗口等价 harness 是 `profile.rs:104`（`#[ignore]`，
测 0/50/200，不测 7）。**

---

## 10. 反例自评 + `.chanlun/definitions/xianduan.md` 现状核

### 10.1 `.chanlun/definitions/xianduan.md` 现在写的是哪一套

**版本 v1.3，状态「已结算」，最后更新 2026-02-16。**（未改动该文件。）

**写的是「特征序列状态机（v1）」，且已有胜负决断。** 逐字（文末「v0/v1 口径决断」节）：

> **决断日期**: 2026-02-16
> **结论**: v1（特征序列法）为唯一正式口径。v0（三笔重叠法）降级为参考实现。
> **含义**：v0 不进入正式管线，不作为下游模块的输入源；上层模块（中枢、走势、买卖点）只对接 v1；
> v1 为唯一符合原文 §67 课特征序列定义的实现。

定义总纲（`:126`／`:131-133`）逐字：

> 「线段是由**至少三笔**组成的结构，且前三笔必须有重叠部分。」
> 「### 线段划分定理　线段被终结，当且仅当至少被"有重叠部分的连续三笔"中的某一笔终结。
> 等价表述：**旧线段终结 = 新线段形成**。」

口径 B 规则（`:160-171`）已含第67课五步（构造特征序列→包含处理→标准序列找分型→两种情况→
新段前三笔重叠结算锚），且 `:207`／`:211-215` 记录了第二特征序列的修复
（「`_second_seq_has_fractal()` 从**同向笔**独立构建第二特征序列」）。

### 10.2 与生产实装对不对得上 —— **对不上，列差异（不改它）**

| # | 差异 | 证据 |
|---|---|---|
| 1 | **完全不提 theta_v0 与 Lean**。`grep -n "Lean\|theta_v0"` **全文零命中** ⟹ 现役 Rust 主引擎（`rust/src/theta_v0/parser/segment.rs`）与形式化层（`formal/Origin/`）**都不在正本文档的视野里** | 检索式 `grep -n "Lean\|theta_v0\|rust" .chanlun/definitions/xianduan.md` |
| 2 | **G-1 的第三范式（P3，遇反向笔即切／恒 2 笔）零记载** | 同上 |
| 3 | **G-2 零记载**：`grep -n "方向性"` **零命中**。文档只写「包含处理」（`:57/:58/:69/:103/:162/:166`），从未把「方向性 vs 外包络」当条目 | `grep -n "方向性" .chanlun/definitions/xianduan.md` |
| 4 | **G-4 零记载**：`grep -n "缺口封闭"` **零命中**；`extend_mode`／strict／optimized 均无记载。文档只有「有/无缺口两种情况」 | 同上 |
| 5 | **G-5 零记载**：`MAX_SECOND_SEQ_SCAN`／`TAIL_WINDOW`／`second_seq_scan_window`／「扫描窗口」**全部零命中**（只有测试类目名「尾窗扫描」出现在 `:215`） | 同上 |
| 6 | **读数与票面矛盾**：`:199` 逐字「**已验证**：v0贪心段数是v1的2-3倍」，与票面「406 vs 237（70%）」≈1.71 倍**不同量级**，且引的是另一个测试 | §3.1 第 4 条 |
| 7 | **`:207` 的「已修复」只覆盖 Python 侧**，未记录 Rust theta_v0 的 BUG-08（`second_kind.rs:90-95`，外包络误用）——**同一件事在两侧各修一次，文档只记了一次** | §4.2 |

⟹ **关票判据「正本落 `.chanlun/definitions/xianduan.md`」目前一条都没落**：
该文档停在 2026-02-16 的 v0/v1 二选一阶段，**G-1（第三范式）／G-2／G-4／G-5 四组在文档里
根本没有条目**，G-3 只有 Python 侧半条。**裁完必须新增四组条目 + 订正 §10.2 第 6、7 两条。**

### 10.3 反例自评：本报告最可能错在哪

| # | 最可能错的断言 | 什么证据能推翻它 |
|---|---|---|
| 1 | **★「Lean `segmentsOf` 在生产笔流上恒切 2 笔」（§2.3）** —— 这是本报告最强、也最可能被翻的推论。风险点：它依赖「相邻笔方向严格交替」，而**「交替」是我从 Rust 实装（`stroke.rs:30/:70`）+ property 测试（`:457`）读出的**，不是从 Lean 侧读出的。Lean 端到端管线的 `strokes` 来自 `OriginInput`（`Pipeline.lean:128`），**其取值域没有任何约束**。 | ① 若 Lean 侧存在某个约束 `OriginInput.strokes` 必须交替的引理/结构（我 grep `formal/` 未找到，但**没有穷举 `formal/` 全部 200+ 文件**）⟹ 结论方向不变但依据要换；② 若生产在某些边界（如 `_extend_prev_stroke` 回写、S-4 那组）会产出**连续同向笔**，则「恒 2」变「≤2」，接受集不相交的结论仍成立但「恒」字要撤；③ 最硬的推翻方式 = 在 Lean 里 `#eval segmentsOf` 一条真实交替笔列（本报告**未跑 Lean**，只做了源码推导 + 引该文件 `:213` 的自述注释） |
| 2 | **「`analyze_termination` 生产不可达」（§4.4）** | ① 若某个 pyo3 binding／`examples/`／`benches/`／外部 crate 调它（我只 grep 了 `rust/`，**未 grep `frontend/`、`trading_system/`、`prototypes/`、`topological-computation/`**）；② 若它被 `#[cfg(feature=...)]` 门后的代码调用 |
| 3 | **G-2「4 处 vs 5 处」计数** | 若存在第 6 处方向性包含处理（如 `topological-computation/`、`prototypes/` 下的独立实现）。我的穷举只覆盖 `rust/src`、`src/newchan`、`formal`、`tests`、`scripts`、`analysis`、`topological-computation`（后者只按「特征序列」词命中筛），**未逐文件读 `prototypes/`、`frontend/`、`trading_system/`** |
| 4 | **「406/237 不可复现」（§3.1）** | 若 `_xcheck_oklo_strokes.json` 能从某个 data cache／外部路径重建（`analysis/_xcheck_secondkind_v1.py` 一类脚本可能能重新生成它）⟹「不可复现」降为「需重跑一步前置」。我**未尝试重建**（只读工序） |
| 5 | **G-4「optimized 无生产构造点」** | 若 `frontend/`、`trading_system/`、NT 适配层（在案 1569 行已实装）里有传 `extend_mode="optimized"` 的路径。我核了 `.github/`、`scripts/`、`src/`、`tests/`、`analysis/`、`rust/src`，**未核 `frontend/`、`trading_system/`、`nt` 适配层** |
| 6 | **§8 的原文引文** | 引文由子代理按两步口径核出并给了界行号，**我本人未逐条回 grep 复核每一行**。最需要复核的是 `067:46`、`067:196`、`078:54`、`077:62` 四条（它们承重最大：分别否证 Lean 对偶分型、支撑 strict、坐实同向笔+包含处理、否证 P3 恒 2 笔） |
| 7 | **§6.5「与 D-5 口径一致」** | **本报告没读过 D-5 票面** ⟹ 这一整条可能对错不明。须拿到 D-5 正文后重写 |

### 10.4 明确的「测不出来 / 查不到」清单

1. **未跑任何 Lean**（`lake env lean` / `#eval`）⟹ §2.3、§2.5、§5.2 的 Lean 侧结论是**源码推导**，非机器验证。
2. **未跑任何回测/harness**（406/237、237→237、28/159 三组读数全部为**出处考据**，非复现）。
3. **`_xcheck_oklo_strokes.json` 不存在且从未入库** ⟹ OKLO 两组读数**当前仓内无可执行复现路径**。
4. **「窗口 7→∞」这个具体对比，仓内无可跑 harness**（`profile.rs:104` 只测 0/50/200）。
5. **`071:42`+`078:54` 那条「分界点两侧禁包含处理」是否在生产落实、落在哪一行 —— 未核完**（§8.2 末）。
6. **D-5 票面未读**（§6.5、§9 G-4 一致性条）。
7. **未核目录**：`frontend/`、`trading_system/`、`prototypes/`、NT 适配层 —— 上述可达性断言的覆盖边界。
8. **《市场天道》页码为块序推算的近似值**，非逐页精确核验（§8.5）。

---

## 附录 A：`grep -rn "特征序列"` 文件级命中计数（穷举方法留痕）

命中 `--include="*.rs" --include="*.py" --include="*.lean"`，排除 `rust/target`，共 **56 个文件**。
前 12 名（用于定位 G-1/G-2/G-3 载体）：

```
57 formal/Phase2/Claim10_SegmentV1.lean
47 formal/Origin/SegmentFeatureComplete.lean
42 rust/src/theta_v0/parser/segment.rs
40 rust/src/theta_v0/parser/second_kind.rs
38 formal/Origin/SegmentFeatureSeq.lean
24 src/newchan/a_segment_v1.py
21 scripts/segment_engine_diagnosis.py
13 scripts/codex_segment_diagnosis.py
13 rust/src/theta_v0/parser/feature_seq.rs
11 analysis/unified_topology_three_paths.py
10 tests/test_segment_gap_classification.py
10 src/newchan/a_feature_sequence.py
```

**已回补（原列为覆盖缺口，已查完）**：`formal/Phase2/Claim10_SegmentV1.lean` —— 见 §11。
**仍未读完**：`formal/Origin/SegmentFeatureComplete.lean`（47 处，只读了 `:469`/`:477` 两行）。

---

## 11. 回补：`formal/Phase2/Claim10_SegmentV1.lean`（★修正 §2.4 的措辞，并新增两处票面未提的分歧）

**Lean 里确实有 P2（特征序列 v1）的形式化**，在 `formal/Phase2/`，不在 `Origin/`。
`:9–15` 逐字：「本模块形式化线段划分 **v1 特征序列法**（第67课）的真完全分类……
本 claim10 补全 v1：第67课特征序列分型 → 线段划分两种情况穷尽 → 第77课方向/奇数性
→ 第78课古怪线段 + "顶高于底"硬约束。」

### 11.1 但它不是「跑得起来的管线」，也不接 Origin

- **它是谓词层，不是构造层**。API 穷举（`grep -n "^def |^theorem |^inductive |^structure "`）：
  有 `featureElements`（`:128`）、`classifyTermination`（`:232`）、
  `Segment.wellFormedV1`（`:348`，`Prop`）、`inclusionAllowedAtBoundary`（`:269`）等，
  **没有任何 `List Stroke → List Segment` 的划分函数**。⟹ 它**判定**一个给定线段是否良构，
  **不产出**线段划分。
- **不被 Origin 管线接入**：`grep -rn "Phase2" formal/Origin/Pipeline.lean
  formal/Origin/SegmentConstruction.lean` = **零命中**；`:34–37` 自述「本模块 **自包含**
  （不 import Formal.* / 不 import Phase2.Claim5）」；`formal/lakefile.toml:20` 把它列为
  **独立 root**。
- **也没有包含处理的合并函数**（只有 `Interval.hasInclusion` 谓词 `:137`）⟹ **不承 G-2**。

**⟹ §2.4 的结论需要收窄措辞（订正我自己）**：准确说法是
「**`Origin/Pipeline.lean` 那条端到端管线**跑的是 P3（每两笔一段），不是生产那套」；
**不能**说「整个形式化层都没有生产那套」——`Phase2/Claim10_SegmentV1.lean` **有** P2 的
判据形式化，只是**它与端到端管线之间没有连线**。
⟹ 更准的病名：**形式化层内部裂成两半**——
「能跑的那条（Origin 管线）用的判据不对」＋「判据对的那条（Phase2 Claim10）跑不起来且没接线」。

### 11.2 ★不要把 `v1_refines_v0` 当成 G-1 的严格包含链（#815 同型陷阱，方向相反）

`Claim10_SegmentV1.lean:489` `theorem v1_refines_v0 (s) (h : s.wellFormedV1) : s.wellFormedV0Skeleton`
+ `:499` `theorem v0_not_imply_v1`，`:497` 逐字「**二口径严格不等（v1 ⊊ v0）**」。

**这两条定理不能用来支持 G-1 的接受集关系**，理由（点名到行）：

- `:476` `def Segment.wellFormedV0Skeleton (s : Segment) : Prop := s.strokes.length ≥ 3`
  ⟹ **被比较的「v0」只是「笔数 ≥3」这一条下界**，**不是** v0 的算法
  （三笔重叠 + 恒 3 笔 + `j+=2`）。
- 该文件**自己已诚实标出**这个边界，`:484–487` 逐字：「★严格边界（codex 吸收）：本定理只证
  "v1 ⟹ length≥3"，**未** 证 "v1 ⟹ claim5 的完整 v0 谓词"……完整 v1⟹claim5-v0 桥接留给
  integrator（须先统一口径）。」
- `:499–505` 的见证是**四笔线段**（`List.replicate 4`）——用「4 不是奇数」违反 v1 的奇数性。
  ⟹ 反例走的是**奇数性**，与三笔重叠算法无关。

⟹ **§2.5 的「P1 与 P2 互不包含且交非空」不被 `v1_refines_v0` 推翻**：后者是
「`wellFormedV1` ⊆ `length≥3`」，与「P1 的输出集 vs P2 的输出集」是两回事。
**若有人引 `:497` 那句「v1 ⊊ v0」来定 G-1，就是 #815 的同型误读（把弱化谓词当算法）。**

### 11.3 ★新增分歧一：前三笔重叠的**相切边界**，P1 用 `<`、P2 用 `<=`（现役，票面未提）

| 侧 | 判据 | 相切（`max_lo == min_hi`）算不算重叠 |
|---|---|---|
| **P1**（Python v0） | `a_segment_v0.py:162` `if overlap_low < overlap_high:` | **不算**（严格 `<`） |
| **P2**（Rust theta_v0） | `rust/src/theta_v0/parser/segment.rs:140` `max_lo <= min_hi` | **算**（`<=`；`:136-138` 还专门打了 `tangent` 探针计数；测试 `:818-824` `three_stroke_overlap_tangent_counts_as_overlap` 锁住「相切=重合 ⟹ true」） |
| **Lean Claim10** | `:471-474` 自述 H4 用 `≤`；claim5 v0 用 `<` | 两个 Lean 模块**自己也不一致** |

`Claim10_SegmentV1.lean:471–474` 逐字：

> 「★H4 口径差异诚实标注（codex 发现）：本模块 H4 前三笔重叠用 `≤`（闭区间，允许边界相切），
> claim5 v0 用 `<`（开区间，严格重叠）。原文（067/077"必须有重合的部分"）**未明确边界相切
> 是否算重合**——这是一个 **未结算的口径细节**（边界相切=重合？）。……本模块不擅自裁定（保留张力）。」

⟹ **G-1 除了「段体多长」之外，还有一条「重叠门槛在相切处怎么判」的现役分歧，
四个载体给出三种答案，且原文明确未表态。**
（相关在案裁定：`tests/test_rust_segment_equivalence.py:6–13` 记 #246/#277/#288
「相切边界（三笔重叠含端点 `<=`、缺口谓词严格 `>`，对齐 Lean Overlaps/HasGap）两侧同批切换」
——**那次切换只覆盖了 theta_v0 与 Lean Origin，没有覆盖 `a_segment_v0.py`。**）

### 11.4 ★新增分歧二：Claim10 自己对「第二特征序列找什么分型」也自相矛盾（接 G-3）

同一文件的溯源段里两句相邻却相反：

- `:46` 逐字：「第二种(第1、2元素有缺口→须第二特征序列出**反向分型**才终结)。"只有两种可能。"」
- `:47` 逐字：「第二种简化（:46）：第二特征序列的分型**不再分第一二种情况，只要有分型即可**。」

⟹ **前一句是「对偶分型」（＝ Lean Origin `SegmentAutoConstruct` 那套），后一句是
「任意分型」（＝ 生产那套）**，两句都挂在第67课名下。
按 §5.3，`067-第67课.md:46`【正文】的原话是「只要有分型就可以」⟹ **`:47` 对、`:46` 那句
「反向分型」是转述时加进去的**，原文里「反向」二字出现在 `067:38`【正文】的
「出现**底**分型」（对向上线段而言），那是**具体举例**，不是「必须对偶」的一般判据。
⟹ **G-3 的「对偶 vs 任意」之争，在这份 Lean 文件的注释里就已经并存了**，
这是它扩散到 `SegmentAutoConstruct.lean:143`（`fractalInSeqB false revSeq`）的一条可疑源头。

### 11.5 Claim10 已形式化的一条、正好填上 §8.2 末尾那个开口（Lean 侧）

`inclusionAllowedAtBoundary`（`:269–271`）逐字：

```lean
269:def inclusionAllowedAtBoundary : TerminationCase → Bool
270:  | TerminationCase.firstKind => false   -- 第71课：转折点两边不包含
271:  | TerminationCase.secondKind => true   -- 第78课:54：第二特征序列必须包含
```

`:259–267` 的范围澄清逐字：「★范围澄清（复核吸收）：本谓词刻画的是 **分界点处（转折点两边）**
的局部包含作用域，**不是** 整个特征序列的包含规则……这 **不** 意味 firstKind 全程禁包含
——仅分界处禁。」

⟹ **§8.2 末尾那条「`071:42`+`078:54` 的分界点两侧禁包含，在实装里落没落」——
Lean 侧（Phase2）落了（谓词层）；生产侧（Rust/Python）**仍未核完**。
`feature_seq.rs:365–369` 的 `append` doc 只讲 71 课「假设转折点」的**先试不合并**逻辑，
**没有**「分界点两侧禁包含」的字样。⟹ **这一条仍是开口，且现在有了 Lean 侧的对照物
（`inclusionAllowedAtBoundary`）可以拿来对拍。**

