---
id: '424'
number: 424
title: "S_net 持久化缓存实装——423号候选5的工程闭合"
type: 回溯结算
status: 生成态
date: 2026-03-11
source: "[新缠论] snet-persist 工位产出 + genealogist 评估"
depends_on:
  - '423'   # 候选5：持久化边界模糊
  - '399'   # 器官性阅读范式（S_net 摄入架构）
epistemological_level: L0
negation_form: none
negation_source: ""
topo_effect: ""
tensions_with:
  - '425'   # S_net 入图问题（如果创建）
rule_version_baseline:
  claude_md_commit: "c937c0d"
  rules_dir_mtime: "2026-03-11"
---

# 424号：S_net 持久化缓存实装

## 推导链

1. 423号候选5 识别了持久化边界模糊：daemon 重启需 30-60 分钟重新摄入语料，但穿越状态已持久化
2. 根因：S_net（signifier_net.py 的 SNet 对象）没有序列化/反序列化接口，无法保存到磁盘
3. v226-swarm snet-persist 工位实装了完整解决方案：
   - `signifier_net.py` 新增 `to_dict()` / `from_dict()` 方法（L0：无损代数操作，索引从 edges 重建）
   - `snet_cache.py` 新文件：三路分支缓存策略
   - `daemon.py` 集成：`_bootstrap_snet` 方法先尝试缓存加载

### 三路分支策略

| 分支 | 条件 | 行为 | 预期耗时 |
|------|------|------|---------|
| 完全命中 | manifest hash 全部匹配 | 直接 pickle.load + gzip 解压 | 数秒 |
| 增量摄入 | 部分文件变更（surface_forms 未变） | 加载缓存 + 只摄入新增/变更文件 | 数秒~数分钟 |
| 完整重建 | 无缓存 / surface_forms 变更 | 全量摄入 + 保存缓存 | 30-60 分钟（首次） |

### manifest 机制

- 文件清单 + SHA-256 快速 hash（前 64KB + 文件大小）
- 比较 (path, hash) 集合，不比较 mtime
- 缓存路径：`~/.swarm/persist/snet_cache/snet_cache.pkl.gz`

## 定义依据

- 423号候选5："持久化边界的明确化——哪些实体应该被持久化为减少启动耗时"
- 399号观察1：器官性阅读范式——S_net 是语料摄入的目标容器
- S_net 的存在论位置：持久实体（block topology 中），不是瞬间快照

## 谱系链接

- **回溯结算**：423号候选5（持久化边界模糊，1 实例）→ 本条实装闭合了该候选的工程缺口
- **前置**：399号（器官性阅读范式——定义了 S_net 的摄入管线）
- **关联**：423号候选4（生产-消费-资源三角形失配）的第 5 实例（S_net 生产不持久化，消费者期望已加载）——本实装修复了该失配

## 影响

- 新增文件：`topological-computation/snet_cache.py`（505 行）
- 修改文件：`topological-computation/signifier_net.py`（新增 to_dict/from_dict 约 100 行）
- 修改文件：`topological-computation/daemon.py`（_bootstrap_snet 集成缓存逻辑）
- 预期效果：daemon 重启从 30-60 分钟降至数秒（缓存命中时）
- 影响模块：daemon 启动流程、S_net 生命周期管理

## 边界条件

1. **缓存一致性**：pickle 序列化依赖 SNet 类结构稳定。如果 SNet 的 __init__ 签名或内部数据结构变更，旧缓存将反序列化失败（graceful degradation：fallback 到完整重建）
2. **surface_forms 变更**：surface_forms 变更触发完整重建（因为 bootstrap_layer_b 依赖它）。如果 surface_forms 频繁变更，缓存命中率下降
3. **L2 验证缺失**：缓存加载后的 S_net 与完整重建的 S_net 是否代数等价，尚未在真实数据上验证。to_dict/from_dict 的 L0 等价性是代数保证，但 pickle 序列化的位级等价性未验证

## 回溯结算

- settled_by: 本条（424号）
- settlement_description: snet-persist 工位实装了 S_net 序列化接口 + 三路缓存策略 + daemon 集成，闭合了 423号候选5 的工程缺口

## 谱系关联

related_records:
  parent: '423'  # 候选5 的来源
  children: []   # 待 425号（S_net 入图）确认后可能成为前置
