# pstack-lite 触发夹具规范

（#1130 验收③④）触发夹具在**新的 Prime 会话**里跑每个正例/反例，然后从会话的实际记录
判断 Skill 是否真的被加载，**不接受模型自报**。

## 夹具定义

夹具是 `fixtures.json`（与本文同目录），schema 见该文件与
`scripts/pstack-lite/run-trigger-fixtures.mjs` 的 `validateFixtures()`。关键约束：

- 每个 item 必须有 `id`、`skill`、`status`（`active` / `draft`）、
  `positive`（正例）与 `negative`（反例）。
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

## 后续票接缝

1. 业务 Skill 落地后，把 `fixtures.json` 里对应 item 的 `status` 改为 `active`，
   按实际触发语义校准 `positive` / `negative` 的 prompt（仍保持 ≥3+3）。
2. 需要行为断言时，往该 item 的 `assertions` 加端到端断言；runner 消费 `assertions` 的
   实现属于后续票。
3. 实跑结果写入 `docs/agents/pstack-lite/results/`（或票内 resolution comment），不依赖
   模型口头确认。
