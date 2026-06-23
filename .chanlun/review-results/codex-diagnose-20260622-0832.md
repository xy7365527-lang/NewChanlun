# Codex 异质对审记录：死锁 Face B（liveness）从「修 recover 欠触发」攻

- 模式：diagnose + decide
- 模型：OpenAI gpt-5.5，reasoning_effort=xhigh（`~/.codex` config 确认）
- 调用：`codex exec --model gpt-5.5 -c model_reasoning_effort=xhigh -c sandbox_mode=read-only`
- session id：019eee70-a9c8-77b1-acb2-aa4af555d5f7
- tokens used：140,334
- 触发：编排者明令实装前 Codex 最高推理审查 Face B 修复方案（task #12）
- 输入上下文：`/tmp/codex-faceb-recover-ctx.md`（Codex 自行 Read 核实 rec_engine.rs / t_engine.rs /
  docs §11.4/§11.5/§11.6/§12.3 / escalation 全文）
- 完整原始输出：`~/.claude/.../tool-results/bwk1fu1lw.txt`（3923 行，含 Codex 自主工具调用链）

---

## Codex 最终裁决（逐字，不软化）

**总结论**：不建议落 R1/R2 这类 recover 容忍补丁。Q1 判 b，应该 escalate。当前证据支持
「recover 欠触发」主要是结构/会计/信号有效域问题，不是一个同级别匹配小 bug。

### Q1 根因归类 —— 判定 **b yes / a no 倾向**

- recover 当前确是「精确同级别 j」契约：rec_engine.rs:743-751，flat 同构 t_engine.rs:698-720。
  但这只说明「level mismatch 可能产生 noop」，**不能证明它是主因**。
- 更硬的因果链：强平在 route 前执行（rec_engine.rs:793）→ Short 在 `c >= SUB_LIQ_FACTOR*basis`
  逐仓强平（:828）→ 强平后 `units<=EPS` 清零（:122）→ 后续同级买点到来时 route 只看到
  `j 无短差` → 计 buy_noop。§12.3 同样把 50% 不回补归为 type1_buy@high 稀疏 + Short 无三阶段
  保护 / 等待期被强平（recursive_t_architecture_v2.md:724）。
- **裁决**：精确同级别匹配是**症状放大器，不是已证实的根因**。R1 只能救「别级仍有活跃短差」
  的子集；对「短差已被强平」的 noop 无效。

### Q2 候选修复评估

| 候选 | 判定 | 理由 |
|---|---|---|
| R1 级别容忍 recover | **invalid / escalate** | 把 `BSP@j` 改成可平 `k!=j` 的短差，改变 recover 的 return-address 语义。数值上若限 `k<parent` 可保 `sub<parent`，但结构上破坏「该级走势完成才升回该级短差」。bit-exact 技术上可对称实现，但**语义不合法**。 |
| R2 ascend 前 recover 孤儿短差 | **invalid** | 当前 lower short **不是孤儿**；`nearest_active_parent(j)` 会动态绑定到新核心（:423）。ascend 只 relabel 核心且要求 target idle（:573）。提前平短差 = 无 BSP 的强制 recover。 |
| R3 drain 误吞 | **no** | 父多时短差方向 `mob=Short`，`j` 若是 Short 走 sink 分支不走 drain（:733）。drain 只吃「同父向遗留仓」。**衰减不是 drain 误消耗。** |
| R4 结构性 escalate | **valid** | Q1=b，正确修复落到 §11.6 reverse-promote / 信号通道 / 会计保护，不是 route_bsp 小改。 |

### Q3 strict bit-exact 落码 —— 判定 **不存在合法的非 Face-A recover 修复落码**

- 读 state 本身不破 bit-exact：现有 `highest_active`/`nearest_active_parent`/`recover` 都读仓位
  状态（:417）。**边界**：现有 state 读法是在 **BSP@j 已给定** 后找父级 / 验证 j 自己是否持短差；
  R1 是用 state 去**选择另一个 k**，等于让路径依赖仓位状态补充「结构事件发生在哪里」——
  **这不是 bit-exact 问题，是结构信号缺失问题**。
- 正确方向若要表达「反向走势涌现到核心级别」，需新增 flat+rec 共享信号通道
  （类似 escalation 已写的 `cascade_*_to[]` / ancestry），而非扫 `instances/layers`（escalation:181）。

### Q4 衰减是否真 liveness 杀手 / Face B 真实目标 —— 判定 **衰减是真杀手；目标应是核心暴露不坍塌，不是 enter 解冻**

- `is_active = units > 1e-12`（:182），1e-6 僵尸核心仍挡 `enter`。它还能 ascend（emergence_upgrade
  只看 active/方向/level，:599）、还能被 sink（`m=u/3`，1e-6 时远大于 EPS，:616）。
- 但单调牛里核心持多 active 是**正确状态**。**如果 recover 真能补齐，n_enters 不增长也没问题；
  核心一直骑牛即可。** 所以「2019+ 重新入场」应改述为「2019+ 核心有效暴露恢复/不坍塌」。
- → 编排者的「修 recover」实际指向**核心暴露不衰减**（暴露坍塌=踏空真因），不是 enter 冻结。

### Q5 与 A''（trend-done-clear）的关系 —— 判定 **A'' 已部分解 enter 冻结；与 recover 修复不干净正交，不能作第二套默认补丁并行**

- A'' 默认启用（:246/:310），route 前检测核心 type1 完成，`clear_all` 后 early return（:858），
  令 `highest_active=None` → 下一买点可 enter。flat 同样实现（t_engine.rs:835）。
- 若 Face B 定义成「enter 解冻」→ A'' 已是一个并行解；若定义成「核心暴露不衰减」→ recover 是
  intra-campaign 暴露维护，A'' 是 campaign boundary reset。两者概念不同，但执行上 A'' 会**抢在
  route/recover 前清仓**，不能无裁决并行。

### 最终建议（逐字）

不要落 R1/R2。当前应 **escalate**：Face B 从「修 recover 欠触发」攻，若只是跨级容忍 recover，
就是用仓位状态**伪造 return address**；若要真正修，需进入 §11.6 信号级联 / 短头会计保护 /
campaign 边界裁决范围。**当前最安全的代码动作只应是加诊断计数**，区分 `buy_noop` 时「别级仍有
活跃短差」还是「短差已强平/不存在」，**不改行为**。

---

## 代理简化质询（判定否定是否成立）

对 Codex 的否定逐条复核源码（问题是否真实 / 是否误读上下文）：

1. **R3 drain 断言**（Codex 判 no）：复核 route_bsp:733-742——drain 仅在 `is_reduce ∧ j active ∧
   j.direction != mob`（持同父向遗留仓）触发；短差=反父向(mob)落 sink 分支。**Codex 断言成立，
   非误读。** 衰减不是 drain 误吞。
2. **Q1 强平链**：复核 :828（Short `c>=SUB_LIQ_FACTOR*basis` 逐仓强平）+ :122（强平 units 清零）。
   链成立——短核心等待 recover 期间确实可被强平归零 → buy_noop。**成立。**
3. **Q4 衰减仍 active/可 ascend/可 sink**：复核 :182/:599/:616。1e-6 > 1e-12 恒 active；
   sink `m=u/3` @1e-6 = 3.3e-7 > EPS 可继续 sink。**成立。**
4. **Q5 A'' 已存在且 default-on**：复核 :310（`enable_trend_done_clear = T_NO_TREND_DONE_CLEAR.is_err()`
   = 默认 on）+ :858-878 early-return clear。**成立——A'' 是一条已部署的 enter 解冻路径。**

**质询结论**：Codex 的异质否定**全部成立，无误读上下文，无被现有机制覆盖的误判**。严重性分级合理。
否定方向与 escalation 已有边界条件 (b)/(c) 及 §12.3 独立收敛——这是**跨模型 + 跨文档的三重交叉确认**。

## 结果包（简化版——纯技术性对审产出）

- **结论**：「修 recover 欠触发」是**错误的攻击面**。Q1=b（结构常量，非同级别匹配 bug），R1/R2/R3
  全否（R1/R2 invalid+语义不合法，R3 不成立），Q3 无合法非 Face-A 落码，Q4 真实目标=核心暴露不
  坍塌而非 enter 解冻，Q5 A'' 已部分解 enter 冻结。**应 escalate（落 R4），不落码 R1/R2。**
- **边界条件（结论翻转）**：(a) 若 buy_noop 实测主因是「别级仍有活跃短差」而非「短差已强平」，
  则 R1 可救其子集——但 Codex/§12.3 证据指向强平主导，需诊断计数实测区分两者占比方能翻转；
  (b) 若把 recover 匹配改为读 `instances` 选 `k!=j`，则从「读 state 验证」越界到「用路径依赖
  仓位状态伪造 return address」——这是合法/不合法的精确分界线。
- **影响声明**：本对审**未改任何引擎代码 / 定义文件**。新增本记录 +（建议）escalation 追加第三轮
  对审节。触及（待裁决后）：rec_engine.rs/t_engine.rs（route_bsp recover 分支 / 强平判据 /
  短头会计保护）、信号层 `cascade_*_to[]` 通道、A'' 去留、546§9.2 闭合。唯一**当前**可落码动作 =
  buy_noop 诊断计数二分（不改行为，bit-exact 安全）。
