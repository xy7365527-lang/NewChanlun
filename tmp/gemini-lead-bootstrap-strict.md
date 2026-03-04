# Lead 最小自举形式——严格讨论结果

## 异质质询代理：Opus（代 Gemini 角色）

---

## 五个待讨论问题的严格判定

### Q1：结构工位的触发时机（meta-observer：首次 scan vs re-scan）

**判定：meta-observer 必须且只能在 re-scan 阶段加入 workstations。**

推导链：
1. meta-observer 的定义职责是"二阶观察——观察蜂群循环的产出"（meta-lead.md:76, dispatch-dag.yaml event_skill_map）
2. 首次 scan 时业务工位尚未执行，不存在可观察的产出
3. 在无观察对象时 spawn 观察者 = 类型错误（观察者的输入域为空集）
4. "永远不要串行"约束不适用于此处——meta-observer 与业务工位之间存在严格数据依赖（业务工位的产出是 meta-observer 的输入），这是串行的合法理由

**实现形式：ceremony_scan.py 新增 `--phase` 参数。**

| `--phase` 值 | 行为 |
|--------------|------|
| `initial`（默认） | 只输出业务工位 + 条件触发的 P0 工位（topo_effect、genealogy_anomaly） |
| `rescan` | 输出结构工位（meta-observer、topology-analyst、gangju-audit）+ 残余业务工位 |

meta-observer 的触发条件不是"scan 判断是否需要"，而是**无条件加入 re-scan 输出**。理由：meta-observer 的存在是蜂群语法的一部分（dispatch-dag.yaml genome_layer），不是条件触发的可选项。

topology-analyst 同理：只要 block-topology/blocks/ 目录存在且非空，re-scan 无条件加入。

### Q2：gangju 的执行时机

**判定：gangju 在两个 phase 都执行，但行为不同。**

| phase | gangju 行为 | 副作用 |
|-------|------------|--------|
| `initial` | 只读：extract_gang + compute_block_stats + compute_genealogy_stats + derive_mu → 输出 baseline | 无（不生成 pending 骨架） |
| `rescan` | 完整执行：同上 + generate_pending_skeleton（如果 audit_needed） | 写入 pending 骨架 |

推导链：
1. gangju 的 derive_mu 依赖 block_stats 和 genealogy_stats
2. 首次 scan 时这些统计反映 pre-work 状态 → baseline
3. re-scan 时反映 post-work 状态 → 可与 baseline 比较
4. generate_pending_skeleton 是写操作，在 pre-work 阶段执行无意义（工位还没做，生成的骨架基于过时状态）
5. 因此：首次 scan 的 gangju 必须是纯函数（无副作用），re-scan 的 gangju 允许副作用

**不需要 scan 区分"两个时刻的 gangju 输出可能不同"——这正是设计意图。** initial 的 gangju 输出是 baseline，rescan 的 gangju 输出是 delta 检测的输入。两者不同是正确的。

### Q3：code-verifier 的触发（scan 是否执行 git diff）

**判定：scan 必须执行 git diff。**

推导链：
1. ceremony_scan.py 第 943-944 行已经调用 `git rev-parse --short HEAD` → git 操作已在 scan 的职责范围内
2. `git diff --name-only HEAD~1 -- "src/**/*.py"` 是确定性操作，不涉及判断
3. 如果 scan 不做 git diff，则 code-verifier 的触发条件无法被计算 → Lead 必须自己决策 → 违反"Lead 不做判断"原则
4. scan 的职责是"确定性调度计划生成器"——git diff 是确定性输入，属于 scan 的合法职责

**精确实现：**
```python
def detect_code_changes(root):
    """检测 src/ 下 Python 文件是否有变更。"""
    try:
        output = subprocess.check_output(
            ["git", "diff", "--name-only", "HEAD~1", "--", "src/**/*.py"],
            cwd=root, text=True
        ).strip()
        return len(output) > 0
    except Exception:
        # git 操作失败时保守返回 False（不触发 code-verifier）
        return False
```

在 rescan phase 中：如果 `detect_code_changes(root)` 为 True，将 code-verifier 加入 workstations。

### Q4：ceremony.md 重写后的 Stop hook 处理

**判定：Stop hook 行为不属于 ceremony.md，属于 meta-lead.md 的"Session 退出"节。**

推导链：
1. ceremony.md 定义的是 spawn 序列（创世时刻的行为）
2. Stop hook 是 session 终止时的中断处理
3. 这是两个不同的生命周期阶段：ceremony = 开始，Stop hook = 结束
4. meta-lead.md 第 115-116 行已定义 "Session 退出" 行为
5. Stop hook 触发时的行为 = Session 退出流程的入口

**精确处理：**
- ceremony.md 中删除 Stop hook 相关内容
- meta-lead.md "Session 退出" 节增加一行：`Stop hook 触发时，执行 Session 退出流程`
- Stop hook 的具体行为（调用 TaskList、写 session 快照）已在 meta-lead.md 中定义，不需要重复

### Q5：增量持久化的责任转移

**判定：各工位写自己的命名空间文件，Lead 在 consume_all 时合并为 session 快照。**

推导链：
1. 选项 A（各工位自行写 session）→ 并发写入冲突 → 不允许
2. 选项 B（Lead 批量写入）→ 中途断掉时丢失 → 不允许
3. meta-lead.md "非中断信息流" 表（第 69-77 行）已定义模式：各工位写文件系统，其他工位下次被唤起时发现
4. 严格形式：各工位完成时写入 `.chanlun/sessions/fragments/{workstation_name}.yaml`（命名空间隔离，无冲突）
5. Lead 的 consume_all 合并 fragments/ 为完整 session 快照（原子操作）
6. 中途断掉时：fragments/ 中已有的文件不丢失，下次热启动时 Lead 从 fragments/ 恢复

**不存在第三种选项。** 这是"文件系统即通信总线"模式的直接推论。

---

## ceremony_scan.py 精确代码变更方案

### 变更 1：新增 `--phase` 参数

位置：`main()` 函数，argparse 部分（第 665-671 行之后）

```python
parser.add_argument(
    "--phase", choices=["initial", "rescan"], default="initial",
    help="扫描阶段：initial=首次扫描（业务工位），rescan=重扫描（结构工位+残余）"
)
```

### 变更 2：gangju 集成

位置：`main()` 函数末尾（第 933 行之后，async_self_ref 之后）

```python
# gangju 集成：纲举目张分析
try:
    sys.path.insert(0, os.path.join(root, "scripts"))
    from gangju_analysis import (
        extract_gang, compute_block_stats as gangju_block_stats,
        compute_genealogy_stats as gangju_genealogy_stats,
        derive_mu, generate_pending_skeleton,
    )
    gang = extract_gang(root)
    gj_block_stats = gangju_block_stats(root)
    gj_genealogy_stats = gangju_genealogy_stats(root)
    filled_mu, empty_mu, new_mu, residue_status = derive_mu(
        gj_block_stats, gj_genealogy_stats, root
    )
    result["gangju"] = {
        "gang": gang,
        "filled_mu": filled_mu,
        "empty_mu": empty_mu,
        "new_mu": new_mu,
        "residue_status": residue_status,
        "audit_needed": len(new_mu) > 0,
    }
    # rescan phase 才允许 gangju 的写副作用
    if args.phase == "rescan" and len(new_mu) > 0:
        generated = generate_pending_skeleton(root, new_mu)
        result["gangju"]["generated_pending"] = generated
        workstations.append({
            "priority": "P1",
            "name": f"gangju-audit：{len(new_mu)}个新目待质询",
            "status": "audit_needed",
            "source": "gangju",
        })
except Exception as exc:
    result["gangju_error"] = f"{type(exc).__name__}: {exc}"
```

### 变更 3：结构工位推导（rescan phase）

位置：`main()` 函数，workstations 最终输出之前

```python
# 结构工位推导（仅 rescan phase）
if args.phase == "rescan":
    # meta-observer：无条件加入（蜂群语法要求）
    workstations.append({
        "priority": "P1",
        "name": "meta-observer",
        "type": "structural",
        "trigger": "swarm_cycle_end",
        "source": "structural_derivation",
    })

    # topology-analyst：blocks/ 非空时加入
    blocks_dir = os.path.join(root, ".chanlun/block-topology/blocks")
    if os.path.isdir(blocks_dir) and glob.glob(os.path.join(blocks_dir, "*.json")):
        workstations.append({
            "priority": "P2",
            "name": "topology-analyst",
            "type": "structural",
            "trigger": "blocks_exist",
            "source": "structural_derivation",
        })

    # code-verifier：src/ 下有 Python 变更时加入
    if detect_code_changes(root):
        workstations.append({
            "priority": "P2",
            "name": "code-verifier",
            "type": "conditional",
            "trigger": "src_py_changed",
            "source": "structural_derivation",
        })
```

### 变更 4：新增 `detect_code_changes` 函数

位置：`compute_delta_blocks` 函数之后（第 662 行之后）

```python
def detect_code_changes(root):
    """检测 src/ 下 Python 文件是否有变更（code-verifier 触发条件）。"""
    try:
        output = subprocess.check_output(
            ["git", "diff", "--name-only", "HEAD~1", "--", "src/"],
            cwd=root, text=True
        ).strip()
        return bool(output)
    except Exception:
        return False
```

### 不变更的部分

- `get_required_skills()`：不变（event_skill_map 读取逻辑独立）
- `get_frozen_nodes()`：不变（frozen 过滤逻辑独立）
- `get_session_workstations()`：不变
- `get_roadmap_workstations()`：不变
- 所有 delta 检测函数：不变

---

## ceremony.md 重写（~40 行，形式即行为）

```markdown
# ceremony.md — Lead 最小自举序列

Lead 的全部行为是执行此序列。序列之外的行为不合法。

## 序列

1. `python scripts/ceremony_scan.py --phase initial`
   → 解析 stdout JSON → 获得 workstations[] + required_skills[]

2. 对 workstations[] 中每个工位，并行执行：
   `TeamCreate` → `Task(workstation.name, agent=workstation.agent)`
   不允许串行。无数据依赖的工位必须并行 spawn。

3. 等待所有 Task 完成（consume_all）。
   每个 Task 完成时，读取其产出文件（`.chanlun/sessions/fragments/{name}.yaml`）。
   不做判断。不做筛选。不做优先级排序。

4. 合并 fragments/ → 写入 `.chanlun/sessions/{timestamp}-session.md`。
   `git add . && git commit && git push`

5. `python scripts/ceremony_scan.py --phase rescan`
   → 解析 stdout JSON → 获得结构工位 + 残余业务工位 + gangju 结果

6. 对 rescan 输出的 workstations[] 并行 spawn（同步骤 2）。

7. 等待所有 Task 完成 → 合并 → 持久化（同步骤 3-4）。

8. 如果 rescan JSON 中 `clean_terminate=true`：`TeamDelete` → 结束。
   否则：回到步骤 1。

## 中断处理

- 概念分离信号 → `/escalate`（不在序列中插入判断步骤）
- 实现层僵持 → 了解双方立场 → 裁定或转为概念分离
- Stop hook → Session 退出流程（写 session 快照 → 结束）

## Lead 不可委托项

- ceremony_scan.py 调用（步骤 1、5）
- TeamCreate / TeamDelete（步骤 2、8）
- git commit/push（步骤 4）
- session 快照合并（步骤 4）

## Lead 不做的事

序列中没有的步骤，Lead 不做。
不判断工位优先级（scan 已排序）。不判断是否需要某工位（scan 已决定）。
不做概念层价值判断（走 /escalate）。
```

---

## 边界条件：方案失败的情况

### 失败条件 1：ceremony_scan.py 本身崩溃

如果 scan 脚本抛出异常（Python 错误、YAML 解析失败、文件系统权限问题），Lead 无法获得 workstations 列表 → 整个序列阻塞。

**缓解**：scan 已有 try/except 保护（每个检测模块独立 catch），但 main() 级别的崩溃无保护。需要在 main() 外层加 try/except，输出 `{"error": "...", "workstations": []}` 而非 traceback。

### 失败条件 2：gangju import 路径问题

gangju_analysis.py 在 scripts/ 目录，ceremony_scan.py 也在 scripts/。`sys.path.insert` 可能与已有 import 冲突（如 gangju_analysis 内部的相对 import）。

**缓解**：gangju_analysis.py 当前只使用标准库 import（json, os, glob, re, sys, yaml），无相对 import。但如果未来 gangju 引入相对 import，此处会断裂。严格形式：gangju 的核心函数应作为 ceremony_scan 的内部函数或通过 `importlib` 显式加载。

### 失败条件 3：fragments/ 并发写入的文件系统限制

Windows 文件系统（NTFS）在高并发小文件写入时可能出现锁竞争。各工位写入不同文件名（命名空间隔离），理论上无冲突，但 NTFS 的目录级锁可能导致写入延迟。

**缓解**：fragments/ 下的文件数量 = 工位数量（通常 < 20），不构成性能瓶颈。如果工位数量超过 50，需要考虑分目录。

### 失败条件 4：re-scan 与 initial scan 的 gangju 输出无法比较

当前方案中 initial scan 的 gangju 输出嵌入在 JSON 中，但 re-scan 时无法自动获取 initial scan 的输出（已被 Lead 消费）。如果需要 delta 比较，Lead 必须将 initial gangju 输出传递给 re-scan。

**缓解**：两种方式：
- (a) initial scan 将 gangju baseline 写入 `.chanlun/sessions/fragments/_gangju_baseline.json`，re-scan 读取
- (b) ceremony_scan.py 新增 `--gangju-baseline` 参数接受 JSON 文件路径

推荐 (a)：符合"文件系统即通信总线"模式，不增加 CLI 参数复杂度。

### 失败条件 5：`--phase rescan` 时 workstations 为空但 clean_terminate=false

如果 rescan 输出的业务工位为空（所有工作已完成），但 gangju 发现 new_mu > 0（审计需要），clean_terminate 为 false。此时 gangju-audit 工位被 spawn，但它的产出可能再次触发 new_mu → 无限循环。

**缓解**：gangju 的 derive_mu 中已有 `audited` 集合过滤（gangju_analysis.py 第 705 行 `if "质询深度不足" not in audited`）。只要 gangju-audit 工位完成后将审计结果写入 settled 谱系，下一轮 derive_mu 会将其排除。但需要确认 gangju-audit agent 的产出格式与 derive_mu 的 audited 检测逻辑匹配。

---

## 总结

五个问题的判定均为确定性结论，不存在需要 Lead 判断的模糊地带：

| # | 问题 | 判定 |
|---|------|------|
| 1 | meta-observer 触发时机 | rescan phase 无条件加入 |
| 2 | gangju 执行时机 | 两个 phase 都执行，initial 只读，rescan 允许副作用 |
| 3 | code-verifier 触发 | scan 必须执行 git diff |
| 4 | Stop hook 处理 | 属于 meta-lead.md，不属于 ceremony.md |
| 5 | 增量持久化 | fragments/ 命名空间隔离 + consume_all 合并 |

ceremony_scan.py 变更量：~60 行新增（1 个新函数 + 3 个代码块插入），0 行删除。
ceremony.md 重写：40 行，完全确定性序列，无判断步骤。
