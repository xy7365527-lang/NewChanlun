# #244 MutexFinalTheorem 全域承重：Lean 全量化域 vs rust 活跃候选子域

- 日期：2026-07-25
- 执行：codex exec（gpt-5.6-sol，read-only，effort=high）
- 验收：主控 session（§二 为主控独立核对与订正，非 codex 产出）
- 票：map #59 子票 #244（自 #221 交接 C 节第 2 条，编排者 2026-07-25 加判）

## 一、核心结论（codex 原判）

问题问的是「Lean 定理域外的部分，在 rust 生产路径上是否承重」。答案分两半：

- **互斥性（两分类同时成立）：未发现漏。** 每个进入加权的 `next_idx` 都调一次总函数 `operation_role_two_segment`，只返回一个 `OperationRole`，不存在「两类同时为真」的表示路径。
- **穷尽性（动作/状态转移来源）：存在真缺口。** registry 恢复祖先是生产可达且承重的**额外来源**，既不在输入 `A_t` 的腿里，也不在 `Γ_trade` 落入 ℬ/𝒦 的候选里，却进入了 `A_{t+1}` 和目标腿集合。

即实际转移是 `AncOK[(A_t \ D) ∪ B ∪ RegistryRestore]`，而声明只写 `AncOK[(A_t \ D) ∪ B]`。

### 域的准确刻画（订正原问题的前提）

rust 侧不是单一「活跃候选域」，而是两个不同的域：

- 角色域 `Dᵣ_role = 已组装 BSP 候选 ∪ next_idx 目标腿元素`
- 动作分桶域 `Dᵣ_fold = Γ_trade ∪ A_t`

两者不能混为一个集合 —— 承重缺口正好落在「在 role 域内、不在 fold 域内」的夹层里。

另注：Lean 角色分类是旧的 `{Root,Same,Sub} × {RootDir,SameDir,SubFollow,ShortDiff}` 12 类，rust 是去根化的 `H×V×δ` 加 `G` 细化 —— 除域差外**还有分类模式差**，不能把 rust enum 的唯一性直接当成 Lean 12 类的对应证明。

## 二、验收裁定（主控）

### 新发现三的机制描述：订正

codex 称恢复元素「被角色函数按 `Ambient/SameLevel`、深度 0 计算」并引 `coverage.rs:1223`。主控独立核对：**结论对，引用的路径错**。

`coverage.rs:1215` 那个返回 `First/Ambient/Plus/SameLevel` 的分支，触发条件是 `elements.get(e_idx) == None`（**索引越界**兜底），与 parent 无关。

真实链路是两条独立的默认值合流（`coverage.rs:1279-1285`）：

1. `attached_dir = None` → `parent_sign(None) = 0` → `classify_vertical` 第一分支 → **`V = Ambient`**
2. `parent = None` → `ell_p = ell_g` → `classify_grade(ell_g, ell_g)` → **`G = SameLevel`**

结果与 codex 所述一致（`Ambient` + `SameLevel`），机制不同。

### 订正后这条**仍然成立，且更值得注意**

`restore_ancestor_chain_from_registry`（`coverage.rs:2058-2066`）为真 LiveDetached 祖先构造 `CoverageElement` 时，写死 `parent: None, attached_dir: None`，**只保留 `parent_id`**。

而源码在 `ell_p` 那一行的注释明写：

> `ponytail: 父越界/None 防御归 ℓ_g（⟹ SameLevel）；None 已被 σ_p=0（∂）截断 V=Ambient`

—— 即这是一条**标注为「防御性（不应到达）」的分支**。但 registry 恢复路径**恒定**产生 `parent=None/attached_dir=None`，于是防御分支被生产路径当作常规路径命中。

**要害**：`parent_id` 明明已知（从 `pe.structural_parent_id` 取到并存了），却没有被用于重建 `parent` 索引与 `attached_dir`，导致该元素的父子关系在角色计算中丢失，角色恒定落到 `V=Ambient / G=SameLevel`。角色进入 `dir_weight` / `w_grade` / ShortDiff 禁用 / `p̃`，因此**可改变实际下单权重**。

归属：**A 类实装缺口（语义错桶）**，不是「Lean 全域无形式保障」的推论，也不是互斥性失败。

### 承重性的边界（照实）

静态控制流只能证「可承重」。要真正产生订单差，还需：恢复链完成、AncOK 保留、权重非零、未被 gross-cap 清零、净额未被其他腿抵消。

**当前窗口是否实际发生：未能判定。** 缺指定数据窗口、χ/nest/risk/voice 配置，以及 `restore_calls`、`state_live_detached`、`restore_complete`、`sep_legs/p̃` 的运行 trace。本次未跑回测、未执行测试（含 `coverage.rs:4578` 那条固定场景的既有测试）。

### 结构性不可达项（采纳）

- 普通 Kᵢ carrier 若始终不成为候选/持仓/恢复祖先，其自身角色不进入目标腿，只作上下文
- 父 carrier 不在 registry 时不会膨胀恢复，子腿仍被 AncOK 剪除（守卫 `coverage.rs:4674`）
- χ/nest 拒绝候选本 bar 不进动作 fold，但**角色已算过** —— 不算角色穷尽性漏
- `strategy_target_legs` 对 `next_idx` 全量 map，未发现「已承担头寸却完全无角色」的路径

## 三、三条结论的分层（按票面要求分开写）

| 层 | 内容 |
|---|---|
| **已知事实**（非新发现） | Lean 对更大全域量化，rust 未对该全域给形式保障；`MutexFinalTheorem.lean:14` 源码自身已登记为待查项 |
| **新发现一** | 角色互斥性本身没有承重漏：域外元素若要承重，必先进 `next_idx` 并接受一次 rust 角色分类 |
| **新发现二** | 动作穷尽性有承重缺口：registry 恢复祖先是声明分桶域外的第三来源 |
| **新发现三**（经主控订正机制） | 真 LiveDetached 恢复元素恒定命中「不应到达」的防御分支，`parent_id` 已知却未用于重建角色输入 ⟹ 唯一但错桶，可改 `p̃` |

## 四、后续票

- **新发现二/三 → 立实装侧票**（穷尽性声明补 RegistryRestore 来源 + 恢复元素角色输入重建）；本 session 只出票不实装
- 运行时验证（`restore_calls` / `p̃` 差值 trace）待窗口与配置指定后另出 task 票

## 五、原始产物

codex 完整输出：`/tmp/codex-244-mutex-loadbearing.md`（本次会话产物，未入库）
