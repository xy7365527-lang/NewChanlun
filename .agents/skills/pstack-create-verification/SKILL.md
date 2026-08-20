---
name: pstack-create-verification
description: "生成项目级验证 Skill：用 Prime skill-creator 规范创建 .agents/skills/verify-<app>/，启动并像用户一样驱动真实 CLI/UI/服务，留下行为证据。当仓库缺少脚本化的用户行为验证入口、且有实施票或明确授权创建文件时使用（如「生成验证 Skill」「端到端验证」「固化用户路径验证」）。仅需单测、已有 e2e/集成验证、或只想要建议时不要用——此时只提建议或开票，不写文件。"
license: MIT
metadata:
  upstream-repo: cursor/plugins
  upstream-sha: fd6dd6f7276956a532bb78a748a8d2818b6eb5f4
  upstream-path: pstack/skills/create-verification-skill/SKILL.md
  license-file: LICENSE.MIT
---

# 生成行为验证 Skill

每个严肃项目都需要一条脚本化的方式去驱动真实应用并证明行为：启动它、像用户一样走一遍功能、留下证据。这个 Skill 在项目缺少这种「真实行为验证入口」时，用 Prime `skill-creator` 的规范生成一个项目级验证 Skill（`.agents/skills/verify-<app>/`）。生成物写给下一个 agent 冷读，而不是写给人类看：它会在任务中途被一个从没见过这应用的 agent 冷启动读取。

## 授权闸（先读，生成任何文件之前）

- 只有在**实施票**或用户**明确授权创建文件**时才写文件、生成验证 Skill。
- 没有授权：只说明「缺验证入口」这个事实，给一句建议，或开一张实施票；**不写任何文件**。
- 生成物落在**隔离 worktree 或临时 fixture**里，绝不写进未授权的主工作区；只有授权范围明确覆盖主工作区时，才把验证 Skill 复制回主工作区。

## 1. 访谈仓库，不访谈用户

从代码库回答下面这些，只有观察不到时才问用户（用普通对话提问，不用结构化问答）：

- **Surface**：用户真正触碰的是什么？Web UI、CLI/TUI、桌面应用、API、移动应用、库？一个仓库可以有几个，选主要的，其余记下来。
- **Run**：应用怎么在本地启动？优先仓库自带的开发命令（package scripts、Makefile、README 快速开始）。记下端口、环境变量、种子数据、认证。
- **Drive**：agent 怎么程序化地跟它交互？先用现成的 harness（Playwright/Cypress spec、expect 脚本、PTY helper、可 curl 的端点、调试端口），没有再选通用配方：Web/Electron 用浏览器/CDP，CLI/TUI 用 tmux/PTY harness，服务用纯 HTTP。
- **Observe**：能抓到什么证据？截图、终端转录、响应体、日志、退出码、数据库状态。
- **Isolate**：两个实例能并排跑吗（端口、数据目录、profile）？不能就在生成的 Skill 里写明：拒绝二次驱动共享实例，比破坏用户会话强。

如果 checkout 原样 build/start 不起来，先修好（或精确报告）再生成；基于坏底座的 Skill 只会教错步骤。当无关缺失资产挡住启动（API 从不服务的静态目录、样例配置），生成的 Skill 可以创建它，明确标成验证脚手架，并在 cleanup 里删掉。

## 2. 生成 Skill

用 Prime `skill-creator` 的规范写 `.agents/skills/verify-<app>/SKILL.md`：YAML frontmatter 里 `name: verify-<app>` 和一段 `description`，点名应用、surface、何时用它——没有 frontmatter 的 Skill 永不注册。各节都要落在访谈真实结论上，不留占位符：

- **Launch**：启动应用的确切命令，以及怎么知道它 ready（日志行、端口应答、prompt）。含 teardown。短命 CLI/TUI 没有常驻服务：launch 意为先 build 二进制（或装依赖），然后每次 drive 在各自隔离的 PTY/tmux 会话里起。
- **Doctor**：一条只读检查回答「这个实例值得驱动吗？」——进程在、版本/build 对、端口归我们、认证有效。任何不对时 agent 先跑它。
- **Drive**：harness 配方，用本仓库真实选择器/命令，不用示例。优先稳定句柄（ARIA label、data 属性、prompt 字符串、路由路径），不用坐标和 tab 顺序。
- **Evidence**：证据抓什么、放哪。写清证明标准：走真实用户路径，不走内部 setter 或 test-only 端点；抓动作和结果态，不只抓最后屏；副作用（写的文件、插的行、发的消息）和可见物一起验；mock 只在生产边界已经隔离了外部系统处用。安全路径是 dry-run/test mode 时，用观察（文件、网络、git ref）核实它到底跳过了什么，别信它的名字：有些 dry-run 仍碰网络或开浏览器。
- **Cleanup**：怎么拆掉这次 run 创建的实例。永不按进程名 kill；只 kill 自己启动的。cleanup 删实例和 scratch 状态，永不删证据：proof 产物在 teardown 后仍留在 Skill 点名的位置。
- **Helpers**：Skill 附带的任何脚本都要可执行，且调用写在正文里。要读者反向工程的 helper 不是 helper。

## 3. 播种 feature map

创建 `.agents/skills/verify-<app>/features/README.md`，每个可识别的用户可见 feature 一个文件（起步取前 3–5 个，来自 routes/命令/菜单/文档）。按 `references/feature-map-example/` 的形状：README 索引 + 每 feature 一文件。每个文件从用户视角回答：feature 是什么、怎么到达、怎么用 harness 驱动、什么可观测终态证明它可用。四个 H2 是 `Sub-features`、`How to get to it (user POV)`、`Driving it with <harness>`、`Gotchas`。map 是仓库维护的验证正本；只驱动一个方便入口的 proof 不完整，map 列了别的入口时就是。

## 4. 生成后先自己证明一遍再交接

端到端跑一遍它自己的指令：launch、doctor、驱动一个已映射的 feature（一个就够；map 的存在是让后续 run 覆盖其余）、抓证据、cleanup。cleanup 后确认证据还在点名位置——吃掉 proof 的 cleanup 这一关失败。失败的修掉，且每次失败迭代后也跑生成的 cleanup，别让坏尝试搁浅进程和端口。从没被执行过的生成 Skill 是草稿，不是交付物。

生成并验证后，用 Prime 的加载方式让它可被发现：新会话启动时自动发现，或在交互会话里 `/reload` 后 `/skill:verify-<app>` 显式驱动。

## 5. 维护循环

`maintain-verification-skill` 命令不在首批移植范围，不要引用。维护靠：应用变更后重跑生成的验证 Skill，并把 feature map 更新到与实际一致；用户问起再建议节奏。

## 来源

- **上游仓库**：`cursor/plugins`（GitHub，`pstack/` 目录）
- **固定 SHA**：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`（不自动跟随上游 `main`）
- **原始 Skill**：`pstack/skills/create-verification-skill/SKILL.md`（连同 `references/feature-map-example/` 下 3 个相对文件）

## 本地 Prime 改写

- **命名**：`create-verification-skill` → `pstack-create-verification`（pstack-lite 统一命名空间，避免与未来用户级同名 Skill shadow）。
- **调用契约**：上游 `disable-model-invocation: true` 移除，改为首批统一 `both`（模型可见可自动调用，同时保留 `/skill:pstack-create-verification` 显式入口）。
- **生成路径**：`.cursor/skills/verify-<app>/` → `.agents/skills/verify-<app>/`（本仓 skills canonical 为 `.agents/skills/`）。
- **生成工具**：Cursor `create-skill` 流程 → Prime `skill-creator` 的 markdown Skill 规范 + `/reload` 加载 + `/skill:<name>` 显式驱动。
- **授权闸与隔离（新增）**：新增「授权闸」节与隔离要求——无实施票/明确授权只建议或开票、不写文件；生成物落隔离 worktree/临时 fixture，不写未授权主工作区。上游无此闸，Prime 侧软只读约束不能当硬隔离用。
- **维护命令**：上游第 5 步指向 `/maintain-verification-skill`（Cursor 命令，不在首批）→ 改为重跑生成 Skill + 更新 feature map。
- **其余**：访谈、生成、feature map 形状与证明四步的决策流原样保留；`references/feature-map-example/` 三个文件逐字保留。

## 许可证

MIT（Copyright (c) 2026 Lauren Tan），见同目录 `LICENSE.MIT`。
