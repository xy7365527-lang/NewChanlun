# 155号谱系：引入 Codex 作为代码层异质审查者

**id**: 155
**status**: 已结算
**type**: 选择（编排者决断 + Gemini decide 审查收敛）
**date**: 2026-02-23

## 矛盾发现

蜂群的异质质询架构存在空位：
- Gemini challenger = 概念层/数学层异质否定（challenge/verify/decide/derive）
- Claude challenger = 反向质询（对 Gemini 产出再质询）
- **代码层审查仍是 Claude 同质审查**（code-reviewer/python-reviewer 都是 Claude）

代码层缺少异质否定源——无法发现 Claude 模型家族共享的代码认知偏差。

## 决断过程

### Gemini decide() 审查（4 条判定）

1. **增加 `decide` 模式**：Codex 需要代码层技术选型代理（数据结构、算法选择）
   - system prompt 聚焦代码域技术选型，不需要完整四分法
2. **Claude-challenger 必须扩展覆盖 Codex 产出**：否则代码层形成封闭的"技术官僚主义"
3. **触发条件修正**：
   - 测试失败 → 自动触发 diagnose（对象否定对象，005b号）
   - 拒绝行数阈值触发（原则3：不允许阈值否定）
4. **定义忠实度处理**（Gemini 原始判定 vs 编排者修正）：
   - Gemini 原始判定：定义忠实度从 Codex 剥离，由 Gemini 交叉验证
   - 编排者修正："让 Codex 在 agent team 中，它不就知道了吗？"
   - 收敛方案：codex-challenger agent（Claude 节点）构建上下文时注入相关缠论定义 → Codex 自然获得定义忠实度审查能力 → 不需要 Gemini 交叉验证
   - 与 Gemini challenger 同构：Gemini 也不"了解缠论"，每次都是 agent 构建上下文后注入的

### 编排者决断

引入 Codex 作为代码层异质审查者。严格双向讨论，严格诊断，严格修复——不是单向 lint，是异质对审。

## 最终设计

### 三种模式

| 模式 | 用途 | 对标 Gemini |
|------|------|-------------|
| `review` | 代码审查——逻辑自洽、边界安全、性能、惯用法、定义忠实度 | challenge |
| `diagnose` | 严格诊断——失败现象→根因→修复方案 | verify |
| `decide` | 代码层技术选型代理——数据结构/算法/架构选择 | decide |

不做 derive（数学推导是 Gemini 的领域）。

### 架构位置

- `src/newchan/codex/` — Python 模块（engine + registry + modes + CLI）
- `.claude/agents/codex-challenger.md` — agent 定义（Claude 节点，拥有完整工具集）
- agent team 成员，通过 Task spawn（不是外部 API 工具）

### 否定来源链完整性

| 层面 | 否定源 | 覆盖 |
|------|--------|------|
| 概念层 | Gemini challenger | challenge/verify/decide/derive |
| 代码层 | Codex challenger | review/diagnose/decide |
| 再质询 | Claude challenger | 对 Gemini + Codex 产出再质询 |

## 下游推论

1. Claude-challenger 再质询范围扩展为 Gemini + Codex 产出
2. dispatch-dag 注册 codex-challenger 为 conditional skill
3. `/code-review` 后自动触发 Codex 异质审查

## 谱系依据

- 030a号：异质否定源的位置（结构工位）
- 005b号：对象否定对象（测试失败 = 对象形式的否定）
- 093号：五约束（约束4：异质验证——异质审查是结构要求）
- 062号：异质性作为结构性要素
- 089号：扬弃（Codex 在 agent team 中 = 内化，不是外部工具）

## 影响声明

- 新增 `src/newchan/codex/` 模块（engine.py, registry.py, modes.py, __main__.py, __init__.py）
- 新增 `src/newchan/codex_challenger.py` 向后兼容垫片
- 新增 `.claude/agents/codex-challenger.md` agent 定义
- 修改 `.claude/agents/claude-challenger.md`（扩展再质询范围）
- 修改 `.chanlun/dispatch-dag.yaml`（注册 codex-challenger）
- 新增 `tests/test_codex_challenger.py`
