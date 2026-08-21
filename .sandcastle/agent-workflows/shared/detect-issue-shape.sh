#!/usr/bin/env bash
#
# #1110 AC-4 第一处适配：issue shape 检测（自上游 agent-implement.yml 提取为独立脚本，
# 便于机械验证/靶向测试）。
#
# 判定（只读一次 REST issue 对象，不 paginate —— `sub_issues_summary.total` /
# `issue_dependencies_summary.blocked_by` 都是聚合数字，不存在 /30 截断问题）：
#   - shape=map     有子票的 map/PRD（sub_issues_summary.total > 0）→ 拒绝
#   - shape=blocked 原生 open blocker（issue_dependencies_summary.blocked_by > 0）→ 拒绝并标 blocked
#   - shape=leaf    leaf sub-issue（有 parent、无子票、无 open blocker）或独立票 → 允许
#
# 环境变量输入：
#   GH_REPO        "owner/repo"
#   ISSUE_NUMBER   issue 编号
#   GITHUB_OUTPUT  可选；提供时把步骤输出写入该文件，否则打印到 stdout（供测试）
set -euo pipefail

: "${GH_REPO:?GH_REPO is required}"
: "${ISSUE_NUMBER:?ISSUE_NUMBER is required}"

# `--jq` 让 gh 在输出前先跑 jq，天然无 ANSI 颜色（本机 FORCE_COLOR/CLICOLOR_FORCE
# 会让裸 `gh api` 在非 tty 下也着色，污染 jq 解析）。
shape_row="$(gh api "repos/${GH_REPO}/issues/${ISSUE_NUMBER}" \
  --jq '[.sub_issues_summary.total, .issue_dependencies_summary.blocked_by, .parent_issue_url] | @tsv')"

IFS=$'\t' read -r sub_total blocked_by parent_url <<< "$shape_row"

# 防御：jq 取不到数字时按 0 处理
case "$sub_total" in
  ''|null) sub_total=0 ;;
esac
case "$blocked_by" in
  ''|null) blocked_by=0 ;;
esac

if [ "$sub_total" -gt 0 ] 2>/dev/null; then
  shape="map"
elif [ "$blocked_by" -gt 0 ] 2>/dev/null; then
  shape="blocked"
else
  shape="leaf"
fi

parent_number=""
if [ -n "$parent_url" ] && [ "$parent_url" != "null" ]; then
  parent_number="$(printf '%s' "$parent_url" | grep -oE '[0-9]+$' || true)"
fi

if [ -n "${GITHUB_OUTPUT:-}" ]; then
  {
    echo "shape=${shape}"
    echo "sub_total=${sub_total}"
    echo "blocked_by=${blocked_by}"
    echo "parent_number=${parent_number}"
  } >> "$GITHUB_OUTPUT"
else
  printf 'shape=%s\nsub_total=%s\nblocked_by=%s\nparent_number=%s\n' \
    "$shape" "$sub_total" "$blocked_by" "$parent_number"
fi
