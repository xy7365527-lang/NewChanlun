#!/usr/bin/env python
"""蜂群 spawn 通用工具。任何节点（ceremony 或 teammate）spawn 蜂群时调用。

用法:
  python scripts/ceremony_scan.py                  # 根 ceremony：全量扫描
  python scripts/ceremony_scan.py --skills         # 只输出 required_skills
  python scripts/ceremony_scan.py --workstations "任务A" "任务B"  # 指定业务工位

075号更新：structural_nodes → required_skills（事件驱动 skill 架构）
079号更新：background_noise 降级 + 业务层任务发现（no_work_fallback）
081号更新：roadmap.yaml 扫描（最高优先级任务来源）+ 终止逻辑修正
089号声明：当前为硬编码优先级扫描，不是 DAG 拓扑排序。
147号-2更新：审查结果整合——基因组修改的异步审查结果回到 ceremony_scan，不回到编排者。
    dispatch-dag.yaml 的 ceremony_sequence 定义了 DAG 格式的 nodes+depends_on，
    但本脚本并未实现 DAG 解析器——扫描顺序由代码逻辑决定（roadmap → session → fallback）。
    DAG 的 ceremony_sequence 由 LLM 解释执行（057号推论：LLM 不是状态机）。
    未来演化路径：重写为真正读取 ceremony_sequence 的 DAG 拓扑排序（选项 C 边界条件）。
"""
import json, os, glob, yaml, sys, argparse, subprocess, re


BACKGROUND_NOISE_STATUSES = {"background_noise", "观察项", "背景噪音"}
TERMINAL_STATUSES = {"已修复", "resolved", "background_noise"}


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


def get_roadmap_workstations(root):
    """从 .chanlun/roadmap.yaml 读取待执行的业务目标（最高优先级任务来源）。

    081号谱系：roadmap 是结构化业务目标载体，ceremony_scan 的最高优先级来源。
    只返回 status=active 的任务。
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
                tasks.append({
                    "priority": task.get("priority", "P2"),
                    "name": task.get("id", task.get("title", "未命名")),
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
            # 格式A：表格行（旧格式）
            if line.startswith("|") and "P" in line:
                parts = [c.strip() for c in line.split("|") if c.strip()]
                if len(parts) >= 3:
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
                import re
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
    """
    tasks = []

    # 默认关闭重型 pytest 扫描，避免 /ceremony 常态 10s+ 阻塞。
    # 需要时显式开启：
    #   CEREMONY_SCAN_ENABLE_PYTEST_FALLBACK=1
    enable_pytest = os.environ.get("CEREMONY_SCAN_ENABLE_PYTEST_FALLBACK", "0") == "1"
    if not enable_pytest:
        return tasks

    # 无 tests 目录时无需触发 pytest。
    if not os.path.isdir(os.path.join(root, "tests")):
        return tasks

    # 1. 测试失败扫描
    try:
        pytest_timeout = int(os.environ.get("CEREMONY_SCAN_PYTEST_TIMEOUT", "5"))
        result = subprocess.run(
            [sys.executable, "-m", "pytest",
             "--maxfail=1",
             "--ignore=tests/test_cli_gateway_plot.py",
             "--ignore=tests/test_data_databento.py",
             "--ignore=tests/test_mcp_bridge.py",
              "--tb=no", "-q"],
            cwd=root, capture_output=True, text=True, timeout=pytest_timeout,
        )
        output = result.stdout + result.stderr
        # 解析 "N failed" 行
        for line in output.split("\n"):
            if "failed" in line and ("passed" in line or "error" in line):
                parts = line.split(",")
                for part in parts:
                    part = part.strip()
                    if "failed" in part:
                        count = part.split()[0]
                        tasks.append({
                            "priority": "P1",
                            "name": f"修复 {count} 个测试失败",
                            "status": "pytest 发现",
                            "source": "test_failures",
                        })
                        break
    except Exception:
        pass

    # 2. 测试覆盖率（如果 pytest-cov 可用）
    # 暂不实现，避免 scan 耗时过长

    return tasks


def get_pattern_buffer_candidate_count(root):
    """统计 pattern-buffer 中 status=candidate 的模式数量。

    兼容两种存储形态：
    1) 分片目录 `.chanlun/pattern-buffer/*.yaml`
    2) 旧单文件 `.chanlun/pattern-buffer.yaml`
    """
    # 优先单文件：与现有 hook 写入路径保持一致（低开销、低风险）
    pb_path = os.path.join(root, ".chanlun/pattern-buffer.yaml")
    if os.path.isfile(pb_path):
        try:
            with open(pb_path, encoding="utf-8") as f:
                pb = yaml.safe_load(f) or {}
            patterns = pb.get("patterns", [])
            if isinstance(patterns, list):
                return sum(
                    1
                    for p in patterns
                    if isinstance(p, dict) and p.get("status") == "candidate"
                )
        except Exception:
            return 0

    # 回退：分片目录（当单文件不存在时）
    shard_dir = os.path.join(root, ".chanlun/pattern-buffer")
    total = 0
    if os.path.isdir(shard_dir):
        for shard in glob.glob(os.path.join(shard_dir, "*.yaml")):
            try:
                with open(shard, encoding="utf-8") as f:
                    data = yaml.safe_load(f) or {}
                patterns = data.get("patterns", [])
                if isinstance(patterns, list):
                    total += sum(
                        1
                        for p in patterns
                        if isinstance(p, dict) and p.get("status") == "candidate"
                    )
            except Exception:
                continue
    if total > 0:
        return total

    return 0


def get_review_results(root):
    """扫描 .chanlun/review-results/ 中的未消费审查结果。

    147号-2（下游推论）：基因组修改产生的异步审查结果回到下一轮 ceremony_scan，
    不回到编排者。审查结果被消费后标记为 consumed，避免重复处理。

    审查结果文件格式（YAML）：
      status: pending | consumed
      source: 产出者标识（如 gemini-challenger、meta-observer）
      target: 被审查文件路径
      timestamp: ISO 时间戳
      findings:
        - severity: conflict | warning | info
          description: 发现描述
          related_genealogy: 相关谱系编号（可选）
    """
    review_dir = os.path.join(root, ".chanlun/review-results")
    results = []
    consumed_files = []

    if not os.path.isdir(review_dir):
        return results, consumed_files

    for review_file in sorted(glob.glob(os.path.join(review_dir, "*.yaml"))):
        try:
            with open(review_file, encoding="utf-8") as f:
                data = yaml.safe_load(f)
            if not isinstance(data, dict):
                continue
            if data.get("status") != "pending":
                continue

            findings = data.get("findings", [])
            if not isinstance(findings, list) or not findings:
                continue

            results.append({
                "file": os.path.basename(review_file),
                "source": data.get("source", "unknown"),
                "target": data.get("target", ""),
                "timestamp": data.get("timestamp", ""),
                "findings": findings,
            })

            # 标记为已消费：将 status 从 pending 改为 consumed
            with open(review_file, encoding="utf-8") as f:
                content = f.read()
            updated = content.replace("status: pending", "status: consumed", 1)
            with open(review_file, "w", encoding="utf-8") as f:
                f.write(updated)
            consumed_files.append(os.path.basename(review_file))

        except Exception:
            continue

    return results, consumed_files


def get_topo_context(root):
    """扫描区块拓扑映射缺口：已结算谱系 vs block-topology 中已有映射。

    返回 topo_context dict，包含 unmapped_count, unmapped_ids, last_mapped。
    拓扑扫描失败不阻塞 ceremony 主流程。
    """
    try:
        meta_path = os.path.join(root, ".chanlun/block-topology/meta.json")
        settled_dir = os.path.join(root, ".chanlun/genealogy/settled")

        if not os.path.isfile(meta_path) or not os.path.isdir(settled_dir):
            return None

        with open(meta_path, encoding="utf-8") as f:
            meta = json.load(f)

        id_mapping = meta.get("id_mapping", {})
        last_mapped = meta.get("last_mapped_genealogy", 0)
        mapped_ids = set(id_mapping.keys())

        # 从已结算谱系文件名提取编号（格式：NNN-title.md 或 NNNa-title.md）
        settled_ids = set()
        for fname in os.listdir(settled_dir):
            if not fname.endswith(".md"):
                continue
            m = re.match(r'^(\d+[a-z]?)-', fname)
            if m:
                settled_ids.add(m.group(1))

        unmapped_ids = sorted(
            settled_ids - mapped_ids,
            key=lambda x: (int(re.match(r'\d+', x).group()), x),
        )

        return {
            "unmapped_count": len(unmapped_ids),
            "unmapped_ids": unmapped_ids,
            "last_mapped": last_mapped,
            "total_settled": len(settled_ids),
            "total_mapped": len(mapped_ids),
        }
    except Exception as exc:
        return {"error": f"{type(exc).__name__}: {exc}"}


def _check_completion(root, check):
    """检查单个 completion_check 条件是否满足。

    支持的 check 类型：
    - file_exists: 文件是否存在
    - script_exists: 脚本文件是否存在（等同于 file_exists）
    - genealogy_settled: settled 目录中是否存在包含关键词的文件
    - test_pass: 测试文件是否存在（不执行测试，避免阻塞扫描）
    """
    if not isinstance(check, dict):
        return False
    check_type = check.get("type", "")
    if check_type in ("file_exists", "script_exists"):
        path = check.get("path", "")
        return os.path.isfile(os.path.join(root, path))
    if check_type == "genealogy_settled":
        keyword = check.get("keyword", "")
        settled_dir = os.path.join(root, ".chanlun/genealogy/settled")
        if not os.path.isdir(settled_dir):
            return False
        for fname in os.listdir(settled_dir):
            if keyword in fname:
                return True
        return False
    if check_type == "test_pass":
        # 只检查测试文件存在（不执行——避免扫描阻塞）
        pattern = check.get("pattern", "")
        return os.path.isfile(os.path.join(root, pattern))
    return False


def _scan_research_lines(root):
    """扫描 gangmu.yaml（纲目），为 active 目的 unblocked next_actions 生成工位。

    gangmu.yaml 结构：gang[] → mu[] 两层嵌套。
    每个 mu（目）等价于旧 research-lines.yaml 中的一条 line。
    纲（gang）提供战略方向归属信息。

    返回 (research_lines_context, new_workstations) 元组。
    research_lines_context 包含 active/blocked/proposed_transitions 信息，
    每个条目增加 gang_id 和 gang_name 字段。
    new_workstations 包含需要追加到工位列表的项。

    纲目扫描失败不阻塞主流程。
    """
    gm_path = os.path.join(root, ".chanlun/gangmu.yaml")
    if not os.path.isfile(gm_path):
        return None, []

    with open(gm_path, encoding="utf-8") as f:
        data = yaml.safe_load(f)

    if not isinstance(data, dict):
        return None, []

    gangs = data.get("gang", [])
    if not isinstance(gangs, list):
        return None, []

    active_lines = []
    blocked_lines = []
    proposed_transitions = []
    new_workstations = []

    for gang in gangs:
        if not isinstance(gang, dict):
            continue
        gang_id = gang.get("id", "")
        gang_name = gang.get("name", gang_id)

        mus = gang.get("mu", [])
        if not isinstance(mus, list):
            continue

        for mu in mus:
            if not isinstance(mu, dict):
                continue
            mu_id = mu.get("id", "")
            mu_name = mu.get("name", mu_id)
            status = mu.get("status", "")

            if status == "blocked":
                blocked_lines.append({
                    "id": mu_id,
                    "name": mu_name,
                    "gang_id": gang_id,
                    "gang_name": gang_name,
                    "blocked_by": mu.get("blocked_by", ""),
                })
                continue

            if status != "active":
                continue

            # 处理 active 目
            next_actions = mu.get("next_actions", [])
            if not isinstance(next_actions, list):
                continue

            unblocked_count = 0
            all_completed = True
            all_blocked = True

            for action in next_actions:
                if not isinstance(action, dict):
                    continue

                action_blocked_by = action.get("blocked_by")
                is_blocked = action_blocked_by is not None and action_blocked_by != ""

                if is_blocked:
                    all_completed = False
                    continue

                # unblocked action——检查 completion_check
                all_blocked = False
                completion_check = action.get("completion_check")
                completed = _check_completion(root, completion_check)

                if completed:
                    continue

                # 未完成的 unblocked action → 生成工位
                all_completed = False
                unblocked_count += 1
                action_type = action.get("type", "engineering")
                target = action.get("target", "未命名")
                description = action.get("description", "")

                new_workstations.append({
                    "priority": "P2",
                    "name": f"纲[{gang_name}]目[{mu_id}]：{target}",
                    "status": f"research_line:{action_type}",
                    "source": "research_lines",
                    "description": description,
                    "research_line": mu_id,
                    "gang_id": gang_id,
                    "gang_name": gang_name,
                })

            active_lines.append({
                "id": mu_id,
                "name": mu_name,
                "gang_id": gang_id,
                "gang_name": gang_name,
                "unblocked_actions": unblocked_count,
            })

            # 状态转换提议：所有 next_actions 都 blocked → 提议 active→blocked
            if next_actions and all_blocked:
                proposed_transitions.append({
                    "line": mu_id,
                    "gang": gang_id,
                    "from": "active",
                    "to": "blocked",
                    "reason": "所有 next_actions 均被阻塞",
                })

            # 状态转换提议：所有 next_actions 都完成 → 提议 active→closed
            if next_actions and all_completed and not all_blocked:
                proposed_transitions.append({
                    "line": mu_id,
                    "gang": gang_id,
                    "from": "active",
                    "to": "closed",
                    "reason": "所有 next_actions 的 completion_check 均已满足",
                })

    context = {
        "active": active_lines,
        "blocked": blocked_lines,
    }
    if proposed_transitions:
        context["proposed_transitions"] = proposed_transitions

    return context, new_workstations


def get_encounter_context(root):
    """偶遇检测：检查最近写入的谱系是否有跨纲偶遇候选。

    341号偶遇标准：迫使修正已结算理解的关联（不绑定时间性）。
    v150 Layer 1 结果：107条跨纲边全部设计内引用，0偶遇。

    增量检测逻辑：
    1. 读取 encounter-records.yaml 的 last_checked_id
    2. 对 last_checked_id 之后的所有谱系执行跨纲边检测
    3. 已知设计内模式自动过滤，未匹配模式标记为 encounter_candidate
    4. 候选追加到 encounter-records.yaml

    偶遇检测失败不阻塞 ceremony 主流程。
    """
    try:
        settled_dir = os.path.join(root, ".chanlun/genealogy/settled")
        records_path = os.path.join(root, ".chanlun/encounter-records.yaml")
        gangmu_path = os.path.join(root, ".chanlun/gangmu.yaml")

        if not os.path.isdir(settled_dir) or not os.path.isfile(gangmu_path):
            return None

        # 读取上次检测位置
        last_checked = 0
        if os.path.isfile(records_path):
            with open(records_path, encoding="utf-8") as f:
                records_data = yaml.safe_load(f) or {}
            lc = records_data.get("last_checked")
            if lc is not None:
                try:
                    last_checked = int(lc)
                except (ValueError, TypeError):
                    last_checked = 0

        # 找到 last_checked 之后的谱系
        new_ids = []
        for fname in os.listdir(settled_dir):
            if not fname.endswith(".md"):
                continue
            m = re.match(r'^(\d+)', fname)
            if m:
                nid = int(m.group(1))
                if nid > last_checked:
                    new_ids.append(nid)

        if not new_ids:
            return {
                "last_checked": last_checked,
                "new_genealogies": 0,
                "total_cross_gang_edges": 0,
                "total_encounter_candidates": 0,
                "status": "up_to_date",
            }

        # 调用 check_encounter.py 的逻辑（内联避免 subprocess 开销）
        result = subprocess.run(
            [sys.executable, os.path.join(root, "scripts/check_encounter.py"),
             "--since", str(last_checked)],
            cwd=root, capture_output=True, text=True, timeout=10,
        )

        if result.returncode not in (0, 1):
            return {"error": f"check_encounter.py failed: {result.stderr[:200]}"}

        check_output = json.loads(result.stdout)

        return {
            "last_checked": last_checked,
            "new_genealogies": check_output.get("checked_count", 0),
            "total_cross_gang_edges": check_output.get("total_cross_gang_edges", 0),
            "total_design_internal": check_output.get("total_design_internal", 0),
            "total_encounter_candidates": check_output.get("total_encounter_candidates", 0),
            "status": "candidates_found" if check_output.get("total_encounter_candidates", 0) > 0 else "no_encounters",
        }

    except Exception as exc:
        return {"error": f"{type(exc).__name__}: {exc}"}


def main():
    parser = argparse.ArgumentParser(description="蜂群 spawn 通用工具")
    parser.add_argument("--skills", action="store_true", help="只输出 required_skills")
    # 保留 --structural 作为 --skills 的别名（向后兼容）
    parser.add_argument("--structural", action="store_true", help="(deprecated) 等同于 --skills")
    parser.add_argument("--workstations", nargs="*", help="指定业务工位名称")
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

    # 根 ceremony 模式：全量扫描
    # 扫描顺序（优先级递减）：
    # 1. roadmap.yaml（最高优先级，结构化业务目标）
    # 2. session 遗留项 + pending 谱系
    # 3. no_work_fallback（测试失败等）
    # 081号：只有 roadmap 为空 AND session 遗留为空 AND fallback 为空，才是真阴性干净终止

    session_name, workstations = get_session_workstations(root)
    result["mode"] = "warm_start" if session_name else "cold_start"
    result["session"] = session_name

    # 1. 最高优先级：roadmap.yaml 中的 active 任务
    roadmap_tasks = get_roadmap_workstations(root)
    if roadmap_tasks:
        # roadmap 任务插入到 workstations 最前（P2 优先级，高于 P3 long_term）
        workstations = roadmap_tasks + workstations
        result["roadmap_tasks_found"] = len(roadmap_tasks)

    # 2. pattern-buffer 达标模式扫描（放在 fallback 前，避免误触发重型扫描）
    pattern_candidates = get_pattern_buffer_candidate_count(root)
    if pattern_candidates > 0:
        result["pattern_buffer_candidates"] = pattern_candidates
        workstations.append({
            "priority": "P1",
            "name": f"结晶检测：{pattern_candidates}个candidate模式",
            "status": "pattern-buffer:candidate",
            "source": "pattern_buffer",
        })

    # 2b. 147号-2：审查结果整合——基因组修改的异步审查结果纳入工位推导
    review_results, consumed_files = get_review_results(root)
    if review_results:
        result["review_results"] = {
            "count": len(review_results),
            "consumed_files": consumed_files,
            "items": review_results,
        }
        for review in review_results:
            # 将 conflict 级别的发现转化为 P1 工位，warning 转化为 P2
            conflict_count = sum(
                1 for f in review["findings"]
                if isinstance(f, dict) and f.get("severity") == "conflict"
            )
            warning_count = sum(
                1 for f in review["findings"]
                if isinstance(f, dict) and f.get("severity") == "warning"
            )
            if conflict_count > 0:
                workstations.append({
                    "priority": "P1",
                    "name": f"审查矛盾：{review['target']}（{conflict_count}个conflict）",
                    "status": f"review:{review['source']}",
                    "source": "review_results",
                    "review_file": review["file"],
                })
            if warning_count > 0:
                workstations.append({
                    "priority": "P2",
                    "name": f"审查警告：{review['target']}（{warning_count}个warning）",
                    "status": f"review:{review['source']}",
                    "source": "review_results",
                    "review_file": review["file"],
                })

    # 2c. 区块拓扑映射缺口扫描：已结算谱系 vs block-topology 映射
    topo_context = get_topo_context(root)
    if topo_context is not None:
        result["topo_context"] = topo_context
        unmapped_count = topo_context.get("unmapped_count", 0)
        if unmapped_count > 0:
            workstations.append({
                "priority": "P2",
                "name": f"拓扑映射：{unmapped_count}个未映射谱系",
                "status": f"unmapped:{','.join(topo_context['unmapped_ids'][:10])}",
                "source": "topo_mapper",
            })

    # 2d. 偶遇检测：检查最近谱系的跨纲边是否有偶遇候选
    encounter_context = get_encounter_context(root)
    if encounter_context is not None:
        result["encounter_context"] = encounter_context
        candidate_count = encounter_context.get("total_encounter_candidates", 0)
        if candidate_count > 0:
            workstations.append({
                "priority": "P1",
                "name": f"偶遇候选：{candidate_count}条跨纲边待审",
                "status": f"encounter_candidates:{candidate_count}",
                "source": "encounter_detection",
            })

    # 2e. 研究线扫描：active 线的 unblocked next_actions 生成工位
    try:
        rl_context, rl_workstations = _scan_research_lines(root)
        if rl_context is not None:
            result["research_lines"] = rl_context
            workstations.extend(rl_workstations)
    except Exception as exc:
        result["research_lines_error"] = f"{type(exc).__name__}: {exc}"

    # 3. 079号：如果无任何工位，执行 no_work_fallback
    if not workstations:
        workstations = discover_business_tasks(root)
        result["fallback_triggered"] = True

    result["workstations"] = workstations

    # 081号：清晰报告干净终止条件
    # 真阴性干净终止 = roadmap 为空 AND session 遗留为空 AND fallback 为空
    #                    AND pending 谱系为空 AND 审查结果为空（147号-2）
    pending_count = len(glob.glob(os.path.join(root, ".chanlun/genealogy/pending/*.md")))
    result["clean_terminate"] = (
        len(roadmap_tasks) == 0
        and len(workstations) == 0
        and pending_count == 0
        and len(review_results) == 0
    )

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
            # 将最近谱系的 unresolved 下游行动转化为工位
            # 只取 genealogy_id >= (最大id - 20) 的，避免被历史项淹没
            recent_unresolved = [
                item for item in da.get("items", [])
                if item.get("status") == "unresolved"
                and item.get("genealogy_id", "0").isdigit()
                and int(item["genealogy_id"]) >= max(
                    (int(i["genealogy_id"]) for i in da.get("items", [])
                     if i.get("genealogy_id", "0").isdigit()),
                    default=0,
                ) - 20
            ]
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

    try:
        result["head"] = subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"], cwd=root, text=True).strip()
    except Exception:
        pass

    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
