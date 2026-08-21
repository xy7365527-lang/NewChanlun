# 走势分解（Fenjie / Decomposition）

> 正本：本文件是 走势分解 的教义正本。
> 最后裁定：[#812](https://github.com/xy7365527-lang/NewChanlun/issues/812)（2026-08-01，zhongshu.md S-6「造唯一、读可多」）
> 受影响代码清单：本文件 §4（Rust / Lean 现役面，点名到行号）
>
> 归口成型：[#1151](https://github.com/xy7365527-lang/NewChanlun/issues/1151)（map [#1094](https://github.com/xy7365527-lang/NewChanlun/issues/1094)，2026-08-19 编排者逐问拍板）

## 1. 纲（归口纲：单入口，正文零复述）

**「造唯一、读可多」**（zhongshu.md S-6，裁定 #812）：构造只有一套（自底向上递归塔，唯一）；读法可多（同级别分解是「在已造好的塔上选一层去读」，不是重新造中枢）。第 38 课「5 分钟延伸出 6 段就当成两个 30 分钟盘整的连接」（`038-第38课.md:22`）属读法侧。

三处现役正本各管一段，本文件只挂指针、不复述：

| 口径 | 正本 | 管什么 |
|---|---|---|
| 构造口径（塔扫描） | [zhongshu.md](zhongshu.md) Z/S 全组（#812） | 中枢构造全程：seed 三段 / 中心定理一延伸 / 九段重切 / 续扫锚 C（Z-5） |
| 读法口径（同级别分解） | [ADR 0011](../docs/adr/0011-operation-decomposition-layer.md)（#827） | 口径 S 扫描：条件 seed、恒三格、禁延伸、允许盘整+盘整、唯一性不变式 |
| 语义基（「唯一」的形式化） | `formal/Origin/CanonicalQuotientTower.lean` | 商化基础层；`∃!` 是推论非公理（`canonical_representative_unique`） |

## 2. 边界条款（已裁归位表）

| 分歧 | 归位 | 裁定 |
|---|---|---|
| 构造 vs 读法（同一图形两种分解结论） | S-6「造唯一、读可多」——两句说的不是同一件事 | #812 |
| 同级别分解的名分（一等模块，横向读法，不回流主干） | ADR 0011 | #827 |
| Lean 固定三格 lift vs 生产动态窗 compose | 长期两制（「语义近邻」，勿逐字段对拍） | #240（#236 前因） |
| 续扫锚 B 口径（`i=j`）改 C 口径（跳过突破段） | zhongshu.md R8/Z-5——**已裁待落地** | #812 |
| 中枢间折叠（盘整+盘整 允许/禁止） | 构造口径禁、同级别分解口径允许（R19 归读法侧） | #812/#827 |

**未裁面**（开着的缝，不属本正本正文）：Φ_n「三段重叠」实例化（对齐 #1087 签收面，查 map #1094 图体 Not yet specified）、Unassigned 账本生产化（已立案为图外实施票 [#1160](https://github.com/xy7365527-lang/NewChanlun/issues/1160)）。

## 3. 接线行

- **#817 N-1（区间套坐标系）**：四坐标系口径以 #817 裁定为准；Rust↔Lean 对齐验收在 #1087（统一扫描装配 + 逐位对拍，签收五件事），本正本不重复造对拍。

## 4. 受影响代码清单（现役面）

| 文件 | 现役角色 |
|---|---|
| `rust/src/theta_v0/classifier/recursive_tower.rs` | 构造口径：canonical 中枢链（seed :792-794 / 延伸吸收 :795-797 / 续扫锚 `i=j` :857——**R8 待改 C** / ≥9 段重切 :821-854） |
| `rust/src/theta_v0/classifier/operation.rs` | 读法口径 S（:83-99）+ 唯一性测试锁 `operation_decomposition_uniqueness_lock` |
| `formal/Origin/CanonicalQuotientTower.lean` | 语义基：Φ_n 抽象参数、`canonical_representative_unique`(:197)、`toRecursiveLevelSystem`(:315) |

**待落地 / 在途指针**（不在清单正文）：R8 续扫锚改 C 落地后回填本清单；#1087 签收后桥侧新增一列（对齐验收在 #1087）。
