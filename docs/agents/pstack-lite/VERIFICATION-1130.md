# pstack-lite 01 靶向验证记录（票 #1130）

> 本文件记录本票验收命令的**实际执行结果**。命令只覆盖 Skill 发现与触发夹具，不运行任何
> 与 Skills 无关的全仓重放。

## 环境

- 分支：`sandcastle/issue-1130`
- prime-agent：`/usr/local/lib/node_modules/prime-agent`（沙盒镜像内，版本 0.7.2）
- 触发夹具实跑模型：`deepseek` / `deepseek-v4-pro`（沙盒内可用；kimi-coding 在本沙盒
  报 402 membership，故不采用）

## ① 发现 + 零 warning 检查

```bash
node scripts/pstack-lite/check-skills.mjs
```

结果：

```text
cwd: /home/agent/workspace
skills: 72 (project 56, user 16)
diagnostics: 0
unslop: absent
```

- 零 warning、零 collision，exit 0。
- `unslop: absent`：全局 `unslop` 安装在主机 `~/.agents/skills/unslop/`（#1127），本沙盒
  镜像内不可见。主机上跑 `node scripts/pstack-lite/check-skills.mjs --expect-unslop`
  会要求该项存在。

## ② shadow / warning 检测自测

```bash
node scripts/pstack-lite/check-skills.mjs --self-test
```

结果（exit 0）：

```text
self-test[合法 Skill 零诊断加载]: PASS
self-test[名字与目录不符产生 warning]: PASS
self-test[空描述产生 warning 且不加载]: PASS
self-test[同名 shadow 产生 collision（项目级胜出）]: PASS
self-test[shadow 后仅保留项目级副本]: PASS
```

覆盖：项目级 Skill 正常加载；名字/描述违规产生 warning；同名 shadow 产生 collision 且
项目级胜出（遮蔽用户级全局副本）。

## ③ 触发夹具分析器自测

```bash
node scripts/pstack-lite/run-trigger-fixtures.mjs --self-test
```

结果（exit 0）：`工具调用读 SKILL.md → loaded`、`模型自报不算加载证据 → not loaded`、
`toolResult 含 Skill 路径 → loaded` 三项全 PASS。证明判定来自会话记录的工具面证据，不采信
模型自报。

## ④ 触发夹具实跑（探针，新 Prime 会话）

```bash
node scripts/pstack-lite/run-trigger-fixtures.mjs --item probe-fixture
```

结果（exit 0，6/6 PASS）：

| case | expect | 实际 | verdict |
|---|---|---|---|
| probe-pos-1 | loaded | loaded | PASS |
| probe-pos-2 | loaded | loaded | PASS |
| probe-pos-3 | loaded | loaded | PASS |
| probe-neg-1 | skipped | not loaded | PASS |
| probe-neg-2 | skipped | not loaded | PASS |
| probe-neg-3 | skipped | not loaded | PASS |

夹具前置确认：`probe-fixture` 在临时夹具项目中被 Prime 发现（registered=true），
注册诊断 0 条。

## ⑤ 固定 SHA 与许可证核对

- 上游 `pstack/LICENSE`（MIT，Copyright (c) 2026 Lauren Tan）与仓内
  `docs/agents/pstack-lite/LICENSE.MIT` 逐字节一致，SHA-256 均为
  `bc957ca6bee02792566a1a028d105e02e247c6e77cf057061674273da77b200e`。
- 上游 `pstack/skills/unslop/SKILL.md`（固定 SHA `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`）
  SHA-256 `181883e539caec8258ec9129e3ba5f133409144a2cbf2aa361158ab94cfc3441`，与 #1127
  记录一致，已登记于 `docs/agents/skill-local-customizations.md`。
- 本表固定 SHA `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`，无任何自动跟随 `main` 机制。

## 已知边界

- 5 个业务 Skill 与 `unslop` 在 `fixtures.json` 里为 `draft`（业务 Skill 未落地），只过
  schema 校验（每项 ≥3 正例 + ≥3 反例），不实跑；后续票改 `active` 后按同一命令实跑。
- `--expect-unslop` 在本沙盒会失败（全局 unslop 不在镜像内），属预期；主机环境应通过。
