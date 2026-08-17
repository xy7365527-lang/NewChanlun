# Sandcastle 踩坑实录（TROUBLESHOOTING）

汇总 #996 / #1004 / #1008 三票实测踩过的坑及对策。每条按「症状 → 成因 → 对策」记，出处标票号。
新坑按同格式追加，并注明来源票。

## 1. colima 只挂 /Users：/tmp 下运行 bind-mount 不可见

- **症状**：在 `/tmp` 下的目录里跑 sandcastle，容器内看不到 worktree（bind-mount 挂载失败或挂空）。
- **成因**：本机容器运行时是 colima（brew formula，免管理员密码，#996 用它替代票面的 Docker Desktop），其 VM 默认只把 `/Users` 挂进虚拟机；`/tmp` 在 VM 里不可见，docker bind-mount 自然拿不到宿主文件。
- **对策**：sandcastle 的运行落点（仓 checkout / worktree 父目录）必须在 `/Users` 之下。改 Dockerfile 等不涉及；这是运行位置约束。（#1008 事故 2；colima 选型见 #996。README 只记了 colima 前置，/Users 落点约束以本节为准）

## 2. `gh --json` 在 FORCE_COLOR 下输出染色，JSON 解析炸

- **症状**：编排脚本里 `gh ... --json ...` 的 stdout 混进 ANSI 色码，`JSON.parse` 报错。
- **成因**：环境里有 `FORCE_COLOR`（或等价着色强制变量）时，gh 即使 `--json` 也会给输出上色。
- **对策**：`main.mts` 内调 gh 前洗掉 env（删 `FORCE_COLOR` / 设 `NO_COLOR=1`），不要把宿主着色环境原样传进去。（#1008 事故 3）

## 3. `sandcastle docker build-image` 镜像名按 cwd 派生

- **症状**：build 出来的镜像名不是预期的 `sandcastle:newchanlun`，后续 prebake / 冒烟拿错或拿不到镜像。
- **成因**：不传 `--image-name` 时，sandcastle 按当前工作目录名派生镜像名——换目录跑就换名。
- **对策**：任何 `build-image` 调用都显式 `--image-name sandcastle:newchanlun`，镜像名全仓钉死这一个。（#1004 坑③、#1008 口径）

## 4. 仓配了 git-lfs hook：镜像里没装 git-lfs 时 checkout 直接失败

- **症状**：沙盒内 `git checkout --detach`（或任何触发 post-checkout 的操作）退出码 2，报 "'git-lfs' was not found on your path"。
- **成因**：本仓 `.git/hooks/post-checkout` 是 git-lfs 安装的 hook，找不到 `git-lfs` 即 `exit 2`；`.gitattributes` 里确有 LFS 路径（如 `.chanlun/block-topology/relations.jsonl`、`docs/chanlun/text/tiandao/*.pdf`）。
- **对策**：① 镜像装 `git-lfs`（Dockerfile 已装）；② `.sandcastle/.env` 设 `GIT_LFS_SKIP_SMUDGE=1`，防 checkout 时把 LFS 实体大文件拉进沙盒。（#996 偏差 3）

## 5. uv 必须预装进镜像：无 TTY 环境不会交互安装

- **症状**：沙盒内 prime-agent 首次用 ipython 工具时 kernel 自举失败或卡住。
- **成因**：prime-agent 的 IPython kernel 自举依赖 uv；裸镜像里没有 uv 时会走交互安装（PRIME_AGENT_INSTALL_UV 机制），而沙盒无 TTY，交互永远等不到输入。
- **对策**：Dockerfile build 期 `curl -LsSf https://astral.sh/uv/install.sh | sh` 预装，并保证 `/home/agent/.local/bin` 在 PATH。（#996 偏差 4）

## 6. `/tmp/prime-agent-*` 残留烤进镜像 → 新容器 daemon rename 撞 EXDEV

- **症状**：预烤镜像起的新容器里 prime-agent daemon 启动失败，报 `EXDEV`（rename 跨设备）。
- **成因**：做 kernel 预烤（见下条）时，一次性容器跑真实 bootstrap 会在 `/tmp` 留下 `prime-agent-*` 运行时残留；若直接 `docker commit` 固化，这些目录进了镜像 overlayfs 下层。新容器启动 daemon 时对同名路径做 rename，撞上 overlayfs 下层目录即 EXDEV。
- **对策**：预烤流程必须是「跑一次真实 bootstrap → **同进程**清理运行时残留（含 `/tmp/prime-agent-*`）→ `docker commit`」。清理要与 agent 同生命周期——`docker start` 重跑已停止的容器会重新拉起 agent，分开跑等于没清。脚本正本：`.sandcastle/prebake-kernel.sh`（#1004 坑①②，注释里有原文）。

## 7. IPython kernel 预烤：runtimeIdentity 哈希必须与运行期逐位一致

- **症状**：明明预烤了 kernel venv，沙盒内首次 ipython 调用仍要自举 1-2 分钟——预烤静默失效。
- **成因**：prime-agent 用 `BOOTSTRAP_VERSION_FILE` 里的 runtimeIdentity 哈希判定 kernel venv 是否过期；哈希输入含 `XDG_DATA_HOME`、pythonSkills JSON 等。build 期复刻 bootstrap 时任何一个输入与运行期不一致，运行期就判定过期、重建 venv，预烤白做。
- **对策**：不要在 Dockerfile 里手工复刻 bootstrap。用 #1004 落地的方案：起一个一次性容器（API key 仅运行时注入）跑一次**真实** bootstrap，让哈希输入天然与运行期一致；随后同进程清理残留（见上条），`docker commit` 固化并用 `--change` 清空镜像 Env（防密钥进 image config，`docker inspect` 复核为「无」）。每次改 Dockerfile 重建镜像后，跑一次 `.sandcastle/prebake-kernel.sh` 恢复预烤。验收读数：干净容器完整首跑（daemon 启动 + API 往返 + ipython 调用）约 10 秒级。（#1004）

## 8. 其他在案坑位（同源三票，防重复踩）

- **prime-agent 不在公共 npm registry**：Dockerfile 不能 `npm install -g prime-agent`，用官方 R2 tarball，版本与宿主机对齐（当前 v0.7.2）。（#996 偏差 2）
- **Node ESM 解析不到全局包**：编排侧（宿主机）须在仓根 `npm install --save-dev @ai-hero/sandcastle tsx`，全局装的 sandcastle 在 `npx tsx .sandcastle/main.mts` 里 import 不到。产生的 `package.json` / `package-lock.json` 已随 #996 入仓。（#996 偏差 5）
- **git 身份不由 sandcastle 处理**：`GIT_AUTHOR_NAME` / `GIT_AUTHOR_EMAIL` / `GIT_COMMITTER_NAME` / `GIT_COMMITTER_EMAIL` 写进 `.sandcastle/.env`（sandcastle 自动解析注入）；kimi-coding 的 key 变量名是 `KIMI_API_KEY`，同样走 `.env`，不落任何已跟踪文件。（#996）
- **上游 0.12.0 open bug 簇**（#999 调研登记，#1008 施工时逐条对）：provider env 丢失（上游 #925+#900，自定义 provider 的 KIMI_API_KEY 注入面会命中，冒烟必须验证沙盒内 env 真的到了）；hooks 退出码被吞（上游 #943，onSandboxReady 失败不 fail-fast）；timeoutMs 被忽略（上游 #907）。
- **拾取闸用共享 label 会抢票**：首跑用 `ready-for-agent` 做拾取闸，把并行会话的 #1013 认领了。修正为专属 label `sandcastle`；分支确定性命名 `sandcastle/issue-<票号>`（#1003 裁定），不用时间戳名。（#1008 事故 1）
- **`TARGET_BRANCH` 是内置 promptArg**：不可覆盖，编排脚本里不要试图传同名参数。（#1008）

## 9. 双管线竞态：认领闸幂等挡不住 TOCTOU（2026-08-17 实撞）

**症状**：两条 main.mts 同时跑，几乎同时 claim 同一张票（摘 label + assign 都是幂等 no-op），两个沙盒同时开干同一分支。

**成因**：pickIssue 的「查未认领 → 认领」不是原子操作——查询与认领之间无锁（#1013 抢票教训修的是 label 面，TOCTOU 窗口仍在）。

**判例处置**：后启动的一条**杀自己、留先启动的**，回报对方。判定先来后到 = 进程启动时间。

**预防**：跑新管线前先 `ps aux | grep -E "[t]sx .sandcastle/main.mts"`——有活进程就不要再起（常驻管线可能一直活着，日志会轮转到 main-loop 文件，别误判「停了」）。

## 10. 主机登出会 SIGTERM 杀整条管线（2026-08-17 实撞）

**症状**：工蜂日志同秒停写、容器被清、实装无 commit、票被 re-queue。

**成因**：macOS loginwindow 用户登出 → launchd 清理用户会话 → 主机 npm exec tsx 进程树收 SIGTERM（nohup 只挡 SIGHUP 不挡这个）。

**判读纪律**：先查日志停写时间是否对齐登出时刻，再判「工蜂卡死」。**判例处置**：中断后把未 commit 的票 re-queue（恢复 sandcastle label + unassign），重启管线即可续。

## 11. 宿主驱动死亡 ≠ 工蜂死亡（bind mount 下的存活判定，2026-08-17 实撞）

**症状**：宿主 npm exec tsx 进程死亡（appDeath，非登出），但沙盒容器与容器内 prime-agent 工蜂仍在干活——工作区是宿主 bind mount，工蜂 commit 直接落回宿主分支。

**判读**：中断后先问「工蜂死了吗」再套 re-queue 纪律——宿主驱动死亡只影响 Phase 2 评审的自动派发（不会派、沙盒不会自动关），不影响 Phase 1 工蜂产出。

**处置**：工蜂活着 ⟹ 不杀、不 re-queue，盯到 commit 或 90 分钟无产出再回报。工蜂 commit 后 Phase 2 评审需人工补派（`sandcastle review <sandbox>` 或重跑两段式主流程由编排层定）。
