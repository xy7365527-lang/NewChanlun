# pstack-lite 来源说明与 MIT 许可证规范

（#1130 验收①）每个项目适配 Skill 必须携带**来源说明**与**许可证**，使维护者能审计
上游仓库、固定 SHA、原始 Skill 与本地 Prime 改写。这是 pstack-lite 各实施票共用的
模板口径，逐项内容由各票填写。

## 必填字段

| 字段 | 内容 | 位置 |
|---|---|---|
| 上游仓库 | `cursor/plugins`（GitHub，`pstack/` 目录） | frontmatter `metadata.upstream-repo` + 正文「来源」节 |
| 固定 SHA | `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4` | frontmatter `metadata.upstream-sha` + 正文「来源」节 |
| 原始 Skill | `pstack/skills/<原名>/SKILL.md`（连同其 `references/`、`scripts/` 等相对文件） | 正文「来源」节 |
| 本地 Prime 改写 | 相对上游改了什么、为什么（逐条，可追溯） | 正文「本地 Prime 改写」节 |
| 许可证 | MIT（Copyright (c) 2026 Lauren Tan） | frontmatter `license: MIT` + 同目录 `LICENSE.MIT` |

## frontmatter 约定

```yaml
---
name: pstack-<原名>
description: <做什么 + 何时触发；写明触发条件、任务、工具与短语>
license: MIT
metadata:
  upstream-repo: cursor/plugins
  upstream-sha: fd6dd6f7276956a532bb78a748a8d2818b6eb5f4
  upstream-path: pstack/skills/<原名>/SKILL.md
  license-file: LICENSE.MIT
---
```

`metadata` 是 Prime 允许的任意键值映射字段；未知字段会被忽略，但这些键只作人读审计，
不改变加载行为。`disable-model-invocation` 不在模板中预设——首批统一采用 `both`
（模型可见可自动调用，同时保留 `/skill:<name>` 显式入口），由各票按需要显式决定。

## 正文「来源」节（必写，模板见 `SKILL.md.template`）

- **上游仓库 / 固定 SHA / 原始 Skill 路径**：三行各一行，贴可点击链接与 SHA。
- **本地 Prime 改写**：逐条写「改了什么 → 为什么」，与上游逐字一致的部分写「原样保留」。
- **许可证**：引用同目录 `LICENSE.MIT`（MIT，Copyright (c) 2026 Lauren Tan）。

## 许可证文件

- `LICENSE.MIT` 是上游 `pstack/LICENSE` 的 MIT 原文，逐字节一致
  （SHA-256 `bc957ca6bee02792566a1a028d105e02e247c6e77cf057061674273da77b200e`），
  版权行保留「Copyright (c) 2026 Lauren Tan」。
- 每个适配 Skill 目录复制一份 `LICENSE.MIT`（`license-file: LICENSE.MIT` 指向同目录副本）。
- 适配版是 MIT 许可作品的派生，分发须连同许可声明。

## 升级与同步边界

- **不自动跟随上游 `main`**，不设同步机器人。升级必须：开票 → 比较旧 SHA 与新 SHA 的
  diff → 重做 Prime 适配 → 通过 `scripts/pstack-lite/check-skills.mjs` 与
  `run-trigger-fixtures.mjs` 靶向验证 → 更新本表与 `skills-lock.json`（如适用）。
- 上游删除或改名不影响本地已固定版本。`unslop` 是用户级全局副本，独立升级，不随项目
  pstack-lite 更新。
