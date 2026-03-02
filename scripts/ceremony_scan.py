#!/usr/bin/env python
"""蜂群 spawn 通用工具。任何节点（ceremony 或 teammate）spawn 蜂群时调用。

用法:
  python scripts/ceremony_scan.py                  # 根 ceremony：全量扫描
  python scripts/ceremony_scan.py --skills         # 只输出 required_skills
  python scripts/ceremony_scan.py --workstations "任务A" "任务B"  # 指定业务工位

075号更新：structural_nodes → required_skills（事件驱动 skill 架构）
079号更新：background_noise 降级 + 业务层任务发现（no_work_fallback）
081号更新：roadmap.yaml 扫描（最高优先级任务来源）+ 终止逻辑修正
089号声明：当前为硬编码来源优先级扫描，不是 DAG 拓扑排序。
147号更新：topo_effect 扫描——frozen 节点的下游工位不 spawn
176号更新：delta_genealogy 检测——RTAS循环是否产生新谱系
178号更新：delta_blocks 检测——block-topology 区块变化
183号更新：async_self_reference 集成——t审查t-1 异步自指审计
274号更新：priority 字段语义明确为来源标记（非全局排序依据）。
    扫描来源有序列（roadmap → session → fallback），但这是"从哪里发现工位"的策略，
    不是"工位之间谁先谁后"的排序。Lead 消费 scan 输出时全部并行 spawn（218号要求），
    不按 priority 字段排序。
    dispatch-dag.yaml 的 ceremony_sequence 定义了 DAG 格式的 nodes+depends_on，
    但本脚本以来源优先级线性扫描实现（roadmap → session → fallback）——这是有意的工程选择。
    DAG 声明保留逻辑依赖信息供 LLM 解释执行（057号推论：LLM 不是状态机），
    代码实现的线性扫描覆盖最常见的执行路径。
    演化路径：如需强制拓扑排序，可重写为真正读取 ceremony_sequence 的 DAG 解析器。
"""
import json, os, glob, yaml, argparse, subprocess, re


BACKGROUND_NOISE_STATUSES = {"background_noise", "观察项", "背景噪音"}
TERMINAL_STATUSES = {"已修复", "resolved", "background_noise"}
VALID_TOPO_TYPES = frozenset({"freeze", "split", "sever"})


def get_required_skills(root):
    """从 dispatch-dag 的 event_skill_map 读取 structural skill 列表。"""
    dag_path = os.path.join(root, ".chanlun/dispatch-dag.yaml")
    skills = []
    if os.path.isfile(dag_path):
        with open(dag_path, encoding="utf-8") as f:
            dag = yaml.safe_load(f)
        for skill in dag.get("event_skill_map", []):
            if skill.get("skill_type") == "structural":
                skills.append({
                    "id": skill["id"],
                    "agent": skill.get("agent", f".claude/agents/{skill['id']}.md"),
                    "triggers": [t.get("event", "") for t in skill.get("triggers", [])]
                })
    return skills


def get_frozen_nodes(root):
    """从 block-topology 读取 frozen 节点集合（147号 + 178号-2 迁移）。

    查询 freezes 关系的 target，对 scope=downstream 的 freeze 做 BFS 展开
    下游依赖。通过 meta.json 反查回旧谱系编号（下游 workstation 过滤用旧编号）。

    downstream 语义由查询方保证（topology_operator 写入时只记录一条 freezes
    关系 + content.scope="downstream"，不展开）。本函数就是那个查询方。
    """
    base = os.path.join(root, ".chanlun/block-topology")
    relations_path = os.path.join(base, "relations.jsonl")
    meta_path = os.path.join(base, "meta.json")

    if not os.path.isfile(relations_path):
        return set()

    # 1. 读取所有关系
    all_rels = []
    try:
        with open(relations_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    all_rels.append(json.loads(line))
    except Exception:
        return set()

    # 2. 提取 freezes 关系的 direct targets + 对应的 event 区块 id
    #    freezes 关系: from=event_block, to=target_block, created_by=event_block
    frozen_sha_ids = set()
    freeze_events = []  # (event_block_id, target_sha)
    for rel in all_rels:
        if rel.get("relation") == "freezes":
            frozen_sha_ids.add(rel["to"])
            freeze_events.append((rel["from"], rel["to"]))

    if not frozen_sha_ids:
        return set()

    # 3. 对 scope=downstream 的 freeze，BFS 展开 target 的所有下游
    #    读取 freeze event 区块的 content.scope
    blocks_dir = os.path.join(base, "blocks")
    downstream_targets = set()
    for event_id, target_sha in freeze_events:
        event_path = os.path.join(blocks_dir, f"{event_id}.json")
        if not os.path.isfile(event_path):
            continue
        try:
            with open(event_path, encoding="utf-8") as f:
                event_block = json.load(f)
            if event_block.get("content", {}).get("scope") == "downstream":
                downstream_targets.add(target_sha)
        except Exception:
            continue

    if downstream_targets:
        # 构建 depends_on 图: parent → children
        # depends_on: from depends_on to → to 是 parent, from 是 child
        children_of = {}
        for rel in all_rels:
            if rel.get("relation") == "depends_on" and rel.get("order") == 1:
                parent = rel["to"]
                child = rel["from"]
                children_of.setdefault(parent, set()).add(child)

        # BFS 从每个 downstream target 展开所有下游
        for target_sha in downstream_targets:
            queue = list(children_of.get(target_sha, set()))
            while queue:
                current = queue.pop()
                if current not in frozen_sha_ids:
                    frozen_sha_ids.add(current)
                    queue.extend(
                        children_of.get(current, set()) - frozen_sha_ids
                    )

    # 4. 反查 id_mapping: sha → 旧编号
    reverse_mapping = {}
    if os.path.isfile(meta_path):
        try:
            with open(meta_path, encoding="utf-8") as f:
                meta = json.load(f)
            for old_id, sha in meta.get("id_mapping", {}).items():
                reverse_mapping[sha] = old_id
        except Exception:
            pass

    frozen_ids = set()
    for sha in frozen_sha_ids:
        old_id = reverse_mapping.get(sha)
        if old_id:
            frozen_ids.add(str(old_id))
        else:
            frozen_ids.add(sha)  # fallback: 新区块无旧映射时用 SHA

    return frozen_ids


def get_topo_effects_from_genealogy(root):
    """扫描已结算谱系文件的 topo_effect 字段（147号：审查结果回到 ceremony_scan）。

    返回 topo_effect 条目列表，供 ceremony 展示和处理。
    """
    effects = []
    for settled_file in glob.glob(os.path.join(root, ".chanlun/genealogy/settled/*.md")):
        try:
            with open(settled_file, encoding="utf-8") as f:
                first_lines = f.read(2000)
            if "topo_effect:" in first_lines and 'topo_effect: ""' not in first_lines:
                # 提取 topo_effect 值
                for line in first_lines.split("\n"):
                    stripped = line.strip()
                    if stripped.startswith("topo_effect:"):
                        val = stripped.split(":", 1)[1].strip().strip('"').strip("'")
                        if val:
                            effects.append({
                                "file": os.path.basename(settled_file),
                                "topo_effect": val,
                            })
                        break
        except Exception:
            pass
    return effects


def detect_pending_topo_effects(root):
    """177号：扫描含结构化 topo_effect 但未执行的谱系文件。

    结构化格式：type:target:scope（如 freeze:062:downstream）。
    已有 topo_executed_at 的文件跳过（已执行）。
    非结构化的描述性 topo_effect 不纳入（不是可执行的拓扑操作）。

    返回 pending topo_effect 列表，供 RTAS 循环执行。
    """
    pending = []
    for settled_file in glob.glob(os.path.join(root, ".chanlun/genealogy/settled/*.md")):
        try:
            with open(settled_file, encoding="utf-8") as f:
                head = f.read(2000)
            # 解析 frontmatter
            fm_match = re.match(r"^---\s*\n(.+?)\n---", head, re.DOTALL)
            if not fm_match:
                continue
            fm = yaml.safe_load(fm_match.group(1))
            if not isinstance(fm, dict):
                continue
            # 已执行则跳过
            if fm.get("topo_executed_at"):
                continue
            # 检查结构化 topo_effect
            te = str(fm.get("topo_effect", "")).strip().strip('"').strip("'")
            if not te:
                continue
            parts = te.split(":")
            if len(parts) != 3 or parts[0] not in VALID_TOPO_TYPES:
                continue
            pending.append({
                "file": os.path.basename(settled_file),
                "topo_effect": te,
                "id": str(fm.get("id", "unknown")),
            })
        except Exception:
            pass
    return pending


def _run_validation_cmd(cmd, root, timeout=30):
    """执行 validation_cmd，返回 (exit_code, stdout, stderr)。超时视为失败。"""
    try:
        result = subprocess.run(
            cmd, shell=True, cwd=root,
            capture_output=True, text=True, timeout=timeout,
        )
        return result.returncode, result.stdout.strip(), result.stderr.strip()
    except subprocess.TimeoutExpired:
        return -1, "", f"validation_cmd timeout ({timeout}s)"
    except Exception as exc:
        return -1, "", f"validation_cmd error: {exc}"


def get_roadmap_workstations(root):
    """从 .chanlun/roadmap.yaml 读取待执行的业务目标（最高优先级任务来源）。

    081号谱系：roadmap 是结构化业务目标载体，ceremony_scan 的最高优先级来源。
    只返回 status=active 的任务。

    验证驱动（VDW）：如果任务有 validation_cmd 字段，先执行验证。
    Exit Code 0 → auto_verified=true，不生成工位。
    Exit Code 非 0 → 生成工位，附带验证输出。
    """
    roadmap_path = os.path.join(root, ".chanlun/roadmap.yaml")
    tasks = []
    if not os.path.isfile(roadmap_path):
        return tasks
    try:
        with open(roadmap_path, encoding="utf-8") as f:
            data = yaml.safe_load(f)
        for task in data.get("tasks", []):
            if task.get("status") == "active":
                task_id = task.get("id", task.get("title", "未命名"))
                validation_cmd = task.get("validation_cmd")
                relevant_files = task.get("relevant_files", [])
                subtasks = task.get("subtasks", [])

                # subtasks 展开：每个 subtask 生成独立工位
                if subtasks:
                    decomposition = task.get("decomposition", "parallel")
                    for st in subtasks:
                        st_id = st.get("id", "unknown")
                        st_name = f"{task_id}:{st_id}"
                        st_validation = st.get("validation_cmd", validation_cmd)
                        st_relevant = st.get("relevant_files", relevant_files)

                        # subtask 级别的 VDW
                        if st_validation:
                            exit_code, stdout, stderr = _run_validation_cmd(
                                st_validation, root,
                            )
                            if exit_code == 0:
                                tasks.append({
                                    "priority": task.get("priority", "P2"),
                                    "name": st_name,
                                    "status": "roadmap:active",
                                    "source": "roadmap",
                                    "description": st.get("description", ""),
                                    "title": st.get("title", ""),
                                    "parallel_group": task_id,
                                    "decomposition": decomposition,
                                    "auto_verified": True,
                                })
                                continue
                            validation_output = (stderr or stdout)[:500]
                            tasks.append({
                                "priority": task.get("priority", "P2"),
                                "name": st_name,
                                "status": "roadmap:active",
                                "source": "roadmap",
                                "description": st.get("description", ""),
                                "title": st.get("title", ""),
                                "relevant_files": st_relevant,
                                "validation_output": validation_output,
                                "done_criteria": st_validation,
                                "parallel_group": task_id,
                                "decomposition": decomposition,
                            })
                        else:
                            tasks.append({
                                "priority": task.get("priority", "P2"),
                                "name": st_name,
                                "status": "roadmap:active",
                                "source": "roadmap",
                                "description": st.get("description", ""),
                                "title": st.get("title", ""),
                                "relevant_files": st_relevant,
                                "parallel_group": task_id,
                                "decomposition": decomposition,
                            })
                    continue

                # 无 subtasks——原有逻辑（向后兼容）
                # 验证驱动：有 validation_cmd 时先执行
                if validation_cmd:
                    exit_code, stdout, stderr = _run_validation_cmd(
                        validation_cmd, root,
                    )
                    if exit_code == 0:
                        # 验证通过——不生成工位，但记录 auto_verified
                        tasks.append({
                            "priority": task.get("priority", "P2"),
                            "name": task_id,
                            "status": "roadmap:active",
                            "source": "roadmap",
                            "description": task.get("description", ""),
                            "title": task.get("title", ""),
                            "auto_verified": True,
                        })
                        continue

                    # 验证失败——生成工位，附带诊断信息
                    validation_output = (stderr or stdout)[:500]
                    tasks.append({
                        "priority": task.get("priority", "P2"),
                        "name": task_id,
                        "status": "roadmap:active",
                        "source": "roadmap",
                        "description": task.get("description", ""),
                        "title": task.get("title", ""),
                        "relevant_files": relevant_files,
                        "validation_output": validation_output,
                        "done_criteria": validation_cmd,
                    })
                else:
                    # 无 validation_cmd——行为不变（向后兼容）
                    tasks.append({
                        "priority": task.get("priority", "P2"),
                        "name": task_id,
                        "status": "roadmap:active",
                        "source": "roadmap",
                        "description": task.get("description", ""),
                        "title": task.get("title", ""),
                    })
    except Exception as exc:
        # roadmap 格式错误时不阻塞扫描，但记录错误
        tasks.append({
            "priority": "P0",
            "name": "roadmap.yaml 解析错误",
            "status": f"error: {exc}",
            "source": "roadmap_error",
        })
    return tasks


def detect_swarm_persistence_gaps(root):
    """153号下游推论：检测有谱系产出证据但无 session 的蜂群。

    方法：从谱系的 negation_source 字段提取 v{N}-swarm 引用，
    与 session 中出现的蜂群编号交叉比对。
    有谱系产出但无 session = 持久化断裂。
    """
    # 1. 从 session 文件提取已记录的蜂群编号
    #    同时扫描 archive/ 下的归档 session（归档不应导致持久化断裂误报）
    session_swarms = set()
    session_globs = [
        os.path.join(root, ".chanlun/sessions/*-session.md"),
        os.path.join(root, ".chanlun/sessions/archive/*-session.md"),
    ]
    for pattern in session_globs:
        for session_file in glob.glob(pattern):
            try:
                with open(session_file, encoding="utf-8") as f:
                    content = f.read(8000)
                for m in re.finditer(r'v(\d+)-swarm', content):
                    session_swarms.add(int(m.group(1)))
            except Exception:
                pass
    # 2. 从谱系文件提取引用的蜂群编号（negation_source / 正文）
    genealogy_swarms = {}  # {swarm_id: [谱系文件名]}
    for settled_file in glob.glob(os.path.join(root, ".chanlun/genealogy/settled/*.md")):
        try:
            with open(settled_file, encoding="utf-8") as f:
                content = f.read(4000)
            for m in re.finditer(r'v(\d+)-swarm', content):
                sid = int(m.group(1))
                fname = os.path.basename(settled_file)
                genealogy_swarms.setdefault(sid, [])
                if fname not in genealogy_swarms[sid]:
                    genealogy_swarms[sid].append(fname)
        except Exception:
            pass
    # 3. 交叉比对：有谱系引用但无 session = 持久化断裂
    gaps = []
    for sid, sources in sorted(genealogy_swarms.items()):
        if sid not in session_swarms:
            gaps.append({"swarm_id": sid, "genealogy_refs": sources})
    return gaps


def get_session_workstations(root):
    """从最新 session 提取遗留工位，过滤 background_noise。"""
    workstations = []
    sessions = glob.glob(os.path.join(root, ".chanlun/sessions/*-session.md"))
    session_name = None
    if sessions:
        sessions.sort(key=os.path.getmtime)
        session_name = os.path.basename(sessions[-1])
        with open(sessions[-1], encoding="utf-8") as f:
            content = f.read()
        in_section = None
        for line in content.split("\n"):
            # 匹配 "## 待处理" 或 "## 遗留" 或 "## 中断" 等节标题
            if line.startswith("## ") and any(kw in line for kw in ("待处理", "遗留", "中断")):
                in_section = line
                continue
            # 新节标题结束当前扫描区
            if in_section and line.startswith("## "):
                in_section = None
                continue
            if not in_section:
                continue
            # 格式A：表格行（旧格式）——优先级列必须是 P\d+ 格式
            if line.startswith("|"):
                parts = [c.strip() for c in line.split("|") if c.strip()]
                if len(parts) >= 3 and re.match(r'^P\d+$', parts[0]):
                    status = parts[2]
                    if status in TERMINAL_STATUSES or "✅" in status or "—" in parts[0]:
                        continue
                    if any(kw in status.lower() for kw in BACKGROUND_NOISE_STATUSES):
                        continue
                    workstations.append({
                        "priority": parts[0],
                        "name": parts[1],
                        "status": status,
                    })
            # 格式B：编号列表（当前格式）
            # 匹配 "1. **xxx**" 或 "- **xxx**"
            elif (line.strip().startswith(("1.", "2.", "3.", "4.", "5.", "6.", "7.", "8.", "9."))
                  or line.strip().startswith("- **")):
                m = re.match(r'^\s*(?:\d+\.\s*|\-\s*)\*\*(.+?)\*\*(?:：|:)?\s*(.*)', line)
                if m:
                    name = m.group(1).strip()
                    desc = m.group(2).strip()
                    # 跳过已完成项
                    if "✅" in line:
                        continue
                    # 跳过长期/观察项
                    if any(kw in name.lower() or kw in desc.lower()
                           for kw in BACKGROUND_NOISE_STATUSES):
                        continue
                    # 从节标题推断优先级
                    priority = "P2" if "待处理" in (in_section or "") else "P3"
                    workstations.append({
                        "priority": priority,
                        "name": name,
                        "status": desc if desc else "session遗留",
                        "source": "session",
                    })
    # pending 谱系
    for p in glob.glob(os.path.join(root, ".chanlun/genealogy/pending/*.md")):
        workstations.append({
            "priority": "P0",
            "name": f"pending:{os.path.basename(p)}",
            "status": "pending",
        })
    return session_name, workstations


def discover_business_tasks(root):
    """no_work_fallback：扫描测试失败、spec 合规等业务层任务。

    对应 dispatch-dag ceremony_sequence.no_work_fallback：
    "扫描 TODO/覆盖率/spec合规/谱系张力，产出至少一个工位"

    注意：测试验证不在此脚本中执行（只读扫描原则）。
    通过 test_verification_needed 信号通知 Lead spawn 独立测试工位。
    """
    tasks = []

    # 1. 测试验证：不在 scan 中执行 pytest（只读扫描原则）
    #    test_verification_needed 信号在顶层 JSON 中输出，Lead spawn 独立测试工位

    # 2. 测试覆盖率（如果 pytest-cov 可用）
    # 暂不实现，避免 scan 耗时过长

    return tasks


def derive_structural_workstations(root, phase):
    """220号下游推论：从 dispatch-dag event_skill_map 推导结构工位。

    phase="initial"：只推导首次扫描时可执行的结构工位
    phase="rescan"：推导 swarm_cycle_end 触发的结构工位（meta-observer、topology-analyst、gangju-audit）

    结构工位作为 ephemeral skill invocation 出现在 workstations 列表中（F3 已解决：075号定义成立）。
    """
    workstations = []

    if phase == "rescan":
        # meta-observer：swarm_cycle_end 触发（dispatch-dag structural skill）
        workstations.append({
            "priority": "P1",
            "name": "meta-observer",
            "type": "structural",
            "trigger": "swarm_cycle_end",
            "status": "structural:rescan",
            "source": "dispatch-dag:event_skill_map",
            "agent": ".claude/agents/meta-observer.md",
            "description": "二阶观察——规则触发/违反模式、语法记录候选、元规则一致性",
        })

        # topology-analyst：blocks 存在时触发（conditional skill）
        blocks_dir = os.path.join(root, ".chanlun/block-topology/blocks")
        if os.path.isdir(blocks_dir):
            block_count = len(glob.glob(os.path.join(blocks_dir, "*.json")))
            if block_count > 0:
                workstations.append({
                    "priority": "P2",
                    "name": "topology-analyst",
                    "type": "conditional",
                    "trigger": "blocks_exist",
                    "status": f"structural:rescan (blocks={block_count})",
                    "source": "dispatch-dag:event_skill_map",
                    "agent": ".claude/agents/topology-analyst.md",
                    "description": f"冷读 .chanlun/block-topology/（{block_count} 区块）",
                })

        # gangju-audit：gangju_analysis 只读函数检测 audit_needed
        audit_needed = _check_gangju_audit_needed(root)
        if audit_needed:
            workstations.append({
                "priority": "P1",
                "name": "gangju-audit",
                "type": "structural",
                "trigger": "audit_needed",
                "status": "structural:rescan",
                "source": "gangju_analysis:audit_needed",
                "description": "纲举目张审计——gemini-challenger(verify) + codex-challenger(review)",
            })

    return workstations


def _check_gangju_audit_needed(root):
    """从 gangju_analysis 导入只读函数，检测是否需要审计。

    只读原则：不调用 generate_pending_skeleton，只检测 audit_needed 条件。
    """
    try:
        import sys
        scripts_dir = os.path.join(root, "scripts")
        if scripts_dir not in sys.path:
            sys.path.insert(0, scripts_dir)
        import gangju_analysis as ga

        # 只读检测：提取纲、计算统计、判断 audit_needed
        gangs = ga.extract_gang(root)
        if not gangs:
            return False
        block_stats = ga.compute_block_stats(root)
        genealogy_stats = ga.compute_genealogy_stats(root)
        mu = ga.derive_mu(gangs, block_stats, genealogy_stats)
        return mu.get("audit_needed", False)
    except Exception:
        return False


def compute_concept_topology_health(root):
    """检测内容级概念拓扑健康状态。

    检查项：
    1. 未 enrich 的区块（有 event 区块但无 content_enrichment rewrite）
    2. 概念重复
    3. 概念冲突（引用了被修正的概念但不知道修正）
    4. 引用-依赖不一致

    返回健康报告 dict，异常时生成工位建议。
    """
    base = os.path.join(root, ".chanlun/block-topology")
    meta_path = os.path.join(base, "meta.json")

    if not os.path.isfile(meta_path):
        return None

    try:
        with open(meta_path, encoding="utf-8") as f:
            meta = json.load(f)
    except Exception:
        return None

    result = {}

    # 1. 未 enrich 的区块检测
    enrichment = meta.get("content_enrichment")
    if enrichment is None:
        total_blocks = meta.get("block_count", 0)
        if total_blocks > 0:
            result["unenriched_blocks"] = total_blocks
            result["enrichment_status"] = "not_run"
    else:
        result["enrichment_status"] = "completed"
        result["enrichment_stats"] = enrichment

    # 2-4. 运行概念拓扑检查（如果 enrichment 已完成）
    if enrichment is not None:
        try:
            import sys
            from pathlib import Path as _Path
            sys_path_added = False
            scripts_dir = os.path.join(root, "scripts")
            if scripts_dir not in sys.path:
                sys.path.insert(0, scripts_dir)
                sys_path_added = True
            from concept_topology_check import run_all_checks
            report = run_all_checks(_Path(base))
            result["concept_health"] = report.get("summary", {})
            if sys_path_added:
                sys.path.remove(scripts_dir)
        except Exception as exc:
            result["concept_health_error"] = f"{type(exc).__name__}: {exc}"

    return result


def detect_genealogy_anomalies(root):
    """检测谱系编号异常：重复编号、文件名编号与内部 id 不一致、dag.yaml 完整性、frontmatter schema。

    返回异常列表，每个元素包含 type、files、detail 字段。
    空列表 = 无异常。
    """
    settled_dir = os.path.join(root, ".chanlun/genealogy/settled/")
    if not os.path.isdir(settled_dir):
        return []

    anomalies = []
    # {number: [filename, ...]}
    num_to_files = {}
    # 标准 frontmatter 字段（至少应包含这些）
    required_frontmatter = {"id", "status", "type", "date"}

    for filepath in glob.glob(os.path.join(settled_dir, "*.md")):
        basename = os.path.basename(filepath)
        m = re.match(r'^(\d+)-(.+)\.md$', basename)
        if not m:
            continue
        file_num = int(m.group(1))

        # Track duplicate numbers
        num_to_files.setdefault(file_num, []).append(basename)

        # Check internal id consistency + frontmatter schema
        try:
            with open(filepath, encoding="utf-8") as f:
                head = f.read(1500)
            id_match = re.search(r'(?:^|\n)\s*\*?\*?id\*?\*?:\s*["\']?(\d+)["\']?', head)
            if id_match:
                internal_id = int(id_match.group(1))
                if internal_id != file_num:
                    anomalies.append({
                        "type": "id_mismatch",
                        "file": basename,
                        "detail": f"filename={file_num}, internal_id={internal_id}",
                    })

            # Frontmatter schema validation
            found_fields = set()
            for line in head.split("\n")[:20]:
                for field in required_frontmatter:
                    # Match both **field**: value and field: value patterns
                    if re.match(rf'^\s*\*?\*?{field}\*?\*?\s*:', line, re.IGNORECASE):
                        found_fields.add(field)
            missing = required_frontmatter - found_fields
            if missing:
                anomalies.append({
                    "type": "missing_frontmatter",
                    "file": basename,
                    "detail": f"缺少字段: {', '.join(sorted(missing))}",
                })
        except Exception:
            pass

    # Report duplicates
    for num, files in sorted(num_to_files.items()):
        if len(files) > 1:
            anomalies.append({
                "type": "duplicate_number",
                "number": num,
                "files": files,
                "detail": f"编号 {num} 被 {len(files)} 个文件使用",
            })

    # Block-topology completeness check: every settled file should have a
    # corresponding entry in meta.json id_mapping (178号-2 迁移：dag.yaml → block-topology)
    meta_path = os.path.join(root, ".chanlun/block-topology/meta.json")
    if os.path.isfile(meta_path):
        try:
            with open(meta_path, encoding="utf-8") as f:
                meta = json.load(f)
            # id_mapping keys are string genealogy ids (e.g. "001", "062")
            # Normalize: strip leading zeros for comparison with file_num_str (int)
            mapped_ids = set()
            for key in meta.get("id_mapping", {}).keys():
                try:
                    mapped_ids.add(str(int(key)))
                except ValueError:
                    mapped_ids.add(key)

            for file_num_str, filenames in num_to_files.items():
                if str(file_num_str) not in mapped_ids:
                    anomalies.append({
                        "type": "missing_block_mapping",
                        "file": filenames[0],
                        "detail": f"编号 {file_num_str} 在 settled/ 中存在但 block-topology 无对应映射",
                    })
        except Exception:
            pass

    return anomalies


def compute_delta_genealogy(root):
    """176号下游推论：检测 RTAS 循环是否产生新谱系（Δ谱系>0）。

    从最新 session 文件中提取上次记录的 settled 数，
    与当前 settled 数比较。如果 session 文件中没有记录，跳过检测。

    返回 dict 或 None（无法检测时）。
    """
    current_settled = len(glob.glob(os.path.join(root, ".chanlun/genealogy/settled/*.md")))

    # 从最新 session 文件中提取上次的 settled 数
    sessions = glob.glob(os.path.join(root, ".chanlun/sessions/*-session.md"))
    if not sessions:
        return None
    sessions.sort(key=os.path.getmtime)
    latest_session = sessions[-1]

    last_settled_count = None
    try:
        with open(latest_session, encoding="utf-8") as f:
            content = f.read()
        # 匹配 "已结算: N 个" 或 "- 已结算: N 个"
        m = re.search(r'已结算:\s*(\d+)\s*个', content)
        if m:
            last_settled_count = int(m.group(1))
    except Exception:
        pass

    if last_settled_count is None:
        return None

    delta = current_settled - last_settled_count
    result = {
        "last_session_settled_count": last_settled_count,
        "current_settled_count": current_settled,
        "delta": delta,
        "warning": "RTAS循环未产生新谱系" if delta == 0 else None,
    }
    return result


def compute_delta_blocks(root):
    """178号下游推论：检测 block-topology 区块变化。

    从 meta.json 读取迁移时区块数，与当前 blocks/ 目录下 *.json 文件数比较。
    区块系统是谱系的补充/升格，与 delta_genealogy 并存。
    """
    block_dir = os.path.join(root, ".chanlun/block-topology/blocks")
    meta_path = os.path.join(root, ".chanlun/block-topology/meta.json")

    # 当前区块数
    if os.path.isdir(block_dir):
        current_blocks = len(glob.glob(os.path.join(block_dir, "*.json")))
    else:
        current_blocks = 0

    # 迁移时区块数（从 meta.json）
    migration_block_count = 0
    if os.path.isfile(meta_path):
        try:
            with open(meta_path, "r", encoding="utf-8") as f:
                meta = json.load(f)
            migration_block_count = meta.get("block_count", 0)
        except Exception:
            pass

    delta = current_blocks - migration_block_count
    return {
        "migration_block_count": migration_block_count,
        "current_block_count": current_blocks,
        "delta": delta,
        "warning": "区块拓扑无新区块" if delta == 0 and current_blocks > 0 else None,
    }


def compute_advancement_candidates(root):
    """编排者战略决策：roadmap_awareness 层（纯信息，不生成工位）。

    scan 负责呈现材料，编排者负责解释/行动。
    manual_dispatch 是正式的编排者决策接口，不是绕过机制。

    来源：
    1. roadmap.yaml 已完成任务的下一步提示
    2. 最近 10 个已结算谱系中 downstream_inferences 标记为"选择"类的未处理项
    3. meta-observer 最近产出中的下游推论

    返回结构化列表，每个元素包含 source/direction/description/category 字段。
    category: choice（需编排者决策）| theory（理论推进）| engineering（工程推进）
    """
    candidates = []

    # 1. roadmap.yaml：已完成任务的 completion_note 中提取后续方向
    roadmap_path = os.path.join(root, ".chanlun/roadmap.yaml")
    if os.path.isfile(roadmap_path):
        try:
            with open(roadmap_path, encoding="utf-8") as f:
                data = yaml.safe_load(f)
            for task in data.get("tasks", []):
                if task.get("status") != "completed":
                    continue
                note = task.get("completion_note", "")
                # 从 completion_note 中提取"待后续"/"后续"/"下一步"等提示
                for line in note.split("\n"):
                    stripped = line.strip()
                    if any(kw in stripped for kw in ("待后续", "后续", "下一步", "TODO", "待")):
                        candidates.append({
                            "source": f"roadmap:{task.get('id', '?')}",
                            "direction": stripped,
                            "description": f"已完成任务 {task.get('title', '')} 的后续方向",
                            "category": "engineering",
                        })
        except Exception:
            pass

    # 2. 最近 10 个已结算谱系的 downstream_inferences 中未解决的选择类项
    settled_files = sorted(
        glob.glob(os.path.join(root, ".chanlun/genealogy/settled/*.md")),
        key=os.path.getmtime,
        reverse=True,
    )[:10]
    for settled_file in settled_files:
        try:
            with open(settled_file, encoding="utf-8") as f:
                head = f.read(4000)
            fm_match = re.match(r"^---\s*\n(.+?)\n---", head, re.DOTALL)
            if not fm_match:
                continue
            fm = yaml.safe_load(fm_match.group(1))
            if not isinstance(fm, dict):
                continue
            genealogy_id = str(fm.get("id", "?"))
            for di in fm.get("downstream_inferences", []):
                if not isinstance(di, dict):
                    continue
                status = str(di.get("status", "")).lower()
                # 状态字段可能包含括号内理由，如 "resolved（理由）"，取括号前关键词
                status_key = re.split(r'[（(]', status, maxsplit=1)[0].strip()
                if status_key in ("resolved", "已结算", "已修复"):
                    continue
                desc = di.get("description", "")
                # 分类：包含"选择"/"decide"/"方向"关键词的是 choice 类
                if any(kw in desc for kw in ("选择", "decide", "方向", "策略")):
                    cat = "choice"
                elif any(kw in desc for kw in ("形式化", "定义", "概念", "定理", "证明")):
                    cat = "theory"
                else:
                    cat = "engineering"
                candidates.append({
                    "source": f"genealogy:{genealogy_id}号-{di.get('id', '?')}",
                    "direction": desc[:200],
                    "description": f"谱系{genealogy_id}号下游推论（status={di.get('status', '?')}）",
                    "category": cat,
                })
        except Exception:
            pass

    # 3. meta-observer 最近产出（最近一个 meta-observation 类型谱系的下游推论）
    for settled_file in settled_files:
        try:
            with open(settled_file, encoding="utf-8") as f:
                head = f.read(4000)
            fm_match = re.match(r"^---\s*\n(.+?)\n---", head, re.DOTALL)
            if not fm_match:
                continue
            fm = yaml.safe_load(fm_match.group(1))
            if not isinstance(fm, dict):
                continue
            source = str(fm.get("source", ""))
            if "meta-observer" not in source:
                continue
            genealogy_id = str(fm.get("id", "?"))
            for di in fm.get("downstream_inferences", []):
                if not isinstance(di, dict):
                    continue
                status = str(di.get("status", "")).lower()
                # 状态字段可能包含括号内理由，如 "resolved（理由）"，取括号前关键词
                status_key = re.split(r'[（(]', status, maxsplit=1)[0].strip()
                if status_key in ("resolved", "已结算", "已修复"):
                    continue
                # 避免与上面的谱系扫描重复
                candidate_id = f"meta-observer:{genealogy_id}号-{di.get('id', '?')}"
                if any(c["source"] == candidate_id for c in candidates):
                    continue
                desc = di.get("description", "")
                candidates.append({
                    "source": candidate_id,
                    "direction": desc[:200],
                    "description": f"meta-observer {genealogy_id}号下游推论（status={di.get('status', '?')}）",
                    "category": "choice" if any(kw in desc for kw in ("选择", "decide", "方向")) else "theory",
                })
            break  # 只取最近一个 meta-observation
        except Exception:
            pass

    return candidates


def main():
    parser = argparse.ArgumentParser(description="蜂群 spawn 通用工具")
    parser.add_argument("--skills", action="store_true", help="只输出 required_skills")
    # 保留 --structural 作为 --skills 的别名（向后兼容）
    parser.add_argument("--structural", action="store_true", help="(deprecated) 等同于 --skills")
    parser.add_argument("--workstations", nargs="*", help="指定业务工位名称")
    parser.add_argument("--workstations-json", type=str, help="JSON 字符串，解析为结构化工位列表")
    parser.add_argument("--phase", choices=["initial", "rescan"], default="initial",
                        help="扫描阶段：initial（首次）或 rescan（工位完成后）")
    args = parser.parse_args()

    root = os.getcwd()
    result = {"required_skills": get_required_skills(root)}

    if args.skills or args.structural:
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return

    if args.workstations:
        result["workstations"] = [{"name": w, "priority": "P1", "status": "pending"} for w in args.workstations]
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return

    if args.workstations_json:
        try:
            ws_list = json.loads(args.workstations_json)
            if not isinstance(ws_list, list):
                ws_list = [ws_list]
            # 每个元素可以是 dict（结构化工位）或 str（名称）
            workstations = []
            for item in ws_list:
                if isinstance(item, dict):
                    # 确保必要字段存在
                    item.setdefault("priority", "P1")
                    item.setdefault("status", "pending")
                    workstations.append(item)
                elif isinstance(item, str):
                    workstations.append({"name": item, "priority": "P1", "status": "pending"})
            result["workstations"] = workstations
        except json.JSONDecodeError as exc:
            result["workstations"] = []
            result["workstations_json_error"] = f"JSON 解析失败: {exc}"
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return

    # 根 ceremony 模式：全量扫描
    # 扫描来源（按发现策略排列，不是工位间的执行排序）：
    # 1. roadmap.yaml（结构化业务目标——最先扫描的来源）
    # 2. session 遗留项 + pending 谱系
    # 3. no_work_fallback（测试失败等）
    # 274号：priority 字段是工位的来源标记，不产生全局排序效力。
    #   Lead spawn 工位时全部并行（218号要求），不按 priority 排序。
    # 081号：只有 roadmap 为空 AND session 遗留为空 AND fallback 为空，才是真阴性干净终止

    session_name, workstations = get_session_workstations(root)
    result["mode"] = "warm_start" if session_name else "cold_start"
    result["session"] = session_name

    # 1. 首先扫描：roadmap.yaml 中的 active 任务
    roadmap_tasks = get_roadmap_workstations(root)
    # VDW：分离 auto_verified 任务（不生成工位）和需要工位的任务
    verified_tasks = [t for t in roadmap_tasks if t.get("auto_verified")]
    unverified_tasks = [t for t in roadmap_tasks if not t.get("auto_verified")]
    if verified_tasks:
        result["auto_verified"] = [
            {"name": t["name"], "status": "auto_verified"} for t in verified_tasks
        ]
    if unverified_tasks:
        # roadmap 任务插入到 workstations 最前（列表拼接顺序≠执行排序，274号：Lead 全部并行 spawn）
        workstations = unverified_tasks + workstations
    result["roadmap_tasks_found"] = len(roadmap_tasks)
    result["roadmap_tasks_verified"] = len(verified_tasks)

    # 147号：扫描 frozen 节点，过滤依赖 frozen 节点的工位
    frozen_nodes = get_frozen_nodes(root)
    if frozen_nodes:
        result["frozen_nodes"] = sorted(frozen_nodes)
        # 过滤掉名称中包含 frozen 节点 id 的工位（冻结路径下游不 spawn）
        pre_filter_count = len(workstations)
        workstations = [
            w for w in workstations
            if not any(fid in w.get("name", "") for fid in frozen_nodes)
        ]
        filtered_count = pre_filter_count - len(workstations)
        if filtered_count > 0:
            result["frozen_filtered_count"] = filtered_count

    # 147号：扫描谱系 topo_effect 字段（审查结果回到 ceremony_scan）
    topo_effects = get_topo_effects_from_genealogy(root)
    if topo_effects:
        result["topo_effects"] = topo_effects

    # 177号：检测未执行的结构化 topo_effect（RTAS 循环待执行项）
    pending_topo = detect_pending_topo_effects(root)
    if pending_topo:
        result["pending_topo_effects"] = pending_topo
        for pt in pending_topo:
            workstations.append({
                "priority": "P0",
                "name": f"topo_effect待执行：{pt['id']}号-{pt['topo_effect']}",
                "status": f"pending_topo: {pt['topo_effect']}",
                "source": "pending_topo_effect",
            })

    # 2. 079号：如果 session 遗留工位为空或全部是背景噪音，执行 no_work_fallback
    # 081号修正：no_work_fallback 仅在 roadmap 也为空时才触发（roadmap 是主工作来源）
    if not workstations:
        workstations = discover_business_tasks(root)
        result["fallback_triggered"] = True

    result["workstations"] = workstations
    result["test_verification_needed"] = True  # Lead spawn 独立测试工位
    result["phase"] = args.phase

    # 220号：结构工位推导（phase 决定触发条件）
    structural_ws = derive_structural_workstations(root, args.phase)
    if structural_ws:
        workstations.extend(structural_ws)
        result["structural_workstations"] = [w["name"] for w in structural_ws]

    # 283号缺口C：incomplete team init 扫描
    # 插入点：structural_ws 追加之后、suspended 过滤之前
    try:
        from scripts.ceremony_state import get_incomplete_team_inits
    except ImportError:
        from ceremony_state import get_incomplete_team_inits
    incomplete_teams = get_incomplete_team_inits()
    if incomplete_teams:
        result["incomplete_team_inits"] = incomplete_teams
        for team_name in incomplete_teams:
            workstations.append({
                "priority": "P0",
                "name": f"team_init_incomplete:{team_name}",
                "status": "team init 未完成，需要清理或重试",
                "source": "team_init_audit",
            })

    # pending 谱系计数（多处使用）
    pending_count = len(glob.glob(os.path.join(root, ".chanlun/genealogy/pending/*.md")))

    # 定义/谱系计数
    defs_path = os.path.join(root, "definitions.yaml")
    if os.path.isfile(defs_path):
        with open(defs_path, encoding="utf-8") as f:
            defs = yaml.safe_load(f)
        if isinstance(defs, dict):
            entities = defs.get("entities", defs.get("definitions", []))
            result["definitions"] = len(entities) if isinstance(entities, list) else 0
    result["pending"] = pending_count
    result["settled"] = len(glob.glob(os.path.join(root, ".chanlun/genealogy/settled/*.md")))

    # 176号：Δ谱系>0 检测——RTAS循环是否产生新谱系
    delta_genealogy = compute_delta_genealogy(root)
    if delta_genealogy is not None:
        result["delta_genealogy"] = delta_genealogy

    # 178号下游推论：Δ区块检测——block-topology 是否有新区块
    result["delta_blocks"] = compute_delta_blocks(root)

    # 内容级概念拓扑健康检测
    concept_health = compute_concept_topology_health(root)
    if concept_health is not None:
        result["concept_topology_health"] = concept_health
        # 异常时生成工位
        health_status = concept_health.get("concept_health", {}).get("health")
        if health_status == "issues_found":
            summary = concept_health.get("concept_health", {})
            workstations.append({
                "priority": "P2",
                "name": f"概念拓扑异常：{summary.get('duplicates', 0)}重复/{summary.get('conflicts', 0)}冲突/{summary.get('missing_dependencies', 0)}遗漏依赖",
                "status": "concept_topology:issues_found",
                "source": "concept_topology_check",
            })
        elif concept_health.get("enrichment_status") == "not_run":
            workstations.append({
                "priority": "P2",
                "name": f"内容级迁移未执行：{concept_health.get('unenriched_blocks', 0)}个区块待 enrich",
                "status": "content_enrichment:not_run",
                "source": "concept_topology_check",
            })

    # 081号下游推论：pattern-buffer 达标模式扫描（分片版）
    pb_dir = os.path.join(root, ".chanlun/pattern-buffer")
    pb_legacy = os.path.join(root, ".chanlun/pattern-buffer.yaml")
    pb_shard_files = []
    if os.path.isdir(pb_dir):
        for shard_name in ("candidates.yaml", "topo-anomalies.yaml"):
            shard_path = os.path.join(pb_dir, shard_name)
            if os.path.isfile(shard_path):
                pb_shard_files.append(shard_path)
    # 向后兼容：旧的单文件
    if os.path.isfile(pb_legacy):
        pb_shard_files.append(pb_legacy)

    candidates = []
    for pb_path in pb_shard_files:
        try:
            with open(pb_path, encoding="utf-8") as f:
                pb = yaml.safe_load(f)
            candidates.extend(
                p for p in pb.get("patterns", [])
                if p.get("status") == "candidate"
            )
        except Exception:
            pass
    if candidates:
        result["pattern_buffer_candidates"] = len(candidates)
        # 达标候选产生结晶工位
        workstations.append({
            "priority": "P1",
            "name": f"结晶检测：{len(candidates)}个candidate模式",
            "status": "pattern-buffer:candidate",
            "source": "pattern_buffer",
        })

    # 153号下游推论：蜂群持久化断裂检测
    swarm_gaps = detect_swarm_persistence_gaps(root)
    if swarm_gaps:
        result["swarm_persistence_gaps"] = swarm_gaps
        result["swarm_persistence_warning"] = (
            f"{len(swarm_gaps)} 个蜂群有谱系产出但无 session: "
            + ", ".join(f"v{g['swarm_id']}-swarm" for g in swarm_gaps)
        )

    # 谱系编号异常检测（编号冲突自动发现 + 修复工位生成）
    genealogy_anomalies = detect_genealogy_anomalies(root)
    if genealogy_anomalies:
        result["genealogy_anomalies"] = genealogy_anomalies
        workstations.append({
            "priority": "P0",
            "name": f"谱系编号异常：{len(genealogy_anomalies)}项",
            "status": "; ".join(a["detail"] for a in genealogy_anomalies),
            "source": "genealogy_anomaly_detection",
        })

    # 081号下游推论：谱系张力扫描（tensions_with 边检查）
    tensions_found = []
    for settled_file in glob.glob(os.path.join(root, ".chanlun/genealogy/settled/*.md")):
        try:
            with open(settled_file, encoding="utf-8") as f:
                first_lines = f.read(2000)
            # 快速检查 YAML frontmatter 中的 tensions_with
            if "tensions_with:" in first_lines and "tensions_with: []" not in first_lines:
                # 提取文件名作为标识
                fname = os.path.basename(settled_file)
                tensions_found.append(fname)
        except Exception:
            pass
    if tensions_found:
        result["tensions_count"] = len(tensions_found)

    # 二阶反馈：下游推论执行审计
    try:
        from downstream_audit import audit as downstream_audit
        da = downstream_audit(root)
        if da["total_actions"] > 0:
            result["downstream_actions"] = {
                "total": da["total_actions"],
                "unresolved": da["unresolved"],
                "execution_rate": da["execution_rate"],
            }
            # 将 unresolved 下游行动转化为工位
            # 动态窗口：<=10 个 unresolved 时全部纳入，>10 个时只取最近 10 个
            all_unresolved = [
                item for item in da.get("items", [])
                if item.get("status") == "unresolved"
                and item.get("genealogy_id", "0").isdigit()
            ]
            if len(all_unresolved) <= 10:
                recent_unresolved = all_unresolved
            else:
                # 按 genealogy_id 降序取最近 10 个
                all_unresolved.sort(
                    key=lambda x: int(x["genealogy_id"]), reverse=True
                )
                recent_unresolved = all_unresolved[:10]
            for item in recent_unresolved:
                workstations.append({
                    "priority": "P2",
                    "name": f"下游推论：{item['genealogy_id']}号-{item['action_index']}",
                    "status": f"unresolved: {item['text'][:80]}",
                    "source": "downstream_audit",
                })
    except Exception as exc:
        # Keep scan resilient, but do not hide failures.
        result["downstream_actions_error"] = f"{type(exc).__name__}: {exc}"

    # 183号目B：异步自指审计（t 审查 t-1）
    try:
        try:
            import scripts.async_self_reference as _asr_mod
        except ImportError:
            import async_self_reference as _asr_mod
        asr = _asr_mod.audit(root)
        result["async_self_ref"] = {
            "t_minus_1_summary": asr.get("t_minus_1_summary"),
            "findings_count": len(asr.get("self_audit_findings", [])),
            "genealogy_needed": asr.get("genealogy_needed", False),
            "findings": asr.get("self_audit_findings", []),
        }
        for finding in asr.get("self_audit_findings", []):
            if finding["type"] in ("stagnation", "anomaly"):
                workstations.append({
                    "priority": "P1",
                    "name": f"异步自指审计：{finding['type']}",
                    "status": finding["detail"][:120],
                    "source": "async_self_ref",
                })
        if asr.get("genealogy_needed") and not any(
            w.get("source") == "async_self_ref" for w in workstations
        ):
            workstations.append({
                "priority": "P2",
                "name": "异步自指审计：需要新谱系",
                "status": "genealogy_needed=true",
                "source": "async_self_ref",
            })
    except Exception as exc:
        result["async_self_ref_error"] = f"{type(exc).__name__}: {exc}"

    # 编排者战略决策：roadmap_awareness 层（纯信息，不生成工位）
    # manual_dispatch 是正式的编排者决策接口，不是绕过机制
    # scan 负责呈现材料，编排者负责解释/行动
    advancement_candidates = compute_advancement_candidates(root)
    if advancement_candidates:
        result["advancement_candidates"] = advancement_candidates
        result["roadmap_awareness_note"] = (
            "advancement_candidates 是纯信息。"
            "编排者通过 manual_dispatch 选择执行哪些方向。"
            "scan 不做 dispatch 决策。"
        )

    # 270号：suspended 工位过滤——从 ceremony_state 读取 suspended 列表，
    # 匹配 workstations 中的 name 字段，过滤掉 suspended 工位
    try:
        from scripts.ceremony_state import get_suspended_workstations
    except ImportError:
        from ceremony_state import get_suspended_workstations
    suspended = get_suspended_workstations()
    if suspended:
        result["suspended_workstations"] = suspended
        pre_suspend_count = len(workstations)
        workstations = [
            w for w in workstations
            if w.get("name", "") not in suspended
        ]
        suspended_filtered = pre_suspend_count - len(workstations)
        if suspended_filtered > 0:
            result["suspended_filtered_count"] = suspended_filtered
        # workstations 引用已更新——同步到 result
        result["workstations"] = workstations

    # 081号：清晰报告干净终止条件（必须在所有 workstations 追加完成后计算）
    # 真阴性干净终止 = roadmap 为空 AND workstations 为空 AND pending 谱系为空
    result["clean_terminate"] = (
        len(roadmap_tasks) == 0
        and len(workstations) == 0
        and pending_count == 0
    )

    try:
        result["head"] = subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"], cwd=root, text=True).strip()
    except Exception:
        pass

    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
