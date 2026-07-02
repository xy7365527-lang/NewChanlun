---
id: "626"
number: 626
status: 生成态   # genealogist 结构维护诊断：block-topology 缺增量迁移入口的结构缺口精确化。最终结算待编排者 /ritual。依赖 178。（待#37 codex 裁定）
date: "2026-06-27"
type: 矛盾发现
depends_on: ["178"]
related: ["549", "566a", "624", "621", "036", "090", "161", "012"]
title: "block-topology 缺增量迁移路径——仅有破坏性全量重建(run_migration)，无 566-575 之类新增谱系的增量补齐入口；后果三联：(1)566-575(含566a)11条 settled 自迁移后未入 id_mapping(meta.json 仅到565)→ceremony_scan 两检测器(genealogy_anomalies/scan_topology_mapping_gap)持续报 missing_block_mapping+unmapped；(2)last_mapped_genealogy=530 与实际(565)脱节(全量重建丢该字段，无增量入口更新它)；(3)run_migration 全量重跑会清空 content_enrichment(314blocks/1148concepts)元数据并因 settled 内容漂移产生重复区块=178号Q1『一次性升格』决断的下游缺口(升格路径未配增量入口)"
negation_source: cc
negation_model: "genealogist 结构维护勘察(2026-06-27)：读 migrate_to_block_topology.py(run_migration L330-337 整体重建 meta，丢弃 content_enrichment/last_mapped_genealogy 字段) + content_enrichment_migration.py(enrich_single_file L133 id_mapping 缺失即 skip，不创建 event 区块) + ceremony_scan.py(get_topo_context L723/scan anomalies L225 均以 meta.json id_mapping.keys() 为映射真相)，确认无增量补齐入口"
negation_form: expansion
# expansion：178号Q1『升格——不是包装旧系统，是一次性转换。建系后只有一套系统』在持续运作中暴露隐藏缺口——
#   『一次性转换』脚本(run_migration)是破坏性全量重建，建系后新增谱系(566-575...)无增量入口入系。
#   178号下游推论1已自述『区块系统是谱系的补充/升格，两套检测需并存』，但未配增量迁移入口——
#   导致谱系持续增长而 block-topology 映射停滞在 565。这是 178号决断在执行中膨胀出的下游缺口。

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：178号『一次性升格后只有一套系统』隐含命题『迁移入口足以维持单一系统同步』在『谱系持续新增』场景下的有效性
# 实际拓扑后果：block-topology 的『event 区块层』与『settled/*.md 谱系层』分裂——
#   (a) 001-565：已迁移，映射存在
#   (b) 566-575(含566a)及之后：settled 已结算但未入 block-topology，映射缺失
topo_effect: "split:178-one-time-upgrade-single-system:block-topology-event-layer-lags-behind-settled-genealogy-layer-no-incremental-entry"
# split：178『一次性升格后单一系统』分裂为两段——
#   (a) 已迁移段(001-565，映射存在) + (b) 滞后段(566-575 及之后，settled 已结算但 block-topology 无映射)
#   根因=只有破坏性全量重建入口，无增量补齐入口；scope=block-topology 映射同步机制(178 升格路径的下游)

# 矛盾（type=矛盾发现 必填）
contradiction:
  description: "block-topology 的迁移入口只有 run_migration(migrate_to_block_topology.py)——它是破坏性全量重建：L330-337 整体重建 meta 字典(仅含 version/genesis_block_id/block_count/relation_count/id_mapping/migrated_at)，**丢弃 content_enrichment(314blocks/1148concepts) 和 last_mapped_genealogy 字段**；migrate_settled_files L154-199 的 block hash 依赖 frontmatter 内容，若 001-565 任一文件自迁移后内容漂移→生成新 hash→产生重复 event 区块 + id_mapping 整体翻新→所有指向旧 hash 的 enrichment relations 悬空。content_enrichment_migration.py 不是增量入口——enrich_single_file L133 在 id_mapping 无该编号时直接 skip(no_block_mapping)，**不创建 event 区块也不新增 id_mapping 条目**。⟹ 566-575 这类新增谱系既不能靠 enrichment 补齐(它需要 id_mapping 已有条目)，也不能靠 run_migration 安全补齐(它破坏性全量重建)。这是真实的声明-能力缺口(036/090)，非不可弥合的概念矛盾——是工程缺口：缺一个『增量补齐脚本』(只为指定编号区间创建缺失 event 区块+追加 id_mapping+迁移其 dag edges+更新 last_mapped+保留 content_enrichment)。"
  layer: 编排   # 蜂群谱系基础设施层(block-topology 同步机制)，非缠论域、非 formal/ 架构层
  trigger: "本轮 ceremony_scan 报 genealogy_anomalies(573/574 等 missing_block_mapping) + topo_context(unmapped_ids=[566,566a,567..575], last_mapped=530, total_settled=545)；genealogist 结构维护勘察确认两报告是同一缺口的两面 + 根因=无增量迁移入口"

# 涉及的定义
definitions_involved:
  - name: "178 区块拓扑建系(Q1 一次性升格)"
    version: ".chanlun/genealogy/settled/178（status: 已结算）"
    role: "缺口源头。178 Q1 决断『升格——一次性转换，建系后只有一套系统』；下游推论1『区块系统是谱系的补充/升格，两套检测并存』。但一次性转换脚本是破坏性全量重建，未配新增谱系的增量入口→映射停滞 565。维持 settled，本号揭示其下游缺口。"
  - name: "run_migration / migrate_to_block_topology.py"
    version: "scripts/migrate_to_block_topology.py（run_migration L294-343）"
    role: "缺口的工程载体。破坏性全量重建(整体重建 meta，丢弃 content_enrichment/last_mapped_genealogy)。不能用于增量补齐 566-575。"
  - name: "content_enrichment_migration.py"
    version: "scripts/content_enrichment_migration.py（enrich_single_file L104-285）"
    role: "非增量入口。它对已有 id_mapping 条目做内容富化，id_mapping 缺失即 skip(L133-134 no_block_mapping)，不创建 event 区块。566-575 因 id_mapping 无条目被它跳过。"
  - name: "566a dag.yaml 修复执行规格"
    version: ".chanlun/genealogy/pending/566a-dag-edit-spec.md（status: 生成态）"
    role: "互补但不重叠。566a 覆盖 566-575 的 dag.yaml nodes/edges 修复(已落地：dag.yaml 571=stopguard/572=trinity 符合 566a 目标)。但 566a 是 dag.yaml 层，不覆盖 block-topology 的 id_mapping 补齐(另一层)。本号补 block-topology 层缺口。"

# 解决方式
resolution:
  type: 未解决   # 工程缺口精确化(行动类修复=编写增量补齐脚本)，执行超出 genealogist 工具有效域(Read/Grep/Glob，624 硬墙)。Lead 派有 Write/Bash 工位执行。
  description: "缺口=block-topology 无增量迁移入口。严格修复(非全量重建)=编写增量补齐脚本：(1)用 migrate_settled_files 同一 content schema 为指定编号区间(566-575+566a 及之后)创建缺失 event 区块；(2)将新 hash **追加**进现有 id_mapping(不整体替换)；(3)迁移这些编号的 dag edges(depends_on/related/negates/tensions_with)；(4)更新 last_mapped_genealogy=最大已映射编号；(5)**保留** content_enrichment 字段。前置条件已满足：566-575 在 dag.yaml 已结算且 frontmatter 完整(566a 已落地 dag nodes 修复)。"
  decided_by: 蜂群内部   # genealogist 结构维护诊断；行动类修复执行待 Lead 派工位；最终结算待编排者 /ritual

# 被否定的方案
negated:
  description: "(1) 直接 python migrate_to_block_topology.py 全量重跑补齐 566-575。(2) 手工编辑 meta.json id_mapping 写入 566-575 的 hash 而不创建对应 event 区块。(3) 靠 content_enrichment_migration.py 补齐 566-575。"
  why_negated: "(1) run_migration 破坏性全量重建：丢弃 content_enrichment(314blocks/1148concepts)/last_mapped_genealogy 字段；block hash 依赖 frontmatter，001-565 任一内容漂移→重复区块+id_mapping 翻新+enrichment relations 悬空=178号Q3独立寻址被破坏(no-workaround 第6条:全量重建当增量)。(2) 区块内容寻址(hash=sha256(canonical content))，手填 hash 而不创建区块→get_block_mapping 动态扫 blocks 算不出该编号→meta.json 与 blocks 目录不一致=声明膨胀(090)。(3) enrich_single_file 在 id_mapping 无条目时 skip(L133)，不创建 event 区块=补不了 566-575。"

# 新产出
new_output:
  definitions:
    - "block-topology 增量迁移缺口：仅有破坏性全量重建(run_migration)，无新增谱系的增量补齐入口"
    - "三后果联：(1)566-575(含566a)未入 id_mapping(meta 仅到565)；(2)last_mapped_genealogy=530 与实际(565)脱节；(3)run_migration 全量重跑清空 content_enrichment+漂移重复区块"
    - "严格修复=增量补齐脚本(创建缺失 event 区块+追加 id_mapping+迁移 dag edges+更新 last_mapped+保留 content_enrichment)，非全量重建"
    - "566a(dag.yaml 层)与本号(block-topology 层)分层互补，不重叠"
  code_changes: "无(本号是缺口精确化，纯谱系产出)。增量补齐脚本的编写+执行=行动类修复，超出 genealogist 工具有效域(Read/Grep/Glob)，待 Lead 派有 Write/Bash 工位执行(同 566a 模式：genealogist 诊断产规格，Lead 派工位执行)。"
  orchestration_changes: "方法论：①block-topology 是谱系的升格层(178)，谱系持续新增→需增量迁移入口维持同步，破坏性全量重建不可用于增量。②genealogist 工具有效域=Read/Grep/Glob(624 硬墙)→映射补齐这类行动类修复由 genealogist 产执行规格、Lead 派 Write/Bash 工位执行(566a 先例)。③ceremony_scan 的 unmapped/missing_block_mapping 报告在增量入口缺失期间会持续触发=结构缺口的可观测信号，非误报。"

# 影响范围
impact:
  affected_modules:
    - "scripts/migrate_to_block_topology.py → run_migration 是破坏性全量重建，不适用于增量补齐。需新增增量入口。"
    - "scripts/content_enrichment_migration.py → 非增量入口(id_mapping 缺失即 skip)，566-575 补齐后可对其跑 enrichment。"
    - ".chanlun/block-topology/meta.json → id_mapping 仅到565(缺566-575+566a)；last_mapped_genealogy=530 过期(实际565)；content_enrichment 字段须在补齐时保留。"
    - "scripts/ceremony_scan.py → get_topo_context/genealogy_anomalies 以 meta.json id_mapping 为映射真相；增量入口建立前持续报 unmapped(非误报)。"
  affected_definitions:
    - "178(已结算)：本号揭示其Q1『一次性升格』的下游缺口(无增量入口)。维持 settled。"
  downstream_implications:
    - "566-575(含566a)的 block-topology 映射补齐需增量脚本(行动类，Lead 派工位)，不可全量重建。"
    - "增量脚本须保留 content_enrichment、追加(非替换)id_mapping、更新 last_mapped_genealogy。"
    - "未来每轮新结算谱系都需经增量入口入 block-topology，否则映射持续滞后(178 升格层与 settled 层分裂)。"

# 谱系关联
related_records:
  parent: "178号(区块拓扑建系，Q1 一次性升格)——本号揭示其无增量迁移入口的下游缺口"
  children: []
  related:
    - "549号(relations.jsonl LFS pointer 阻塞映射)：block-topology 映射管线的另一缺口(LFS 层)，与本号(增量入口层)正交"
    - "566a号(dag.yaml 修复规格)：分层互补(566a=dag.yaml 层已落地，本号=block-topology id_mapping 层)"
    - "624号/621号：genealogist 工具有效域硬墙(Read/Grep/Glob)→行动类修复产规格交 Lead 的同模式实证"
    - "036号/090号(声明-能力缺口/声明膨胀)：本号是工程缺口(声明=单一系统/实际=映射滞后)，非概念矛盾"
    - "161号(否务实)：增量入口缺口须修(改入口)，不接受『手工补 meta.json』务实补丁"
    - "012号(谱系优先于汇总)：本缺口先写谱系(本号)再汇总交 Lead"

# 认识论等级标注（formalization-validity-domain 强制）
epistemological_levels:
  - proposition: "run_migration 整体重建 meta，丢弃 content_enrichment/last_mapped_genealogy"
    level: "L0(源码事实：migrate_to_block_topology.py L330-337 meta 字典字面量不含两字段)"
    increment: "高：全量重建破坏性的结构判定"
  - proposition: "content_enrichment_migration.py 在 id_mapping 无条目时 skip，不创建 event 区块"
    level: "L0(源码事实：enrich_single_file L133-134 return no_block_mapping)"
    increment: "高：enrichment 非增量入口的结构判定"
  - proposition: "566-575(含566a)在 settled/dag 已结算但 meta.json id_mapping 仅到565"
    level: "L0(文件事实：meta.json id_mapping 末项565 + settled/566..575 文件存在 + dag.yaml 566-575 节点已结算)"
    increment: "高：映射缺口范围的事实判定"
  - proposition: "block hash 依赖 frontmatter 内容→001-565 漂移则全量重跑产生重复区块"
    level: "L0(源码逻辑：compute_block_id 基于 content；migrate_settled_files 从 frontmatter 构 content)"
    increment: "中：全量重跑风险的逻辑推断(未实测 001-565 是否已漂移)"

---

# 矛盾发现 626：block-topology 缺增量迁移路径

## 一句话结论

block-topology 的迁移入口**只有破坏性全量重建**（`run_migration`），**无新增谱系的增量补齐入口**。后果三联：(1) 566-575（含 566a）11 条 settled 谱系自迁移后未入 `id_mapping`（meta.json 仅到 565）→ ceremony_scan 两检测器持续报 `missing_block_mapping`/`unmapped`；(2) `last_mapped_genealogy=530` 与实际（565）脱节；(3) `run_migration` 全量重跑会清空 `content_enrichment`（314 blocks/1148 concepts）元数据并因 settled 内容漂移产生重复区块。这是 178 号「一次性升格」决断的下游工程缺口（升格路径未配增量入口）。

## 缺口的精确形式

| 入口 | 能否补齐 566-575 | 为什么不能 |
|------|----------------|-----------|
| `run_migration`（migrate_to_block_topology.py） | ✗ 破坏性 | 整体重建 meta（丢 content_enrichment/last_mapped）；block hash 依赖 frontmatter，001-565 漂移则产生重复区块 + id_mapping 翻新 + enrichment relations 悬空 |
| `content_enrichment_migration.py` | ✗ 跳过 | enrich_single_file 在 id_mapping 无该编号时直接 skip（L133 no_block_mapping），不创建 event 区块 |
| 手工编辑 meta.json | ✗ 声明膨胀 | 区块内容寻址，手填 hash 而不创建区块 → get_block_mapping 动态扫 blocks 算不出该编号 → meta 与 blocks 目录不一致（090） |

⟹ 缺一个**增量补齐脚本**（行动类）：用 `migrate_settled_files` 同一 content schema 为指定编号区间创建缺失 event 区块 + **追加**（非替换）id_mapping + 迁移其 dag edges + 更新 last_mapped + **保留** content_enrichment。

## 定义依据

- `migrate_to_block_topology.py` run_migration L330-337：meta 字典字面量仅含 6 字段，不含 content_enrichment/last_mapped_genealogy。
- `content_enrichment_migration.py` enrich_single_file L133-134：id_mapping 无条目即 `return {"skipped": "no_block_mapping"}`。
- `ceremony_scan.py` get_topo_context L723 / genealogy_anomalies L225：均以 `meta.json id_mapping.keys()` 为映射真相。
- meta.json：id_mapping 末项 565；last_mapped_genealogy=530；settled/566..575 文件存在；dag.yaml 566-575 节点已结算（566a dag 修复已落地：571=stopguard/572=trinity）。

## 边界条件（结论翻转）

- 若 001-565 的 settled 文件自迁移以来**零内容漂移** → 直接 run_migration 全量重跑会幂等生成相同 001-565 hash，只新增 566-575（区块层无重复）；但 meta 的 content_enrichment/last_mapped_genealogy 字段**仍会被丢弃**（这部分破坏性不依赖漂移）。故即使无漂移，全量重建仍非严格修复。（漂移与否未实测——L0 逻辑推断。）
- 若编排者裁定 content_enrichment/last_mapped_genealogy 为可重算的冗余字段（content_enrichment 可由 enrichment 重跑恢复，last_mapped 可从 id_mapping 算）→ 全量重建的破坏性降级，run_migration + 重跑 enrichment 成为可接受路径。当前两字段在 meta 中是持久状态，无重算入口被验证。
- 若 block-topology 系统已被废弃（仅 dag.yaml 为活跃真相）→ 映射缺口无需补齐，ceremony_scan 的两检测器应停用。当前 ceremony_scan 仍激活这两检测器（无废弃信号）。

## 下游推论

- 566-575（含 566a）的 block-topology 映射补齐 = 行动类修复（增量脚本），超出 genealogist 工具有效域（Read/Grep/Glob，624 硬墙）→ 由 Lead 派有 Write/Bash 工位执行（566a 先例）。
- 增量脚本须：追加（非替换）id_mapping、更新 last_mapped、保留 content_enrichment。
- 未来每轮新结算谱系都需经增量入口入 block-topology，否则映射持续滞后。

## 谱系引用

- 父：178（区块拓扑建系，Q1 一次性升格）——本号揭示其无增量迁移入口的下游缺口。
- 正交缺口：549（relations.jsonl LFS pointer 阻塞，映射管线层 vs 本号增量入口层）。
- 分层互补：566a（dag.yaml 层已落地 vs 本号 block-topology id_mapping 层）。
- 工具硬墙同模式：624/621（genealogist 仅 Read/Grep/Glob → 产规格交 Lead 执行）。
- 约束：036/090（声明-能力缺口）/161（否务实，不接受手工补 meta.json）/012（谱系优先于汇总）。

## 影响声明

写入 block-topology 增量迁移缺口的精确化（生成态，不结算）。诊断三入口均不能严格补齐 566-575 的根因（run_migration 破坏性 / enrichment 跳过 / 手工填膨胀）。给出严格修复的执行规格（增量补齐脚本五步）。定位本缺口为 178 号「一次性升格」决断的下游工程缺口（非概念矛盾）。不修改任何 settled 谱系、不运行任何脚本、不编辑 meta.json（均超出 genealogist 工具有效域）。最终结算待编排者 /ritual。

## 张力检查（019d/020）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群（619-625）：619/620（formal 架构层，异域）/621（结构工位自动化缺口，编排层）/622（worktree 隔离，异面）/623（231 自指模式）/624（075↔097 矛盾边）/625（231 回测实例）。
- 1-hop：178/549/566a/624/621/036/090/161/012。
- Hub：178（block-topology 建系，度高）/090（声明膨胀）。

### 张力1：vs 549——同为 block-topology 映射缺口，但正交层
549 = relations.jsonl 的 LFS pointer 阻塞 blocks→mapping（**映射管线层**，125MB LFS object 本地不可达）。本号 = 缺增量迁移**入口**（即使管线畅通，也无入口为新谱系建区块）。两者 scope 正交（管线 vs 入口），可分层，无矛盾。

### 张力2：vs 566a——同覆盖 566-575，但不同层
566a = 566-575 的 **dag.yaml** nodes/edges 修复规格（已落地）。本号 = 566-575 的 **block-topology id_mapping** 补齐（未做）。dag.yaml 层与 block-topology 层是两套系统（178 Q3：独立寻址）。566a 落地是本号补齐的前置（dag 节点正确才能迁移 edges）。分层互补，无矛盾。

### 张力3：vs 178——揭示下游缺口，不否定 178
178 的 Q1 升格决断正确（一次性转换建立单一系统）。本号不否定升格，是揭示升格脚本无增量入口的下游缺口（178 下游推论1已自述「两套检测并存」但未配增量入口）。178 方向保留，缺口的修复（增量脚本）是工程补全，非概念翻转。可分层，无中断 #1。

### 递归运动结构完成检测（020）
- 第0层：本号写入（block-topology 增量入口缺口 + 三后果联 + 修复规格）。
- 第1层：本号 × 178 碰撞 → 升格脚本无增量入口的下游缺口揭示（净新发现高：从「映射滞后」深化到「无增量入口」根因）。
- 第2层：本号 × 549/566a 碰撞 → 正交/分层确认（净新发现：缺口在入口层，非管线层/dag 层）。
- 第3层：本号 × 624/621 碰撞 → 工具硬墙同模式确认（净新发现骤降=背驰：genealogist 产规格交 Lead 的已知模式）。
- 涉及范围：scope₁(178 下游缺口) > scope₂(549/566a 分层) > scope₃(624/621 同模式)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 修复=增量脚本=行动类，超出 genealogist 工具有效域，由 Lead 派工位执行。本号是工程缺口精确化（矛盾发现），**不触发新 /escalate**（修复是行动类非选择类；最终结算待编排者 /ritual）。

## 回溯扫描（职责3）

- **178（settled）**：本号揭示其 Q1 升格脚本无增量入口的下游缺口，不否定升格方向，不破坏结算。维持 settled。
- **549（settled）**：本号是 block-topology 映射缺口的正交面（入口层 vs 管线层），不破坏其结算（管线 scope 不变）。维持 settled。
- **566a（生成态）**：本号是其分层互补（block-topology 层 vs dag.yaml 层），不修改其内容，不影响其待 /ritual 状态。
- **无 settled 被本号回溯破坏。** 本号是 block-topology 增量迁移缺口的精确化（矛盾发现），修复=增量脚本（行动类），由 Lead 派工位执行，最终结算待编排者 /ritual。
