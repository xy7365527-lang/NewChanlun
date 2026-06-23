# 审查结论：#41.2 hook 修复复核（任务 #44 / 约束3 异工位复核）

- **审查节点**：c-producer-review（≠ c-producer-mechanize，满足约束3 执行不可自观）
- **被审对象**：`.claude/hooks/ceremony-completion-guard.sh` + `.claude/team-topology.json` 的 (c)spawn mandate 机制化改动（#41.2 / task#43）
- **评审基准锚点（pin）**：hook blob `76987eae639a129c51716f9f0caaf17c31ded89a`；topology 改动 = `spawn_mandate` 新增字段（+9 行）
- **认识论等级**：L1（hook 逻辑/管线正确性验证，输入为构造的失败模式；非真实蜂群运行的 L2 行为验证）
- **日期**：2026-06-23

> ⚠ **并发修改告警（强约束，影响复核有效性）**：审查期间该 hook 文件被 #41.2 修复节点**并发修改过一次**——
> 首次 Read 捕获的版本用 `|| SPAWN_MANDATE=""` + 内联 `default`；当前评审版本（blob `76987eae63`）改为
> `MANDATE_FALLBACK` bash 变量 + `[ -z ] && SPAWN_MANDATE=$MANDATE_FALLBACK` 守卫（注释标注"codex 异质审计 C/D 修复"）。
> 本结论**仅对 blob `76987eae63` 有效**。若 #41.2 在 commit 前再次修改，#45 异质审计须对最终 commit 版本重验。
> 注：注释引用的"codex 异质审计"修复发生在 task#45（pending）之前，疑为修复节点的预修，非 #45 正式产出。

---

## 结论（A/B/C 逐条判决）

### A. (c)spawn mandate 注入逻辑正确 — **PASS**

**证据（行号 = blob `76987eae63`）**：
- L360–378：`MANDATE_FALLBACK` 定义 + `SPAWN_MANDATE` 从**单一权威源** `.claude/team-topology.json` 的
  `spawn_mandate.template` 读取（canonical single source，编辑只此一处）。
- L386：外层路由 python `mandate = sys.argv[5] if len(sys.argv) > 5 else ''`。
- L393–401：`if unowned:` 分支把 `【spawn mandate 强制（生产端机制化 #41/约束4）——被 spawn 工位 prompt 必须含：{mandate}】`
  嵌入 (c)spawn 路由指令。
- L417：`$SPAWN_MANDATE` 作为 argv[5] 传入路由 python。
- 实测：template 读取成功 len=389，内容以"第一步 invoke sub-swarm-ceremony skill"开头；mandate 引用的
  `.claude/skills/sub-swarm-ceremony/SKILL.md` **文件存在**（11112 B），路径有效。
- **覆盖性论证**：hook 模型中 Lead 的 spawn 路径 == (c)spawn 路径（仅对 owner 空、未阻塞 pending spawn，
  见 L335–337 `UNOWNED_TASKS` 装填）。owned pending 已有工位、不再 spawn。故 mandate 挂在 (c)spawn 指令上
  = 覆盖**每一次** Lead spawn。✓

**边界条件（A 翻转条件 + 诚实限制）**：
- 这是**生产端机制化**——路由 reason **总是携带** mandate 文本；但把 mandate 真正写进被 spawn 子工位的
  `Agent()` prompt 这一步，仍**依赖 Lead 执行该指令**。Stop hook 无法拦截 `Agent()` 工具调用做硬注入。
- 即 016号"no code = no enforce"的残余：机制化到 Stop hook 能触及的上限（路由必带），最终 prompt 注入是
  Lead 执行层。任务 A 问的是"路由指令是否自动带 mandate"——答案 **是**，故判 PASS；但"子 prompt 是否
  必然含 mandate"超出 Stop hook 可机制化范围，属 137号文本提示对执行层的固有天花板，应在 #46 结晶时显式声明。

### B. 未削弱 check1/1.5/2/3/4/5 任何 block/exit 拦停逻辑 — **PASS（针对 #43）**

**证据**：
- `git diff HEAD` 触及 hook 的 hunk：#43 仅 3 个 hunk，**全部落在 check 2 内**
  （`@@ +357` SPAWN_MANDATE 定义、`@@ +382` 路由 python 加 mandate 参数、`@@ +413` argv 传参）。
- check 1（L176–224）、check 1.5（L226–276）、check 3（L421–445）、check 4（L447–474）、
  check 5（L476–501）**均不在 #43 diff 内** → 相对 HEAD 逐字不变 → 未削弱。各自的
  `'decision': 'block'` + `exit 0` 在工作树完好（grep 确认 L219/223、L270/274、L439/443、L468/472、L495/499）。
- check 2（L278–419）：#43 唯一改动的检查。`decision:block`（L413）+ `exit 0`（L417）保留；
  SPAWN_MANDATE 是**纯增量** route reason 内容；ACTIVE_TASKS>0 仍无条件 block。check 2 内**未引入**
  任何"不 block 直接 exit"的新分支。✓

**边界条件 / 分离改动告警（B 翻转条件）**：
- ⚠ 同一未提交工作树**还含 check 0**（L51–110，task#40 注释）——一条**新增的"放行停机"路径**：
  当 `CTX_PCT ≥ RELEASE_PCT(默认85%)` 时 `exit 0` **不输出** decision=block（绕过下方所有 block 检查）。
- check 0 **不属于 #43/#41.2 范围**（属 task#40 编排者裁决：context 临界 compact 优先于蜂群持续，
  no-workaround 严格解，蜂群态已持久化），**非 #43 的意外削弱**，是**蓄意、有裁决依据的 Stop-Guard 设计性放行口**。
- 但它**会随 #43 同一 diff 一并 commit**。任务 B 列举的是 check1/1.5/2/3/4/5（未列 check 0）——故 check 0
  超出本审查 scope，**不构成 #43 的 B-FAIL**；此处**显式 flag 给 #45/Lead**：若按整个 commit 审，check 0 是
  按设计开的拦停口，须确认 task#40 裁决覆盖之（task#40 已 completed，裁决文本在 check 0 注释 L52–64）。

### C. SPAWN_MANDATE 失败模式（python 异常/文件缺失）下 hook 仍输出合法 block JSON — **PASS**

**证据（实测，当前精确逻辑）**：
- L366 `MANDATE_FALLBACK` bash 变量 + L378 `[ -z "$SPAWN_MANDATE" ] && SPAWN_MANDATE="$MANDATE_FALLBACK"`：
  **保证 mandate 永不为空**。
- 逐失败模式实测 → 均回落到 fallback、且最终 block JSON 合法：
  - 文件缺失（cwd 无 team-topology.json）→ python except→pass→空→守卫填 fallback ✓
  - python 二进制崩溃（command-substitution 整体失败，无 `set -e` 故不 abort）→ 空→守卫填 fallback ✓
  - JSON 损坏 → except→pass→空→fallback ✓
  - `spawn_mandate` key 缺失 / template 为空串 → `.get(...,'')` 返回空→守卫填 fallback ✓
- L24 仅 `set -uo pipefail`（**无 errexit `-e`**）→ command-substitution 失败不中止脚本，继续到外层路由 python。
- 外层路由 python（L380–417）：`json.dumps(..., ensure_ascii=False)` 对 mandate 内容做转义，实测含
  引号/大括号/换行的 mandate 仍 roundtrip 为**合法 JSON**；`decision:block` + `exit 0` 保留。
- **fail-loud 确认**：所有 SPAWN_MANDATE 读取失败下，block **仍 fire**，绝不 fail-silent。✓

**边界条件 / 既存观测（非 #43 回归）**：
- 若**外层路由 python 本身**灾难性崩溃（非 SPAWN_MANDATE 读取失败，而是 emit block JSON 的那段 python 死掉），
  脚本会落到 L418 `exit 0` 而**未** emit block → fail-open。此结构**早于 #43**（emit-block python + exit 0 一直如此），
  **非 #43 引入**，且 SPAWN_MANDATE 失败不触发它（C 指定的失败模式已全部 fail-loud）。仅作既存属性记录，
  不计入 #43 的 C 判决。

---

## 三条总判：A=PASS / B=PASS（含 check 0 分离改动 flag）/ C=PASS

无 #43 引入的 no-patch / fail-silent / 拦停削弱违规。唯一需 #45/Lead 留意的是**与 #43 同 diff 捆绑的 check 0
放行口**（task#40 设计性、有裁决），及 A 的**生产端机制化天花板**（016号：子 prompt 实际注入仍 Lead 执行）。

## 定义依据
- **约束3（执行不可自观）**：本审查由 c-producer-review 执行，≠ #41.1/#41.2 的 c-producer-mechanize，满足异工位复核。
- **约束4（093号 异质审计硬节点）**：本节点为同质 Claude 审查，**不替代** #45 codex-challenger 异质审计；本结论
  作为 #45 输入（任务 #44 描述要求）。
- **137号（正面格式机制化）**：mandate 用正面格式嵌入路由，符合"否定性文本提示对执行层无效"的修复方向；
  但其执行层落地仍受 016号约束（见 A 边界）。
- **no-patch-mentality / no-workaround**：check 0 是 task#40 对"蜂群持续 vs compact"真矛盾的严格解（非绕过），
  不在本审查否定范围。

## 边界条件（本审查结论翻转条件）
1. 若 #41.2 在 commit 前再次修改 hook（blob ≠ `76987eae63`）→ 本 A/B/C 结论失效，须重验。
2. 若 #45 codex 异质审计判定 check 0 放行口超出 task#40 裁决覆盖范围 → B 的分离改动 flag 升级为 commit 级阻断项。
3. 若实蜂群运行（L2）显示 Lead 收到 mandate 路由后**未**将 mandate 注入子 prompt → A 的"生产端机制化"有效域
  被否证，需追加执行层机制（超 Stop hook 能力，属 016号开放轴）。

## 下游推论
- A=PASS ⇒ #35 消费端（Lead 扫无主任务 spawn）+ #41 生产端（工位布设子 DAG）的 (c) 双端机制化在路由层闭合；
  但闭环的最后一跳（子 prompt 实际含 mandate）仍是 Lead 行为依赖，#46 结晶须诚实声明此天花板，勿声明膨胀为"硬强制"。
- B=PASS ⇒ Stop-Guard 五检查拦停面在 #43 后不变；但 check 0 改变了整体停机语义（context 临界优先 compact），
  下游任何依赖"蜂群任务活跃 ⇒ 必 block"假设的逻辑须知晓 85% 放行例外。
- C=PASS ⇒ mandate 读取链 fail-loud，单一源 + 降级 fallback 不维护双份副本（避免 drift），符合 config 化原则。

## 谱系引用
- 涉及概念分离领域：(c) 生产端/消费端机制化（#35/#41）、约束3/约束4（093号/095/096/097 四类节点 DAG）、
  016号（no code = no enforce）、137号（正面格式）、075号扬弃（结构工位=teammate）。
- 不确定是否已有"context 临界放行 vs 蜂群持续"的 settled 谱系——check 0 注释引 task#40 编排者裁决，
  但未见对应 `.chanlun/genealogy/settled/` 条目；建议 #46/genealogist 核查是否需结晶 task#40 裁决为谱系。

## 影响声明
- 本产出**仅为审查结论**，**未改动**任何代码/配置（审查节点不修复，no-patch：发现削弱不替修）。
- 落盘：`.chanlun/review-results/c-producer-review-44-20260623.md`（作为 #45 codex 异质审计输入）。
- 触及判断的模块：`ceremony-completion-guard.sh`（check 0 + check 2）、`team-topology.json`（spawn_mandate）。
