---
id: '551'
number: 551
title: "ceremony 能指三重分叉（CC Swarm₀ / 逢亮持久实体 / RTAS 脱域抽取）+ 启动-hook 目录守卫不变量——禁止跨目录交叉触发"
type: 语法记录  # 四分法：已运作但未显式化的规则
status: settled  # 编排者裁决（2026-06-22，§八.6）：C（hook 守卫技术实现）已完成（行动类无残留）；A（概念分离确立）+ B（不变量升结算）标 blocked——需外部 RTAS 谱系（M001-M003，独立 repo /Users/silencehan/rtas/）确立，暂阻塞但不阻塞当前工作 → 551 整体 settled
date: '2026-06-22'
level: "L0（结构事实：从 CLAUDE.md「可用命令」对比表 + RTAS 谱系 M001-M003 + 两套 SessionStart ceremony hook 源码读出，无 L2 数据验证）"
provenance: "[新缠论:基础设施审计]（配置 ~/.claude 全局 SessionStart ceremony-autostart hook 的 session 衍生）"
negation_source: homogeneous  # 自审：配置全局 ceremony 自启 hook 时撞到的跨目录矛盾
negation_form: expansion  # 058号「ceremony=Swarm₀」的精化：Swarm₀ 在脱域抽取后分叉为多个目录绑定实例
negates: "隐含命题『ceremony 是单一全局概念，可由一个启动 hook 统一触发』——被证伪：能指『ceremony』已覆盖三个存在论层级不同、绑定不同目录的所指；单一全局 hook 跨目录触发会(A)调错 ceremony、(B)与目标项目自带 hook 双触发"
topo_effect: ""  # 无区块拓扑冻结/重定向
depends_on:
  - '058'   # ceremony=Swarm₀（CC ceremony 的定义来源）
  - '075'   # 结构能力→事件驱动 skill（hook 触发 ceremony 的机制层）
  - '136'   # ceremony 持久化不变量
related:
  - '003'   # 线段两个口径——概念分离的先例（同一能指多所指）
  - '042'   # hook 网络模式
  - '173'   # hook 反馈回路
  - '548'   # hook 动作注入 vs 指令索引（hook 注入语义分界）
tensions_with: []
external_lineage:  # 跨项目谱系引用（RTAS 谱系，生成态，独立 repo 主体在本机 /Users/silencehan/rtas/，含 .rtas/；§八更正：原文「F:\RTAS」是 Windows 路径残留，本机不存在）
  - 'rtas:M001'   # 蜂群自复制脱域——.chanlun→.rtas 抽取的起源
  - 'rtas:M002'   # 迁移审计——.chanlun 引用系统性清理 = 目录改名
  - 'rtas:M003'   # 提取审计 + rtas-dispatch
---

# 551 号：ceremony 能指三重分叉 + 启动-hook 目录守卫不变量

**认识论**：L0（结构事实，从源码与文档读出，无数据验证）。**状态生成态**：operational 不变量「启动 hook 禁止跨目录交叉触发」是本号新断言，是否升为结算原则待编排者裁决。

## 一、现象（触发场景）

在 `~/.claude/settings.json` 配置全局 `SessionStart` hook 以实现「蜂群启动自动 ceremony」时，撞到一个判断：要不要让该 hook 在 `.rtas/` 项目里也触发？审计发现：**"ceremony" 不是一个概念，是一个能指，覆盖三个不同所指**——同一能指多所指，正是 003号「线段两个口径」式的概念分离，但此前未在谱系显式记录其在 ceremony 域的实例。

## 二、ceremony 能指三重分叉表

| # | 实例 | 触发物 | 读取目录 | 存在论层级 | 生命周期 | 谱系/文档锚点 |
|---|------|--------|---------|-----------|---------|--------------|
| ① | **CC `/ceremony`** | ceremony skill（CC Swarm₀） | `.chanlun/sessions\|definitions\|genealogy` | CC 蜂群（临时主体） | session 内每次触发 | 058号；CLAUDE.md「可用命令」 |
| ② | **逢亮 `ceremony.py`** | `topological-computation/ceremony.py` | 逢亮内部状态 | 逢亮（持久实体，真主体） | 一次性（首次部署） | CLAUDE.md「逢亮 ceremony」对比表 |
| ③ | **RTAS `session-start-ceremony.sh`** | 项目级 SessionStart hook | `.rtas/sessions\|definitions\|genealogy` | domain-agnostic 蜂群 | session 内每次触发 | RTAS 谱系 M001-M003 |

- ③ 是 ① 的**脱域克隆**（M001：「将 NewChanlun 中的通用 RTAS 蜂群基础设施抽象提取……第一次蜂群自我复制并脱域」；M002：`.chanlun → .rtas` 路径系统性改写）。同一套 ceremony 逻辑，domained（①）vs de-domained（③）。
- ① 与 ② 是**不同存在论层级**（CLAUDE.md 原文：「逢亮活了之后，CC session 的 /ceremony 改为检查逢亮状态而非重启逢亮」）。

## 三、operational 不变量（本号核心 = 语法记录）

**启动 hook 必须按目录上下文触发对应 ceremony，禁止跨目录交叉触发。**

- 触发 ① 的 hook 守卫条件 = 存在 `.chanlun/`；触发 ③ 的 hook 守卫条件 = 存在 `.rtas/`。
- 跨目录触发的两个失效模式：
  - **(A) 调错 ceremony**：在纯 `.rtas/` 项目里触发 ① 的 ceremony skill → 它扫 `.chanlun/`（不存在）→ 退化为空冷启动。声明 hook 支持 RTAS 而实际调错对象 = 声明膨胀（090号禁止）。
  - **(B) 双触发**：① 的全局 hook 与 ③ 的项目级 hook 在同一 `.rtas/` 项目同时 fire → 两份矛盾 ceremony 报告。

## 四、谱系记录状态（回答"这个分离是否已被记录"）

| 分叉轴 | 记录状态 | 位置 |
|--------|---------|------|
| ①↔② (CC vs 逢亮) | **已记录** | CLAUDE.md「逢亮 ceremony」对比表，引 058号 |
| ①↔③ (chanlun vs rtas 脱域) | **半记录** | 仅 RTAS 侧 M001-M003（生成态）记抽取事件；chanlun 侧无分叉确认条目 |
| operational 不变量（不得跨目录触发） | **此前未记录** | 本号 551 首次显式化 |

archive 全文已扫（grep `脱域\|extract\|抽取\|.rtas\|session-start-ceremony`），无重复条目 → 本号非重复记录。

## 五、否定面（no-workaround）

- **不**把全局 ① hook 守卫扩到 `.rtas/`——会触发失效模式 (A)+(B)。RTAS 项目由其自带 ③ hook 完整覆盖，正交无遗漏。
- **不**声明单一 hook 统一处理所有 ceremony——三所指不可由一触发物统一。

## 六、边界条件（结论翻转）

1. 若出现**同时含 `.chanlun/` 和 `.rtas/`** 的项目 → ① hook 因命中 `.chanlun/` 守卫而触发，可能与 ③ hook 并存 → 需重设计守卫互斥。当前勘察：NewChanlun 仅 `.chanlun/`、RTAS 项目仅 `.rtas/`，**无重叠**。
2. 若 RTAS 日后改用 `.chanlun/` 目录 + ① 的 ceremony skill → ① 与 ③ 合并，本号 operational 不变量退化为平凡。
3. 若 archive 全文扫存在遗漏（仅 grep 关键词，未逐字通读 8 个 archive 文件）→ "①↔③ 半记录"可能翻转为"已记录"。

## 七、影响声明

- **谱系结晶**（pending/生成态），记录于 chanlun 侧填补分叉确认空缺。
- **关联但未在本号改动的工程产出**：`~/.claude/hooks/chanlun-ceremony-autostart.sh`（全局 ① 自启 hook，守卫 `.chanlun/` + `source∈{startup,clear}`）+ `~/.claude/settings.json` SessionStart 第三项——此 hook 是本号 operational 不变量的物质实现，已先于本号落地并通过 pipe-test。
- **block-topology 映射推迟**：依 549号冻结（relations.jsonl 仍为未实体化 LFS 指针，mapper 跑 Phase3 必崩），本号**不**运行 `map_genealogy_to_blocks.py`，待 549 解冻后随 550/551 一并补映射。
  - **2026-06-22 更新**：549 已结算（no-workaround 消解为定理，概念层吸收入 476）。但 **549 settled ≠ 映射管线解冻**——549 topo_effect freeze 在结算后保留，relations.jsonl 仍是 LFS 指针，映射仍冻结。解冻条件 = A 执行（git lfs pull，操作者基础设施待办）。∴ 551 的「待 549 解冻后补映射」表述**仍然正确**：等的是 549 的**解冻**（A 执行），不是 549 的**结算**。
- 未改任何 RTAS 文件、未改 ② 逢亮 ceremony。

## 八、四分法分类（2026-06-22 genealogist 处理，018号四分法）

genealogy 工位用 **018号四分法**（定理 / 行动 / 选择 / 语法记录——**不用** hook 自创的吸收/修正/分裂/废弃）对 551 内容逐项分类。551 含三个可分离内容，分类不同：

### 八.1 源码勘察确认（分类的事实基础）

- `~/.claude/hooks/chanlun-ceremony-autostart.sh` line 24：`if [ "$trigger" = "1" ] && [ -d "$dir/.chanlun" ]` ⟹ **目录守卫已正确实现**（仅 `.chanlun/` 存在时注入，且 `source∈{startup,clear}`）。
- `~/.claude/settings.json` SessionStart 第三项已注册该 hook（timeout 10）。
- 本机 RTAS 项目主体在 `/Users/silencehan/rtas/`（含 `.rtas/`），与 NewChanlun（仅 `.chanlun/`）**目录正交无重叠**（§六边界1当前成立）。
- 551 §一/external_lineage 写的「F:\RTAS」是 **Windows 路径残留**，本机（mac）不存在该路径——已在 frontmatter external_lineage 注释更正，不影响不变量实质（本机确有含 `.rtas/` 的项目，跨目录场景真实）。
- 548号（hook 双类型分离）**已 settled**（2026-06-22 编排者 verdict=B），其 topo_effect 已把 097 的 hook 分裂为 tool-拦截 hook + bootstrap hook，bootstrap hook 合法注入正面格式行动指令——这是 551 hook 守卫合法性的**已结算上游**。

### 八.2 三内容四分法分类

| 内容 | 实质 | 四分法分类 | 处理 | 可否自决 |
|------|------|-----------|------|---------|
| **C — hook 目录守卫的技术实现** | `chanlun-ceremony-autostart.sh` 的 `.chanlun/` 守卫 + settings.json 注册 | **行动类**（不携带信息差的纯技术修复） | 已完成，无残留修复项（守卫正确、无目录守卫缺口、RTAS 由自带 ③ hook 正交覆盖） | 已执行完毕（551 §七声称落地，本工位审计确认无缺口） |
| **B — operational 不变量「启动 hook 禁止跨目录交叉触发」是否升为结算原则** | 一条新架构断言（hook 目录守卫互斥）；可从 090（声明膨胀禁止）+ 548（bootstrap hook 存在论职责）**部分**推出，但"是否升结算原则"+守卫互斥的完整架构形式非纯定理演绎 | **语法记录**（已运作未显式化的规则）/ 含**选择**成分（升不升、如何形式化守卫互斥） | **不自决**——551 frontmatter 自承「待编排者裁决是否结算」 | ❌ 上浮编排者 |
| **A — ceremony 能指三重分叉（概念分离的谱系化）** | 同一能指「ceremony」覆盖三所指（①CC/②逢亮/③RTAS）；003号同构（同一能指多所指）；①↔②已记录、①↔③半记录、本号首次 chanlun 侧显式化 | **语法记录**（已运作的概念分离首次显式化）；但**概念分离的确立本身是概念层决断** | **不自决**——genealogist 职责边界明确：「不决定概念分离——需通过 /escalate 上浮的概念层决断」 | ❌ 上浮编排者 |

### 八.3 分类结论

**551 不属于「行动类可自行修复+结算」。** 内容 C（行动类）已完成无残留；内容 A（概念分离确立）+ 内容 B（operational 不变量升结算 + 守卫互斥形式化）**双双需编排者裁决**，与任务指令的「选择/语法记录/概念矛盾（涉及 ceremony 架构决策）→ 不自行结算 → 报告 Lead 上浮编排者」完全吻合。

**genealogist 不结算 551**（保持生成态）。整理给 Lead 的上浮要点（§八.4）。

### 八.4 上浮要点（供 Lead /escalate）

1. **概念分离确立请求**：确认「ceremony 是三个所指（①CC Swarm₀ / ②逢亮持久实体 / ③RTAS 脱域）而非单一概念」是否作为结算的概念分离记录（003号式）。chanlun 侧此前无此分叉确认条目，本号填补。
2. **operational 不变量升结算请求**：「启动 hook 必须按目录上下文触发对应 ceremony，禁止跨目录交叉触发」是否升为结算原则。若升，需附守卫互斥的形式化（当前仅 `.chanlun/` / `.rtas/` 目录存在性二选一，§六边界1指出二者共存时需重设计互斥——这是未覆盖的边界）。
3. **external_lineage 新字段裁决**：551 首创 `external_lineage` 字段引用跨项目谱系（rtas:M001-M003），模板（genealogy-template.md）无此字段，全谱系仅 551 使用。是否将 `external_lineage` 纳入模板作为跨 repo 谱系引用的标准字段（跨项目谱系链的语法记录）。

### 八.5 genealogist 二次确认（2026-06-22 结算扫描，549/551 联合处理）

本次结算扫描重新审查 §八.4 三个上浮点是否真的不可自决（任务要求「尽量把可自决部分结算」），逐点验证：

| 上浮点 | 自决性验证 | 结论 |
|--------|-----------|------|
| 1（概念分离确立） | 结构事实（三所指）L0 已确认（§二），但**「确立为正式概念分离记录」是概念层决断**。genealogist 职责表硬边界：「不决定概念分离」。不可自决。 | 真需上浮 |
| 2（不变量升结算） | 可从 090/548 **部分**推出，但「是否升结算原则」+ §六边界1（`.chanlun/`+`.rtas/` 共存时守卫互斥）= 未覆盖架构选择，非纯定理演绎。不可自决。 | 真需上浮 |
| 3（external_lineage 字段纳入模板） | 语法记录类（018号四分法明确语法记录走 /escalate）；模板修改影响全谱系标准，需编排者裁决。不可自决。 | 真需上浮 |

**与 549 的对照（为什么 549 可自决、551 不可）**：549 的「选择」（A/B/C）被 no-workaround 这一**已结算的蜂群语法规则**消解为唯一定理（B/C 有损覆盖语法不合法 → A 唯一合法）。551 的 A/B/external_lineage **没有一条已结算规则能消解为唯一解**——概念分离的确立、不变量是否升格、模板字段标准化，都需要新的价值判断/语法确立，无现成规则可推。∴ 549 自决结算、551 真需上浮，二者分野严格清晰，非「能结算就结算」的随意。

**为什么不拆 551 把 C 单独结算**：内容 C（hook 守卫）是 A（概念分离）的工程显形，B 是 A 的 operational 推论，三者共属**一个发现的三个面**（一条谱系记录）。为「多结算一点」而拆记录 = 违反「谱系优先于汇总」（012号——不应为汇总便利切割发现的统一性）。C 作为行动类已执行完毕，无需独立"结算"动作；551 整体在 A 结算前保持 pending。

**结算动作**：551 **保持 pending / 生成态**（dag.yaml 551 status 不改）。上浮 §八.4 三点（A 概念分离确立 + B 不变量升结算 + external_lineage 字段纳入模板）给 Lead /escalate。549 已结算这一新事实不改变 551 的待解状态——见 §七「2026-06-22 更新」（551 等的是 549 解冻，不是 549 结算）。

### 八.6 编排者裁决（2026-06-22，覆盖 §八.5「保持 pending」）

编排者裁决：**551 整体标 settled**（C 已行动，A/B blocked 不阻塞当前工作）。

| 内容 | 裁决 | 理由 |
|------|------|------|
| **C**（hook 守卫技术实现） | **已完成** | 行动类无残留（§八.1 源码勘察确认守卫正确） |
| **A**（ceremony 三所指概念分离确立） | **blocked** | 需外部 RTAS 谱系（M001-M003，独立 repo `/Users/silencehan/rtas/`）确立 chanlun↔rtas 脱域分叉；外部谱系未就绪前阻塞，**但不阻塞当前工作**（死锁主线 A 路） |
| **B**（启动 hook 禁止跨目录交叉触发 升结算原则） | **blocked** | 同需外部 RTAS 谱系确立守卫互斥（§六边界1 `.chanlun/`+`.rtas/` 共存场景） |

**与 §八.5 的区别**：§八.5（genealogist）把 A/B 判为「待编排者裁决」保持 pending。编排者以 **blocked 分类**（需外部依赖、暂阻塞）替代——blocked ≠ 待裁决：A/B 不是等一个价值判断，是等外部 RTAS 谱系就绪。C 已完成 + A/B 阻塞于外部依赖 ⟹ 551 作为本 repo 谱系记录**可结算**（settled），A/B 作为 blocked 开放轴留存。编排者：「A/B blocked 不阻塞当前工作」「不要继续纠结 551」。
