# 051 — 运行时连接设计：元编排/结晶/谱系三者自生长回路

**类型**: 定理（四分法复核：从选择重分类）
**状态**: 已结算
**结算日期**: 2026-02-20
**结算方式**: 四分法复核——A/C 被已结算原则消除，B 是逻辑唯一解
**日期**: 2026-02-20
**提案者**: architect 工位
**前置**: 043-self-growth-loop, 049-unified-orchestration-protocol
**关联**: 016-runtime-enforcement-layer, 042-hook-network-pattern, 041-orchestrator-proxy

---

## 一、当前连接图（已通 vs 断裂）

### 已实现的连接

```
[session_end hook]
  post-session-pattern-detect.sh
    → 写入 .chanlun/pattern-buffer.yaml
    ✓ 路径已通（hook 存在）

[git commit hook]
  crystallization-guard.sh
    → 读取 pattern-buffer.yaml（frequency >= 3 且 status=candidate/settled）
    → 读取 .crystallization-debt.json
    → 阻断 commit（如有未处理模式）
    ✓ 路径已通（hook 存在，逻辑完整）

[router.py]
  match_event() 纯函数
    → 从 dispatch-spec.yaml 加载路由表
    → 返回命中路由列表
    ✓ 路径已通（代码存在，测试通过）

[dispatch-spec.yaml]
  orchestration_protocol.routes
    → session_end → pattern_detector → post_session_pattern_detect
    → session_start → meta_lead → ceremony
    ✓ 路由声明已存在

[skill-crystallizer agent]
  .claude/agents/skill-crystallizer.md
    → 读取 pattern-buffer → 生成 skill → 提交 MutationRequest → 注册 manifest
    ✓ agent 定义存在

[evolution/ 模块]
  mutation.py + registry.py + manifest_reader.py
    ✓ 代码存在
```

### 断裂点（5处）

```
断裂 A：router.py ↔ hook 网络
  router.py 是纯函数，返回路由列表但不执行。
  没有任何调用者将 router.match_event() 的结果转化为实际 hook 调用。
  → 路由表是声明，不是运行时。

断裂 B：session_end 事件 ↔ pattern_detector 目标
  dispatch-spec.yaml 声明了 session_end → pattern_detector 路由。
  但 pattern_detector 不是一个可被调用的实体——
  post-session-pattern-detect.sh 是 hook，不是 agent，无法被路由表"调用"。
  → 事件源和执行者之间没有桥接层。

断裂 C：pattern-buffer 达标 ↔ skill-crystallizer spawn
  crystallization-guard.sh 检测到达标模式后只做阻断，不触发 skill-crystallizer。
  skill-crystallizer 的 activation_condition 在 dispatch-spec.yaml 中声明，
  但没有任何机制在条件满足时自动 spawn 该 agent。
  → 检测到信号，但信号没有接收者。

断裂 D：skill-crystallizer 完成 ↔ 谱系记录
  skill-crystallizer.md 第4步写道"向 genealogist 发送消息，请求记录结晶事件"。
  但 genealogist 是结构工位，在 session 中持续运行——
  skill-crystallizer 是按需激活的，两者的生命周期不同步。
  → 消息发送路径存在，但接收方的存在性无法保证。

断裂 E：谱系 settled ↔ 元编排自动加载
  043号谱系声明"新 skill 注册到 manifest → 下次 ceremony 自动加载"。
  ceremony 序列（dispatch-spec.yaml ceremony_sequence）中没有"扫描新增 skill"步骤。
  → manifest 更新后，元编排不知道有新 skill。
```

---

## 二、连接方案

### 方案 A：事件总线执行层（最小侵入）

**核心思路**：在 router.py 之上增加一个薄执行层 `event_bus.py`，
将路由匹配结果转化为实际的 agent spawn 或 hook 调用。

**需要新增/修改的文件**：

```
新增：src/newchan/orchestration/event_bus.py
  - dispatch(event_type, context) → 调用 router.match_event()，
    对每条命中路由执行对应 action
  - action 执行器映射表：
    "post_session_pattern_detect" → 调用 post-session-pattern-detect.sh
    "spawn_skill_crystallizer"    → 通过 Task tool spawn skill-crystallizer agent
    "ceremony"                    → 通知 meta-lead 执行 ceremony
    "lineage_consistency_check"   → 通知 genealogist

新增：dispatch-spec.yaml 中 automation.crystallization 增加字段：
  spawn_trigger: "pattern_buffer_candidate_threshold"
  spawn_action: "spawn_skill_crystallizer"

修改：crystallization-guard.sh
  阻断后，额外写入 .chanlun/.pending-crystallization-spawn.flag
  （信号文件，供 event_bus 轮询）

修改：dispatch-spec.yaml ceremony_sequence
  在 load-definitions 步骤后增加：
  - step: "scan-new-skills"
    action: "读取 manifest.yaml，加载上次 ceremony 后新增的 skill"
```

**事件流路径**：

```
session_end
  → post-session-pattern-detect.sh（hook，直接执行）
    → 写入 pattern-buffer.yaml
      → crystallization-guard.sh 检测到达标模式
        → 写入 .pending-crystallization-spawn.flag
          → event_bus.py 轮询发现 flag
            → spawn skill-crystallizer agent
              → 结晶完成 → 更新 manifest.yaml
                → 下次 ceremony 的 scan-new-skills 步骤加载新 skill
```

**与现有 hook 网络的集成**：
- hook 继续作为事件入口（不变）
- event_bus 作为 hook 和 agent 之间的桥接层
- 不修改现有 hook 逻辑，只在 crystallization-guard 末尾增加 flag 写入

**断裂修复覆盖**：A ✓ B ✓ C ✓ E ✓（D 部分修复：通过 genealogist 消息路径）

---

### 方案 B：谱系驱动的 Pull 模型（最符合现有架构风格）

**核心思路**：不新增执行层，而是让各工位在被唤起时主动扫描文件系统状态，
按需执行。连接通过文件系统状态（pattern-buffer、manifest、谱系）隐式建立。

**需要新增/修改的文件**：

```
修改：.claude/agents/genealogist.md
  增加职责：每次被唤起时，扫描 pattern-buffer.yaml 中 status=candidate 且
  frequency >= promotion_threshold 的模式，如有则 spawn skill-crystallizer。

修改：.claude/agents/meta-lead.md
  ceremony 的 load-definitions 步骤后增加：
  扫描 manifest.yaml 的 generated 时间戳，
  如果比上次 session 新，重新加载 skill 列表。

修改：dispatch-spec.yaml structural_stations.genealogist.purpose
  增加："pattern-buffer 扫描与 skill-crystallizer spawn"

新增：.chanlun/genealogy/settled/ 中结晶事件记录格式约定
  （skill-crystallizer 完成后写入，genealogist 下次扫描时发现）
```

**事件流路径**：

```
session_end
  → post-session-pattern-detect.sh（hook，直接执行）
    → 写入 pattern-buffer.yaml

[下次蜂群循环，genealogist 被唤起]
  → 扫描 pattern-buffer.yaml
    → 发现达标模式 → spawn skill-crystallizer
      → 结晶完成 → 写入 manifest.yaml + 写入谱系结晶记录

[下次 ceremony]
  → meta-lead 扫描 manifest.yaml 时间戳
    → 发现新 skill → 加载
```

**与现有 hook 网络的集成**：
- 完全不修改 hook
- 利用现有文件系统作为消息总线（meta-lead.md 已有此模式）
- genealogist 已是结构工位，扩展其职责最小侵入

**断裂修复覆盖**：B ✓ C ✓ D ✓ E ✓（A 保持现状：router.py 作为工具库，不作为运行时）

---

### 方案 C：dispatch-spec 驱动的 Agent 自检（最激进）

**核心思路**：将 dispatch-spec.yaml 的 orchestration_protocol.routes 变成
每个 agent 启动时的自检清单。每个 agent 在执行前先查路由表，
判断自己是否应该触发其他 agent。

**需要新增/修改的文件**：

```
新增：src/newchan/orchestration/self_check.py
  - check_triggers(agent_name, completed_actions) → list[Route]
    查路由表，返回当前 agent 完成的动作应该触发的下游路由

新增：.claude/agents/base-protocol.md（所有 agent 的公共前置）
  每个 agent 完成主要动作后，调用 self_check.py 检查是否需要触发下游

修改：所有 structural_stations agent 定义
  在末尾增加"完成后执行 self_check"步骤

修改：dispatch-spec.yaml orchestration_protocol.routes
  增加 agent_completion 事件类型：
  - event: agent_completion
    pattern: "skill-crystallizer"
    target: genealogist
    action: "record_crystallization_event"
  - event: agent_completion
    pattern: "genealogist"
    target: meta-lead
    action: "update_skill_registry"
```

**事件流路径**：

```
skill-crystallizer 完成
  → self_check.py 查路由表
    → 发现 agent_completion/skill-crystallizer → genealogist/record_crystallization_event
      → spawn genealogist（或发消息给已运行的 genealogist）
        → genealogist 完成
          → self_check.py 查路由表
            → 发现 agent_completion/genealogist → meta-lead/update_skill_registry
              → meta-lead 更新 skill 注册表
```

**与现有 hook 网络的集成**：
- hook 作为外部事件入口（不变）
- agent 内部通过 self_check 形成链式触发
- 路由表成为系统的单一 truth source（连接声明和执行）

**断裂修复覆盖**：A ✓ B ✓ C ✓ D ✓ E ✓（全覆盖，但复杂度最高）

---

## 三、方案对比

| 维度 | 方案 A（事件总线执行层） | 方案 B（Pull 模型） | 方案 C（Agent 自检） |
|------|------------------------|-------------------|-------------------|
| 新增代码量 | 中（event_bus.py + flag 机制） | 小（只改 agent 定义） | 大（self_check.py + 所有 agent 修改） |
| 与现有架构一致性 | 中（新增执行层） | 高（沿用文件系统总线） | 低（引入新协议） |
| 断裂修复完整性 | 4/5 | 4/5 | 5/5 |
| 可观测性 | 中（flag 文件） | 低（隐式） | 高（路由表可查） |
| 引入新依赖 | 是（轮询机制） | 否 | 是（self_check 协议） |
| 与 meta-lead.md 风格一致 | 部分 | 完全一致 | 不一致 |

---

## 四、结算记录（v6 session 四分法复核）

原分类为"选择"，四分法复核重分类为"定理"：

### 消除推导

| 方案 | 消除依据 | 已结算原则 |
|------|----------|-----------|
| C | 修改所有 agent 定义 = spec 层强制 | 039号："016 模式在 spec 层的复现通过环境层（hook 网络）而非 spec 层解决" |
| A | 引入 event_bus.py 轮询 = 第二套运行时 | 042号：hook 网络是运行时层，不应引入竞争性运行时机制 |
| B | 唯一剩余 | 042号（文件系统通信模式）+ 039号（环境层优先）均支持 |

### 断裂 A 的重新评估

断裂 A（router.py 不执行）不是真正的断裂。042号确立 hook 是运行时层，router.py 作为纯函数参考工具库是 by-design。路由表的价值在于声明性文档和 match_event() 查询，不在于自动执行。

### 边界条件验证

方案 B 的前提"genealogist 在 session 中被唤起"已由 dispatch-spec 保证：genealogist 是 mandatory structural station（can_be_skipped: false）。

### 已结算原则

**方案 B（Pull 模型）是唯一符合已结算原则的运行时连接方案。** 实现路径：修改 genealogist.md + meta-lead.md + dispatch-spec.yaml。

### 下游行动（行动类，可直接执行）

1. 修改 genealogist.md：增加 pattern-buffer 扫描职责
2. 修改 meta-lead.md：ceremony 增加 scan-new-skills 步骤
3. 修改 dispatch-spec.yaml：更新 genealogist purpose + ceremony sequence

---

## 五、结果包

**结论**：识别出 5 处运行时断裂，提出 3 个修复方案。

**定义依据**：
- 043号：自生长回路要求 pattern-buffer → 分型检测 → 结晶 → 元编排加载 的完整闭合
- 049号：语义事件总线要求路由声明对应实际执行
- meta-lead.md：文件系统是非中断信息流的标准通道

**边界条件**：
- 如果 genealogist 在 session 中不被唤起（方案 B 的前提），Pull 模型失效
- 如果 session 频率极低（每天一次），方案 B 的延迟可接受；高频 session 下方案 A 更合适
- 如果 dispatch-spec.yaml 路由表继续扩张，方案 C 的维护成本会超过收益

**下游推论**：
- 选择方案 B → genealogist.md 需要修改，dispatch-spec.yaml 需要小幅修改
- 选择方案 A → 需要新增 event_bus.py，并建立轮询机制
- 选择方案 C → 需要新增 self_check.py 并修改所有 agent 定义

**谱系引用**：
- 043号（自生长回路）：本文档是其"待实现"部分的具体化
- 049号（统一编排协议）：本文档是其路由表 runtime 实现的设计
- 016号（运行时强制层）：断裂 A-E 均是"声明存在但执行不存在"的实例

**影响声明**：
- 写入 `.chanlun/genealogy/pending/051-runtime-connection-design.md`
- 不修改任何现有文件
- 等待编排者选择方案后，由对应工位执行实现
