---
id: "347"
title: "Content-Addressed Block Topology"
status: "已结算"
type: "工程决策"
date: "2026-03-04"
depends_on: ["345"]
related: ["090"]
negated_by: []
negates: []
---

# 工程决策 347: Content-Addressed Block Topology

**类型**: 工程决策
**状态**: 已结算
**日期**: 2026-03-04
**前置**: 345号（区块完整性验证机制）
**域**: 区块拓扑 (Block Topology)
**来源**: 编排者选择："选最严格的，让谱系变成真正的严格区块拓扑。"

---

## 决策描述

将区块拓扑从索引式（block_id = SHA256(metadata)）升级为 content-addressed storage（block_id = SHA256(full_text)）。

### 旧架构

- 区块 = 索引，指向 .md 文件
- block_id = SHA256(type + source + content_metadata + refs)
- content_hash 字段只能事后检测文件变化
- .md 文件可变，区块与内容之间是弱关联

### 新架构

- 区块 = 内容容器，存储完整 .md 文本于 `content.full_text`
- block_id = SHA256(full_text)——文件名即内容指纹
- 改内容 = 新区块，旧区块物理不可变
- `replaces` 字段建立修订链
- .md 文件 = 工作副本（方便阅读），区块才是 source of truth

### 存在论意义

索引式区块的 block_id 由 metadata 决定——同一份谱系内容可以有不同的 block_id（只要 metadata 不同）。这意味着区块的"身份"不取决于"它说了什么"，而取决于"它怎么被分类的"。

Content-addressed 区块的 block_id 由内容本身决定——说了同样话的区块必然是同一个区块，说了不同话的区块必然不同。身份 = 内容。不可变性不是外部约束（"不许改"），而是内在属性（"改了就是另一个"）。

## 实现

### 新增文件

- `scripts/block_ops.py`: Content-addressed 区块操作 API
  - `create_block(genealogy_path, blocks_dir)` → block_id
  - `get_block_content(block_id, blocks_dir)` → full_text
  - `verify_working_copy(genealogy_path, blocks_dir)` → bool
  - `rebuild_working_copy(block_id, output_path, blocks_dir)`
  - `create_revision(old_block_id, new_genealogy_path, blocks_dir)` → new_block_id

- `scripts/migrate_to_content_addressed.py`: 迁移脚本
  - `plan_migration(root)` → dry-run 报告
  - `execute_migration(root)` → 实际迁移
  - 幂等：可安全多次运行
  - 更新 meta.json id_mapping + relations.jsonl 中的旧 hash

### 修改文件

- `scripts/verify_block_integrity.py`: 新增 content-addressed 验证路径
  - 区块有 `content.full_text` → 验证 `block_id == SHA256(full_text)`
  - 额外检查工作副本漂移（.md 与 full_text 不一致）
  - 向后兼容：legacy 区块仍走 content_hash 验证

### 测试

- `tests/test_block_ops.py`: 27 个测试（create/get/verify/rebuild/revision/round-trip）
- `tests/test_content_addressed_migration.py`: 18 个测试（plan/execute/idempotent/remap）
- `tests/test_block_integrity.py`: 新增 5 个 content-addressed 验证测试

## 边界条件

- 无 source_file 的区块（rewrite/tension/consensus）迁移时保持不变
- 空内容 .md 文件不允许创建区块（ValueError）
- 内容未变化时 create_revision 拒绝创建（ValueError）
- 迁移后 legacy content_hash 字段被移除

## 迁移策略

迁移脚本已就绪但未执行。1198 个区块的实际迁移需要编排者确认后运行：
```
python scripts/migrate_to_content_addressed.py            # dry-run
python scripts/migrate_to_content_addressed.py --execute  # 实际迁移
```
