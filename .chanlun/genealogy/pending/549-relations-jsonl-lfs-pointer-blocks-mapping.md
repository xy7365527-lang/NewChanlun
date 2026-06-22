---
id: '549'
number: 549
title: "relations.jsonl 沦为未实体化 LFS 指针——476号(append-only ⊥ long-running)张力的物质显形，冻结 block-topology 映射管线"
type: 矛盾发现
status: 生成态  # 待操作者基础设施动作解决（git lfs pull 实体化 OR 从 relations.jsonl.bak 恢复）
date: '2026-06-22'
level: "L0（block_topology.py:441-444 read_all_relations 对 LFS 指针首行 json.loads 必抛 JSONDecodeError；map_genealogy_to_blocks.py Phase3 路径分析）+ L1（文件状态：134 字节指针 vs meta.json relation_count=6859 vs relations.jsonl.bak 323KB）"
负责工位: genealogist（session-4130a713/genealogy-maintenance，拓扑映射任务 #8 衍生）
provenance: "[新缠论:实装审计]（block-topology 映射管线 + git-lfs 状态）"
negation_source: homogeneous
negation_form: waiting  # après-coup：当前不可结算，待操作者基础设施动作后回溯规定解冻
negates: "隐含命题『block-topology 映射管线（map_genealogy_to_blocks.py）随时可执行』——被证伪：relations.jsonl 是 134 字节 LFS 指针（LFS object 未 pull），read_all_relations 对其首行 `version https://git-lfs.github.com/spec/v1` 调 json.loads 必抛 JSONDecodeError；mapper Phase3 (_build_existing_keys→read_all_relations) 在 Phase2 已 write_meta 之后崩溃 = 部分损坏（meta.json 已加 531-547，relations 未写，脚本异常退出）"
topo_effect: "freeze:block-topology-mapping-pipeline:downstream（冻结 531-547 映射 + 一切后续谱系→区块映射；待 relations.jsonl 实体化后解冻）"
depends_on:
  - '476'   # jsonl append-only ⊥ long-running——本号是其物质显形（无界 append → relations.jsonl 膨胀 → 迁 LFS → 未实体化）
  - '178'   # 区块拓扑建系（relations.jsonl append-only 关系层的来源）
related:
  - '424'   # snet persistence implementation（同类持久化-长运行张力）
tensions_with:
  - '178'   # 178（建立 append-only relations.jsonl 关系层）⊥ 476（append-only 不兼容 long-running）——本号是该张力边在 git-lfs 层的物质实例
---

# 549 号：relations.jsonl 沦为未实体化 LFS 指针——冻结 block-topology 映射管线

**认识论**：L0（源码崩溃路径）+ L1（文件状态不一致）。**状态生成态**：解决依赖操作者的基础设施动作，蜂群不自行裁决（git-lfs 配置 + 哪个状态是正典涉及操作者判断）。

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
| relations.jsonl.bak（真实 JSONL） | 323,173 B | 真实备份，≈ 6859 关系 × ~47B/行，**与 relation_count 自洽** |

125MB（LFS）与 6859 关系（meta）/323KB（.bak）**量级不符**——指向 relations.jsonl 历史上曾无界膨胀（476号张力）后迁入 LFS，而当前正典关系层实为 .bak。三方不一致 = 操作者需裁决哪个是正典。

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

满足任一即可解冻映射，本号回溯结算：
1. **`git lfs pull`** 实体化 relations.jsonl（若 LFS 125MB object 是正典）；或
2. **从 relations.jsonl.bak 恢复** relations.jsonl（若 .bak 的 6859 关系是正典，与 meta.json 自洽，更可能）；
3. 解决后再跑 `map_genealogy_to_blocks.py --all-unmapped` 映射 531-547（幂等，content-addressed + 关系去重）。

## 七、影响声明

谱系结晶（pending/生成态），未改任何文件，未运行 mapper。**冻结**：531-547 的 block-topology 映射 + 17 个 missing_block_mapping 异常（task #8）+ 一切后续谱系→区块映射，直到 relations.jsonl 实体化。已 SendMessage team-lead 上浮基础设施阻塞（动作类，非概念裁决）。
