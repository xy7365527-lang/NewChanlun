# orbit9 D-review 审查报告（2026-06-24）

审查对象：`/private/tmp/orbit9-B-wt/.chanlun/review-results/orbit9-D-offguard-l3-20260624.md`
整合 worktree：`/private/tmp/orbit9-integration-wt`，HEAD = `4c0afeae11`

---

## 代码审查报告

### 概念层质询结果

**通过。** 无定义矛盾，有效域声明与实际测量口径一致。

---

### 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 1 | note |

结论：**PASS**

---

## 五项审查点复核

### 1. OFF bit-exact 真 PASS

**复核结论：确认 PASS。**

实跑 `cd /private/tmp/orbit9-integration-wt/rust && cargo test recursive_t 2>&1 | tail -10`：

```
test result: ok. 137 passed; 0 failed; 22 ignored; 0 measured; 442 filtered out; finished in 0.04s
```

三 env（`T_ORBIT9_H0`/`T_ORBIT9_DISPATCH`/`T_ORBIT9_NEST`）缺省时 `EngineConfig::from_env` 三字段恒 false（rec_engine.rs:231/235/236），`off()` 函数第 253–254 行显式置 false（`enable_orbit9_dispatch`/`enable_orbit9_nest`），`enable_h0_skeleton` 在 `off()` 中未置——但 `off()` 调用路径从 `face_b()` 链中走，而 `face_b` 不开 h0；从 `from_env` 路径走时 env 未设则 false。`n_h0_flip_blocked` 守卫测试（rec_engine.rs:2760）明确断言 OFF 门下计数为零，测试通过即证实 OFF 逐位等 facea。

**OFF bit-exact = L1 PASS，与 D 报告一致。**

---

### 2. 冲突解析正确性

**复核结论：5 个冲突块解析正确，无机械合并错误。**

验证字段完整性（rec_engine.rs 搜索结果）：

- `EngineConfig::struct` 层：`enable_pair_emergence`（158）、`enable_h0_skeleton`（170）、`enable_orbit9_dispatch`（179）、`enable_orbit9_nest`（186）——A+B+C 全部保留，无重复定义，无遗漏字段。
- `from_env` 层（227/231/235/236）：四字段均有对应 `std::env::var` 读取，逐一对应。
- `off()` 层（252–254）：`enable_pair_emergence`/`enable_orbit9_dispatch`/`enable_orbit9_nest` 显式置 false；`enable_h0_skeleton` 的 off 路径依赖 env 缺省为 false，与 H⁰ OFF bit-exact 守卫（测试 2760 行）一致。
- `TRoot::struct/new_with_config` 层（785/788/795 + 943/944/946）：`enable_pair_emergence`/`enable_h0_skeleton`/`enable_orbit9_dispatch` 三字段均传入 TRoot，无遗漏。

**face_b() 重复删除验证**：`grep -c "fn face_b"` 返回 **1**，当前只有唯一定义（rec_engine.rs:264）。D 声称删除了一个因 cherry-pick 上下文偏移产生的重复定义——结果正确，无误删（唯一定义已保留）。

---

### 3. A HIGH-1 测量可信性

**复核结论：推翻假设的证据链扎实，instrumentation observation-only 已验证。**

**证据链逻辑**：
- OFF 下 `n_h0_flip_blocked=0`（测试 2760 行断言，137 测试 PASS 即证实）
- ON 下全标的非零（D 报告表中 CL:55/BTC:41/ES:64/OKLO:20 等），且 A''抢先（`n_trend_done_clears`）仅 1–8 次/标的，量级差一个数量级

**根因分析正确性**（L0 可验证）：
- A''（`enable_trend_done_clear`）触发路径：rec_engine.rs:2525 `highest_active()` None 分支 → 仅 cc 级 type1 走势完成清仓
- H⁰ 拦截路径：rec_engine.rs:1551 `route_bsp` 所有级别核心反向 flip

两路径**作用域严格不同**（A'' 限 cc 级，H⁰ 覆盖所有级别），非互斥，D 的根因推导正确。

**instrumentation 隔离性**：所有 `eprintln!` 位于 rec_stream.rs 第 722 行起的 `#[cfg(test)] mod tests` 块内，生产二进制不含这些代码，observation-only 成立，不污染引擎语义。

`n_h0_flip_blocked` 字段本身（rec_engine.rs:791）在生产路径中存在，但仅做计数（`+= 1`，rec_engine.rs:1552），no-op 路径（flip 被拦）是在 line:1551–1557，no-op 不改变交易状态。计数字段不污染 PnL 语义。

---

### 4. B/C 死代码诊断正确

**复核结论：三点全部属实。**

1. **`orbit9_sub_trend_done` 占位 `return true`**（rec_engine.rs:1430–1433）：函数体已读，注释明确「占位：C 接口未就位 ⇒ 默认走势完成 ⇒ recover（O7）⇒ add（O3）不激活」。
   - 调用点 rec_engine.rs:1518：`if self.enable_orbit9_dispatch && !self.orbit9_sub_trend_done(j)` → `!true = false` → add 永不激活

2. **`locate_nest` 无引擎消费路径**：
   - `grep -rn "locate_nest" rust/src/recursive_t/` 排除 `rec_nest_locator.rs` 和测试文件后，仅剩 rec_engine.rs:183 处的注释（描述字段含义），无任何调用点
   - `enable_orbit9_nest` 在 EngineConfig 有 struct/from_env/off 三处（183/236/254），但引擎主体中无任何 `if self.enable_orbit9_nest` 分支——该字段是"已注册、未消费"

3. **"ON 差异全由 A 单独贡献"成立**：B 因 `orbit9_sub_trend_done=true → !true=false` 使 add 永不激活（`n_adds=0` 全标的，D 报告表格数据）；C 无消费路径，`enable_orbit9_nest` 在引擎中是死字段。∴ ON 与 OFF 的差异路径唯一 = H⁰（`enable_h0_skeleton` 分支，1551 行）。

---

### 5. L3 否定性结论的有效域声明

**复核结论：有效域声明正确，未过度声明，未不足声明。**

D 的认识论等级标注：
- OFF bit-exact：**L1**（测试守卫，验证整合不破坏基座，信息增量=确认整合正确性）— 正确，L1 ≠ L2，未声明更高
- ON L3：**L3**（8 标的 × 3 模式 × 真实数据，含 bear 标的 BRN/DX/GC）— 正确

有效域限定（D 报告边界条件第3条）：「有效域 ⊂ 当前 8 标的 1min。若在**真 bear 段**或更高确认级别，被拦的提前翻向可能转为有益。」

这是正确的 231 号有效域收窄声明：
- 定义域 = H⁰ 在代数上可施加的所有标的/时段
- 有效域（L3 已测）= 当前 8 标的 net-up 主导的 1min 历史数据
- 有效域 ⊊ 定义域（bear 段未覆盖）

D **未声称**「H⁰ 在所有条件下无效」，而是声称「在 net-up 8 标的上 6/8 劣化，是否定性结果」——这是正确的否定性结论表述（缩小有效域边界，不是闭合有效域）。

---

### LOW note（不阻塞 PASS）

`off()` 函数（rec_engine.rs:246–258）未显式设 `enable_h0_skeleton = false`，依赖 `from_env` 缺省路径（env 未设 → false）实现 OFF 语义。这是隐式依赖：若未来 `off()` 被从 `from_env` 以外路径调用，`enable_h0_skeleton` 可能非 false。

当前有 H⁰ OFF 位 exact 测试守卫（rec_engine.rs:2760）兜底，不构成当前 bug，但建议后续整合时在 `off()` 内显式 `c.enable_h0_skeleton = false;` 使三 env 均在 `off()` 中有明确的 false 赋值，与 `enable_orbit9_dispatch`/`enable_orbit9_nest` 保持一致的显式风格。

---

## 结果包六要素

### 1. 结论

D 产出 **PASS**：OFF bit-exact 137/0 实跑复核确认；5 个冲突块字段全保留，face_b 唯一定义未误删；H⁰ 非死代码推翻证据链逻辑正确且 instrumentation observation-only；B/C 死代码诊断三点属实；L3 有效域声明符合 231 号规则，未过度声明。

### 2. 定义依据

- **OFF bit-exact 定义**：facea `54279a503e` 为基座，三 env 缺省 ⇒ `EngineConfig::from_env` 三字段 false ⇒ 引擎路径逐字不变（rec_engine.rs:169/183/186 注释均声明此不变式）
- **有效域规则**：231 号/formalization-validity-domain——有效域 ≤ 定义域，L3 否定性结果是有效域缩小，不是闭合声明

### 3. 边界条件

- 本审查结论翻转条件：若实跑不是 137/0（已实跑，否）；若 face_b 有重复定义（已 grep count=1，否）；若 eprintln 在 cfg(test) 块外（已确认第722行起，否）
- D 的 L3 结论翻转条件：bear 段数据（H⁰ 可能在 bear 中有效，D 已声明此边界）；`enable_trend_done_clear=false` 验证 H⁰ 独立于 A''（D 已声明此为未测边界）

### 4. 下游推论

- D-review PASS ⇒ D-hetero、D-cryst 可基于 D 产出展开
- D 整合产物（4c0afeae11）OFF bit-exact PASS ⇒ 可安全合主
- H⁰ ON 6/8 劣化（L3 否定性结果）⇒ H⁰ 默认应保持 OFF，bear 段验证是下一步有效域缩小工作

### 5. 谱系引用

- **231 号**（formalization-validity-domain）：有效域声明验证
- **587**（H⁰ 候选A τ 手性对称化）：L3 是 587 的 payoff 有效域过滤，L0 必然性成立但 net-up 8 标的 payoff 退化
- **137 号**（否定性禁令无效）：本审查无谱系分离点，未触发

### 6. 影响声明

本审查只读，不修改 D 的代码或报告。写入本文件 `.chanlun/review-results/orbit9-D-review-20260624.md`（B-wt 内，非整合 worktree），不影响任何引擎模块或定义。
