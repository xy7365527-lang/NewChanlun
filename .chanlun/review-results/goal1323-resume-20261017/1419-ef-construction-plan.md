# #1419 E/F Construction 问题识别与修复方案

**日期**：2026-10-17  
**工位**：/root/continue_1419_ef_construction (Claude Opus 5.5)  
**绑定票**：#1419（TB-02-E 中枢 seed/核心/外缘/成员）  
**文档性质**：工作草稿（验收条件未完成，不关闭 #1419）

---

## 一、当前状态核实

### 1.1 工作分支

- **当前分支**：`codex/1419-resume-20261017`
- **最新 commit**：`38aeca9d46`（Phase 2: 扩展来源审计白名单）
- **前一 commit**：`018c6542e5`（merge: integrate #1405/#1420）
- **基线**：`121949ee99`（v3 合流）

### 1.2 Tracked 文件修改（7 个）

```
M AGENTS.md
M CLAUDE.md
D rust/src/recursive_t/backtest.rs           # 已删除 644 行
D rust/src/recursive_t/backtest_run.rs       # 已删除 774 行
M rust/src/recursive_t/center.rs             # 注释掉 backtest 导入
M rust/src/recursive_t/rec_stream.rs         # 注释掉 backtest 导入
M rust/src/recursive_t/t_engine_run.rs       # 注释掉 backtest 导入
```

**说明**：这 7 个文件是 recursive_t 清理工作（Destination ③），属于另一条并行线，**不属于 #1419 E/F construction 范围**。

### 1.3 #1419 最新实施状态

根据 issue comment（2026-10-17 19:16）：

> 续接既有认领，2026-10-17恢复核查：旧 `/private/tmp/goal1323-native-20261012` 已不存在，旧未提交修复与原始证据尚未证明恢复。保存的合流提交 `018c6542e5` 仍在，已恢复到独立持久工作树及 `codex/1419-resume-20261017` 分支，不覆盖旧分支。

> 实查基线 `121949ee99959cf728408faa6b8478d9abc27935` 的 [CI 38854239929](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38854239929) 为 completed/failure，包含16项 `s_structure_session` 编译错误和缺失真实fixture，不能描述为仅既有lib失败。合流HEAD仍需自己的检查；不挪用基线结果。

**关键信息**：
- **基线 CI 失败**：16 项编译错误 + 缺失真实 fixture
- **工作树位置**：`.sandcastle/worktrees/resume-1419`（需核实是否存在）
- **已知缺陷**：codec 接口、布尔编码、来源审计白名单、Edge 渲染

---

## 二、问题识别（按优先级排序）

### 2.1 P0：既有 HEAD 编译阻断（非本次 diff 新引入）

**症状**：基线 `121949ee99` 的 CI 失败，包含 16 项 `s_structure_session` 编译错误。

**根因**：
1. `source_audit.rs:1766` 和 `:1791` 引用 `rule`，但形参和局部作用域未定义该变量
2. 其他 13-15 项未明确的编译错误

**受影响文件**：
- `rust/src/bin/s_session_v3/source_audit.rs:1766` / `:1791`
- 其他 13-15 个未识别的编译错误点

**验证方法**：
```bash
cd /Users/silencehan/Projects/NewChanlun
git checkout 121949ee99
cargo build --release 2>&1 | tee ci-121949-compiler-errors.txt
```

**修复路径**：
1. 把真实冻结 rule 沿 full/native 两条同核调用链传递
2. 修复其他 13-15 项编译错误

**估算工作量**：3-5 小时（需先复现全部 16 项错误）

---

### 2.2 P1：新域生成 C/D/E/F 后，发布前来源审计仍只接受 A/B（HIGH-1）

**症状**：`source_audit.rs:1700-1709` 的 typed kind 白名单只有 CC004/005/007/054 与 `tb02b::KINDS`。新域 `generation.rs:1034-1075` 无条件纳入 C/D/E/F，首代真实发布在 `prepared.rs:906` 调 `source_audit::audit_generated` 时会以"未声明typed事实种类"拒绝。

**根因**：来源审计白名单未同步新结构域的 C/D/E/F kinds。

**受影响文件**：
- `rust/src/bin/s_session_v3/source_audit.rs:1700-1709`（白名单）
- `rust/src/bin/s_session_v3/generation.rs:1034-1075`（无条件生成 C/D/E/F）
- `rust/src/bin/s_session_v3/prepared.rs:906`（发布前调用审计）

**修复路径**：
1. **扩展白名单**（commit `38aeca9d46` **已部分完成**）：
   - ✅ C/D：`tb02c.rs:59` 的 7 个 KINDS（已添加）
   - ✅ E：`tb02e.rs:18` 的 7 个 KINDS（已添加）
   - ✅ F：`tb02f.rs:15` 的 5 个 KINDS（已添加）
2. **保持审计严格性**：核 payload.slot 与规范 fact_key，保持 source_references → raw 原件、source_coords 相等、obj3 身份校验
3. **运行验证**：确认新域对象成功通过发布前来源审计

**当前状态**：
- ✅ 静态修改已完成（commit `38aeca9d46`）
- ❌ **尚未编译验证**（主控禁止 cargo）
- ❌ **尚未运行验证**（需要真实 S3/Q 输入）

**估算剩余工作量**：1-2 小时（编译验证 + 真实输入运行验证）

---

### 2.3 P2：新结构分类请求与既有二元槽位审计不兼容（HIGH-2）

**症状（原始）**：`content_structure.rs` 的 `classification_request` 使用五元键 `[structure-classification/1, cc, axis, subject, path]`，但 `source_pending.rs:615` 要求规范二元数组，五元键会报"知识请求键不是规范二元槽位"。

**当前状态**（根据 `1419-codec-fix.md`）：
- ✅ 静态缺陷已消除（codec 接口已修复，SHA256 `1b3001a1...` 保持规范二元键）
- ❌ **仍有 key 外部值边渲染阻断**：
  - `content_producer::explicit_request` 调用 `canonical_text(&key)` 时新建私有 MemoryNodes
  - 外部 store 的 slot 节点未在此 scratch 中
  - 真实请求键含 Edge 时会 MissingNode

**受影响文件**：
- `rust/src/bin/s_session_v3/content_structure.rs`（已修复 codec 接口）
- `rust/src/bin/s_session_v3/content_producer.rs:2983`（Edge 渲染阻断）

**修复路径**：
1. `content_producer.rs:2983` 的 `canonical_text` 应使用传入真实 store 规范渲染含边 key
2. 或使用真正自足的小值，不依赖外部 store 的 Edge 节点

**估算工作量**：2-3 小时（Edge 渲染修复 + 验证）

---

### 2.4 P3：历史 A/B oracle 比较固定 13 轴，生成端按 V2 规则选择 17 轴（MEDIUM-1）

**症状**：`restrict_oracle_to_tb02ab`（generation.rs:3663-3710）只去 C/D 对象并固定重算 NewBi 13 轴，但 `generate_draft`（:3798）仍把 oracle 的 RULE_REVISION 传给生成器，`finish_axes` 因而选 SegmentSecond 17 轴。`oracle_prefix_parity_two_prefixes`（:4176）等完整状态逐字段比较会出现 13/17 轴不同。

**根因**：A/B oracle 投影的逻辑域及身份映射未明确，13 轴与 17 轴混用。

**受影响文件**：
- `rust/src/bin/s_session_v3/generation.rs:3663-3710`（`restrict_oracle_to_tb02ab`）
- `rust/src/bin/s_session_v3/generation.rs:3798`（`generate_draft`）
- `rust/src/bin/s_session_v3/generation.rs:4176`（`oracle_prefix_parity_two_prefixes`）

**修复路径**：
1. 明确 A/B oracle 投影的 rule_revision 应为 NewBi
2. 修改 `restrict_oracle_to_tb02ab` 冻结 rule_revision 字段
3. 不能改生产目录回固定 13 轴，也不要删除对拍或把不一致当预期

**估算工作量**：3-4 小时（oracle 投影逻辑 + 测试修复）

---

### 2.5 未明确的"第二种游标因果未闭合"问题

**来源**：checkpoint 提到"识别第二种游标因果未闭合问题"，但 issue comment 和审计文档均未明确该问题。

**可能含义**：
1. **猜测 1**：`SecondFoldStore::new(round_budget())` 后恢复同一份游标簿，消除预算 u32 当 store 的问题（已在 `1419-codec-fix.md` 修复）
2. **猜测 2**：`restrict_oracle_to_tb02ab` 的第二特征序列候选始终没等到分型（已在 issue comment 更正为 E1 吸收态，与第二种候选无关）

**当前判断**：此问题在现有文档中**不存在**或**已被更正**。若真实存在，需要用户或 #1424 独审报告明确。

---

## 三、修复方案（按优先级排序）

### 3.1 Phase 1：解除编译阻断（P0，3-5 小时）

**目标**：修复基线 `121949ee99` 的 16 项编译错误。

**步骤**：
1. 切换到基线：`git checkout 121949ee99`
2. 复现全部编译错误：`cargo build --release 2>&1 | tee ci-121949-errors.txt`
3. 修复 `source_audit.rs:1766` / `:1791` 的 `rule` 未定义
4. 修复其他 13-15 项编译错误（需逐个识别）
5. 验证编译通过：`cargo build --release`
6. 提交修复：`git commit -m "fix: resolve 16 compilation errors in 121949ee99 baseline"`

**风险**：主控禁止 cargo，需要用户授权或在隔离工位执行。

---

### 3.2 Phase 2：验证白名单扩展（P1，1-2 小时）

**目标**：验证 commit `38aeca9d46` 的白名单扩展能让 C/D/E/F 对象通过发布前审计。

**步骤**：
1. 确认 Phase 1 完成（编译通过）
2. 切换到 `codex/1419-resume-20261017`：`git checkout codex/1419-resume-20261017`
3. 编译验证：`cargo build --release`（期望 0 errors，允许 warnings）
4. 运行单测：`cargo test --release s_session_v3::source_audit`
5. **需要真实输入**：如无现成 fixture，标记为"待运行验证"

**验收标准**：
- ✅ 编译通过（0 errors）
- ✅ 白名单测试通过（含 C/D/E/F 各 1 个 kind）
- ⏸️ 真实 S3/Q 输入验证（需 #1406 C6a CI 通过或独立 fixture）

---

### 3.3 Phase 3：修复 Edge 渲染（P2，2-3 小时）

**目标**：让含 Edge 的请求键不再 MissingNode。

**步骤**：
1. 定位 `content_producer.rs:2983` 的 `canonical_text` 调用点
2. 分析 `classification_request` 的实际 key 结构（是否真的含 Edge）
3. 修改 `canonical_text` 使用传入真实 store，或使用自足小值
4. 添加单测：含 Edge 的请求键成功渲染
5. 提交修复：`git commit -m "fix(#1419): resolve Edge rendering in classification_request"`

**风险**：如果请求键实际不含 Edge，则此修复不需要。需先运行验证确认。

---

### 3.4 Phase 4：修复 oracle 对拍（P3，3-4 小时）

**目标**：让 A/B oracle 对拍不再报 13/17 轴不同。

**步骤**：
1. 明确 `restrict_oracle_to_tb02ab` 的 rule_revision 应冻结为 NewBi
2. 修改 `generation.rs:3663-3710`，冻结 oracle 的 rule_revision 字段
3. 验证 `oracle_prefix_parity_two_prefixes` 不再报轴数不同
4. 提交修复：`git commit -m "fix(#1419): freeze oracle rule_revision to NewBi in restrict_oracle_to_tb02ab"`

---

## 四、风险评估

### 4.1 主控禁令冲突

**风险**：主控明确禁止 cargo/重型验证，但 Phase 1-3 均需要编译和运行测试。

**缓解措施**：
1. 在隔离工位（`.sandcastle/worktrees/resume-1419`）执行编译和测试
2. 或派发子代理（Claude Fable 5.1 或 Opus）专门处理编译验证
3. 或请求用户临时解除禁令

---

### 4.2 未明确的"第二种游标因果未闭合"问题

**风险**：如果此问题真实存在且未在现有文档中识别，Phase 1-4 可能无法完全解锁 #1419。

**缓解措施**：
1. 完成 Phase 1-4 后，运行真实 S3/Q 输入，观察是否有新错误
2. 如发现新问题，回到本文档补充 Phase 5

---

### 4.3 CI 失败的其他 13-15 项编译错误未明确

**风险**：Phase 1 可能耗时超过 5 小时，甚至发现更多未知缺陷。

**缓解措施**：
1. 先读取 `.chanlun/review-results/goal1323-resume-20261017/ci-121949-compiler-errors.txt`（如存在）
2. 如不存在，派发子代理执行 Phase 1 的步骤 1-2，复现全部错误
3. 主控根据错误清单决定修复路径

---

## 五、验收标准

### 5.1 最小验收标准（必须全过）

1. ✅ `cargo build --release` 通过（0 errors，允许 warnings）
2. ✅ `cargo test --release s_session_v3::source_audit` 通过
3. ✅ 新域 C/D/E/F 对象的白名单测试通过
4. ✅ 含 Edge 的请求键（如存在）成功渲染，无 MissingNode
5. ✅ A/B oracle 对拍不再报 13/17 轴不同

### 5.2 完整验收标准（#1419 关票门）

根据 issue body，#1419 的完整验收条件：

- [ ] 三个连续次级单元的真实良构输入到 strict seed 的桥有见证，正式核心非退化
- [ ] 核心和全部动态外缘同时保留；后续成员变化只改其适用角色，连接与未归属尾部不偷吞
- [ ] 通过正式会话入口运行本片非真空输入；读取公共查询/变化并在浏览器核对新对象、变化和证据
- [ ] 以独立定义/裁定工作例或场所夹具独立账簿为 oracle；同输入新进程重放和所改持久边界恢复对拍

**说明**：Phase 1-4 只解除编译和白名单阻塞，**不满足完整验收标准**。真实 S3/Q 输入、浏览器核对、恢复对拍需要 #1406 C6a CI 通过或独立 fixture。

---

## 六、总工作量与时间估算

| Phase | 问题 | 工作量 | 前置依赖 |
|-------|------|--------|----------|
| Phase 1 | P0：编译阻断 | 3-5 h | 无 |
| Phase 2 | P1：白名单验证 | 1-2 h | Phase 1 |
| Phase 3 | P2：Edge 渲染 | 2-3 h | Phase 1 |
| Phase 4 | P3：oracle 对拍 | 3-4 h | Phase 1, Phase 2 |

**总工作量**：9-14 小时

**估算完成时间**：
- 单人串行：2-3 天
- 多子代理并行（Phase 2-4 可并行）：1-2 天

---

## 七、下一步行动

### 7.1 立即执行（需用户确认）

1. **确认主控禁令范围**：
   - 问题：Phase 1-4 均需 cargo 编译和测试，是否允许在隔离工位执行？
   - 方案 A：派发子代理在独立工位执行
   - 方案 B：临时解除禁令，主控执行
   - 方案 C：手动执行 Phase 1-4 的非编译部分（静态审查）

2. **核实"第二种游标因果未闭合"问题**：
   - 问题：现有文档未明确该问题，是否真实存在？
   - 来源：#1424 独审报告或用户补充说明

### 7.2 派发子代理（推荐）

如果用户批准子代理执行 cargo 编译：

```bash
# 子代理 A：Phase 1（编译阻断）
# 子代理 B：Phase 2（白名单验证）
# 子代理 C：Phase 3（Edge 渲染）
# 子代理 D：Phase 4（oracle 对拍）
```

主控等待 A 完成后，并行启动 B/C/D。

---

## 八、产出声明

- **文档性质**：工作草稿（与 #1419 绑定，不作为独立正本）
- **验收状态**：方案已识别，修复未执行，验收条件未完成
- **不关闭票**：#1419 / #1323 / #1279
- **不声称完成**：Phase 1-4 只是编译和白名单解锁，真实 S3/Q 验收需要 #1406 或独立 fixture
