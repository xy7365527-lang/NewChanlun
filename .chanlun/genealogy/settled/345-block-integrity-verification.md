---
id: "345"
title: "区块拓扑内容完整性验证机制"
status: "已结算"
type: "工程决策"
date: "2026-03-04"
depends_on: []
related: []
negated_by: []
negates: []
---

# 工程决策 345：区块拓扑内容完整性验证机制

**类型**: 工程决策

## 问题

区块拓扑作为谱系的索引层，只保证谱系→区块的映射关系，不保证谱系文件内容的不可变更性。

当前缺口：
1. 谱系文件（.md）是普通文本文件，任何编辑器都能修改
2. 区块中没有记录文件内容的哈希值（或仅记录了与 block_id 相同的值，不是文件内容哈希）
3. 修改谱系文件后，区块拓扑无法检测到内容变更
4. 没有区分"合法修改"（如标记下游推论已执行）和"非法篡改"

## 解决方案

引入 `content_hash` 字段（SHA256）作为文件内容的指纹：

### 验证脚本 `scripts/verify_block_integrity.py`

三种模式：
- **验证模式**（默认）：重新计算所有有源文件的区块的 SHA256，与 `content_hash` 比较
- **盖章模式**（`--stamp`）：为缺少 `content_hash` 的区块写入当前文件哈希
- **重签名模式**（`--resign NUM`）：合法修改后更新指定区块的 `content_hash`

### 区块格式变更

在现有区块 JSON 中新增顶层 `content_hash` 字段：
- 旧区块不受影响（向后兼容）
- `content_hash` 是谱系源文件内容的 SHA256
- 不同于 `block.id`（后者是 `{type, source, content, refs}` 的 SHA256）

### 合法修改流程

1. 编辑谱系文件
2. 运行 `python scripts/verify_block_integrity.py --resign <谱系编号>`
3. 重签名更新 `content_hash`
4. 提交区块文件变更

### 数据发现

首次运行结果：
- 1196 个区块中，367 个有源文件引用
- 364 个缺少 `content_hash`（需要 `--stamp`）
- 2 个的 `content_hash` 与文件内容不匹配（旧值是 block_id，不是文件哈希）
- 1 个源文件路径格式损坏（313号，`source_file` 字段中路径分隔符乱码）

## 边界条件

- 此验证是**读时检查**：只在验证脚本运行时检测，不阻止写入
- `content_hash` 不改变区块的不可变语义——区块本身（`block.id`）仍由内容寻址
- `content_hash` 是区块之外的元数据，标记源文件的状态

## 影响声明

- 新增 `scripts/verify_block_integrity.py`
- 新增 `tests/test_block_integrity.py`（22 个测试）
- 不修改 `scripts/block_topology.py`（核心模块零变更）
- 不修改现有区块格式定义（`content_hash` 是可选字段）
