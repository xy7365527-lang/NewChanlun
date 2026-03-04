# Codex: Phase 3 Round 2 实现规格讨论

==============================
焦点 1：组件三（Morse 地形）
==============================

1. 输出持久化  
   推荐：默认**启动时全量重算**，可选缓存。  
   • 函数 `build_morse_landscape(..., cache_path: str | None = ".chanlun/morse_landscape.json")`  
   • 若 `cache_path` 存在且 `mtime(cache) ≥ mtime(relations.jsonl)`，则直接反序列化；否则重算并覆盖写入。  
   • 文件格式：JSON  
     ```json
     {
       "algo_version": "ts_kruskal_v1",
       "generated_at": "2024-03-02T12:00:00Z",
       "edge_marks": { "e_hash": "tree" | "critical", ... }
     }
     ```

2. 无向化  
   • 先对 `(min(a,b), max(a,b))` 做 key，若出现两条相反方向则只保留一条，无权值重复。  
   • 如需保留方向信息，可把 “双向” 另存到 `duplicates` 集合供 UI 查询，不纳入 Kruskal。  
   • 这样得到无向边 997 条（≤1156），保持裁定的 `cycle_rank = 862` 逻辑一致。

3. 增量更新  
   • 数据量小 → 简单可靠：新增 reference 边时直接触发 `rebuild_morse_landscape()` 全量重算。  
   • 重算 O(E log E) ≈ 1 ms，不必维护复杂增量 UF。

4. 862 条 critical 边的导航  
   • MorseLandscape 提供  
     - `critical_edges()` → list[Edge]  
     - `cycles()` → dict[int/*cycle_id*/, list[Edge]]  // 依据 Union-Find root 做连通类拆分  
   • `traverse.py` 中暴露：  
     ```bash
     traverse list-critical [--group-by cycle|none] [--format json/table]
     ```

5. 数据结构  
   ```python
   @dataclass
   class MorseLandscape:
       edge_marks: dict[str, Literal["tree", "critical"]]  # key = edge_sha1(from+to+ts)
       cycles: dict[int, list[str]]  # cycle_id -> list[edge_sha1]
       stats: dict[str, Any]         # {nodes, edges, critical_cnt, algo_version}
   ```

----------------------------------------------------
未确定 / 需确认  
• “双向引用”是否一定视为同一概念？若保留两条需调整 cycle_rank 预期。  
• 是否需要把各 cycle 输出成 topo-sorted block 列表？

==============================
焦点 2：组件一（traverse.py）
==============================

1. query_block  
   ```python
   def query_block(block_id: str,
                   morse: MorseLandscape | None = None,
                   registry: ConceptRegistry | None = None) -> dict
   ```  
   返回：
   ```
   {
     "block_id": "...",
     "genealogy_number": 42,
     "title": "...",
     "layer": 2,
     "status": "active",
     "out_edges": {
        "references": [ {to,id,mark:"critical"}, ... ],
        ...
     },
     "in_edges": {...},
     "concepts": [ {"concept_id": "...", "term": "...", "authoritative": true}, ... ],
     "morse_summary": {"critical_out": 3, "critical_in": 1, "cycle_ids": [7,21]}
   }
   ```

2. query_pair  
   - 默认仅在 `references` 子图（与 Morse 保持一致）；`--all-relations` 切换到全图。  
   - 无权：BFS 即可；若 --prefer-tree 则对 “tree” 边权 0、“critical” 权 1 用 Dijkstra。  
   ```python
   def query_pair(src: str, dst: str, use_full: bool = False,
                  prefer_tree: bool = False) -> dict | None
   ```  
   输出：  
   ```
   {"nodes": [id1,id2,...], "edges": ["references","references",...], "marks": ["tree","critical",...]}
   ```

3. query_concept  
   ```python
   def query_concept(concept_id: str, registry: ConceptRegistry) -> dict
   ```  
   返回：
   ```
   {
     "term": "...",
     "authoritative_blocks": [...],
     "other_blocks": [...],
     "reference_edges": [{"from": id1, "to": id2, "mark": "..."}]
   }
   ```

4. CLI 输出  
   • `--format json` 输出 JSON；默认人类可读表格（rich + tabulate）。  
   • 出错：写 stderr，exit 1，JSON 格式时 `{"error":"...","code":404}`。

5. 错误处理  
   - 块不存在 → 404 + exit 1  
   - 概念无定义 → 200 + 空数组  
   - 路径不可达 → `None` / `"unreachable": true`

----------------------------------------------------
需确认  
• BFS/Dijkstra 时是否限制最大深度？  
• 人类表格应包含 Morse 标记配色方案？

==============================
焦点 3：组件二（偶遇记录）
==============================

1. Union-Find 状态  
   推荐方案 A：每次 `add_relation()` 调用时**重建 UF**。  
   - 代码简洁、无需锁  
   - 对 1156 条边，重建 <1 ms  
   - 函数 `is_critical_edge(u,v) -> bool`

2. topo_event 写入  
   推荐方案 B：集中日志 `.chanlun/topo_events.jsonl`，每行：  
   ```
   {"ts":"2024-03-02T12:01:55Z","edge":{"from":u,"to":v},"event":"new_critical"}
   ```  
   优点：append-only、方便审计；不污染区块元数据。

3. depends_on DAG 守护  
   - 算法：插入 (u,v) 时 DFS v→… 查 u  
   - 出现环：拒绝并抛 `CircularDependencyError`（保持 Layer1 正确）。  
   - 错误对象带冲突路径供提示。

4. 偶遇语义  
   • 仅记录事件 + CLI 提醒：`traverse pending-encounters`。  
   • 后续可挂 webhook 推送，不自动生成区块。

----------------------------------------------------
需确认  
• 是否需要对重复 critical 边去重报警？  
• 非 references 类型边仍记录 topo_event 吗？

==============================
焦点 4：组件四（概念注册表）
==============================

1. 文件格式  
   推荐单文件 JSON：`.chanlun/concept_registry.json`  
   结构：
   ```json
   {
     "concept_id": {
       "term": "Maximum Modulus Principle",
       "authoritative": true,
       "defining_blocks": ["b1","b2"],
       "reference_count": 37
     },
     ...
   }
   ```

2. 文件路径  
   `.chanlun/` 目录已聚合缓存，便于版本管控与 CI 校验。

3. 更新策略  
   • ceremony 结束后**全量重导**：保证与数据快照一致，避免累积漂移。  
   • CLI：`python tools/export_concepts.py --force`

4. authoritative 标记  
   - 来源于 block_topology 里 `type == "definition"`  
   - 若缺失该字段，默认 `authoritative=false` 并在日志发 WARN。

5. 人类可读字段  
   `term` 必含原始短语；ID 继续用 SHA-256，保持稳定指针。

----------------------------------------------------
需确认  
• 未来是否需要二级索引 term→concept_id（处理同义词）？

==============================
焦点 5：前置基础设施
==============================

1. classify_layer  
   ```python
   LAYER_MAP = {
       # layer1
       "depends_on":1,"negates":1,"negated_by":1,"supersedes":1,"residue_of":1,
       "reopens":1,"tensions_with":1,"freezes":1,"splits":1,"severs":1,
       # layer2
       "references":2,"defines":2,"modifies":2,"refines":2,"revises":2,"annotates":2,
       # layer3
       "records":3,"related":3,
   }

   def classify_layer(rel_type: str) -> int:
       try:
           return LAYER_MAP[rel_type]
       except KeyError:
           raise ValueError(f"Unknown relation type: {rel_type}")
   ```

2. block_id ↔︎ genealogy_number  
   ```python
   def get_block_mapping(path="data/block_topology.jsonl"
                         ) -> tuple[dict[str,int],dict[int,str]]:
       id2num, num2id = {}, {}
       with open(path) as fh:
           for line in fh:
               rec = json.loads(line)
               bid, num = rec["block_id"], rec["genealogy_number"]
               id2num[bid] = num
               num2id[num] = bid
       return id2num, num2id
   ```
   - 进程级缓存：用 `functools.lru_cache(maxsize=1)` 装饰。  
   - 单元测试：构造 3 条记录文件，断言映射互逆。

3. concept_id → block_id 反向索引  
   • 直接从概念注册表反序列化：`registry[concept_id]["defining_blocks"]`  
   • 若注册表缺失时回落到全量扫描 defines 关系。  
   ```python
   def load_concept_index(registry_path=".chanlun/concept_registry.json"
                          ) -> dict[str, list[str]]:
       try:
           return {k: v["defining_blocks"] for k, v in json.load(open(reg_path)).items()}
       except FileNotFoundError:
           return rebuild_from_relations()
   ```

----------------------------------------------------
测试要点  
• classify_layer 对未知类型抛异常  
• block_mapping 双射一致  
• concept_index 与 relation 扫描结果相同（基准数据集）

==============================
整体接口一致性与测试
==============================

• MorseLandscape、ConceptRegistry 均为只读对象，在进程内共享。  
• `traverse.py` 的所有查询函数都接受这两类对象，方便注入 mock 实例进行单元测试。  
• 主 CLI 统一 `--format` `--json-indent` `--color` 选项。  
• pytest 目录结构  
  ```
  tests/
    test_morse.py
    test_traverse_block.py
    test_encounter.py
    test_concept_registry.py
    test_layer_classify.py
  ```

==============================
与 Gemini 潜在分歧
==============================

• 我将双向引用折叠为一条无向边；若 Gemini 需要保留两条，需同步 cycle_rank 定义。  
• depends_on 环检测：我主张“硬拒绝”插入，Gemini 可能倾向“标记警告”。  
• 概念注册表全量重导，Gemini 可能主张增量；理由见更新策略。