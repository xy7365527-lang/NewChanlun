#!/usr/bin/env bash
# auto_ceremony.sh — 自主 ceremony 触发脚本
#
# 由 OpenClaw cron 每30分钟调用。功能：
# 1. 运行 ceremony_scan.py --summary 检测工位
# 2. 检查 .ceremony-step / ceremony_wal.json 防止重复触发
# 3. 有工位且无正在执行的 ceremony → 启动 claude CLI 执行 /ceremony
# 4. 日志追加到 tmp/auto_ceremony.log
#
# 幂等性保证：
# - 重复运行安全（检查锁文件）
# - 日志追加不覆盖
# - ceremony 结束后锁自动清除（由 ceremony_state.py clear_step 负责）

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_FILE="${PROJECT_ROOT}/tmp/auto_ceremony.log"
CEREMONY_STEP_FILE="${PROJECT_ROOT}/.ceremony-step"
CEREMONY_WAL_FILE="${PROJECT_ROOT}/.chanlun/ceremony_wal.json"
SCAN_SCRIPT="${PROJECT_ROOT}/scripts/ceremony_scan.py"

timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

log() {
    echo "[$(timestamp)] $*" >> "${LOG_FILE}"
}

# 确保日志目录存在
mkdir -p "$(dirname "${LOG_FILE}")"

# ── 1. 检查是否有 ceremony 正在执行 ──

if [[ -f "${CEREMONY_STEP_FILE}" ]]; then
    log "SKIP: .ceremony-step exists — ceremony already in progress"
    exit 0
fi

if [[ -f "${CEREMONY_WAL_FILE}" ]]; then
    # WAL 文件存在意味着 ceremony 正在执行或异常中断
    # 不自动触发新 ceremony，让下次手动 ceremony 处理恢复
    log "SKIP: ceremony_wal.json exists — ceremony in progress or needs recovery"
    exit 0
fi

# ── 2. 运行 ceremony_scan --summary ──

log "START: running ceremony_scan.py --summary"

scan_output=$(cd "${PROJECT_ROOT}" && python "${SCAN_SCRIPT}" --summary 2>&1) || {
    log "ERROR: ceremony_scan.py failed with exit code $?"
    log "OUTPUT: ${scan_output}"
    exit 1
}

log "SCAN: ${scan_output}"

# ── 3. 检查是否有工位待处理 ──

if echo "${scan_output}" | grep -q "工位待处理"; then
    log "TRIGGER: workstations found, launching claude /ceremony"

    # 启动 claude CLI 执行 /ceremony
    # --print: 输出到 stdout（非交互模式）
    # 工作目录必须是项目根
    ceremony_output=$(cd "${PROJECT_ROOT}" && claude --print "/ceremony" 2>&1) || {
        exit_code=$?
        log "CEREMONY_EXIT: claude exited with code ${exit_code}"
        # 非零退出不一定是错误（ceremony 可能正常结束但有 escalate）
    }

    # 记录 ceremony 输出的最后20行（避免日志膨胀）
    log "CEREMONY_DONE: last 20 lines of output:"
    echo "${ceremony_output}" | tail -20 >> "${LOG_FILE}"

    log "END: ceremony execution complete"
else
    log "IDLE: no workstations found, nothing to do"
fi

# ═══════════════════════════════════════════════════════════════
# ESCALATE INTEGRATION (v219-swarm/escalate-integrate)
# ═══════════════════════════════════════════════════════════════
#
# 已实装：scripts/escalate_ceremony_hook.py
#   用法: python scripts/escalate_ceremony_hook.py --message "矛盾描述" --source "谱系号"
#   Exit codes: 0=L1(继续), 1=L2/L3(已发送,应挂起), 2=L2/L3(发送失败)
#   配置: ~/.openclaw/openclaw.json → escalate.target_conversation
#
# 未实装的缺口（详见 topological-computation/docs/escalate-integration-gaps.md）：
#   - 回复路由：escalate 回复如何持久化并路由回挂起的工位
#   - 工位挂起：hook exit code 1 → ceremony lead 调用 suspend 的集成
#   - 超时策略：发送超时/回复超时的降级行为
# ═══════════════════════════════════════════════════════════════
