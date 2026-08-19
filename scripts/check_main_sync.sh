#!/bin/sh
# origin/main 同步检查（#810 历史事故；#1101 / #1103 现行口径）
#
# 现行基线：`main` 是唯一现役本地线与远端默认分支，CI 触发器只看
# `origin/main`。本地 main 已合入、但尚未推到 origin/main 时，CI 仍会在旧树上
# 把关；本脚本在落后面含 CI 把关目录时提醒推送。
#
# 历史：#810 于 2026-07-30 查出旧 `main-rewritten` 镜像落后本地主线 545 个
# 提交，期间 CI 全绿；追平后才暴露 fmt、LFS 与 Lean↔Rust 对拍三类红。
# #1101 将远端 main 与现役谱系并轨后，镜像特例与旧脚本名一并退役。
#
# 口径：只提醒，不阻断（exit 0）。只有先证明 `origin/main` 是本地 `main`
# 的祖先，才会给出快进 push 命令；本地落后或两线分叉时只告警。推送是对外
# 动作，脚本不替人执行。调用前先 `git fetch origin main`，保证远端读数新鲜。
#
# 用法：
#   直接跑        sh scripts/check_main_sync.sh
#   挂 post-commit  在 .git/hooks/post-commit 里加一行（勿删已有的 git-lfs 行）：
#                   sh "$(git rev-parse --show-toplevel)/scripts/check_main_sync.sh"

set -e

REMOTE_MAIN=origin/main
WORKLINE=main

git rev-parse --verify --quiet "$REMOTE_MAIN" >/dev/null || exit 0
git rev-parse --verify --quiet "$WORKLINE" >/dev/null || exit 0

# 给出 push 命令前，先机械证明它会是快进。落后或分叉不能沿用“本地领先”
# 的提醒，否则会向人提供一个注定被拒绝或需要强推的命令。
if ! git merge-base --is-ancestor "$REMOTE_MAIN" "$WORKLINE"; then
  if git merge-base --is-ancestor "$WORKLINE" "$REMOTE_MAIN"; then
    BEHIND=$(git rev-list --count "$WORKLINE".."$REMOTE_MAIN")
    printf '\n\033[33m⚠ 本地 main 落后 origin/main %s 个提交。\033[0m\n' "$BEHIND"
    printf '  未证明 origin/main 是本地 main 的祖先；请先同步并复核，不提供 push 命令。\n\n'
  else
    printf '\n\033[31m⚠ 本地 main 与 origin/main 已分叉。\033[0m\n'
    printf '  无法证明快进关系；须人工裁定谱系，不提供 push 命令。\n\n'
  fi
  exit 0
fi

AHEAD=$(git rev-list --count "$REMOTE_MAIN".."$WORKLINE")
[ "$AHEAD" -eq 0 ] && exit 0

# 只有 CI 真正把关的目录落后时才出声；纯文档漂移不刷 post-commit 输出。
GATED=$(git diff --name-only "$REMOTE_MAIN".."$WORKLINE" -- rust/ .github/ formal/ src/ tests/ scripts/ | head -1)
[ -z "$GATED" ] && exit 0

printf '\n\033[33m⚠ origin/main 落后本地 main %s 个提交，且落后面含 CI 把关目录。\033[0m\n' "$AHEAD"
printf '  CI 现在把关的是 %s（%s），不是本地 main（%s）。\n' \
  "$REMOTE_MAIN" "$(git rev-parse --short "$REMOTE_MAIN")" "$(git rev-parse --short "$WORKLINE")"
printf '  已证明 origin/main 是本地 main 的祖先。\n'
printf '  推送主线（快进；属对外动作，需人批准）：\n'
printf '    git push origin %s:main\n\n' "$WORKLINE"

exit 0
