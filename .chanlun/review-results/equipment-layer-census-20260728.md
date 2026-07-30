# 装备层四套目录被引用面盘点（#509 wayfinder research 供数）

- 日期：2026-07-28 ｜ 执行：只读盘点 agent ｜ 分支：main @ a3bd8ac008
- 对象：`.claude/`、`.agents/`、`deploy/`、`agent/`
- 口径：名字级 diff + 内容级抽检（SKILL.md diff）；git 跟踪状态以 `git status` 为准

---

## 1. 各目录内容清单

### 1.1 `.claude/`（tracked，主体在 git 内）
| 子目录 | 数量 | 说明 |
|---|---|---|
| `skills/` | 82 项 = 58  symlink + 24 实体目录 | **58 个 symlink 全部指向 `../../.agents/skills/<name>`**；24 个实体目录为 .claude 独有（见下） |
| `commands/` | 7 | build-fix / code-review / plan / refactor-clean / tdd / update-docs / verify（ECC 系） |
| `rules/` | 2 子目录 11 文件 | `common/` 6（coding-style, git-workflow, hooks, patterns, security, testing）+ `python/` 5（coding-style, hooks, patterns, security, testing）；**根级 rules 已全部被删（见 §4）** |
| `agents/` | **0（空）** | 2026-07-10 swarm-uninstall 时已归档（settings.local.json 授权记录佐证） |
| `hooks/`、`archive/`、`worktrees/`、`settings.json`、`settings.local.json` | — | hooks + 60+ 个 agent worktree |

.claude 独有的 24 个实体 skill（无 symlink、.agents 中无对应）：brainstorming, dialectical-trading-system, dispatching-parallel-agents, domain-conventions, domain-principles, executing-plans, finishing-a-development-branch, gemini-math, knowledge-crystallization, math-tools, options-selector, plan-review, project-topology, receiving-code-review, requesting-code-review, subagent-driven-development, systematic-debugging, test-driven-development, tradingview-chanlun, using-git-worktrees, using-superpowers, verification-before-completion, writing-plans, writing-skills（多为 superpowers/项目自研系）。

### 1.2 `.agents/`（**untracked，`??`**）
- 仅 `skills/`，54 个实体目录。全部带 `agents/openai.yaml`（Codex 接口元数据）。
- **是物理正本**：`.claude/skills` 的 58 个 symlink 中 54 个解析到此处；与 `.claude` 同名 skill 内容 54/54 全同（逐文件 diff 验证）。

### 1.3 `agent/`（**untracked，`??`**）
- 仅 `skills/`，41 个实体目录。带 `agents/openai.yaml`，且 SKILL.md frontmatter 被改写为 Codex schema（去掉 `name:`、`disable-model-invocation`，description 加引号）——与 .agents 同名文件**内容不同**（格式转换版，非拷贝）。
- 集合 = skills-lock.json 的 58 − 17：缺 13 个 Leonxlnx/taste-skill 前端审美系（brandkit, design-taste-frontend×2, full-output-enforcement, gpt-taste, high-end-visual-design, image-to-code, imagegen-frontend-mobile/web, industrial-brutalist-ui, minimalist-ui, redesign-existing-projects, stitch-design-taste）+ 4 个 lock 有但 .agents 缺失的（见 §3 drift）。即 **agent/ = 纯工程系子集**。

### 1.4 `deploy/`（tracked）
| 子目录 | 数量 | 名字 |
|---|---|---|
| `agents/` | 6 | meta-lead, genealogist, quality-guard, source-auditor, meta-observer, topology-manager（蜂群工位卡组） |
| `commands/` | 4 | ceremony, inquire, escalate, ritual（元编排仪式） |
| `rules/` | 3 | no-workaround, result-package, testing-override |
| `skills/` | 1 | meta-orchestration（SKILL.md + README + references/2 模板） |
- DEPLOY.md 描述的部署包清单还列了 `CLAUDE.md.example`、`install.sh`、`hooks/`，**当前工作区均不存在**（包不完整，或与文档漂移）。

## 2. 重叠度（名字级 diff）

- `.claude` ∩ `.agents` = **54**，且 54/54 内容逐字节相同（symlink 天然保证）。
- `.claude` ∩ `agent` = 41；`.agents` ∩ `agent` = 41；`agent` ⊂ `.agents`（同名但 frontmatter 被转换，内容异）。
- `.claude` 独有 28（24 实体 + 4 个**悬空 symlink**：to-issues, review, decision-mapping, to-prd —— lock 有记录但 .agents 实体已丢失）。
- `deploy/` 与其余三套**零重叠**（不同命名空间：元编排 vs 工程技能）。
- 结论：**不是四份拷贝，是「1 正本 + 2 投影 + 1 独立包」**：`.agents/skills` 正本 → `.claude/skills` symlink 投影（Claude Code）+ `agent/skills` 格式转换投影（Codex，工程子集）；`deploy/` 是元编排分发包，独立。

## 3. 被谁引用（消费者判定）

| 目录 | 消费者 | 证据 |
|---|---|---|
| `.claude/` | **Claude Code**（主）；**Kimi/Copilot/Codex 间接** | settings.json/settings.local.json 为 Claude Code schema；根 AGENTS.md 明示「skills 在 `.claude/skills/`，agents 在 `.claude/agents/`」，让所有非 Claude harness 指回此处；`.kimi-code/mcp.json` 只注册 claude-audit MCP，不消费 skill 目录 |
| `.agents/skills` | **skills CLI（mattpocock）正本存储**；Codex 元数据来源 | skills-lock.json（58 条 hash lock，根目录、tracked、`M` 未提交）记录 github 来源；每个 skill 内嵌 `agents/openai.yaml`；`.claude/skills` symlink 指向它。untracked → **正本不在 git 里，靠 lock 可重建** |
| `agent/skills` | **Codex / OpenAI harness**（工程子集、Codex schema frontmatter） | openai.yaml + 转换版 frontmatter；与 .agents 同日生成（2026-07-22 02:35 同批安装）；untracked |
| `deploy/` | **Claude Code 全局层（`~/.claude/`）分发包** | DEPLOY.md 全文：cp 到 `~/.claude/{rules,commands,skills,agents}`；仓内 `.claude/` 不含其内容（meta-orchestration 未装到项目级），即 deploy 的消费者是**用户级 Claude Code，不是本仓任何 harness 的运行时** |

lock drift：skills-lock.json 58 条 vs `.agents/skills` 54 实体 —— 差 decision-mapping, review, to-issues, to-prd（= 4 个悬空 symlink），疑似卸载了一半（lock 未同步）。

## 4. `.claude/rules/` 删除面判定

- 实际删除 **12 个文件 / 633 行**（git-context 记 13，实测 12），全部 **unstaged、仅工作区**：
  - ECC 系 2：`common/agents.md`、`common/performance.md`
  - 项目自研 10：formalization-validity-domain, lead-parallel-dispatch, llm-role-boundary, **model-allocation**, no-patch-mentality, no-unnecessary-escalation, no-workaround, post-commit-flow, result-package, testing-override
- 最近历史：`14f7ddcf7b`（2026-07-04）「模型分配规则**永久固化**（编排者三令）」——删除面里就有这份「永久令」。
- 迁移证据核查：CLAUDE.md / AGENTS.md / CONTEXT.md 均**无**这些规则的正文或引用；无 commit、无文档记录本次删除；scripts/ 里 13 处注释仅以文化标签形式提及规则名（不读文件）。
- 混杂性：删 ECC 残留（agents/performance）像有意清理；但同批删掉「编排者永久令」model-allocation 与 no-workaround 等核心禁令，与在案意图直接冲突。
- **判定：疑似事故残留或半途重构，不像已裁定的有意重构。** 若是重构，缺「内容迁移去向 + 一笔说明 commit」；建议编排者裁定前不要 `git add` 该删除面。注：deploy/rules/ 仍保有 no-workaround / result-package / testing-override 三份，可恢复其中 3 件；其余 9 件只能从 git HEAD 恢复。

## 5. 唯一源（canonical）建议

**技能层：canonical = `.agents/skills/`（事实正本已是它），但必须收进 git。**
- 理由：① 物理现实——`.claude/skills` 58 个 symlink 已指向它，内容 54/54 全同，它已是单一实体源；② 跨 harness 中立——`.agents/` 是 AGENTS.md 生态约定位置，Claude（symlink）与 Codex（openai.yaml + agent/ 转换）都从它派生；③ 当前它 untracked 是最大的账实不符：正本不在版本控制内，靠 skills-lock.json 才能重建，且 lock 已 drift 4 条。
- 收编动作：a) `git add .agents/skills`，修复或注销 4 个悬空 symlink（lock↔实体对齐）；b) `.claude/skills` 保持 symlink 投影（Claude Code 消费面不动），24 个 .claude 独有实体 skill 或迁入 .agents 或声明为 Claude-only 例外；c) `agent/skills` 声明为**生成物**（Codex 投影），加进 .gitignore 或由安装脚本重建，杜绝手工改它造成三源漂移。

**元编排层：canonical = `deploy/`（已是 tracked 分发包），保持独立。**
- 理由：与技能层零重叠、消费者是用户级 `~/.claude/`，定位是「分发源」而非「运行时」；仓内 `.claude/agents` 已空（2026-07-10 卸载），说明项目级运行时有意不含工位卡组。
- 收编动作：补齐 DEPLOY.md 声称但缺失的 `install.sh` / `CLAUDE.md.example` / `hooks/`，或在 DEPLOY.md 删除这些行，消除文档漂移。

**rules 层：canonical 待定，先裁定 §4 删除面。** 若裁定删除生效，则 `.claude/rules/{common,python}`（ECC 系）为剩余正本；若恢复，建议项目自研 rules 单设一处（如 `.claude/rules/` 根级保留）并在 AGENTS.md 登记索引，deploy/rules 三件与之建立「谁是谁的源」的显式声明（当前 deploy/rules 的 no-workaround 等与被删根级 rules 同名，疑似同源两拷）。

## 6. 关键风险清单
1. **正本 untracked**：`.agents/`（54 skill）+ `agent/`（41 skill）均不在 git，机器丢失即靠 lock 重建（且 lock 已 drift 4）。
2. **12 文件未提交删除**含「编排者永久令」，性质未定。
3. **4 个悬空 symlink**：to-issues / review / decision-mapping / to-prd 在 Claude Code 面不可加载。
4. `deploy/DEPLOY.md` 与包内容漂移（install.sh 等 3 件缺失）。
5. `.claude/agents/` 空目录 + settings 里残留 swarm 授权，与 deploy 工位卡组的关系无文档说明。
