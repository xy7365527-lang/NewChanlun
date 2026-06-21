# 矛盾上浮：T 步骤c 全局加「5条件 ∧ MACD面积衰减」⊥ §6.6/§6.7 裁决「MACD 仅补 move/L1、recL2+ 禁 MACD」+ nf_only 缺口的算子方向

> 上浮人：T 算子 MACD 接入工位（2026-06-18 17:52，实装前架构核验）
> 触发：执行任务「给 T 算子步骤c（`is_trend_perfected`）加 MACD 面积背驰确认，5条件 ∧ MACD面积(C)<MACD面积(A)，全局，两者同时满足才产买卖点」。
> 读 `divergence.rs` + 设计文档 §6.6/§6.7 + 阶段2 报告 `analysis/T_vs_v3_comparison.md` 后，发现任务规格与**今天（同日）刚结算的编排者裁决 + 8 标的 L3 否定性结果**结构性冲突。
> 分类（四分法）：**选择**（MACD 加在哪些级别 / 用什么算子方向 / 是否先做前置甄别，需价值判断）+ **触及未结算开放轴**（move/L1 的 nf_only 真伪未经 L3 收益甄别）。
> 等级：**L3**（8 标的真实数据交叉验证，`T_vs_v3_comparison.md`）。
> 关联：`docs/unified_recursive_operator_T.md` §6.6/§6.7、`analysis/T_vs_v3_comparison.md`。

---

## 1. 停下来：为什么这不是可自决的定理或行动

任务要求是**全局、无条件**地把 step c) 改成 `5条件 ∧ 结构力度衰减 ∧ MACD面积衰减`。但设计文档 §6.6/§6.7（编排者 2026-06-18 裁决）+ 阶段2 L3 报告明文裁定：

> 「加 MACD 的缺口**仅限 move/L1 趋势市的 nf_only**（且需 L3 收益验证甄别 nf_only 是真买卖点还是趋势市过触发——开放轴）；**recL2+ 禁止 MACD 修正（T 已优）**。」（设计文档 §6.6 末，行 756-759）

两者来自**同一编排者、同一日期**，互相矛盾。我无法判断哪个是编排者最新意图：

- 若闷头执行全局 AND → 主动砍掉 recL2+ 处 T 已被 L3 证明正确、且 v3 根本没有的高层信号（= 用一次 commit 推翻一个刚结算的实证结论 = `no-patch-mentality` 妥协方案 + `no-workaround` 绕过矛盾）。
- 若擅自把任务「重新解释」为只在 move/L1 加 → 替编排者做了范围价值判断。
- 若「recL2+ 跳过 MACD」硬编码同时讨好两边 → 正是 `no-workaround` 禁止的「把矛盾两端分别硬编码为特例」。

故**不可自决**，走 /escalate。（工程层「MACD 怎么接进 Rust」我已设计好，见附录——本上浮只为概念层裁决。）

---

## 2. 精确描述矛盾

`AND` 是合取门：加上它，T 步骤c 产出的 type1 买卖点**单调减少**（结构满足但 MACD 未衰减的点被滤掉）。三个独立冲突点：

### 冲突① 范围：全局 ⊥ 仅 move/L1（recL2+ 已被 L3 证明 T 优）

阶段2 报告发现3（行 49-58）：低匹配率在两级别**成因相反**——

| 级别（T.level→ladder） | 主导缺口 | 成因 | 报告裁定 |
|---|---|---|---|
| move/L1 (L0→3) | `nf_only`（v3 有、T 无 = T 漏判方向） | 第37课5条件严格 vs nf 区间套触发宽松 | 候选补点，**需先 L3 甄别** |
| recL2+ (L≥1→≥4) | `t_only`（T 有、v3 无） | **v3 bug-c 实证**：σ-ascend 高层 nf 指数级稀疏 | **T 优于 v3，禁止 MACD「修正」** |

recL3+ 处 T 每标的产 14–163 个高层信号，v3 nf 近零（recL4+ ≈ 0）。这是 §5.2 bug-c「T 架构消除高层稀疏」的**首个 L3 实证**。全局加 AND → 在 recL2+ 砍掉这批 T 正确产出 → T 退回 v3 bug-c 水平。

### 冲突② 算子方向：AND 收紧 ⊥ nf_only 缺口需放宽

阶段2 主缺口是 **nf_only（T 漏判，T 产得太少）**。逐标的 move/L1 数据（nf_only ≫ t_only）：

| 标的 | move/L1 sell nf_only | t_only | move/L1 buy nf_only | t_only |
|---|---|---|---|---|
| OKLO | 411 | 155 | 461 | 164 |
| BTC | 4395 | 1162 | 4225 | 1305 |
| ES | 3527 | 854 | 2876 | 1177 |
| CL | 2825 | 980 | 2682 | 1094 |

`AND MACD` 让 T 产**更少** → nf_only **扩大**，nf 匹配率（matched/n_nf）**下降**。AND 能减少的只有 t_only（T 误判），但 t_only 在 move/L1 占比小、未经甄别，在 recL2+ 是 T 优（不能减）。

**任务步骤6 期望「匹配率提升」，但 AND 在 L3 数据下大概率使 nf_match_rate 恶化**——补漏判（nf_only）需要放宽判据或独立信号，AND 是反方向算子。

### 冲突③ 前置开放轴未结算

报告结果包边界条件（行 73）+ 对阶段3指向（行 62）明文：

> move/L1 的 nf_only「需 L3 收益验证甄别是『T 漏判的真买卖点』还是『nf 在趋势市的过度触发』——**当前对照不能区分二者（开放轴）**」。

即「move/L1 该不该加 MACD」本身尚未结算。任务直接加 MACD 跳过了这个前置甄别——若 nf_only 实为 v3 趋势市过触发噪声，则 T 的严格性反而是优点（报告行 73：「结论翻转为 T 优于 nf」），加 MACD 毫无意义甚至有害。

---

## 3. 涉及的定义 / 裁决

- **`docs/unified_recursive_operator_T.md` §6.5 划线**：背驰「结构形态 + 结构化力度」= 拓扑必然（T 内部）；「MACD 精确力度」= 度量剩余（groupoid 轴，**T 外部**，confirmed 层精化）。任务把 MACD 拉进 T 步骤c = 把度量剩余拉进 H⁰ 构造轴。
- **§6.6/§6.7 裁决（编排者 2026-06-18）**：纯结构优先渐进，MACD「仅在阶段2暴露缺口处加」，缺口经 L3 定位为 **move/L1 局部、非全局**。
- **第24课 `[原文]`**：MACD 是辅助（90%），配合中枢才 100%。任务的 AND（结构∧MACD）在缠论上正统，但与项目**实测有效域裁决**冲突。
- **`formalization-validity-domain.md`**：缠论定义域（所有 level 都该有力度衰减判据）≠ 有效域（L3 显示 recL2+ 用 MACD 反而有害）。全局加 = 有效域膨胀。
- **谱系 537（candidate/confirmed 双重性）**：T 步骤c 产 candidate；MACD 升 confirmed 在 groupoid 轴。AND 在步骤c 内 = 把 confirmed 判据坍缩进 candidate 层。

---

## 4. 谱系 / 先例比对

- **`project_divergence_gate_entry_exit_falsified`**（记忆）：MACD 面积背驰门控进出场在某 regime 否证——D<A<BH，门控砍 90% 进场 = 减暴露，强趋势跑输。**与本矛盾同构**：MACD AND 门 = 收紧 = 减信号，在强趋势 regime 有害。这是「MACD 收紧门方向错」的**第二次显形**。
- **`project_ph_settle_usage_boundary`**（记忆）：PH settle = candidate 必要条件、非操作充分确认。同构：纯结构 candidate ≠ confirmed，MACD 在 confirmed 层（§6.5）。
- **regime 函数主题**（`project_nrf_v4_strict_accounting` 等十余例）：纯拓扑替代率是 regime 函数（报告发现2）。全局统一加 MACD = 无视 regime 依赖 = 与项目反复确认的主题冲突。

先例一致指向：**MACD 收紧门不是全局普适，是 regime/级别局部的有效域问题**——这正是任务全局 AND 所抹除的。

---

## 5. Lead 的建议方案

| 选项 | 内容 | 取舍 | 是否守裁决 |
|---|---|---|---|
| **1 全局 AND** | 字面执行任务，所有 level 加 MACD AND | 砍 recL2+ T 优势，推翻 bug-c L3 实证；匹配率大概率恶化 | ✗ 违 §6.6 |
| **2 仅 move/L1 AND** | 只在 T.level=0(ladder3) 加，recL2+ 保持纯结构 | 守裁决范围，但 AND 仍补不了 nf_only 主缺口（冲突②未解），且跳过前置甄别（冲突③） | 部分 |
| **3 先 L3 收益甄别，再定 MACD 加法** | 先跑 move/L1 nf_only 的收益验证（真买卖点 vs 过触发），据结果决定加不加、用 AND 还是放宽算子 | 最严格（formalization-validity-domain），信息增量最高；最慢 | ✓ 完全 |
| **4 重定目标** | 若目标是补 nf_only，需放宽 T 判据/独立信号，不是 AND；若目标是减 t_only 误判，AND 仅适用 move/L1 且需先确认 t_only 是误判 | 直面冲突②的算子方向错 | ✓ |

**Lead 倾向**：选项 **3**（或 3→4）。理由：阶段2 是同项目刚产出的 L3 否定性结果，它精确定位了「缺口在 move/L1、且真伪未定」。在甄别 nf_only 真伪之前加任何 MACD 门，都是用先验信念覆盖刚得到的 L3 数据——正是 §6.6 裁决要避免的「MACD 兜底掩盖纯拓扑真实有效域」。若编排者已知此代价仍要全局推进（选项1），我立即执行（技术已就绪，见附录）。

---

## 6. 需要编排者决断的问题（缠论语言）

1. **划线**：背驰的「精确力度确认」（MACD 面积 C<A）应留在 groupoid 轴（区间套定位时的 confirmed 精化，§6.5 原裁决），还是下放进 T 步骤c（走势完美判定本身就要求 MACD 衰减）？
2. **级别范围**：若下放进步骤c，是否所有级别都加？阶段2 L3 证明 recL2+（递归层）T 纯结构已优于带 MACD 的 v3——递归层的「走势完美」要不要 MACD 这道度量门？
3. **算子方向**：当前 move/L1 的主缺口是 **T 漏判（nf_only）**——这意味着 T 的第37课5条件**过严**。再加一道 MACD AND 门会让 T **更严、漏判更多**。编排者的目标是「让 T 追上 v3 的信号密度」（需放宽），还是「让 T 砍掉自己的误判」（需收紧，但 move/L1 误判 t_only 占比小）？
4. **前置甄别**：是否先做 move/L1 nf_only 的 L3 收益验证（甄别真买卖点 vs v3 趋势市过触发），再决定 MACD？还是接受不甄别直接加？

---

## 附录：技术准备（裁决后可立即执行，工程层已就绪）

无论裁决哪个选项，MACD 接入方案已设计好，保持 T 的 standalone 纯结构定位不被 close 价格污染：

- **数据可得性**：对照脚本中 `orchestrator` 已逐 bar 喂入 close、`enable_macd_divergence=True`、内部已有 `bi.merged_to_raw()` 映射 + 在线 MACD（`orchestrator.rs:959`）。`compute_macd_batch`（`macd.rs`，已 bit-exact）可在调用方算出 raw-bar 上的 hist。
- **坐标系陷阱**：T 的 `Unit.start_bar/end_bar` 来自线段端点 = **merged bar 坐标**；MACD hist 在 **raw bar**。必须经 `merged_to_raw` 转换（与引擎层 `divergence.rs::compute_force` 同款），否则面积全错。
- **注入方案（与 `inner_zhongshu_count` 同构）**：`Unit` 增携 `area_pos`/`area_neg`（按该单元 raw-bar 区间的 hist 正/负面积，**不预先按方向取**）。`leg_strength` 的 MACD 档按**段所属趋势方向**统一取（上涨段 ΣΣarea_pos，下跌段 Σarea_neg）——保住面积可加性 + 方向一致性。封装时 `area_*` 随 `inner_zhongshu_count` 一起求和上传，递归层自然聚合。
- **FFI**：`run_recursive_t` 扩展接收 `hist` + `merged_to_raw`（或预算好的 per-segment area），由 a₀ 线段层注入。
- **声明一致性（强制）**：实装后必须同步改 `divergence.rs` / `ffi.rs` 模块头「纯结构、零 MACD」声明——否则文档声明零 MACD、代码却用 MACD = 声明膨胀（`no-patch-mentality` 禁止）。
