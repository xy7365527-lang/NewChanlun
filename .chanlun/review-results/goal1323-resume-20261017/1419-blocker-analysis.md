# #1419 阻塞器分析与修复路径

日期：2026-10-17。基于工作树 `.sandcastle/worktrees/resume-1419` 的工程审查报告。

## 核心阻塞器（2 个）

### HIGH-1：新域生成 C/D/E/F 后，发布前来源审计仍只接受 A/B

**症状**：`source_audit.rs:1700-1709` 的 typed kind 白名单只有 CC004/005/007/054 与 `tb02b::KINDS`。新域 `generation.rs:1034-1075` 无条件纳入 C/D/E/F，至少三种 CC056 版本根即使没有完整段/中心也会产生。首代真实发布在 `prepared.rs:906` 调 `source_audit::audit_generated` → `source_audit.rs:2231` 的 `audit_object_record` → 必以"未声明typed事实种类"拒绝。

**根因**：来源审计白名单未同步新结构域的 C/D/E/F kinds。

**受影响文件**（精确位置）：
- `rust/src/s_session_v3/source_audit.rs:1700-1709`（白名单）
- `rust/src/s_session_v3/generation.rs:1034-1075`（无条件生成 C/D/E/F）
- `rust/src/s_session_v3/prepared.rs:906`（发布前调用审计）

**修复路径**：
1. **扩展白名单**：在 `source_audit.rs:1700-1709` 添加：
   - C/D：`tb02c.rs:59` 的 7 个 KINDS（头：`s-segment/3` / `CONSTRUCTION_RULE` / `seed_closed_strict_second`）
   - E：`tb02e.rs:18` 的 7 个 KINDS（头：`s-center/1` / `center-seed/1` / `strict_seed_zn_outer_c_resume`）
   - F：`tb02f.rs:15` 的 5 个 KINDS（头：`s-center-pair/1` / `center-pair/1` / `zn_outer_five_leaf_pending_ge9`）
   - view_role 均 main
2. **保持审计严格性**：核 payload.slot 与规范 fact_key，保持 source_references → raw 原件、source_coords 相等、obj3 身份校验
3. **不要放宽**：不以 starts_with 放宽所有未来对象，也不要重跑 parser 取代原件审计

**估算工作量**：4-6 小时（白名单扩展 + 19 个 kinds 精确校验 + 单测）

---

### HIGH-2：新结构分类请求与既有二元槽位审计不兼容（已静态修复，待运行验证）

**症状（原始）**：`content_structure.rs` 的 `classification_request` 使用五元键 `[structure-classification/1, cc, axis, subject, path]`，但 `source_pending.rs:615` 要求规范二元数组，五元键会报"知识请求键不是规范二元槽位"。

**当前状态**：
- ✅ 静态缺陷已消除（SHA256 `1b3001a12ffd822700a14475b6b90650707242301cd329ca50deb53f2da9bda3` 保持规范二元键）
- ❌ 仍有 key 外部值边渲染阻断（`content_producer::explicit_request` 调用 `canonical_text(&key)` 时新建私有 MemoryNodes，外部 store 的 slot 节点未在此 scratch 中，真实请求键含 Edge 时会 MissingNode）

**修复路径**：
1. `content_producer.rs:2983` 的 `canonical_text` 应使用传入真实 store 规范渲染含边 key，或使用真正自足的小值
2. 不得把该故障误报为二元协议仍不通过

**估算工作量**：2-3 小时（Edge 渲染修复 + 验证）

---

## 次要问题（2 个）

### MEDIUM-1：历史 A/B oracle 比较固定 13 轴，生成端按 V2 规则选择 17 轴

**症状**：`restrict_oracle_to_tb02ab`（generation.rs:3663-3710）只去 C/D 对象并固定重算 NewBi 13 轴，但 `generate_draft`（:3798）仍把 oracle 的 RULE_REVISION 传给生成器，`finish_axes` 因而选 SegmentSecond 17 轴。`oracle_prefix_parity_two_prefixes`（:4176）等完整状态逐字段比较会出现 13/17 轴不同。

**修复路径**：明确 A/B oracle 投影的逻辑域及身份映射，不能改生产目录回固定 13 轴，也不要删除对拍或把不一致当预期。

**估算工作量**：3-4 小时（oracle 投影逻辑 + 测试修复）

---

### 既有 HEAD 编译阻断（非本次 diff 新引入）

**症状**：`source_audit.rs:1766` 和 `:1791` 引用 `rule`，但所在 `audit_object_record_with_slots`（:1644）的形参、局部作用域均未定义该变量。

**修复路径**：把真实冻结 rule 沿 full/native 两条同核调用链传递，不能固定成新域。

**估算工作量**：1-2 小时（变量传递 + 编译验证）

---

## 修复优先级与总工作量

| 优先级 | 问题 | 工作量 | 前置依赖 |
|--------|------|--------|----------|
| P0 | 既有 HEAD 编译阻断 | 1-2 h | 无 |
| P1 | HIGH-1（白名单扩展） | 4-6 h | P0 |
| P2 | HIGH-2（Edge 渲染） | 2-3 h | P0 |
| P3 | MEDIUM-1（oracle 对拍） | 3-4 h | P1, P2 |

**总工作量**：10-15 小时

**估算完成时间**：2-3 天（假设单人全职，串行修复）

---

## 修复路径建议

### Phase 1：解除编译阻断（1-2 小时）
1. 修复 `source_audit.rs:1766` / `:1791` 的 `rule` 未定义
2. 验证 `cargo build --release` 通过

### Phase 2：扩展来源审计白名单（4-6 小时）
1. 读取 `tb02c.rs:59` / `tb02e.rs:18` / `tb02f.rs:15` 的完整 KINDS 列表
2. 在 `source_audit.rs:1700-1709` 添加 19 个 kinds 的精确校验
3. 保持 payload.slot / fact_key / source_coords / obj3 审计严格性
4. 添加单测覆盖 C/D/E/F 各 1 个 kind 的发布前审计通过场景

### Phase 3：修复 Edge 渲染（2-3 小时）
1. 修改 `content_producer.rs:2983` 的 `canonical_text` 使用传入真实 store
2. 验证含 Edge 的请求键不再 MissingNode

### Phase 4：修复 oracle 对拍（3-4 小时）
1. 明确 A/B oracle 投影的 rule_revision 应为 NewBi
2. 修改 `restrict_oracle_to_tb02ab` 冻结 rule_revision 字段
3. 验证 `oracle_prefix_parity_two_prefixes` 不再报 13/17 轴不同

---

## 验收标准

1. ✅ `cargo build --release` 通过（无编译错误）
2. ✅ `cargo test --release s_session_v3::generation` 通过（含新增 C/D/E/F 白名单测试）
3. ✅ 新域 C/D/E/F 对象成功通过发布前来源审计
4. ✅ 含 Edge 的请求键成功渲染（无 MissingNode）
5. ✅ A/B oracle 对拍不再报 13/17 轴不同
6. ✅ 恢复树（`.sandcastle/worktrees/resume-1419`）完整编译与测试通过

---

## 下游依赖

#1419 解锁后，可以推进：
- TB-02-S（#1488）：B2 实施依赖来源审计通过
- TB-02-V（#1405）：生产级验收依赖发布链完整
- #1304 Phase 1：回放引擎依赖 C/D/E/F 对象可信

---

**产出工位**：Opus 5.5（主控）  
**绑定票**：#1419（TB-02-E）  
**文档性质**：工作草稿（与工程审查报告绑定，不作为独立正本）
