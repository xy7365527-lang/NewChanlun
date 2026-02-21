#!/bin/bash
# Lead 权限切换 — 神圣疯狂的物质化（032号谱系）
#
# 用法:
#   .claude/hooks/lead-permissions.sh restrict   # ceremony 后：剥夺执行权限
#   .claude/hooks/lead-permissions.sh restore    # 逃生舱口：恢复全权限
#
# restrict 模式：Lead 只能 Read/Glob/Grep/Task/SendMessage
# restore 模式：恢复全权限（必须写谱系记录原因）

set -euo pipefail

MODE="${1:-restrict}"
SETTINGS_FILE=".claude/settings.local.json"

case "$MODE" in
  restrict)
    cat > "$SETTINGS_FILE" << 'RESTRICT_EOF'
{
  "permissions": {
    "allow": [
      "Read",
      "Glob",
      "Grep",
      "Task",
      "SendMessage",
      "TaskCreate",
      "TaskList",
      "TaskGet",
      "TaskUpdate",
      "TodoWrite",
      "ToolSearch",
      "Skill",
      "WebFetch(*)",
      "WebSearch"
    ],
    "deny": [
      "Edit",
      "MultiEdit",
      "Write",
      "Bash(*)",
      "NotebookEdit",
      "mcp__plugin_serena_serena__replace_content",
      "mcp__plugin_serena_serena__replace_symbol_body",
      "mcp__plugin_serena_serena__insert_after_symbol",
      "mcp__plugin_serena_serena__insert_before_symbol",
      "mcp__plugin_serena_serena__rename_symbol",
      "mcp__plugin_serena_serena__create_text_file",
      "mcp__plugin_serena_serena__execute_shell_command"
    ]
  }
}
RESTRICT_EOF
    echo "Lead permissions RESTRICTED (032: divine madness)"
    ;;

  restore)
    cat > "$SETTINGS_FILE" << 'RESTORE_EOF'
{
  "permissions": {
    "allow": [
      "Bash(*)",
      "WebFetch(*)",
      "WebSearch",
      "mcp__plugin_playwright_playwright__*",
      "mcp__plugin_serena_serena__*",
      "mcp__plugin_context7_context7__*",
      "mcp__plugin_figma_figma__*",
      "mcp__plugin_greptile_greptile__*",
      "mcp__ide__*"
    ],
    "deny": []
  }
}
RESTORE_EOF
    echo "Lead permissions RESTORED (escape hatch — must write genealogy)"
    ;;

  *)
    echo "Usage: $0 {restrict|restore}" >&2
    exit 1
    ;;
esac
