# 纲举目张代码层异质审查结果

**日期**：2026-02-24
**审查模式**：review
**模型**：gpt-5.2-codex（codex-5.3 降级）
**持久化**：`.chanlun/review-results/codex-review-20260224-0613.md`（CLI 自动完成）

---

## 一、Codex 原始审查结论（6项）

| # | 严重性 | 位置 | 问题描述 |
|---|--------|------|---------|
| 1 | **致命** | `consensus_trigger.py: trigger_ceremony` | 全仓库无生产级调用点，整条管道不可达，属于死代码 |
| 2 | **重要** | `stance_parser.py`, `consensus_trigger.py: extract_from_*` | `parse_stance_sequence()` 可能得到空序列，`extract_from_*()` 无空序列硬校验，导致静默失败 |
| 3 | **重要** | `consensus_trigger.py: extract_from_*/trigger_ceremony` | `unresolved=None` 可能被透传至 tension block，类型不符合定义 |
| 4 | **重要** | `consensus_ceremony.py: write_consensus_ceremony` | 三区块"原子写入"若为多次调用则仅单块级别原子，跨三块失败留残缺 |
| 5 | **重要** | `block_topology.py: query_relations` + 迁移数据 | 旧 relations.jsonl 可能缺 `order` 字段，导致 `query_relations(order=1)` 失效，解释 reflexive layer 为 0 |
| 6 | **建议** | `ceremony_scan.py: detect_genealogy_anomalies` | 仍基于文件名编号体系，未完全迁移为 block-topology ID 体系 |

---

## 二、代理质询与判定

### 第1项（致命）：trigger_ceremony 死代码 → **否定成立**

`trigger_ceremony()` 在全仓库确实无任何生产调用点。`detect_convergence_from_review_file()` 和 `find_trigger_block_for_review()` 同样存在但从未被调度。

**完整调用链断点位置**：调用链在代码层面是自洽的（参见下方 Q1 分析），但没有任何 CLI、hook 或 daemon 触发链的入口。管道在 `stance_parser.parse_stance_declaration()` 之前就已断开——因为没有代码将 Gemini/Codex 的回复文本传入 `parse_stance_declaration()`。

### 第2项（重要）：空序列静默失败 → **否定成立**

`extract_from_gemini_verify()` 和 `extract_from_plan_review()` 都接受 `stance_sequence: list[StanceDeclaration]`，但没有 `if not stance_sequence` 守卫。空序列时：
- `derive_concession_trace([])` 返回空 `ConcessionTrace`
- `computed_concessions = []`，`gemini_conceded = []`，`codex_conceded = []`
- 仍然调用 `trigger_ceremony()` 写入三块，但 residue 内容为空列表
- 这是**静默失败**：写入了语义无效的 ceremony，没有错误

### 第3项（重要）：unresolved=None 透传 → **否定不成立（Codex 误判）**

`extract_from_gemini_verify()` 第271行：
```python
unresolved=unresolved if unresolved is not None else [],
```
`extract_from_plan_review()` 第327行：
```python
unresolved=unresolved if unresolved is not None else [],
```
两处均已归一化为 `[]`，`InquiryCycleResult.unresolved` 接收到的始终是 `list[str]`，不存在 `None` 透传问题。**Codex 未读到这个归一化行。**

### 第4项（重要）：三区块原子性 → **否定不成立（Codex 误判）**

`write_consensus_ceremony()` 最后：
```python
all_blocks = [consensus, residue, tension, rewrite_residue, rewrite_tension]
all_relations = [rel_residue, rel_tension, records_residue, records_tension]
_atomic_write_ceremony(all_blocks, all_relations, base)
```
5块+4关系通过单一 `_atomic_write_ceremony()` 调用一次性写入临时目录后 commit，不是多次调用。原子性已正确实现。**Codex 对实现路径的假设是错误的。**

### 第5项（重要）：旧关系缺 order 字段 → **否定不成立（Codex 误判）**

实测：958条关系全部含 `order=1` 字段，`query_relations(order=1)` 返回全部 958 条。

Reflexive layer 为 0 的真实原因（Codex 未诊断出）：
- 958条旧关系由迁移脚本通过 `append_relation()` 直接写入，**绕过了** `write_block_with_relations()` 的 rewrite 触发逻辑
- `write_block_with_relations()` 的 rewrite 触发仅在写入新区块时生效，对已存在的迁移关系无追溯
- 只有共识仪式（`write_consensus_ceremony()`）或新的 order=1 关系写入才会产生 rewrite 区块
- 这是设计层面的现实（迁移关系不追溯触发 rewrite），不是 bug

### 第6项（建议）：ceremony_scan 迁移不完整 → **部分成立**

`detect_genealogy_anomalies()` 已包含 block-topology completeness check（第471-494行），会核查每个 settled 文件是否在 `meta.json id_mapping` 中有对应。但旧的文件名编号一致性和 frontmatter schema 检查仍保留，这是并行检查而非互斥替换。合理性取决于是否需要保留双重验证。

---

## 三、管道连通性诊断（Q1-Q4）

### Q1：trigger_ceremony() 调用链完整性

调用链在代码层面自洽，但存在**生产入口断点**：

```
[断点所在] → Gemini/Codex 回复文本
                  ↓ 无代码传递文本
  stance_parser.parse_stance_declaration()
                  ↓
  consensus_trigger.extract_from_*()
                  ↓
  consensus_trigger.trigger_ceremony()
                  ↓
  consensus_ceremony.write_consensus_ceremony()
                  ↓ 磁盘写入 ✓（代码正确）
```

断点不在数据结构层，在**调用入口层**：没有任何脚本读取 review-results 文件然后驱动 stance 解析。

### Q2：ceremony_scan 迁移状态

- `get_frozen_nodes()` → 已迁移到 block-topology relations.jsonl ✓
- `detect_genealogy_anomalies()` → 已增加 block-topology check，保留旧编号检查（双重检查）
- dag.yaml 完整性检查 → 已删除，无等价替代（Codex 第6项部分成立）

**迁移结论**：`ceremony_scan` 不再依赖 dag.yaml（dag.yaml 完整性检查已删除）。主要读取路径已切换到 block-topology。

### Q3：共识仪式第一次执行的最短路径

最短路径（技术可行）：
```python
from scripts.block_topology import list_blocks, DEFAULT_BASE
from scripts.consensus_trigger import InquiryCycleResult, trigger_ceremony

# 取任意一个现有 event 区块作为 trigger
blocks = list_blocks(block_type="event")
trigger_id = blocks[0]["id"]

# 手动构建 InquiryCycleResult（绕过 stance 解析流程）
result = InquiryCycleResult(
    scenario="gemini_verify",
    trigger_block_id=trigger_id,
    conclusion="手动触发测试仪式",
    gemini_conceded=[],
    codex_conceded=[],
    concession_reasons={},
    unresolved=[],
)

# 触发仪式
blocks_written = trigger_ceremony(result)
print(blocks_written)
```

前置条件：无特殊条件，仅需 `PYTHONPATH=.` 和有效的 trigger_block_id（已有 182 个可用）。

**技术障碍**：无技术障碍。第一次执行可以是手动 CLI 调用，不需要先跑通完整质询循环。

### Q4：Reflexive layer 实现方案

当前 Reflexive layer 为 0 的根因：迁移写入绕过了 rewrite 触发逻辑（`append_relation()` vs `write_block_with_relations()`）。

激活路径：
1. **最简**：运行一次共识仪式（`trigger_ceremony()`），内部 `write_consensus_ceremony()` 自动生成 2 个 rewrite 区块（`rewrite_residue` + `rewrite_tension`）
2. **追溯激活**：对现有 958 条 depends_on 关系批量触发 rewrite（不推荐——这 958 条是迁移产物，追溯 rewrite 无语义价值）

---

## 四、结论

### 否定成立的问题（需处理）

**[致命] 管道无生产入口**
- 影响模块：`consensus_trigger.py`、所有 review-results 文件
- 修复路径：新增 hook 或 CLI，读取 `.chanlun/review-results/` → 检测收敛 → 解析 stance → 调用 `trigger_ceremony()`
- 最短路径：先用手动 Python 调用验证端到端路径，再自动化

**[重要] 空序列静默失败**
- 影响模块：`consensus_trigger.py: extract_from_*`
- 修复路径：在 `extract_from_*()` 入口加 `if not stance_sequence: raise ValueError(...)` 守卫

### 否定不成立的误判（Codex 未读到的防御）

| 项 | 误判原因 |
|---|---|
| 第3项（unresolved=None）| Codex 未读到第271/327行的 `... if unresolved is not None else []` 归一化 |
| 第4项（原子性）| Codex 对实现路径假设错误——实际是单一 `_atomic_write_ceremony()` 调用 |
| 第5项（order 字段）| Codex 假设迁移数据无 order 字段，实测全部含 order=1 |

### 边界条件

- 若未来有新的迁移脚本绕过 `write_block_with_relations()`，第5项的误判会变为真实问题
- 若 Gemini/Codex 回复格式不遵守 stance 输出协议，第2项（空序列）将直接触发

### 影响声明

- 无代码修改（本次为纯审查）
- 审查结果写入 `.chanlun/review-results/codex-review-20260224-0613.md`（CLI 自动完成）
