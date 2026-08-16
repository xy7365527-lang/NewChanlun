FROM node:22-bookworm

# 系统依赖（python3/venv 供 prime-agent 的 IPython kernel 用；
# build-essential/pkg-config 供本仓 rust/ 的 cargo check/build —— #1010）
RUN apt-get update && apt-get install -y \
  git \
  git-lfs \
  curl \
  jq \
  python3 \
  python3-venv \
  build-essential \
  pkg-config \
  ca-certificates \
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

# ---------- #1010：工具链扩容 ----------

# Rust 工具链（rustup stable + cargo；rust-analyzer 供 serena 的 rust LSP）。
# 本仓 rust/ = 单 crate（pyo3 0.23 + serde，默认 feature 无 C 依赖），
# cargo check 只需 build-essential + python3（pyo3 build script 找解释器）。
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --default-toolchain stable --profile default --component rust-analyzer \
  && /home/agent/.cargo/bin/rustup component add rust-src
ENV PATH="/home/agent/.cargo/bin:$PATH"

# serena server（容器内自起 streamable-http daemon 127.0.0.1:8017；宿主 daemon 容器内不可达）。
# 宿主机为 1.6.2.dev0（dev 构建），镜像取最近正式版 1.7.0，漂移已在 #1010 报告登记。
RUN uv tool install 'serena-agent==1.7.0'

# codebase-memory-mcp：npm 包装的 native runtime（postinstall 从 GitHub Releases
# 拉 linux/arm64 二进制）。版本钉宿主机同款 0.8.1。
# npm 全局 prefix 放 agent home（非 root 不可写 /usr/local）。
ENV NPM_CONFIG_PREFIX=/home/agent/.npm-global
ENV PATH="/home/agent/.npm-global/bin:$PATH"
RUN npm install -g codebase-memory-mcp@0.8.1 \
  && mkdir -p /home/agent/.local/bin \
  && ln -sf /home/agent/.npm-global/bin/codebase-memory-mcp /home/agent/.local/bin/codebase-memory-mcp

# cli-hub-mcp：server.py 拷入 + venv 重建（venv 不可跨平台拷贝；
# cli_hub 包 = 公共 PyPI 的 cli-anything-hub，fastmcp 同装）。
COPY --chown=${AGENT_UID}:${AGENT_GID} .sandcastle/image/cli-hub-mcp/server.py /home/agent/.agents/mcp/cli-hub-mcp/server.py
RUN uv venv /home/agent/.agents/mcp/cli-hub-mcp/.venv \
  && uv pip install --python /home/agent/.agents/mcp/cli-hub-mcp/.venv/bin/python \
     fastmcp 'cli-anything-hub==0.4.1'

# 三个 MCP 的 prime-agent Python skill（user 级 skills 目录，prime-agent 自动发现
# 并装进 kernel venv；stdio 型 MCP 在 prime-agent 里由 Python 侧 skill 承载）。
COPY --chown=${AGENT_UID}:${AGENT_GID} .sandcastle/image/skills/ /home/agent/.agents/skills/

# prime-agent 设置种子：mcpServers 登记（与宿主 ~/.prime/agent/settings.json 同构，
# 路径换容器内；provider 凭据走 .sandcastle/.env 注入，不落镜像）。
COPY --chown=${AGENT_UID}:${AGENT_GID} .sandcastle/image/prime-agent-settings.json /home/agent/.prime/agent/settings.json

# ---------- /#1010 ----------

WORKDIR /home/agent

# worktree 模式下 sandcastle 把 git worktree 挂到 /home/agent/workspace 并以其为 cwd。
ENTRYPOINT ["sleep", "infinity"]
