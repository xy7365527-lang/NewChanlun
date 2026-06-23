# bootstrap 结构工位机制强制修复（20260623）

**工位**：bootstrap 机制修复
**谱系号**：562（`.chanlun/genealogy/settled/562-bootstrap-structural-enforce.md`）
**编排者裁决**：结构工位是 teammate（必须 spawn），bootstrap 从文本提示升格为机制强制。

## 结论

在 `.claude/hooks/ceremony-completion-guard.sh`（Stop-Guard）新增**检查 1.5：结构工位
bootstrap 强制**，缺任一常设结构工位即 block + 路由具体缺失列表。bootstrap 由「纯文本提示」
（agent-team-bootstrap.sh 的 systemMessage）升格为「机制强制」（Stop hook 阻断）。

## 改了哪个 hook、哪几行

**文件**：`.claude/hooks/ceremony-completion-guard.sh`

1. **文件头注释**（line 5-9 区域）：在 075号注释下追加「075号扬弃」说明——结构能力恢复为
   teammate（095/096 真递归提供共享 inbox 化解孤岛），重新引入结构工位存在性检查；
   075 的 skill 事件驱动层下沉为 Write/Stop 轻量守卫。

2. **新增检查 1.5**（插入在检查 1 死寂检测之后、检查 2 任务队列之前，约 50 行）：
   - 从 Stop hook 输入提取 `session_id`。
   - 内联 python 扫描 `$HOME/.claude/teams/*/config.json`，按 `leadSessionId == session_id`
     精确定位**本 lead session** 的 team config。
   - 收集 `members[].agentType` 集合，与 6 个常设结构角色比对，输出缺失列表。
   - 缺失非空 → `decision: block` + 路由「立即并行 spawn 缺失的结构工位 [列表]」；
     写计数器 `COUNT+1:PRE_ACTIVE_TASKS`（145 熔断兼容）；`exit 0`。

## 检测机制（严格可靠，非模糊匹配）

**信号 = team config.json 的 `member.agentType`**（不是 display name 的子串匹配）。

- 根因发现：`Task(subagent_type="genealogist")` 落盘时 `member.agentType = "genealogist"`，
  而 display name 可以是业务名（如 `geneal-p4` / `geneal-560` / `topo-mapper`）。
  **agentType 携带规范结构类型，与业务命名解耦** → 业务命名的结构工位自动被识别为已覆盖。
- 本 session 实测：`geneal-p4`、`geneal-560` 的 agentType 均为 `genealogist`，
  检测精确判定缺失 = `[meta-lead, quality-guard, code-verifier, meta-observer,
  topology-manager]`，与编排者描述的实际跳过 5 个**逐字一致**。
- **lead-only 门控**：`leadSessionId == session_id` 天然把检查限制到 lead session
  （teammate 的 session_id 不匹配任何 leadSessionId → 跳过，不误报、不阻断 teammate）。
- **无 team → 不强制**：solo 会话（未形成蜂群）跳过，结构工位仅在蜂群语境强制。

为何不用 display-name 子串匹配：那是模糊近似（topology-analyst 会误判为 topology-manager，
不同业务名的 genealogist 会漏判），违反 no-patch 严格性。agentType 是精确、规范、零误报的信号。

## 测试结果（7 场景全绿）

| 场景 | 输入 | 期望 | 结果 |
|------|------|------|------|
| A 真缺 5 个 | 真实 lead session-14c95478 | block，列出 5 个缺失 | PASS（genealogist 经 geneal-* 识别为已覆盖，不在缺失集） |
| B1 全 6 present | 临时 team，6 规范 agentType | 不 block | PASS |
| B2 仅 genealogist | 临时 team，仅 genealogist | 5 缺，genealogist 不在缺失集 | PASS |
| B3 drop 单个 | 临时 team 去掉 code-verifier | 精确报 [code-verifier] | PASS |
| C 非 lead | 随机 session_id 无匹配 team | 跳过（不 block） | PASS |
| D 空 session_id | 无 session_id 字段 | 跳过（无误匹配） | PASS |
| E 145 熔断 | counter=3:0，任务态稳定 | 放行（避免死锁） | PASS |

- `bash -n` 语法检查通过。
- 现有 Stop-Guard 5 个检查（死寂/任务队列/生成态谱系/proof-required标签/四分法）bit-exact
  保留——检查 1.5 纯加性（block 时 early-exit，放行时直通），与四分法检查（检查 5）不冲突。

## 075 vs 095/096 矛盾解决（Aufhebung）

编排者已裁决（结构 = teammate），故非不可弥合矛盾，按扬弃处理：

- **否定** 075 的「结构 = skill，ceremony 不再 spawn」。
- **保留** 075 的反孤岛动机——095/096 真递归团队的共享 inbox 从根上化解孤岛，恢复 teammate
  不重建孤岛问题。
- **提升** skill + 事件驱动层下沉为 Write/Stop 轻量守卫，与结构工位 teammate 本体并存。
- 谱系交叉引用：562 `negates: ["075"]`；075 `negated_by: ["562"]`（已更新）。

## 谱系号 + commit

- 谱系：**562**（settled，`.chanlun/genealogy/settled/562-bootstrap-structural-enforce.md`）。
- **hook 代码 commit**：`b3756dbc4c`（"feat(hook): bootstrap 结构工位机制强制(检查1.5)"）。
  - 多 session 碰撞：Lead 在本工位 Edit 落盘后，opportunistic 拾取本工位的工作树编辑
    并 commit（commit 内容与本工位 Edit 逐字一致——检查 1.5 措辞、`STRUCT_MISSING` 变量名、
    6 结构工位列表全部匹配）。该 commit 未携带谱系号 562。
  - Lead 在 session note（commit `45bde449fd`）记录此为"bootstrap结构工位强制修复（编排者裁决，
    bootstrap-fix 工位进行中）"+"碰撞裁定"，确认本工位为该任务唯一 owner。
- **谱系/报告 commit**：本工位补 commit `562 + 075 negated_by 交叉引用 + 本报告`
  （hook 代码已在 b3756dbc4c 落地，本 commit 只补未落地的谱系层与报告）。

## 多 session 碰撞处理（feedback_task_queue_owner_liveness 先例）

- hook 代码与本工位 Edit 逐字一致 → 无内容冲突，无需重写或回退（bit-exact）。
- 562 谱系号经核验空闲（HEAD 无 562，无竞争 genealogy 文件描述本修复）→ 安全占用。
- 075 的 `negated_by: ["562"]` 在 HEAD 中缺失（grep=0）→ 本工位编辑为净新增，无冲突。
- 结论：碰撞仅发生在 hook 代码落盘层（已由 Lead 提前 commit，内容等同），谱系层无碰撞，
  本工位补齐谱系 + 报告即完成交付。
