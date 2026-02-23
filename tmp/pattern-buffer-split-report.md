# pattern-buffer 分片报告

## 背景

`.chanlun/pattern-buffer.yaml` (297KB, 6231行, 701条条目) 是 context 耗尽的头号杀手，约占 200K context 的 48%。

## 分片策略

按 **工具类型 (Bash/Edit/Write) + 状态 (observed/candidate)** 分类，超过 30KB 的类别按条目数量均匀分批。

### 原始数据分布

| 类别 | 条目数 | 原始大小 |
|------|--------|---------|
| anomaly-bash-candidate | 265 | 124.5 KB |
| anomaly-bash-observed | 210 | 80.6 KB |
| anomaly-edit-candidate | 100 | 39.3 KB |
| anomaly-edit-observed | 19 | 7.0 KB |
| anomaly-write-candidate | 23 | 8.9 KB |
| anomaly-write-observed | 80 | 30.0 KB |
| patterns (pat-*) | 4 | 1.1 KB |

### 生成的分片文件

| 文件 | 条目数 | 大小 |
|------|--------|------|
| anomaly-bash-candidate-part1.yaml | 53 | 23.3 KB |
| anomaly-bash-candidate-part2.yaml | 53 | 24.6 KB |
| anomaly-bash-candidate-part3.yaml | 53 | 27.1 KB |
| anomaly-bash-candidate-part4.yaml | 53 | 27.1 KB |
| anomaly-bash-candidate-part5.yaml | 53 | 26.8 KB |
| anomaly-bash-observed-part1.yaml | 70 | 27.5 KB |
| anomaly-bash-observed-part2.yaml | 70 | 27.8 KB |
| anomaly-bash-observed-part3.yaml | 70 | 27.7 KB |
| anomaly-edit-candidate-part1.yaml | 50 | 20.2 KB |
| anomaly-edit-candidate-part2.yaml | 50 | 20.5 KB |
| anomaly-edit-observed.yaml | 19 | 7.4 KB |
| anomaly-write-candidate.yaml | 23 | 9.3 KB |
| anomaly-write-observed-part1.yaml | 40 | 15.5 KB |
| anomaly-write-observed-part2.yaml | 40 | 15.6 KB |
| patterns.yaml | 4 | 1.4 KB |
| **index.yaml** | - | **2.6 KB** |

所有分片 < 30KB，index.yaml 2.6KB < 5KB。

## 完整性验证

- 原始条目数：701（含 49 个预先存在的重复 ID）
- 分片后条目数：701
- 重复 ID 模式完全匹配原始文件
- 无数据丢失，无额外数据引入

## 更新的引用文件

| 文件 | 更新内容 |
|------|---------|
| `scripts/ceremony_scan.py` | 改为遍历 `.chanlun/pattern-buffer/` 目录下所有分片（保留对旧单文件的 fallback 兼容） |
| `.chanlun/dispatch-dag.yaml` | `buffer_path` → `buffer_dir` + `index_path` |
| `.chanlun/dispatch-spec.yaml` | 同上 |
| `.claude/agents/genealogist.md` | 文件路径引用从 `.yaml` 改为 `/` 目录 |
| `.claude/agents/quality-guard.md` | 同上 |
| `.claude/agents/skill-crystallizer.md` | 触发条件路径 + 状态更新说明 |
| `README.md` | 项目结构树中的路径 |
| `.chanlun/manifest.yaml` | skill-crystallizer 描述 |

## 注意事项

1. 原始 `pattern-buffer.yaml` 有 YAML 语法问题（signature 字段含未转义引号），分片文件继承了同样的问题。这是预先存在的——不在本次分片的修复范围内。
2. 49 个重复 ID 也是预先存在的，已如实保留。
3. `ceremony_scan.py` 包含向后兼容逻辑：如果分片目录不存在则回退到旧的单文件路径。
