# pstack-technical-writing 靶向验证记录（票 #1135）

> 本文件记录本票验收命令的**实际执行结果**，以及一份端到端手工验证样本（受保护内容
> 逐字比对 + 结构/清晰度改善核对）。实装阶段不跑 `run-trigger-fixtures.mjs` 的真实模型
> 调用（时间闸规定），该部分留给独立评审工蜂与宿主验收。

## 环境

- 分支：`sandcastle/issue-1135`
- 本仓已安装：`.agents/skills/pstack-technical-writing/`（`SKILL.md` + `LICENSE.MIT`）

## ① 发现 + 诊断分类检查

```bash
node scripts/pstack-lite/check-skills.mjs
```

结果（exit 0）：

```text
cwd: /home/agent/workspace
agentDir: /home/agent/.prime/agent
skills: 73 (project 57, user 16)
rawDiagnostics: 0
expectedProjectionCollisions: 0
unexpectedDiagnostics: 0
unslop: absent
```

`unexpectedDiagnostics: 0` —— 新增的 `pstack-technical-writing` 未产生任何 shadow /
warning，`unslop: absent` 是本沙盒既有边界（全局 `unslop` 装在主机 `~/.agents/skills/`，
不在本沙盒镜像内，见 #1130 VERIFICATION 记录同款说明）。

## ② shadow / 投影分类自测

```bash
node scripts/pstack-lite/check-skills.mjs --self-test
```

结果（exit 0）：11 项全 PASS（合法 Skill 零诊断加载、命名/描述违规产生 warning、同名
shadow 判 collision 且项目级胜出、`unslop`/`pstack-*` 的碰撞一律不享受投影例外）。

## ③ 触发夹具分析器自测

```bash
node scripts/pstack-lite/run-trigger-fixtures.mjs --self-test
```

结果（`"ok": true`）：三项全 PASS —— 工具调用读 `SKILL.md` 判 loaded、模型自报文本不算
证据判 not loaded、`toolResult` 含 Skill 路径判 loaded。此结果同时证明 `fixtures.json`
（含本票新增的 `pstack-technical-writing` active 条目、`assertions` 字段、
`skill_dir: "../../../.agents/skills/pstack-technical-writing"`）能被 schema 校验通过
并成功加载——脚本以 `validateFixtures()` 在启动时解析该文件，解析失败会直接抛错退出，
本次未抛错。

`fixtures.json` 里 `pstack-technical-writing` 的 3 正例 + 3 反例 prompt 已就位（README/
ADR/PR 文案结构改写 vs unslop 纯语言润色/逐字引文/营销文案），真实模型驱动的
`--item pstack-technical-writing` 实跑（判定「自动触发/跳过」）留给独立评审工蜂或宿主
验收执行，理由：本阶段时间闸禁止真实模型调用（见票内「本批次时间闸」段）。

## ④ 端到端手工验证样本（受保护内容 + 结构/清晰度）

由于本阶段不跑真实模型，本节用**手工按 Skill 规则改写**的方式模拟端到端效果，
样本同时含代码、命令、标识符、日志、错误信息、结构化数据（JSON）、测试输出、逐字引文
八类受保护内容，覆盖两条 `assertions`。

### 受保护内容逐字比对（`tw-assert-protected-verbatim`）

改写前后，以下 7 段逐字截取自 before/after 样本，全部按子串精确比对：

| 受保护内容类型 | 结果 |
|---|---|
| 代码块（Python） | 逐字相同 |
| 命令（`python budget_checker.py --write`） | 逐字相同 |
| 错误信息（`ERROR: budget exceeded: ...`） | 逐字相同 |
| 日志（两行 `INFO`/`ERROR` 时间戳日志） | 逐字相同 |
| 测试输出（`test result: ok. 2 passed; ...`） | 逐字相同 |
| 结构化数据（`budget.json` 的 JSON 块） | 逐字相同 |
| 逐字引文（`014-第14课.md:34` 一句） | 逐字相同 |

验证脚本（子串包含比对，7/7 PASS）：

```python
protected_snippets = [代码块, bash 命令, 错误信息, 日志两行, 测试输出三行, JSON 块, 逐字引文]
for snip in protected_snippets:
    assert snip in before_text and snip in after_text
# 结果：0 True True / 1 True True / 2 True True / 3 True True
#      4 True True / 5 True True / 6 True True
```

### 结构与清晰度改善（`tw-assert-structure-clarity`）

- **文档类型（Diátaxis）**：改写前混杂说明与操作指引不分段；改写后标题分「使用方法」
  一节，动作型内容（调用 `load_config()`/`validate()`、运行命令）与说明型内容
  （模块做什么）分离，贴合 reference + how-to 的边界。
- **读者任务（Google 风格）**：改写前「你需要先调用……然后你可以调用……」用第二人称但
  夹杂被动与从句堆叠；改写后改成祈使句「调用 `load_config()` 取回……再调用……」，条件
  前置（「只有在需要下调预算时才加 `--write`」）。
- **句子载重（STE）**：改写前单句「运行下面这条命令可以在命令行里直接跑一次校验，并且
  会把当前的导入计数写回 `budget.json`（注意：只有在需要下调预算时才应该这样做）」一句
  夹两个动作+一个条件；改写后拆成两句，条件前置到独立分句。
- **可验证事实**：标识符（`load_config`、`Config`、`validate()`）、文件名
  （`budget.json`）、命令与代码在改写前后保持真实且未被替换为同义词描述——符合 Skill
  「代码库即词表」的规则。
- **空话删除**：「总的来说」「另外值得一提的是」「其实是比较复杂的」等填充语在改写后
  删除（Skill 三条总规则之一：「删掉不承担任何功能的词」）。

样本文件（本次验证的工作产物，不入库、仅记录于此供审计复现）：

```text
/tmp/before.md  （改写前，八类受保护内容 + 冗余自然语言）
/tmp/after.md   （改写后，结构分节 + 句子清晰度改善，受保护内容逐字不变）
```

## ⑤ 命名、固定 SHA、来源、许可证核对

- `frontmatter.name`：`pstack-technical-writing`；`license: MIT`；无
  `disable-model-invocation`（`both` 调用契约：模型可见可自动调用 + 保留
  `/skill:pstack-technical-writing` 显式入口）。
- `metadata.upstream-repo` = `cursor/plugins`，`metadata.upstream-sha` =
  `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`，`metadata.upstream-path` =
  `pstack/skills/technical-writing/SKILL.md`，`metadata.license-file` = `LICENSE.MIT`——
  与 `docs/agents/pstack-lite/SOURCE-STATEMENT.md` 规定的必填字段一致。
- `.agents/skills/pstack-technical-writing/LICENSE.MIT` 与
  `docs/agents/pstack-lite/LICENSE.MIT`（#1130 已核对 SHA-256
  `bc957ca6bee02792566a1a028d105e02e247c6e77cf057061674273da77b200e`）逐字节一致
  （`diff` 空输出）。
- `docs/agents/skill-local-customizations.md` 的 pstack-lite 索引已把
  `pstack-technical-writing` 行状态改为「已实现（#1135）」并填改写摘要。

## ⑥ unslop 唯一用户级副本核对

- 本票未在 `.agents/skills/` 或任何项目路径下创建 `unslop` 或包装同名目录；
  `check-skills.mjs` 本沙盒结果 `unslop: absent`（因该 Skill 只装在主机
  `~/.agents/skills/unslop/`，不在本沙盒镜像内，属既有边界，非本票引入）。
- `git status` 确认本票新增文件只有 `.agents/skills/pstack-technical-writing/`
  一个目录，未新建或修改任何 `unslop` 相关路径。

## 已知边界

- 真实模型驱动的 `run-trigger-fixtures.mjs --item pstack-technical-writing`
  （AC②「自动触发」的模型面证据）未在本阶段执行，留给独立评审工蜂或宿主验收；
  `fixtures.json` 的正例/反例 prompt 与 schema 已就位，可直接跑。
- ④的端到端样本为手工按 Skill 规则改写、非真实模型输出；`assertions` 两条的语义已由
  该样本人工验证一致，真实模型输出的等价验证同样留给触发夹具实跑覆盖。
