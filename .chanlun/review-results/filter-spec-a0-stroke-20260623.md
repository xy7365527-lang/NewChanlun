---
trigger: "signal_resolution_1s_bi_a0 roadmap task — filter-spec 工位"
target: "a0=Stroke 信号层分辨率升级规格分析"
mode: "spec-analysis"
result: "E1 实验设计完成，认识论等级厘清，前置依赖确认"
date: "2026-06-23"
depends_on: ["531", "547", "556", "557"]
---

# filter-spec 分析报告
## 信号层分辨率升级规格分析：1s 数据 + a0=笔

**日期**：2026-06-23
**任务**：规格分析，不做实装

---

## 一、认识论背景（来自谱系）

### 当前状态定位

| 维度 | 现状 | 谱系依据 |
|------|------|---------|
| 数据粒度 | 1min bar | stream.rs 默认 |
| a0 来源 | Segment（线段，confirmed && Settled） | A0Source::Segment（默认） |
| 递归深度 | 约 5-6 级别（`tree_trend_stats` 诊断） | rec_stream.rs 诊断字段 |
| a0=Stroke 参数化 | 已实装（A0Source::Stroke 枚举值 + build_a0_fast 分支） | types.rs:72 / stream.rs:111 |
| a0=Stroke 是否回测过 | 未见 L2/L3 结果 | types.rs:69「filter-spec 下游推论1」注释 |

---

## 二、a0=笔 vs a0=线段 的认识论等级分析

### 531号否证的覆盖范围（关键厘清）

**结论：531号不覆盖 a0=笔升级。**

理由：531号的控制实验只改了 bar 粒度（30min→1min），a0 类型保持不变（线段）。两个升级维度正交：

| 实验 | bar 粒度 | a0 来源 | 认识论等级 |
|------|---------|---------|-----------|
| 531号控制实验 | 30min→1min | Segment→Segment | L2（控制实验方向稳健） |
| 本提案（a0=笔） | 1min→1min | Segment→Stroke | **L0（代数推理，未跑）** |
| 本提案（1s+a0=笔）| 1min→1s | Segment→Stroke | **L0（纯代数，未跑）** |

### 形式化命题 P_bi

「a0 从线段切换为笔，在同 bar 粒度下，使递归深度增加 1-2 级，带来可测的捕获率提升」

- **L0 部分**：递归深度增加是代数必然（多一个确认层）。信息增量 = 零。
- **L2 部分**：捕获率提升是经验命题，需要真实数据否证。**目前未跑，认识论等级 L0**。

---

## 三、1s 笔下的僵尸核心死锁分析

**结论：1s 粒度在 547号 cascade 判据未修复时会加剧死锁。**

| 死锁维度 | 1min 现状 | 1s 加剧机制 |
|---------|---------|------------|
| cascade 次级别触发翻空（547号） | j=1，牛段 −70% | 1s 产生更多 L0/L1 笔和走势；触发次数线性增加 |
| 僵尸核心死锁（546/552号） | sink 衰减至 EPS | 信号更密集 → sink 更频繁 → 更早陷入僵尸 |

**前置依赖**：547号 cascade 修复（`e_level≥cc` → `e_level==最高活跃走势级别`）是 1s 实验可解读的必要前提。

---

## 四、否证条件设计

### 实验 E1：a0 切换实验（1min bar，仅改 a0）

| 维度 | 控制 | 变化 |
|------|------|------|
| bar 粒度 | 1min（固定） | 不变 |
| a0 来源 | Segment（基线） | → Stroke（唯一变量） |
| PerfectionMode | Structural（OFF 配置，固定） | 不变 |
| 标的/时段 | BTC + CL（先跑已知体） | 不变 |

**否证条件**：BTC P1 final_nav < +664%（基线退化）→ 命题否证

**为什么先跑 1min 不急于 1s**：
1. 1min a0=笔 是最小可否证单位
2. 531号已证 1min→1s（同 Segment）放大残余熵（L2 方向稳健）
3. 两升级正交，应分步测

### 实验 E2（E1 成立后）：1s bar + a0=Segment

531号逻辑延伸，与 E1 正交独立。

---

## 五、工程可行性

**a0 已完全参数化，可立即切换。**

- `types.rs:72`：`A0Source { Segment, Stroke }` 已定义
- `rec_stream.rs:117`：`new_with_a0(mode, A0Source::Stroke)` 已实装
- `stream.rs:111-130`：Stroke 分支已实装
- backtest runner 传参：**需确认**，估计 5-10 行新增

547号 cascade 修复位置：`rec_engine.rs::cascade_reverse_to_core` 判据改为 `e_level == 最高活跃走势级别`

---

## 六、结论（结果包六要素）

1. **结论**：a0=Stroke 代码就绪（L0），捕获率提升未测（L2 待否证）；E1 实验是最小可执行单位；547号 cascade 修复是 1s 实验的前置依赖
2. **定义依据**：第65课 aₙ=f(aₙ₋₁)；531号（bar粒度维度）；547号（cascade死锁）；556/557号（顶层稀疏 regime）
3. **边界条件**：E1 BTC P1 < +664% → 否证；cascade 修复后信噪比提升才可测 1s
4. **下游推论**：E1 否证 → 1s+笔组合失去依据；E1 成立 → E2 才有意义
5. **谱系引用**：531/547/552/556/557号；MEMORY `project_1min_resolution_irreducible`
6. **影响声明**：本报告为规格分析文档，未改动代码

---

## 七、代码证据摘要

```
types.rs:72-78      A0Source 枚举，Stroke 分支已定义
stream.rs:111-130   A0Source::Stroke 分支已实装（confirmed 笔过滤）
rec_stream.rs:117   new_with_a0(mode, a0_source) 入口已实装
rec_engine.rs:47    MAX_LEVEL=8（与 a0 无关）
```

547号 cascade 修复位置（待实装）：`rec_engine.rs::cascade_reverse_to_core / core_flip`
