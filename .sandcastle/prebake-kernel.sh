#!/usr/bin/env bash
# prebake-kernel.sh —— 把 prime-agent 的 IPython kernel venv 预烤进镜像（#1004）。
# 原理：bootstrap 由 prime-agent 在首次 ipython 调用时自动完成（uv 装包，1-2 分钟）；
# 本脚本在一次性容器里触发一次真实 bootstrap（key 只在运行时注入），
# 然后 docker commit 把已配好的 ~/.prime/agent/kernel-venv 固化进镜像，
# 并用 --change 清空容器 Env，防止密钥随 commit 进镜像 config。
# 用法：.sandcastle/prebake-kernel.sh [image名]（默认 sandcastle:newchanlun）
set -euo pipefail

IMAGE="${1:-sandcastle:newchanlun}"
CONTAINER="sc-prebake-$$"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

command -v docker >/dev/null || { echo "docker 不在 PATH"; exit 1; }
[[ -f "$REPO_ROOT/.sandcastle/.env" ]] || { echo "缺 .sandcastle/.env"; exit 1; }

echo "▸ 起一次性容器触发 kernel bootstrap（约 1-2 分钟），同进程内清理运行时残留"
# 注意①：清理必须与 agent 同生命周期（docker start 重跑容器会重新拉起 agent）。
# 注意②：/tmp/prime-agent-* 若烤进镜像，新鲜容器 daemon 的 rename 会撞 overlayfs 下层目录 EXDEV（#1004 实测）。
docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run --name "$CONTAINER" --env-file "$REPO_ROOT/.sandcastle/.env" \
  --entrypoint sh "$IMAGE" -c \
  'prime-agent -p "用 ipython 工具计算 1+1，只回复结果数字"; rc=$?;
   rm -rf /tmp/prime-agent-* ~/.prime/agent/daemon-workers ~/.prime/agent/session-leases ~/.prime/agent/logs ~/.prime/agent/sessions;
   exit $rc' || {
    echo "✗ bootstrap 触发失败"; docker rm -f "$CONTAINER" >/dev/null; exit 1; }

echo "▸ commit 固化（--change 清空 Env 防密钥入镜像）"
docker commit \
  --change 'ENV KIMI_API_KEY=' --change 'ENV DEEPSEEK_API_KEY=' \
  --change 'ENV ZAI_API_KEY=' --change 'ENV MOONSHOT_API_KEY=' --change 'ENV GH_TOKEN=' \
  --change 'ENV GIT_AUTHOR_NAME=' --change 'ENV GIT_AUTHOR_EMAIL=' \
  --change 'ENV GIT_COMMITTER_NAME=' --change 'ENV GIT_COMMITTER_EMAIL=' \
  "$CONTAINER" "$IMAGE" >/dev/null
docker rm -f "$CONTAINER" >/dev/null

echo "▸ 验证：新容器首次 ipython 调用应秒级（不再现 bootstrap 等待）"
docker run --rm --entrypoint sh "$IMAGE" -c \
  'test -x ~/.prime/agent/kernel-venv/bin/python && ~/.prime/agent/kernel-venv/bin/python -c "import rlm, ipykernel; print(\"kernel venv OK\")"'
echo "✓ 预烤完成：$IMAGE"
