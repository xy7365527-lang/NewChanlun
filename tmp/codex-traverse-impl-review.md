# Codex 代码审查：Phase 3 穿越基础设施

审查日期：2026-03-02
审查者：Codex 级别审查（claude-sonnet-4-6）
基准规格：tmp/phase3-discuss-report-r2.md

---

## 严重性分类

### Fatal (必须修复)
**F-1**: query_pair 内部二次构建 MorseLandscape，接口不对称导致双重 I/O 与潜在数据不一致

位置：scripts/traverse.py:160-163

query_block 接受外部传入的 MorseLandscape 参数，避免重复 I/O。
而 query_pair 在找到路径后内部重建 MorseLandscape：

这导致三个问题：

(1) 接口不一致：同批次查询中 query_block 用调用方传入的 morse 对象，query_pair 自己读磁盘重建。若 relations.jsonl 在两次查询之间更新，两者读到不同版本。

(2) 双重 I/O：query_pair 首先通过 read_all_relations(base) 读一次 relations.jsonl 构建无向邻接表，找到路径后再调用 build_morse_landscape 又完整读一次。对 1156 条边做了两次完整读取和处理。

(3) 路径常量独立：morse_landscape.py 的 DEFAULT_RELATIONS_PATH 与 block_topology.DEFAULT_BASE 是两套独立常量，路径値相同但无引用关系。当 query_pair 接受非默认 base 时，两路径常量的独立性是长期隐患。
**F-2**: get_block_mapping 签名与 R2 规格存在结构性分歧

规格（R2 收敛规格）定义: path: str = "data/block_topology.jsonl"  # 读单个 JSONL 文件
实现: base: Path = DEFAULT_BASE  # 读 blocks/ 目录下所有 JSON 文件

两个接口完全不同——规格读 JSONL，实现读目录。实现方案更合理（与现有存储结构一致），但这是规格与实现的结构性分歧。需要确认规格是否已变更为当前实现方案。

---

### Important (应该修复)

**I-1**: query_pair BFS 路径跟踪内存开销 O(V^2)

位置：scripts/traverse.py:147-174

每次入队都复制完整路径列表（path_nodes + [neighbor]）。BFS 最坏情况总内存 O(V^2)。正确做法是只存 prev 字典，找到 dst 后回溯路径，内存 O(V)。对 296 节点规模当前不会 OOM，但不是正确实现模式。

**I-2**: build_concept_registry 的 authoritative 判定依据与规格不一致

规格（R2 焦点4）: "authoritative 标记：block_topology 的 type=definition"
实现（concept_registry.py:95）: has_definition = bool(rel.get("concept_definition"))

规格依据区块类型（type=definition），实现依据关系边字段（concept_definition 非空）。如果 type=definition 区块写 defines 边时保证携带 concept_definition 字段，两者等价。若不保证，行为不同。需要确认这是规格变更还是实现偏差。

**I-3**: MorseLandscape frozen=True 与 dict 字段产生误导性 hashability

位置：scripts/morse_landscape.py:22

frozen=True 使实例看似可哈希，但 dict 字段导致 hash() 调用时抛出 TypeError: unhashable type: "dict"。如果此对象被用作 dict key 或放入 set 将在运行时报错。规格未要求 MorseLandscape 可哈希，frozen=True 在此产生误导。建议：去掉 frozen=True 并加不变性说明注释。

**I-4**: check_encounter 的调用前提（边已写入）仅靠注释，未在代码中强制

位置：scripts/encounter_guard.py:53

若调用顺序颛倒（先 check_encounter 后写边），Kruskal 不含新边，返回 False（tree）。边写入后实际应为 critical，静默错误，不抛异常。规格（R2 焦点3）的 add_relation 集成点未实现为统一函数，这是架构层面的隐患。

**I-5**: DFS 环检测使用递归，存在 Python 栈溢出风险

位置：scripts/encounter_guard.py:128-141

Python 默认递归深度 1000。depends_on 图若形成 300+ 节点长链，可能触发 RecursionError。建议改为显式栈迭代 DFS。
---

### Suggestion (建议)

**S-1**: query_pair 与 query_block 接口不对称，建议统一
建议 query_pair 也接受可选 morse: MorseLandscape | None = None 参数，None 时才内部重建。批量查询时可复用同一 morse 对象。

**S-2**: morse_landscape.py 路径常量与 block_topology.DEFAULT_BASE 独立定义
建议 morse_landscape.py 从 block_topology.DEFAULT_BASE 推导路径，避免路径字符串重复。

**S-3**: query_block 的 morse_summary 缺少 cycle_ids 字段
规格 R2 焦点2: morse_summary: {critical_out, critical_in, cycle_ids}
实现只返回: critical_out, critical_in。cycle_ids 缺失需明确标注（有意省略还是遗漏）。

**S-4**: query_pair 返回的 edge_key 方向与遍历方向不一致，需文档说明
从 B 走到 A（反向穿越 A->B 边）时，edge_key 仍是 "A:B"。这是有意为之（保持与 morse edge_marks key 一致），建议在 docstring 中明确说明。

**S-5**: UnionFind.union 中 swap 后变量语义不直观，建议加注释。

**S-6**: CLI main 缺少 --base 参数，路径依赖当前工作目录。建议增加 --base 参数支持配置化路径。

**S-7**: test_traverse.py 中 query_pair 测试未验证 marks 字段内容。建议补充验证 tree/critical 场景的测试用例。

---

## 规格一致性检查（10 项）

| # | 检查项 | 结果 |
|---|--------|------|
| 1 | MorseLandscape dataclass 字段与规格一致 | PASS: edge_marks + stats 字段名称和类型一致；stats 含额外 tree 字段（规格未禁止） |
| 2 | build_morse_landscape 参数、排序、无向化规则 | PASS: timestamp ASC + (from, to) tie-breaking；无向化不合并双向引用 |
| 3 | 862 = 1156 - 296 + 2 硬约束在测试中验证 | PASS: TestRealData.test_real_data_hard_constraint 验证三个数値和公式 |
| 4 | query_block/query_pair/query_concept 返回结构 | PARTIAL: query_block 缺 cycle_ids；其余字段与规格一致 |
| 5 | LAYER_MAP 覆盖完整 | PASS: test_layer_map_completeness 验证 LAYER_MAP 覆盖全部 RELATION_TYPES |
| 6 | 概念注册表格式与规格一致 | PASS: 单文件 JSON，路径 .chanlun/concept_registry.json，4 字段格式正确 |
| 7 | 偶遇检测使用纯重算策略 | PASS: cache_path=None 强制全量重算，无增量状态 |
| 8 | DAG 守护仅在 depends_on 子图检测，硬拒绝 | PASS: 只过滤 depends_on 边；raise CircularDependencyError |
| 9 | query_pair 仅在 references-only 子图 BFS（无 Dijkstra） | PASS: 只过滤 references 边；纯 BFS，无权重 |
| 10 | R2-D4: 有向边直接视为无向边，不合并双向引用 | PASS: 两个方向都加入邻接表；edge_key 保持有向格式；test_no_merge_bidirectional 验证 |

---

## 总结

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| FATAL | 2 | F-1（接口不对称 + 双重 I/O），F-2（规格签名分歧） |
| IMPORTANT | 5 | I-1 到 I-5 |
| SUGGESTION | 7 | S-1 到 S-7 |

结论：WARNING

算法正确性整体良好：
- Union-Find path compression + union by rank 实现正确
- BFS 无向图实现正确（双向邻接表，edge_key 保持有向格式）
- DFS 环检测逻辑正确（自环 + 间接环均覆盖）
- 时序 Kruskal 排序策略（timestamp ASC + tie-breaking）正确

10 项规格一致性检查 9 项通过，1 项部分通过（cycle_ids 缺失）。

主要问题：F-1（query_pair 接口不对称导致双重 I/O）和 F-2（get_block_mapping 规格签名分歧）需要在合入前明确解决或确认规格变更。I-2（authoritative 判据）需要确认写入侧行为。