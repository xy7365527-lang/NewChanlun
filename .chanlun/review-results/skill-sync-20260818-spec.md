# Spec 轴评审：skill 同步（ticket-919-final 工作树，.agents/skills/ 13 文件）

结论：**通过**。验收口径四要素全部核实成立；1 处 Spec 前提例外；无范围蔓延、无实现可疑点。

## (a) Spec 要求但缺失/不完整的

- 「9 个 skill 逐文件等于同步前 user 侧副本」——**已核实成立**。同步前真实副本尚存于
  `~/.agents/skill-conflict-backup-20260818/`（冲突备份），对 9 个 skill 逐一 `diff -r`
  备份目录 vs 工作树：13 个文件全部逐字节一致，且文件集相同（无多/缺文件）。
- 前提例外：Spec 第 1 行「其余同名 skill 内容一致、不动」对 **research** 不成立——
  user 侧多一个 `DESCRIPTION.md`（179B），仓库无、亦未同步。research 不在 9 个名单内，
  故不违反「不得有其他改动」，但属遗留不一致，Spec 前提与实际不完全相符。

## (b) diff 里 Spec 没要求的（范围蔓延）

- **无**。`git diff HEAD -- .agents/skills/` 13 个文件全部落在 9 个名单 skill 内；
  无新增/删除/未跟踪文件；`skills-lock.json`（仓库根）零改动；仓库 `.agents/skills/`
  内无符号链接（user 侧 9 个目录均为指向仓库的 symlink，符合「symlink 在仓库外」）。
- diff 方向自洽：`-` 行均为仓库旧措辞（PRDs / one question at a time），`+` 行为较新
  措辞（specs / rounds / frontier / design tree），与「user 侧是较新版本」一致。
- 注：工作树另有 `.sandcastle/`、`.chanlun/agent-roster-*` 改动，属本次评审目标之外。

## (c) 看起来实现了但实现可疑的

- **无**。
- 特别核对项 `implement/SKILL.md` 新增 `disable-model-invocation: true`：与同步前备份
  逐字节一致，**确为 user 侧原样内容**（非手改注入）。
- 语义影响（如实陈述，不裁决）：本机安装的 prime-agent
  `dist/core/skills.js` 注释明确——该旗标使 skill **从系统提示/模型可见技能集中排除
  （不进模型目录）**，仅能经用户显式 `/implement` 唤起；skill 仍保留在磁盘及用户命令
  列表。另据 `.chanlun/agent-roster-20260814.md`，此旗曾为本机定制刻意移除，本次同步
  将其恢复为上游形态。
