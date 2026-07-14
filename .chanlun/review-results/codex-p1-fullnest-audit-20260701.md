# Codex 异质审计：P1 FullNest N^δ 证书（2026-07-01）

> 审计者：codex-challenger（codex-cli model）
> 任务编号：#39（P1 FullNest 异质审计，约束4硬节点）
> 输入：`/tmp/codex-p1-fullnest-ctx.md` + 代码读取（econ_positive.rs / nest.rs）

---

## Codex 原始 Verdict（完整）

```
verdict: conditional

stances:
  production_wiring: accept         // econ_positive.rs:270 调用确实接通
  wiring_equals_fullnest: reject    // 接通≠每次都是多级递归
  base_case_is_zero_call: reject    // base-case 退化≠零调用
  partial_rung_break_downgrade: needs_work  // 链断 break 导致静默降级
  sister_proof_function_level_zero_call: reject  // §3.3"函数级零调用"被否定
  epistemic_L0_label: accept_with_domain_qualification  // L0 标注基本诚实但需补有效域
```

### Codex 三问判断

**问题1（生产路径接通性）**：
`econ_positive.rs:270` 调用真实接通；`signals.push` 前必过 `build_multilevel_nest_cert`。
但"接通"只证明调用了 `n_delta()`，不证明每次都是多级递归。
单层 tower → `rungs=[]` → 退化为 `Conf^δ_e`（base-case）。这是合法退化，不是"零调用"，
但**也不是 FullNest 多级执行的证据**。

**问题2（姊妹证明 §3.3 时序矛盾）**：
如果 §3.3 指"函数级零调用"，与当前代码冲突，应判为旧状态或错误陈述。
如果 §3.3 指"内容级退化（运行时 rungs 总为空）"，仍需运行统计支持，当前代码不能成立此说。
→ **§3.3 至少应更新为"当前生产链已调用 N^δ；多级实际命中率未验证"**。

**问题3（有效域诚实性）**：
L0 标注基本诚实（结构过滤，不声明 alpha）。需补一句有效域限制：
大部分 bar 若 tower 只有 1 层，则退化为 `Conf^δ_e`；
只有 `rungs.len()>0` 的信号才真正触发跨级 Cand 与 J 嵌套递归。

---

## 审计者判定

### 否定成立性分析

**Codex 三条否定：**

1. **"接通等于多级执行"（wiring_equals_fullnest: reject）**
   - 成立。函数接通是必要条件，不是充分条件。证明"生产信号经过真正多级 N^δ"需要统计 `rungs.len() >= 1` 的命中率。
   - 严重性：重要（影响 consolidated-verdict 的"P1 已修复"声明精度）

2. **"链断 break 导致静默降级"（partial_rung_break_downgrade: needs_work）**
   - `econ_positive.rs:505-511`：`for k in (lvl+1)..max_k { ... } else { break; }`——当某级无包含段，`break` 使 rung_buf 只包含部分上级层，证书被静默降为 `(lvl+1)..k` 子链，而非完整 tower 顶层链。
   - 如果 P1 要求从 tower 顶层完整下钻，这是**设计决策点**：break = 允许部分链；return None = 要求完整链。
   - 当前代码允许部分链但报告称为 "FullNest"——需澄清语义。

3. **"有效域无统计（建议级）"**
   - 无 `rungs_0` / `rungs_1_plus` 分层计数，无法知道生产信号中多级触达的实际占比。
   - 不是阻断性问题，但不符合诚实有效域声明标准（formalization-validity-domain.md）。

**Codex 否定不成立的部分：**

- **base_case_is_zero_call: reject**——姊妹证明 §3.3 的"零调用"如果指函数级，当前代码已否定。函数被接通在 econ_positive.rs:270 是代码事实。
- **production_wiring: accept**——接通成立。

### 最终判定

| 问题 | Codex 否定 | 成立？ | 理由 |
|------|-----------|--------|------|
| 生产路径接通 | accept | — | 代码事实，无争议 |
| 接通=多级执行 | reject（否定成立） | 成立 | rungs 空时退化，需统计证明多级命中率 |
| 链断 break 静默降级 | needs_work（否定成立） | 成立 | 设计歧义，partial chain vs full chain 未区分 |
| 姊妹证明函数级零调用 | reject（否定成立） | 成立——否定的否定 | §3.3 描述旧状态，需更新 |
| L0 标注 | accept_with_qualification | 基本成立 | 需补充有效域限制句 |

---

## 结果包（简化版——纯技术性产出）

### 结论
Codex verdict: **conditional**（非确证，非完全否定）。

confirmed-verdict（14:23）说"W1 区间套已修复"——在函数接通层面成立（L0 代码事实）。
但"已修复"不等于"生产信号都经过真正多级 N^δ 递归"：
- 单层 tower → base-case 退化，合法但不是 FullNest
- 链断 break → partial chain 静默降级
- 无运行时统计 → 多级命中率未知

姊妹证明 §3.3（18:41）的"零调用"如指函数级，与当前代码矛盾，判为旧状态描述需更新。
如指"内容级退化（rungs 实际总为空）"，Codex 判断需统计证据，当前代码无法自证。

### 边界条件
以下任一成立则"已修复"声明翻转（从 L0 函数接通降为 L0 部分接通+需 L2 统计）：
1. 回测标的历史数据中 tower 层数绝大多数为 1（单层）→ 信号集实际等同于旧 Conf^δ 门
2. `partial_rung_break_downgrade` 被视为 FullNest 定义的违反 → 需改为 `return None`（拒绝部分链）
3. L2 统计显示 `rungs_1_plus` 命中率极低 → 功能性等同旧 base-case

以下成立则"已修复"声明升为"经验有效"（L2）：
1. W-VERIFY 输出 `rungs.len() >= 1` 信号比例 + 信号 μ̂ 与旧门有显著差异

### 影响声明
1. **consolidated-verdict-20260701.md A节**：需补充"函数接通确认；多级命中率待 W-VERIFY 统计"
2. **hundred-percent-divergence-proof-20260701.md §3.3**：需更新"当前已调用 N^δ，函数级零调用已根除；运行时 rungs 实际分布待统计"
3. **econ_positive.rs:263-270 注释**：需补"单层 tower 时退化为 Conf^δ 基例"的诚实说明
4. **econ_positive.rs:505-511 链断逻辑**：需明确设计意图（partial chain vs full chain）并在注释中声明

---

## 两个需要 verdict-p1 工位确认的问题

Q1. `build_nest_certificate` 中的链断 `break`（econ_positive.rs:511）——是设计选择（允许 partial chain）还是遗留 bug（应 return None）？如果是设计选择，需更新 FullNest 的语义说明。

Q2. verdict-p1 工位对 Q5"语义验收不降级"的最终判定结论是什么？consolidated-verdict 宣布已修复，但姊妹证明（18:41 落盘，晚于 consolidated-verdict 14:23）仍描述旧状态——verdict-p1 是否已产出最终判定？
