# #1010 镜像工具链扩容：验收报告（issue1010-image-toolchain）

> 票：xy7365527-lang/NewChanlun #1010（task，#1006 裁定面落地）
> 执行：2026-08-16，worktree `/tmp/nc-1010`（branch `ticket-1010-toolchain`），镜像 `sandcastle:newchanlun`
> 口径：本票只到「镜像与凭据就绪 + 逐条 AC 留证」；**不合入 main**（合入走人工闸）。

## 一、改动清单

### 1. Rust 工具链（AC-1）
- Dockerfile 追加（agent 用户下）：rustup stable（cargo 1.97.1 / rustc 1.97.1）+ `rust-analyzer` 组件 + `rust-src`；
- 系统依赖追加 `build-essential pkg-config ca-certificates`（本仓 `rust/` = 单 crate，pyo3 0.23 + serde，默认 feature 无 C 依赖；pyo3 build script 需要 python3，镜像本有）。
- 未按 CI 依赖面额外装库——`cargo check` 实测零缺库报错（见验收），故无增补。

### 2. serena（AC-2，容器内自起形态）
- `uv tool install serena-agent==1.7.0`（宿主为 1.6.2.dev0 dev 构建，镜像取最近正式版——**版本漂移登记**）；
- rust-analyzer 由 rustup 组件提供（`rust-analyzer 1.97.1`）；
- 接入方式：Python skill 复刻宿主 `~/.agents/skills/serena` → 镜像 `/home/agent/.agents/skills/serena`，**一处适配**：`.serena/` 在本仓被 gitignore，sandcastle worktree 里没有 `project.yml`，skill 首次调用时在 git root（无 git 则 cwd）自动生成最小 `project.yml`（rust+python）再拉起 daemon（`127.0.0.1:8017`，容器内自起，宿主 daemon 不可达问题由此绕过）。

### 3. codebase-memory（AC-3）
- 宿主二进制是 macOS arm64 Mach-O（269MB），不可入 Linux 镜像；其二进制来源查实：公共 npm 包 `codebase-memory-mcp`（postinstall 从 GitHub Releases `DeusData/codebase-memory-mcp` 拉平台 native runtime）。镜像内 `npm install -g codebase-memory-mcp@0.8.1`（钉宿主同款版本；npm 全局 prefix 置 `/home/agent/.npm-global` 避 root 写 /usr/local），并 symlink 到 `~/.local/bin/codebase-memory-mcp` 与宿主路径对齐；
- Python skill 复刻宿主同款 → 镜像，**一处适配**：二进制解析加 PATH fallback（npm 安装位与宿主 `~/.local/bin` 不同）。

### 4. cli-hub-mcp（AC-4）
- venv 不可跨平台拷贝 → 镜像内重建：`server.py` 从宿主 `~/.agents/mcp/cli-hub-mcp/` 拷入（构建资产 `.sandcastle/image/cli-hub-mcp/server.py`），`uv venv` + `uv pip install fastmcp cli-anything-hub==0.4.1`；
- **`cli_hub` 包溯源结论**：不在 `cli-hub` 名下，而在公共 PyPI 的 **`cli-anything-hub`**（0.4.1，HKUDS/CLI-Anything，MIT），依赖仅 click+requests；server.py 另需 fastmcp；
- 新写薄 Python skill `cli-hub-mcp`（stdio 承载，与宿主 settings.json stdio 登记同构）→ `/home/agent/.agents/skills/cli-hub-mcp`。

### 5. provider 凭据（AC-5）
- env 映射从 prime-agent bundle（`getApiKeyEnvVars`，dist/bundle/chunk-6GUVYK6P.js）查实：
  `deepseek→DEEPSEEK_API_KEY`、`zai→ZAI_API_KEY`、`moonshotai/moonshotai-cn→MOONSHOT_API_KEY`、`kimi-coding→KIMI_API_KEY`（#996 已配）；
- 三把 key 从宿主 `~/.prime/agent/auth.json` 静默追加进**主仓检出** `.sandcastle/.env`（脚本写文件，全程未打印值）；
- 镜像 agent home 种子 `settings.json`（`.sandcastle/image/prime-agent-settings.json` → `/home/agent/.prime/agent/settings.json`）：defaultProvider kimi-coding/k3 + 三个 mcpServers 登记（serena=http 127.0.0.1:8017/mcp，另两个 stdio）——与宿主 `~/.prime/agent/settings.json` 同构（宿主仅有 cli-hub 一条 stdio；stdio 型在 prime-agent 架构里由 Python 侧 skill 承载，见「坑-1」）。

## 二、验收证据（逐条 AC）

### AC-1 沙盒内 cargo
```
cargo 1.97.1 (c980f4866 2026-06-30) / rustc 1.97.1 / rust-analyzer 1.97.1
```
- `cargo check`（本仓 `rust/` 全量即单 crate，**默认 features 口径**：pyo3+serde；nautilus 等 optional feature 未开）：源以 `git archive HEAD rust` 从本 worktree 取、容器内解包构建（colima 只挂 $HOME，`/tmp/nc-1010` 不可直接 bind-mount——操作口径登记），`CARGO_TARGET_DIR` 指容器内临时目录，**未碰主仓检出 target/**：
```
warning: `newchan_rust` (bin "strict_nest_check") generated 9 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 48.22s
CHECK_EXIT=0 ELAPSED=49s
```
耗时 49s，远低于 10 分钟上限，未裁剪任何目标。

### AC-2 三个 MCP 沙盒内 prime-agent 可见可调
- 可见：`prime-agent skills` 输出含 `codebase_memory` / `serena` / `cli-hub-mcp` 三条（修复坑-1 后复测）；
- 可调（server 级直连，容器内）：codebase-memory stdio → `list_projects` 返回 `{"projects":[]}`（14 tools）；cli-hub stdio → `search("image")` 返回 comfyui/gimp 等（8 tools）；serena streamable-http → daemon 4s 起、29 tools、`find_symbol("target_fn")` 经 rust-analyzer 命中 `src/lib.rs:0`；
- 可调（prime-agent 端到端，容器内 `prime-agent -p --provider kimi-coding --model k3`，事件流 `/tmp/nc1010-run2.jsonl`）：agent 在 ipython 内核依次 `import codebase_memory → list_projects()`、`import cli_hub_mcp → search("gimp")`、`import serena → list_tools()` **全部一次成功、无 sys.path workaround**，serena daemon 在无 git 仓库的 cwd 下自动 provisioning 拉起。

### AC-3 provider 探针（容器内 `prime-agent -p --mode json --provider <p> --model <m> '只回复ok'`）
| provider | model | 结果 |
|---|---|---|
| deepseek | deepseek-v4-flash | ✅ `text:"ok"`，stopReason stop |
| zai | glm-5.2 | ✅ `text:"ok"`（responseModel 实为 glm-5.3；走 coding 端点） |
| moonshotai | kimi-k3 | ❌ **401 Invalid Authentication——见「坑-3」，源 key 本身已失效，非沙盒问题** |

（zai 首探 glm-5.2-highspeed 得 429「订阅计划不含 Highspeed」，换 glm-5.2 即过——订阅面登记。）

### AC-4 豁免与密钥零命中
- `git check-ignore -v .sandcastle/.env` 命中（主仓与本 worktree 均：`.sandcastle/.gitignore:1:.env`）；
- `git grep -F` 以四把 key 原值扫**主仓与本 worktree全部已跟踪文件**：8/8 零命中（脚本静默比对，未打印值）。

## 三、坑与残留

1. **宿主 codebase-memory skill 的 SKILL.md frontmatter 是坏 YAML**（`description` 内含 ASCII `Triggers on: ` 触发「Nested mappings are not allowed in compact mappings」），prime-agent 的 skill loader 静默跳过该 skill——**宿主侧同样坏**（宿主 kernel-venv 无 codebase_memory  editable 安装，`prime-agent skills` 不可见；此前「可用」应是走了手工 sys.path 等旁路）。镜像内副本已修为 `>-` 折叠标量。**残留**：宿主原件 `~/.agents/skills/codebase-memory/SKILL.md` 未动（非本票范围），建议后续同样修复。
2. **colima 只 bind-mount $HOME**：`/tmp/nc-1010` 这类 /tmp worktree 无法直接挂进容器。本次验证用 `git archive` 管道注入源。sandcastle 正式跑法不受影响（其 worktree 在仓内 `.sandcastle/worktrees/`，属 $HOME）。**后续若把工作树放 /tmp 需知此限**。
3. **moonshot key 失效（票面 AC 唯一未过项）**：`auth.json` 里 `moonshotai`/`moonshotai-cn` 为同一把 72 字符 key，直连 `api.moonshot.ai` 与 `api.moonshot.cn` 的 `/v1/models` **均 401**；宿主 prime-agent 同 key 同 401（非沙盒差异）。凭据迁移与 env 接线本身正确（同管线 deepseek/zai 均过）。**需要用户刷新 moonshot key 后重跑一次探针即可关此项**。
4. **版本漂移登记**：宿主 serena 1.6.2.dev0（dev 构建） vs 镜像 1.7.0（正式版）；codebase-memory-mcp 镜像钉 0.8.1 = 宿主；cli-anything-hub 钉 0.4.1 = 宿主；rust 工具链宿主为 Homebrew cargo 1.96.0 vs 镜像 rustup 1.97.1（无锁定口径，按 stable 取新）。
5. `.sandcastle/.env.example` 在主仓**未被 git 跟踪**，本票未在分支内补建（避免合入时与主仓未跟踪文件冲突）；env 映射文档暂落本报告 §1.5，建议后续票把 .env.example 纳入跟踪。
6. 镜像体积增量未精确计量（rustup + serena + npm runtime），如需控制可后续把 rust 工具链挪到独立 layer 缓存优化；本次构建两次均一次通过（构建日志 `/tmp/nc-1010-build*.log`）。

## 四、交付物

- 本 worktree commit：`.sandcastle/Dockerfile`（扩容）+ `.sandcastle/image/`（cli-hub server.py、三个 Python skill、settings 种子）+ 本报告；
- 主仓 `.sandcastle/.env`：三把 key 静默追加（gitignored，随主仓检出走，不进任何 commit）；
- 镜像 `sandcastle:newchanlun` 已按新 Dockerfile 重建（本机 colima docker）。
