# Codex 异质审查：Lead 最小自举形式——严格诊断

## 诊断方法

每个判定基于代码层事实（文件路径:行号），不接受"合理"/"可行"等模糊判定。

---

## 一、5 个待讨论问题逐一严格诊断

### Q1：结构工位的触发时机（首次 scan vs re-scan）

**代码事实**：
- `dispatch-dag.yaml` 第 200-201 行：meta-observer 触发条件 = `swarm_cycle_end`
- `dispatch-dag.yaml` 第 272-274 行：topology-analyst 触发条件 = `swarm_cycle_end` + `block-topology/blocks/ 存在且区块数 > 0`
- `ceremony_scan.py` 当前输出中无 `type: structural` 工位——只有业务工位（第 770 行 `result["workstations"] = workstations`，workstations 来源全部是 roadmap/session/fallback/downstream_audit/async_self_ref/pattern_buffer/genealogy_anomaly/pending_topo）

**严格判定**：meta-observer 和 topology-analyst 的触发条件是 `swarm_cycle_end`——即蜂群循环结束时。首次 scan 时蜂群循环尚未开始，此时 spawn meta-observer = 观察空集。代码事实否定"首次 scan 就加入结构工位"。

**严格形式**：ceremony_scan.py 需要 `--phase` 参数区分 `initial`（首次）和 `rescan`（循环结束后）。`phase=initial` 不输出 `swarm_cycle_end` 触发的结构工位；`phase=rescan` 输出。

**边界条件**：如果 `phase=rescan` 时 meta-observer 每次都产出新发现，re-scan 循环不终止。需要终止条件：rescan 产出的 workstations 与上一轮完全相同 → 终止（不动点）。

---

### Q2：gangju 的执行时机

**代码事实**：
- `gangju_analysis.py` 第 369 行 `derive_mu(block_stats, genealogy_stats, root)` 的输入全部来自文件系统读取
- `compute_block_stats()` 第 49-116 行：读 `.chanlun/block-topology/blocks/*.json` + `relations.jsonl` + `meta.json`
- `compute_genealogy_stats()` 第 123-154 行：读 `.chanlun/genealogy/settled/*.md` + `pending/*.md`
- `derive_mu()` 第 369-756 行：纯函数（输入 → 输出），无副作用
- `generate_pending_skeleton()` 第 777-842 行：**写入** `.chanlun/genealogy/pending/` ——这是写操作

**严格判定**：gangju 的分析函数（extract_gang, compute_block_stats, compute_genealogy_stats, derive_mu）全部是只读的。文件系统状态本身就是时间戳——首次 scan 读到的是 pre-work 状态，re-scan 读到的是 post-work 状态。scan 不需要区分，文件系统状态即区分。

**关键发现**：`generate_pending_skeleton()` 是写操作。ceremony_scan.py 的设计原则是"只读扫描"（第 479 行、第 484 行明确声明）。gangju 并入 scan 时，**必须只导入只读函数，不导入 generate_pending_skeleton**。pending 骨架生成应由 Lead 在 consume 阶段根据 `audit_needed=true` 单独执行。

**严格形式**：ceremony_scan.py 导入 gangju 的 4 个只读函数，在 main() 末尾调用 derive_mu()，将结果写入 JSON 输出的 `gangju` 字段。`audit_needed=true` 时在 workstations[] 中追加审计工位。不调用 generate_pending_skeleton。

---

### Q3：code-verifier 的触发（scan 是否应执行 git diff）

**代码事实**：
- `dispatch-dag.yaml` 第 207-213 行：code-verifier 触发条件 = `file_write` on `src/**/*.py` 和 `tests/**/*.py`——这是**事件**，不是状态
- `ceremony_scan.py` 第 479 行："测试验证不在此脚本中执行（只读扫描原则）"
- `ceremony_scan.py` 第 484 行："测试验证：不在 scan 中执行 pytest（只读扫描原则）"
- ceremony_scan.py 唯一的 subprocess 调用是 `_run_validation_cmd()`（第 213-224 行）用于 roadmap VDW，和 `git rev-parse`（第 943-944 行）

**严格判定**：code-verifier 的触发是 `file_write` 事件——由 hooks（PostToolUse）检测，不是由 scan 检测。scan 检测**状态**（文件系统快照），hooks 检测**事件**（操作发生）。将 git diff 加入 scan = 混淆状态检测与事件检测。代码事实否定"scan 应执行 git diff"。

**严格形式**：code-verifier 保持事件驱动（hook 触发），不进入 scan 输出。scan 不执行 git diff。

---

### Q4：ceremony.md 重写后的 Stop hook 处理

**代码事实**：
- `.claude/agents/ceremony.md` 不存在（Glob 搜索无结果）
- ceremony 行为当前定义在 `dispatch-dag.yaml` 第 638-727 行（ceremony_sequence）
- Stop hook 是 Claude Code 平台级事件，与 ceremony 的 bootstrap 序列无关
- `.claude/rules/no-unnecessary-escalation.md` 已定义输出格式约束（格式 A/B/C）——Stop hook 拦截时的行为属于"输出格式"范畴

**严格判定**：Stop hook 处理 = "被中断时做什么"。ceremony.md 定义 = "启动时做什么"。两者是不同的关注点。Stop hook 行为属于 grammar_rules 层（`.claude/rules/`），不属于 ceremony 层。

**严格形式**：Stop hook 处理写入 `.claude/rules/` 下的规则文件（可以是 `no-unnecessary-escalation.md` 的扩展，或独立的 `stop-hook-behavior.md`）。ceremony.md 只定义 bootstrap 链路。

---

### Q5：增量持久化的责任转移

**代码事实**：
- `meta-lead.md` 第 116 行："Session 退出: 写入 .chanlun/sessions/：时间戳、任务进度、生成态谱系 ID、下次启动优先级"——session 写入是 Lead 的已定义职责
- session 文件格式是 markdown（`.chanlun/sessions/*-session.md`），ceremony_scan.py 第 407-470 行解析其结构
- Claude Code 的 SendMessage 是串行消费的（Lead 逐条接收 teammate 消息）——不存在并发写入问题，因为 Lead 是单线程消费者

**严格判定**：
- 选项 A（各工位自行写入）：代码事实否定——session 文件有结构化格式（表格行、节标题），多 agent 并发写入同一 markdown 文件 = 格式破坏
- 选项 B（Lead batch 写入 consume_all 时）：存在真实风险——中途断掉丢失状态
- 严格形式：Lead 逐条消费 SendMessage，每条 completion 到达时增量追加 session。这不是"Lead 做实质工作"——这是路由+持久化，是 meta-lead.md 已定义的职责。SendMessage 的串行消费保证无并发冲突。

**边界条件**：如果 Lead 在增量写入中途崩溃，session 文件可能不完整。缓解：写入前先读取当前内容，追加后原子写入（write to temp → rename）。但 Claude Code 平台不保证原子文件操作——这是平台层约束，不是方案层缺陷。

---

## 二、ceremony_scan.py 扩展精确代码变更方案

### 变更 1：新增 `--phase` 参数（第 669-670 行区域）

```python
# 在 parser.add_argument("--workstations-json", ...) 之后新增：
parser.add_argument("--phase", choices=["initial", "rescan"], default="initial",
                    help="扫描阶段：initial（首次）或 rescan（工位完成后）")
```

### 变更 2：新增 `derive_structural_workstations()` 函数（第 491 行之后）

```python
def derive_structural_workstations(root, phase="initial"):
    """从 dispatch-dag event_skill_map 推导结构工位（swarm_cycle_end 触发）。

    phase="initial": 不输出 swarm_cycle_end 工位（循环未开始）
    phase="rescan": 输出 meta-observer + topology-analyst（条件满足时）
    """
    if phase != "rescan":
        return []

    structural = []
    # meta-observer: swarm_cycle_end（无条件）
    structural.append({
        "priority": "P1",
        "name": "meta-observer",
        "type": "structural",
        "trigger": "swarm_cycle_end",
        "status": "pending",
        "source": "structural_derivation",
    })
    # topology-analyst: swarm_cycle_end + blocks 存在
    blocks_dir = os.path.join(root, ".chanlun/block-topology/blocks")
    if os.path.isdir(blocks_dir) and glob.glob(os.path.join(blocks_dir, "*.json")):
        structural.append({
            "priority": "P1",
            "name": "topology-analyst",
            "type": "conditional",
            "trigger": "swarm_cycle_end:blocks_exist",
            "status": "pending",
            "source": "structural_derivation",
        })
    return structural
```

### 变更 3：gangju 只读函数导入 + 集成（第 900 行区域，async_self_ref 之后）

```python
    # gangju 纲举目张集成（只读分析，不生成 pending 骨架）
    try:
        try:
            import scripts.gangju_analysis as _gangju_mod
        except ImportError:
            import gangju_analysis as _gangju_mod

        _g_block = _gangju_mod.compute_block_stats(root)
        _g_genea = _gangju_mod.compute_genealogy_stats(root)
        _g_filled, _g_empty, _g_new, _g_residue = _gangju_mod.derive_mu(
            _g_block, _g_genea, root,
        )
        result["gangju"] = {
            "filled_mu": _g_filled,
            "empty_mu": _g_empty,
            "new_mu": _g_new,
            "residue_status": _g_residue,
            "audit_needed": len(_g_new) > 0,
        }
        if _g_new:
            workstations.append({
                "priority": "P1",
                "name": f"纲举目张审计：{len(_g_new)}个新目",
                "status": "gangju:audit_needed",
                "source": "gangju_analysis",
                "new_mu_names": [m["mu"] for m in _g_new],
            })
    except Exception as exc:
        result["gangju_error"] = f"{type(exc).__name__}: {exc}"
```

### 变更 4：结构工位注入（main() 末尾，workstations 最终组装后）

```python
    # 结构工位推导（phase=rescan 时注入 swarm_cycle_end 工位）
    structural_ws = derive_structural_workstations(root, phase=args.phase)
    if structural_ws:
        workstations.extend(structural_ws)
        result["structural_workstations"] = structural_ws
```

### 不变更的部分

- `generate_pending_skeleton()` 不被 ceremony_scan.py 调用——只读扫描原则
- `_run_validation_cmd()` 保持现状——VDW 是 roadmap 的已有功能
- git diff 不引入——code-verifier 保持事件驱动

---

## 三、ceremony.md 精确文本（~40 行）

文件路径：`.claude/agents/ceremony.md`（新建）

```markdown
---
name: ceremony
description: Lead 最小自举序列——形式即行为，不依赖记忆
---

## 链路（不可委托，不可重排）

1. `python scripts/ceremony_scan.py --phase initial` → JSON
2. JSON.clean_terminate == true → 输出 `[020号反转] 干净终止` → 停止
3. TeamCreate
4. JSON.workstations[] 全部并行 spawn（无 depends_on 的工位并行，有 depends_on 的按序）
5. 逐条 consume SendMessage：每条 completion 到达时增量写 session
6. 全部完成 → `python scripts/ceremony_scan.py --phase rescan`
7. rescan.workstations[] 非空且与上轮不同 → 回到步骤 4
8. rescan.workstations[] 为空或与上轮相同（不动点） → persist session → TeamDelete

## 白名单（Lead 只执行这三类）

| 类 | 操作 |
|----|------|
| 调度 | ceremony_scan.py, TeamCreate, TeamDelete |
| 路由 | Task spawn, SendMessage 转发 |
| 持久化 | git commit/push, session 写入 |

Lead 不做实质认知工作。scan 输出什么就 spawn 什么。

## 不变量

- **确定性**：相同文件系统状态 → 相同 scan 输出 → 相同 workstations
- **并行默认**：workstations[] 无依赖关系的工位全部并行 spawn
- **增量持久化**：每条 completion 到达时写 session，不等 consume_all
- **不动点终止**：rescan 输出与上轮相同 → 循环终止（防止无限 re-scan）
- **只读扫描**：scan 不写文件、不执行 git diff、不运行测试（VDW 除外）
```

行数：约 35 行（含 frontmatter）。

---

## 四、与 Gemini 方案预判分歧点

| 维度 | Codex 判定 | Gemini 可能判定 | 分歧根源 |
|------|-----------|----------------|---------|
| gangju 导入方式 | import 只读函数（代码复用，单一事实源） | 可能主张内联核心逻辑（自包含） | 耦合度 vs 自包含性 |
| phase 区分方式 | 显式 `--phase` 参数（调用方声明意图） | 可能主张隐式检测（从 session 状态推断） | 显式 vs 隐式——显式更严格（不依赖推断） |
| 不动点终止 | workstations 集合相等 → 终止 | 可能主张 max_rescan_depth 硬限制 | 数学终止条件 vs 工程安全阀——两者不矛盾，可并存 |
| Stop hook 归属 | grammar_rules 层（`.claude/rules/`） | 可能主张放入 ceremony.md（bootstrap 包含中断处理） | 关注点分离——ceremony = 启动，rules = 运行时约束 |
| pending 骨架生成 | scan 不生成（只读原则），Lead 在 consume 阶段生成 | 可能主张 scan 直接生成（减少 Lead 逻辑） | 只读扫描原则是否为硬约束——代码事实支持硬约束（第 479/484 行明确声明） |

---

## 五、边界条件：方案失败场景

### 1. gangju import 失败
- **触发**：gangju_analysis.py 不在 sys.path 中，或其依赖（yaml）缺失
- **影响**：scan 输出无 `gangju` 字段，Lead 失去纲举目张审计能力
- **缓解**：try/except 已覆盖（与 async_self_reference 同模式），scan 不崩溃
- **严重度**：低——gangju 是增强功能，不是核心链路

### 2. re-scan 无限循环
- **触发**：meta-observer 每次 rescan 都产出新发现（如谱系张力持续存在）
- **影响**：Lead 永远不终止
- **缓解**：不动点终止条件（workstations 集合相等 → 终止）。如果 meta-observer 每次产出不同的发现，不动点不成立——需要 max_rescan_depth 作为安全阀（建议 max=3）
- **严重度**：高——必须在 ceremony.md 中声明终止条件

### 3. TeamCreate 平台限制
- **触发**：Claude Code 平台限制同时 team 数量
- **影响**：无法 spawn workstations
- **缓解**：当前无代码层缓解。需要 fallback：TeamCreate 失败时 Lead 串行执行（退化模式）
- **严重度**：中——平台约束，非方案缺陷

### 4. ceremony_scan.py 与 gangju_analysis.py 的 block_stats 重复计算
- **触发**：ceremony_scan.py 已有 `compute_delta_blocks()`（第 630-661 行），gangju 也有 `compute_block_stats()`（第 49-116 行）。两者都读 block-topology。
- **影响**：同一次 scan 中 block-topology 被读两次——性能浪费，且如果中间有写入（不应该有，scan 是只读的），可能不一致
- **缓解**：scan 的 `compute_delta_blocks()` 是轻量版（只算 delta），gangju 的是完整版（type/relation/source 分布）。两者不冲突但有冗余。未来可统一为 gangju 版本，scan 的 delta_blocks 从 gangju 输出中提取。
- **严重度**：低——性能影响可忽略（文件系统读取，非网络调用）

### 5. session 文件原子性
- **触发**：Lead 增量写 session 时崩溃
- **影响**：session 文件不完整，下次 warm_start 解析可能失败或遗漏工位
- **缓解**：ceremony_scan.py 的 `get_session_workstations()` 对格式有容错（第 427-462 行，多种格式匹配）。但如果 markdown 结构被截断（如 `## 待处理` 节标题写入但内容未写入），可能误判为空。
- **严重度**：中——需要原子写入（write-to-temp + rename），但 Claude Code 平台不保证此操作

---

## 诊断结论

5 个问题的严格判定：

1. **结构工位触发时机**：`--phase rescan` 时才输出（代码事实：触发条件 = swarm_cycle_end）
2. **gangju 执行时机**：scan 调用只读函数，文件系统状态即时间戳（代码事实：derive_mu 是纯函数）
3. **code-verifier**：不进入 scan（代码事实：触发条件 = file_write 事件，非状态）
4. **Stop hook**：归属 grammar_rules 层（代码事实：ceremony.md 不存在，Stop 是运行时约束）
5. **增量持久化**：Lead 逐条 consume 时写入（代码事实：meta-lead.md 已定义 session 写入为 Lead 职责）

核心风险：re-scan 无限循环（需不动点终止 + max_rescan_depth 安全阀）。
