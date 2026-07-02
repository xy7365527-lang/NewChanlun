# Codex P2 R2 实装独立异质审计

**工位**：#34 p2-audit（codex-challenger，异质代理，约束4硬节点）
**日期**：2026-07-01
**被审对象**：p2-impl #32 的 R2 实装
**依据**：
- p2-impl 代码：signal.rs / interp.rs / divergence.rs / bsp.rs / econ_positive.rs
- p2-review #33 报告（独立核，不背书）
- codex-p2-plan-review-20260701.md（七护栏上下文）

---

## Codex 原始输出（逐字）

```
基于你给出的片段审查；本地工作区为空，codebase-memory MCP 调用被取消，所以不能
独立跑全仓定位。

- 问题：`struct_break_dir` 同时承担"结构破坏方向元数据"和"零 bit 候选可恢复为
  可交易方向"的权限。`candidate_dir` 只检查 `!conf_plus() && !conf_minus()`，
  不检查零 bit 的原因是否确为 C>=A MACD 预删。因此只要未来某条构造路径给零 bit
  点填了 `struct_break_dir=Some(_)`，就会被提升为 Long/Short 并进入 μ 样本。
  这不是当前片段必然 bug，但恢复资格没有被类型或字段显式表达，安全性依赖所有
  构造点的外部约束。
  位置：interp.rs:candidate_dir；signal.rs:judge_first_cached
  严重性：重要
  修复建议：把字段拆开，保留 struct_break_dir 作为结构方向元数据，另加显式恢复
  资格，例如 `dir_recovery: Option<RecoveryReason::MacdCGeA>` 或在构造处只对确认
  为 C>=A 的候选设置 `recover_dir_from_struct_break=true`。`candidate_dir` 应检查
  该资格，而不是仅凭 struct_break_dir。

- 问题：GOLDEN 更新不能单独作为 bit-exact 证据。若 GOLDEN 是基于 derive(Debug)
  字符串，新增 struct_break_dir 后哈希变化确实可以解释为 Debug 扩展，但这个
  oracle 已经混入非六 bit 字段。它只能证明快照变化被接受，不能证明
  MuClass/class_index bit-exact。
  位置：GOLDEN fixture / Debug-hash 测试
  严重性：建议
  修复建议：保留 Debug GOLDEN 可以，但另建只序列化 BspBits/class_index()/MuClass
  六维 key 的 bit-exact GOLDEN。现有逐案 extract_signals_bit_exact_vs_orig_per_case
  是必要的，建议让它成为主验收，而不是 Debug hash。

Q1：candidate_dir 的严格零 bit 判断能防住 (1,1) 双触发被 struct_break_dir 误改
向；struct_break_dir 不进 class_index/MuClass 这一点在设计上成立，前提是
class_index() 真的只读六 bit。

Q2：方向恢复到 μ 采样的链路在片段层面闭合：assemble_gamma_with_tower →
candidate_dir → 非 Flat → 不被 Flat 过滤 → signals.push。声明必须收窄为
"几何门通过且 A/C 可配对候选的 MACD C>=A 预删偏差消除"，不能说是 Γ_struct
全量或 canonical 力度层完成。

Q3：接受分层验收，但只能条件通过。R2 可以闭合"结构方向恢复/消 MACD 预删偏差"；
WeakTheta/ForceState 支配序未接通不应阻塞 R2。反过来，任何声称"buy1 已按
canonical 力度判据完成选择偏差消除"的表述都应拒绝，必须落到 #42。

---stance-declaration---
verdict: conditional
stances:
  struct_break_dir_recovery_gate: needs_work
  debug_golden_as_bit_exact_oracle: needs_work
  double_trigger_guard: accept
  mu_sampling_connection: accept
  force_layer_defer_scope: accept_with_narrow_claim
---end-stance---
```

---

## 审计代理判定

### Q1-A：严格零 bit 防 (1,1) 双触发 — **否定不成立（Codex 接受，与 p2-review 一致）**

Codex 接受：`!conf_plus() && !conf_minus()` 严格排除 (1,1) 双触发（conf_plus=true），只
匹配真零 bit。candidate_dir 实现正确，p2-review G1 PASS 得到独立确认。

### Q1-B：struct_break_dir 不进 class_index — **否定条件成立（设计上正确，测试守护已在位）**

Codex 条件接受（"设计上成立，前提是 class_index() 真的只读六 bit"）。

**核验**：class_index() 只读 level/delta/i_class/parent_dir/short_swing/position 六维，
struct_break_dir 在 BspPoint 外层（不在 BspBits/MuClass 序列化路径）。per-case 测试
`struct_break_dir_recovers_direction_without_touching_class_index` 验证逐案 class_index
不变。设计正确，测试守护在位。

**判定**：否定不成立（条件已满足）。

### Q1-C：GOLDEN 更新作为 bit-exact oracle — **否定成立（Codex 发现有效缺陷，已被 per-case 覆盖）**

Codex 指出：Debug hash GOLDEN（0x37d2→0x56ed）混入非六 bit 字段，不能单独作为
class_index bit-exact 证据。

**质询**：否定是否成立？——**成立**。但 p2-review G4 已处理：六 bit 不变由
`extract_signals_bit_exact_vs_orig_per_case` 逐案验证（主验收），GOLDEN 只是快照辅助
（方案A）。p2-review 注释明确"六 bit 值本身不变通过 per-case 逐案验证"。

**判定**：Codex 否定成立但已被 p2-review 覆盖。非新返工项，Codex 建议（per-case 作主
验收）与当前实装设计一致。

### Q2：选择偏差消除接通链 — **否定不成立，链路闭合，但声明须收窄**

Codex 接受：`assemble_gamma_with_tower` → `candidate_dir` → non-Flat → `signals.push` 链路
闭合。零 bit + struct_break_dir=Some(方向) → candidate_dir 返回 Long/Short → 不被
`if c.dir == Flat continue` 过滤 → 进 μ 样本。

声明收窄要求（Codex 明确）：
- 只消除"几何门通过且 A/C 可配对候选的 MACD C≥A 预删偏差"
- 不声明 Γ_struct 全量进样本
- 不声明 canonical 力度判据（支配序）完成

p2-review G6 已收窄声明（L1 认识论标注）。**判定：接通链成立，声明已收窄，无返工项。**

### Q3：struct_break_dir 双职责/恢复资格无显式 gate — **否定部分成立（防御性建议，非当前 bug）**

这是 Codex 独立发现、p2-review 未覆盖的新质疑。

**Codex 质疑**：struct_break_dir 同时承担元数据和恢复资格，恢复 safety 依赖所有构造点
的外部约束（非类型系统保证）。

**质询结论**：
- 当前片段不存在 bug。`judge_first_cached` 是唯一第一类 BspPoint 构造路径，对所有破中
  枢候选无条件置 struct_break_dir=Some（C<A 有 buy1 六 bit，candidate_dir 走 root_sel 不受
  struct_break_dir 影响；C≥A 零 bit，靠 struct_break_dir 恢复方向）。
- B2/S2 构造路径（signal.rs:412/430）明确 struct_break_dir: None，无野路径。
- 未来风险：新增构造路径若误置 Some 会引入误恢复。这是防御性担忧，非当前 R2 实装错误。

**判定**：否定成立（有效防御性建议），但**不阻塞 R2 crystallize**。记录为未来改进候选
（可在 #42 或独立 task 中用 newtype/恢复资格字段强化类型安全性）。

### Q4（scope 裁定）：力度层 defer #42 分层验收合法性 — **Codex 接受分层验收**

Codex 明确：R2 可以闭合"结构方向恢复/消 MACD 预删偏差"；WeakTheta/ForceState 支配序
不接通不应阻塞 R2。禁止声称"buy1 已按 canonical 力度判据完成选择偏差消除"。

**认识论核心**（Codex 判定等价于）：
- 方向恢复消除的是"MACD C≥A 预删"这个**结构方向选择偏差**（几何 gate 通过候选的样本扩展）
- 力度判据（支配序）是 #42 的工作，两者是不同层的消除目标
- 两者可分层验收——前者是 R2 的责任范畴，后者是 #42 的责任范畴

**判定**：scope 裁定接受，力度 defer 合法。p2-review G3/G7b HONEST GAP 声明合法。

---

## 综合 verdict

**verdict: conditional-pass（R2 可 crystallize，含一个防御性建议记录）**

| 维度 | 判定 | 阻塞 R2？ |
|------|------|---------|
| 双触发 (1,1) 防护 | 否定不成立（Codex 接受） | 无 |
| struct_break_dir 不进 class_index | 否定不成立（条件已满足） | 无 |
| GOLDEN Debug hash 弱性 | 否定成立但已被 per-case 覆盖 | 无 |
| μ 采样接通链 | 否定不成立（Codex 接受链路闭合） | 无 |
| struct_break_dir 双职责（防御性） | 否定部分成立（防御性建议） | **否**（非 bug） |
| 声明收窄 | Codex 要求已满足（G6 已收窄） | 无 |
| scope 裁定 | Codex 接受分层验收 | 无 |

**不阻塞 R2 crystallize 的理由**：
1. 所有硬约束（bit-exact 隔离 / 接通链 / 双触发防护）Codex 接受或否定已被覆盖
2. struct_break_dir 双职责是防御性建议（当前无 bug），future task 处理
3. 力度层 defer #42 获 Codex 明确接受

**需随 #42 处理的 Codex 建议（非 R2 阻塞）**：
- struct_break_dir 字段分离（newtype 或显式恢复资格 flag），增强类型安全
- per-case 六 bit GOLDEN 作主验收（已是，建议保持）

---

## 边界条件

以下条件满足时本裁定翻转：
1. 发现 B2/S2/其他 BspPoint 构造路径给零 bit 点填了 struct_break_dir=Some → struct_break_dir
   双职责 bug 实化 → G1/bit-exact 护栏可能失效 → 需返工
2. class_index() 被发现读了 struct_break_dir（六 bit 之外的字段）→ bucket key 改变 → 硬约束
   失守 → 需返工
3. 有证据 buy1 bit 必须代表 canonical Weak_Θ 支配序背驰（非 MACD-area proxy）→ scope 裁定
   需重新审 → 力度不接通 = R2 未完成

---

## 影响声明

- 落盘：`.chanlun/review-results/codex-p2-impl-audit-20260701.md`（本文件）
- 未改任何 src/
- 下游：#35 P2-crystallize 可推进（R2 conditional-pass），建议携带 Codex 防御性建议记录
