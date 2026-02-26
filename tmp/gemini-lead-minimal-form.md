# Lead 最小形式——严格讨论结果

模式：discuss（严格讨论）
日期：2026-02-26

---

## 核心命题

Lead 的当前形式是"胖节点"——违反 069号递归拓扑原则（Lead 应与工位同构）。
严格的 Lead = 纯调度器，所有实质工作 spawn 为工位。

**编排者补充约束（结晶≠记忆）**：
当前 ceremony.md 是一份长文档，Lead 靠"记住"文档内容来执行——这是记忆依赖，不是结晶。
真正的结晶 = Lead 的形式本身就是最小的自举形式，不需要记忆就能运作。
就像 Claude Code 的 30 行 bash，它的形式就是它的行为。

这个约束比"缩短文档"更严格：ceremony.md 必须达到"形式即行为"的程度——
Lead 读到文档，不需要"理解"和"记忆"，只需要"执行"。
任何需要 Lead 做判断、做解释、做推理的内容 = 记忆依赖 = 不是结晶。

---

## 1. Lead 最小形式伪代码（~25行）

```
# Lead = 感知 → 创建 → 调度 → 消费 → 持久化 → 重扫描 → 清理

state = ceremony_scan()                    # 感知器（唯一保留在 Lead 内的读操作）

if state.workstations is empty:
    session_write("干净终止")
    commit_push()
    exit()                                 # 020号反转：干净终止

TeamCreate(team_name="v{N}-swarm")         # bootstrap 行为（不可委托）

spawn_all_parallel(state.workstations)     # 调度（业务工位 + 结构工位全部在列表内）

consume_all()                              # 消费所有工位结果

session_write(results)
commit_push()                              # 持久化不变量（不可委托）

state2 = ceremony_scan()                   # 重扫描
if state2.workstations:
    spawn_all_parallel(state2.workstations)
    consume_all()
    session_write()
    commit_push()

TeamDelete()                               # 清理（不可委托）
```

**形式即行为的检验**：这段伪代码中没有任何需要 Lead "理解"的内容。
每一行都是直接执行的操作，不依赖 Lead 对背景知识的记忆。
ceremony.md 的目标就是让这段伪代码成为文档本身。

关键变化：
- 无 RTAS 循环内的"Lead 不空转"逻辑（那些操作已在 workstations 列表内）
- 无 gangju_analysis.py 直接调用（移入 ceremony_scan.py 输出）
- 无 meta-observer 直接 spawn（移入 workstations 列表）
- 无 topology-analyst 直接 spawn（移入 workstations 列表）
- 无决策逻辑（audit_needed 判断移入 ceremony_scan.py）

---

## 2. 可 spawn 为工位的职责

| 当前 Lead 行为 | 迁移目标 | 迁移方式 |
|--------------|---------|---------|
| `gangju_analysis.py` 调用 | gangju-analyst 工位 | ceremony_scan.py 输出中包含 |
| meta-observer 二阶观察 | meta-observer 工位 | ceremony_scan.py 输出中包含 |
| 拓扑分析家 spawn（179号） | topology-analyst 工位 | ceremony_scan.py 输出中包含 |
| audit_needed → spawn 审计 | audit 工位 | ceremony_scan.py 决策后输出 |
| 增量持久化（工位完成时写 session） | 各工位自行写入 | 工位完成时调用 session 写入 |

---

## 3. 不可委托的职责（创世 Gap 最小残余）

| 职责 | 不可委托的理由 |
|------|--------------|
| `ceremony_scan.py` 调用 | 感知器——Lead 需要扫描结果才能知道 spawn 什么。严格数据依赖：scan → spawn。不是 LLM 工作，是确定性 Python |
| `TeamCreate` | Bootstrap 行为——只有 Lead 能创建团队。这是创世 Gap 的物质形态 |
| `Task spawn`（调度行为本身） | Lead 的本质定义——调度器的调度行为不可外包 |
| `git commit/push` | 持久化不变量——Lead 的根本职责。git 操作有严格顺序依赖（add→commit→push），且失败时需 Lead 处理（rebase 后重推） |
| `TeamDelete` | 清理——与 TeamCreate 对称，属于 bootstrap/teardown 层 |

**069号 Sinthome 的最小化**：以上5项是创世 Gap 的不可消除残余。不是 bug，是支撑点。
尝试消灭它们（比如把 TeamCreate 也 spawn 为工位）= 尝试消灭 Swarm₀ 的奇点性 = 违反 069号边界条件1。

---

## 4. ceremony_scan.py：感知器，不是工位

**结论**：ceremony_scan.py 保留为 Lead 的感知器，不 spawn 为工位。

**理由**：
- Lead 需要扫描结果才能知道 spawn 什么——严格数据依赖（scan → spawn）
- 把 scan 变成工位 = 先 spawn scan 工位 → 等待 → 再 spawn 实际工位 = 增加一层串行开销，无收益
- scan 是确定性 Python，不是 LLM 工作——不需要 agent 能力

**但 ceremony_scan.py 需要扩展**：

当前 ceremony_scan.py 只输出业务工位（从谱系推导）。
扩展后，它应该输出**所有工位**（业务 + 结构）：

```json
{
  "workstations": [
    {"name": "笔-线段-分析", "type": "business", ...},
    {"name": "gangju-analyst", "type": "structural", ...},
    {"name": "meta-observer", "type": "structural", ...},
    {"name": "topology-analyst", "type": "structural", ...},
    {"name": "audit-workstation", "type": "structural", "condition": "audit_needed=true"}
  ]
}
```

这样 Lead 只需要 `spawn_all_parallel(state.workstations)`，不需要任何决策逻辑。

---

## 5. 决策逻辑放在哪里

**结论**：所有决策逻辑移入 ceremony_scan.py（Python 脚本）。

**依据**：057号原则——"LLM 不是状态机，确定性逻辑由 Python 脚本执行"。

**结晶约束的强化**：决策逻辑留在 ceremony.md = Lead 需要"记住"决策规则才能执行 = 记忆依赖。
决策逻辑移入 ceremony_scan.py = 决策结果已编码在 JSON 输出中 = Lead 只需执行，不需要记忆。

当前 Lead 内的决策逻辑：
- `gangju audit_needed: true → spawn 审计工位` → 移入 ceremony_scan.py
- `ceremony_scan 发现新工位 → 回到步骤4` → 变为 Lead 的重扫描循环（保留，但简化为纯调度）
- `workstations 为空 → 干净终止` → 保留在 Lead（这是调度逻辑，不是业务决策）

Lead 内保留的"决策"只有：
- `if workstations is empty → 干净终止`（调度逻辑，不是业务决策）
- `if new_state.workstations → 再次 spawn`（同上）

---

## 6. ceremony.md 重写方案

### 重写原则

**结晶约束**：ceremony.md 的每一行必须是可直接执行的操作，不允许出现需要 Lead 推理或记忆的内容。
具体删除：
1. 所有"Lead 并行化原则"节（工程建议，需要 Lead 记住并应用）
2. "严格串行的操作"节（规则说明，需要 Lead 记住）
3. RTAS 循环内的"Lead 不空转"逻辑（需要 Lead 判断何时执行）
4. gangju_analysis.py 直接调用（移入 ceremony_scan.py）
5. 所有解释性段落（"为什么这样做"的内容 = 记忆依赖）

保留：步骤结构（scan → TeamCreate → spawn → consume → persist → re-scan → TeamDelete）

### 重写后结构（~40行，形式即行为）

```markdown
# /ceremony — Swarm₀：递归蜂群的第0层

## 步骤 1：扫描（感知器）
python scripts/ceremony_scan.py
输出 JSON：mode / workstations（业务+结构）/ required_skills / session

## 步骤 2：摘要
[ceremony] {mode} | 定义 {definitions} 条 | 谱系 {settled}/{pending} | HEAD {head}
[ceremony] 工位 {N} 个（业务+结构）

## 步骤 3：TeamCreate
TeamCreate(team_name="v{N}-swarm")

## 步骤 4：并行 spawn 所有工位
spawn_all_parallel(state.workstations)
（workstations 列表由 ceremony_scan.py 决定，Lead 不做额外决策）

如果 workstations 为空：session_write("干净终止") + commit_push() → 退出

## 步骤 5：消费结果
consume_all()

## 步骤 6：持久化
session_write(results) + commit_push()

## 步骤 7：重扫描
python scripts/ceremony_scan.py
如果有新工位 → 回到步骤 4
如果无新工位 → 步骤 8

## 步骤 8：清理
TeamDelete()

## 白名单（仅以下 Bash 调用合法）
- python scripts/ceremony_scan.py
- git add / git commit / git push
- git fetch / git rebase（push 失败恢复）
```

**白名单从 5 项缩减为 3 类**（gangju_analysis.py 移入 ceremony_scan.py 内部调用）。

---

## 边界条件

1. ceremony_scan.py 扩展（输出结构工位）是本方案的前提——如果 scan 不扩展，Lead 仍需自己决策 gangju/meta-observer/topology
2. 增量持久化（工位完成时写 session）的责任转移到各工位——需要工位 prompt 中明确要求
3. Stop hook 拦截逻辑（"每次 Stop hook 拦截时，执行步骤 1"）保留——这是 Lead 的 bootstrap 行为，不可委托

---

## 下游推论

1. **ceremony_scan.py 需要扩展**：增加结构工位推导逻辑（gangju、meta-observer、topology-analyst、audit）
2. **ceremony.md 可缩减到 ~40 行**：删除所有决策逻辑、并行化原则、解释性段落
3. **工位 prompt 需要包含 session 增量写入指令**：因为增量持久化责任转移
4. **白名单从 5 项缩减为 3 类**：gangju_analysis.py 不再是 Lead 的直接调用
5. **结晶约束的检验标准**：ceremony.md 中任何需要 Lead "理解"才能执行的内容 = 未结晶 = 需要删除或移入 ceremony_scan.py

---

## 谱系引用

- 069号：递归拓扑异步自指——Lead 必须在拓扑内，创世 Gap 是不可消除的奇点
- 057号：LLM 不是状态机——决策逻辑移入 ceremony_scan.py
- 058号：ceremony 是 Swarm₀
- 090号：严格性语法规则——胖节点形式不严格
- 174号：谱系即生成引擎——ceremony_scan.py 是谱系的感知器
- 218号：Lead 并行化原则（被本方案吸收：并行由 spawn_all_parallel 保证，不需要 Lead 内的并行化原则节）
