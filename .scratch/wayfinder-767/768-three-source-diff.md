# Wayfinder 三源口径比对报告（issue #768）

比对对象：
1. **视频**：`_wayfinder_ Nothing is too big to plan anymore.txt`（443 行，带时间戳字幕，下称"视频"）
2. **skill 正文**：`~/.claude/skills/{wayfinder,prototype,to-spec,to-tickets,implement,grilling,handoff,claude-handoff}/SKILL.md`
   及各自 `agents/openai.yaml`（这些实际是指向 `~/.agents/skills/...` 的符号链接，已用 `-L` 解析并读取真实文件）
3. **路由器**：`~/.claude/skills/ask-matt/SKILL.md` 及 `agents/openai.yaml`

引文规则：全部逐字摘录原文，中文只用于说明差异点本身；找不到对应内容的一律写"未提及"，不猜测、不代填。

---

## 一、逐条差异清单

### D1：单一会话上限——具体数字口径不一致（100K vs ~120K token）

**一句话**：wayfinder 把"一张票的大小"钉死在 100K token 会话，ask-matt 把"smart zone"钉在约 120K token，二者是两个不同的数字，且没有互相引用对方来说明为什么不一样。

- **视频怎么说**：只提到概念，没给出具体数字。01:11–01:18：
  > "Some work is bigger than what you can fit into the context window and especially the smart zone of the context window of the agent."
- **skill 正文怎么说**（`~/.claude/skills/wayfinder/SKILL.md` 第57行，"### Tickets"小节）：
  > "Its body is the question, sized to one 100K token agent session:"
- **ask-matt 怎么说**（`~/.claude/skills/ask-matt/SKILL.md`，"Context hygiene"小节）：
  > "The limit on this is the **[smart zone](https://www.aihero.dev/ai-coding-dictionary/smart-zone)**: the window (~120k tokens on state-of-the-art models) within which the model still reasons sharply."

### D2："一票一个独立会话"这个节拍，视频讲得比 skill 正文更绝对

**一句话**：视频说"每张票都必须有自己独立的会话"，wayfinder/SKILL.md 只说"每个会话最多解决一张票（研究票除外）"——前者是强制"必须拆"，后者是约束"不能并"，措辞强度不同。（本条与疑点 b 重叠，详见下方疑点 b 独立小节，此处不重复展开。）

### D3：视频用的"handoff skill"实际对应的是 claude-handoff，而不是 handoff——但 ask-matt 从未提过 claude-handoff

**一句话**：视频描述的"自动写 prompt、自动起子代理"这个动作，在 skill 仓库里对应的是 `claude-handoff`（会真的后台拉起一个 agent），而不是普通的 `handoff`（只写一个文件，人要手动开新会话去读）；但 ask-matt 的"Crossing sessions"一节自始至终只提 `/handoff`，完全没提 `claude-handoff`。

- **视频怎么说**（06:57–07:14）：
  > "The way I did that was I just called wayfinder on that ticket name. I did it in a slightly fancier way where I actually have a handoff skill that automatically wrote me a prompt and spawned a clawed sub agent. But what it was essentially doing is just calling the wayfinder skill on this map and on the specific ticket wherever it was."
- **skill 正文怎么说**：
  - `~/.claude/skills/handoff/SKILL.md`：
    > "Write a handoff document summarising the current conversation so a fresh agent can continue the work. Save to the temporary directory of the user's OS - not the current workspace."
    （这是"写文件、人工开新会话"的模式，不会自动起子代理。）
  - `~/.claude/skills/claude-handoff/SKILL.md`：
    > "Write a handoff summary of the current conversation so a fresh agent can continue the work. Instead of saving it, launch a background agent seeded with the summary as its prompt: `claude --bg --name "<descriptive name>" "<handoff summary>"`. It starts in the current working directory and returns immediately; the user manages it with `claude agents`."
    （这才是"自动写 prompt + 自动起子代理"，与视频描述的动作吻合。）
- **ask-matt 怎么说**（"Crossing sessions"小节，只提 `/handoff`）：
  > "**`/handoff`** — when a thread is full or you need to branch off (e.g. into a `/prototype` session), this compacts the conversation into a markdown file. You don't continue in place — you **open a new session and reference that file** to carry the context across."
  全文搜索 `claude-handoff` 无匹配——**未提及**。

### D4：wayfinder/SKILL.md 自身完全没有描述"地图完成之后去哪"——这段完全是 ask-matt 和视频在说，两者内容才一致（详见疑点 d）

见下方疑点 d 独立小节。

### D5：视频的"反瀑布"论证（prototype 防止 wayfinder 退化成瀑布）在两份 skill 正文里都不存在

见下方疑点 a 独立小节。

### D6：视频把 ticket type 讲成行为描述，skill 正文引入了 HITL/AFK 这套形式化词汇，视频从未使用这两个缩写

**一句话**：wayfinder/SKILL.md 明确给每种票分类"HITL（人在环内）"或"AFK（agent 自主）"，视频只描述行为（比如研究票"不用盯着看"），从没说过 HITL / AFK 这两个词。

- **视频怎么说**（07:52–08:03，描述 research 票）：
  > "Research tickets are where the agent needs to go off and find some information and bring it back and it usually kicks it off immediately. So you don't actually need to watch it. It does it in a sub agent and then reports back."
- **skill 正文怎么说**（`~/.claude/skills/wayfinder/SKILL.md`，"## Ticket Types"）：
  > "Every ticket is either **HITL** — human in the loop, worked *with* a human who speaks for themselves — or **AFK**, driven by the agent alone. A HITL ticket only resolves through that live exchange; the agent never stands in for the human's side of it (a grilling agent that answers its own questions has broken this)."
  > "- **Research** (AFK): ... Resolved by a `/research` **subagent**."
- **ask-matt 怎么说**：全文搜索 "HITL"、"AFK" 均无匹配——**未提及**。

### D7："setup-matt-pocock-skills" 这个具体命令名，视频里说的是"setup map skills"，措辞不一致

**一句话**：三份 skill 正文（wayfinder / to-spec / to-tickets）统一要求先跑 `/setup-matt-pocock-skills` 做 tracker 配置，视频里 Matt 口头说的是"setup map skills"，不是这个精确命令名（可能是口语简化或转录误差，但两者字面不同）。

- **视频怎么说**（05:33–05:36）：
  > "You just need to do a little bit of configuration via setup map skills. Use it with linear, use it with Jira, use it with literally whatever you like."
- **skill 正文怎么说**（`~/.claude/skills/wayfinder/SKILL.md` 第25行）：
  > "The issue tracker should have been provided to you — run `/setup-matt-pocock-skills` if not."
  同样的命令名也出现在 `~/.claude/skills/to-spec/SKILL.md` 第9行和 `~/.claude/skills/to-tickets/SKILL.md` 第11行。
- **ask-matt 怎么说**（"## Precondition"）：
  > "**`/setup-matt-pocock-skills`** — run before your first engineering flow to configure the issue tracker, triage labels, and doc layout the other skills assume. Custom issue trackers also work."

### D8：视频讲了"spec 用完即删、不留存"的纪律，to-spec/SKILL.md 和 ask-matt 都完全没提这条

**一句话**：视频明确说自己流派是"spec 落地进代码后就把 issue 关掉、spec 从仓库里消失、几乎不会再回头看"，这是和主流 spec-driven-development（保留 spec 反复编辑）刻意划清界限的一条纪律；但 `to-spec/SKILL.md` 通篇没讲 spec 的生命周期终点，ask-matt 也没讲。

- **视频怎么说**（13:33–13:47）：
  > "people when they get to the end of this, they will keep that spec around somewhere. For me, I close the issue containing the spec and the spec is gone. It's gone from my repository. I rarely if ever refer to it again. Once the spec is present in the code, then you can just delete the spec."
- **skill 正文怎么说**：`~/.claude/skills/to-spec/SKILL.md` 只讲怎么写 spec、发布到 tracker、打 `ready-for-agent` 标签，全文没有一句讲 spec 完成后要关闭/删除——**未提及**。
- **ask-matt 怎么说**：全文没有"close the issue"、"delete the spec"这类表述——**未提及**。

### D9：视频提到"Wayfinder 也能处理非编码任务（花园办公室）"，这条只有 wayfinder/SKILL.md 有对应的形式化表述，ask-matt 没提

**一句话**：wayfinder 的"域无关"能力在视频里是用真实非编程案例（造花园办公室）讲的，skill 正文用一句抽象声明覆盖了它，ask-matt 完全没提这个维度。

- **视频怎么说**（12:16–12:33）：
  > "I've actually been using Wayfinder for non-coding tasks. So, I've been meaning to put up a garden office in my garden and uh I've been using Wayfinder for that. So, it's uh commissioning a site survey, uh figuring out all that stuff, figuring out who to contact, doing all the research, finding the different firms that could build it."
- **skill 正文怎么说**（`~/.claude/skills/wayfinder/SKILL.md` 第9行）：
  > "The map is domain-agnostic — engineering work, course content, whatever fits the shape."
- **ask-matt 怎么说**：wayfinder 段落里没有"domain-agnostic"或非编码场景的表述——**未提及**。

### D10：wayfinder/SKILL.md 的"Refer by name"纪律（禁止裸编号，必须用带链接的标题称呼票据），视频和 ask-matt 都没提

**一句话**：这是纯 skill 正文独有的一条纪律，另外两个来源都没讲。

- **skill 正文怎么说**（`~/.claude/skills/wayfinder/SKILL.md`，"## Refer by name"）：
  > "Every map and ticket is an issue, so it has a **name** — its title. In everything the human reads — narration, the map's Decisions-so-far — refer to it by that name, never by a bare id, number, or slug. A wall of `#42, #43, #44` is illegible; names read at a glance."
- **视频怎么说**：**未提及**（视频里 Matt 讲例子时用的是"the clips during publish race"这种标题式称呼，行为上大致相符，但没有把这总结成一条显式纪律）。
- **ask-matt 怎么说**：**未提及**。

---

## 二、四条疑点逐一查实

### 疑点 a：ask-matt 讲 wayfinder 的那一段，是否完全没提 prototype？

**结论：核实为真——完全没提。** ask-matt 里"On-ramps"小节讲 wayfinder 的完整一段，逐字如下（`~/.claude/skills/ask-matt/SKILL.md`）：

> "- **A huge, foggy effort — a greenfield project or a huge feature build, too big for one session** → **`/wayfinder`**, the most cognitively demanding flow here. When the way from here to the destination isn't visible yet, it charts a **shared map** of **decision tickets** on the issue tracker and resolves them one at a time — producing **decisions, not deliverables** — until the fog is pushed back and the way is clear. Where **`/grill-with-docs`** sharpens an idea you can hold in one session, wayfinder is for the idea you can't — and it's slower and denser, so save it for exactly that, never a well-scoped feature.
>
>   When the map clears, **it hands off, it doesn't build**: merge onto the main flow at **`/to-spec`**, which collapses the map's linked decisions into a buildable plan, then `/to-tickets` and `/implement` as usual. Looping the map straight into `/implement` skips that collapse and throws the linked detail away — go straight to `/implement` only when the effort turned out genuinely small."

这两段里没有"prototype"这个词，也没有 research / grilling / task 这几种票型的任何提及——ask-matt 的 wayfinder 段落只讲"这个 flow 在整体路线图里的位置"和"完成后怎么交棒"，完全不触及票据分类学。`/prototype` 在 ask-matt 全文其他地方确实出现过（主流程第2步、"Standalone"一节），但都不是在 wayfinder 语境下，也从未出现视频里那句"prototype 防止 wayfinder 退化成瀑布"的论证。

对照视频 08:03–08:42 原句：

> "Prototype tickets, which are the next type here, create a prototype, which is so unbelievably invaluable for really seeing things come to life as you're planning. I've done a whole extra video on this on how important prototypes are, and it reuses the prototype skill from that video. Some folks look at Wayfinder and they think, "God, that's a lot of planning. Doesn't that look like waterfall?" And the prototypes are the way that you prevent it from becoming waterfall. Huge amounts of lowfidelity upfront planning. A prototype is a highfidelity way to get feedback on what you're actually building. And the fact that Wfinder encourages you to build so many prototypes means that the output is unbelievably good."

再核对 `~/.claude/skills/wayfinder/SKILL.md` 本身："## Ticket Types"里的 Prototype 定义是：

> "**Prototype** (HITL): Raise the fidelity of the discussion by making a cheap, rough, concrete artifact to react to — an outline, a rough take, a stub, or UI/logic code via the /prototype skill. Links the prototype as an asset. Use when "how should it look" or "how should it behave" is the key question."

wayfinder/SKILL.md 提到了 prototype 票型，但同样没有"防瀑布"这个论证角度——全文（包括 `prototype/SKILL.md`、`LOGIC.md`、`UI.md`）搜索 "waterfall" 均无匹配。所以"prototype 防止 wayfinder 退化成瀑布"是**纯视频独有**的论证，两份 skill 正文（wayfinder 和 ask-matt）都没有这个框架，ask-matt 更是连 prototype 这个词都没提。

### 疑点 b："一票一个独立会话"这个节拍，wayfinder/SKILL.md 和 ask-matt/SKILL.md 各自有没有明写？

**结论：wayfinder/SKILL.md 用不同措辞间接表达了同一约束，但没有视频那句"每张票都要自己的会话"那么绝对；ask-matt 完全没提这个节拍。**

- **视频怎么说**（03:15–03:20，明确、绝对）：
  > "And each of these things on the map, they are tickets. Each ticket requires its own individual session with the agent."
  以及后面具体描述这个节拍怎么落地（06:54–07:14，含 handoff 细节，见上方 D3）：
  > "And so what I did was I then worked through each of those tickets in a new session. The way I did that was I just called wayfinder on that ticket name. I did it in a slightly fancier way where I actually have a handoff skill that automatically wrote me a prompt and spawned a clawed sub agent."

- **wayfinder/SKILL.md 怎么说**（"## Invocation"开头）：
  > "Two modes. Either way, **never resolve more than one ticket per session** — with the exception of research tickets."
  以及"### Tickets"小节里票据大小的约束：
  > "Its body is the question, sized to one 100K token agent session"
  以及"### Work through the map"末尾：
  > "The user may run unblocked tickets in parallel, so expect other sessions to be editing the tracker concurrently."

  这三处合起来，语义上等价于"一票配一个会话"，但措辞是"一个会话里最多解决一张票"（约束会话，不是约束票），比视频"每张票都必须有自己独立的会话"要弱一点——SKILL.md 没有禁止"这个会话什么都不做，只是空转/退出"，也没提视频里那个"自动 spawn 子代理"的具体机制（那是 D3 里讲的 claude-handoff 细节，SKILL.md 本身完全不提任何 handoff 机制）。

- **ask-matt/SKILL.md 怎么说**：全文搜索"one ticket"、"per session"、"individual session"均无匹配，wayfinder 段落（即疑点 a 里引的那两段）里也没有这个节拍——**未提及**。ask-matt 里唯一讲"会话边界"规则的是主流程"Context hygiene"一节（讲 grill-with-docs → to-spec → to-tickets 要在同一个会话里不间断），但那条规则针对的是主流程 1–3 步，不是 wayfinder 的逐票会话节拍。

### 疑点 c：ask-matt「第1–3步不间断上下文窗口」纪律，与 wayfinder「一票一会话、状态放 tracker」是否冲突？

**结论：字面上确有张力，但严格讲不是硬冲突——两条规则的作用域不同，问题在于两份文档都没有明写"wayfinder 进入主流程时这条规则怎么算"，属于文档留白，不是相互矛盾的断言。**

两边原句对照：

- **ask-matt/SKILL.md**（"### Context hygiene"，紧跟在主流程步骤1-3之后）：
  > "Keep steps 1–3 in **one unbroken context window** — don't compact or clear until after `/to-tickets` — so the grilling, spec, and tickets all build on the same thinking. Each `/implement` then starts fresh, working from the ticket."

- **wayfinder/SKILL.md**：
  > "Two modes. Either way, **never resolve more than one ticket per session** — with the exception of research tickets."
  > "**Where the map, its child tickets, blocking, and frontier queries physically live is tracker-specific.**"（状态显式存放在 tracker，不是上下文窗口）
  > "The user may run unblocked tickets in parallel, so expect other sessions to be editing the tracker concurrently."

分析张力所在：ask-matt 的"steps 1–3"字面指的是主流程本身编号的三步（1. `/grill-with-docs`；2. 视情况走 prototype/handoff 分支；3. `/to-spec`+`/to-tickets` 或直接 `/implement`），这条规则的前提是"你是从 grill-with-docs 直接进来的"。而 wayfinder 是"On-ramps"一节里单独列出的一条岔路，按 ask-matt 自己的说法是"merge onto the main flow at `/to-spec`"——也就是说 wayfinder 把主流程的第1、2步换成了它自己的多会话流程（宪章上明确允许、甚至要求跨会话，状态记在 tracker 而非上下文窗口），然后只在第3步（`/to-spec` 起）才汇入主流程。

所以从最窄的字面解读看：wayfinder 走图的那部分（对应主流程第1-2步的角色）**不受**"steps 1–3 不间断"这条规则约束，因为 wayfinder 根本不是在做"steps 1–3"，它是那条规则生效范围之外的另一条岔路；等地图收敛、调用 `/to-spec` 时，才重新进入受这条规则辖制的"第3步"区间。按这个读法，两条规则不矛盾，各管一段。

但问题是：**两份文档都没有一句话明确写出"wayfinder 走完之后从 `/to-spec` 开始才受 context-hygiene 约束"这句衔接**。ask-matt 的 wayfinder 段落只说"merge onto the main flow at `/to-spec`"，没有回头呼应 Context hygiene 那条规则；wayfinder/SKILL.md 更是完全没有提到 `/to-spec`（见下方疑点 d），也没有提到"context hygiene"或"smart zone"这个概念。所以这是一处**未言明的衔接空隙**，而不是两条互相打脸的断言——按字面各自读都自洽，但没人写清楚接缝处怎么处理，实操时容易被理解成冲突。

### 疑点 d：`/to-spec` 从 wayfinder 图上交棒的口径，三方是否一致？

**结论：视频和 ask-matt 口径一致（甚至措辞高度接近），但 wayfinder/SKILL.md 自身对这一步完全没有描述——这是三方里最大的一处空白，而不是分歧。**

- **视频怎么说**（09:44–10:31）：
  > "So, then once the map is complete, what do you then go and do with it? Well, this one because its detonation was a speck, the wayfinder map is probably a little bit too dense to create a spec. So, what I like to do is create a spec from the map. This was the spec that I created from it. And you can see it's basically the same setup as I've had before. I literally just called to spec on the wayfinder map and it pulled in this enormous document with basically all of the decisions that have been pulled from the wayfinder map into this uh GitHub issue. The initial draft was actually too large for GitHub's character limit."
  （注：视频原字幕这里有明显语音转文字误差，"detonation"应为"destination"，"speck"应为"spec"——按逐字引用规则原样保留，不代为纠正。）
  紧接着 10:31–10:52：
  > "And from there I turned it into tickets using my usual approach which is to spec and then to tickets. In other words, Wfinder fits in just in exactly the same place that grill with docs does in my usual approach."

- **ask-matt/SKILL.md 怎么说**（wayfinder 段落第二段，已在疑点 a 引过，此处复述关键句）：
  > "When the map clears, **it hands off, it doesn't build**: merge onto the main flow at **`/to-spec`**, which collapses the map's linked decisions into a buildable plan, then `/to-tickets` and `/implement` as usual. Looping the map straight into `/implement` skips that collapse and throws the linked detail away — go straight to `/implement` only when the effort turned out genuinely small."

  这与视频的"地图太密，不能直接当 spec 用，要用 `/to-spec` 先收敛"是同一个论证：视频说地图"too dense to create a spec [directly serve as one]"，ask-matt 说跳过 `/to-spec` 直接进 `/implement` 会"throws the linked detail away"——两者措辞不同但结论完全一致：**不能把地图直接接进 `/implement`，必须先过一道 `/to-spec` 收敛**。

- **wayfinder/SKILL.md 怎么说**：全文搜索 "to-spec"、"to-tickets"、"hands off"均**无匹配**。`~/.claude/skills/wayfinder/SKILL.md` 的"## Invocation"只定义了两个模式——"Chart the map"和"Work through the map"，两个模式的收尾都是"继续走图"或"停下等下一个会话"，通篇没有一句描述"地图收敛之后该怎么办、该调用哪个 skill 交棒"。也就是说，**wayfinder 自己的正文对"图收敛之后去哪"这件事完全沉默**，这个环节的全部说明只存在于 ask-matt（路由器）和视频（Matt 本人的口头示范）里，且两者内容彼此吻合。

- 另外核实一下 `to-spec/SKILL.md` 这一侧：它本身也没有反过来提"如果输入是一张 wayfinder 地图，要怎么处理"这种特殊分支——`~/.claude/skills/to-spec/SKILL.md` 通篇讲的是"synthesize what you already know"，把 wayfinder 地图当成"当前对话上下文"的一种来源隐式覆盖，没有专门条款。这也是一处沉默，但不影响上面"视频与 ask-matt 一致"的结论。

---

## 三、prototype 判据完整摘录（供后续票据直接引用）

来源：`~/.claude/skills/prototype/SKILL.md`、`~/.claude/skills/prototype/LOGIC.md`、`~/.claude/skills/prototype/UI.md`（`agents/openai.yaml` 只有展示名，无判据内容，不再重复摘录）。

### 3.1 什么算一个合格的原型（总纲，`prototype/SKILL.md`）

- **定义**：
  > "A prototype is **throwaway code that answers a question**. The question decides the shape."
- **分支选择**（问题类型决定走哪条分支）：
  > "- **"Does this logic / state model feel right?"** → [LOGIC.md](LOGIC.md). Build a tiny interactive terminal app that pushes the state machine through cases that are hard to reason about on paper."
  > "- **"What should this look like?"** → [UI.md](UI.md). Generate several radically different UI variations on a single route, switchable via a URL search param and a floating bottom bar."
  > "The two branches produce very different artifacts — getting this wrong wastes the whole prototype. If the question is genuinely ambiguous and the user isn't reachable, default to whichever branch better matches the surrounding code (a backend module → logic; a page or component → UI) and state the assumption at the top of the prototype."

- **两条分支共用的六条规则**（"## Rules that apply to both"）：
  1. > "**Throwaway from day one, and clearly marked as such.** Locate the prototype code close to where it will actually be used (next to the module or page it's prototyping for) so context is obvious — but name it so a casual reader can see it's a prototype, not production. For throwaway UI routes, obey whatever routing convention the project already uses; don't invent a new top-level structure."
  2. > "**One command to run.** Whatever the project's existing task runner supports — `pnpm <name>`, `python <path>`, `bun <path>`, etc. The user must be able to start it without thinking."
  3. > "**No persistence by default.** State lives in memory. Persistence is the thing the prototype is _checking_, not something it should depend on. If the question explicitly involves a database, hit a scratch DB or a local file with a clear "PROTOTYPE — wipe me" name."
  4. > "**Skip the polish.** No tests, no error handling beyond what makes the prototype _runnable_, no abstractions. The point is to learn something fast."
  5. > "**Surface the state.** After every action (logic) or on every variant switch (UI), print or render the full relevant state so the user can see what changed."
  6. > "**Capture it when done.** Fold any validated decision into the real code, then capture the prototype itself as a **primary source**: commit it to a throwaway branch, out of main, and leave a context pointer to that branch on the implementation issue. Capture the answer too — the verdict and the question it settled — in the issue or a commit. The main branch keeps only the validated decision."

### 3.2 Logic 分支判据（`prototype/LOGIC.md`）

- **适用场景**：
  > "'I'm not sure if this state machine handles the edge case where X then Y.'"
  > "'Does this data model actually let me represent the case where...'"
  > "'I want to feel out what the API should look like before writing it.'"
  > "Anything where the user wants to **press buttons and watch state change**."

- **产出流程（7 步）**：
  1. 先写下问题：
     > "Before writing code, write down what state model and what question you're prototyping. One paragraph, in the prototype's README or a comment at the top of the file. A logic prototype that answers the wrong question is pure waste — make the question explicit so it can be checked later, whether the user is watching now or returning to it AFK."
  2. 语言选用宿主项目已有的运行时，"Match the project's existing conventions for tooling — don't add a new package manager or runtime just for the prototype."
  3. 把逻辑隔离进可移植模块（四种可选形状：pure reducer / state machine / 一组纯函数 / 有清晰方法面的 class）：
     > "Put the actual logic — the bit that's answering the question — behind a small, pure interface that could be lifted out and dropped into the real codebase later. The TUI around it is throwaway; the logic module shouldn't be."
     > "Keep it pure: no I/O, no terminal code, no `console.log` for control flow. The TUI imports it and calls into it; nothing flows the other direction."
  4. 建最小 TUI：每帧先渲染"当前状态"（bold 字段名/dim 次要信息），再渲染"键位提示"；"clear the screen... and re-render the whole frame"，"The whole frame should fit on one screen."
  5. 一条命令可跑：接入项目已有 task runner；没有 task runner 就把命令写在 README 顶部。
  6. 交给用户：
     > "Give the user the run command. They'll drive it themselves; the interesting moments are when they say "wait, that shouldn't be possible" or "huh, I assumed X would be different" — those are the bugs in the _idea_, which is the whole point."
  7. 收尾捕获（对应总纲第6条的 logic 版本）：
     > "the validated reducer / machine / function set lifts into the real module (the decision, absorbed); the TUI shell rides along to the throwaway branch that keeps the prototype as a primary source."

- **抛弃纪律（Anti-patterns，5 条）**：
  > "- **Don't add tests.** A prototype that needs tests is no longer a prototype."
  > "- **Don't wire it to the real database.** Use an in-memory store unless the question is specifically about persistence."
  > "- **Don't generalise.** No "what if we wanted to support X later." The prototype answers one question."
  > "- **Don't blur the logic and the TUI together.** If the reducer / state machine references `console.log`, prompts, or terminal escape codes, it's no longer portable. Keep the TUI as a thin shell over a pure module."
  > "- **Don't ship the TUI shell into production.** The shell is optimised for being driven by hand from a terminal. The logic module behind it is the bit worth keeping."

### 3.3 UI 分支判据（`prototype/UI.md`）

- **适用场景**：
  > "'What should this page look like?'"
  > "'I want to see a few options for this dashboard before committing.'"
  > "'Try a different layout for the settings screen.'"
  > "Any time the user would otherwise spend a day picking between three vague mockups in their head."

- **两种子形态，强烈偏向子形态 A**：
  > "A UI prototype is much easier to judge when it's **butting up against the rest of the app** — real header, real sidebar, real data, real density. A throwaway route on its own is a vacuum: every variant looks fine in isolation. Default to sub-shape A whenever there's a plausible existing page to host the variants. Only reach for sub-shape B if the prototype genuinely has no nearby home."
  - 子形态 A（挂在已有页面上，`?variant=` 参数切换，数据获取/鉴权都不变，只换渲染子树）——首选。
  - 子形态 B（全新路由，仅当"确实没有任何现成页面可以挂"时才用，路径要带 `prototype` 字样表明是原型）——最后手段。

- **产出流程（6 步）**：
  1. 写下问题并定 N：
     > "Default to **3 variants**. More than 5 stops being radically different and starts being noise — cap there."
  2. 生成结构上真正不同的变体：
     > "Variants must be **structurally different** — different layout, different information hierarchy, different primary affordance, not just different colours. Three slightly-tweaked card grids isn't a UI prototype, it's wallpaper. If two drafts come out too similar, redo one with explicit "do not use a card grid" guidance."
  3. 用单一 switcher 组件按 `?variant=` 参数切换渲染。
  4. 建浮动切换条：左箭头（上一个，循环）/ 当前变体标签 / 右箭头（下一个，循环）；键盘 ←→ 也能切换（输入框聚焦时不拦截）；样式要与页面本身有视觉区分；且：
     > "Hidden in production builds — gate on `process.env.NODE_ENV !== 'production'` or an equivalent check, so a stray prototype merge can't ship the bar to users."
  5. 交给用户，把 URL 和各 `variant` 键交出去；预期反馈是"我要 B 的头 + C 的侧边栏"这种混搭需求。
  6. 收尾捕获与清理（对应总纲第6条的 UI 版本）：
     > "**Sub-shape A** — fold the winner into the existing page; drop the losing variants and the switcher from main."
     > "**Sub-shape B** — promote the winning variant to a real route; drop the throwaway route and the switcher from main."
     > "The full set of variants is the primary source, so it lands on the throwaway branch, not the bin — variant components and the switcher left in the main branch rot fast and confuse the next reader."

- **抛弃纪律（Anti-patterns，4 条）**：
  > "- **Variants that differ only in colour or copy.** That's a tweak, not a prototype. Real variants disagree about structure."
  > "- **Sharing too much code between variants.** A shared `<Header>` is fine; a shared `<Layout>` defeats the point. Each variant should be free to throw out the layout."
  > "- **Wiring variants to real mutations.** Read-only prototypes are fine. If a variant needs to mutate, point it at a stub — the question is "what should this look like", not "does the backend work"."
  > "- **Promoting the prototype directly to production.** The variant code was written under prototype constraints (no tests, minimal error handling). Rewrite it properly when you fold it in."

---

## 四、材料核对记录

- 视频文稿：`/Users/silencehan/Downloads/_wayfinder_ Nothing is too big to plan anymore.txt`，443 行全部读完。
- skill 目录在 `~/.claude/skills/` 下均为指向 `~/.agents/skills/` 的符号链接，已用 `find -L` 解析真实路径并逐份读取，包括：
  - `wayfinder/SKILL.md` + `wayfinder/agents/openai.yaml`
  - `prototype/SKILL.md` + `prototype/LOGIC.md` + `prototype/UI.md` + `prototype/agents/openai.yaml`
  - `to-spec/SKILL.md` + `to-spec/agents/openai.yaml`
  - `to-tickets/SKILL.md` + `to-tickets/agents/openai.yaml`
  - `implement/SKILL.md` + `implement/agents/openai.yaml`
  - `grilling/SKILL.md` + `grilling/agents/openai.yaml`
  - `handoff/SKILL.md` + `handoff/agents/openai.yaml`
  - `claude-handoff/SKILL.md` + `claude-handoff/agents/openai.yaml`
  - `ask-matt/SKILL.md` + `ask-matt/agents/openai.yaml`
- 全程未修改任何 skill 文件、未 commit、未建分支，仅在 `.scratch/wayfinder-767/` 下落盘本报告。
