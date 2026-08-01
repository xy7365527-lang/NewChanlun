Part of #787

## Question

**笔与线段的教义正本**——第二批概念票第 2 张。清点见 [总缝规则过筛 #809](https://github.com/xy7365527-lang/NewChanlun/issues/809)。

**两个概念合并成一张票，不拆**：[#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 的理由是 G-1/G-3 直接依赖 S-1 定出的笔集合，拆开会互踢。

### 笔（4 组）

| 组 | 问题 |
|---|---|
| **S-1** | 「至少 N 根独立 K」算的是什么量——`merged_gap≥2 ∧ raw_gap≥3` 双条件（`a_stroke.py:176`）↔ `source_index` 之差 `>3` 单条件（`theta_v0/parser/stroke.rs:60`）↔ `noSharedBar ∧ barsBetween ≥ 3` 不考虑包含（`SegmentFeatureSeq.lean:374`）。**三者在有包含合并的区间上给出不同笔集合，零对拍。** |
| **S-2** | 旧笔存废——族 I 支持 `mode=strict` 老笔 ↔ theta_v0「旧笔禁用」。（配置档并存，非失败兜底。） |
| **S-3** | gap 不足时怎么回退——`j+=1` 不动起点 ↔ `i+=2` 放弃起点。 |
| **S-4** | 已确认的笔能不能回写改写——`_extend_prev_stroke` 原地改写 ↔ theta_v0「已确认结构不可回写重分解」。 |

### 线段（5 组）

| 组 | 问题 |
|---|---|
| **G-1** | 线段的构造范式——三笔重叠法恒 3 笔（`a_segment_v0.py:147`）↔ 特征序列增量状态机（3 处）↔ 遇第一根反向笔即切（`SegmentConstruction.lean:63`）。**且 `Pipeline.lean:95` 的 Lean 端到端管线用的就是第三种 ⟹ 形式化层跑的不是生产那套线段定义。** 实测量级差：406 段 vs 参考 237 段（70% 过分段）。 |
| **G-2** | 特征序列包含处理是不是方向性的——方向性（4 处）↔ 外包络无条件 min/max（`theta_v0/parser/segment.rs:196`，自称「中性实现」）。 |
| **G-3** | 第二特征序列是什么——同向笔新建、任意分型即确认（3 处）↔ 取剩余**反向**元素、只找对偶分型（`SegmentAutoConstruct.lean:177`，`:174` 自承是「代理」）。**根本不是同一条序列。** |
| **G-4** | 缺口封闭降级要不要——`strict` 做（67 课）↔ `optimized` 不做。两档并存，默认 strict。**灰区**：不是失败兜底，但两档并存，与 D-5 同型，两票口径须一致。 |
| **G-5** | 第二特征序列扫描窗口——`MAX_SECOND_SEQ_SCAN=50` ↔ 无限。附单点实测「窗口 7→∞ 输出不变」——**单点实测一致，未证等价**（090）。 |

### 关票判据

正本落 `.chanlun/definitions/bi.md` 与 `.chanlun/definitions/xianduan.md`（两份），各附受影响代码清单（点名到行号）。

### 纪律

`/grill-with-docs`，**一次一问**，agent 不得替人答。落完文档才关票。
