## Destination

一份落仓的**工作流正本**（落在 `docs/agents/wayfinder-routing.md`，`AGENTS.md` 加指针），写清四件事：本仓什么信号进 wayfinder 流、四种票型在本仓各自长什么样及准入判据、一票一会话的并行纪律与 AFK/HITL 载体分叉、图的寿命与 `/to-spec` 交棒边界。写完即到达——本图不改任何 skill、不改任何代码。

## Notes

- **域**：元层工作流设计。对照源 = Matt Pocock `_wayfinder_ Nothing is too big to plan anymore.txt`（本地 `~/Downloads/`）+ `~/.claude/skills/{wayfinder,ask-matt,prototype,to-spec,to-tickets,implement,grilling,handoff,claude-handoff}/SKILL.md`。
- **每会话必读**：`docs/agents/issue-tracker.md`（Wayfinding operations 段）、`~/.claude/CLAUDE.md`（模型路由表 + 派发授权 + ticket 闸门）。
- **已定前提（不再重议）**：
  1. skill 本体不改。用户明示「假如我们的 skill 版本没有问题 也不需要改变 skills」——问题在路由，不在 skill。
  2. AFK 票（research / 可无人值守的 task）载体 = **本体在当前会话用 Agent 工具派子代理**，按 `~/.claude/CLAUDE.md` 模型路由表从便宜档往贵档挑第一个够格的；结果回流本体验收后写回 tracker。
  3. 一票一会话 + 多会话并行 = **现状已如此**，不是待改项，是待补纪律项。
  4. **HITL 吞吐纪律**（#776 裁定，2026-07-30）——本图剩余 grilling 票即刻适用：默认入口 = `/batch-grill-me` 节奏 + `/domain-modeling` 落文档并调（一轮问完当前 frontier，超 4 题拆两轮）；票内查事实按代价分流；一票一会话例外面 = 全部 AFK 票。条款正文与论证见 [并行吞吐：HITL 瓶颈与 batch 化 #776](https://github.com/xy7365527-lang/NewChanlun/issues/776)，跨图通用部分由 #773 外移正本，此处只留指针。
- **本图带实装**（#770 裁定③ 自指落地，2026-07-30）：[正本落盘 #773](https://github.com/xy7365527-lang/NewChanlun/issues/773) 交付 destination 本身，属标准甲类。理由 = 本图 destination 就是一份文档，写它即到达，另起 to-spec 链是空转；该票走 AFK 子代理落盘 + 本体验收。

- **本图开图时的事实基线**（2026-07-30 点查，21 张图 / 271 张已关子票）：
  | 票型 | 已关 | 未关 |
  |---|---|---|
  | task | 102 | 22 |
  | grilling | 85 | 3 |
  | research | 84 | 4 |
  | **prototype** | **0** | **0** |

## Decisions so far

<!-- 一行一张已关票：够判断相关性即可，细节回票里看 -->

- [task 票名分收束：解阻塞 vs 交付](https://github.com/xy7365527-lang/NewChanlun/issues/770) — 「收还是不收」问错了：task 标签下**五种**东西（甲纯实装/乙探针喂决策/丙实验跑批/丁文书簿记/戊标签贴错），乙丙本来就够格。甲类口径 = **带实装的图 Notes 须显式声明 + 理由，没声明 = 纯决策图、实装票不准挂**（声明制自执行，不设巡查/CI 硬门，与 delivery-discipline 同款）。21 张图仅 2 张声明过。存量：#695/#529/本图补声明，#737（task 0/4）本就合规不需要，已关 16 张不动。判别一句话：**这张票做完，是解开了一个决策，还是交付了一件成果？**

- [三方口径差异清单：视频 vs skill 正文 vs ask-matt](https://github.com/xy7365527-lang/NewChanlun/issues/768) — ask-matt 与 wayfinder 两份正文**都没提 prototype 票型**（本仓 0/0 的成因）；`wayfinder/SKILL.md` 对「图完成之后去哪」完全没写、从不点名 `/to-spec`（三方最大空白）；「一票一会话」skill 里只有间接表达；上下文纪律那条**不是矛盾是接缝留白**（订正开票时的原判）。

- [并行吞吐：HITL 瓶颈与 batch 化](https://github.com/xy7365527-lang/NewChanlun/issues/776) — 订正开票前提：本仓 grilling 票现行默认是 `grill-with-docs`（本地 grill 家族共四个，开票只点了两个），而它转调 `grilling` 故自带一次一问、拿不到 batch 节奏。裁定 grilling 票默认 = **`/batch-grill-me` 节奏 + `/domain-modeling` 落文档并调**（`batch-grill-me` 是 `grilling` 的严格泛化，其 frontier 规则本身禁止把有依赖的题并轮，链式题不会被伤；不新建合体 skill）；超 4 题拆两轮按下游解锁数排序，不挪用 `to-questionnaire`。另定三条 HITL 吞吐纪律：票内查事实**按代价分**（单条自查／多文件强制派子代理）、push right **按可逆性分界**（写仓的合入点逐次批准不动，纯决策纯读右移攒 brief）、一票一会话例外面**扩到全部 AFK 票**（该规则保护的是人的上下文，AFK 不消耗）。**Notes 预授权四条**：沉淀并入开图第二轮 + 走图随手回写；预授权边界**按有无先例**（有判例可套用，新取舍／风险偏好／taste 仍须当场拍）；判错痕迹落 **comment 不落 body**（#772 已查实 body 编辑史 API 取不到）——Notes 条目编号 + 票内声明「依 Notes N-k 判定」；膨胀**按归属分流**（跨图通用外移正本，Notes 只留指针，软信号约 10 条）。条款落盘归 #773。

- [并行纪律：多会话同写一张图的冲突口径](https://github.com/xy7365527-lang/NewChanlun/issues/772) — 订正开票前提：**丢的是索引不是决策**。决策正本在子票的 resolution comment（append-only，并发碰不到），图正文 Decisions-so-far 按设计只是索引，且**可从已关子票列表重建**——原判的「无解的静默丢失」降级为「可检测、可重建的索引漂移」。据此五裁：①写入协议**保留正文整体重写**（不换评论流——那会牺牲图「一次加载看全貌」的核心价值），配三条纪律：写的那一刻才读正文、写前用 `updated_at` 重比一次（应用层读后重比，仍有毫秒级残窗，挡的是秒级真实竞态；冲突重试一次，再冲突报人不盲循环）、外加④的对账；②**认领竞态不管**——assign 是加进集合本就锁不住，HITL 撞车当场可见、AFK 撞车只白跑一个子代理；③**谁清的雾谁毕业**，重复票看得见能关，后发现者关后开的那张指向先开的；④**对账每次走图会话开头跑**（顺手一条查询，丢行最多存活一个会话）；⑤git 层冲突移出本票挂 `/resolving-merge-conflicts` 指针，**写仓的 AFK 子代理强制 worktree、只读的不强制**（本仓已实跑几十个 worktree，成本零点几秒；主仓撞车有 `git stash` 抹改动的实伤前例）——声明制自执行，不设巡查/CI 硬门。条款落盘归 #773。

## Not yet specified

- **票型 × 模型档位的对应**：全局路由表（GPT-5.6 Sol / Sonnet 5 / Opus 5 / Fable 5）里哪档配哪种 AFK 票。要等原型票形态和 task 票名分定了才够锐。
- **长驻总账图的交棒时机**：若「两种图分家」判为要分，长驻图不关闭，那 `/to-spec` 在它上面按什么节拍触发。依赖两种图分家那张票。
- **丁类与戊类票的归属**（#770 浮出）：丁 = 文书/名分/口径落文这类簿记票（#734 ADR 文本债、#725 文书归置、#616 落文书、#520 注释订正），既不产决策也不产功能；戊 = 标签与标题打架（#157 标题 `[grilling]` 贴 task 标签、`[debt]` 一批、#564 是 bug、#670 是评审）。两者同源——非甲非乙的票该贴什么标签。粗于一票，可能并成正本里一行，也可能各开一票。已关票的重贴标签在范围外，只议未来。
- **正本与既有三份 agents 文档的边界**：`delivery-discipline.md` / `generation-constitution.md` / `stat-provenance.md` 已有交付纪律口径，正本写到哪停、哪些只挂指针。

## Out of scope

<!-- 越过 destination 的活：已关闭，永不毕业 -->

- **改 skill 本体**（`~/.claude/skills/*`）——用户明示 skill 版本无问题、不需要改。本图只产路由口径，不动 skill 文件。
- **非 wayfinder 流程的路由**（`/triage`、`/diagnosing-bugs`、`/improve-codebase-architecture` 等 on-ramp）——本图只覆盖「Matt 视频那套」= wayfinder 主体 + `/to-spec` → `/to-tickets` → `/implement` 交棒链。
- **回溯改判 271 张已关票**——历史票不重贴标签、不重开。正本只约束未来。



