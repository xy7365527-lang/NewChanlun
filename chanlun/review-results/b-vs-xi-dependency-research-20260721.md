# #108 调研报告：b 级别层叠 vs ⑪ 塔内原生化（Consume_at）依赖关系

| 字段 | 值 |
|---|---|
| 票据 | issue #108（wayfinder:research，AFK）；parent map #106 |
| 日期 | 2026-07-21 |
| 性质 | 纯只读分析——零 rust 代码改动、零 cargo、零 git mutation、主仓零写入 |
| 代码基线 | worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717` |
| 结论 | **⑪ 自带级别信息，与 b 互不依赖（可并行）** |

---

## 三选一结论

**结论：⑪ 自带级别信息，互不依赖（可并行推进）。**

一句话理由：`Consume_at` 的级别归因来自塔原生 `event_level`（经谱系→`BspKey.formation_level` 链路自洽传递），而 b 修复的是下游证书查询层 `admission.rs:typed_lookup` 的级别投影断裂（固定 ℓ=lvl+1 只查一个方向）——两者在不同层操作，不存在物质依赖。

---

## 任务 1：⑪ 的 Consume_at 到底要什么——级别归因机制

### 1.1 Consume_at 的签名与输入契约

来源：`doc-divergence-endtoend-prototype-20260718.md §1/:75–:87`、`mainline-merged-roadmap-20260717.md:62/:77`、`e2e-consume-at-implementation-audit-20260719.md §A`。

```
Consume_at(as_of, prior_bsp_by_key, prior_links_by_key,
           closed_lineage, ManagedBspPolicy)
  -> (ManagedBspCreation[], BspLink[])
   | NoConsumption | InvalidPolicy | InconsistentState | LateAuthorization
```

**级别信息进入 Consume_at 的唯一通道 = `closed_lineage`**。`closed_lineage` 是 `BuildLineage_at` → `CloseFloor_at` 产出的已闭合谱系修订流，其中每条谱系携带：

- `LineageKey`：含规则版本 + 有序 `EventKey` 路径 + `EdgeKey` 路径（路线图:64）
- 每个 `EventKey` 携带 `event_level`（固定为运行该背驰谓词的塔桶/中枢级别，路线图:64）

### 1.2 级别归因机制：BspKey.formation_level

Consume_at 输出的 BSP 身份键（路线图:64 `E2E-L` 段）：

> `BspKey` 是 `(formation_level, point_class, side, source_index)` 全局点身份。

- `formation_level` = 产出该 BSP 的谱系事件的 `event_level`
- `event_level` = 塔递归构造中该事件所在 `Classification.levels` 的下标（`classifier/mod.rs:214-217`，级别 = `Vec` 下标）
- `source_index` = 原始 K 序坐标（L0 投影位置）

**关键事实：`formation_level` 是塔原生级别，直接从谱系事件继承，不经过任何外部级别映射或投影。**

### 1.3 Consume_at 的级别信息不需要多视窗投影

Consume_at 只消费 `Closed` 谱系（`e2e-consume-at-implementation-audit-20260719.md §A.2/A.4`），谱系的 `event_level` 在 `ObserveCandidate_at`/`ReviseEvent_at` 阶段就已经从塔的 `Classification` 中固定写入。这个过程：

1. 塔 `classify_impl` 递归 compose 出各级别 `LevelState`（`classifier/mod.rs:394-472`）
2. 各级别产 BSP（`classification.levels[lvl].bsp: &[BspPoint]`）
3. BSP 经 `ObserveCandidate_at` 获得稳定身份（含 `event_level`）
4. 谱系经 `BuildLineage_at`/`CloseFloor_at` 闭合
5. `Consume_at` 消费闭合谱系，产出 `ManagedBspCreation`（`BspKey.formation_level` = 谱系 `event_level`）

全程级别信息自洽传递，**不需要 b 的多视窗投影**。

---

## 任务 2：既有实装中 Consume_at / consumable_closed 的级别信息使用

### 2.1 consumable_closed：消费侧唯一门户（已携带级别）

`nest_lifecycle.rs:570-574`（裁定 #64 §2(a)「消费不放开」）：

```rust
pub fn consumable_closed(&self) -> impl Iterator<Item = &NestLifecycleEntry> {
    self.entries
        .values()
        .filter(|entry| entry.state == NestEventState::Confirmed)
}
```

返回的 `NestLifecycleEntry` 携带 `key: LifecycleKey`（`nest_lifecycle.rs:170-177`）：

```rust
pub struct LifecycleKey {
    pub level: u32,           // ← 塔原生级别（event.level）
    pub side: Side,
    pub kind: NestDivergenceKind,
    pub seg_a: (usize, usize),
    pub seg_c_full: (usize, usize),
    pub b_center_start: usize,
}
```

**`level` 来自 `NestCandidateEvent.level`**（`nest_lifecycle.rs:184-193` `LifecycleKey::from_event`），而 `NestCandidateEvent.level` 由 provider 从塔的 `Classification` 投影（`level_view.rs:660-719`）。

**级别信息已存在且正确——它从塔原生结构直传，不经过任何多视窗投影。**

### 2.2 nest.rs 的 N^δ 装配：只用 source_index，不用级别桥

`nest.rs` 的区间套证书装配（`assemble_certificate`，nest.rs:802-810/:987-989）只消费已闭合完整 `c_p`（`CandDeltaEvent`），身份键 `NestEventIdentity`（nest.rs:396-400）含 `level` 但装配本身不做跨级别查询。

### 2.3 级别桥 ℓ=lvl+1：在 admission.rs，不在 Consume_at 路径

身份桥断裂点 `admission.rs:564-568`（`typed_lookup` 查 `lvl+1`）是**证书查询层**——它在 runner 主循环里被调用（`runner.rs:1137-1468` NestChainGate），对已装配的证书做准入查询。这是：

- **Consume_at 的下游消费者**（如果 Consume_at 落地后）
- **不在 Consume_at 的输入路径上**（Consume_at 的输入是谱系，不是证书查询）

---

## 任务 3：依赖关系裁定

### 3.1 两者的操作层

| 维度 | b（级别层叠多视窗） | ⑪（Consume_at / 塔内原生化） |
|---|---|---|
| **操作层** | 证书查询/表示层（`admission.rs`） | 生产/形成层（谱系→BSP 授权事务） |
| **修复对象** | `typed_lookup` 固定 ℓ+1 → 多级别查询 | `Consume_at` 完全缺失 → 事务边界实装 |
| **级别信息来源** | 下游投影（BSP → L0 K 线投射） | 上游原生（塔 `Classification` → `event_level`） |
| **级别正确性** | 当前断裂（43.2% BSP 在 L2/L3/L4 有覆盖但桥够不到） | 自洽（`formation_level` 从谱系直传） |

### 3.2 为什么不存在物质依赖

1. **⑪ 的级别归因不经过 b 的修复路径**：Consume_at 的 `formation_level` 从谱系 `event_level` 直传，谱系从塔 `Classification` 直产。这条链路完全在塔内部，不经过 `admission.rs` 的级别桥。

2. **b 的修复不影响 ⑪ 的输入**：b 改的是 `admission.rs:typed_lookup` 的查询键构造（从固定 ℓ+1 改为多级别）。这不改变谱系事件本身的级别，也不改变 `Classification.levels` 的结构。

3. **b 的修复是 ⑪ 输出的下游消费者问题**：即使 Consume_at 落地，产出的 `ManagedBspCreation` 若被 admission 层用固定 ℓ+1 桥查询，仍会有 43.2% 的 miss。但这是**下游消费缺陷**，不是 Consume_at 形成的前置条件。

4. **互不阻塞的证据——LifecycleKey.level 已正确**：当前 `NestLifecycleBook`（E2E 生命周期的 sidecar 切片）的 `LifecycleKey.level` 已经携带正确的塔原生级别（`nest_lifecycle.rs:172`），`consumable_closed()` 返回的 Confirmed entry 的级别信息完整。这说明级别归因的基础设施在 sidecar 层已就位，不需要等 b 的多视窗投影。

### 3.3 与 ADR 的关系

ADR `adr-level-identity-multiview-20260721.md:31` 原文：

> 与 map #59 横切⑪「塔内原生化长线」**同级或为其前置**

ADR 的措辞是"同级或为其前置"，留了两个可能。本调研的结论是**同级（互不依赖，可并行）**，理由如上。

ADR 说"为其前置"的场景成立当且仅当：Consume_at 需要从 admission 层回读级别信息来做归因。但实际不是——Consume_at 的级别归因是从谱系上游直传的。

### 3.4 前置依赖（真实存在的）

Consume_at 的**真实前置依赖**是 E2E-N3–E2E-N6（路线图:79 明定）：

| 依赖 | 内容 | 当前状态 |
|---|---|---|
| E2E-N3 | Closed 谱系（`Lineage`/`LineageKey`/`LineageRevision`） | 缺失（`e2e-consume-at-implementation-audit §C3`） |
| E2E-N4 | Event↔BSP 身份边（`ProjectionKey`/`LinkKey`） | 缺失（§C4） |
| E2E-N5 | 首证钟（五钟） | 部分实装（`NestLifecycleBook` 五钟已落地） |
| E2E-N6 | 选择器 | 部分实装（`sel_order`/typed DFS） |

这些依赖与 b 无关——它们是谱系基础设施，不是级别表示。

---

## 关键代码锚

| 锚 | 用途 |
|---|---|
| `nest_lifecycle.rs:172` | `LifecycleKey.level: u32`——级别已在身份键中 |
| `nest_lifecycle.rs:184-193` | `from_event`——级别从 provider 事件直传 |
| `nest_lifecycle.rs:570-574` | `consumable_closed()`——消费门户返回携带级别的 Confirmed entry |
| `admission.rs:564-568` | `typed_lookup` 固定 `lvl+1`——**b 修复的断裂点**（Consume_at 下游） |
| `mainline-merged-roadmap-20260717.md:64` | `BspKey = (formation_level,…)`——Consume_at 级别归因的定义 |
| `e2e-consume-at-implementation-audit-20260719.md §A` | Consume_at 签名与语义冻结 |
| `classifier/mod.rs:214-217` | `Classification.levels: Vec<LevelState>`——级别 = 数组下标（塔原生） |

---

## 附：调研读取的文件清单

| 文件 | 用途 |
|---|---|
| `chanlun/review-results/e2e-consume-at-implementation-audit-20260719.md` | Consume_at 形式化定义 + 实装审计 |
| `chanlun/plans/mainline-merged-roadmap-20260717.md` | E2E-O/N7/L 定义 + 依赖序 |
| `chanlun/review-results/doc-divergence-endtoend-prototype-20260718.md` | E2E 原型 §1 Consume_at 签名/语义 |
| `rust/src/theta_v0/classifier/nest_lifecycle.rs` | NestLifecycleBook 实装（LifecycleKey/consumable_closed） |
| `rust/src/theta_v0/classifier/nest.rs` | N^δ 装配（consume 逻辑不查级别桥） |
| `chanlun/escalate/adr-level-identity-multiview-20260721.md` | b 的教义裁定 |
| `chanlun/review-results/level-mapping-fracture-research-20260721.md` | 级别映射断裂分析（b 的前置调研） |
| `chanlun/review-results/issue106-implementation-card-20260721.md` | #106 实装卡（b 的 parent issue） |

— 报告结束。零 rust 代码改动、零 cargo、零 git mutation、主仓零写入。 —
