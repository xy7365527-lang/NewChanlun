# pstack-lite 触发夹具规范

（#1130 验收③④）触发夹具在**新的 Prime 会话**里跑每个正例/反例，然后从会话的实际记录
判断 Skill 是否真的被加载，**不接受模型自报**。

## 夹具定义

夹具是 `fixtures.json`（与本文同目录），schema 见该文件与
`scripts/pstack-lite/run-trigger-fixtures.mjs` 的 `validateFixtures()`。关键约束：

- 每个 item 必须有 `id`、`skill`、`status`（`active` / `draft`）、
  `positive`（正例）与 `negative`（反例）。
- 可选 `skill_dir`：实跑时复制进临时夹具项目的 Skill 目录（相对本目录）；缺省解析到
  `fixtures/<skill>`，只适用于探针等放在 `fixtures/` 下的条目。可选 `scope: "user"`
  表示用户级全局 Skill（如 `unslop`），实跑时从 `~/.agents/skills/<skill>/` 取目录。
- **每项至少 3 个正例 + 3 个反例**，schema 校验强制。
- 每个 case 有 `id`、`prompt`、`expect`（`loaded` / `skipped`）。
- `assertions` 字段预留，后续票可往里加**端到端行为断言**（例如「回复必须逐字包含某
  token」「受保护内容必须逐字不变」），runner 目前不消费它，只作结构化预留。

## 判断依据（证据语义）

`prime-agent -p` 的会话记录是 `--session-dir` 目录下的 `<id>.jsonl`（含 session 头 +
message 事件）。`run-trigger-fixtures.mjs` 的 `analyzeSession()` 只采信两类**工具面**
证据，判定 Skill 的目录路径（含 `SKILL.md`）是否出现：

1. assistant 的 `toolCall`（`arguments.code` / `arguments.command` / `name`）；
2. `toolResult` 的 `toolName`、`content[].text`、`details.stdout` / `details.stderr`。

**不算证据**（防止模型自报）：

- assistant 的 `text`（「我已经加载了 X」）—— 不作为加载依据；
- assistant 的 `thinking`；
- user 消息里的 prompt（prompt 本身提到 skill 名不证明加载）。

判定：正例 `expect=loaded` 要求会话记录里出现该 Skill 目录路径；反例 `expect=skipped`
要求不出现。正例 + 反例全部 PASS 才算该项通过。

## 探针夹具（机制自证）

`fixtures/probe-fixture/` 是唯一 `status=active` 的条目。它把 marker token 的**值**
只放在 SKILL.md 正文（description 只提 token 名），所以模型不读文件就拿不到值——这保证
「加载」有可观察副作用。`run-trigger-fixtures.mjs --self-test` 再用合成记录验证分析器
本身（含「模型自报不算证据」这条）。

## 实跑与记录

```bash
# 全部 active 条目（当前只有探针）
node scripts/pstack-lite/run-trigger-fixtures.mjs --provider <p> --model <m> --out /tmp/report.json

# 只跑某一条
node scripts/pstack-lite/run-trigger-fixtures.mjs --item probe-fixture

# 分析器自测（不依赖模型、不依赖网络）
node scripts/pstack-lite/run-trigger-fixtures.mjs --self-test
```

每次实跑会为每个 case 起一个全新会话（`--session-dir` 独立临时目录），跑完分析并清理。
provider/model 默认 `deepseek` / `deepseek-v4-pro`（沙盒内可用）；主机环境用
`PRIME_AGENT_PROVIDER` / `PRIME_AGENT_MODEL` 或 `--provider` / `--model` 指定实际可用档。

## pstack-blast-radius 降级记录（#1138）

`pstack-blast-radius` 的原 3+3 自动触发夹具已从 `active` 降为 `draft`。多轮真实宿主测试在
修正漏触发后仍随机误加载不同反例，超过 #1129 的单项降级阈值；因此 Skill 现为
`disable-model-invocation: true` 的 user-only 能力，只保留 `/skill:pstack-blast-radius` 显式入口。
原 3+3 不删除，作为未来重新启用语料；只有在**至少两个模型上重复稳定通过**后，才可恢复
自动路由。#1139 十任务试点不计其自动触发，只能把显式调用作为对照。

## code-review adversarial 路由降级记录（#1137）

`code-review-adversarial` 的自动 3+3 夹具已从 `active` 降为 `draft`。当前宿主真实读数为 4/6：
三个反例都正确跳过，但 `adversarial review` 与 `challenge/stress test` 两个显式语义正例未加载，
只有 `tear this apart: multi-model review` 加载。因此 adversarial 子模式在首批改为
`/skill:code-review` **explicit-only pilot**；高风险、争议或关键词不再自动升级，标准
Standards/Spec 双轴仍正常自动路由。原 3+3 保留为重启语料；只有在至少两个模型上重复稳定
触发、且 3-selector/2-owner 行为门可真实运行后，才可恢复 `active` 与自动升级。#1139 只试点
标准 code-review；显式 adversarial 调用仅作对照。

## code-review 模型多样性门夹具（#1137）

`adversarial-gate-fixtures.json` 用结构化的 `selector` / `provider` / `base_family` 候选
锁定十一条纯确定性场景：从 4 个不同候选中选择 3 个且覆盖 2 个基础模型所有者桶成功、
4 输入含 1 个精确重复并去重为 3 个成功、同厂 OpenAI 仍只有 1 桶而不可用、跨两个所有者
桶时同 family 的不同版本与推理档均合法、同 selector 元数据冲突不可用、发现集跨桶但选中
三名 reviewer 同桶时不可用、跨厂启动失败，以及 admission 已成功但缺真实 child 结果。
另以两条 Prime Inference 聚合 provider 场景锁定 owner 分桶：ZAI 与 Alibaba 算两桶，同一
基础 owner 的不同版本与推理档不重复计数。运行：

```bash
node scripts/pstack-lite/check-adversarial-gate.mjs
```

该检查只读取 JSON 与 `code-review` 文档，不调用 `rlm`、`prime-agent`、网络或真实模型；它
同时锁定 `unavailable` / `degraded` 显式状态、标准 Standards/Spec 回退、session-dir/最终
JSONL 回流和「admission 不是结果」契约。

## 后续票接缝

1. 业务 Skill 落地后，把 `fixtures.json` 里对应 item 的 `status` 改为 `active`，
   并用 `skill_dir` 指向已安装的 Skill 目录（业务 Skill 在 `.agents/skills/<name>/`，
   须显式给 `skill_dir`，缺省只解析到 `fixtures/<skill>`），再按实际触发语义校准
   `positive` / `negative` 的 prompt（仍保持 ≥3+3）。
2. 需要行为断言时，往该 item 的 `assertions` 加端到端断言；runner 消费 `assertions` 的
   实现属于后续票。
3. 实跑结果写入 `docs/agents/pstack-lite/results/`（或票内 resolution comment），不依赖
   模型口头确认。
