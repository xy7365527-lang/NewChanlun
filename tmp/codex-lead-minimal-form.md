# Codex Diagnose 结果：Lead 最小形式

**模式**：diagnose
**时间**：2026-02-26
**模型**：gpt-5.2-codex（codex-5.3 不可用，自动降级）
**持久化**：`.chanlun/review-results/codex-diagnose-20260226-0546.md`

---

## 一、Codex 诊断结论

**根因**：调度与执行未分层。ceremony.md 将执行逻辑（脚本运行、会话写入、分析执行）内嵌为 Lead 行为，违反 069/090/218 的同构要求。

**分类判定**：实现错误（不改变任何定义即可修复）。无定义冲突。

---

## 二、问题1：步骤标注（调度 vs 执行）

| 步骤 | 调度逻辑（Lead 保留） | 执行逻辑（应 spawn） |
|------|----------------------|---------------------|
| 步骤1：扫描 | 触发扫描、阻塞等待结果 | 运行 ceremony_scan.py |
| 步骤2：摘要 | 输出摘要格式 | 无 |
| 步骤3：TeamCreate | 创建 team、维护唯一标识 | 无 |
| 步骤4：spawn | 遍历 JSON、判断分支、决定 spawn/route | 实际任务执行（每个工位本体）|
| 步骤4：无工位分支 | 决定终止 | session 写入/commit/push（因持久化不变量可保留）|
| 步骤5：行动声明 | 输出声明 | 无 |
| 步骤6：RTAS 循环 | 调度 TaskList、并行汇报、触发 shutdown、spawn 辅助 | 结果分析、测试执行、拓扑分析、meta-observer |
| 循环内：增量持久化 | 决定何时持久化 | session 写入（执行逻辑）|
| 循环内：git 操作 | 决定提交时机 | git 操作（因串行化不变量保留在 Lead）|
| 循环内：重扫描 | 决定是否重扫 | 运行 gangju_analysis.py / ceremony_scan.py |

---

## 三、问题2：ceremony_scan.py 作为工位的可行性

**结论**：技术上可以作为工位执行。

- 脚本是确定性扫描，不依赖 LLM 状态
- Lead 通过 TaskList/TaskResult 消费 JSON（工位将 stdout 原样写入任务输出）
- **边界条件**：JSON 原样回传不可信时（LLM 参与导致内容变形）必须保留在 Lead 内

**风险3（Codex 自识别）**：scan 作为工位后，Lead 必须等待其完成才能 spawn 业务工位——这是严格串行依赖，与直接内联 Bash 调用效果相同，只是多了一层工位包装。

---

## 四、问题3：Bash 白名单划界

| 白名单项 | 可 spawn？ | 原因 |
|---------|-----------|------|
| `python scripts/ceremony_scan.py` | ✅ 可 | 执行逻辑，可通过任务输出回传 JSON |
| `python scripts/gangju_analysis.py` | ✅ 可 | 执行逻辑，输出 JSON 用于调度 |
| `git add / git commit / git push` | ❌ 不可 | 串行化不变量，必须 Lead 单点控制 |
| `git fetch / git rebase` | ❌ 不可 | repo 状态操作，需单点控制 |
| `session 文件写入` | ⚠️ 建议不委托 | 与 commit/push 强绑定，需串行一致性 |

---

## 五、问题4：Lead 最小形式（~30 行伪代码）

**保留**：扫描 → 生成工位 → TeamCreate → spawn → TaskList 监控 → 持久化 → 重扫描 → TeamDelete
**删除**：冗长解释、Gemini/长期工位细节（移入工位或 scan 规则）
**合并**：步骤2与步骤5；步骤6与循环内操作

```pseudo
scan = run_scan()                    # 可 spawn 工位
summary(scan)
if scan.workstations.empty:
    persist_session(); git_commit_push(); exit

team = TeamCreate("v{N}-swarm")
spawn_all(scan.workstations, team)

while true:
    status = TaskList(team)
    done = collect_done(status)
    if done:
        update_session(done)
    if any_in_progress(status):
        spawn_aux_analysis()         # 拓扑/observer/测试等
        continue

    persist_session(); git_commit_push()

    gangju = run_gangju()            # 可 spawn 工位
    scan = run_scan()                # 可 spawn 工位
    if gangju.audit_needed or scan.workstations:
        spawn_all(scan.workstations, team)
        continue

    TeamDelete(team)
    break
```

---

## 六、问题5：风险分析

| 风险 | 现象 | 修复方案 |
|------|------|---------|
| 工位间协调丢失 | 工位 A 输出不能驱动工位 B | scan 输出中加入依赖字段，Lead 基于依赖调度 spawn |
| 持久化不变量被破坏 | 退出路径未触发 session+commit+push | session 写入 + git 操作由 Lead 单点执行，工位只返回增量变更 |
| scan 作为工位导致同步失效 | Lead 在 scan 输出到达前就 spawn 业务工位 | TaskList 等待 scan 完成并解析 JSON 后再继续 |
| 是否引入新串行依赖 | — | **不会**：`ceremony_scan 输出 → spawn` 已是严格串行操作 |

---

## 七、我的判定

### 否定成立的部分

Codex 的核心诊断成立：

1. **调度/执行分层**——ceremony.md 确实混合了两种逻辑，这是实现错误
2. **git 操作不可委托**——持久化不变量的核心，判定正确
3. **30行伪代码结构**——合理，保留了调度骨架，剥离了执行细节

### 需要质疑的部分

**ceremony_scan.py 工位化的实际价值存疑**：

Codex 说 scan 可以 spawn 为工位，但同时在风险3中承认：spawn 后 Lead 仍必须等待其完成（严格串行依赖）。这意味着：

- 工位化 scan = 把一个 Bash 调用变成 Task spawn + 等待
- 串行依赖不变，只是执行环境变了
- 实际上增加了一层间接性，未必减少 Lead 的"胖"

**结论**：scan 工位化是形式上的同构，不是实质上的减负。真正的减负来自 RTAS 循环内的执行逻辑（拓扑分析、meta-observer、测试验证）的工位化——这些是真正可以并行、不阻塞 Lead 的执行逻辑。

### 边界条件

如果 scan 工位化的目的是"形式同构"（069号），则成立。
如果目的是"减少 Lead 的执行负担"，则对 scan 本身效果有限（串行依赖不变）。

---

## 八、影响声明

- 涉及文件：`.claude/commands/ceremony.md`
- 不涉及定义文件修改
- 修复路径：ceremony.md 重写（调度/执行分层）
- 持久化：`.chanlun/review-results/codex-diagnose-20260226-0546.md`
