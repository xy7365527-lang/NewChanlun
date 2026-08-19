# pstack-lite 适配骨架（prefactor，票 #1130）

本目录是 pstack-lite 各实施票共用的 **Prime 适配骨架与验收接缝**。本票只搭骨架，不实现
任何业务 Skill。业务 Skill（`pstack-how` / `pstack-arena` / `pstack-blast-radius` /
`pstack-create-verification` / `pstack-technical-writing`）与合并能力由后续票落地。

## 文件地图

| 文件 | 作用 | 验收 |
|---|---|---|
| `SOURCE-STATEMENT.md` | 来源说明 + MIT 许可证模板规范 | ① |
| `SKILL.md.template` | 新建适配 Skill 时复制的模板 | ① |
| `LICENSE.MIT` | 上游 pstack 的 MIT 原文（Copyright (c) 2026 Lauren Tan） | ① |
| `TRIGGER-FIXTURES.md` | 触发夹具规范 + 证据语义 | ③④ |
| `fixtures.json` | 夹具定义（探针可实跑；5 业务 Skill + unslop 为 draft 预留） | ④ |
| `fixtures/probe-fixture/` | 探针夹具 Skill（自证加载判定机制） | ③ |
| `../skill-local-customizations.md` | 本地定制总表（pstack-lite 索引 + 固定 SHA） | ⑤ |
| `scripts/pstack-lite/check-skills.mjs` | Prime 递归发现 + 零 warning / shadow 检查 | ②⑥ |
| `scripts/pstack-lite/run-trigger-fixtures.mjs` | 触发夹具 runner + 会话记录证据分析 | ③④⑥ |

## 两条靶向验证命令（不跑与 Skills 无关的全仓重放）

```bash
# ① 发现 + 零 warning（+ shadow 自测）
node scripts/pstack-lite/check-skills.mjs
node scripts/pstack-lite/check-skills.mjs --self-test

# ② 触发夹具（证据来自新会话记录，不采信模型自报）
node scripts/pstack-lite/run-trigger-fixtures.mjs --self-test
node scripts/pstack-lite/run-trigger-fixtures.mjs --item probe-fixture
```

## 各实施票接缝

- **新建 `pstack-*` Skill**：复制 `SKILL.md.template` → 填 name/description/来源说明 →
  同目录放 `LICENSE.MIT`。
- **登记来源**：在 `docs/agents/skill-local-customizations.md` 的 pstack-lite 索引把该行
  状态改为「已实现」并填「本地改写」摘要。
- **验证加载**：`node scripts/pstack-lite/check-skills.mjs` 要求零 warning、无 shadow。
- **验证触发**：`fixtures.json` 对应 item 改 `active` 并校准 prompt，跑
  `node scripts/pstack-lite/run-trigger-fixtures.mjs --item <id>`。
- **升级上游**：开票 → 比较旧/新 SHA diff → 重做适配 → 更新总表固定 SHA → 重跑两条命令。
  不得自动跟随上游 `main`。
