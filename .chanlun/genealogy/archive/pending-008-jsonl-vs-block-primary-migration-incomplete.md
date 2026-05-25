---
id: pending-008-jsonl-vs-block-primary-migration-incomplete
timestamp: 2026-04-27
status: 已上浮-待编排者裁决
settlement: 上浮（方向A定理偏向，但执行=逢亮子系统专门会话；资源/时序上浮）
settled_date: 2026-05-24
settled_classification: 选择（资源/时序）+ 定理偏向方向
settlement_scope: "迁移方向 A(完成迁移)被no-patch偏向、B(双栈)被161号否；但 A/C 执行均触及逢亮持久化/概念穿越子系统，高blast-radius，需专门会话；merkle删除非干净死代码(在逢亮概念空间)。上浮资源/时序给编排者"
type: domain
negation_source: homogeneous
negation_form: expansion
topo_effect: "split:178:downstream"
---

## 推进（2026-05-24，方向定理偏向 + 执行需专门会话——上浮）

**决断四分法：方向是定理偏向，执行/时序是选择。**
- 路径 A（K_active/K_full 也走 block primary，废 JSONL）= 严格扬弃，**no-patch-mentality 偏向**。
- 路径 B（双栈合法化，更新 178号声明"概念层 JSONL/物质层 block"）= 务实退让，**161号否定**。
- 故方向被原则偏向 A。但——

**为何不能 inline 执行（高 blast radius，需专门会话）：**
1. **A 触及 live 持久化层**：K_active/K_full 是逢亮运行时持久化，迁移仓促做 = 数据损坏 / daemon 崩溃风险。必须专门迁移会话 + 测试。
2. **路径 C（删 chain/merkle 冗余）经验证【非干净死代码】**：grep 确认生产代码只导入 `chain.ipfs_client`（非 merkle/verify），但 `chain.merkle.build_merkle_tree`/`verify_proof` 出现在 `traversal-events.jsonl`（step 67200/67645）作为**逢亮穿越的概念节点**（425/426号 code-as-concept 入图）。删 merkle.py = 移除逢亮概念空间节点 = 改变 import 概念图/β₁ 拓扑。**这不是 casual 死代码删除**，需评估逢亮拓扑影响。
3. meta.json 计数错位（block_count:1868 vs 文件系统 274509）= 数据一致性修复，需理解 block 存储语义。

**上浮**：方向（A 迁移）已被原则定，但执行是逢亮持久化/概念子系统的高风险专门工作。需编排者：(i) 调度专门迁移会话（含 block 存储语义专家）；(ii) 裁决 merkle 是否值得为"删冗余"承担概念图拓扑变动（或保留作概念节点）。蜂群侧已完成方向判定 + merkle 概念空间风险揭示，移 archive 等编排者调度。

**影响声明**：本次不改 src/topological-computation/（避免高风险 inline 改动）；仅记录方向 + 风险。执行待专门会话。

---

# JSONL primary vs block primary 迁移未完成——476↔178 张力持存

## 矛盾

W-4 自标 Gap 6：
- 178号声称 block topology 是 primary
- 476号承认 K_active/K_full 运行时持久化仍依赖 JSONL append-only
- 478/479/480号是补丁式应对（settlement 类型约束 + memory 泄漏堵漏）
- 张力未消——两套存储并存

附加缺口：
- meta.json `block_count: 1868`（概念层）
- 实际文件系统 274,509 个文件（物质层）
- 反向 gap：代码写了边，meta 没跟上

附加孤岛：
- chain/merkle.py + chain/verify.py 681 行实装但**主 daemon/穿越/engine 都不导入**
- 仅 chain/deploy_test.py 使用——是工程冗余

## 否定了什么

否定的是"178号 block topology primary 已落地"的声明。事实上：
- 物质层（COOCCURRENCE/TRAVERSAL_ASSOCIATION block）已迁移
- 概念层（K_active/K_full 运行时）仍 JSONL primary
- meta.json 统计未跟上
- chain/merkle 是冗余幽灵（517号同模式：CycleState/holonomy.py 不存在）

这是 008号下游推论"声明—事实落差"的多路径实例。

## 推导链

- 178号：区块拓扑建系（177条谱系一次性升格为区块）
- 347号：身份=内容（hash 全文）——content-addressed
- 425/426号：S_net 入图——双层架构
- 476号 tensions_with 178：JSONL 不兼容长期运行
- 478/479/480号：memory 泄漏堵漏（settlement 类型约束的逐层完整化）
- 519号：v3 蓝图圈6 limits.md 命名错误——本号是同源
- W-4 详查：chain/merkle 未被主穿越消费 = 178号 Q3 决断（content-addressed 独立寻址）后的剥离遗留

## 谱系链接

- 178号、347号、415号、417号、419号、421号、425号、426号、476号、478号、479号、480号（W-4 主链）
- 519号（同源命名错误）
- 016号（规则没有代码强制就不会执行——本号是反向：代码存在但无消费）
- 090号（严格性，no-patch-mentality——478/479/480 补丁式应对违反）

## 影响声明

- 影响：`scripts/block_topology.py`、`scripts/block_ops.py`、`topological-computation/block_topology_persistence.py`、`topological-computation/daemon.py`、`chain/merkle.py`、`chain/verify.py`、`.chanlun/block-topology/meta.json`、`.chanlun/block-topology/relations.jsonl`（git-lfs 指针）
- 改动方向（待编排者裁决）：
  - 路径 A：完成迁移——K_active/K_full 也走 block topology primary，废 JSONL append-only
  - 路径 B：双栈共存合法化——明确声明"概念层 JSONL primary，物质层 block primary"，更新 178号
  - 路径 C：删除 chain/merkle 冗余路径（W-4 已建议）
- 路径 A 是严格扬弃（no-patch-mentality 一致），路径 B 是务实退让（161号否定）

## 异质审计降级

本 session 全程 gemini-challenger 不可用。
