# pstack-lite 全套受控验收记录（GitHub Issue #1138）

> **结论：PASS——准入 GitHub Issue #1139 的 10 个真实任务试点，但带三项明示降级。**
>
> - `pstack-blast-radius` 是 `draft`、`user-only`，只允许用户通过
>   `/skill:pstack-blast-radius` 显式调用；它的多轮真实自动路由测试失败，**没有通过触发门**。
> - `code-review-adversarial` 是 `draft`、`explicit-only` 子模式；不足 3 个唯一 selector 或
>   2 个基础 owner 桶时，正确结果是 `unavailable`；启动或回流失败时是 `degraded`，随后继续
>   Standards/Spec。本文**不声称跨厂商 adversarial 成功**。
> - `unslop` 是唯一用户级全局行为，不是项目级 active trigger item；本文只采信其混合内容
>   行为证据，不把其失败的自动触发测试改写成 PASS。

本记录把实现快照与验收记录提交明确分开：

- 仓库：`xy7365527-lang/NewChanlun`
- 工作树：`/Users/silencehan/Projects/NewChanlun-1138-acceptance`
- 分支：`ticket-1138-host-acceptance`
- 实现快照：`216a3dd5d3746602b1ff85522fba192863460c5c`
- 审阅时的文档提交（amend 前）：`93b16cdd8b8e02f8ab9727e27267fa5eb84b584d`
- Prime Agent：`0.7.3`
- 上游 pstack 固定 SHA：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`
- 验收日期：2026-08-20（Asia/Shanghai）

`93b16cdd8b8e02f8ab9727e27267fa5eb84b584d` 相对实现快照只新增本文，不包含任何实现
改动；本次 amend 后提交哈希会变化，但提交范围仍只有本文。因此，下面所有实现检查都固定到
`216a3dd5d3746602b1ff85522fba192863460c5c`，文档提交与 checkout 的 `HEAD` 均不充当实现
证据。

票据正本：

- [SPEC #1129：Prime Agent pstack-lite 首批精选移植](https://github.com/xy7365527-lang/NewChanlun/issues/1129)
- [验收 #1138：运行全套受控验收](https://github.com/xy7365527-lang/NewChanlun/issues/1138)
- [试点 #1139：完成 10 个真实任务试点并作 go/no-go 裁决](https://github.com/xy7365527-lang/NewChanlun/issues/1139)

三次 `gh issue view <number> --repo xy7365527-lang/NewChanlun --json
number,title,body,comments,state,url,labels,createdAt,updatedAt,closedAt` 均 exit `0`；三张票当前均为
OPEN，#1129、#1138、#1139 的评论数组均为空。

## 1. 验收口径与证据边界

本轮遵守用户的禁令：**没有重跑任何真实模型会话，也没有跑全仓测试**。行为证据来自
`.sandcastle/acceptance-1138/` 中已经完成的真实会话；Prime loader、契约脚本、JSON、Git、
symlink、来源、SHA 与许可证等确定性实现检查一律在实现快照上运行。

为保证文档提交后仍能逐条重放，后文命令共同使用以下前置步骤：在临时 detached worktree
检出实现快照，并把未入 Git 的证据树单独指回本验收 worktree。后续命令中的
`$implementation_root`、`$evidence_root` 等变量均来自这里：

```bash
acceptance_repo=/Users/silencehan/Projects/NewChanlun-1138-acceptance
implementation=216a3dd5d3746602b1ff85522fba192863460c5c
snapshot_parent="$(mktemp -d /tmp/newchanlun-1138-snapshot.XXXXXX)"
implementation_root="$snapshot_parent/implementation"
evidence_root="$acceptance_repo/.sandcastle/acceptance-1138"

git -C "$acceptance_repo" worktree add --detach "$implementation_root" "$implementation"
test "$(git -C "$implementation_root" rev-parse HEAD)" = "$implementation"
test -d "$evidence_root"
export ACCEPTANCE_REPO="$acceptance_repo"
export IMPLEMENTATION="$implementation"
export IMPLEMENTATION_ROOT="$implementation_root"
export EVIDENCE_ROOT="$evidence_root"
```

行为证据所在的四个历史固定点全部是实现快照的祖先：

| 固定点 | 用途 | `git merge-base --is-ancestor <rev> "$implementation"` |
|---|---|---:|
| `097a91708ecbd43d501abb6c860ce0ad8bbf9f45` | how、arena fan-in、create-verification | 0 |
| `e65b439ad60bf53ac09143dce0b06cce947daf03` | arena candidate、code-review degradation | 0 |
| `a4e2a02da97ad9cbc31b2963408bd2c5b293fa95` | blast、technical-writing、unslop | 0 |
| `5075d3e0e82f2aefadf6bf8911a13c8e0cc32472` | architect 最终重试 | 0 |

实现快照的可信桥是：

1. 六项 active capability 的 frontmatter `description` 与各自成功 source commit 逐字相同；
2. 六项 fixture 对象与成功 source commit 逐对象相同；
3. source commit 与四个行为固定点都是实现快照的祖先；
4. 实现快照的 loader、契约、自测、来源与集成检查全部通过。

这组桥接检查证明“成功时的路由合同没有被后续提交改写”，**不等价于在
`216a3dd5...` 重跑了真实模型**。

## 2. 硬门一：实现快照的加载与来源

### 2.1 隔离 HOME 的项目加载

实际命令：

```bash
(
loader_home="$(mktemp -d /tmp/newchanlun-1138-loader.XXXXXX)"
env HOME="$loader_home" \
  PRIME_AGENT_DIST=/Users/silencehan/.local/lib/node_modules/prime-agent/dist \
  node "$implementation_root/scripts/pstack-lite/check-skills.mjs" \
  --cwd "$implementation_root" \
  --agent-dir "$loader_home/.prime/agent" \
  --json
loader_rc=$?
rm -rf "$loader_home"
exit "$loader_rc"
)
```

Exit `0`：`ok=true`；共发现 74 个 Skill（project 61、loader 标记为 user 的默认资源 13）；
`rawDiagnostics=0`、`expectedProjectionCollisions=0`、`unexpectedDiagnostics=0`。隔离 HOME 中
`unslop=absent`，符合预期。

随后直接使用同一 Prime `DefaultResourceLoader` 列名，以下 8/8 均存在、均为 project scope，
diagnostics 为 0：

- `pstack-how`
- `pstack-arena`
- `pstack-blast-radius`
- `pstack-create-verification`
- `pstack-technical-writing`
- `code-review`
- `codebase-design`
- `design-an-interface`

列名使用的实际命令如下；Exit `0`，8/8 project scope，diagnostics=0：

```bash
(
loader_list_home="$(mktemp -d /tmp/newchanlun-1138-loader-list.XXXXXX)"
env HOME="$loader_list_home" \
  PRIME_AGENT_DIST=/Users/silencehan/.local/lib/node_modules/prime-agent/dist \
  node --input-type=module <<'NODE'
import path from "node:path";
import assert from "node:assert/strict";
const root = process.env.IMPLEMENTATION_ROOT;
assert.ok(root);
const dist = process.env.PRIME_AGENT_DIST;
const { DefaultResourceLoader } = await import(path.join(dist, "core/resource-loader.js"));
const loader = new DefaultResourceLoader({
  cwd: root,
  agentDir: path.join(process.env.HOME, ".prime/agent"),
});
await loader.reload();
const {skills, diagnostics} = loader.getSkills();
const targets = [
  "pstack-how",
  "pstack-arena",
  "pstack-blast-radius",
  "pstack-create-verification",
  "pstack-technical-writing",
  "code-review",
  "codebase-design",
  "design-an-interface",
];
const rows = targets.map((name) => {
  const skill = skills.find((x) => x.name === name);
  assert.ok(skill);
  assert.equal(skill.sourceInfo?.scope, "project");
  return {name, scope: skill.sourceInfo.scope, filePath: skill.filePath};
});
assert.equal(diagnostics.length, 0);
console.log(JSON.stringify({
  ok: true,
  targetSkills: rows.length,
  diagnostics: diagnostics.length,
  rows,
}, null, 2));
NODE
loader_list_rc=$?
rm -rf "$loader_list_home"
exit "$loader_list_rc"
)
```

这证明实现快照的五个 pstack 目录与三个现役合并宿主都能由 Prime 的真实 loader 发现；
它不把 `draft` 等同于缺文件，`draft` 管自动路由，不管显式可发现性。

### 2.2 真实 canonical root 的 unslop 门与 worktree 限制

验收时实际命令如下。检查器固定在实现快照；canonical project、真实 `HOME` 与真实
`agentDir` 则按宿主现场读取：

```bash
env HOME=/Users/silencehan \
  PRIME_AGENT_DIST=/Users/silencehan/.local/lib/node_modules/prime-agent/dist \
  node "$implementation_root/scripts/pstack-lite/check-skills.mjs" \
  --cwd /Users/silencehan/Projects/NewChanlun \
  --agent-dir /Users/silencehan/.prime/agent \
  --expect-unslop \
  --json
```

验收时 Exit `0`：`ok=true`；130 个 Skill（project 56、user 74）；raw / expected / unexpected
diagnostics 均为 0。`unslop` 位于
`/Users/silencehan/.agents/skills/unslop/SKILL.md`，scope=`user`，`projectCopy=null`；其
SHA-256 是 `181883e539caec8258ec9129e3ba5f133409144a2cbf2aa361158ab94cfc3441`。
这里的 130 是静态报告与同一现场命令记录的验收时读数；实现侧只固定执行该测量的
`216a3dd5...` 检查器，不把 canonical checkout 冒充实现快照。canonical project、HOME 与
`agentDir` 是票外可变宿主状态，之后重放仍执行同一命令，但后续 package 注册不得反向改写
本票的时点计数。

必须同时记录 worktree 限制：canonical root 实际是
`ticket-919-final@cc4d86c1ce69c443286964df1d991e37bed1fd11`，有 17 个既有 status 条目；其中
实现快照中的 `check-skills.mjs` 在 canonical root 不存在，五个 pstack Skill 目录也为 0。因此：

- 本命令证明真实宿主的 unslop 唯一性与零诊断布局；
- §2.1 证明本验收 worktree 的 pstack 集成加载；
- **不能声称 `216a3dd5...` 已部署到 canonical root 后做过一次组合加载。**

### 2.3 实现快照的确定性契约

以下均为本轮实际命令，没有调用模型或网络：

| 命令 | Exit | 结果 |
|---|---:|---|
| `node "$implementation_root/scripts/pstack-lite/check-skills.mjs" --self-test --json`（隔离 HOME） | 0 | 10/10：shadow 6 + projection 4 |
| `node "$implementation_root/scripts/pstack-lite/run-trigger-fixtures.mjs" --self-test` | 0 | 7/7；模型自报不算证据，timeout/exit 无硬证据不得假绿 |
| `node "$implementation_root/scripts/pstack-lite/check-pstack-how-contract.mjs"` | 0 | 18/18 |
| `node "$implementation_root/scripts/pstack-lite/check-design-interface-contract.mjs"` | 0 | 34/34；含三渠道完成门、无 arena 重复扇出、单方案 codebase-design |
| `node "$implementation_root/scripts/pstack-lite/check-adversarial-gate.mjs"` | 0 | 11 cases、0 failures |
| `node "$implementation_root/scripts/pstack-lite/check-blast-radius-description.mjs"` | 0 | `ok=true`、0 failures；锁定 user-only 契约 |

### 2.4 symlink、来源、固定 SHA 与许可证

机械断言 Exit `0`：

| 项目 | 数量 | 结果 |
|---|---:|---|
| `.agents/skills/pstack-*` canonical 目录 | 5 | PASS |
| `.claude/skills/pstack-*` 相对 symlink | 5 | 全部为 `../../.agents/skills/<name>`，realpath 对齐 |
| frontmatter `upstream-repo` / `upstream-sha` / `upstream-path` / `license-file` | 5/5 | PASS |
| `## 来源` 与 `## 本地 Prime 改写` | 5/5 | PASS |
| MIT 文件 | 6 | 基准 1 + 五份副本逐字节相同 |
| MIT bytes / SHA-256 | 1067 / `bc957ca6bee02792566a1a028d105e02e247c6e77cf057061674273da77b200e` | PASS |
| architect 等价来源说明 | 2 | `codebase-design` + `design-an-interface` |
| interrogate/adversarial 来源文件 | 5 | MODE + reviewer prompt + rubric + code quality + lead judgment |

五项 upstream path 分别是：

- `pstack/skills/how/SKILL.md`
- `pstack/skills/arena/SKILL.md`
- `pstack/skills/blast-radius/SKILL.md`
- `pstack/skills/create-verification-skill/SKILL.md`
- `pstack/skills/technical-writing/SKILL.md`

JSON 语法门使用实现快照的 `git ls-files` 选出的 4 个 pstack JSON，加
`trigger-success/` 的 7 个权威 JSON；11/11 `JSON.parse` 成功，Exit `0`。一次更宽的探索性
扫描覆盖 183 个 evidence/implementation-tree JSON 时，发现
`.sandcastle/acceptance-1138/arena/e2e-openai-cli.json` 是早期失败尝试留下的 0-byte
artifact，因而该宽扫描 Exit `1`（182 个可解析、1 个空文件）。本记录保留该事实，不删除或
伪装修复历史失败；硬门只覆盖实现快照 JSON 与最终 trigger-success JSON。

## 3. 硬门二：实现快照六项 active capability 的触发门

### 3.1 active / draft 分界

实现快照的 `docs/agents/pstack-lite/fixtures.json` 共有 7 个 `active`，其中
`probe-fixture` 只是分析器自证探针，**不计能力项**。正式 active capability 恰为 6：

1. `pstack-how`
2. `pstack-arena`
3. `pstack-create-verification`
4. `pstack-technical-writing`
5. `architect-merge-design-an-interface`
6. `architect-merge-codebase-design`

三个 `draft` 恰为：

- `pstack-blast-radius`
- `code-review-adversarial`
- `unslop`

三个 draft 各自仍保留 3 正 + 3 反语料，供未来重新启用时重测；它们**不进入本节的 6 项、
36 case 统计**。

### 3.2 manifest、raw report 与 description 的机械闭环

总表：`.sandcastle/acceptance-1138/trigger-success/manifest.json`，5620 bytes，SHA-256
`bfc08720b5bed8b6c41dd4f63c184664fb498f644b6a4361d35cc93c1da778e2`。

机械断言同时完成以下检查，Exit `0`：

- manifest `current_head` 精确等于 `216a3dd5d3746602b1ff85522fba192863460c5c`；
- 实现快照的 `active` capability 集合与 manifest 六项集合相等；
- 六份 raw report 均 `ok=true`、`registered=true`、`skipped=false`；
- manifest 与 raw report 的 item、case 顺序、`[id, verdict, loaded]` 逐项相同；
- 18/18 正例为 `loaded=true` + PASS；18/18 反例为 `loaded=false` + PASS；
- 对每项执行 `git show <source_commit>:<SKILL.md>`，实现快照与 source 的 frontmatter
  `description` 值逐字相同；值的 SHA-256 与 manifest 相同；
- 实现快照的 fixture 对象与对应 source commit 的 fixture 对象逐对象相同；
- 六个 source commit 都是实现快照的祖先。

| capability | 成功 source commit | provider / model | 正 / 反 | description SHA-256 | raw report SHA-256 |
|---|---|---|---:|---|---|
| `pstack-how` | `f973c7ab565fd833e4a235faa43170d74f9d7f0f` | anthropic / claude-sonnet-5 | 3/3 | `99371105b7a741df3dd67b08ac0728728fa74d2669f5321a5ec52105771590e8` | `7ea459d8e2d5de87bf059de86ef18bdc6be56616d78b46318b0797304bfd4c30` |
| `pstack-arena` | `3278b8467ef5055f7d03fc2c4e7453ee80513097` | openai-codex / gpt-5.6-sol | 3/3 | `66c389f622a566a7999a22de2fc62a30f9e7fe0051444e0d3d40f3e86c8e0338` | `ff50144d2ca90e327dd7b40621c64651e838fd29d448149ed6096c219a53d3c1` |
| `pstack-create-verification` | `56ed47765f87b95484615fca3c63d87ed7d524a4` | anthropic / claude-sonnet-5 | 3/3 | `cc7c7e5ce5b47ed0110f7a604710a53e50f9b054396aea7808d04a4ebcfa923e` | `e45f6c7e026e01d2c41ba61c3780cab8b7b5af99aeb63b83aadb339780bb1401` |
| `pstack-technical-writing` | `242aef56e5ab4b9616bc6514e835ed747270c9ee` | anthropic / claude-sonnet-5 | 3/3 | `b1225647503e3f872f6843ce6fccde9fed70b81f615f29098f042280f0584f71` | `4f40eb6a183e39398714c8334bf991cb82d94a128e50ac17a3c8dce05dbd5b92` |
| `architect-merge-design-an-interface` | `ff8adf1108dd4e4145c5cdff9149bed40816f639` | anthropic / claude-sonnet-5 | 3/3 | `7afdd5f83ca79c28c26b4f0743ec541a1e10040feb588c7d1840d2a73fd69a31` | `6de247743f101d1103c8bbc07380c596f67e03034d53bebddc48e5673d0c4569` |
| `architect-merge-codebase-design` | `ff8adf1108dd4e4145c5cdff9149bed40816f639` | anthropic / claude-sonnet-5 | 3/3 | `2cec03776b894387ec1056e40a47ed7bc56de7d03851ed3a3038bd3e05f05087` | `8fca7f6a7d70bfc08807b899fbda6000b999d086213bc3c9826482c17b22ab01` |

六份 raw report 位于：

```text
.sandcastle/acceptance-1138/trigger-success/pstack-how.json
.sandcastle/acceptance-1138/trigger-success/pstack-arena.json
.sandcastle/acceptance-1138/trigger-success/pstack-create-verification.json
.sandcastle/acceptance-1138/trigger-success/pstack-technical-writing.json
.sandcastle/acceptance-1138/trigger-success/architect-merge-design-an-interface.json
.sandcastle/acceptance-1138/trigger-success/architect-merge-codebase-design.json
```

六份 raw report 的 `.fixtures` 字段仍记录生成时的绝对路径
`/Users/silencehan/Projects/NewChanlun-pstack-accept/docs/agents/pstack-lite/fixtures.json`，不是本验收
worktree。该字段只说明历史 runner 当时读取的文件位置，不能单独充当实现快照 provenance；
本轮用 manifest 的 source commit、snapshot/source description 逐字比较、fixture 对象逐对象比较和
祖先检查建立实现快照桥。

18 个正例中 7 个是在 timeout 前已经取得 `loaded=true` 工具硬证据，分别是
`how-pos-1`、`how-pos-2`、`arch-dai-pos-1/2/3`、`arch-cbd-pos-1/3`。按 runner 的已自测
合同，这足以证明“触发并实际读 Skill”，**不证明对应回答完整或质量合格**；行为质量由下一节
独立场景证明。

### 3.3 三项 draft 不是“已通过”

#### blast-radius

可判定的真实 3+3 尝试没有一轮 6/6：Anthropic 4/6；ZAI 5/6；三个 OpenAI 收紧轮次各
5/6；另有一轮 6 ERROR 的基础设施失败，不计行为结果。失败在不同轮次落到不同正/反例，超过
#1129 的单项降级阈值。

因此提交 `8042f292d651d5f1c0e33c366352614dc9f36949` 实装 user-only，合并提交
`94cf9024b4521f240f61aec92bee2842d7497c2a` 落实现谱系；实现快照的
`check-blast-radius-description.mjs` 通过，证明的是**降级合同正确**，不是自动触发 PASS。

#### code-review adversarial

真实触发历程包括三轮 6 ERROR、一轮 4 PASS + 2 ERROR、一轮 5 PASS + 1 FAIL，以及最终
OpenAI 4/6（两个显式语义正例漏载）。行为场景发现 8 个 selector 但只有 1 个 owner 桶，
正确拒绝 adversarial admission。

因此提交 `4f47e759c840b1d6d9bdd7b483a875ef9ad33763` 实装 explicit-only，实现快照
`216a3dd5d3746602b1ff85522fba192863460c5c` 为合并降级提交。它是 `code-review` 的显式子模式，
不是另一个 Skill，也不是自动升级能力。

#### unslop

最终自动触发报告的三个正例全部 `loaded=false`，三个反例正确跳过；故它不是项目 active
trigger item。它保留为用户级全局默认行为，行为证据见 §4.8。

## 4. 硬门三：八个行为场景

本节只引用用户指定的最终证据，并把更早失败留在同一证据树中。八个场景目录下现存
`report*.md` 共 38 份：how 6、arena 8、blast 2、create-verification 7、
technical-writing 1、architect 6、code-review 7、unslop 1。本文没有删除、覆盖或把旧 FAIL
重标为 PASS。

### 4.1 pstack-how

- 证据：`.sandcastle/acceptance-1138/how/report-behavior-openai2.md`
- 固定点：`097a91708ecbd43d501abb6c860ce0ad8bbf9f45`
- 文件：24,870 bytes / 342 lines / SHA-256
  `5bfd0d989a166e57793099053fbaf61ebb32d1f6b4491e687be4d9e792037ec6`
- 判定：**行为合同 PASS；外层 wrapper exit 1。**

两个独立 explorer 各自完成 admission，并各自写出六节完整 fallback（13,038 / 13,580
bytes）；根代理完整读取后再复核源码。三份 JSONL 共 64 条 assistant message、42 个 tool call，
provider/model 路由偏差为 0。wrapper 非零是因为宿主把 `--autonomous-gate` 自然语言误作 shell
命令，反复得到 `/bin/sh: The: command not found`；这不能抹掉已完成的 child/fan-in，也不能
写成整进程 exit-0。

剩余代码事实：`ThetaPiStream::consume_managed` 没有 `rust/src` 生产调用点，也未由 PyO3
暴露。更早的 `report.md`、rerun、ZAI 与首轮 OpenAI 失败报告全部保留。

### 4.2 pstack-arena

候选证据：

- `.sandcastle/acceptance-1138/arena/report-behavior-openai.md`
- `e65b439ad60bf53ac09143dce0b06cce947daf03`
- 9,889 bytes / 122 lines / SHA-256
  `df76a7cfb6011132934ac6e5298cf6acbf948f98a3c6819f0baa5b4f15ec58f0`
- 该报告自身判定仍为 **FAIL**：三个互盲 child 文件存在，但旧验收错误要求“文件且消息”；
  `agent_message.send` 在宿主不可用，消息 0/3，judge / Pick / Graft / Verify 未执行。

最终 fan-in：

- `.sandcastle/acceptance-1138/arena/report-fanin-openai.md`
- `097a91708ecbd43d501abb6c860ce0ad8bbf9f45`
- 14,008 bytes / 174 lines / SHA-256
  `9c3e6b85dc7f0b884d91fb6668c66d88d2f1de960649a5862f18ba05c399f708`
- 判定：**PASS**。

fan-in 没有重跑三个候选；它按当前“完整消息**或** child-owned 文件”合同先核验三份原候选，
再 admission 恰好一个共同 judge，完成 Pick / Graft / Verify。最终为 5/5 rubric、7/7 内容
合同。Candidate 1/3 超过旧 1200 code-point 上限的格式缺陷仍保留；本证据只证明文本 wireframe
静态合同，不冒充 UI 运行。

### 4.3 pstack-blast-radius（只取显式行为）

- 证据：`.sandcastle/acceptance-1138/blast/report.md`
- 固定点：`a4e2a02da97ad9cbc31b2963408bd2c5b293fa95`
- 8,928 bytes / 90 lines / SHA-256
  `3ad927b46d53fdf1f2925fa70bb24335f06c39ea5c64dea12153360dfb58d8dc`
- 报告总判定：**FAIL**；原 3+3 只过 4/6。

本验收只采信其显式行为部分：真实 Skill 读取；对 #1130 runner diff 的调用链与 diff 外消费者
核对；Prime 内与宿主独立 `run-trigger-fixtures --self-test` 均 7/7、exit 0；实际 toolCall 中
Wayfinder / RLM / 子代理为 0。触发 FAIL 不被行为分析抵消。过程还曾瞬时创建 ignored
`.serena` 后删除，故不能声称整个过程绝对没有允许目录外副作用。

### 4.4 pstack-create-verification

- 证据：`.sandcastle/acceptance-1138/create-verification/report-behavior-openai.md`
- 固定点：`097a91708ecbd43d501abb6c860ce0ad8bbf9f45`
- 11,482 bytes / 164 lines / SHA-256
  `2dd2364bf34e6ccc498ab78b8a6d35cc2f53c49a70afd92cf1348cfdba70672d`
- 判定：**PASS**。

一次 Prime attempt 在授权外部 scratch `/tmp/newchanlun-1138-create-verification-openai` 内实际读取
`pstack-create-verification` 与 Prime `skill-creator`，生成 deterministic demo CLI 和
`verify-demo`。Launch / Doctor / Drive / Cleanup 四阶段 exit 均为 0、stderr 均为空；Cleanup
删除 runtime、保留 proof；19/19 checksum 通过。

同一 session 更早的生成树范围检查和首次 Python parse 曾失败；报告保留失败、确认没有残留
runtime/proof 后才修正并执行唯一正式行为 run。它没有用最终成功覆盖失败迭代。

### 4.5 pstack-technical-writing

- 证据：`.sandcastle/acceptance-1138/technical-writing/report.md`
- 固定点：`a4e2a02da97ad9cbc31b2963408bd2c5b293fa95`
- 6,987 bytes / 140 lines / SHA-256
  `ff2c0103f8cc2612b7b8324f9bed6344c237273df007774f4af405c11e9f1ca5`
- 判定：**PASS**。

最终 3+3 触发通过；独立 E2E exit 0，并在同一 toolCall 中先读项目
`pstack-technical-writing`，再读全局 `unslop`。8/8 受保护片段逐字节相同；二级标题 0→4、
指定 AI 腔 11→0、最长句 123→70。首轮 worker/socket 的六例 ERROR 保留在
`trigger-attempt1-worker-socket-error.json`，不计 Skill 结果。

### 4.6 architect 合并能力

- 证据：`.sandcastle/acceptance-1138/architect/report-behavior-openai3.md`
- 固定点：`5075d3e0e82f2aefadf6bf8911a13c8e0cc32472`
- 12,987 bytes / 148 lines / SHA-256
  `a8b743a4830635da26a73ebc889eecbf86db6ed066cc1495efc73c108844f2e5`
- 判定：**PASS**。

正向和 trivial control 均 exit 0。正向会话在任何 fan-out 前完整读取
`design-an-interface` 与 `codebase-design`，随后唯一一次 fan-out 恰好三个直接 child；三个
结果分别 760 / 807 / 756 words，六标题机械门 3/3。`agent_message` 不可用，三个 child 按
合同写各自预分配 fallback；父级在 3/3 完成后才复核四类 red flag 并综合。control 无 Skill、
0 toolCall，只输出 `CONTROL_OK: 2+2=4`。更早 openai2 / ZAI / daemon 失败报告保留。

### 4.7 code-review adversarial 降级合同

- 证据：`.sandcastle/acceptance-1138/code-review/report-degraded-openai.md`
- 固定点：`e65b439ad60bf53ac09143dce0b06cce947daf03`
- 13,453 bytes / 211 lines / SHA-256
  `71e7af7e5fbee59c095a62a486ed5c3d3a2fbbd43abdabe136f2e4b664a14512`
- 判定：**降级合同 PASS；adversarial 本身没有通过。**

根代理实际执行 `rlm.find_models(limit=8)`：8 个唯一 selector 全属 Anthropic，一个 owner
桶。3-selector / 2-owner gate 正确拒绝，输出 `Adversarial status: unavailable`；adversarial
admission 为 0。随后 Standards 与 Spec 两个独立 child 均完成并由根读取最终 JSONL；确定性
gate 11/11。

`report-crossmodel-openai.md` 仍是 FAIL：可用结果来自三个 OpenAI 模型，其他厂商失败；它
不是跨厂商成功。该旧尝试还没有满足完整 child 回流合同，且 reviewer A 执行了一次只读
`gh issue view 1130`，违反该轮 no-touch-GitHub 约束；虽然没有修改 GitHub，也不能从失败史中
删去。本文不使用同厂三模型冒充 adversarial PASS。

### 4.8 全局 unslop 混合行为

- 证据：`.sandcastle/acceptance-1138/unslop/report.md`
- 固定点：`a4e2a02da97ad9cbc31b2963408bd2c5b293fa95`
- 4,543 bytes / 56 lines / SHA-256
  `74cee7fd45d7eeb0afa3374c2ec8373e55445406d6802aebe7f938ded28c1946`
- 报告总判定：**FAIL**；混合行为部分 PASS。

混合会话真实读取全局 Skill；共 3 个 tool call，唯一工具写目标是场景内 `output.md`；7/7
保护段逐字节相同；自然语言由 63 词降到 30 词，预设套话命中为 0。自动触发则是三个正例
全部漏载、三个反例正确跳过。首轮 daemon 失败还向 `~/.prime/agent/logs/` 写入诊断，因此
不能声称整个文件系统零场景外写入。它只作为全局行为证据，不进入项目 active 触发统计。

## 5. 硬门四：集成

### 5.1 实现快照结构与历史路径

以 pstack-lite 开始前的固定点
`68c9540b9b79aa987085a8fe9fd59923b6dc1f7b` 为基线，枚举 first-parent 上 subject 为
`merge(pstack-lite):` 的提交，共 18 个；逐个相对第一父提交取路径并集，共 53 个。允许范围是：

- `.agents/skills/pstack-*`
- `.claude/skills/pstack-*`
- `.agents/skills/{code-review,codebase-design,design-an-interface}/`
- `docs/agents/pstack-lite/`
- `docs/agents/skill-local-customizations.md`
- `scripts/pstack-lite/`

白名单外 tracked path = 0。

Wayfinder 保护面：

```text
.agents/skills/wayfinder/
.agents/skills/to-spec/
.agents/skills/to-tickets/
.agents/skills/implement/
docs/agents/wayfinder-workflow.md
docs/agents/wayfinder-session-brief.md
scripts/wayfinder_engine.mts
scripts/wayfinder_engine.test.mts
scripts/wayfinder_session_brief.sh
```

`git diff --quiet 68c9540b... "$implementation" -- <上述保护面>` exit `0`。53 个 changed path 中
Wayfinder/router/sticky 路径命中 0；pstack scope 的新增行中 router/sticky 命中 0。
`Wayfinder` 文本命中 7 行，逐行均为“不包裹/不接管/唯一上层/复用现役”的边界或既有定制索引，
没有执行入口。文本扫描的终点明确是实现快照，不是文档提交，因此不会把本文自己的
Wayfinder/router 边界说明计入实现结果。

### 5.2 四个集成负断言

| 断言 | 实现快照结果 | 行为证据 |
|---|---:|---|
| Wayfinder takeover | 0 | 各最终场景实际 toolCall 中为 0；保护面 diff 为空 |
| duplicate fan-out | 0 | how 计划内 2 explorer；arena 旧 3 candidate + 后续唯一 judge；architect 唯一 3 child；code-review 降级只有 Standards/Spec 两 child |
| 未授权 tracked / staged write | 0 | 最终可采信运行前后 diff/index 均空；本轮写文档前也为空 |
| 项目级 `unslop` 副本 | 0 | loader `projectCopy=null`；`.agents/skills/unslop` 不存在 |

另有：

- `.agents/skills/pstack-architect` = 0；
- `.agents/skills/pstack-interrogate` = 0；
- 用户级 `~/.agents/skills/pstack-*` 投影 = 0；
- canonical `.agents/skills/wayfinder/SKILL.md` 恰一份；
- 实现快照的 `code-review/SKILL.md` 仍有 Standards/Spec 默认双轴，frontmatter 无
  `disable-model-invocation`；真实降级场景也证明两轴可运行；
- `design-an-interface` 与 `codebase-design` 两条现役设计流都可加载，触发门各 3+3，行为场景
  证明分工可运行；
- `check-pstack-how-contract` 与 `check-design-interface-contract` 明确锁定 admission handle 不是
  结果，根必须读取消息、child-owned 文件或允许的最终 JSONL。

### 5.3 写入边界的准确表述

“零未授权 tracked/staged 写入”成立；“整个文件系统从未有场景外写入”不成立。必须保留：

- blast 过程曾创建 ignored `.serena` 后删除；
- unslop 首轮向宿主 Prime 日志写诊断；
- arena/how/architect 的 `agent_message` host 通道不可用，最终走合同允许的文件或 final JSONL；
- create-verification 的授权 proof 位于 `/tmp/newchanlun-1138-create-verification-openai`；
- `.sandcastle/acceptance-1138/` 是未跟踪的本地证据树。

## 6. 实现快照命令账

以下命令都是只读或在 `mktemp` 临时目录内自清理；没有真实模型、没有全仓测试。

```bash
git -C "$acceptance_repo" branch --show-current
git -C "$implementation_root" rev-parse HEAD
prime-agent --version

gh issue view 1129 --repo xy7365527-lang/NewChanlun \
  --json number,title,body,comments,state,url,labels,createdAt,updatedAt,closedAt
gh issue view 1138 --repo xy7365527-lang/NewChanlun \
  --json number,title,body,comments,state,url,labels,createdAt,updatedAt,closedAt
gh issue view 1139 --repo xy7365527-lang/NewChanlun \
  --json number,title,body,comments,state,url,labels,createdAt,updatedAt,closedAt

snapshot_selftest_home="$(mktemp -d /tmp/newchanlun-1138-selftest.XXXXXX)"
(
  set -e
  cd "$implementation_root"
  env HOME="$snapshot_selftest_home" \
    PRIME_AGENT_DIST=/Users/silencehan/.local/lib/node_modules/prime-agent/dist \
    node scripts/pstack-lite/check-skills.mjs --self-test --json
  node scripts/pstack-lite/run-trigger-fixtures.mjs --self-test
  node scripts/pstack-lite/check-pstack-how-contract.mjs
  node scripts/pstack-lite/check-design-interface-contract.mjs
  node scripts/pstack-lite/check-adversarial-gate.mjs
  node scripts/pstack-lite/check-blast-radius-description.mjs
)
selftest_rc=$?
rm -rf "$snapshot_selftest_home"
test "$selftest_rc" -eq 0

git -C "$implementation_root" diff --quiet
git -C "$implementation_root" diff --cached --quiet
git -C "$implementation_root" status --porcelain=v1 --untracked-files=no
git -C "$acceptance_repo" status --short
```

各命令 exit：

| 组 | Exit / 计数 |
|---|---|
| branch / 实现快照 / Prime version | 0 / 0 / 0 |
| 三张 `gh issue view` | 0 / 0 / 0 |
| isolated loader / target list / canonical unslop | 0（74=61+13）/ 0（8/8）/ 0（130=56+74） |
| check-skills self-test | 0，10/10 |
| trigger-runner self-test | 0，7/7 |
| how / design / adversarial / blast contract | 0（18/18）/ 0（34/34）/ 0（11/11）/ 0（0 failure） |
| trigger manifest + raw + description + fixture audit | 0，6 reports / 36 cases / 18 loaded / 18 skipped / 6 description unchanged / 6 fixture unchanged |
| snapshot/source/behavior ancestor audit | 0，9/9 |
| pstack JSON + final trigger JSON | 0，11/11 |
| symlink/source/SHA/license | 0，5/5/5/6 |
| integration | 0，18 merge commits / 53 paths / 0 unauthorized path / 0 router-sticky line / 7 Wayfinder boundary lines |
| 写文档前 worktree / index diff | 0 / 0；tracked status 为空 |

写文档前 `git status --short` 只有三个既有未跟踪根：

```text
?? .sandcastle/acceptance-1138/
?? dbg-upl7fr/
?? node-compile-cache/
```

它们不属于本提交。

### 6.1 六项 trigger / description 断言原文

为使“description 未变”可以机械重放，以下保留本轮实际运行的内联命令；Exit `0`，输出即
§3.2 的 6 reports / 36 cases / 18 loaded / 18 skipped / 6 description unchanged：

```bash
(
set -e
cd "$implementation_root"
node --input-type=module <<'NODE'
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

const implementation = process.env.IMPLEMENTATION;
const acceptanceRepo = process.env.ACCEPTANCE_REPO;
const evidenceRoot = process.env.EVIDENCE_ROOT;
assert.ok(implementation && acceptanceRepo && evidenceRoot);
const manifestPath = path.join(evidenceRoot, "trigger-success/manifest.json");
const fixturesPath = "docs/agents/pstack-lite/fixtures.json";
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const fixtures = JSON.parse(fs.readFileSync(fixturesPath, "utf8"));
const snapshotHead = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
const pathByItem = {
  "pstack-how": ".agents/skills/pstack-how/SKILL.md",
  "pstack-arena": ".agents/skills/pstack-arena/SKILL.md",
  "pstack-create-verification": ".agents/skills/pstack-create-verification/SKILL.md",
  "pstack-technical-writing": ".agents/skills/pstack-technical-writing/SKILL.md",
  "architect-merge-design-an-interface": ".agents/skills/design-an-interface/SKILL.md",
  "architect-merge-codebase-design": ".agents/skills/codebase-design/SKILL.md",
};
function description(text) {
  const lines = text.split("\n");
  assert.equal(lines[0], "---");
  const end = lines.indexOf("---", 1);
  assert.ok(end > 1);
  const line = lines.slice(1, end).find((x) => x.startsWith("description:"));
  assert.ok(line);
  const raw = line.slice("description:".length).trim();
  return raw.startsWith('"') ? JSON.parse(raw) : raw;
}
const active = fixtures.items.filter((x) => x.status === "active" && x.id !== "probe-fixture");
assert.equal(snapshotHead, implementation);
assert.equal(manifest.current_head, implementation);
assert.equal(active.length, 6);
assert.deepEqual(active.map((x) => x.id).sort(), manifest.items.map((x) => x.item).sort());
let positive = 0;
let negative = 0;
const rows = [];
for (const item of manifest.items) {
  const skillPath = pathByItem[item.item];
  assert.ok(skillPath);
  const snapshotText = fs.readFileSync(skillPath, "utf8");
  const sourceText = execFileSync("git", ["show", item.source_commit + ":" + skillPath], {
    encoding: "utf8",
  });
  const snapshotDescription = description(snapshotText);
  const sourceDescription = description(sourceText);
  assert.equal(snapshotDescription, sourceDescription);
  const digest = crypto.createHash("sha256").update(snapshotDescription).digest("hex");
  assert.equal(digest, item.current_description_sha256);
  assert.equal(item.frontmatter_description_unchanged, true);
  const reportPath = path.isAbsolute(item.report)
    ? item.report
    : path.join(acceptanceRepo, item.report);
  const raw = JSON.parse(fs.readFileSync(reportPath, "utf8"));
  assert.equal(raw.ok, true);
  assert.equal(raw.items.length, 1);
  assert.equal(raw.items[0].id, item.item);
  assert.equal(raw.items[0].status, "active");
  assert.equal(raw.items[0].registered, true);
  assert.equal(raw.items[0].skipped, false);
  assert.equal(raw.items[0].cases.length, 6);
  const reportCases = raw.items[0].cases.map((c) => [c.id, c.verdict, c.loaded]);
  assert.deepEqual(reportCases, item.cases);
  const pos = raw.items[0].cases.filter((c) => c.expect === "loaded");
  const neg = raw.items[0].cases.filter((c) => c.expect === "skipped");
  assert.equal(pos.length, 3);
  assert.equal(neg.length, 3);
  assert.ok(pos.every((c) => c.verdict === "PASS" && c.loaded === true));
  assert.ok(neg.every((c) => c.verdict === "PASS" && c.loaded === false));
  positive += pos.length;
  negative += neg.length;
  rows.push({
    item: item.item,
    source_commit: item.source_commit,
    provider: item.provider,
    model: item.model,
    cases: "6/6",
    description_sha256: digest,
  });
}
console.log(JSON.stringify({
  ok: true,
  implementationSnapshot: implementation,
  activeCapabilities: active.length,
  manifestItems: manifest.items.length,
  reports: rows.length,
  positiveLoaded: positive,
  negativeSkipped: negative,
  descriptionsUnchanged: rows.length,
  rows,
}, null, 2));
NODE
)
```

fixture 对象不变另用同样的 `manifest.items` 循环执行：

```bash
(
set -e
cd "$implementation_root"
node --input-type=module <<'NODE'
import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
const implementation = process.env.IMPLEMENTATION;
const evidenceRoot = process.env.EVIDENCE_ROOT;
assert.ok(implementation && evidenceRoot);
assert.equal(execFileSync("git", ["rev-parse", "HEAD"], {encoding: "utf8"}).trim(), implementation);
const manifest = JSON.parse(fs.readFileSync(
  path.join(evidenceRoot, "trigger-success/manifest.json"), "utf8"));
assert.equal(manifest.current_head, implementation);
const snapshot = JSON.parse(fs.readFileSync("docs/agents/pstack-lite/fixtures.json", "utf8"));
const rows = [];
for (const entry of manifest.items) {
  const source = JSON.parse(execFileSync("git", [
    "show", entry.source_commit + ":docs/agents/pstack-lite/fixtures.json",
  ], {encoding: "utf8"}));
  const a = snapshot.items.find((x) => x.id === entry.item);
  const b = source.items.find((x) => x.id === entry.item);
  assert.deepEqual(a, b);
  rows.push({item: entry.item, source_commit: entry.source_commit, fixtureObjectUnchanged: true});
}
console.log(JSON.stringify({ok: true, items: rows.length, rows}, null, 2));
NODE
)
```

Exit `0`，6/6 fixture object unchanged。

### 6.2 source / behavior 祖先关系断言原文

以下 5 个唯一 `source_commit` 与 4 个行为固定点逐一对实现快照执行
`git merge-base --is-ancestor`；Exit `0`，9/9：

```bash
node --input-type=module <<'NODE'
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

const implementation = process.env.IMPLEMENTATION;
assert.equal(implementation, "216a3dd5d3746602b1ff85522fba192863460c5c");
const revisions = [
  ["source", "f973c7ab565fd833e4a235faa43170d74f9d7f0f"],
  ["source", "3278b8467ef5055f7d03fc2c4e7453ee80513097"],
  ["source", "56ed47765f87b95484615fca3c63d87ed7d524a4"],
  ["source", "242aef56e5ab4b9616bc6514e835ed747270c9ee"],
  ["source", "ff8adf1108dd4e4145c5cdff9149bed40816f639"],
  ["behavior", "097a91708ecbd43d501abb6c860ce0ad8bbf9f45"],
  ["behavior", "e65b439ad60bf53ac09143dce0b06cce947daf03"],
  ["behavior", "a4e2a02da97ad9cbc31b2963408bd2c5b293fa95"],
  ["behavior", "5075d3e0e82f2aefadf6bf8911a13c8e0cc32472"],
];
const rows = revisions.map(([kind, rev]) => {
  const result = spawnSync("git", ["merge-base", "--is-ancestor", rev, implementation]);
  assert.equal(result.status, 0, `${rev} is not an ancestor of ${implementation}`);
  return {kind, rev, ancestor: true};
});
console.log(JSON.stringify({ok: true, implementationSnapshot: implementation, checks: rows.length, rows}, null, 2));
NODE
```

### 6.3 JSON 语法断言原文

```bash
(
set -e
cd "$implementation_root"
node --input-type=module <<'NODE'
import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
const implementation = process.env.IMPLEMENTATION;
const evidenceRoot = process.env.EVIDENCE_ROOT;
assert.ok(implementation && evidenceRoot);
assert.equal(execFileSync("git", ["rev-parse", "HEAD"], {encoding: "utf8"}).trim(), implementation);
const trackedRoots = [
  "docs/agents/pstack-lite",
  ".agents/skills/pstack-how",
  ".agents/skills/pstack-arena",
  ".agents/skills/pstack-blast-radius",
  ".agents/skills/pstack-create-verification",
  ".agents/skills/pstack-technical-writing",
  ".agents/skills/code-review",
  ".agents/skills/design-an-interface",
  ".agents/skills/codebase-design",
];
const tracked = execFileSync("git", ["ls-files", ...trackedRoots], {encoding: "utf8"})
  .trim().split("\n").filter((x) => x.endsWith(".json"));
const triggerRoot = path.join(evidenceRoot, "trigger-success");
const trigger = execFileSync("rg", [
  "--files", triggerRoot, "-g", "*.json",
], {encoding: "utf8"}).trim().split("\n").filter(Boolean);
const files = [...new Set([...tracked, ...trigger])].sort();
assert.ok(files.length > 0);
for (const file of files) JSON.parse(fs.readFileSync(file, "utf8"));
console.log(JSON.stringify({
  ok: true,
  trackedJson: tracked.length,
  triggerEvidenceJson: trigger.length,
  total: files.length,
}, null, 2));
NODE
)
```

Exit `0`，tracked 4 + final trigger 7 = 11/11。

### 6.4 symlink / source / SHA / license 断言原文

```bash
(
set -e
cd "$implementation_root"
node --input-type=module <<'NODE'
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

const implementation = process.env.IMPLEMENTATION;
assert.equal(execFileSync("git", ["rev-parse", "HEAD"], {encoding: "utf8"}).trim(), implementation);
const fixed = "fd6dd6f7276956a532bb78a748a8d2818b6eb5f4";
const licenseDigest = "bc957ca6bee02792566a1a028d105e02e247c6e77cf057061674273da77b200e";
const skills = {
  "pstack-how": "pstack/skills/how/SKILL.md",
  "pstack-arena": "pstack/skills/arena/SKILL.md",
  "pstack-blast-radius": "pstack/skills/blast-radius/SKILL.md",
  "pstack-create-verification": "pstack/skills/create-verification-skill/SKILL.md",
  "pstack-technical-writing": "pstack/skills/technical-writing/SKILL.md",
};
const baseLicense = fs.readFileSync("docs/agents/pstack-lite/LICENSE.MIT");
assert.equal(crypto.createHash("sha256").update(baseLicense).digest("hex"), licenseDigest);
const rows = [];
for (const [name, upstreamPath] of Object.entries(skills)) {
  const skillDir = path.join(".agents/skills", name);
  const skillPath = path.join(skillDir, "SKILL.md");
  const projection = path.join(".claude/skills", name);
  const body = fs.readFileSync(skillPath, "utf8");
  assert.equal(fs.lstatSync(projection).isSymbolicLink(), true);
  assert.equal(fs.readlinkSync(projection), "../../.agents/skills/" + name);
  assert.equal(fs.realpathSync(projection), fs.realpathSync(skillDir));
  assert.ok(body.split("\n").includes("name: " + name));
  assert.ok(body.split("\n").includes("license: MIT"));
  assert.ok(body.split("\n").includes("  upstream-repo: cursor/plugins"));
  assert.ok(body.split("\n").includes("  upstream-sha: " + fixed));
  assert.ok(body.split("\n").includes("  upstream-path: " + upstreamPath));
  assert.ok(body.split("\n").includes("  license-file: LICENSE.MIT"));
  assert.ok(body.includes("## 来源"));
  assert.ok(body.includes("## 本地 Prime 改写"));
  const copied = fs.readFileSync(path.join(skillDir, "LICENSE.MIT"));
  assert.equal(Buffer.compare(copied, baseLicense), 0);
  const digest = crypto.createHash("sha256").update(copied).digest("hex");
  assert.equal(digest, licenseDigest);
  rows.push({name, projection: fs.readlinkSync(projection), upstreamPath, licenseSha256: digest});
}
const architect = [
  ".agents/skills/codebase-design/ARCHITECT-MERGE.md",
  ".agents/skills/design-an-interface/ARCHITECT-MERGE.md",
];
for (const p of architect) {
  const body = fs.readFileSync(p, "utf8");
  assert.ok(body.includes("cursor/plugins"));
  assert.ok(body.includes(fixed));
  assert.ok(body.includes("pstack/skills/architect/SKILL.md"));
  assert.ok(body.includes("MIT"));
}
const adversarial = [
  ".agents/skills/code-review/ADVERSARIAL-MODE.md",
  ".agents/skills/code-review/ADVERSARIAL-REVIEWER-PROMPT.md",
  ".agents/skills/code-review/ADVERSARIAL-RUBRIC.md",
  ".agents/skills/code-review/ADVERSARIAL-CODE-QUALITY.md",
  ".agents/skills/code-review/ADVERSARIAL-LEAD-JUDGMENT.md",
];
for (const p of adversarial) assert.ok(fs.statSync(p).size > 0);
const mode = fs.readFileSync(adversarial[0], "utf8");
assert.ok(mode.includes("cursor/plugins"));
assert.ok(mode.includes(fixed));
assert.ok(mode.includes("pstack/skills/interrogate/SKILL.md"));
assert.ok(mode.includes("MIT"));
console.log(JSON.stringify({
  ok: true,
  canonicalSkills: rows.length,
  projections: rows.length,
  fixedShaAssertions: rows.length,
  licenseFiles: rows.length + 1,
  licenseBytes: baseLicense.length,
  licenseSha256: licenseDigest,
  architectProvenanceFiles: architect.length,
  adversarialSourceFiles: adversarial.length,
  rows,
}, null, 2));
NODE
)
```

Exit `0`，结果见 §2.4。

### 6.5 集成断言原文

```bash
(
set -e
cd "$implementation_root"
node --input-type=module <<'NODE'
import fs from "node:fs";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";

const base = "68c9540b9b79aa987085a8fe9fd59923b6dc1f7b";
const implementation = process.env.IMPLEMENTATION;
const snapshotHead = execFileSync("git", ["rev-parse", "HEAD"], {encoding: "utf8"}).trim();
assert.equal(snapshotHead, implementation);
const verificationRecord = "docs/agents/pstack-lite/VERIFICATION-1138.md";
assert.equal(fs.existsSync(verificationRecord), false);
const log = execFileSync("git", [
  "log", "--first-parent", "--format=%H%x09%s", base + ".." + implementation,
], {encoding: "utf8"}).trim().split("\n").filter(Boolean);
const commits = log.filter((x) => x.includes("merge(pstack-lite):"))
  .map((x) => x.split("\t")[0]);
const changed = new Set();
for (const commit of commits) {
  const out = execFileSync("git", ["diff", "--name-only", commit + "^1", commit], {
    encoding: "utf8",
  });
  for (const p of out.trim().split("\n").filter(Boolean)) changed.add(p);
}
const allowed = (p) =>
  p.startsWith(".agents/skills/pstack-") ||
  p.startsWith(".agents/skills/code-review/") ||
  p.startsWith(".agents/skills/codebase-design/") ||
  p.startsWith(".agents/skills/design-an-interface/") ||
  p.startsWith(".claude/skills/pstack-") ||
  p.startsWith("docs/agents/pstack-lite/") ||
  p === "docs/agents/skill-local-customizations.md" ||
  p.startsWith("scripts/pstack-lite/");
const unauthorized = [...changed].filter((p) => !allowed(p));
assert.deepEqual(unauthorized, []);

const protectedPaths = [
  ".agents/skills/wayfinder",
  ".agents/skills/to-spec",
  ".agents/skills/to-tickets",
  ".agents/skills/implement",
  "docs/agents/wayfinder-workflow.md",
  "docs/agents/wayfinder-session-brief.md",
  "scripts/wayfinder_engine.mts",
  "scripts/wayfinder_engine.test.mts",
  "scripts/wayfinder_session_brief.sh",
];
const protectedDiff = spawnSync("git", [
  "diff", "--quiet", base, implementation, "--", ...protectedPaths,
]);
assert.equal(protectedDiff.status, 0);
const pathControlHits = [...changed].filter((p) => /wayfinder|router|sticky/i.test(p));
assert.equal(pathControlHits.length, 0);

const scopePaths = [
  ".agents/skills/pstack-*",
  ".agents/skills/code-review",
  ".agents/skills/codebase-design",
  ".agents/skills/design-an-interface",
  "docs/agents/pstack-lite",
  ":(exclude)docs/agents/pstack-lite/VERIFICATION-1138.md",
  "docs/agents/skill-local-customizations.md",
  "scripts/pstack-lite",
];
const patch = execFileSync("git", [
  "diff", "--unified=0", base, implementation, "--", ...scopePaths,
], {encoding: "utf8"});
const added = patch.split("\n").filter((x) => /^\+[^+]/.test(x));
const routerStickyAdded = added.filter((x) => /router|sticky/i.test(x));
const wayfinderAdded = added.filter((x) => /wayfinder/i.test(x));
assert.equal(routerStickyAdded.length, 0);
assert.equal(wayfinderAdded.length, 7);

for (const p of [
  ".agents/skills/unslop",
  ".agents/skills/pstack-architect",
  ".agents/skills/pstack-interrogate",
]) assert.equal(fs.existsSync(p), false);
const globalPstack = fs.readdirSync("/Users/silencehan/.agents/skills")
  .filter((x) => x.startsWith("pstack-"));
assert.equal(globalPstack.length, 0);
assert.equal(fs.existsSync(".agents/skills/wayfinder/SKILL.md"), true);

const codeReview = fs.readFileSync(".agents/skills/code-review/SKILL.md", "utf8");
assert.ok(codeReview.includes("**Standards**"));
assert.ok(codeReview.includes("**Spec**"));
assert.equal(codeReview.split("\n").slice(0, 8)
  .some((x) => x.startsWith("disable-model-invocation:")), false);
const designFlows = [
  ".agents/skills/design-an-interface/SKILL.md",
  ".agents/skills/codebase-design/SKILL.md",
].filter((p) => fs.existsSync(p));
assert.equal(designFlows.length, 2);

const fixtures = JSON.parse(fs.readFileSync("docs/agents/pstack-lite/fixtures.json", "utf8"));
const active = fixtures.items.filter((x) => x.status === "active");
const activeCapabilities = active.filter((x) => x.id !== "probe-fixture");
const draft = fixtures.items.filter((x) => x.status === "draft").map((x) => x.id).sort();
assert.equal(active.length, 7);
assert.equal(activeCapabilities.length, 6);
assert.deepEqual(draft, ["code-review-adversarial", "pstack-blast-radius", "unslop"]);
assert.match(fs.readFileSync(".agents/skills/pstack-blast-radius/SKILL.md", "utf8"),
  /^disable-model-invocation: true$/m);
assert.ok(fs.readFileSync(".agents/skills/code-review/ADVERSARIAL-MODE.md", "utf8")
  .includes("explicit-only"));
console.log(JSON.stringify({
  ok: true,
  base,
  implementationSnapshot: implementation,
  verificationRecordInSnapshot: false,
  pstackFirstParentMerges: commits.length,
  changedPaths: changed.size,
  unauthorizedTrackedPaths: unauthorized.length,
  protectedWayfinderDiffExit: protectedDiff.status,
  controlPathHits: pathControlHits.length,
  routerStickyAddedLines: routerStickyAdded.length,
  wayfinderBoundaryOrReuseLines: wayfinderAdded.length,
  projectUnslopCopies: 0,
  pstackArchitectCopies: 0,
  pstackInterrogateCopies: 0,
  globalPstackProjections: globalPstack.length,
  canonicalWayfinderSkills: 1,
  standardsSpecFlow: true,
  designFlows: designFlows.length,
  activeIncludingProbe: active.length,
  activeCapabilities: activeCapabilities.length,
  draftItems: draft,
}, null, 2));
NODE
)
```

Exit `0`，输出 18 个 pstack first-parent merge、53 个 changed path、0 个白名单外路径、
Wayfinder 保护面 exit 0、三类重复目录均 0、Standards/Spec 与两条 design flow 可用。

### 6.6 临时实现 worktree 清理

完成全部重放后移除 §1 创建的临时 worktree；这不会触碰验收证据树：

```bash
git -C "$acceptance_repo" worktree remove "$implementation_root"
rmdir "$snapshot_parent"
```

## 7. 剩余限制

1. **没有在实现快照重跑模型。** 用户明确禁止；快照桥由祖先关系、description/fixture
   不变与确定性合同组成。
2. **canonical root 不是实现快照。** 它证明全局 unslop，不证明 pstack 已部署到真实宿主
   canonical checkout。
3. **触发 raw session 不再存在。** runner 在逐 case 分析后清理临时会话；可复核层级止于
   manifest + 六份 raw report。7 个正例只在 timeout 前证明 loaded，不证明回答完成。
   六份 report 的 `.fixtures` 还是生成时 `NewChanlun-pstack-accept` worktree 的绝对路径；
   实现快照 provenance 依赖本轮的 source/description/fixture/ancestor 机械桥，而不是该绝对路径。
4. **本地证据跨机器可携性有限。** `.sandcastle/acceptance-1138/` 未入 Git；
   create-verification 的最终 proof 在 `/tmp`。本文用路径、bytes 与 SHA-256 固化可追溯性。
5. **触发说明有已知漂移。** `TRIGGER-FIXTURES.md:41` 与 `:49` 仍写“唯一/当前只有 probe
   active”；实现快照的 `fixtures.json` 实际是 7 active（含 probe）+ 3 draft。因用户要求本提交
   只含本验收记录，本轮不改该文件；状态以 JSON 与 manifest 为准。
6. **历史失败 artifact 保留。** `arena/e2e-openai-cli.json` 为 0 bytes；它不属于最终
   trigger-success JSON，也没有被删除。
7. **adversarial 没有跨厂商成功证据。** 当前只有正确降级证据；未来必须真实满足 3 selector /
   2 owner，再读取三个 reviewer 的真实产物，才能称 adversarial PASS。

## 8. #1139 试点范围

准入 #1139，但试点必须采用以下收窄口径：

### 正式自动路由统计

只把六个 active capability 纳入自动路由统计：

- `pstack-how`
- `pstack-arena`
- `pstack-create-verification`
- `pstack-technical-writing`
- `architect-merge-design-an-interface`
- `architect-merge-codebase-design`

`probe-fixture` 完全排除。

### 对照与降级项

- blast-radius 只做用户显式 `user-only` 对照；不自动加载不是漏触发，显式调用也不抬高自动
  路由分母。
- code-review 正式试点只计现役 Standards/Spec；adversarial 可做显式对照，但每次仍先过
  3-selector / 2-owner 门。不足时 `unavailable`、启动/回流失败时 `degraded` 是正确结果，不是
  adversarial 成功。
- unslop 只观察全局混合内容行为和受保护内容，不作为项目 active trigger item。

### 仍然有效的 #1139 门槛

- 10 个任务覆盖至少三类 Wayfinder/实施场景，不用同一模板凑数；
- 路由正确至少 9/10；
- 未授权写入 0；
- Wayfinder 接管 0；
- 输出可直接采用或只需局部修改至少 8/10；
- 单项若发生高严重度越权、接管、受保护内容破坏或 shadow，立即回退该项；
- 单项若累计两次误触发或漏触发，降为 user-only，调整 description 后只重跑该项验证。

因此，#1138 的最终裁决不是“原始七项自动路由全部成功”，而是：**六项 active capability
通过四道硬门；blast、adversarial、unslop 按已声明边界降级或转为全局行为后，整体可进入
#1139 受控试点。**
