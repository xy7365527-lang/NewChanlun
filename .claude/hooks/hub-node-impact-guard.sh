#!/bin/bash
# hub-node-impact-guard.sh — Hub 节点影响链守卫（061号+060号）
# 修改高频引用谱系节点时输出影响链警告
# 触发：PreToolUse/Write + PreToolUse/Edit

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('tool_name',''))" 2>/dev/null || echo "")

if [ "$TOOL_NAME" != "Write" ] && [ "$TOOL_NAME" != "Edit" ]; then
  exit 0
fi

FILE_PATH=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('tool_input',{}).get('file_path',''))" 2>/dev/null || echo "")

# 只检查已结算谱系
echo "$FILE_PATH" | grep -q "genealogy/settled/" || exit 0

# 提取编号
BASENAME=$(basename "$FILE_PATH" .md)
NODE_ID=$(echo "$BASENAME" | grep -oP '^\d{3}[a-z]?' || echo "")

if [ -z "$NODE_ID" ]; then
  exit 0
fi

# Hub 节点影响表（Top 5，来自 061号 DAG 分析）
# 使用环境变量传参给 python，避免 shell 注入
json_allow() {
  MSG="$1" python -c "
import json, os
msg = os.environ['MSG']
print(json.dumps({
    'hookSpecificOutput': {
        'hookEventName': 'PreToolUse',
        'permissionDecision': 'allow',
        'permissionDecisionReason': msg,
        'additionalContext': msg
    }
}))
"
}

case "$NODE_ID" in
  "020")
    json_allow "[hub-node-impact-guard] Hub 节点修改：020号（构成性矛盾）— 被 21 条谱系引用（31%覆盖面）。影响链：019d,020a,021,022,027,028,032,033,035,036,039,041,043,044,052,053,054,056,058,059,060。请确认修改的向后兼容性。"
    ;;
  "016")
    json_allow "[hub-node-impact-guard] Hub 节点修改：016号（运行时执行层）— 被 19 条谱系引用。影响链：017,019,019b,019c,019d,032,033,034,036,038,039,042,043,044,045,048,049,051,053。请确认修改的向后兼容性。"
    ;;
  "005b")
    json_allow "[hub-node-impact-guard] Hub 节点修改：005b号（对象否定对象语法）— 被 17 条谱系引用。影响链：004,005,005a,006,010,012,015,016,019c,020,024,041,042,044,046,053,054。请确认修改的向后兼容性。"
    ;;
  "013")
    json_allow "[hub-node-impact-guard] Hub 节点修改：013号（蜂群结构工位）— 被 14 条谱系引用。影响链：014,019,019a,019b,019d,020,030,032,037,039,043,045,046,060。请确认修改的向后兼容性。"
    ;;
  "033")
    json_allow "[hub-node-impact-guard] Hub 节点修改：033号（声明式dispatch-dag）— 被 9 条谱系引用。影响链：034,036,037,038,039,042,043,045,059。请确认修改的向后兼容性。"
    ;;
  *)
    exit 0
    ;;
esac
exit 0
