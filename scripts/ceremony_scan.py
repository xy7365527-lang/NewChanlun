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
import json, os, glob, yaml, sys, argparse, subprocess


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


def scan_code_settlement_requests(root):
    """扫描 encounter_log (traversal-events.jsonl) 中的 code_settlement_request.

    逢亮的自诊断通过 proprioception → norm violation → code gap mapping 产出
    code_settlement_request。审批通过对话界面完成：operator 在对话中说"批准"后，
    daemon_api 写入 operator_ruling type=approved。

    扫描两类工位：
    1. 已批准 (operator_ruling=approved): 生成可执行工位
    2. 待审批 (status=pending, 无对应 ruling): 生成 pending_approval 工位（仅信息展示）
    """
    log_path = os.path.join(root, "topological-computation/.chanlun/traversal-events.jsonl")
    if not os.path.isfile(log_path):
        return []

    # Parse all entries
    requests = []   # code_settlement_request entries
    rulings = []    # operator_ruling entries
    with open(log_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
            except (json.JSONDecodeError, ValueError):
                continue
            entry_type = entry.get("type")
            if entry_type == "code_settlement_request" and entry.get("status") == "pending":
                requests.append(entry)
            elif entry_type == "operator_ruling":
                rulings.append(entry)

    # Build ruling index: diagnosed_file|timestamp → ruling
    approved_keys: set = set()
    rejected_keys: set = set()
    for r in rulings:
        key = f"{r.get('diagnosed_file', '')}|{r.get('original_timestamp', '')}"
        if r.get("ruling") == "approved":
            approved_keys.add(key)
        elif r.get("ruling") == "rejected":
            rejected_keys.add(key)

    workstations = []
    for req in requests:
        key = f"{req.get('diagnosed_file', '')}|{req.get('timestamp', '')}"
        norm_info = req.get("norm_violation", {})

        if key in approved_keys:
            # Approved: generate executable workstation
            workstations.append({
                "priority": "P0",
                "name": f"code-settlement: {req.get('diagnosed_file', 'unknown')}",
                "description": (
                    f"Gap: {req.get('gap_description', '')}\n"
                    f"Direction: {req.get('proposed_direction', '')}\n"
                    f"Basis: {req.get('theoretical_basis', '')}"
                ),
                "source": "self_diagnosis",
                "status": "operator_approved",
                "severity": norm_info.get("severity", "unknown"),
            })
        elif key not in rejected_keys:
            # Pending: informational only (waiting for operator dialogue approval)
            workstations.append({
                "priority": "P1",
                "name": f"pending-proposal: {req.get('diagnosed_file', 'unknown')}",
                "description": (
                    f"Gap: {req.get('gap_description', '')}\n"
                    f"Direction: {req.get('proposed_direction', '')}\n"
                    f"等待 operator 在对话中批准"
                ),
                "source": "self_diagnosis",
                "status": "pending_approval",
                "severity": norm_info.get("severity", "unknown"),
            })
        # Rejected: skip entirely

    return workstations


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

    # 2c. 逢亮自诊断：code_settlement_request (提案权，需 operator 审批)
    code_settlement_ws = scan_code_settlement_requests(root)
    if code_settlement_ws:
        result["code_settlement_requests"] = len(code_settlement_ws)
        workstations.extend(code_settlement_ws)

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
        and len(code_settlement_ws) == 0
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
