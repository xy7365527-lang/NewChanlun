#!/bin/bash
# Stop Hook — 通用停机阻断（Universal Stop-Guard）
# 048号谱系：从 044号（ceremony 专用）泛化为全场景覆盖
# 069号更新：废弃 ceremony 计数器状态机，改用显式状态检测
# 075号更新：移除 dominator node 检查（结构能力由 skill 事件驱动，不再是 teammate）
# 075号扬弃（编排者裁决，本次修复）：结构能力恢复为 teammate（095/096 真递归提供共享
#   inbox，化解 075 动机中的"孤岛"问题）→ 重新引入结构工位存在性检查（检查 1.5，agentType 信号）。
#   075 的 skill 事件驱动用于 Write/Stop 触发的轻量守卫，结构工位本体是常设 teammate（team-topology.json）。
# 155号修复：僵尸工位检测——使用 owner 而非 subject 标识工位；completed 不再报告为空闲
#
# 触发：Stop 事件（agent 即将结束 turn）
# 逻辑：
#   1. 检测"有活干但没人在干"的死寂状态 → 注入强指令启动蜂群
#   2. 蜂群任务队列非空 → 阻止 + 注入具体路由
#   3. 谱系有生成态矛盾 → 阻止 + 注入具体文件名和四分法指令
#   4. @proof-required 标签未验证 → 阻止 + 路由到 Gemini 数学验证
#   5. 四分法违規检测（通用） → 阻止
#   6. 以上均无 → 放行
#
# 熔断机制：
#   - 连续阻止 >= 3 次且状态无变化 → 允许停止（145号智能熔断）
#   - 用户 INTERRUPT → 允许停止

set -uo pipefail

resolve_python() {
    for candidate in python python3; do
        if command -v "$candidate" >/dev/null 2>&1; then
            if "$candidate" -c "import sys" >/dev/null 2>&1; then
                echo "$candidate"
                return 0
            fi
        fi
    done
    return 1
}

PYTHON_BIN="$(resolve_python || true)"
if [ -z "$PYTHON_BIN" ]; then
    exit 0
fi

python() { command "$PYTHON_BIN" "$@"; }

input=$(timeout 3 cat 2>/dev/null || echo "{}")
cwd=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd', '.'))" 2>/dev/null || echo ".")
cd "$cwd" 2>/dev/null || true

COUNTER=".chanlun/.stop-guard-counter"

# ─── 检查 0：context 临界 → 放行停机，让 autocompact 在 turn 边界 fire ───
# 编排者裁决（no-workaround 严格解，2026-06-23 task#40）：
#   真优先级矛盾——Stop-Guard 蜂群持续性 vs context 临界 compact 必要性。
#   严格解：context 临界时 compact 优先于蜂群持续。蜂群状态已持久化在文件系统
#   （~/.claude/tasks/ teams/ + .chanlun/genealogy/ sessions/），compact 后由
#   session-start-ceremony.sh 重注入角色锚点恢复，不丢失。
# 机制根因：autocompact 在停机点（turn 边界）触发。Stop-Guard 在蜂群任务活跃时
#   每轮 block 停机 → turn 边界永不到达 → autocompact 永不 fire → context 突破
#   75% 阈值后无界增长（实测 Lead session 14c95478 达 779k/1M=77.9% 未 compact）。
#   修复：context ≥ 放行阈值时，无条件放行停机（绕过下方所有 block 检查），
#   让 autocompact 取回它的 turn 边界。
# 阈值：放行阈值默认 85%（> autocompact 的 75%，给蜂群 10% 运行余量后强制 compact；
#   1M 窗口下 85% 留 150k headroom，避免硬上限溢出）。窗口默认 1M（Opus 4.8 [1m]），
#   sonnet 回退 200k；二者均可由 env 覆盖（不写死，遵守 config 化原则）。
TRANSCRIPT_PATH=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('transcript_path',''))" 2>/dev/null || echo "")
RELEASE_PCT="${CLAUDE_STOPGUARD_RELEASE_PCT:-85}"
if [ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ]; then
    CTX_PCT=$(python -c '
import json,sys
path=sys.argv[1]
env_window=sys.argv[2] if len(sys.argv)>2 else ""
last_tot=0; last_model=""
try:
    with open(path) as f:
        for line in f:
            line=line.strip()
            if not line: continue
            try: d=json.loads(line)
            except: continue
            if d.get("isSidechain"): continue
            msg=d.get("message",{})
            if not isinstance(msg,dict): continue
            u=msg.get("usage")
            if not u: continue
            last_tot=(u.get("input_tokens",0)+u.get("cache_creation_input_tokens",0)+u.get("cache_read_input_tokens",0)+u.get("output_tokens",0))
            m=msg.get("model","")
            if m and m!="<synthetic>": last_model=m
except Exception:
    print("0"); sys.exit(0)
if env_window and env_window.isdigit() and int(env_window)>0:
    window=int(env_window)
else:
    window=200000 if "sonnet" in last_model.lower() else 1000000
print(int(last_tot*100/window) if window>0 else 0)
' "$TRANSCRIPT_PATH" "${CLAUDE_CTX_WINDOW_TOKENS:-}" 2>/dev/null || echo "0")
    CTX_PCT=$((CTX_PCT + 0))
    if [ "$CTX_PCT" -ge "$RELEASE_PCT" ]; then
        # context 临界：先保存 session 状态（precompact-save.sh 会在 PreCompact 再存一次，
        # 双保险），重置熔断计数器，然后放行停机（不输出 decision=block）。
        cd "$cwd" 2>/dev/null || true
        bash scripts/write_session.sh >/dev/null 2>&1 || true
        rm -f "$COUNTER" 2>/dev/null || true
        python -c "
import json,sys
print(json.dumps({'continue': True, 'suppressOutput': False,
  'systemMessage': '[Stop-Guard] context 临界 '+sys.argv[1]+'% ≥ '+sys.argv[2]+'% 放行阈值——放行停机让 autocompact fire（编排者裁决 task#40：context 临界 compact 优先于蜂群持续）。蜂群状态已持久化，compact 后 session-start-ceremony 重注入恢复。'}, ensure_ascii=False))
" "$CTX_PCT" "$RELEASE_PCT"
        exit 0
    fi
fi
# 注（571号否定3：与 check3 责任方过滤正交）：check0 沿 context 维度全局放行（含责任方，
#   context 临界逃生阀）；下方 check3 沿责任方维度放行非责任方（正常 context 内的对象粒度
#   过滤）。二者无重叠、非冗余——一个解 context 临界死锁，一个解非责任方无意义阻断死锁。

# ─── 会话/Lead 任务目录解析（(c)裁决 2026-06-23：任务扫描 scope 到本 Lead 的 team） ───
# 旧实现全局扫描 ~/.claude/tasks/*/（跨 session 污染——其他 session 的陈旧 in_progress 任务
# 会误阻本 Lead 停机，见 feedback_task_queue_owner_liveness / reference_stopguard_zombie_tasks）。
# (c) 修复：通过 leadSessionId==session_id 定位本 Lead 的 team，只扫该 team 的任务目录。
# team 目录名 == 任务目录名（实测一致），故 basename 即任务子目录名。
# session_id 为空或无匹配 team（solo 会话）→ LEAD_TASK_DIR 为空 → 不强制任务队列（与检查1.5同构）。
SESSION_ID=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('session_id',''))" 2>/dev/null || echo "")
LEAD_TASK_DIR=""
LEAD_TEAM=""

# ── teammate 判定：首条 transcript 记录 type=agent-setting（spawn 时冻结的 subagent_type）──
# teammate 不领 team，也不负责 ceremony/结构工位——LEAD_TEAM 解析与 check1.5 均须排除。
IS_TEAMMATE=0
if [ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ]; then
    IS_TEAMMATE=$(python -c '
import json, sys
tp = sys.argv[1]
try:
    with open(tp, encoding="utf-8") as fh:
        for i, line in enumerate(fh):
            if i > 10:
                break
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except Exception:
                continue
            a = rec.get("agentSetting")
            if isinstance(a, str) and a:
                print("1"); sys.exit(0)
except Exception:
    pass
print("0")
' "$TRANSCRIPT_PATH" 2>/dev/null || echo "0")
fi

# ── 本 session→team 绑定解析（#129 修复：session 延续后 leadSessionId 精确匹配永久误报）──
# 根因：隐式 team 的 leadSessionId 冻结在首建 session id；compact/续接后当前 session_id 漂移，
#   leadSessionId==session_id 永假 → 误判「无 team」→ check1.5 __NO_TEAM__ 永久 block。
# 信号（按可靠性降序）：1) leadSessionId==session_id（compact 前精确；fresh team 无 spawn 回执时唯一可靠）；
#   2) fallback：本 session transcript 里引用某活 team 成员 agentId 后缀 @<teamName>（spawn 回执 /
#      teammate 消息 = 本 session 与 team 的直接绑定证据），取引用最多的活 team。续接 session 会把
#      agentId 重写成当前 session 后缀（无 team 目录），故按「活 team 目录存在」过滤自动排除幻影后缀。
# teammate 会引用自身 team 后缀，故须由 IS_TEAMMATE 排除以免误判为 Lead。
# ponytail: fallback 全量读 transcript（实测 37M/0.16s）；若将来 transcript 巨大到影响 Stop 延迟，
#   升级路径=命中活 team 计数达阈值即早退。
if [ "$IS_TEAMMATE" -eq 0 ] && [ -n "$SESSION_ID" ]; then
    LEAD_TEAM=$(python -c '
import json, os, sys
session_id = sys.argv[1]; teams = sys.argv[2]; transcript = sys.argv[3]
live = []
if os.path.isdir(teams):
    for name in sorted(os.listdir(teams)):
        cfg = os.path.join(teams, name, "config.json")
        if not os.path.isfile(cfg):
            continue
        try:
            d = json.load(open(cfg))
        except Exception:
            continue
        live.append(name)
        if d.get("leadSessionId", "") == session_id:
            print(name); sys.exit(0)
if transcript and os.path.isfile(transcript):
    try:
        with open(transcript, encoding="utf-8") as f:
            text = f.read()
    except Exception:
        text = ""
    counts = {n: text.count("@" + n) for n in live}
    counts = {k: v for k, v in counts.items() if v}
    if counts:
        print(max(counts, key=counts.get))
' "$SESSION_ID" "$HOME/.claude/teams" "$TRANSCRIPT_PATH" 2>/dev/null || echo "")
fi
if [ -n "$LEAD_TEAM" ] && [ -d "$HOME/.claude/tasks/$LEAD_TEAM" ]; then
    LEAD_TASK_DIR="$HOME/.claude/tasks/$LEAD_TEAM"
fi

# ─── 熔断检查（145号：智能熔断，计数器格式 COUNT:LAST_ACTIVE_TASKS） ───
COUNT=0
LAST_ACTIVE=0
if [ -f "$COUNTER" ]; then
    COUNTER_DATA=$(cat "$COUNTER" 2>/dev/null || echo "0:0")
    COUNT=$(echo "$COUNTER_DATA" | cut -d: -f1)
    LAST_ACTIVE=$(echo "$COUNTER_DATA" | cut -d: -f2)
    COUNT=$((COUNT + 0))
    LAST_ACTIVE=$((LAST_ACTIVE + 0))
fi

# 注意：ACTIVE_TASKS 在下方检查2中计算，此处先做预计算以支持智能熔断
PRE_ACTIVE_TASKS=0
if [ -n "$LEAD_TASK_DIR" ]; then
    for tf in "$LEAD_TASK_DIR"/*.json; do
        [ -f "$tf" ] || continue
        case "$tf" in *.lock) continue ;; esac
        ts=$(python -c "
import json, sys
with open(sys.argv[1]) as f: d=json.load(f)
s=d.get('status','')
if s in ('pending','in_progress'): print('1')
else: print('0')
" "$tf" 2>/dev/null || echo "0")
        PRE_ACTIVE_TASKS=$((PRE_ACTIVE_TASKS + ts))
    done
fi

if [ "$COUNT" -ge 3 ] && [ "$PRE_ACTIVE_TASKS" -eq "$LAST_ACTIVE" ]; then
    # 状态停滞 3 次，放行
    rm -f "$COUNTER" 2>/dev/null || true
    exit 0
fi

# ─── 熔断计数器唯一写点（#136 口径统一修复） ───
# 根因：check2 写点曾写 ACTIVE_TASKS（自算口径，排除 blocked pending，实测 5），而放行
#   判断用 PRE_ACTIVE_TASKS（全部 pending+in_progress，实测 9）——两口径分叉致 LAST_ACTIVE
#   恒 5、PRE 恒 9 永不相等，熔断死锁（counter=11:5 仍不放行）。
# 统一：所有 block 分支经本函数写入，与放行判断同口径 PRE_ACTIVE_TASKS。「状态停滞」语义
#   = Lead 任务目录的 pending+in_progress 全集不变（blocked pending 也是状态的一部分——
#   解除阻塞即状态变化，须 reset）。ACTIVE_TASKS 仅用于 check2 路由文本，不进计数器。
# 145号语义：任务态变化 → reset 1:新值；不变 → COUNT+1，连续 3 次由顶部放行。
write_counter() {
    if [ "$PRE_ACTIVE_TASKS" -ne "$LAST_ACTIVE" ]; then
        echo "1:$PRE_ACTIVE_TASKS" > "$COUNTER"
    else
        echo "$((COUNT + 1)):$PRE_ACTIVE_TASKS" > "$COUNTER"
    fi
}

# ─── 检查 1：069号下游推论或 session 中断点有未执行的工作 ───
# 检测"有活干但没人在干"：session 记录有下一轮方向，但没有活跃蜂群
HAS_PENDING_WORK=0
WORK_DESCRIPTION=""

# 检查最新 session 的下一轮方向
LATEST_SESSION=$(ls -t .chanlun/sessions/20*-session.md 2>/dev/null | head -1) || true
if [ -n "$LATEST_SESSION" ]; then
    # 检查是否有"下一轮方向"或"下一轮 Blocker"章节
    if grep -q "## 下一轮" "$LATEST_SESSION" 2>/dev/null; then
        # 检查是否有活跃的蜂群（team 目录存在）
        ACTIVE_TEAMS=$(ls -d "$HOME/.claude/teams"/*/ 2>/dev/null | wc -l) || ACTIVE_TEAMS=0
        if [ "$ACTIVE_TEAMS" -eq 0 ]; then
            # 有工作但没有蜂群在跑
            WORK_DESCRIPTION=$(python -c "
import sys
with open(sys.argv[1], 'r', encoding='utf-8') as f:
    lines = f.readlines()
in_next = False
items = []
for line in lines:
    if line.startswith('## 下一轮'):
        in_next = True
        continue
    if in_next and line.startswith('## '):
        break
    if in_next and line.strip().startswith(('-', '*', '1', '2', '3', '4', '5')):
        items.append(line.strip().lstrip('-*0123456789. '))
print(' | '.join(items[:5]))
" "$LATEST_SESSION" 2>/dev/null || echo "")
            if [ -n "$WORK_DESCRIPTION" ]; then
                HAS_PENDING_WORK=1
            fi
        fi
    fi
fi

if [ "$HAS_PENDING_WORK" -gt 0 ]; then
    write_counter
    python -c "
import json, sys
work = sys.argv[1]
print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 检测到死寂状态：有待推进工作但无活跃蜂群。立即评估并行工位数，拉取递归拓扑蜂群。待推进: {work}'
}, ensure_ascii=False))
" "$WORK_DESCRIPTION"
    exit 0
fi

# ─── 检查 1.5：结构工位 bootstrap 强制（095/096号 + 本号扬弃 075号） ───
# 137号：否定性文本提示对行为执行层无效——结构工位 spawn 从"文本提示"升格为"机制强制"。
# 编排者裁决：结构工位是 teammate（spawn，095/096），非纯 skill 事件驱动（075 被扬弃）。
# 检测信号（严格可靠，非模糊匹配）：team config.json 的 member.agentType。
#   - agentType 是 Task(subagent_type=…) 落盘的规范结构类型；业务命名（如 geneal-p4）
#     不改变 agentType=genealogist，故 geneal-p4/geneal-560 自动算 genealogist 已覆盖。
#   - 仅对 LEAD session 生效：team 归属由上方 LEAD_TEAM 统一解析（#129：leadSessionId 精确匹配在
#     compact/续接后 session_id 漂移永久误报，改为 leadSessionId 主 + transcript @<teamName> 绑定 fallback）；
#     teammate（IS_TEAMMATE=1）→ 跳过（teammate 无法也不负责 spawn 结构工位）。
#   - 无 team（未形成蜂群）→ 不强制（solo 会话无蜂群约束）。
# 145号兼容：沿用检查3/4 的计数器写法（增量 COUNT + 存 PRE_ACTIVE_TASKS），
#   连续3次任务态不变由顶部熔断放行，避免死锁。
if [ -n "$SESSION_ID" ]; then
    STRUCT_MISSING=$(python -c '
import json, os, sys
lead_team = sys.argv[1]; teams_dir = sys.argv[2]
is_teammate = sys.argv[3] == "1"; pending_dir = sys.argv[4]
required = ["meta-lead","genealogist","quality-guard","code-verifier","meta-observer","topology-manager"]
if lead_team:
    try:
        with open(os.path.join(teams_dir, lead_team, "config.json")) as f:
            d = json.load(f)
    except Exception:
        sys.exit(0)   # 配置不可读 → fail-open 不阻断
    types = set(m.get("agentType","") for m in d.get("members", []))
    print(",".join(r for r in required if r not in types))
    sys.exit(0)
# 无 team（LEAD_TEAM 空）：闭合 check1.5 鸡生蛋缺口（option A 强化逼迫，编排者裁决 2026-06-25）。
# teammate 不负责 ceremony → 放行；Lead/主 session 且 pending 谱系非空 → 强制 ceremony bootstrap。
if is_teammate:
    sys.exit(0)
has_work = os.path.isdir(pending_dir) and any(fn.endswith(".md") for fn in os.listdir(pending_dir))
if has_work:
    print("__NO_TEAM__")
' "$LEAD_TEAM" "$HOME/.claude/teams" "$IS_TEAMMATE" ".chanlun/genealogy/pending" 2>/dev/null || echo "")
    if [ -n "$STRUCT_MISSING" ]; then
        write_counter
        if [ "$STRUCT_MISSING" = "__NO_TEAM__" ]; then
            python -c "
import json
print(json.dumps({
    'decision': 'block',
    'reason': '[Stop-Guard] 本 session 未 bootstrap 蜂群（无 team）且有待推进的生成态谱系。不允许停止。第一动作=invoke /ceremony 热启动序列：运行 python scripts/ceremony_state.py write 1 initial → python scripts/ceremony_scan.py，由 scan 确定性输出决定本轮工位并并行 spawn（含 6 个常设结构工位 + 业务工位）。137号机制强制：ceremony 自动触发从 SessionStart 文本提示（平台不能 block）升格为 Stop block 强制。若确实无需蜂群，连续 3 次停机由顶部熔断自动放行。'
}, ensure_ascii=False))
"
        else
            python -c "
import json, sys
missing = sys.argv[1]
print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 蜂群缺失常设结构工位（095/096号 bootstrap 强制；137号 hook 机制化——文本提示升格为机制强制）: [{missing}]。不允许停止。立即并行 spawn 缺失的结构工位 teammates：Agent(name=结构工位名, subagent_type=结构工位名, run_in_background=true)（隐式 team / flat roster，非 stale Task(team_name=)），见 .claude/team-topology.json structural_agents。spawn 时 prompt 注入 topo_address+parent_callback 基因（073a/274号，#41 第二轮）。结构工位是 teammate（编排者裁决扬弃 075号"结构=skill"），不是纯 skill 事件驱动。'
}, ensure_ascii=False))
" "$STRUCT_MISSING"
        fi
        exit 0
    fi
fi

# ─── 检查 2：蜂群任务队列 ───
# 扫描活跃任务状态，生成具体路由指令
# 145号修复：排除被 blockedBy 阻塞的 pending 任务 + 僵尸任务检测
ACTIVE_TASKS=0
PENDING_TASKS=""
IN_PROGRESS_TASKS=""
UNOWNED_TASKS=""
if [ -n "$LEAD_TASK_DIR" ]; then
    for task_file in "$LEAD_TASK_DIR"/*.json; do
        [ -f "$task_file" ] || continue
        # 跳过 .lock 文件
        case "$task_file" in *.lock) continue ;; esac
        # 提取 status, owner, subject, blockedBy
        # 155号修复：使用 owner（agent name）而非 subject 作为工位标识
        TASK_INFO=$(python -c "
import json, sys
with open(sys.argv[1]) as f:
    d = json.load(f)
status = d.get('status','')
owner = d.get('owner','')
subject = d.get('subject','')
blocked = d.get('blockedBy', [])
# 只有当 blockedBy 中所有任务都已完成时，才算未阻塞
has_open_blockers = False
if blocked:
    import os
    task_dir = os.path.dirname(sys.argv[1])
    for bid in blocked:
        bf = os.path.join(task_dir, f'{bid}.json')
        if os.path.exists(bf):
            with open(bf) as bfh:
                bs = json.load(bfh).get('status','')
            if bs != 'completed':
                has_open_blockers = True
                break
        else:
            has_open_blockers = True
            break
# 输出格式: status<TAB>blocked<TAB>owner<TAB>subject
# 使用 TAB 分隔避免 subject 中含空格导致 cut 错位
print(f'{status}\t{\"BLOCKED\" if has_open_blockers else \"UNBLOCKED\"}\t{owner}\t{subject}')
" "$task_file" 2>/dev/null) || continue
        TASK_STATUS=$(echo "$TASK_INFO" | cut -f1)
        TASK_BLOCKED=$(echo "$TASK_INFO" | cut -f2)
        TASK_OWNER=$(echo "$TASK_INFO" | cut -f3)
        TASK_SUBJECT=$(echo "$TASK_INFO" | cut -f4-)
        # 155号修复：用 owner 标识工位，subject 仅作描述
        # completed 任务不再报告为"空闲工位"——工位退出后不应被追踪
        DISPLAY_NAME="${TASK_OWNER:-未分配}"
        case "$TASK_STATUS" in
            pending)
                # 145号：被阻塞的 pending 不计为活跃——它们无法被分配
                if [ "$TASK_BLOCKED" = "UNBLOCKED" ]; then
                    ACTIVE_TASKS=$((ACTIVE_TASKS + 1))
                    PENDING_TASKS="${PENDING_TASKS:+$PENDING_TASKS, }${TASK_SUBJECT}(→${DISPLAY_NAME})"
                    # (c)裁决 2026-06-23：无主（owner 空）未阻塞 pending = 工位向下递归
                    # 产出的子任务，Lead 必须 spawn 一个工位消费（spawn-per-unowned）。
                    if [ -z "$TASK_OWNER" ]; then
                        UNOWNED_TASKS="${UNOWNED_TASKS:+$UNOWNED_TASKS, }${TASK_SUBJECT}"
                    fi
                fi
                ;;
            in_progress)
                ACTIVE_TASKS=$((ACTIVE_TASKS + 1))
                IN_PROGRESS_TASKS="${IN_PROGRESS_TASKS:+$IN_PROGRESS_TASKS, }${DISPLAY_NAME}:${TASK_SUBJECT}"
                ;;
            completed)
                # 155号修复：completed 任务不计入活跃，不报告为空闲
                # 已完成 = 工位已交付，不需要路由指令
                ;;
        esac
    done
fi

if [ "$ACTIVE_TASKS" -gt 0 ]; then
    # #136：计数器统一走 write_counter（PRE 口径）；ACTIVE_TASKS 仅用于下方路由文本
    write_counter
    # ─── (c) 生产端 spawn mandate（#41 机制化，约束4 093号）───
    # canonical 单一源：.claude/team-topology.json spawn_mandate.template（唯一权威，编辑只此一处）。
    # 降级 fallback（codex 异质审计 D 修复）：canonical 不可读时启用，明确标注 [降级 fallback]，
    #   不维护第二份完整副本（避免双源 drift/声明膨胀）——只保留一行降级提示指向 SKILL.md。
    # 137号：否定性文本提示对执行层无效——用正面格式把"工位必须 invoke sub-swarm-ceremony 布设四类节点子 DAG"
    #   嵌入 Lead 收到的 (c)spawn 路由，使每次 spawn 自动携带 mandate（消费端 #35 之补全=生产端）。
    MANDATE_FALLBACK='[team-topology.json spawn_mandate 不可读——降级 fallback；完整 template/event_skill_map 见 .claude/team-topology.json + .claude/skills/sub-swarm-ceremony/SKILL.md] 不变量：spawn 带基因 topo_address+parent_callback；工位 invoke sub-swarm-ceremony 评估分解→四类节点子DAG（含 codex-challenger 异质审计=约束4 硬节点）；事件→TaskCreate(metadata.agent_type)→Lead spawn。'
    SPAWN_MANDATE=$(python -c "
import json, sys
try:
    with open('.claude/team-topology.json') as f:
        m = json.load(f).get('spawn_mandate', {}).get('template', '')
    sys.stdout.write(m)
except Exception:
    pass
" 2>/dev/null)
    # codex 异质审计 C 修复：python 整体失败或 canonical 为空 → 用降级 fallback，mandate 永不为空。
    [ -z "$SPAWN_MANDATE" ] && SPAWN_MANDATE="$MANDATE_FALLBACK"
    # 155号修复：移除僵尸工位报告（completed 不再参与路由）
    python -c "
import json, sys
active = int(sys.argv[1])
pending = sys.argv[2]
in_progress = sys.argv[3]
unowned = sys.argv[4]
mandate = sys.argv[5] if len(sys.argv) > 5 else ''

# 构建具体路由指令（(c)裁决 2026-06-23：Lead 为每个无主任务 spawn 一个工位）
instructions = []

# (c) 核心：无主（owner 空）未阻塞 pending = 工位向下递归产出的子任务，Lead 必须 spawn
# #41 生产端机制化：spawn 时把 spawn_mandate 注入被 spawn 工位的 prompt（强制 sub-swarm-ceremony 分解评估）。
if unowned:
    instructions.append(
        '(c)spawn：为每个无主任务 spawn 一个工位 '
        'Agent(name=任务标识, subagent_type=metadata.agent_type 或 general-purpose, '
        'run_in_background=true)。【spawn mandate 强制（生产端机制化 #41/约束4）——'
        f'被 spawn 工位 prompt 必须含：{mandate}】'
        ' 这是 (c) 生产端递归——工位自己 invoke sub-swarm-ceremony 布设子 DAG，你只 spawn。'
        f'无主任务: [{unowned}]'
    )

# 全部未阻塞 pending（含已分配但工位未起/未认领）
if pending:
    instructions.append(f'未阻塞 pending: [{pending}]')

# 运行中工位
if in_progress:
    instructions.append(f'运行中: [{in_progress}]，等待汇报')

route = ' | '.join(instructions) if instructions else '请检查 TaskList 并推进未完成任务'

print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 蜂群任务队列有 {active} 个活跃任务。不允许停止。路由指令: {route}'
}, ensure_ascii=False))
" "$ACTIVE_TASKS" "$PENDING_TASKS" "$IN_PROGRESS_TASKS" "$UNOWNED_TASKS" "$SPAWN_MANDATE"
    exit 0
fi

# ─── 检查 2.5：idle 工位检测（#55 反 idle 生命周期机制化）───
# 机制化"teammate 生命周期 = 任务生命周期"（069号 RTAS）：业务工位完成其全部任务后仍存活
#   （isActive=True，占用 pty）= idle。
# no-workaround 严格形式：harness 无法让 teammate 自消亡（flat roster + 无自 shutdown 工具，
#   swarm-mechanism-fix-20260623 已结算）→ teammate 职责止于"汇报 + 声明 ready-for-shutdown"
#   （由 spawn_mandate.anti_idle 注入工位 prompt），实际 shutdown 释放 pty 是 Lead 职责，
#   机制化为本检查的路由提示（不假装 teammate 自消亡）。
# 检测信号（严格可靠，非模糊匹配，沿用 check1.5 的 agentType 信号 + isActive pty 存活标志）：
#   - team config.json member.isActive==True（pty 存活）
#   - member.name == 某 completed 任务的 owner（任务生命周期已终结）
#   - 该 member 不拥有任何 pending/in_progress 任务（无活跃任务=确实 idle）
#   - 排除结构工位（meta-lead/genealogist/... 常设，生命周期=蜂群非任务，由 check1.5 强制存在；
#     flag 之会与 check1.5 振荡）+ 排除 team-lead（Lead 自身 in-process，不 shutdown 自己）
# 仅当 ACTIVE_TASKS==0（本检查在 check2 提前退出之后）触发——有活跃任务时 Lead 正路由活跃工作，
#   idle 清理让位（避免噪声；idle 工位在活跃任务清空后由本检查统一捕获）。
# 145号兼容：沿用计数器写法，连续3次任务态不变由顶部熔断放行（Lead 无法 shutdown 时不死锁）。
IDLE_TEAMMATES=""
if [ -n "$LEAD_TEAM" ] && [ -n "$LEAD_TASK_DIR" ] && [ -f "$HOME/.claude/teams/$LEAD_TEAM/config.json" ]; then
    IDLE_TEAMMATES=$(python -c "
import json, os, sys
cfg_path = sys.argv[1]
task_dir = sys.argv[2]
# 结构工位（常设）+ team-lead：生命周期=蜂群非任务，排除（与 check1.5 required 一致 + lead）
STRUCTURAL = {'team-lead','meta-lead','genealogist','quality-guard','code-verifier','meta-observer','topology-manager'}
try:
    with open(cfg_path) as f:
        cfg = json.load(f)
except Exception:
    sys.exit(0)
# 存活的业务工位：isActive 严格 True，且非结构工位（按 name 与 agentType 双重排除）
alive = set()
for m in cfg.get('members', []):
    name = m.get('name','')
    if not name or name in STRUCTURAL:
        continue
    if m.get('agentType','') in STRUCTURAL:
        continue
    if m.get('isActive') is True:
        alive.add(name)
if not alive:
    sys.exit(0)
# 任务所有权映射：owner -> 有无活跃任务 / 有无 completed 任务
has_active = set()
has_completed = set()
for fn in os.listdir(task_dir):
    if not fn.endswith('.json'):
        continue
    try:
        with open(os.path.join(task_dir, fn)) as f:
            d = json.load(f)
    except Exception:
        continue
    owner = d.get('owner','')
    if not owner or owner not in alive:
        continue
    st = d.get('status','')
    if st in ('pending','in_progress'):
        has_active.add(owner)
    elif st == 'completed':
        has_completed.add(owner)
# idle = 存活 + 有 completed 任务 + 无活跃任务
idle = sorted(n for n in alive if n in has_completed and n not in has_active)
print(','.join(idle))
" "$HOME/.claude/teams/$LEAD_TEAM/config.json" "$LEAD_TASK_DIR" 2>/dev/null || echo "")
fi

if [ -n "$IDLE_TEAMMATES" ]; then
    write_counter
    python -c "
import json, sys
idle = sys.argv[1]
print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 检测到 idle 工位（任务全 completed 但 isActive=True 占用 pty）: [{idle}]。不允许停止。#55 反 idle 生命周期机制化：teammate 生命周期=任务生命周期（完成即 shutdown 不 idle）。no-workaround 严格形式——harness 无法让 teammate 自消亡，Lead 消费其汇报后立即 shutdown 释放 pty（向工位发 shutdown_request 或平台 shutdown）。(c) 循环不复用 idle：新无主任务 spawn 新 teammate（非复活 idle）。结构工位（meta-lead/genealogist/quality-guard/code-verifier/meta-observer/topology-manager 常设）已排除，不在此列。'
}, ensure_ascii=False))
" "$IDLE_TEAMMATES"
    exit 0
fi

# ─── 检查 3：生成态谱系矛盾（571号责任方过滤）───
# 571号谱系：阻断对象从「任何触发 Stop 的 agent」（session 级）精确化为「该 pending 的责任方
#   agent」（责任方级）。非责任方放行——048号「蜂群级有工作→不停」不下降为个体命题（释放非
#   责任方 ≠ 释放责任方；责任方仍被阻断，蜂群循环继续）。
# codex#77 异质审查三否定约束（review-results/codex-review-stopguard-amendment-77-20260623.md）：
#   否定1（排除自称后门）：责任判定只读两个客观来源——pending frontmatter 的 responsible_agents
#     字段 + 平台在 transcript 首条 type=agent-setting 记录写入的 agentType。绝不读取 agent 在
#     Stop 输出里的自称（stop_hook_content 从不参与责任判定）。
#   否定2（responsible_agents 字段前置）：责任方来源是每个 pending 显式声明的机器可读字段，
#     非 type→agentType 硬编码映射表（避免映射表 drift / 声明膨胀）。
#   否定3（对齐 check0）：见 check0 末尾注——check0 沿 context 维度全局放行（含责任方），check3
#     沿责任方维度放行非责任方（正常 context 内）。二者正交、无重叠、非冗余。
# 身份来源（实测修正 codex FB-1）：team config 的 members 无 sessionId 字段，无法由 session_id
#   映射 agentType。客观来源是 transcript 首条 type=agent-setting 记录的 agentSetting（=平台
#   spawn 时记录的 subagent_type；业务命名如 geneal-p4 仍归一为 agentType=genealogist）。
# 作用域（572 决断点A）：Lead session（leadSessionId==session_id，即 LEAD_TEAM 非空）保留全
#   swarm 判据（负责全局调度/spawn 结算工位）→ 无条件阻断；责任方过滤仅作用于 teammate。
# Fallback（验证d，保守=阻断防漏）：responsible_agents 字段缺失的 pending → 视为责任方；
#   agentType 不可确定（无 agent-setting，如 main/solo session）→ 同样保守阻断。
PENDING_COUNT=0
PENDING_FILES=""
if [ -d ".chanlun/genealogy/pending" ]; then
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        PENDING_COUNT=$((PENDING_COUNT + 1))
        BASENAME=$(basename "$f")
        PENDING_FILES="${PENDING_FILES:+$PENDING_FILES, }$BASENAME"
    done < <(find .chanlun/genealogy/pending -name "*.md" -type f 2>/dev/null)
fi

if [ "$PENDING_COUNT" -gt 0 ]; then
    # 责任方判定：Lead 无条件责任方（全 swarm 判据）；teammate 按 responsible_agents 过滤。
    if [ -n "$LEAD_TEAM" ]; then
        IS_RESPONSIBLE=1
    else
        IS_RESPONSIBLE=$(python -c "
import json, os, sys
tp = sys.argv[1]; pending_dir = sys.argv[2]
Q2 = chr(34); Q1 = chr(39)
def clean(v):
    return v.strip().strip(Q2).strip(Q1)
# agentType = transcript 首条 agent-setting 的 agentSetting（平台客观身份，非运行时自称）
agent_type = None
if tp and os.path.isfile(tp):
    try:
        with open(tp, encoding='utf-8') as fh:
            for i, line in enumerate(fh):
                if i > 10:
                    break
                line = line.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except Exception:
                    continue
                a = rec.get('agentSetting')
                if isinstance(a, str) and a:
                    agent_type = a
                    break
    except Exception:
        pass
if not agent_type:
    print('1'); sys.exit(0)   # 身份不可确定 → fail-safe 阻断
def parse_responsible(path):
    try:
        with open(path, encoding='utf-8') as fh:
            text = fh.read()
    except Exception:
        return None
    lines = text.splitlines()
    if lines and lines[0].strip() == '---':
        fm = []
        for ln in lines[1:]:
            if ln.strip() == '---':
                break
            fm.append(ln)
    else:
        fm = lines
    for i, ln in enumerate(fm):
        s = ln.strip()
        if s.startswith('responsible_agents:'):
            rest = s[len('responsible_agents:'):].strip()
            if rest.startswith('[') and ']' in rest:
                inner = rest[1:rest.index(']')]
                return [clean(x) for x in inner.split(',') if x.strip()]
            if rest == '' or rest.startswith('#'):
                items = []
                for ln2 in fm[i+1:]:
                    s2 = ln2.strip()
                    if s2.startswith('- '):
                        items.append(clean(s2[2:]))
                    elif s2 == '':
                        continue
                    else:
                        break
                return items
            val = clean(rest.split(' #')[0])
            return [val] if val else []
    return None
is_resp = 0
if os.path.isdir(pending_dir):
    for fn in sorted(os.listdir(pending_dir)):
        if not fn.endswith('.md'):
            continue
        rp = parse_responsible(os.path.join(pending_dir, fn))
        if rp is None:
            is_resp = 1   # responsible_agents 字段缺失 → fail-safe 视为责任方
            break
        if agent_type in rp:
            is_resp = 1
            break
print(str(is_resp))
" "$TRANSCRIPT_PATH" ".chanlun/genealogy/pending" 2>/dev/null || echo "1")
        [ -z "$IS_RESPONSIBLE" ] && IS_RESPONSIBLE=1
    fi

    if [ "$IS_RESPONSIBLE" -eq 1 ]; then
        write_counter
        python -c "
import json, sys
n = sys.argv[1]
files = sys.argv[2]
print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 谱系有 {n} 个生成态 pending: [{files}]。你是其中至少一个 pending 的责任方（或 Lead / 身份不可确定，保守阻断）。不允许停止。pending 结算属 genealogist（结算/张力检查）+ 编排者（选择类/语法记录裁决）职责，非 Lead 自结算。018号四分法=定理/选择/语法记录/行动。571号责任方过滤：非责任方 agent 已放行（蜂群级持续性由责任方存活保证，非下降为个体命题）。'
}, ensure_ascii=False))
" "$PENDING_COUNT" "$PENDING_FILES"
        exit 0
    fi
    # 非责任方（571号）：不写 counter（FB-3 counter 竞态免疫），放行 → 继续后续检查 check4/5。
fi

# ─── 检查 4：@proof-required 标签扫描 ───
PROOF_REQUIRED=0
PROOF_LOCATIONS=""
if [ -d "spec" ] || [ -d "src" ] || [ -d ".chanlun" ]; then
    PROOF_SCAN=$(grep -rn "@proof-required" spec/ src/ .chanlun/ 2>/dev/null | grep -v '/genealogy/settled/' | grep -v 'README.md' | grep -v 'dispatch-spec.yaml' | grep -v 'dispatch-dag.yaml' | grep -v 'tags:.*@proof-required' | grep -v '/sessions/' | grep -v 'proof-verified.txt' | grep -v '"full_text"' || true)
    if [ -n "$PROOF_SCAN" ]; then
        PROOF_REQUIRED=$(echo "$PROOF_SCAN" | wc -l)
        PROOF_LOCATIONS=$(echo "$PROOF_SCAN" | head -5 | while IFS= read -r line; do
            FILE=$(echo "$line" | cut -d: -f1)
            LINE_NUM=$(echo "$line" | cut -d: -f2)
            echo "$FILE:$LINE_NUM"
        done | tr '\n' ', ' | sed 's/, $//')
    fi
fi

if [ "$PROOF_REQUIRED" -gt 0 ]; then
    write_counter
    python -c "
import json, sys
n = sys.argv[1]
locs = sys.argv[2]
print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 发现 {n} 个 @proof-required 标签未验证: [{locs}]。不允许停止。路由指令: 路由到 Gemini 数学验证，或标记为已验证。'
}, ensure_ascii=False))
" "$PROOF_REQUIRED" "$PROOF_LOCATIONS"
    exit 0
fi

# ─── 检查 5（通用）：四分法违規检测（不限 ceremony） ───
# 226号 P3 重现修复：四分法是通用语法规则（no-unnecessary-escalation.md），不应 ceremony-gated
# 原 058号谱系 ceremony 专用 → 去门控化为全场景覆盖
CONFIRM_PATTERNS='待确认|以上理解是否正确|如有偏差请指出|是否现在处理|是否有新的|请确认|等待.*确认|你要我|先讨论|还是先|要不要|需不需要|是否需要|你觉得|你认为|你希望|还是认为|你要的是'
STOP_CONTENT=$(echo "$input" | python -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('stop_hook_content', d.get('content', '')))
except: pass
" 2>/dev/null || true)
# 排除 /escalate 上下文中的合法问号（选择类/语法记录类上浮是允许的）
IS_ESCALATE=0
if echo "$STOP_CONTENT" | grep -q '/escalate' 2>/dev/null; then
    IS_ESCALATE=1
fi
if [ "$IS_ESCALATE" -eq 0 ] && echo "$STOP_CONTENT" | grep -qP "$CONFIRM_PATTERNS" 2>/dev/null; then
    python -c "
import json
print(json.dumps({
    'decision': 'block',
    'reason': '[Stop-Guard] 检测到四分法违规：输出包含确认请求/选择上浮模式。行动类事项直接执行，不提问。立即执行格式A：输出 → 接下来：[具体动作] 并紧跟工具调用。'
}, ensure_ascii=False))
"
    exit 0
fi

# ─── 全部检查通过：写入 session 后允许停止 ───
cd "$cwd" 2>/dev/null || true
bash scripts/write_session.sh >/dev/null 2>&1 || true
rm -f "$COUNTER" 2>/dev/null || true
exit 0
