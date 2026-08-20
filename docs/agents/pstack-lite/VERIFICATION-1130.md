# pstack-lite 01 靶向验证记录（票 #1130）

> 本文件记录本票验收命令的**实际执行结果**。命令只覆盖 Skill 发现与触发夹具，不运行任何
> 与 Skills 无关的全仓重放。

## 环境

- 分支：`sandcastle/issue-1130`
- prime-agent：`/usr/local/lib/node_modules/prime-agent`（沙盒镜像内，版本 0.7.2）
- 触发夹具实跑模型：`deepseek` / `deepseek-v4-pro`（沙盒内可用；kimi-coding 在本沙盒
  报 402 membership，故不采用）

## ① 发现 + 诊断分类检查

```bash
node scripts/pstack-lite/check-skills.mjs
```

结果（沙盒镜像，Prime 0.7.2）：

```text
cwd: /home/agent/workspace
skills: 72 (project 56, user 16)
rawDiagnostics: 0
expectedProjectionCollisions: 0
unexpectedDiagnostics: 0
unslop: absent
```

- 沙盒 raw diagnostics=0、expected projection=0、unexpected=0，exit 0。
- `unslop: absent`：全局 `unslop` 安装在主机 `~/.agents/skills/unslop/`（#1127），本沙盒
  镜像内不可见。主机上跑 `node scripts/pstack-lite/check-skills.mjs --expect-unslop`
  会要求该项存在。

**宿主既定 symlink 投影（重要口径）**：真实宿主的 `~/.agents/skills/<name>` 是指向本仓
项目级 `.agents/skills/<name>` 的 symlink 投影（既定布局），`unslop` 本身是唯一用户级
副本、无项目副本。宿主的 Prime（0.7.3）会把每个投影登记为一条 collision（沙盒 0.7.2 会
静默去重、不产生 collision，故「沙盒 raw diagnostics=0」不能代表宿主）。`check-skills.mjs`
把这些「用户级 symlink 投影到项目级同名 Skill」分类为 `expectedProjectionCollisions`
（打印/JSON 留痕，但不判失败），**合入门是 `unexpectedDiagnostics=0`**——不是「宿主 raw
零 warning」。所有非投影诊断（真实 shadow / warning / error）仍进入 `unexpectedDiagnostics`
并判失败；`unslop` 与 `pstack-*` 的任何碰撞不享受投影例外，一律失败。

## ② shadow / warning 检测 + 投影分类自测

```bash
node scripts/pstack-lite/check-skills.mjs --self-test
```

结果（exit 0）：

```text
self-test[shadow]
  PASS — 合法 Skill 零诊断加载
  PASS — 名字与目录不符产生 warning
  PASS — 空描述产生 warning 且不加载
  PASS — 同名 shadow 产生 collision（项目级胜出）
  PASS — shadow 后仅保留项目级副本
  PASS — 真实 shadow + warning 全部判 unexpected（不享受投影例外）
self-test[projection]
  PASS — 用户级 symlink 投影 -> expected（不计为新错误）
  PASS — 真实非投影 shadow -> unexpected
  PASS — 投影形态的项目级 unslop 仍 -> unexpected
  PASS — 投影形态的 pstack-* collision 仍 -> unexpected
```

覆盖：项目级 Skill 正常加载；名字/描述违规产生 warning；同名 shadow 产生 collision 且
项目级胜出（遮蔽用户级全局副本）；真实 shadow/warning 一律判 unexpected；用户级 symlink
投影判 expected；`unslop` / `pstack-*` 的投影不享受例外、仍判 unexpected。投影自测与真实
非投影 shadow 自测分开（两个独立临时夹具）。

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
- `--expect-unslop` 在本沙盒会失败（全局 unslop 不在镜像内），属预期；主机环境应通过
  （unslop 为唯一用户级副本、无项目副本，`unexpectedDiagnostics=0`）。
