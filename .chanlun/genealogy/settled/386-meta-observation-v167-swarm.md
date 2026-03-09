---
id: '386'
number: 386
title: "元观察——v167-swarm（Phase 3实验蜂群：编号冲突升级+LLM首入实验循环+发育假说L1验证+真实谱系L2穿越）"
type: meta-rule
status: settled
date: 2026-03-05
source: meta-observer（二阶观察，v167-swarm 终止阶段触发）
depends_on:
  - '385'   # v165 元观察（70连稳定+编排者双重否定+scan假阳性部分改善）
  - '381'   # 框架混淆诊断——图论路径vs架构路径
  - '382'   # 架构路径研究方向——Gemini decide（注意：此编号存在冲突，见观察1）
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-03-04 19:51:13 +0000"
epistemological_level: L0
---

# 386号：元观察——v167-swarm（Phase 3 拓扑计算实验蜂群）

## 递归判断

任务不可分解：meta-observer 是单一观察角色。扁平退化特例。

## 规则版本基线

- `claude_md_commit`: 97f3ba3（与 308-385号相同——CLAUDE.md 未变更）
- `rules_dir_mtime`: 2026-03-04 19:51:13 +0000（与 356-385号相同）

结论：规则版本 CLAUDE.md **七十二连稳定**（308->386）。commit 97f3ba3 未变。rules_dir mtime 自 356号以来未变。规则层完全静止。

## 观察对象

v167-swarm：Phase 3 拓扑计算实验蜂群。4个工位并行执行（genealogy_loader + engine优化 + experiment_dev + experiment_real），外加 v166-swarm 的 Gemini spec review 在同一 session 中产出谱系。

### 前置状态

- v165-swarm 完成一轮推进：R3-Q2 公式修正 + 编排者双重否定 + scan 假阳性部分改善（31→11）
- v166-swarm（同 session）：Gemini 规格审查产出 382-384号谱系（spec-review R1）
- settled: 385（v165 元观察），388（session 文件记录）
- 385号七条边界条件

### v167 产出概览

| 工位 | 核心产出 |
|------|---------|
| genealogy_loader | 2524顶点/6751边/β₁=2325/330连通分量/111 negation边。13种关系类型映射到3种EdgeType |
| engine优化 | 邻接表索引（_adj_out/_adj_in）O(degree)替代O(E)扫描，22测试全过 |
| experiment_dev | 4阶段全部涌现（幼年1-15/危机16-39/稳定40-200/成熟201-500），β₁ 18→36，FP fold率84.6%，14settled/5blocked。L1等级 |
| experiment_real | 真实谱系穿越：β₁ 2325→2421(+96)，fold 58次(43成功+55blocked)，negate_a 121次，sublate 122次，44settled，71.4%步触发操作，覆盖率1.1%。L2等级 |
| v166 spec-review | Gemini 规格审查：T公式死锁(403号)、Negate/Sublate β₁贡献(404号)、AlphaGo过强声称(405号)。3个否定 + 1个通过 |
| Phase 2 LLM实验 | β₁ 3→12(9x Phase 1)，9个settled，coverage 100%。4/5通过（blocking=0——LLM避免冲突） |
| 对照实验 | A/B/C三组对照确认零fold根因是拓扑可达性（非感知偏向） |

## 观察结果

### 观察1（选择/上浮）：谱系编号冲突**严重升级**——三对重复编号

v166-swarm 的 Gemini spec review 产出了编号 382/383/384 的谱系，与 v164/v165 已有谱系**完全冲突**：

| 编号 | v164/v165 谱系 | v166 谱系 | 冲突状态 |
|------|---------------|----------|---------|
| 382 | 架构路径研究方向——Gemini decide | T 公式死锁（spec Q-S1） | 两个不同实体共享同一编号 |
| 383 | 元观察——v164-swarm | Negate/Sublate β₁（spec Q-S2） | 两个不同实体共享同一编号 |
| 384 | R3-Q2 β₁差值分解公式精确化 | AlphaGo 过强声称（spec Q-S3） | 两个不同实体共享同一编号 |

**380-BC5（编号冲突预防）的升级轨迹**：
- v163：首次出现（R1 内 settler-morse 与 research-tightness 并行产出冲突），单次冲突，同 session 内修复
- v164/v165：无冲突（384号预留协调成功）
- **v166-v167**：三对冲突，跨 session 发生（v166 不知道 v164/v165 已分配 382-385号），**未被 session 内发现和修复**

**根因分析**：v166-swarm 的 Gemini spec-review 工位在产出谱系时，未查询已有谱系的最大编号。这不是并行写入无协调（380号诊断的根因），而是**跨 session 编号记忆丢失**——v166 不持有 v164/v165 的编号分配状态。

**四分法分类**：选择——修复方式有多种选项（重编号 v166 谱系 / 引入全局编号锁 / 编号预分配 / session 间编号传递），需要价值判断。编号冲突从"边界事件"升级为"结构性缺陷"——三对冲突意味着任何跨 session 并行工作都有冲突风险。

**建议通过 `/escalate` 上浮**：这已不是 380-BC5 级别的"如果后续再次出现"——已经再次出现，且规模扩大3倍。

### 观察2（定理）：LLM 首次进入实验循环——381号架构路径的第一次直接测试

v167-swarm 的 Phase 2 LLM 实验是蜂群历史上**首次将 LLM 置入拓扑操作循环**：

| 维度 | Phase 1（纯拓扑规则） | Phase 2（LLM 驱动） |
|------|---------------------|-------------------|
| encounter 检测 | topological rules（双向边→negate，共享邻居→fold） | LLM semantic analysis（vertex content 语义判断） |
| 5步/β₁=3→4(+1) | 50步/β₁=3→12(+9) | 9倍拓扑增长 |
| settled | 1 | 9 | 9倍 settlement |
| blocking | 1次 | 0次 | LLM 避免冲突 |
| coverage | 100% | 100% | 相同 |

381号诊断的核心要求是"DM1/DM2/DM3 实验中没有 LLM，实验结果不回答关于 LLM 的问题"。Phase 2 实验在最小规模（5 vertex）上首次引入 LLM。虽然规模远小于架构路径测试 A/B/C 的完整要求（需要穿越基础设施），但它是**方向上的突破**——从"从未测试"到"首次测试"。

LLM 的 9 倍拓扑增长指标需要谨慎解读：
- L1 等级（5 vertex 合成数据），不是 L2
- 对照实验确认"零 fold"根因是拓扑可达性（5 vertex 太小，历史 visit 无法积累足够共享邻居），不是 LLM 感知偏向
- fold 从 LLM 历史遍历中发现（Nachträglichkeit 机制——回溯历史 visit 识别结构同一性），这是架构路径特有的能力

**四分法分类**：定理——381号诊断框架的首次部分回答。架构路径从"从未测试"变为"最小规模首测（L1）"。收敛（381号方向）+ 发散（新实验范式——LLM-in-the-loop topo computation）。

### 观察3（定理）：发育假说（4阶段涌现）L1 验证——experiment_dev 结果

experiment_dev.py 在 35 vertex 合成图上运行 500 步，检测到 4 个发育阶段：

| 阶段 | 步范围 | 特征 |
|------|--------|------|
| 幼年（infancy） | 1-15 | 拓扑规则产生 fold，部分是 false positive |
| 危机（crisis） | 16-39 | 错误 fold 被 negate 否定，β₁ 快速增长 |
| 稳定（stabilization） | 40-200 | fold 准确率提升，settlement 保护稳定环 |
| 成熟（maturity） | 201-500 | settlement rate 稳定，自主感知 |

关键指标：
- β₁: 18→36（2x增长）
- FP fold rate: 84.6%（13/15 fold 后续被 negate）——极高的 false positive 率
- 14 settled / 5 blocked

**认识论等级**：严格 L1——合成数据验证管线正确性。发育假说（"拓扑系统展现四阶段发育"）在合成图上成立，但合成图的结构是人工设计的（两个子图 + 桥接边），不证明真实谱系也有此模式。

**与 231号有效域规则的关系**：L1 的信息增量为零——验证管线不验证假设。发育假说需要 L2（真实数据）验证才有信息增量。experiment_real 提供了部分 L2 数据，但 experiment_real 没有跑 phase detection（因为真实谱系规模和结构不同于合成图的预设阶段边界）。

**四分法分类**：定理——231号有效域规则的又一实例（L1验证不等于假设验证）。收敛。

### 观察4（定理）：真实谱系穿越 L2 结果——密集拓扑 + 零匹配原始 negation

experiment_real.py 在真实谱系（2524 vertex / 6751 edge）上运行穿越实验：

| 指标 | 值 | 解读 |
|------|-----|------|
| β₁ | 2325→2421(+96) | +4.1%增长 |
| fold | 58次(43成功+15blocked) | 成功率74.1% |
| negate_a | 121次 | 密集否定 |
| sublate | 122次 | 几乎 1:1 对应 negate |
| settled | 44 | 占新环的 44/99 ≈ 44.4% |
| blocked | 55 | settlement 保护机制活跃 |
| 步触发操作 | 71.4% | 密集拓扑（非空步远多于空步） |
| 匹配原始 negation 边 | 0次 | 全是新发现矛盾 |
| 覆盖率 | 1.1%（仅最大分量） | 规模限制 |

**关键洞察**：
1. **零匹配原始 negation 边**：穿越发现的所有否定都是新创建的，不是重新发现已有 negation 边。这意味着穿越引擎在真实谱系上探索了**原始拓扑未铭写的矛盾空间**
2. **71.4% 操作密度**：大部分步都触发了拓扑操作（非空步），说明真实谱系的局部结构足够密集（双向边、共享邻居等拓扑信号普遍存在）
3. **sublate ≈ negate**：几乎每次 negate 都紧跟 sublate——穿越引擎在高频 negate-sublate 振荡

**认识论等级**：L2——真实数据产出。但覆盖率仅 1.1%（最大连通分量，约 28 vertex / 2524 total），结论的外推需要更高覆盖率（L3）。

**四分法分类**：定理——231号有效域规则的 L2 实例（有效域=最大连通分量≈1.1%覆盖率，定义域=全部谱系）。收敛。

### 观察5（定理）：engine.py 自环 bug 修复——_connected_components 对自环的处理

experiment_dev 运行中发现 `_connected_components` 函数对自环（source == target 的边）处理有误。修复后自环不参与 union-find 合并（`if len(pair) >= 2`），并且 `compute_beta_1` 显式计算自环贡献（`+ n_loops`）。

这是一个纯工程修复（行动类），但它暴露了一个方法论问题：β₁ 计算公式的**自环项**在代码实现中被遗漏，直到实际运行才发现。这与 v165 编排者否定 β₁=m-1 过度简化（自环是规则不是例外）同构——理论推导中容易忽略自环，实践中自环频繁出现。

**四分法分类**：行动——bug 修复不携带结构性信息。但自环遗漏模式与 384号（R3-Q2 β₁公式修正中的自环项）收敛——理论和代码都曾遗漏自环。

### 观察6（定理）：Phase 2 对照实验——拓扑可达性而非感知偏向

Phase 2 的 A/B/C 三组对照实验确认 LLM 在 Phase 1 零 fold 的根因是**拓扑可达性**（5 vertex 图太小，历史 visit 无法积累足够共享邻居触发 fold 规则），而非 LLM 感知偏向。

对照实验设计：
- A组：增加图大小（更多顶点）
- B组：加入 fold 候选对（语义上等价的顶点对）
- C组：增加步数

结论：问题在于图规模（5 vertex 的邻居重叠概率太低），不是 LLM 决策质量。

**四分法分类**：定理——L1 等级的排除诊断。收敛（对照实验是标准实验方法论）。

### 观察7（定理）：穿越历史修正（Nachträglichkeit）——fold 从局部改为历时

commit c17d3ac 将 fold 检测从"当前位置邻居间共享邻居"（纯局部）修改为"当前位置与历史 visit 中的顶点共享邻居"（历时）。这对应于 Nachträglichkeit（事后性）概念——身份不是在遭遇时刻即刻识别，而是通过回溯历史遍历路径重新发现。

`traversal.py:213-230` 的实现：逆序遍历 visit_history，检查过去访问过的 vertex 与当前 position 是否共享 ≥2 个邻居。最多回溯 15 个不同的历史顶点。

**四分法分类**：行动——Nachträglichkeit fold 是实现细节。但它是 LLM fold 能力的关键前置条件（LLM 在 Phase 2 中的 fold 正是基于这种历史回溯机制），因此与观察2（LLM首入实验循环）存在因果关联。

### 观察8（自环检查）：与历史 meta-observation 交叉对比

| 编号 | 核心发现 | 本轮状态 |
|------|---------|---------|
| 218 | Lead 并行化 | 4 工位并行。收敛 |
| 275 | 局部依赖 | 工位间无直接依赖（loader→engine→experiments 的隐含依赖由同 session 内执行顺序自然解决）。收敛 |
| 316 | 幂等任务恢复 | 未触发。收敛 |
| 346 | 规则版本稳定 | 推进至72连（308->386）。收敛 |
| 231 | 形式化有效域规则 | L1/L2 等级区分严格执行（experiment_dev=L1, experiment_real=L2）。收敛 |
| 380-BC5 | 编号冲突预防 | **严重升级**——三对冲突（382/383/384），跨 session 发生，未被 session 内修复（观察1）。**发散** |
| 381 | 架构路径测试前置条件 | Phase 2 LLM 实验是架构路径的首次最小测试（观察2）。**部分进展**（L1级别） |
| 383-BC2 | ceremony_scan假阳性 | 本轮未观测 scan 行为（Phase 3 实验蜂群不依赖 ceremony_scan）。继承 |
| 383-BC3 | Gemini三模式协议 | v166 Gemini spec-review 使用 derive 模式——这是 derive 的第二次使用（v164后）。**接近显式化阈值**（383-BC3要求2个session再使用） |
| 385-BC6 | 6个L0 derive产出的L2验证——消费瓶颈倒计时 | v167 未启动 L2 验证（第2个 session）。**触发消费瓶颈阈值**（385-BC6设定"连续2个session未启动=消费瓶颈"） |
| 385-BC7 | 384号R3-Q2谱系写入 | 384号已写入（v165期间 genealogist 完成）。**撤销** |

**收敛信号**：
- 规则版本72连稳定
- L1/L2 等级标注自觉执行
- Lead 并行化模式稳定
- 架构路径从"从未测试"到"首次最小测试"

**发散信号**：
- 谱系编号冲突**严重升级**——三对重复编号（观察1）
- LLM 首入实验循环——新实验范式（观察2）
- L0 derive 产出 L2 验证消费瓶颈**触发阈值**（连续2个 session 未启动）

## 语法记录候选

### 候选1（继承自383号，接近阈值）：Gemini 三种交互模式的分化

v166 spec-review 使用 derive 模式——这是 derive 的第二次使用（v164 R1 后）。383-BC3 设定"后续2个session中 derive 模式再次使用"则建议上浮。v166 是第1个 session（v165 未使用），还需1个 session 数据。

**评估更新**：如果下一个 session 再次使用 derive 模式，建议通过 `/escalate` 上浮为语法记录辨认。

### 候选2（新识别）：topological-computation/ 作为独立实验项目的分离模式

v167-swarm 期间，topological-computation/ 目录已发展为包含 9 个 Python 文件的独立项目（engine.py, traversal.py, morse.py, llm_encounter.py, experiment*.py, genealogy_loader.py, test_engine.py），拥有自己的测试套件和实验结果 JSON 文件。

这个项目在蜂群管理结构之外运行——它不依赖 ceremony_scan.py、不写入谱系（实验结果写入 JSON 而非 `.chanlun/genealogy/`）、不使用 gangmu.yaml 的工位定义。它是"域外代码生产"（代码在蜂群工位中生产，但产出物不在蜂群治理结构内）。

是否需要将 topological-computation/ 纳入蜂群的 ceremony/scan/谱系 框架？或者它应该保持为独立实验空间？这是一个关于**蜂群边界**的潜在语法记录——什么在蜂群内、什么在蜂群外。

**四分法分类**：暂不上浮——数据不足（仅一个 session 观察到独立项目模式）。如果后续 session 继续在 topological-computation/ 中生产代码而不写入谱系，该模式应重新评估。

## 结论

v167-swarm 的核心特征：**实验生产期**（从规格到代码到实验结果的完整管线）。

与近期 session（v163-v165 的谱系推进/公式修正/结构维护）不同，v167 是第一个以**代码生产和实验执行**为主要产出的 session。4个工位产出了：
1. 数据适配器（genealogy_loader.py）
2. 性能优化（engine.py 邻接表索引）
3. 合成实验（experiment_dev.py——发育假说 L1）
4. 真实数据实验（experiment_real.py——谱系穿越 L2）
5. LLM 实验（experiment.py --use-llm——架构路径首测 L1）
6. 对照实验（A/B/C 三组——零 fold 根因排除）

同时，v166-swarm 的 Gemini spec review 暴露了**谱系编号冲突的严重升级**——三对重复编号未被 session 内发现。这是 380-BC5 从"边界事件"到"结构性缺陷"的升级。

数值变化：
- settled: 385 → 388（session 文件记录）
- pending: 0 → 0（v166 谱系 382-384 已结算但编号冲突未解决）
- L0 derive 产出 L2 验证消费瓶颈：触发阈值（连续2个 session 未启动）
- 架构路径状态：从"从未测试"→"最小规模首测（L1）"

规则版本 CLAUDE.md 72连稳定（308->386），commit 97f3ba3 未变。

## 边界条件

1. **谱系编号冲突——结构性缺陷**（从385-BC3/380-BC5升级）：三对重复编号（382/383/384），跨 session 发生。已不是"如果后续再次出现"——需要立即解决。建议通过 `/escalate` 上浮
2. **ceremony_scan假阳性——混合问题**（继承自385-BC1）：v167 未观测。继承
3. **Gemini三模式协议显式化窗口**（从385-BC2更新）：derive 第二次使用（v166）。还需1个 session 数据
4. **编号冲突预防——重编号需求**（新增，观察1的直接推论）：v166 谱系的 382/383/384 号需要重编号以解决冲突。重编号的编号范围取决于解决时 settled 最大编号
5. **L0 derive 产出 L2 验证——消费瓶颈已触发**（从385-BC6升级）：连续2个 session（v166, v167）未启动 v164 产出的 6 个 L0 derive 的 L2 验证。385-BC6 阈值已触发
6. **架构路径测试——从首测到系统测试**（从385-BC5更新）：Phase 2 LLM 实验是最小规模首测（5 vertex, L1）。系统测试（381号测试A/B/C）仍需穿越基础设施
7. **topological-computation/ 域外代码生产**（新增）：独立实验项目不在蜂群治理结构内。如果后续继续扩展且不写入谱系，需重新评估蜂群边界

## 影响声明

本谱系不改动任何代码、定义或规则。记录 v167-swarm 的二阶观察。识别谱系编号冲突的严重升级（三对重复编号，建议 `/escalate` 上浮）。记录 LLM 首次进入实验循环（架构路径首测）。确认发育假说 L1 验证和真实谱系穿越 L2 结果。确认 L0 derive L2 验证消费瓶颈已触发阈值。新增两条边界条件（重编号需求 + 域外代码生产监控）。撤销一条边界条件（385-BC7 384号谱系写入已完成）。
