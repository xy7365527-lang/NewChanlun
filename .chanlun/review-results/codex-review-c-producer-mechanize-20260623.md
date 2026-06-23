# Codex 异质审计（约束4）— 任务 #41 (c) 生产端机制化 hook 修复

> 模式：codex exec（ChatGPT 订阅登录路径；newchan.codex API 路径因 429 insufficient_quota 不可用）
> 模型：gpt-5.2-codex（gpt-5.3-codex 降级）｜ tokens: 43,182 ｜ 日期: 2026-06-23
> 审计目标：ceremony-completion-guard.sh check2 (c)spawn mandate 注入 + team-topology.json spawn_mandate
> 审计请求 A-E：正确性 / 拦停回归 / 健壮性 / 单一源drift / shell注入

## Codex 逐项裁决（原文）

**逐项裁决**

A. **CONCERN**  
[hook:393](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:393)-[400](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:400) 确实让 Stop-Guard 的 `(c)spawn` 路由文本携带 mandate。但它只是要求 Lead 把 mandate 放进被 spawn 工位 prompt；hook 没有直接改写实际 `Agent/Task` payload，也没有校验新工位第一步是否真的 invoke `sub-swarm-ceremony`。所以“路由每次提示携带”成立，“每次 spawn 自动注入并执行”不完全成立。

B. **FAIL**  
diff 新增了 check0：[hook:51](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:51)-[110](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:110)。其中 [hook:97](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:97)-[108](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:108) 在 context 达阈值时直接输出 `continue: true` 并 `exit 0`，绕过 check1/check1.5/check2/check3/check4/check5。无论 task#40 是否有独立理由，这都违反本审计的硬约束“只允许改 check2 的 (c)spawn 文本，不许动任何 block 判定”。

C. **CONCERN**  
`.claude/team-topology.json` 读取失败时，内部 `try/except` 会打印 fallback，随后 [hook:413](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:413)-[416](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:416) 仍会输出 block JSON。  
但外层 `|| SPAWN_MANDATE=""` 是弱 fallback：如果该 Python 片段本身语法/运行失败，JSON block 仍会输出，但 mandate 为空，路由会变成“prompt 必须含：”的空指令。对“block JSON 正常输出”是可接受，对“mandate 永不丢失”有洞。

D. **FAIL**  
[team-topology:136](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:136) 与 [hook fallback:366](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:366)-[371](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:371) 内容已经不一致：topology 支持 `codex-challenger 或 gemini-challenger`、写明 `unowned`、`blockedBy 全部任务节点`、`.chanlun/+SendMessage`；fallback 更简略且只写 `codex-challenger`。这不是单一源，而是双源加声明膨胀，后续必然 drift。

E. **PASS**  
当前实现没有明显 shell 注入风险。`SPAWN_MANDATE=$(...)` 在赋值上下文不会做词拆分；传参处使用 `"$SPAWN_MANDATE"`，中文、括号、冒号、空格和引号都作为单一 argv 传入 Python；最终用 `json.dumps(..., ensure_ascii=False)` 输出 JSON。除非未来把 mandate 拿去 `eval` 或拼接成 shell 代码，否则这里安全。

**总裁决**：hook 的 `(c)spawn` 文本注入方向基本正确，但本 diff 存在明确拦停回归，且 fallback 破坏单一源；因此不能裁定为“正确且无拦停回归”。
2026-06-23T09:19:12.909738Z ERROR codex_core::session: failed to record rollout items: thread 019ef3c3-4d9a-7351-874a-1806cd8e3c86 not found
tokens used
43,182
**逐项裁决**

A. **CONCERN**  
[hook:393](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:393)-[400](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:400) 确实让 Stop-Guard 的 `(c)spawn` 路由文本携带 mandate。但它只是要求 Lead 把 mandate 放进被 spawn 工位 prompt；hook 没有直接改写实际 `Agent/Task` payload，也没有校验新工位第一步是否真的 invoke `sub-swarm-ceremony`。所以“路由每次提示携带”成立，“每次 spawn 自动注入并执行”不完全成立。

B. **FAIL**  
diff 新增了 check0：[hook:51](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:51)-[110](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:110)。其中 [hook:97](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:97)-[108](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:108) 在 context 达阈值时直接输出 `continue: true` 并 `exit 0`，绕过 check1/check1.5/check2/check3/check4/check5。无论 task#40 是否有独立理由，这都违反本审计的硬约束“只允许改 check2 的 (c)spawn 文本，不许动任何 block 判定”。

C. **CONCERN**  
`.claude/team-topology.json` 读取失败时，内部 `try/except` 会打印 fallback，随后 [hook:413](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:413)-[416](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:416) 仍会输出 block JSON。  
但外层 `|| SPAWN_MANDATE=""` 是弱 fallback：如果该 Python 片段本身语法/运行失败，JSON block 仍会输出，但 mandate 为空，路由会变成“prompt 必须含：”的空指令。对“block JSON 正常输出”是可接受，对“mandate 永不丢失”有洞。

D. **FAIL**  
[team-topology:136](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:136) 与 [hook fallback:366](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:366)-[371](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:371) 内容已经不一致：topology 支持 `codex-challenger 或 gemini-challenger`、写明 `unowned`、`blockedBy 全部任务节点`、`.chanlun/+SendMessage`；fallback 更简略且只写 `codex-challenger`。这不是单一源，而是双源加声明膨胀，后续必然 drift。

E. **PASS**  
当前实现没有明显 shell 注入风险。`SPAWN_MANDATE=$(...)` 在赋值上下文不会做词拆分；传参处使用 `"$SPAWN_MANDATE"`，中文、括号、冒号、空格和引号都作为单一 argv 传入 Python；最终用 `json.dumps(..., ensure_ascii=False)` 输出 JSON。除非未来把 mandate 拿去 `eval` 或拼接成 shell 代码，否则这里安全。

**总裁决**：hook 的 `(c)spawn` 文本注入方向基本正确，但本 diff 存在明确拦停回归，且 fallback 破坏单一源；因此不能裁定为“正确且无拦停回归”。
