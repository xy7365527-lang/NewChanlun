# CLAUDE.md

## Language
Always respond in Chinese-simplified (简体中文).

## 缠论资料入口（本仓库）

- **速查/可编码定义**：`缠论知识库.md`
- **缠师原始博文（一级权威）**：`docs/chanlun/text/blog/INDEX.md`（108课 + 序篇 + 课间博文 + 答疑）
- **《股市技术理论》编纂版拆分**：`docs/chanlun/text/chan99/INDEX.md`
- **两份思维导图（文字版）**：`docs/chanlun/text/mindmaps/INDEX.md`
- **资料总览**：`docs/chanlun/README.md`

## 来源权威性（三级权威链）

缠论原文的权威性按以下顺序递减：

1. **缠师原始博文**（108课 + 课间答疑 + 附加文章如《忽闻台风可休市》）— **最终权威**
   - 仓库路径：`docs/chanlun/text/blog/INDEX.md`
   - 来源：https://www.fengmr.com/chanlun.html （枫之羁绊，原文+配图+答疑）
2. **《股市技术理论》编纂版**（`docs/chanlun/text/chan99/`）— 本仓库当前主要参考
   - 这是第三方从博文编纂而成的书籍，**可能有遗漏或编排差异**
   - 已知遗漏：第67课（线段划分标准）、第71课（线段再分辨）、第77课、第78课（古怪线段）、《忽闻台风可休市》（新笔定义）等
3. **思维导图 / 第三方总结**（`docs/chanlun/text/mindmaps/`）— 辅助理解，不作为定义依据

**原则**：当层级间有出入时，以更高层级为准。当前仓库主要依赖第2级，因此在涉及古怪线段、新笔定义等博文专有内容时，**必须回溯原始博文**而非仅依赖编纂版。

### 已知缺口（已补录 ✓）

以下内容曾缺失于编纂版，现已通过博文收录补齐（`docs/chanlun/text/blog/`）：

- ✅ 第67课：线段划分标准（特征序列法完整定义）
- ✅ 第71课：线段划分标准的再分辨（特征序列包含关系严格性）
- ✅ 第77课：一些概念的再分辨
- ✅ 第78课：继续说线段的划分（古怪线段 + "顶高于底"硬约束）
- ✅ 《忽闻台风可休市》：新笔定义（包含在第81课页面中）

> 相关谱系：`.chanlun/genealogy/pending/001-degenerate-segment.md`、`.chanlun/genealogy/pending/002-source-incompleteness.md`

## 检索/引用原则（避免 prompt is too long）

- **不要**整本引用 `docs/chanlun/mineru/**/full.md` 或 PDF；优先只读取拆分后的小文件（`docs/chanlun/text/**`）。
- 当用户提问涉及定义/定理/流程时：先在仓库内检索关键词，只打开最相关的 1-3 个小文件片段再回答。
- 若可用检索工具：优先用它在 `docs/chanlun/` 下搜索并按需读取，尽量不要让用户手工提供具体路径。
- **权威性**：遵循上述三级权威链。编纂版与原始博文有出入时，以博文为准。

## 级别口径（本项目关键约束）

- **级别 = 递归层级（level_id）**：从 1分钟K线为底座无限向上递归构造走势类型/中枢。
- **禁止**用"日线/30分/5分"等时间周期替代级别定义（多周期下钻最多只能作为展示/验算手段）。

## 概念溯源标签

本仓库中的概念按来源分为两类，所有新增定义/原则必须标注：

- **[旧缠论]**：缠师原始博文中直接阐述的概念（最终权威 = 原文）
- **[旧缠论:隐含]**：原文使用但未独立命名的概念（如 GG/DD 波动区间）
- **[旧缠论:选择]**：原文有多种解读，编排者选择了其中一种
- **[新缠论]**：编排者在旧缠论基础上提出的扩展或新增原则

> 溯源框架详见 `.chanlun/genealogy/pending/004-provenance-framework.md`

## 元编排规则

本项目使用元编排 v2 方法论（ECC 工程底座 + 概念层守卫）。

### 核心原则
1. **概念优先于代码。** 定义不清楚时不写代码。
2. **不绕过矛盾。** 见 `~/.claude/rules/no-workaround.md`。
3. **所有产出必须可质询。** 见 `~/.claude/rules/result-package.md`。
4. **谱系必须维护。** 每次矛盾处理后写入 `.chanlun/genealogy/`。
5. **定义变更必须通过仪式。** 使用 `/ritual`，不直接编辑定义文件。
6. **ceremony 是持续执行授权，不是一次性启动。** ceremony 建立目标后自主推进所有可推进工作，commit/push 不是断点，不要停下来等编排者确认下一步。只在概念矛盾（`/escalate`）、缺外部数据、或不可逆定义变更时才停。
7. **对象否定对象。** [新缠论] 体系中一个对象被否定的唯一来源是内在否定或外部对象生成。不允许超时、阈值、或非对象来源的否定。见 `.chanlun/genealogy/pending/005-object-negation-principle.md`。
8. **热启动保障蜂群持续自动化。** 当上下文压缩（compact）发生时，通过 session 记录和 ceremony 恢复蜂群状态，确保自动化不因上下文截断而中断。蜂群是持续运行的，不是单次启动。
9. **蜂群是默认工作模式，不是可选优化。** 每个工作节点必须先评估可并行的独立工位数（≥2 即拉蜂群）。蜂群在整个会话中持续运作：完成一轮并行后，汇总结果，再评估下一轮可并行工位，循环至任务完成。单线程顺序执行只在任务间有严格依赖时才允许。

### 知识仓库映射
元编排中的 `knowledge/` 在本项目中对应：
- 速查定义：`缠论知识库.md`
- 域对象 schema：`definitions.yaml`
- 规则规范：`docs/spec/`（segment_rules_v1.md, zhongshu_rules_v1.md, move_rules_v1.md 等）
- 原文参考：`docs/chanlun/text/chan99/`（编纂版，已知不完整；完整原文见三级权威链）

### 谱系目录
- `.chanlun/genealogy/pending/` — 生成态矛盾
- `.chanlun/genealogy/settled/` — 已结算记录

### 可用命令
- `/ceremony` — 创世仪式（会话开始时执行）
- `/inquire` — 四步质询序列
- `/escalate` — 矛盾上浮
- `/ritual` — 定义广播仪式
- `/plan` — 实现规划（ECC，受元编排约束）
- `/tdd` — 测试驱动开发（ECC，受元编排约束）
- `/code-review` — 代码审查（ECC，受元编排约束）

### 热启动机制（Warm Start）

蜂群的持续性通过三级恢复保障：

| 级别 | 触发条件 | 恢复方式 | 状态来源 |
|------|----------|----------|----------|
| L0 正常 | 上下文充足 | 蜂群持续运行 | 内存（当前对话） |
| L1 compact | 上下文压缩 | PreCompact 保存 + SessionStart 恢复 | session 文件 |
| L2 新对话 | 上下文彻底耗尽 | 新对话 + ceremony 从 session 文件恢复 | session 文件 + `.chanlun/` |

**L1 — 同会话 compact 恢复**（已实现）：
1. PreCompact 钩子（`.claude/hooks/precompact-save.sh`）：压缩前自动写入蜂群状态到 `*-precompact.md`
2. 压缩后加载最近 session 记录，恢复定义基底、工作进度、待决事项
3. 直接续接中断点，不需要重新 ceremony

**L2 — 跨会话热启动**（已实现）：
1. 当 compact 后上下文依然不足（或会话被关闭/超时），启动新对话
2. 新对话执行 `/ceremony`，ceremony 自动检测 session 记录，切换为**热启动模式**：
   - 版本对比：扫描 `.chanlun/definitions/` 当前版本 vs session 记录版本
   - 差异报告：只报告变更项（`↑`升级/`+`新增/`-`消失），未变更项标记 `=`
   - 跳过已结算且版本未变更的定义的重新验证
   - 直接加载中断点和阻塞项
   - **不等待编排者确认**，直接进入蜂群循环
3. session 文件是跨对话的唯一状态载体，必须在每次 commit 时同步更新

**设计目标**：蜂群永远在线。compact 和新对话都不是中断，而是状态快照+恢复的不同级别。
