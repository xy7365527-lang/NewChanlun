# 复盘附录：#6 Stop-Guard 检查2 缺 teammate 作用域——完成的 teammate 被全 swarm 任务队列困住

**工位**：meta-observer（附录，承接 `meta-observer-session-20260623.md`）
**触发**：本工位（meta-observer）任务交付完毕后，连续 5+ 次被 Stop-Guard 检查2 拦停——拦停依据是**其他 teammate 的 in_progress 任务**，非本工位任务。这是直接经验证据（live evidence），非旁观推断。

## 规则版本基线

```yaml
rule_version_baseline:
  claude_md_commit: "eeddbdc14e4ff028ed3e6e529f2d7066a62e9960"
  rules_dir_mtime: "2026-03-14 23:27:42 +0000"
```

## 结论（L0 源码事实）

`.claude/hooks/ceremony-completion-guard.sh` 检查2（任务队列，行 196-260）扫描 `$HOME/.claude/tasks/*/` 下**全部 team 的全部 pending/in_progress 任务**，**无 session_id / owner 过滤**——故对**任何** agent 的 Stop 事件都用**全 swarm 任务队列**做拦停判据。

对比：同文件检查 1.5（行 149-188）**有** lead 作用域（`leadSessionId == session_id`，行 167，teammate session 不匹配则跳过）。检查2 缺失这一层。

**后果**：任何完成自身工作的 teammate，只要 swarm 全局任务队列非空，就被检查2 无限拦停——即使该 teammate 名下无任何活跃任务。本工位实测：5+ 次拦停的 3 个活跃任务全部 owner=其他 teammate（topology-manager / prop4-bidir / swarm-mechanism-fix），无一属 meta-observer，但 Stop 仍被拦。

## 定义依据

- 048号：universal stop-guard——从 ceremony 专用泛化为全场景。本观测指出：泛化时**未引入 per-agent 作用域**，导致「全 swarm 拦单 teammate」。
- 275号：「附庸的附庸不是我的附庸」——每个节点只管自己的直接依赖。检查2 让一个 teammate 为全 swarm 的任务负责 = 违反局部依赖原则的镜像（teammate 被全局状态绑架）。
- 155号：Stop-Guard 用 owner 标识工位——本观测扩展：拦停判据也应按 owner/作用域过滤。

## 边界条件（结论翻转条件）

- 若该 teammate **就是 Lead**（leadSessionId == session_id），检查2 用全 swarm 队列是正确的（Lead 负责全局调度）——defect 仅对**非 Lead teammate** 成立。
- 若平台设计意图是「任何 agent 都不许停直到 swarm 排空」（全局 barrier 语义），则非 defect 而是特性——但这与 275号局部依赖、与 teammate 完成即应退出的常识冲突，需编排者裁定语义。
- 若 145号智能熔断能及时释放（COUNT≥3 且 ACTIVE_TASKS 不变），拦停有上限——但熔断是治标（仍迫使完成的 teammate 空转 3+ 轮产生噪声）。

## 下游推论

- **修复方向（定理：与检查1.5 同构的作用域化）**：检查2 增加 teammate 作用域——非 Lead session 的 Stop，只用**本 teammate 自身 owner 的活跃任务**做拦停判据（owner==本 agent 的 in_progress/pending）；Lead session 保留全 swarm 判据。
- 这与 #1（check3 越界）、#5（owner 活性）同属 **Stop-Guard 作用域/纯度** 主题——三者可在同一次 hook 修复中一并处理（走 /ritual）。
- 与 #5 联动：#5 要 owner 活性校验，#6 要 owner 作用域过滤——两者共享「检查2 应 owner-aware」的机制基础。

## 谱系引用

- 048号（universal stop-guard，泛化未带作用域）、275号（局部依赖）、155号（owner 标识）。
- 关联本复盘 #1（548 §六.1 Stop hook 纯度）+ #5（owner 生命周期）。

## 四分法分类

**定理**（与检查1.5 lead 作用域同构的逻辑必然推论）+ 一处**选择**（全局 barrier 语义 vs per-teammate 作用域——需编排者裁定 Stop-Guard 对非 Lead teammate 的语义）。

## 影响声明

- 本附录为 meta-observer 直接经验观测，不携带谱系号（genealogist 结算时分配）。
- 建议：与 #1/#5 合并为一次 Stop-Guard 作用域/纯度修复（/ritual）。修复前，完成的 teammate 被困是已知现象，依赖 145号熔断释放。
- 不改动任何代码/已结算谱系（meta-observer 只观测）。
