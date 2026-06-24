---
id: '549'
number: 549
title: "relations.jsonl 沦为未实体化 LFS 指针——476号(append-only ⊥ long-running)张力的物质显形，冻结 block-topology 映射管线"
type: 矛盾发现
status: 已结算  # §十 no-workaround 结算：B/C 有损覆盖违反 no-workaround 被排除 → A（无损 git lfs pull）唯一合法 = 定理（非三选一价值判断）；概念层吸收入 476。注意：概念结算 ≠ 映射管线解冻（topo_effect freeze 保留，解冻条件 = A 执行）
settlement:
  type: 蜂群内部消化（吸收 + 回溯结算）
  decided_by: genealogist（no-workaround 蜂群语法规则的逻辑必然，非编排者价值判断——"选择"被消解为"定理"）
  date: '2026-06-22'
  conceptual_absorption: "概念层吸收入 476号（jsonl append-only ⊥ long-running）——549 = 476 张力边在 git-lfs 层的物质实例。relations.jsonl 沦为未实体化 LFS 指针 = 476 预言的『长运行无界 append → 膨胀 → 迁 LFS』在本机的具体显形。工程下游推论（部分损坏态 §八 / 粘性陷阱 §八 / 扫描盲区 scan-state-unobservable §八，094 又一实例）作为 476 实例的机制记录保留。"
  canonical_recovery: "唯一无损路径 = A（安装 git-lfs + 配置远端 + git lfs pull 实体化 6889 正典）。no-workaround 排除有损 B（.bak 1118，丢 5771）/ C（mapper 重生成上限 30.5%=2101/6889，丢 4788，运行时拓扑层永久不可重建）。"
  thaw_condition: "概念结算 ≠ 映射管线解冻。topo_effect freeze 保留——relations.jsonl 仍是 LFS 指针，531-548 映射 + 后续谱系→区块映射仍冻结。解冻条件 = A 执行（操作者基础设施动作：装 git-lfs CLI + 配置远端 + git lfs pull），属基础设施待办，与本号概念结算分离。蜂群无 git push/远端凭据权限，无法自行执行 A。"
  infra_todo: "A 的执行（git lfs pull）= 操作者基础设施动作，本机 .git/lfs/ 不存在 + git-lfs CLI 未安装（§九.2 四种检查全空）。这是基础设施待办，不是概念矛盾，与 549 概念结算分离。"
date: '2026-06-22'
level: "L0（block_topology.py:441-444 read_all_relations 对 LFS 指针首行 json.loads 必抛 JSONDecodeError；map_genealogy_to_blocks.py Phase3 路径分析）+ L1（文件状态：134 字节指针 vs meta.json relation_count=6859/6889 vs relations.jsonl.bak 1089 行——§九实测推翻 §三的 6859 估算）+ L2（verify549 量化勘察：C 重建上限 30.5%=2101/6889，运行时拓扑层硬不可重建，§十）"
负责工位: genealogist（session-4130a713/genealogy-maintenance，拓扑映射任务 #8 衍生；2026-06-22 恢复任务 §九；2026-06-22 verify549 量化勘察 + no-workaround 结算 §十）
provenance: "[新缠论:实装审计]（block-topology 映射管线 + git-lfs 状态）"
negation_source: homogeneous
negation_form: waiting  # après-coup：概念已结算（吸收入 476），但映射管线解冻仍待操作者基础设施动作（A 执行）后回溯规定
negates: "隐含命题『block-topology 映射管线（map_genealogy_to_blocks.py）随时可执行』——被证伪：relations.jsonl 是 134 字节 LFS 指针（LFS object 未 pull），read_all_relations 对其首行 `version https://git-lfs.github.com/spec/v1` 调 json.loads 必抛 JSONDecodeError；mapper Phase3 (_build_existing_keys→read_all_relations) 在 Phase2 已 write_meta 之后崩溃 = 部分损坏（meta.json 已加 531-547，relations 未写，脚本异常退出）"
topo_effect: "freeze:block-topology-mapping-pipeline:downstream（冻结 531-548 映射 + 一切后续谱系→区块映射；解冻条件 = A 执行 git lfs pull，§十结算后 freeze 保留——settled ≠ 解冻）"
depends_on:
  - '476'   # jsonl append-only ⊥ long-running——本号是其物质显形（无界 append → relations.jsonl 膨胀 → 迁 LFS → 未实体化）；§十概念层吸收入此
  - '178'   # 区块拓扑建系（relations.jsonl append-only 关系层的来源）
related:
  - '424'   # snet persistence implementation（同类持久化-长运行张力）
  - '094'   # 审计层断裂——scan-state-unobservable（§八扫描盲区是其又一实例）
tensions_with:
  - '178'   # 178（建立 append-only relations.jsonl 关系层）⊥ 476（append-only 不兼容 long-running）——本号是该张力边在 git-lfs 层的物质实例
---

# 549 号：relations.jsonl 沦为未实体化 LFS 指针——冻结 block-topology 映射管线

**认识论**：L0（源码崩溃路径）+ L1（文件状态不一致）+ L2（verify549 量化勘察，§十）。**状态已结算**（2026-06-22 §十 no-workaround 结算）：原判「解决依赖操作者基础设施动作」→ §九升级为「选择」→ §十 **no-workaround 消解为「定理」**：B/C 有损覆盖违反蜂群语法规则 no-workaround → 被排除 → A 唯一无损 = 唯一合法 = 定理（非三选一价值判断）。概念层吸收入 476号；映射管线解冻仍待 A 执行（topo_effect freeze 保留）。

## 一、现象（拓扑映射任务 #8 受阻）

ceremony_scan 报告 17 个未映射谱系（531-547），P2 工位要求用 `scripts/map_genealogy_to_blocks.py` 映射到 block-topology。执行前审计发现 `.chanlun/block-topology/relations.jsonl` **不是 JSONL，是 134 字节的 git-lfs 指针**：

```
version https://git-lfs.github.com/spec/v1
oid sha256:3b895e989dc4d9666d6bcb647ea7f7bdea739c6302d4f0a90baa9489a006d416
size 125404332
```

## 二、崩溃路径（L0，为什么 mapper 必然部分损坏）

`map_genealogy_to_blocks.py::map_genealogy_numbers`：
- **Phase 1**（建区块，写 `blocks/`）：成功。
- **Phase 2**（`write_meta`，把 531-547 加入 `meta.json` id_mapping）：成功 ← **此时 meta.json 已被改写**。
- **Phase 3**（`if new_mappings:` → `_build_existing_keys(base)` → `read_all_relations(base)`）：
  - `block_topology.py:441-444`：`for line in jsonl_path.read_text().splitlines(): json.loads(line)`
  - 首行 `version https://git-lfs.github.com/spec/v1` → **JSONDecodeError（未捕获）→ 脚本崩溃**。

后果：meta.json 被写入新映射，但 relations 未写、脚本异常退出 = **部分损坏状态**。此外 `append_relation` 用 `open(...,"a")` 会把 JSON 行**追加到 LFS 指针**（污染指针文件）。

## 三、文件状态三方不一致（L1）

| 来源 | 大小/值 | 含义 |
|---|---|---|
| relations.jsonl（LFS 指针 size 字段） | 125,404,332 B（~125MB） | LFS object 声称的大小（未 pull，本地无实体） |
| meta.json `relation_count` | 6859 | 元数据声称的关系数 |
| relations.jsonl.bak（真实 JSONL） | 323,173 B | 真实备份 |

> **⚠ §三本表的「.bak ≈ 6859 关系×~47B/行，与 relation_count 自洽」估算已被 §九 实测推翻**：.bak 实测仅 **1089 行**（不是 6859）。323KB÷47B 的字节估算错误——.bak 的行平均字节远大于 47B（含 64-hex 的 from/to/created_by 三个字段，单行 ~300B）。保留本表原文以存证错误推导，正确数据见 §九.1。

## 四、根因谱系：476 号张力的物质显形

- **476号**（jsonl-append-only-incompatible-long-running）：append-only JSONL 与长运行不兼容。relations.jsonl 是 append-only JSONL。
- **178号**（区块拓扑建系）：建立了 append-only relations.jsonl 关系层。
- 链条：178 的 append-only 设计 → 长运行无界 append（476 预言）→ relations.jsonl 膨胀至 125MB → 迁 git-lfs → 本地未 pull = 指针 → mapper 崩溃。
- **本号 = `178-476` 张力边（dag.yaml，无 valid_until = 活跃张力）的 git-lfs 层物质实例**。476 是抽象命题，549 是它在本机的具体显形。

## 五、否定面（no-workaround）

- **不**向 LFS 指针 append（污染指针）。
- **不**运行 mapper 真实模式（Phase3 必崩 + 部分损坏 meta.json）。
- **不**蜂群自行 `git lfs pull` 或从 .bak 覆盖 relations.jsonl——哪个状态是正典涉及 git-lfs 配置与操作者判断（属基础设施动作，非概念裁决）。dry-run 安全（Phase1 提前返回，不触 Phase2/3）。

## 六、解冻路径（待操作者，结论翻转条件）

> **⚠ §九 实测：以下两条路径在本机双双不可用，见 §九.2。本节保留为原始设想。§十结算后唯一无损解冻路径 = A（见 §十）。**

满足任一即可解冻映射，本号回溯结算：
1. **`git lfs pull`** 实体化 relations.jsonl（若 LFS 125MB object 是正典）；或
2. **从 relations.jsonl.bak 恢复** relations.jsonl（若 .bak 的关系是正典，与 meta.json 自洽，更可能）；
3. 解决后再跑 `map_genealogy_to_blocks.py --all-unmapped` 映射 531-548（幂等，content-addressed + 关系去重）。

## 八、预测的部分损坏已物质化（2026-06-22 复查，§二预言被证实）

复查 ceremony_scan + meta.json + relations.jsonl，§二预言的「Phase2 写 meta 后 Phase3 崩溃 = 部分损坏」**已发生**：

| 层 | 状态 | 含义 |
|---|---|---|
| meta.json id_mapping | **已含 531-548**（某并发 session 跑了 mapper Phase1+2） | 节点已映射 |
| relations.jsonl | **仍是 134 字节 LFS 指针**（未变） | Phase3 read_all_relations 崩在 LFS 指针首行 ⟹ **531-548 的关系边从未写入主文件** |

block-topology 现处**节点完备 / 边缺失**的不一致态：missing_block_mapping 从 17 降到 1（仅剩新建的 550），但 531-548 的 depends_on/negates/related/tensions_with 边**全部缺失于主 relations.jsonl**（部分被旁路写入未追踪的 relations_531_548.jsonl，见 §九.1）。

### 粘性陷阱（sticky trap，新下游推论）

LFS 修复后**简单重跑 mapper 不能补边**：mapper `for num in numbers: if num in id_mapping: continue` ⟹ 531-548 已在 id_mapping ⟹ 全跳过 ⟹ `new_mappings` 为空 ⟹ `if new_mappings:` 门控关闭 Phase3 ⟹ **边永远不写**。正确修复二选一：
1. 先从 meta.json id_mapping **删除 531-548**（+回退 block_count/relation_count），再跑 mapper（new_mappings 非空 ⟹ Phase3 跑 ⟹ 写边）；或
2. LFS 实体化后从 dag.yaml 手动回填 531-548 的边到 relations.jsonl。

### 扫描盲区（scan-state-unobservable / 094 又一实例）

`detect_genealogy_anomalies` 的 missing_block_mapping 只查 meta.json id_mapping，**不查关系边完备性** ⟹ 边缺失时仍报「已映射」。这是 pattern-buffer `scan-state-unobservable`（429/094 审计层断裂）的又一实例：relations.jsonl 完备性无 scan 消费者 = 静默损坏不可观测。建议 ceremony_scan 增 relations 完备性检查（关联 ceremony-scan-completeness skill 草案）。

> §十结算后定位：本节工程下游推论（部分损坏态 / 粘性陷阱 / 扫描盲区）作为 476 实例的**机制记录保留**——它们是 476 抽象张力在 git-lfs 层显形时的具体破坏机制，不构成独立未解矛盾。

## 九、恢复尝试中止：正典裁决前提被证伪（2026-06-22 genealogy 工位深度勘察）

genealogy 工位（session 处理 549 恢复+补边任务）执行前做了硬数据勘察，**§三「.bak ≈ 6859 关系，与 relation_count 自洽」的前提被证伪**，触发任务指令的 STOP 条件「若 .bak 与 meta 行数对不上，停下报告 Lead，不强行覆盖」。

### 九.1 实测数据（推翻 §三估算）

| 来源 | 实测值 | 与 §三声称对比 |
|---|---|---|
| relations.jsonl（工作区） | 134 字节 / 3 行，LFS 指针 oid `3b895e98…a006d416`, size 125404332 | 与 §一一致（仍是指针） |
| relations.jsonl.bak | **1089 行**（合法 JSONL，0 解析错误） | §三称「≈6859 关系」**错**——基于 323KB÷47B 估算，实测仅 1089 行 |
| relations_531_548.jsonl（未追踪 `??`） | 30 行（531-548 的边，某并发 session 旁路写出） | §八粘性陷阱的物质载体 |
| relations_483_484.jsonl | 13 行（483/484 的边） | §三未提及 |
| **.bak + 两增量合并去重** | **1118 条** | 与 meta relation_count **差约 5740 条** |
| meta.json relation_count（HEAD） | 6859 | — |
| meta.json relation_count（工作区，已 `M`） | 6889（=6859+30，含 531-548 增量计数） | — |
| meta.json block_count（HEAD→工作区） | 1868→1886（+18 = 未追踪的 531-548 区块） | — |

.bak 关系类型分布：depends_on 509 / related 452 / tensions_with 24 / negated_by 13 / negates 12 / records 15 / residue_of 4 / splits 5 / None 55（order: 1=1018, 2=15, None=56）。

### 九.2 两条解冻路径双双不可用（§六的两条路径都被堵死）

- **路径1（`git lfs pull`）彻底不可用**：本机 `.git/lfs/` 目录**不存在**；目标 LFS object（oid `3b895e98…`，125MB）**不在本地缓存**；git-lfs CLI **未以任何形式安装**（`which git-lfs` / `git lfs version` / `brew list git-lfs` / 全盘 `find / -name git-lfs` 四种检查全空）。⟹ 6859 条完整正典**只存在于远端 LFS object**，本机无法实体化，也无法手动从 `.git/lfs/objects/` 取出（目录不存在）。
- **路径2（从 .bak 恢复）= 不可逆数据损坏**：.bak 只有 1089 行（+增量去重 1118），用它覆盖 = 把 meta 声称的 6859 条关系**缩成 1118 条 = 丢失约 5740 条已建立的关系边**。备份 LFS 指针无意义——指针指向的 125MB object 本机不存在，**备份指针 ≠ 备份数据**，无法回退到 6859 状态。可逆性保证（任务要求）在 object 缺失时**形同虚设**。

### 九.3 矛盾的精确形式（§九原判：需 Lead 上浮编排者；§十修正：no-workaround 消解为定理，无需上浮）

**relation_count=6859 这个正典数，其物质载体（125MB LFS object）在本机不可达；本机所有可用的关系数据（.bak 1089 + 增量 43）合计仅 1118 条，无法重建 6859。** 三种处置互斥，每种都丢信息或都需外部动作：

| 选项 | 动作 | 代价 |
|---|---|---|
| A | 安装 git-lfs + 配置远端 + `git lfs pull` 实体化 6859 正典 | 需操作者基础设施动作（装 CLI、确认远端可达），蜂群无 git push/远端凭据权限 |
| B | 接受 .bak(1089)+增量(43) 为新正典，重算 meta（relation_count←1118, block_count 相应回退），承认 5740 条边永久丢失 | 不可逆丢失约 5740 条关系；且需确认这 5740 条是否真有不可替代信息（可能多为可从 dag.yaml/blocks 重新生成的迁移边） |
| C | 从 dag.yaml + blocks/ 元数据**重新生成**全部关系边（mapper 全量重跑，content-addressed 幂等），用生成结果替代 LFS object | 需先解决"mapper 跳过已映射节点"的粘性陷阱（§八）+ 验证重新生成能否覆盖 6859 的全部语义（depends_on/negates/related/tensions_with + content_enrichment 的 defines/references 边——后者 .bak 里有 55 条 None-relation + residue_of/splits，未必能从 dag.yaml 重建） |

**§九原判**：genealogist 不自行选择，保持生成态，等待 Lead 上浮编排者裁决 A/B/C。
**§十修正（结算）**：此「选择」前提缺一约束——no-workaround（蜂群语法规则）。加入后 B/C（有损覆盖）被排除，A 成为定理性唯一合法路径，无需编排者价值判断。详见 §十。

### 九.4 已确认未做的事（可逆性保持）

- **未** cp .bak 覆盖 relations.jsonl（STOP 条件触发）。
- **未** 运行 mapper 真实模式。
- **未** 改 meta.json（工作区的 `M` 标记 + 18 个未追踪 blocks + 两个增量文件均为**本工位介入前**的既有状态，非本工位产出）。
- **唯一改动**：§九 + frontmatter status 注释 + §三/§六 的证伪标注（§十不改任何拓扑数据文件，保持 §九.4 可逆性）。

## 七、影响声明

谱系结晶，未改任何拓扑数据文件，未运行 mapper。**冻结**：531-548 的 block-topology 关系边 + 一切后续谱系→区块映射，直到 relations.jsonl 解冻（§十：A 执行）。§八粘性陷阱 + §九正典缺失叠加 = 工程下游机制记录（保留为 476 实例的破坏机制）。

## 十、verify549 量化勘察 + no-workaround 结算（2026-06-22）

### 十.1 verify549 量化勘察（L2，§九.3 三选项损失量化）

verify549 工位完成量化勘察，把 §九.3 的定性「都丢信息」精确化为可比较的损失量：

| 选项 | 无损？ | 重建上限 | 永久不可重建部分 |
|---|---|---|---|
| **A**（git lfs pull） | **唯一无损** | 6889/6889（100%） | 无 |
| B（.bak 1118 覆盖） | 有损 | 1118/6889（16.2%）→ 丢 5771 | 5771 条边 |
| C（mapper 重生成） | **严重有损** | **2101/6889（30.5%）** → 丢 4788 | 运行时拓扑层（residue_of/splits/freezes/severs/supersedes/reopens）**永久不可重建** |

**C 的硬约束（关键修正）**：
- **重建上限仅 30.5%（2101/6889）**——C 不是「全量重跑覆盖」，是有损重生成。
- **运行时拓扑层永久不可重建**：residue_of/splits/freezes/severs/supersedes/reopens 这些边是**共识仪式 / 剩余积累的历史事件**沉积，**无静态源**（不在 dag.yaml/blocks 正文里），mapper 无法从任何现存文件重新生成。一旦丢失即永久丢失。
- **正文派生层（defines/references ~5500）需 content_enrichment_migration 重跑**：超出 §九.3 选项 C 的字面定义（C 字面只说「从 dag.yaml + blocks/ 元数据重新生成」，未含 content_enrichment 重迁移），即 C 即便补跑 content_enrichment 也只到 30.5%。

### 十.2 §九.1 None-relation 55 条误判修正

§九.1 把 .bak 的 None-relation 55 条列为「可能不可重建」（§九.3 选项 C 代价栏）。verify549 勘察修正：这 55 条是 **legacy 格式**（用 `type` 字段而非 `relation` 字段），`_normalize_relation` 归一化后映射回 depends_on(42)/related(5)/tensions_with(7)/negates(1)，**是可重建 5 类，不构成额外损失**。即 None-relation 不是 C 的额外失分项——C 的 30.5% 上限已是修正后口径。

### 十.3 no-workaround 结算逻辑（"选择"消解为"定理"）

§九原判「三者无一无损 → 保持选择 → 上浮编排者」**漏算一个约束**：verify549 的量化是纯损失比较，**未纳入 no-workaround 蜂群语法规则**。

加入 no-workaround（`no-workaround.md`：禁止绕过概念矛盾 / 禁止有损覆盖让"先跑通"；与 `no-patch-mentality.md` 互补：禁止非严格产出）：

1. **B / C 是有损覆盖 relations.jsonl**——B 丢 5771 条边，C 丢 4788 条边 + 运行时拓扑层永久不可重建。用残缺数据覆盖正典 = "先这样后面再改"的有损 workaround = **违反 no-workaround → 被排除（不是"代价大的选项"，是语法不合法的选项）**。
2. **A 是唯一无损路径**——`git lfs pull` 实体化远端完整 6889 正典，零损失。
3. ∴ **A = 唯一无损 = 唯一不违反 no-workaround = 唯一合法路径 = 定理**。

这不是三选一的价值判断（不需要编排者权衡 A/B/C 哪个"更好"）——一旦 no-workaround 作为已结算的蜂群语法规则前置，B/C 自动出局，A 成为**逻辑必然的唯一解**。「选择」（018号四分法）被消解为「定理」（已结算原则的逻辑必然推论）→ 不上浮，genealogist 自决结算。

**与 no-workaround.md「必须做的」对照**：矛盾已精确描述（§九.3）+ 依据的定义已说明（476/no-workaround）+ 唯一无损修复路径已确定（A）→ 这是 no-workaround 的严格结算，**不是「先移走再说」**。

### 十.4 A 的执行 = 操作者基础设施待办（与概念结算分离）

A 的执行（装 git-lfs CLI + 配置远端 + `git lfs pull`）= **操作者基础设施动作**：
- 本机 `.git/lfs/` 不存在 + git-lfs CLI 未安装（§九.2 四种检查全空）。
- 蜂群无 git push / 远端凭据权限，无法自行执行 A。

∴ A 的执行是**基础设施待办**，与 549 的**概念结算分离**。概念矛盾（哪条路径合法）已结算（= A）；物质实现（实际把 object pull 下来）待操作者。

### 十.5 四分法分类 + 结算动作

| 项 | 分类（018号四分法） | 处理 |
|---|---|---|
| 549 概念内容（relations.jsonl 沦为 LFS 指针冻结映射管线 = 178 append-only ⊥ 476 long-running 张力的 git-lfs 层物质实例） | **吸收**（被 476 号张力吸收） | 概念层吸收入 476，549 status → 已结算 |
| 正典恢复路径选择（A/B/C） | **定理**（no-workaround 消解为唯一合法 A） | 不上浮，genealogist 自决 |
| A 的执行（git lfs pull） | **行动**（操作者基础设施动作，蜂群无权限） | 基础设施待办，与概念结算分离 |
| 工程下游推论（部分损坏态/粘性陷阱/扫描盲区） | 机制记录 | 作为 476 实例的破坏机制保留（§八） |

**结算动作**：
1. frontmatter status 生成态 → 已结算，加 settlement 字段（本文件头部）。
2. topo_effect freeze **保留**——概念结算 ≠ 映射管线解冻。relations.jsonl 仍是指针，映射仍冻结，解冻条件 = A 执行。
3. 概念层吸收入 476（476 frontmatter children 加 549 反向引用）。
4. 文件 pending/ → settled/（git mv 由 Lead 执行：`git rm pending/549-*.md` 旧文件，本结算文件已写入 settled/）。

### 十.6 关键区分：settled ≠ thawed（结算 ≠ 解冻）

549 settled = **概念矛盾已结算**（哪条路径合法 = A，已确定）。但 block-topology 映射管线**仍冻结**：
- relations.jsonl 仍是 134 字节 LFS 指针（未变）。
- 531-548 映射 + 后续谱系→区块映射仍不可执行。
- 解冻条件 = A 执行（git lfs pull，操作者基础设施动作）。

**settled 不等于解冻**——这是本号结算的核心区分。topo_effect freeze 在 settled 后**保留有效**，直到 A 执行才回溯解冻（negation_form: waiting 的 après-coup 在解冻维度仍成立）。

---

## 十一、回溯扫描确认：569/573 缺映射 = 冻结下游（genealogist，2026-06-23）

> 触发：ceremony_scan 报 573/569 在 settled/ 但 block-topology 无对应映射；Lead 任务"核实真异常 vs scan 误报，若真缺→TaskCreate 拓扑映射子任务"。genealogist 回溯扫描裁决：**非新异常，是 549 冻结的已知下游后果；不创建拓扑映射子任务**（创建即违反 §五/§十 no-workaround 结算）。

### 十一.1 实测状态（2026-06-23）

| 检查项 | 实测 | 与 549 一致性 |
|---|---|---|
| `relations.jsonl` | **仍 134 字节 LFS 指针**（oid `3b895e98…a006d416`、size 125404332，与 §一逐字一致，未变） | ✅ 映射管线仍冻结（settled ≠ 解冻，§十.6） |
| meta.json id_mapping 最高映射号 | **565**（531-565 在册，含 549/550 的 Phase1+2 部分态） | ✅ 节点完备/边缺失态（§八） |
| **566-573** | **完全无映射**（不在 meta.json id_mapping，relations 边亦无） | ✅ 落入"一切后续谱系→区块映射"冻结范围（topo_effect） |

### 十一.2 裁决：定理类（549 冻结的逻辑必然推论），非新异常

- ceremony_scan 的 `missing_block_mapping` 报 569/573 = **正确观测**（映射确实缺失），但**非新异常**——这是 549 topo_effect `freeze:block-topology-mapping-pipeline:downstream` 明文覆盖"一切后续谱系→区块映射"的**预期下游**。566-573 缺映射是 549 冻结的**逻辑必然推论**（每个 ≥ 549 的谱系在 A 执行前必然缺映射）。
- **不创建拓扑映射子任务**（topology-manager）。理由（no-workaround，§五/§十）：
  1. 运行 `map_genealogy_to_blocks.py` 真实模式 ⇒ Phase3 `read_all_relations` 对 LFS 指针首行 `json.loads` 必抛 JSONDecodeError 崩溃（§二），且 Phase2 已写 meta = **部分损坏**（节点完备/边缺失，§八粘性陷阱）。
  2. 即便只跑 Phase1+2（不崩），也制造 566-573 的"节点完备/边缺失"不一致态（§八已证有害）。
  3. 解冻条件 = A 执行（git lfs pull）= **操作者基础设施待办**（§十.4，蜂群无 git-lfs CLI / 远端凭据权限）。拓扑映射子任务无法绕过此前提。
- **四分法**：549 冻结是已结算原则 ⇒ "566-573 缺映射"是其**定理**（逻辑必然推论）⇒ 自动结算，不上浮、不创建子任务（no-unnecessary-escalation 四分法：定理类不允许提问/创建绕过性工作）。

### 十一.3 处置

- 566-573（及一切后续谱系，含本轮新增 574）映射**统一归入 549 冻结范围**，等待 A 执行后**批量解冻重映射**（粘性陷阱 §八.修复：A 实体化 relations.jsonl 后，先从 meta.json 删除 531-565 + 回退计数，再全量重跑 mapper 写边；或从 dag.yaml 回填边）。
- **唯一真阻塞 = 549 §十 A（git lfs pull）**，已作为 549 `infra_todo` 追踪（操作者基础设施动作，蜂群无权限）。无新工位可推进映射，无新谱系号。
- **建议（接 §八扫描盲区）**：ceremony_scan 的 `missing_block_mapping` 检查应区分"549 冻结范围内的已知缺映射"（≥ 549，预期态）vs"真新异常"（< 549 的意外缺失），避免每轮 re-scan 重复报 549 下游为异常（= scan-state-unobservable 094 的逆向：已知冻结态被反复误报为新异常）。此为 ceremony-scan-completeness skill 草案的追加项。
