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
VALID_TOPO_TYPES = frozenset({"freeze", "split", "sever"})


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
    frozen_sha_ids = set()
    freeze_events = []  # (event_block_id, target_sha)
    for rel in all_rels:
        if rel.get("relation") == "freezes":
            frozen_sha_ids.add(rel["to"])
            freeze_events.append((rel["from"], rel["to"]))

    if not frozen_sha_ids:
        return set()

    # 3. 对 scope=downstream 的 freeze，BFS 展开 target 的所有下游
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
        children_of = {}
        for rel in all_rels:
            if rel.get("relation") == "depends_on" and rel.get("order") == 1:
                parent = rel["to"]
                child = rel["from"]
                children_of.setdefault(parent, set()).add(child)

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
            frozen_ids.add(sha)

    return frozen_ids


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
            fm_match = re.match(r"^---\s*\n(.+?)\n---", head, re.DOTALL)
            if not fm_match:
                continue
            fm = yaml.safe_load(fm_match.group(1))
            if not isinstance(fm, dict):
                continue
            if fm.get("topo_executed_at"):
                continue
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


def detect_genealogy_anomalies(root):
    """检测谱系编号异常：重复编号、文件名编号与内部 id 不一致、block-topology 完整性、frontmatter schema。

    返回异常列表，每个元素包含 type、files、detail 字段。
    空列表 = 无异常。
    """
    settled_dir = os.path.join(root, ".chanlun/genealogy/settled/")
    if not os.path.isdir(settled_dir):
        return []

    anomalies = []
    num_to_files = {}
    required_frontmatter = {"id", "status", "type", "date"}

    for filepath in glob.glob(os.path.join(settled_dir, "*.md")):
        basename = os.path.basename(filepath)
        m = re.match(r'^(\d+)-(.+)\.md$', basename)
        if not m:
            continue
        file_num = int(m.group(1))
        num_to_files.setdefault(file_num, []).append(basename)

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

            found_fields = set()
            for line in head.split("\n")[:20]:
                for field in required_frontmatter:
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


def compute_delta_blocks(root):
    """178号下游推论：检测 block-topology 区块变化。

    从 meta.json 读取迁移时区块数，与当前 blocks/ 目录下 *.json 文件数比较。
    区块系统是谱系的补充/升格，与 delta_genealogy 并存。
    """
    block_dir = os.path.join(root, ".chanlun/block-topology/blocks")
    meta_path = os.path.join(root, ".chanlun/block-topology/meta.json")

    if os.path.isdir(block_dir):
        current_blocks = len(glob.glob(os.path.join(block_dir, "*.json")))
    else:
        current_blocks = 0

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

                # unblocked action——检查 completed_at（gangmu_update.py 标记）或 completion_check
                all_blocked = False
                if action.get("completed_at"):
                    continue
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

    # 2f. 谱系编号异常检测（编号冲突自动发现 + 修复工位生成）
    genealogy_anomalies = detect_genealogy_anomalies(root)
    if genealogy_anomalies:
        result["genealogy_anomalies"] = genealogy_anomalies
        workstations.append({
            "priority": "P0",
            "name": f"谱系编号异常：{len(genealogy_anomalies)}项",
            "status": "; ".join(a["detail"] for a in genealogy_anomalies),
            "source": "genealogy_anomaly_detection",
        })

    # 2g. 183号目B：异步自指审计（t 审查 t-1）
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

    # 3. 079号：如果无任何工位，执行 no_work_fallback
    if not workstations:
        workstations = discover_business_tasks(root)
        result["fallback_triggered"] = True

    result["workstations"] = workstations

    # 270号：suspended 工位过滤——从 ceremony_state 读取 suspended 列表
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
