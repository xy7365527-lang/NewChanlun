FROM node:22-bookworm

# 系统依赖（python3/venv 供 prime-agent 的 IPython kernel 用）
RUN apt-get update && apt-get install -y \
  git \
  git-lfs \
  curl \
  jq \
  python3 \
  python3-venv \
  && rm -rf /var/lib/apt/lists/*

# GitHub CLI（正式 prompt 模板用 gh 读 issue）
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
  | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
  && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
  | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
  && apt-get update && apt-get install -y gh \
  && rm -rf /var/lib/apt/lists/*

# UID/GID 对齐（sandcastle docker build-image 默认传宿主机 UID/GID）
ARG AGENT_UID=1000
ARG AGENT_GID=1000
RUN groupmod -o -g $AGENT_GID node && usermod -o -u $AGENT_UID -g $AGENT_GID -d /home/agent -m -l agent node

# prime-agent CLI（不在公共 registry，用官方 R2 tarball；版本与宿主机对齐）
RUN npm install -g https://pub-728493de92a943e2a9b2d17b4719f318.r2.dev/releases/v0.7.2/prime-agent-0.7.2.tgz

USER ${AGENT_UID}:${AGENT_GID}

# uv：prime-agent 的 IPython kernel 自举依赖；沙盒无 TTY，不会交互安装，必须预装
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
ENV PATH="/home/agent/.local/bin:$PATH"

WORKDIR /home/agent

# worktree 模式下 sandcastle 把 git worktree 挂到 /home/agent/workspace 并以其为 cwd。
ENTRYPOINT ["sleep", "infinity"]