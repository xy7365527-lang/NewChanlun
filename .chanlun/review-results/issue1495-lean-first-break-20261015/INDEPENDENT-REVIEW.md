# #1495 候选独立评审

结论：**PASS_SCOPED_SEMANTIC_REVIEW**。限定于本报告绑定的两文件候选，未解决的必须修复项为 **0**。该结论认可定义、三态、首个出窗元素与等价引理，以及引文订正；不代表全 Lean 消费链迁移完成，也不代替主控的最终验收或合入决定。

评审日期：2026-10-15。评审工位：`/root/frontier_resume_1015`，与实现工位 `/root/recover_1495_lean` 分离。本工位只读产品代码，只写本报告；未运行 lake/cargo、未访问外盘、未提交或关票。

## 候选绑定与依据

- 工作树：`/private/tmp/goal1323-native-20261012/wt-1495-recovery`。
- 基线及当前 HEAD：`dedf11f047b9c809c167b35e728e4a2ee8bf3494`；评审对象为该基线以上未提交差异。
- `formal/Origin/SegmentFeatureSeq.lean` SHA-256：`a9978ce332cea564ab324114859a9d7a4baaea55c11bf924dd5e1af6b988ba01`。
- `formal/Phase2/Claim10_SegmentV1.lean` SHA-256：`dec380404b1bc208cec004dd80a5770c64c969b22617236ae00a188d4c8ac71d`。
- 需求依据：#1495（#1323 / SPEC #1340 下游），#1490 裁定及 `.chanlun/definitions/xianduan.md` v2.3 §7.1、§15 X-31。所读原票留存于本目录 `event-29.json`；正本原文采用 `docs/chanlun/text/blog/` 下 067、071、078 课。
- 纪律依据：AGENTS.md 的权威链、#793 正本边界、#799 不得宽严兜底、#804 语义一致要求及引文归属规则。

采用 `/Users/silencehan/.agents/skills/code-review/SKILL.md` 的 Standards/Spec 两轴结构。并发槽位已满，两轴由本独立评审工位直接完成；本报告不是双子代理或多模型对抗评审。

## Standards

**1 项范围声明问题已修复，0 项未决。**

原新增注释声称严格分型谓词“只服务第二种情况与本节等价锁”，与既有 `SegmentFeatureComplete.lean:103`、`:123` 的第一种情况消费方式不符。作者已收窄为“本节第一种情况只按先破判据确认”，并明确既有 `SegmentEndUp/Down` 与历史消费者未迁移、不声称全构造链已接入。已回读最终注释及上述 SHA，修复成立。

新定义内部没有严格分型失败后换先破判据放行、或先破失败后换严格分型放行的分支；严格 `IsTopFractal/IsBottomFractal` 保持原定义。无并列等价引理与并列直接判决各自承担不同证明责任，未形成 #799 所禁止的宽严 fallback。上下对偶经 `[low, high] ↦ [-high, -low]` 的价格映射逐项一致。

071 的边界包含规则引用由 `:40` 订正到 `:42`，缺口定义的 `:40` 引用保留。Claim10 的相关注释改为正本路径及其行号。已核所用 071 `:26/:28/:30/:32/:38/:40/:42` 位于正文边界 `:46` 前，没有行首或行内注冒充作者正文。未发现须另行处置的结构性代码异味。

## Spec

**0 项未解决的必须修复项。**

1. `FirstBreakConfirmsUp/Down`（`:383/:387`）保留“相等不是破”的边界；`turnWindowStateUp/Down`（`:411/:419`）按列表先后给出 confirmed、killedByNewExtreme、openAtPrefixEnd 三态。包含元素被跳过，未被合并进候选 b。同一向下笔同时越过两端时 high 起点先于 low 终点，因此向上候选先 killed；向下情况镜像。
2. `firstOutsideWindow`（`:548`）及 `firstOutsideWindow_not_contains`（`:552`）为局部等价引理的 `¬ Contains b c` 提供来源。完整前缀通过 `turnWindowStateUp/Down_firstOutside`（`:562/:580`）归约到首个出窗元素，继而连接 singleton、局部先破与严格分型。该前提不是为证明方便任意缩域。
3. `FirstKindConfirmsUp/Down`（`:619/:623`）显式加入第一种情况的 `¬ HasGap a b`；完整前缀等价引理（`:628/:637`）两侧使用同一无缺口条件。无并列只需 c 与 b 的相关极值不同，比“所有端点两两不同”更弱，未附加额外缩域。
4. 并列确认而严格分型不成立的上下引理（`:450/:456`）由先破直接判决。追加数据不改变已裁决状态（`:502/:516`），OPEN 等价于全部前缀元素均被 b 包含（`:599/:608`）。包含元素仍 OPEN 与相反时间顺序产生不同终态的反例（`:647/:654`）明确了可达域及先后次序的重要性。

### 适用边界

`FeatureElem` 仅保存区间，不携带方向或时间戳。§1 的调用方约束及新增 `:407–410` 注释要求输入按时间排列、与 b 同向。这些定理在声明的抽象输入域上成立；尚未机器证明原始行情或笔序列到该输入域的方向、时序连接。

旧 `SegmentEndUp/Down`（`:319/:323`）、`SegmentFeatureComplete.lean` 的 `SegEndComplete/FeatureConfirmed`、`SegmentAutoConstruct.lean` 旧扫描仍保留原消费方式。例 a=[0,8]、b=[2,10]、c=[1,10] 无缺口且先破确认成立，但严格顶分型不成立，足以说明旧 strict 消费链与新定义不在全域等价。本次候选没有新增该旧路径的失败兜底；#1495/X-31 的限定定义与引理工作不等于这些消费者已迁移，也不挡 #1493。

## 验证证据与射程

本工位最终独立执行 `git rev-parse HEAD`、两文件 SHA-256、`git status --short`、`git diff --check`：基线和文件哈希与本报告一致，工作树仅上述两文件修改，差异检查 exit=0。已扫描新增差异，`sorry/admit/axiom/unsafe/native_decide` 无新增命中。

已读取实现工位落盘的下列收据，未另行重复执行构建或 fixture 检查：

- `LIGHT-BUILD.log`：两目标命令为 `PATH=/Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin:$PATH /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake build Origin.SegmentFeatureSeq Claim10_SegmentV1`，cwd 为本工作树 `formal/`；实现工位工具返回 exit=0，完整日志显示 `Built Origin.SegmentFeatureSeq (581ms)`、`Build completed successfully (6 jobs)`。最终运行复用了已构建依赖，不等于六项均在该次重新编译。
- `TOKEN-AND-PREDICATE-CHECK.json`：实现工位报告扫描 152 份 tracked Lean 文件，可执行 `sorry/admit/axiom` 为 0，严格分型谓词未改。此处引用的是落盘检查结果，不将其冒充本工位重复执行的全仓扫描。
- `FIXTURE-DEPENDENCIES-BUILD.log`：授权的依赖闭包构建 27 jobs 成功。日志中 `Formal/CenterTrichotomy.lean:110` 有一条未使用 simp 参数警告，位于本次未改文件。
- `FIXTURE-DRIFT.log`：原命令 `python3 scripts/check_fixture_drift.py`，实现工位返回 exit=0；center、certificate_chain、parity 三组均为 **0 字段差异且字节一致**。再生成物保留于 `fixture_drift_g2j60zhw/`。这证明这三组现有导出物无漂移，不证明新定义已接入旧消费链。
- `FULL-BUILD.log`：封报告时实现工位补交根已批准的原命令 `lake build` 全量构建收据，工具返回 exit=0。本工位核对完成行 `Build completed successfully (158 jobs)` 并检索错误与警告，无 `error:` 命中；两条警告均在本次未改文件（`Formal/CenterTrichotomy.lean:110` 的 unused simp 参数、`Origin/SelfSimilarity.lean:414` 的未显式引用变量）。日志中的 `propext`、`Quot.sound` 是 Lean 基础逻辑公理依赖输出，不是新增 `axiom` 声明。补交后再次核对两源码 SHA，仍与本报告绑定一致。

`FIXTURE-CLOSURE.json` 是构建前的 27 模块闭包快照，其中 4 已构建、23 缺失；不能把该快照单独当作构建完成证明。两目标与 fixture 闭包构建本身不等于全量 `lake build`；全量门另由上述 `FULL-BUILD.log` 证明。全量运行复用了该独立工作树已有构建缓存，日志有 Replayed 项，不能说 158 项均冷启动重编。本工位已读实现收据及 `FINAL-SOURCE-MANIFEST.json`，未独立重跑；清单与独立核得的两源码哈希一致。最终交付与合入决定仍由主控裁决。

恢复方面，本工位已读 `RECOVERY.md` 与 `RECOVERY-MANIFEST.json`。它们记载从精确 Git 基线重放内盘历史日志三个写入体、核对 18 份历史片段；本工位没有独立重跑该恢复流程，也没有验证当前外盘源文件字节身份。本次语义评审绑定的是上述当前候选 SHA，不依赖外盘文件现状。

Standards：1 项已修复、0 项未决；Spec：0 项未决。限定语义评审通过，后续交付须保留本报告的候选绑定与验证边界。
