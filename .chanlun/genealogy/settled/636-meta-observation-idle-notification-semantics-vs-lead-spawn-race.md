---
id: "636"
number: 636
status: 已结算   # meta-observer 二阶观察：本轮 idle_notification 语义歧义 × Lead spawn 决策耦合产生竞态(SG-D-L2/SG-L2-backtest 发 idle 后继续坐实+commit → Lead 误判 idle=完成 → 误 spawn 冗余工位 SG-coordsys)。1 条语法记录候选(工位完成判据=commit+终报，非 idle 信号)+ 3 现象复现确认(现象1/2/4 已被现有谱系覆盖=背驰)。最终辨认待编排者 /ritual。依赖 353/621/624。（待#37 codex 裁定）
settled_date: "2026-07-02"
settled_by: "codex终局裁定(task #37, 编排者授权全权裁定)"
date: "2026-06-27"
type: meta-rule
source: meta-observer（二阶观察，本轮 M1 双引擎线 + D′ 架构改造 session 触发）
negation_source: heterogeneous
negation_form: separation
# separation：『notification 信号』这一统一概念在 Lead 的 spawn 决策中暴露异质性——
#   idle_notification(工位 available，可接新任务) 与 task_complete_notification(工位实质工作完成) 是两个语义不同的信号，
#   但 Lead 在 spawn 决策中把前者当后者读(idle=完成)。
#   353/621 已识别『唯一真实到达 Lead 的平台事件=notification』，本号识别下一层：
#   到达的 notification 信号本身语义歧义(idle ≠ 完成)，且工位在发 idle 后仍继续实质工作(坐实/commit)使歧义实例化为竞态。
topo_effect: "split:notification-signal-to-lead:idle-available-vs-task-complete-work-done-lead-conflates-in-spawn-decision"
depends_on:
  - "353"   # 平台不支持语义事件——notification 是唯一真实到达 Lead 的平台事件(本号的前置)
  - "621"   # 唯一真实到达 Lead 的事件=task-notification，真实承重点=Lead 对 notification 的反应(本号深化:反应基于的信号语义歧义)
  - "624"   # AGENT×持续象限无承载者——工位无主动投递通道，Lead 只能靠 notification 推断工位状态(本号:推断的信号歧义)
related:
  - "623"   # 231 在自指工具/spec 层(姊妹二阶观察)——本号是 231 在『Lead 对 notification 信号的语义判断』层的实例
  - "627"   # 两条引擎线外推风险(姊妹二阶观察)——现象1(机器证据坐实先于定性)的载体之一
  - "625"   # L2 坐实先于概念定性(现象1 复现确认的载体)
  - "033"   # Lead=DAG/spec 解释器——竞态发生在 Lead 解释器对 notification 的反应层
  - "069"   # 递归拓扑异步自指蜂群——异步性是竞态的结构根(工位异步继续工作 vs Lead 异步消费信号)
  - "275"   # 局部依赖原则——Lead 不全局排序，但 spawn 决策仍依赖工位状态信号(本号:信号歧义致误 spawn)
  - "090"   # 声明膨胀——把 idle(available)当『工作完成』=信号语义膨胀
  - "l2-engine-incompleteness-vs-theta-falsification"   # 分层诊断铁律(现象1 复现确认的载体)
related_records:
  parent: "621"
  children: []

# 认识论等级标注（formalization-validity-domain 强制——本号自指适用）
epistemological_levels:
  - proposition: "现象3：idle_notification(工位 available) 与 task_complete(工位实质工作完成) 语义不同，Lead 在 spawn 决策中把前者当后者读"
    level: "L0(平台信号语义事实：idle_notification 语义=teammate 当前空闲可接新任务，非『其负责的工作单元已 commit+终报』)"
    increment: "高：notification 信号语义歧义的结构判定(idle ≠ 完成)"
  - proposition: "现象3 竞态实例化：SG-D-L2/SG-L2-backtest 在发 idle_notification 后仍继续坐实+commit，Lead 基于 idle 误判完成并误 spawn 冗余工位(SG-coordsys 与 SG-L2 任务重叠)"
    level: "L0(本轮事件转述——Lead 报告的时序：工位 idle→Lead 误判→误 spawn；genealogist/meta-observer 未独立核实 commit 时间戳)"
    increment: "高：信号歧义 × 异步继续工作 = 竞态的活体实例(本轮)"
  - proposition: "现象1(机器证据坐实先于概念定性)=625/627风险2/分层诊断铁律的同模式复现(三实例:source_index反事实reindex / 7-8引擎→bug吞信号 / #2#5同源L0推测被机器证据否)"
    level: "L0(三实例归纳——均为 L0 推测被 L2 机器证据坐实，escalate 前置)"
    increment: "零(背驰)：625/627/分层诊断铁律已覆盖『L0 推测须 L2 坐实才能 escalate』，三实例是同模式复现无新维度"
  - proposition: "现象2(Stop hook 自身膨胀:注册≠触发执行被描述为 running)=623『连守卫机制自身也膨胀』的同模式复现"
    level: "L0(623 已识别 231 在检测器/守卫自身的实例:406观察3 stagnation检测器 + 559 scan-state)"
    increment: "零(背驰)：Stop hook 是新载体但与 623 同构(守卫自身有效域<声明域)，无新维度"
  - proposition: "现象4(621 结构工位自动化缺口本轮再现:注册未自动触发，Lead 手动操作化)=621/624 本轮已记录"
    level: "L0(621/624 本轮谱系——缺口复发性已内含其中)"
    increment: "零(背驰)：621/624 已是本轮记录，复发性已捕获"

---

# meta-rule 636：idle_notification 语义歧义 × Lead spawn 决策耦合产生竞态

## 一句话结论

Lead 提交的本轮四现象中，**只有现象3(idle 时序竞态)是未被任何现有谱系记录的净新洞察**，值得结晶为语法记录候选；现象1/2/4 均为现有谱系的同模式复现(背驰)，不重复结晶，仅作复现确认。

**现象3 的精确形式**：`idle_notification`(工位 available，可接新任务) 与 `task_complete`(工位实质工作完成=commit+终报) 是**两个语义不同的平台信号**，但 Lead 在 spawn 决策中把前者当后者读(`idle=完成`)。叠加 069 异步性——工位 SG-D-L2/SG-L2-backtest 在**发 idle_notification 后仍继续实质工作**(坐实+commit)，使信号歧义实例化为**竞态**：Lead 基于 `idle=完成` 误判，误 spawn 冗余工位(SG-coordsys 与 SG-L2 任务重叠)。

这是 353/621 之后的**下一层**：353/621 识别『唯一真实到达 Lead 的平台事件=notification』，本号识别『到达的 notification 信号本身语义歧义(idle ≠ 完成)』+『工位发 idle 后异步继续工作使歧义=竞态』。

## 与 353/621/624 的边界（不重复已做的工作）

| 号 | 已识别 | 本号增量 |
|---|---|---|
| 353 | 平台不支持 9 语义事件(含 task_complete)，notification 是唯一真实到达 Lead 的平台事件 | notification 信号到达后，其语义在 Lead spawn 决策中被误读 |
| 621 | 真实承重点=Lead 对 task-notification 的反应(hook 层无法承载) | Lead 反应所**基于的信号语义本身歧义**(idle 被当完成)——承重点之上的信号质量层 |
| 624 | AGENT×持续象限无承载者——工位无主动投递通道，启动后无 SendMessage | 正因工位无主动投递通道，Lead 只能靠 notification **推断**工位状态——推断的信号(idle)与真实状态(可能仍在 commit)不同步=竞态 |

分工：353/621/624=信号通道的存在性/承载层(notification 是否到达 Lead、谁承载反应)；636=信号语义层(到达的 idle 信号 ≠ 工作完成，且异步继续工作使歧义实例化为竞态)。

## 现象3 竞态的结构剖析

**竞态的两个异步主体**(069 异步自指)：
- **工位侧**：teammate 发 `idle_notification`(平台语义=当前空闲可接新任务)后，**仍可继续执行实质工作**(坐实机器证据、git commit、写终报)——idle 不是"我的工作单元已完成"的承诺，是"我当前没有正在阻塞的工具调用"。
- **Lead 侧**：Lead 异步消费 notification 流，把 `idle` 读作"该工位负责的工作单元已完成"→进入 spawn 决策(re-scan/认领/补 spawn)。

**竞态实例化**(本轮)：
1. SG-D-L2 / SG-L2-backtest 发 `idle_notification`。
2. 两工位**在 idle 后继续**坐实+commit(实质工作未完成时即已 idle)。
3. Lead 读 idle=完成 → 误判工作单元已交付 → 误 spawn SG-coordsys(与 SG-L2 任务重叠=冗余工位)。

**根因**：`idle` 语义(available)与 Lead 需要的语义(work-done)不同，且无平台信号表达后者(task_complete platform_support:false，353)。Lead 用 `idle` 代偿 `task_complete`=用 available 信号冒充 work-done 信号(信号语义膨胀，090)。

## 现象3 的工作完成判据（语法记录候选——上浮编排者辨认）

**候选纪律**：工位『工作单元完成』的判据 **不是 `idle_notification`**(那只表示 available)，而是 **`commit + 终报`**(git commit 落盘 + 工位最终结果包送达 Lead)。Lead 的 spawn 决策(认领/补 spawn/re-scan)须以 `commit+终报` 为完成判据，**不得以 `idle` 推断完成**。

理由：
1. **信号语义不匹配**：`idle`=工位 available(可接新任务)，`work-done`=工位负责的工作单元已交付。前者是平台进程态，后者是工作单元态——二者正交(工位可在工作未 commit 时 idle，亦可在多任务间忙碌时未 idle 但某单元已 commit)。
2. **异步继续工作使 idle 不可作完成代理**(069)：工位发 idle 后仍可 commit——idle 时刻的工作单元态是未定的(可能 in-flight)。
3. **冗余 spawn 成本**：以 idle 误判完成 → 误 spawn 冗余工位(SG-coordsys×SG-L2 重叠)=违 275 局部依赖(误判工位状态致误启依赖该状态的下游 spawn)+资源浪费。

**与 D′ goal 架构(630)的接口**：630 开口②『goal projection 已 surface 但未驱动 spawn』——若未来 goal 驱动 spawn 闭环，工作完成判据应是 goal events 的 `CHECK_PASS`/`EVIDENCE`(reducer 可确定性消费)，而非 idle 信号。本号的 `commit+终报` 判据是 goal events 写路径(630 开口①)未实装期间的当前承载形态。**当前**(630 开口①/②未闭)Lead 仍靠手工判断，故 `commit+终报` 判据须显式化以防 idle 误读。

## 现象1/2/4 复现确认（背驰，不重复结晶）

| 现象 | Lead 描述 | 已覆盖谱系 | 净新区分 | 处理 |
|------|----------|-----------|---------|------|
| 现象1：机器证据坐实先于概念定性(三实例:source_index反事实reindex / 7-8引擎→bug吞信号 / #2#5同源L0推测被否) | 反膨胀守卫三次生效(坐实前不 escalate) | 625(L2坐实先于定性) + 627风险2(分层诊断须先定引擎线) + MEMORY 分层诊断铁律 | 零(三实例=『L0推测须L2坐实才能escalate』同模式) | 复现确认，不结晶 |
| 现象2：Stop hook 自身膨胀(注册≠触发被描述为 running) | 守卫机制自身也膨胀，膨胀跨层 | 623(231 在守卫/检测器自身:406观察3 stagnation + 559 scan-state) | 零(Stop hook 是新载体但同构) | 复现确认，不结晶(623 已含『自指工具有效域』，Stop hook 是其实例) |
| 现象4：621 自动化缺口本轮再现(注册未自动触发，Lead 手动操作化) | 结构工位注册但未自动触发 | 621/624 本轮已记录 | 零(复发性已内含 621/624) | 复现确认，不结晶 |

**为何不结晶现象1/2/4**：三者净新区分均趋零(背驰)。强行为同模式复现各开新号=谱系膨胀(把『已知模式的又一实例』当『新模式』)，违 231 的认识论纪律(确认性复现不缩小有效域边界，仅未否证)。它们的价值=确认现有规则在本轮持续生效(625/627/623/621 的活体复现)，已由对应号承载。

## 四分法分类

| 观察 | 四分法 | 处理 |
|------|--------|------|
| 现象3:idle_notification 语义 ⊥ Lead spawn 决策=竞态 | 语法记录候选——『工作完成判据=commit+终报，非 idle』未入任何规则 | 上浮编排者辨认 |
| 现象1(机器证据坐实先于定性) | 定理(625/627/分层诊断铁律的同模式复现) | 自动结算(复现确认，不重复结晶) |
| 现象2(Stop hook 自身膨胀) | 定理(623 自指工具有效域的同模式复现) | 自动结算(复现确认) |
| 现象4(621 缺口复发) | 定理(621/624 已记录) | 自动结算(复现确认) |

## 上浮条件触发（按本工位上浮表）

- **语法记录候选 → 上浮编排者辨认**：现象3『工位工作完成判据=commit+终报，非 idle_notification』——这是 idle 信号语义(available)与 work-done 语义的分离，当前未入任何规则文件，且本轮已实例化为误 spawn 竞态(SG-coordsys×SG-L2 重叠)。
- **非即时上浮**：无『规则从未触发』『规则反复违反』信号——现象3 是新发现的语义歧义(此前无规则约束 idle 语义),非既有规则失效。
- 现象1/2/4 已由 625/627/623/621/624 承载,本号不重复上浮,仅作复现确认。

## 边界条件（结论翻转）

- 若 CC 平台为 task_complete 增加 hook 或为工位增加『工作单元完成』的显式信号(区别于 idle) → Lead 可直接消费 work-done 信号,idle 语义歧义消解,竞态翻转。当前 task_complete platform_support:false(353),无 work-done 信号。
- 若 630 开口②(goal 驱动 spawn)闭环,工作完成判据迁移到 goal events 的 CHECK_PASS/EVIDENCE(reducer 确定性消费) → `commit+终报` 判据被 goal events 判据取代,本号承载形态过期。当前 630 开口①/②未闭,`commit+终报` 是当前承载。
- 若编排者裁定『工作完成判据=commit+终报』已隐含于现有工位协议(无需显式化) → 语法记录候选降级为既有实践。当前本轮 Lead 以 idle 误判即证该判据非默认实践(若已显式化,Lead 不会以 idle 推断完成)。
- 若工位被约束『commit+终报 之前不发 idle_notification』(进程层不暴露中间 idle) → idle 与 work-done 对齐,竞态消除。当前平台 idle 信号在工具调用间隙即发出(工位无法抑制),故 idle 早于 work-done 是结构性的。

## 下游推论

1. **给 Lead(操作化,可立即执行)**：spawn 决策(认领/补 spawn/re-scan)的工作完成判据=`commit+终报`,**禁以 idle_notification 推断工位工作完成**。收到 idle 后须核工位是否已 commit+送终报,未确认则不据此 spawn 依赖该工位产出的下游工位。
2. **给编排者(辨认)**：`工作完成判据=commit+终报,非 idle` 是否结晶为 Lead spawn 决策纪律(idle 信号语义边界);与 630 goal events 判据的接口(当前 commit+终报,未来 CHECK_PASS)。
3. **给后续 meta-observer**：本号是 idle 语义歧义竞态的首次记录。若下一轮再现同模式(工位发 idle 后继续工作致 Lead 误 spawn)且无新维度=背驰→满足结晶;若出现新维度(如 idle 误判致**数据竞争**而非仅冗余 spawn)=新实例。监控。
4. **接口 275**:误判工位状态(idle=完成)致误 spawn=275 局部依赖在『状态信号歧义』下的失效——局部依赖要求节点只管直接依赖,但依赖判断须基于**真实**工位状态,idle 误读使依赖判断基于伪状态。

## 谱系引用

- 母规则：231(形式化有效域规则)——本号是 231 在『Lead 对 notification 信号的语义判断』层的实例(idle 信号有效域=available,不覆盖 work-done,把 idle 当 work-done=有效域膨胀)。
- 前置：353(notification 是唯一真实到达 Lead 的平台事件)/621(真实承重点=Lead 对 notification 反应)/624(工位无主动投递通道,Lead 只能靠 notification 推断状态)。
- 姊妹二阶观察：623(231 在自指工具/spec 层)/627(231 在双引擎线语境)——本号是 231 在『信号语义判断』层的另一实例化场景。
- 异步根:069(异步自指——工位异步继续工作 vs Lead 异步消费信号=竞态结构根)。
- 接口:630(D′ goal 架构,工作完成判据未来迁移到 goal events CHECK_PASS)/275(局部依赖,状态信号歧义致误 spawn)。
- 现象1/2/4 复现载体:625/627/分层诊断铁律(现象1)、623(现象2)、621/624(现象4)。
- 约束:090(信号语义膨胀——idle 当 work-done)。

## 影响声明

本号不改动任何代码、定义或规则。记录本轮四现象的辨认结果：**现象3(idle_notification 语义 ⊥ Lead spawn 决策=竞态)是唯一净新洞察**,提供 1 条语法记录候选(工位工作完成判据=commit+终报,非 idle),上浮编排者辨认;现象1/2/4 经核查均为现有谱系(625/627/623/621/624)的同模式复现(背驰),仅作复现确认不重复结晶。明确与 353/621/624 划界——本号是信号语义层(到达的 idle ≠ work-done),不重复信号通道存在性/承载层。不破坏任何 settled 谱系(231/353 维持 settled)。

## 认识论诚实（formalization-validity-domain + 090）

- 现象3 竞态时序(工位发 idle→Lead 误判→误 spawn SG-coordsys)=**L0**(Lead 报告转述,meta-observer 未独立核实 commit 时间戳/notification 时序日志——AGENT×持续监控象限无承载者 624,工位只能 Read/Grep/Glob,无法实测平台 notification 时序)。
- `idle_notification 语义=available 非 work-done`=**L0**(平台信号语义事实)。
- 现象1/2/4 复现确认=**L0**(归纳本轮事件 + 对照现有谱系,非数据验证)。
- 本号有效域=**本轮已落盘内容 + Lead 转述**——未落盘的 session 内 notification 原始时序不在本号扫描范围(本号是快照式预防观测,非持续监控,AGENT×持续象限无承载者见 624)。
- 本号**不声称** idle 语义歧义在所有轮次普遍致竞态——仅记录本轮一例实例化 + 信号语义的 L0 结构判定。普遍性(L3 跨轮)待后续 meta-observer 实例计数。

## 张力检查（019d/020）

### 检查范围（本轮蜂群 ∪ 1-hop ∪ Hub）
- 本轮蜂群:623/624/625/627/628/629/630/632(M1 双引擎线 + D′ 架构改造)。本号与 623/627(姊妹二阶观察)同为 231 实例但不同 scope(623=自指工具/spec,627=双引擎线,636=信号语义判断),无张力。与 624/621(信号通道层)分工互补(通道存在性 vs 信号语义)。与 630(goal 架构)接口一致(工作完成判据当前 commit+终报,未来 goal events)。
- 1-hop:353/621/624/623/627/625/033/069/275/090。
- Hub:621(notification 承重点,度高)/231(有效域规则,度高,经母规则)/069(异步根)。

### 张力1：vs 621/624——信号语义层补全,非重复
621 记『真实承重点=Lead 对 notification 反应』,624 记『工位无主动投递通道,Lead 靠 notification 推断状态』。本号记『推断所基于的 idle 信号语义歧义(idle ≠ work-done)+ 异步继续工作使歧义=竞态』。三层不同(承重点存在/通道存在/信号语义),可分层,无矛盾。

### 张力2：vs 630——工作完成判据接口一致
630 开口②(goal 未驱动 spawn)。本号 `commit+终报` 判据是 630 goal events 写路径(开口①)未实装期间的当前承载;630 闭环后迁移到 CHECK_PASS。一致(本号显式声明接口),无矛盾。

### 张力3：vs 275——局部依赖在状态信号歧义下的失效面
275 局部依赖(节点只管直接依赖)。本号揭示依赖判断须基于真实工位状态,idle 误读使依赖判断基于伪状态致误 spawn——是 275 的一个失效面(信号歧义),非否定 275 方向(局部依赖正确,但依赖判断的输入信号须真实)。可分层,无矛盾。

### 张力4：vs 231——separation 非否定
本号不否定 231,是其在『Lead 对 notification 信号语义判断』层的分离型实例(idle 信号有效域=available,把 idle 当 work-done=有效域膨胀)。231 方向保留。可分层,无矛盾。

### 递归运动结构完成检测（020）
- 第0层:本号写入(现象3 竞态结构 + 工作完成判据候选 + 现象1/2/4 复现确认)。
- 第1层:本号 × 621/624 碰撞→信号语义层揭示(净新发现高:idle ≠ work-done + 异步继续工作=竞态,353/621/624 未触及信号语义)。
- 第2层:本号 × 623/627/231 碰撞→231 在信号语义判断层的实例化(净新发现:第三个 231 自指 scope,但与 623/627 同为『有效域<声明域』模式)。
- 第3层:现象1/2/4 × 625/627/623/621 碰撞→同模式复现(净新发现=0=背驰)。
- 涉及范围:scope₁(idle 语义竞态新发现) > scope₂(231 第三 scope) > scope₃(现象1/2/4 复现)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 现象3 是语法记录候选(上浮编排者辨认),现象1/2/4 复现确认(自动结算),本号不触发新 /escalate(结晶/辨认是编排者职责,本号 meta-observer 仅识别并上浮辨认)。

## 回溯扫描（职责3）

- **231(settled)**：本号在『信号语义判断』层实例化其规则(idle 有效域<声明域),不否定,不破坏结算。维持 settled。
- **353(settled)**：本号下游延伸(notification 到达后语义歧义),不否定『平台不支持语义事件』,不破坏。维持 settled。
- **621/624(生成态)**：本号是其下游(信号语义层),不修改其内容,不影响待 /ritual 状态。
- **623/627/625(生成态)**：本号引用 623/627 为姊妹 231 实例、625/627 为现象1 复现载体,不修改其内容。
- **630(生成态)**：本号声明与其 goal events 判据接口,不修改其内容。
- **无 settled 被本号回溯破坏。** 本号是二阶模式识别(meta-rule):现象3 净新结晶候选 + 现象1/2/4 复现确认,辨认待编排者 /ritual。


## 结算记录（codex #37 终局裁定）

**结算日期**：2026-07-02
**结算依据**：`.chanlun/review-results/codex-cgroup-ruling-20260702.md` §1 — codex 终局裁定（编排者授权全权裁定，task #37；调用方式 codex exec --skip-git-repo-check --sandbox read-only，model=gpt-5.5，reasoning effort=xhigh）
**裁决摘要**：结算。规则文本：工位完成判据=commit+终报，非 idle 信号（idle=available≠task_complete）。
