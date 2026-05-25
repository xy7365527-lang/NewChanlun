#!/bin/bash
# launchd_wrapper.sh — chanlun-soros K4 weekly pipeline 的严格 C 方案包装器
#
# 责任:
#   1. 跑 pipelines/run_weekly.py
#   2. 不论成败，原子写 last_run.json 到 NewChanlun/external/k4-soros-reports/
#   3. 成功则把 outputs/ 下最新 .md 拷贝到 reports 目录（按日期命名）
#   4. 失败则把 stderr tail 嵌入 last_run.json
#
# 谱系: spec-execution-gap 严格关闭——cron 失败必须在 sandbox 端可见

set -u  # 注意：不 set -e，要捕获 pipeline 失败而非整脚本退出

CHANLUN_SOROS_DIR="$HOME/Projects/chanlun-soros"
REPORTS_DIR="$HOME/Projects/NewChanlun/external/k4-soros-reports"
LAST_RUN="$REPORTS_DIR/last_run.json"
TODAY=$(date +%Y-%m-%d)
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
STDERR_TMP=$(mktemp)

# 确保 reports 目录存在
mkdir -p "$REPORTS_DIR"

cd "$CHANLUN_SOROS_DIR" || {
  cat > "$LAST_RUN.tmp" <<EOF
{
  "timestamp": "$TIMESTAMP",
  "date": "$TODAY",
  "exit_code": 127,
  "status": "fatal",
  "error": "chanlun-soros directory not found at $CHANLUN_SOROS_DIR",
  "stderr_tail": ""
}
EOF
  mv "$LAST_RUN.tmp" "$LAST_RUN"
  exit 127
}

# 运行 pipeline，捕获 stderr
python pipelines/run_weekly.py 2> "$STDERR_TMP"
EXIT_CODE=$?
STDERR_TAIL=$(tail -c 2000 "$STDERR_TMP" | python -c "import json,sys; print(json.dumps(sys.stdin.read()))")

# 找最新输出（按 mtime）
LATEST_OUTPUT=""
if [ $EXIT_CODE -eq 0 ] && [ -d "outputs" ]; then
  LATEST_OUTPUT=$(ls -t outputs/*.md outputs/*.json 2>/dev/null | head -1)
  if [ -n "$LATEST_OUTPUT" ]; then
    # 按日期命名拷贝（不覆盖 cron 写入的最新）
    EXT="${LATEST_OUTPUT##*.}"
    cp "$LATEST_OUTPUT" "$REPORTS_DIR/$TODAY.$EXT"
  fi
fi

# 原子写 last_run.json
STATUS="ok"
[ $EXIT_CODE -ne 0 ] && STATUS="failed"

cat > "$LAST_RUN.tmp" <<EOF
{
  "timestamp": "$TIMESTAMP",
  "date": "$TODAY",
  "exit_code": $EXIT_CODE,
  "status": "$STATUS",
  "latest_report": "$TODAY.md",
  "source_output": "$LATEST_OUTPUT",
  "stderr_tail": $STDERR_TAIL
}
EOF
mv "$LAST_RUN.tmp" "$LAST_RUN"

rm -f "$STDERR_TMP"
exit $EXIT_CODE
