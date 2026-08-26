#!/usr/bin/env bash
# claude-migrate.sh —— Claude Code 数据面迁移助手（#1238）。
#
# 官方重定位入口只有一个：环境变量 CLAUDE_CONFIG_DIR。本脚本做四件事：
#   1. preflight   源/目标预检 + 空间预检（fail-closed）
#   2. baseline / final-copy   复制 ~/.claude/（内置凭据安全 excludes，rsync 幂等增量）
#   3. config-json 把 ~/.claude.json 用 jq 白名单剥成只剩 mcpServers（源侧执行，凭据不过机）
#   4. emit-env / verify / rollback   切换（打印 export）/ 结构校验 / 回滚（unset）
#
# 不碰 macOS 控制面：Desktop VM bundle、Keychain、签名包一律不复制、不 symlink（#1236）。
# 运行前提：bash 3.2+（macOS / Linux）+ rsync（baseline/final-copy）+ jq（config-json/verify）。
#
# 用法：claude-migrate.sh <子命令> [选项]
set -uo pipefail

SRC=""
DST=""
CONFIG_JSON=""
OUT=""
ENV_FILE=""
PROJECT_DIR_NAME=""
RSYNC_CMD="${RSYNC:-rsync}"
RSYNC_OPTS=(-aAX --numeric-ids --delete --partial)
INCLUDE_HISTORY=0
DRY_RUN=0

# 默认 excludes：运行时 / 易重生 / 可能含凭据的路径，一律不复制。
# `.claude.json` 只经 config-json 白名单迁移（凭据不过机），rsync 不复制也不删除。
# 依据：官方 claude-directory 页目录表 + 明文存储告警（见 RUNBOOK.md §3/§6）。
DEFAULT_EXCLUDES=(
  .claude.json
  sessions/
  shell-snapshots/
  backups/
  debug/
  image-cache/
  paste-cache/
  uploads/
  todos/
  statsig/
  logs/
  history.jsonl
  remote-settings.json
  policy-limits.json
)
EXCLUDES=("${DEFAULT_EXCLUDES[@]}")

log()  { printf '%s\n' "[claude-migrate] $*" >&2; }
die()  { printf '%s\n' "[claude-migrate] 错误: $*" >&2; exit 1; }
err()  { printf '%s\n' "[claude-migrate] 错误: $*" >&2; }

run() {
  printf '%s\n' "+ $*" >&2
  if [ "$DRY_RUN" -eq 1 ]; then return 0; fi
  "$@"
}

require_src_dst() {
  [ -n "$SRC" ] || die "缺 --src"
  [ -n "$DST" ] || die "缺 --dst"
}

require_dst() { [ -n "$DST" ] || die "缺 --dst"; }

# ── preflight ──────────────────────────────────────────────────────────────
cmd_preflight() {
  require_src_dst
  [ -e "$SRC" ] || { err "源路径不存在：$SRC"; return 1; }
  [ -L "$SRC" ] && { err "源路径是 symlink，拒绝（官方入口是 env 而非 symlink）：$SRC"; return 1; }
  [ -d "$SRC" ] || { err "源路径不是目录：$SRC"; return 1; }
  [ -L "$DST" ] && { err "目标路径不应是 symlink：$DST"; return 1; }
  if [ -e "$DST" ] && [ ! -d "$DST" ]; then
    err "目标路径已存在且不是目录：$DST"; return 1
  fi
  case "$SRC" in
    "$DST"|"$DST"/*) err "SRC 不能位于 DST 之内（$SRC vs $DST）"; return 1 ;;
  esac
  case "$DST" in
    "$SRC"|"$SRC"/*) err "DST 不能位于 SRC 之内（$DST vs $SRC）"; return 1 ;;
  esac
  if [ -n "$CONFIG_JSON" ] && [ ! -f "$CONFIG_JSON" ]; then
    err "config-json 文件不存在：$CONFIG_JSON"; return 1
  fi
  if [ "$DRY_RUN" -eq 0 ]; then
    local src_kb dst_avail_kb
    src_kb=$(du -sk "$SRC" | awk '{print $1}') || { log "du 源失败（继续，不阻断）"; src_kb=0; }
    # 空间预检只在 DST 本机可解析时生效：跨机迁移时 DST 是远端路径，df 解析不到即跳过（目标机校验兜底）。
    # 不 mkdir -p DST——preflight 只读不落盘，避免跨机时在本机误建 /srv/... 或触发权限失败。
    dst_avail_kb=$(df -Pk "$DST" 2>/dev/null | awk 'NR==2 {print $4}')
    if [ -n "${dst_avail_kb:-}" ] && [ "${src_kb:-0}" -gt 0 ] && [ "$dst_avail_kb" -lt "$src_kb" ]; then
      err "空间不足：源 $src_kb KiB > 目标可用 $dst_avail_kb KiB"; return 1
    fi
  fi
  log "preflight 通过：src=$SRC dst=$DST"
  log "注意：停写点 = 退出所有 claude 会话 + Desktop/VS Code/JetBrains（官方：它们都写 ~/.claude/）。"
}

# ── 复制（baseline / final-copy，同一实现） ────────────────────────────────
do_copy() {
  local kind="$1"
  require_src_dst
  log "$kind：$SRC/ -> $DST/"
  local -a cmd
  cmd=("$RSYNC_CMD" "${RSYNC_OPTS[@]}")
  local e
  for e in "${EXCLUDES[@]}"; do
    cmd+=("--exclude=$e")
  done
  cmd+=("$SRC/" "$DST/")
  printf '%s\n' "+ ${cmd[*]}" >&2
  if [ "$DRY_RUN" -eq 1 ]; then return 0; fi
  mkdir -p "$DST" || return 1
  "${cmd[@]}" || { log "$kind 复制失败" >&2; return 1; }
}

cmd_baseline()   { do_copy "基线复制（服务在线）"; }
cmd_final_copy() { do_copy "最终增量复制（停写后）"; }

# ── config-json：凭据白名单（源侧执行，fail-closed） ───────────────────────
cmd_config_json() {
  [ -n "$CONFIG_JSON" ] || die "缺 --config-json <源 ~/.claude.json>"
  local out
  if [ -n "$OUT" ]; then
    out="$OUT"
  elif [ -n "$DST" ]; then
    out="$DST/.claude.json"
  else
    die "缺 --out 或 --dst"
  fi
  if [ "$out" = "$CONFIG_JSON" ]; then
    die "输出不能覆盖输入（--out 与 --config-json 同路径）"
  fi
  if ! command -v jq >/dev/null 2>&1; then
    die "缺 jq（凭据白名单是强制的，不装 jq 就拒绝，绝不裸拷 ~/.claude.json）"
  fi
  if [ "$DRY_RUN" -eq 1 ]; then
    log "（dry-run）jq 白名单：$CONFIG_JSON -> $out（只保留 mcpServers）"
    return 0
  fi
  # 只保留 user 档 mcpServers；OAuth/凭据/machineID/userID/projects 一律丢弃。
  if ! jq -e '{mcpServers: (.mcpServers // {})}' "$CONFIG_JSON" > "$out"; then
    err "jq 白名单失败（源 JSON 可能非法）：$CONFIG_JSON"
    rm -f "$out"
    return 1
  fi
  log "已生成凭据白名单副本：$out（仅 mcpServers 键，勿再混入 auth）"
}

# ── emit-env：切换（打印 export，供操作员抄入 guest shell profile） ────────
cmd_emit_env() {
  require_dst
  printf 'export CLAUDE_CONFIG_DIR=%q\n' "$DST"
  if [ -n "$PROJECT_DIR_NAME" ]; then
    printf 'export CLAUDE_CODE_PROJECT_DIR_NAME=%q\n' "$PROJECT_DIR_NAME"
  fi
  if [ -n "$ENV_FILE" ] && [ "$DRY_RUN" -eq 0 ]; then
    {
      printf 'export CLAUDE_CONFIG_DIR=%q\n' "$DST"
      [ -n "$PROJECT_DIR_NAME" ] && printf 'export CLAUDE_CODE_PROJECT_DIR_NAME=%q\n' "$PROJECT_DIR_NAME"
    } > "$ENV_FILE" || { err "写 env 文件失败：$ENV_FILE"; return 1; }
    log "已写 env 片段：$ENV_FILE"
  fi
  log "回滚 = unset CLAUDE_CONFIG_DIR（macOS 旧路径自始未动）。"
}

# ── verify：结构校验（目标非空 / .claude.json 合法且无凭据键 / 无 sessions/） ──
cmd_verify() {
  require_dst
  if [ "$DRY_RUN" -eq 1 ]; then log "（dry-run 跳过校验）"; return 0; fi
  local ok=1
  if [ ! -d "$DST" ] || [ -z "$(ls -A "$DST" 2>/dev/null)" ]; then
    log "verify 失败：目标为空 $DST" >&2; ok=0
  fi
  if [ -e "$DST/sessions" ]; then
    log "verify 失败：目标含 sessions/（运行时状态，不应被复制）" >&2; ok=0
  fi
  local cj="$DST/.claude.json"
  if [ ! -f "$cj" ]; then
    log "verify 失败：缺 $cj" >&2; ok=0
  elif ! command -v jq >/dev/null 2>&1; then
    log "verify 跳过 .claude.json 键检查（无 jq）" >&2
  else
    local extra
    extra=$(jq -r 'keys[] | select(. != "mcpServers")' "$cj" 2>/dev/null)
    if [ -n "$extra" ]; then
      log "verify 失败：.claude.json 含 mcpServers 之外的键：$(printf '%s' "$extra" | tr '\n' ' ')" >&2; ok=0
    elif ! jq -e 'type == "object"' "$cj" >/dev/null 2>&1; then
      log "verify 失败：.claude.json 非法 JSON" >&2; ok=0
    fi
  fi
  [ "$ok" -eq 1 ] || return 1
  log "verify 通过：$DST 非空、.claude.json 仅 mcpServers、无 sessions/。"
}

# ── rollback：撤 env（旧路径未动，无需搬回） ────────────────────────────────
cmd_rollback() {
  if [ -n "$ENV_FILE" ] && [ -e "$ENV_FILE" ] && [ "$DRY_RUN" -eq 0 ]; then
    run rm -f "$ENV_FILE" || { err "移除 env 文件失败：$ENV_FILE"; return 1; }
  fi
  log "回滚：unset CLAUDE_CONFIG_DIR（并从 guest shell profile 移除该行）。"
  log "macOS 源侧 ~/.claude/ 全程未动，无数据搬回动作。"
}

usage() {
  cat >&2 <<'EOF'
用法：claude-migrate.sh <子命令> [选项]

子命令：
  preflight   源/目标/空间预检
  baseline    服务在线全量复制 ~/.claude/（内置 excludes）
  final-copy  停写后增量复制
  config-json 凭据白名单：~/.claude.json -> 仅 mcpServers（源侧执行）
  emit-env    打印 export CLAUDE_CONFIG_DIR=...（切换）
  verify      目标非空 + .claude.json 仅 mcpServers + 无 sessions/
  rollback    unset env（旧路径未动）

选项：
  --src <路径>              旧 ~/.claude 目录（baseline/final-copy/preflight）
  --dst <路径>              目标 CLAUDE_CONFIG_DIR（如 /srv/agents/claude）
  --config-json <文件>      旧 ~/.claude.json（config-json/preflight）
  --out <文件>              config-json 输出（默认 <dst>/.claude.json）
  --env-file <文件>         emit-env/rollback 落盘位置（默认只打印）
  --project-dir-name <名>   emit-env 时附 CLAUDE_CODE_PROJECT_DIR_NAME
  --include-history         复制 history.jsonl（默认禁：可能含粘贴明文凭据）
  --exclude <glob>          追加排除（可重复）
  --rsync <路径>            rsync 可执行（默认 $RSYNC_CMD；沙盒对拍用 fake）
  --rsync-opts <'...'>      追加 rsync 选项（整体一个参数，按空白拆分）
  --dry-run                 只打印不执行
EOF
  exit 2
}

# ── 参数解析 ────────────────────────────────────────────────────────────────
COMMAND=""
while [ $# -gt 0 ]; do
  case "$1" in
    preflight|baseline|final-copy|config-json|emit-env|verify|rollback)
      if [ -z "$COMMAND" ]; then COMMAND="$1"; else log "子命令只能给一个（已给 $COMMAND）" >&2; exit 2; fi
      ;;
    --src) SRC="${2:-}"; shift ;;
    --dst) DST="${2:-}"; shift ;;
    --config-json) CONFIG_JSON="${2:-}"; shift ;;
    --out) OUT="${2:-}"; shift ;;
    --env-file) ENV_FILE="${2:-}"; shift ;;
    --project-dir-name) PROJECT_DIR_NAME="${2:-}"; shift ;;
    --include-history) INCLUDE_HISTORY=1 ;;
    --exclude) EXCLUDES+=("${2:-}"); shift ;;
    --rsync) RSYNC_CMD="${2:-}"; shift ;;
    --rsync-opts) read -r -a _rsync_extra <<< "${2:-}"; RSYNC_OPTS+=("${_rsync_extra[@]}"); shift ;;
    --dry-run) DRY_RUN=1 ;;
    -h|--help) usage ;;
    *) log "未知参数：$1" >&2; usage ;;
  esac
  shift
done

[ -n "$COMMAND" ] || usage

# --include-history：从默认 excludes 移除 history.jsonl
if [ "$INCLUDE_HISTORY" -eq 1 ]; then
  _kept=()
  for e in "${EXCLUDES[@]}"; do
    [ "$e" = "history.jsonl" ] || _kept+=("$e")
  done
  EXCLUDES=("${_kept[@]}")
fi

case "$COMMAND" in
  preflight)   cmd_preflight ;;
  baseline)    cmd_baseline ;;
  final-copy)  cmd_final_copy ;;
  config-json) cmd_config_json ;;
  emit-env)    cmd_emit_env ;;
  verify)      cmd_verify ;;
  rollback)    cmd_rollback ;;
  *) usage ;;
esac
