#!/bin/sh
# CI 镜像同步检查（#810，2026-07-30 立）
#
# 背景：本仓 `main` 不推 origin——远端 `main` 是另一条祖传历史（与本地 main
# 零共同祖先），本地这条重写过的线以 `main-rewritten` 这个名字发布，CI 触发器
# 挂在它上面。于是存在一个静默失效模式：**提交落在 main，忘了推镜像 ⟹ CI 一直
# 在旧树上把关，而且是绿的。**
#
# 实事故（本脚本的成因，非举例）：2026-07-30 查出镜像落后主线 **545 个提交**。
# 期间 CI 全绿。镜像追上后当场暴露三条此前不可见的红：
#   1. `cargo fmt --check` 6 文件 11 处漂移 —— #611 那道闸装了也响了，只是响在别处；
#   2. pytest `test_morse_landscape` LFS pointer 分支炸而非 skip（#818）；
#   3. 两条 Lean↔Rust 一类买点对拍断言（#811，教义分歧，不许修绿）。
#
# 口径：**只提醒，不阻断**（exit 0）。理由——推镜像是对外动作，按仓内纪律须人
# 批准，脚本不替人决定推不推；它只保证「落后」这件事不会没人知道。
#
# 用法：
#   直接跑        sh scripts/check_mirror_sync.sh
#   挂 post-commit  在 .git/hooks/post-commit 里加一行（勿删已有的 git-lfs 行）：
#                   sh "$(git rev-parse --show-toplevel)/scripts/check_mirror_sync.sh"

set -e

MIRROR=origin/main-rewritten
WORKLINE=main

git rev-parse --verify --quiet "$MIRROR" >/dev/null || exit 0
git rev-parse --verify --quiet "$WORKLINE" >/dev/null || exit 0

AHEAD=$(git rev-list --count "$MIRROR".."$WORKLINE" 2>/dev/null || echo 0)
[ "$AHEAD" -eq 0 ] && exit 0

# 只有 CI 真正把关的那两块落后时才出声——纯文档漂移不值得每次提交都刷屏。
GATED=$(git diff --name-only "$MIRROR".."$WORKLINE" -- rust/ .github/ formal/ src/ tests/ scripts/ | head -1)
[ -z "$GATED" ] && exit 0

printf '\n\033[33m⚠ CI 镜像落后主线 %s 个提交，且落后面含 CI 把关的目录。\033[0m\n' "$AHEAD"
printf '  CI 现在把关的是 %s（%s），不是你刚提交的树。\n' "$MIRROR" "$(git rev-parse --short "$MIRROR")"
printf '  推镜像（快进；属对外动作，需人批准）：\n'
printf '    git push origin %s:main-rewritten\n\n' "$WORKLINE"

exit 0
