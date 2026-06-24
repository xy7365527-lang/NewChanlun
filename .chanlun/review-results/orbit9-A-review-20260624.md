# 代码审查报告：H⁰ 方向骨架（task#39 / orbit9-A-h0skeleton）

审查工位：orbit9-A-review（异工位，约束3）
被审工位：orbit9-A-h0skeleton
审查文件：`rust/src/recursive_t/rec_engine.rs`（173 行新增）
日期：2026-06-24

---

## 概念层质询结果

**通过——无矛盾**。

六要素验核：

1. **结论（实装）**：`enable_h0_skeleton`（env `T_ORBIT9_H0`）门控 flip confirmed 门。OFF 时 `flip_confirmed` 参数被忽略，flip 逐字不变 bit-exact。ON 时仅 `t1buy/t1sell`（type1=走势完成）触发 flip，非 type1 反向 ⇒ no-op + `n_h0_flip_blocked++`。

2. **定义依据**：
   - 587 候选A：τ 手性对称化（short_pnl -93% 根因 = 空头加仓/平仓不对称）
   - 541 / P2 时序约束：τ flip 仅在 φ=0 走势完成点合法
   - exhaustive #39：τ 完全对称裁定（9=9 全对称）
   - `t1buy/t1sell` = 走势完成确认信号（judge_divergence 创新高确认），与 flat `TSignalView` bit-exact 对称

3. **边界条件**：
   - `enable_trend_done_clear=true`（A'' 走势完成清仓块）时，type1 反向走 A'' 路径不走 route_bsp flip——H⁰ confirmed 门不被触发。测试中已用 `cfg.enable_trend_done_clear = false` 隔离，隔离原因有注释。**生产路径需确认哪条路径优先**（A'' vs flip），若 A'' 优先则 H⁰ 门实际触达率降低。

4. **下游推论**：
   - B 工位（orbit9-B-dispatch）接口：`route_bsp` 签名增加 `flip_confirmed: bool` 参数。B 工位若直接调用 route_bsp 需适配此参数。
   - D 工位（L3 验收）需在 `T_ORBIT9_H0` ON 状态下跑八标的，验证 OFF bit-exact 和 ON 收益变化。

5. **谱系引用**：587 号（τ手性对称化）、541号（P2 时序约束）、#39（exhaustive 分类）

6. **影响声明**：`EngineConfig`、`TRoot` struct 各增 1 字段；`route_bsp` 签名改变（新增参数）；on_bar 调用点传入 `view.t1buy/t1sell`。

---

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 1 | warning |
| MEDIUM | 1 | info |
| LOW | 0 | pass |

---

### HIGH-1：生产路径下 A'' 与 H⁰ flip 门的触达关系未验证

**位置**：`rec_engine.rs` flip 分支 + `enable_trend_done_clear` 路径。

**问题**：测试 `h0_skeleton_tests` 通过关闭 `enable_trend_done_clear` 隔离 flip 路径，注释说明 "否则核心方向 type1 反向会被 A'' 拦走清仓（不走 flip）"。

这意味着在生产配置（`enable_trend_done_clear=true`）下，type1 反向触发的是 A'' 清仓路径，**不走 route_bsp flip**，H⁰ 的 confirmed 门可能 **从未被实际触发**（flip 分支只剩非 type1 反向，而非 type1 反向被 H⁰ 门全拦）。

两种情形：
- A：生产中 A'' 优先，flip 分支仅收非 type1 反向 → H⁰ ON 等价于"禁止所有 flip" → 有效域退化，不是 587 候选A 的意图。
- B：生产中 flip 分支收 type1 反向（A'' OFF 或 A'' 路径不覆盖此情形）→ H⁰ 有效。

**翻转条件**：若生产路径 `enable_trend_done_clear=false`，或 A'' 与 flip 路径不重叠，则此问题不成立。

**要求**：需说明生产配置下 A'' 与 H⁰ flip 的路径优先关系，或补一条 `enable_trend_done_clear=true` 的集成测试。

---

### MEDIUM-1：`route_bsp` 签名变更对 B 工位的接口约束未在 review-results 中声明

**位置**：`rec_engine.rs:1399`：`fn route_bsp(&mut self, j: usize, is_buy: bool, flip_confirmed: bool, node: TrendNode, c: f64)`

**问题**：B 工位（9 轨道分派）若扩展 route_bsp 或在其基础上构建 orbit9 dispatch 路径，需处理 `flip_confirmed` 参数。A 工位自审报告未声明此接口约定对下游工位的约束，可能导致 B 工位对参数语义理解分歧（flip_confirmed 是 type1 信号还是更宽泛的 confirmed 定义？）。

**翻转条件**：若 B 工位不调用 route_bsp 而是在更高层封装，则此问题不成立。

---

## 审查要点逐项核验

| 要点 | 结论 |
|------|------|
| ① 从分类派生（587候选A τ手性骨架）非 ad-hoc | PASS：confirmed 门来自 587候选A + 541 P2 时序约束，不是临时逻辑 |
| ② OFF bit-exact（env门控，不设env逐位=base） | PASS：`std::env::var("T_ORBIT9_H0").is_ok()` 缺省 OFF，flip 逐字不变；测试③验证 |
| ③ 禁扁平门/禁单触发器（#34否证教训） | PASS：门在 route_bsp 内部，不是 on_bar 顶层扁平 if；confirmed 信号由调用点传入，不是 route_bsp 内部读全局状态 |
| ④ 多空τ对称（空头加仓=卖回镜像） | PASS（L0 群论）：同一 flip 分支无方向条件分支，t1buy ↔ t1sell 镜像；测试④验证；**但见 HIGH-1**：生产路径触达率存疑 |
| ⑤ 接口清晰（与B分派/C定位分离） | PASS：flip_confirmed 由调用点注入，route_bsp 不感知 B/C 逻辑；C 任务的 `is_sub_trend_done` 作为独立接口预留（fallback=recover）；**但见 MEDIUM-1**：接口约束未向下游声明 |
| ⑥ payoff 不进定义域 | PASS：confirmed 门在 flip 触发时机层（操作语义），不修改 sink/recover/clear_all 账本逻辑；τ 对称账本中性注释明确 |

---

## 结论

**WARNING**（可合并，但 HIGH-1 须在 D 工位 L3 验收前澄清）

- 核心逻辑正确，τ 对称、OFF bit-exact、接口隔离均符合要求
- HIGH-1（A'' 与 H⁰ flip 路径优先关系）是验收前必须澄清的疑点——若 A'' 优先则 H⁰ ON 下 flip 实际等于全禁，与 587 候选A 意图不符
- MEDIUM-1（接口约束声明缺失）建议在 B 工位 review 前补齐

---

*认识论等级：本报告结论 = L0（代码结构审查，无 L2/L3 回测数据）*
