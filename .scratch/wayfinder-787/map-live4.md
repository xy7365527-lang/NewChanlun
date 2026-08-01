## Destination

**一份递归式统一的架构正本落盘**：从五词体系（递归·区间套·背驰·级别·买卖点）推到目标模块图、再推到线怎么切——三层贯通的一条推导链，后续所有新图新线照它接。

**关图条件（2026-07-30 编排者裁定，因「正本不落地怎么能算完成」之问而加）**：**正本落盘 ≠ 本图完成。**本图关闭须同时满足两条——① 正本文档写完入仓；② **下游实施线已开、有票号、有主**（照正本去改代码那条线）。

本图**不因此变成带实装的图**（它仍是纯决策图，实装不进来），加的只是「**下游有没有真的接上**」这个前置检查。成因即本图立图发现之一：[#743](https://github.com/xy7365527-lang/NewChanlun/issues/743) 关图时声明三雾归 [#529](https://github.com/xy7365527-lang/NewChanlun/issues/529)，而 [#529](https://github.com/xy7365527-lang/NewChanlun/issues/529) 里 0 命中——**移交是声明的，没人核对落点**。跨图通用的那半已外移进 `docs/agents/wayfinder-workflow.md`「图的寿命与关图判据」（commit `47f2048575`），本条只留本图特有的部分。

到达 = 这份正本写完并入仓。**终点是文档，故不出 spec**（本仓正本判据：终点是文档就直接写），收口票正文即实施总单。

### 前提：统一必须是递归式的（2026-07-30 编排者裁定，非待裁项）

> **「我们这张图的目的不仅是统一，并且也必然是递归式的统一。」**

理由（编排者当场给出）：**系统所有东西都是递归自相似的，所以要不断复用重复这些模块。如果每次都自己造轮子，当然会重复——而实际上架构本身就应该重复。这就是原因。**

**这条是本图的立图前提，不是本图的待裁问题。**据此：

- 目标模块图的验收标准不是「模块划清楚了」，而是「**同一套判定在每一级复用同一个模块**」；
- 教义口径收敛（[#793](https://github.com/xy7365527-lang/NewChanlun/issues/793)）的目标形态随之被约束：不是「40 套里选一套」，而是「收成一套**能在每级复用**的」；
- [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 由「该不该递归自相似」改为「**递归式统一怎么落地**」——「该不该」已由本前提回答。

**★ 双向递归（2026-07-30 编排者当场提出，待验但已列为前提级认识）**：

> **「缠论有区间套的递归，也有同级别分解，这两个模式是不是我们恰好需要呢？」**

**缠论有两个方向的递归，故「塔 vs 树」是伪二选一——两种表示各管一个方向，是领域决定的，非实现凑巧**：

| 方向 | 缠论对应 | 适配表示 | 实现现状 |
|---|---|---|---|
| **横向**：同一层内把走势切成若干段 | **同级别分解** | **塔**（`Vec<Vec<LeveledMove>>`，相邻三段挨着，看重叠自然） | ✅ `recursive_t` 信号层 `apply_t(units, level, mode)` **做到自相似**；`theta_v0` **已 90% 同形态** |
| **纵向**：给定点往下一级钻找更精确位置 | **区间套** | **树**（`RMove = Segment \| Compose`，往下钻天然） | ❓ **疑似空转**——`cand_delta` 对 `Type2 \| Type3` **per-rung 恒 `true`**，只在入口 base gate 查一次 |

⟹ **「同一套判定在每级复用」说得不够，应是两套复用**：① 横向分解的判定在每层复用（已 90%）；② 纵向下钻的判定在每次下钻复用。

**【2026-07-30 [#802](https://github.com/xy7365527-lang/NewChanlun/issues/802) 实证订正】** 「纵向疑似为空」**已被推翻**——下钻机制层是**带测试锁的真自递归**且真用了嵌套结构。**成立的是弱形式**：π 门 Type2/3 的 rung 链零判别力（`Type2|Type3 => true`），实测 **95.36% 信号 rungs 空**。**真缺口不在递归本身，在消费侧三处空洞**：深度算出来没人看（生产只吃 `is_some()`）、`lvl==0` 免门、Type1 从不下钻。**双向递归这个认识框架仍然成立且有用**——它正是这次能精确定位到「消费侧」而非泛泛归咎「架构不对」的原因。

**若该分析成立，其后果远超架构层**：[证书为什么没被放行 #737](https://github.com/xy7365527-lang/NewChanlun/issues/737) 定案的主因（背驰判定说不成立）**可能不是根因**——根因是**纵向那个方向压根没实现成递归**：区间套本该一级级钻下去、每级做同样判定，实际是入口查一次、下面全部放行。**[#802](https://github.com/xy7365527-lang/NewChanlun/issues/802) 已验：二者是两个不同机制（[#737](https://github.com/xy7365527-lang/NewChanlun/issues/737) 的 78% 属链臂基例缺证书，π 门 rung 恒真是另一条），[#737](https://github.com/xy7365527-lang/NewChanlun/issues/737) 定案不改判。**

**边界判据（2026-07-30 编排者裁定，取代口号式的「完全模块化」）**：

> **【2026-07-30 [#803](https://github.com/xy7365527-lang/NewChanlun/issues/803) 实证补注】** 会话中曾推测「三阶段应归自相似、单例的只是资金池」——**已被否证**：三阶段在本仓事实上也是单例，且有**独立于账户的原文理由**（第 31 课全局门、chan99「级别只决定量」）。**但该推测的方向没白费**：它逼出了真正每级一份的那个对象——**`LegPair` + `leg_open_units`**。**教训入判据：用本判据分类时，先确认「被分类的那个东西」是不是你以为的那个**——归错对象比归错类更隐蔽。


> **凡自相似者必模块化复用，凡单例者显式声明为单例。**

即：**凡本身递归自相似的东西（缠论结构判定——中枢/背驰/走势在每级同构），架构必须自相似复用，不复用的地方要举证；凡本身单例的东西（资金、账户、连接、下单通道），不逼它自相似，但必须把「我是单例」显式写明**，不许再用开关和环境变量假装通用（`recursive_t` 操作层 22 开关 + 24 处 `env::var` 即反面）。

**为什么要这条边界（实证，非妥协）**：[#800](https://github.com/xy7365527-lang/NewChanlun/issues/800) 查实操作层「每级独立自我复制」**试过并被实测打回**——三标的 L3 全部 `sink=0`、做空腿结构性不可能激活、`recursive_t_deadlock_escalation.md:103` 记「4 角度全 infeasible」，最终由编排者第五方案裁决回退、per-instance 三阶段明文删除（`rec_engine.rs:41`）。[#801](https://github.com/xy7365527-lang/NewChanlun/issues/801) 独立划出同一条线（其结论**仅覆盖结构判定层**）。

**根因非工程能力**：架构能否自相似，取决于**被建模的东西本身是否自相似**。中枢在 1 分钟与日线上是同一个东西 ⟹ 同码复用天经地义；**而账户只有一份，不是每级一份** ⟹ 三阶段资金战役本质单例，强行每级一份实测即 `sink=0`。

**推论——「统一」的对象要分两类**：**该铲的重复**（两套 fill 同跑给出 −10.25% vs −23.06% 且仓内无解释、40 套口径、两条互不相通的 nautilus 集成）vs **合法的多样**（ADR-0005 三个独立对照臂、Lean 形式化层与 Rust 实现层本就是两种东西）。不区分，统一会把该留的一并铲掉。

**范围（2026-07-30 编排者追加裁定）**：**不止 `theta_v0` 一层——所有层都要统一到递归自相似模块化。**原话：「我们实际上有好几层，我认为都要统一递归自相似模块化，这是最关键的。」范围以 [疆域测绘 #790](https://github.com/xy7365527-lang/NewChanlun/issues/790) 的六区为锚（`src/newchan` / `rust/src/*.rs` 顶层散件 / `rust/src/theta_v0/` / `rust/src/recursive_t/` / `formal/` / `trading_system/`）。

**该裁定的实证支撑（[#790](https://github.com/xy7365527-lang/NewChanlun/issues/790) + [#800](https://github.com/xy7365527-lang/NewChanlun/issues/800) 合读，2026-07-30 当场撞出）**：

- **两条信号层各跑各的，互不相通**——实盘壳走 `trading_system/` → Python `nautilus_trader` → **`newchan_rust.RecTStream`（`recursive_t` 系）**（`rec_t_strategy.py:45`）；回测/研究走 `theta_v0` → Rust 侧 nautilus crate → CLI。
- **`theta_v0`（17 万行，最大区、40 套口径主场）零 Python 出口**（`lib.rs:76` 注释「骨架阶段（未接入 PyO3）」，PyO3 导出清单零条目）⟹ **其回测结果没有路径流回实盘壳**。
- **反讽**：唯一做到递归自相似、且有回归测试锁「所有级别用同一套代码」的信号层（`recursive_t` 信号层 2611 行，[#800](https://github.com/xy7365527-lang/NewChanlun/issues/800)），**正是实盘在用的那个**；而被定性为失败的那套引擎，其算法部分才接到了实盘。

⟹ **「统一」的范围不能只画在 `theta_v0` 内部**，否则统一完的东西仍然流不到实盘。

**次序（2026-07-30 编排者裁定）**：**接通（两条 nautilus 集成合流 / `theta_v0` 的 Python 出口 / 实盘段）排在统一之后**——先把递归自相似的形态定下来、模块收敛完，再一次性接通。理由是现在接通等于把两套「五概念无一同名同义」的东西焊在一起，焊完更难拆。**代价明写**：在此之前，一切修复（含 [#737](https://github.com/xy7365527-lang/NewChanlun/issues/737) 定案要投的两块确证缺陷）都不会改变实盘行为——因为**目前没有任何一条路在拿缠论信号实盘下单**（[#792](https://github.com/xy7365527-lang/NewChanlun/issues/792)：`LiveNode` 全仓 3 处命中全是注释、零代码使用；`hl_verify_nautilus.py` 能真实下单但不消费缠论信号）。

**自指推论一并采纳**：架构工作反复重做（[#743](https://github.com/xy7365527-lang/NewChanlun/issues/743) 端到端模块化 → 本图统一架构），本身即「架构缺自相似性」的症状。

**已知的反面标本**：`rust/src/recursive_t/`（16 文件 / 17383 行）是上一次朝该方向的尝试，编排者定性「失败，不成熟」——**一次为消除重复而做的自相似尝试，本身产出了第二套引擎**。成因复盘见 [#800](https://github.com/xy7365527-lang/NewChanlun/issues/800)，其结论是 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 的前置输入。

### ★★ 前提第二次订正（2026-07-30，[#805](https://github.com/xy7365527-lang/NewChanlun/issues/805)/[#806](https://github.com/xy7365527-lang/NewChanlun/issues/806)/[#807](https://github.com/xy7365527-lang/NewChanlun/issues/807) 回报 + [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 走图对话，编排者当场确认）

**动机一字未改**（仍是「要不断复用重复这些模块，架构本身就应该重复」）。**改的是机制**——原措辞管得住「一层里别写两套」，管不住「再造一台完整的引擎」（`theta_v0` 另起 170506 行按老前提竟然合规）。

**订正后的三条：**

> **① 每一层都要求「一套代码、各级复用」——全层适用，无例外。**
> **② 跨级传什么，按能不能复制分：信息（级别身份、方向、信号、判定结果）必须传；资源（钱、仓位、额度）不跨级传，每级各有一套完整的账。**
> **③ 信息的跨级传有两个方向——往上合成、往下钻——它们是配对的，不是同一个。**

**② 的正读**（编排者确认的措辞）：这不是「禁止递归」，是**说明递归在资源侧长什么样**——不是把钱传下去，而是**每级各长一套一模一样的账**。那正是自相似。

**三条的来源分账**（照实）：① ③ 出自编排者原话（「要不断复用重复这些模块」/「缠论有区间套的递归，也有同级别分解」）；**② 由主控提出**，编排者经三步场景逐步确认：大级别方向传给次级别（`σ_u = −σ_{p(u)}`，**信息，传**）→ 次级别自算手数（**各级独立记账**）→ 执行层合成净敞口下单（`ffi.rs:283`）。编排者原话：**「的确，我自相似的意图就是这个意思。」**

**★ 拆分：自相似 ≠ 递归**（[#805](https://github.com/xy7365527-lang/NewChanlun/issues/805) 提出，[#807](https://github.com/xy7365527-lang/NewChanlun/issues/807) 坐实）。**自相似**＝每级长得一样；**递归**＝上一级的结果喂下一级。`LegPair` 只有前者（`Vec<LegPair>` 按 k 下标访问、零自调用，且 `prove_pair_isolation` **panic 守卫明令禁止跨级写**）。两条要求必须分开下达，混着下达就会把不该接的接起来。

**⟹ 旧边界判据「凡自相似者必模块化复用，凡单例者显式声明为单例」需精化**：二分漏掉了 `LegPair` 这一类——**每级一份、但不跨级传递**，既不是单例，也不是递归。新尺子（信息/资源）覆盖得住它，且**分界线不在「哪一层」**：操作层两样都有（赋格的方向传递是信息，仓位账是资源），规则分开下。**再次撞上本仓元判据：别拿外形当分类维度**——「操作层」是外形，「能不能复制」才承重。

**三张已关图为 ② 背书**（走图当场核，非事后附会）：[内在级别多视窗端到端 #106](https://github.com/xy7365527-lang/NewChanlun/issues/106) 裁「**链即身份**，严格链**逐级闭合到 L0**，缺/断环即拒」＝**信息跨级传，是多视窗的地基**；[按级别管理出场 #135](https://github.com/xy7365527-lang/NewChanlun/issues/135) 标题即「多级别仓位的**独立**出场机制」＝**资源各级独立**；[多空双开与反向操作对齐 #278](https://github.com/xy7365527-lang/NewChanlun/issues/278) + `CONTEXT.md:36` 赋格树 `σ_u = −σ_{p(u)}`（子声部方向由父给，代码字段 `depth`）＝**方向是信息，传**。

**★ 顺带解释了 [#791](https://github.com/xy7365527-lang/NewChanlun/issues/791) 的「兜圈子」**：#106→#126→#278→#529 四条线反复重开「级别归属该怎么判」，成因是**该词底下压着两个问题**——「这个买卖点属于哪一级」（信息，靠链谱系传）与「这笔仓位算哪一级的」（资源，各记各的）。四张图当成一个问题解，所以永远解不干净。

### 主词裁定：用「模块化」，不用「递归式统一」（2026-07-30 编排者裁定）

正本的主词是**架构模块化**，「递归自相似」降为它在缠论域内的**具体形态**：

> **架构模块化 ＝ 一个判断一个模块，每一级复用同一个。**（＝ 上文 ①）
> 在缠论域里，这些模块之间的关系是**递归**的——往上合成、往下钻，两个方向配对。（＝ ③）
> 在资源侧，模块照样复用（每级一套一模一样的账），**只是不传数据**。（＝ ②）

**为什么不以「递归」作主词**：① 资源侧本就不递归，拿「递归」当主词会让人以为那部分不归这条管；② 「模块化」更中性，不会让人只盯着「递归的东西」去复用。

**⚠️ 强制排除句（这个词在本仓有前科，必须带）**：

> **本图说的「模块化」不指文件组织与可见性收紧**（那已由 [端到端模块化 #743](https://github.com/xy7365527-lang/NewChanlun/issues/743) 做过一整张图、七票），**指判定逻辑只写一份、各级复用**。

成因：#743 整张图名为「端到端模块化」，七票干的全是代码组织层（注释订正、孤儿删除、env 键注册、文件瘦身）——**屋子收拾整齐了，重复的判断一条没合**。不带这句排除，第二次落空概率很高。

### 另起一套的退场条款（2026-07-30 编排者裁定 B）

**不禁止另起，但另起必须配退场条款**——写「我不复用旧的」可以，**必须同时写清「旧的什么时候死、谁负责杀」**。

成因（[#807](https://github.com/xy7365527-lang/NewChanlun/issues/807) 实证）：`theta_v0/mod.rs:13-18` 当年只写了前半句（「不复用旧 ladder，避免语义漂入」），**没有后半句**——于是 `recursive_t` 6-25 停止开发但**没死**，挂到今天 35 天，期间两次被核查判为「现役、不可删」。**为避免语义分歧而重写，制造了语义分歧**（[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 那 40 套口径的主要来源之一）。

**这类重复比「不知道已经有什么」更难治**——它有一份看起来很正当的辩护书挡在前面。

## Notes

### 起源：两个实测发现

本图不是从「想统一一下」起意的，是从两次当场验证起意的（2026-07-30 开图会话）：

1. **[#743 端到端模块化](https://github.com/xy7365527-lang/NewChanlun/issues/743) 关票时移交的三片雾无人接手**——关票 comment 写「身份键收敛、新旧证书链终局、目标模块图定稿三雾归 map #529 与后续新线」，但实查 [#529](https://github.com/xy7365527-lang/NewChanlun/issues/529) 雾段：「模块图」0 命中、「证书链」0 命中，「后续新线」也从未开。**架构整合把地基打平了，最有统一价值的三片雾却落地时悬空。**
2. **系统的主人对自己系统的状态判断出现反向偏差**——开图会话中，编排者判断「NT 生产路径没接进去线」；实测 `rust/src/theta_v0/nautilus/` **1569 行、`theta_v0/mod.rs:118` 无条件 `pub mod nautilus;` 进编译树、Cargo.toml 五个真 nautilus 依赖 v0.60.0、真实 `use nautilus_*` 在产**；模块头注释还专门订正过一次（#524：「原骨架状态段过期作废——三项自称均已为假」）。**订正发生了，但没有传到人这里。**

**根因判定（本图的立图前提）**：「每条线在兜圈子、重复造轮子」是症状；根因是**没有人（包括编排者）知道现在到底已经有什么**。不知道有什么，才会重复造；六条线各自朝一个方向走却不知道彼此在做同一件事。故本图第一批子票**全是 AFK 勘察**——不先问人要清单，先派探针测出清单做成图，人看着指认修正。这是本仓正本「答案已经在仓里、只是没被读出来 = 探针票（乙类 task），不是 grilling」的直接应用。

### 疆域

**在内**（从 K 线到下单这条链上、本仓自写的代码）：

| 区 | 内容 |
|---|---|
| `src/newchan` | Python 引擎（`docs/ROADMAP.md` 描述的五层递归管线） |
| `rust/src/*.rs` | 8 层逐位等价重写（bi/seg/zs/move/BSP/PH/MACD/Orchestrator） |
| `rust/src/theta_v0/` | classifier / backtest / strategy / **nautilus 适配层** |
| `formal/` | Lean 形式化链（149 文件） |
| `analysis/` | 脚本（658 文件） |
| `trading_system/` | `rec_t_strategy.py` 那套 |

**三层推导链**（本图要接通的东西）：

- **甲 教义层**：五词之间的完整关系（`AGENTS.md` / [SPEC #756](https://github.com/xy7365527-lang/NewChanlun/issues/756) 已立一段话；更深的「统一递归算子 T / 每级别一个 T 实例」是否要重裁，属本图待裁）
- **乙 代码层**：甲在代码里各就其位的形状——身份键、目标模块图、证书链归属（= #743 移交失败那三片雾）
- **丙 组织层**：乙决定线怎么切、一条线管到哪、新线开在哪

甲有了但立得简略，乙只做了一半（#743 做完接口层就关），丙从来没按乙切过——所以每开一条新线都要靠人在脑子里重推一遍甲到丙，推不动就各兜各的圈子。

### 两行声明

- **本图不带实装**（无实装声明 = 纯决策图，甲类纯实装票不准挂）。真改行为的代码去后续实施链。**两个合法例外**（正本明写不算实装）：乙类探针票会写只读测量代码；裁定票 resolution 的落文档可含零行为变更注释。
- **本图寿命预期：一次寻路图，正本落盘即关**（🗺️）。正本本身长期有效，但图不长驻。

### 排序原则：打扫干净屋子再请客（编排者 2026-07-30 定）

**统一优先于接实盘。** 屋子没扫干净之前不请客——实盘那一段（`LiveNode` 空缺、theta_v0 零 PyO3 出口、`trading_system/live/` 唯一能下单的脚本不消费缠论信号）**排在统一之后**，不与之并行。

这条同时是对「回测通了是不是好消息」的裁断：[#794](https://github.com/xy7365527-lang/NewChanlun/issues/794) 实测那条通路内部自带中枢口径分叉、区间套零执行、一类买卖点恒 0，**管道通不代表流的东西对**。编排者原话：「代码根本没统一，有回测也没用。」

### 雾的四类处置（2026-07-30 第一批勘察收官后归类）

本图的雾按**处置方式**分四类，不按主题分——避免看成一团：

| 类 | 内容 | 处置 |
|---|---|---|
| **A** | 正本载体 / 同名异物 / 收敛的单位与次序 | 已在 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 票面，一裁即解 |
| **B** | 甲层收敛的具体标的（中枢分叉、区间套零执行、背驰五力度、笔/线段/走势类型多口径） | **不预切** —— #793 定完方法后按方法切票 |
| **C** | 实装类（CI 不跑 `cargo test`、测试守链外代码、实盘段空缺） | **本图碰不了**（无实装声明）：走 triage 挂 `debt` 或排进正本落地后的实施链。**不堵本图的路** |
| **D** | 乙层目标模块图 / 丙层线的切法 | 依赖甲层收敛结果，排在其后 |

**收敛路径**：#793 定方法 → 切 B 类裁定票 → 逐张裁（一票一会话）→ 甲层收敛 → 乙层模块图 → 丙层线归位 → 正本落盘 → 关图。C 类平行走。

### 每会话必读

1. `docs/agents/wayfinder-workflow.md`（本仓 wayfinder 唯一正本）
2. `docs/agents/issue-tracker.md`「Wayfinding operations」段
3. `AGENTS.md`（#517 定的全 harness 唯一正本入口）
4. `docs/agents/delivery-discipline.md`（本图勘察票碰代码，按域追加）
5. `docs/ROADMAP.md`（四大支柱与五里程碑；注意其引擎描述指向 Python 那套，与现役 rust 链的关系正是本图待测项）

### 纪律

- **勘察票一律 AFK**，写报告故强制开 worktree（`Agent` 工具 `isolation: "worktree"`）；只读子代理不强制。
- **裁定票严格一票一会话、一次一问**，agent 不得替人答。
- 报告落 `.chanlun/review-results/`，入仓引用按 `delivery-discipline.md` 关票门第 6 子句。
- 090 照实：测不出来就写测不出来，**不许把「查不到」写成「不受影响」**。

## Decisions so far
- [grilling：总缝规则——同一判定每级必须是同一个](https://github.com/xy7365527-lang/NewChanlun/issues/804) — **三条裁定；范围因 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 已关当场缩掉一半**（原④项中「适用域」改引 #799 裁定六七、「机械锁载体」改引其裁定十一）。①**规则落在语义不落在形状**——判别式＝把两级输入换成同一份数据、输出须逐位相同；否掉形状口径的决定性理由是**实测四个病灶只有 `is_l0` 一个能被 grep 命中，而它恰恰最易伪装**（改写成 `TABLE[level]` 即可让 grep 归零而分歧一点没变）；grep 就此**降级为分诊线索非判据**；机械锁属「行为不变式」⟹ 载体只能是对拍测试锁，**落地前置＝先让 CI 跑 `cargo test`**。②**举证门＝例外可以有但必须自带退场条件**：须同时满足「指得出实质（缺该维度＋权威源背书）／写得出退场条件（答不出即不算数）／不超 #799 裁定二的全仓 3 条上限」；否掉全认（辩护太好补，规则自废）与全否（砍它要动数据类型、成本从未估过、且 Lean 侧上级本就用外缘判据）。③**举证书载体随概念走**——正本进 `.chanlun/definitions/<概念>.md` 例外条款段，由该概念的裁定票点头，**不为每处例外单开票、不另建总清单**（代价：清单是散的）。**★ 走图当场查实并推翻票面前提**：「同一判定吃不同输入类型」这条合法性辩护在本仓**不成立**——`center_from_segments`（`center.rs:210`）与 `center_from_window`（`:252`）**签名逐字相同**，只差一句 `dir_alternates`，`compose_level:304`（`:859` 同款）以 `if is_l0` 分派，**与 `div_cand`/`sublevel_diverges` 是同一形状**、正踩 #799 判别式。**⟹ 当场判出第一条不合规**：其「诚实有效域」举证书（`center.rs:235`）**理由够格但缺退场条件、且载体在注释非定义文件**，须由第二批中枢概念票补齐；收敛到哪一套本票不裁。落文档：`AGENTS.md` 新增「总缝规则」节，commit `dac8ff9fa1`，零行为变更。剩余分歧面清点已开成 [#809](https://github.com/xy7365527-lang/NewChanlun/issues/809)
- [task 探针：recursive_t 的真实成因核查](https://github.com/xy7365527-lang/NewChanlun/issues/807) — **编排者的回忆属实，但两台引擎记反了：互斥定义策略带出的是 `theta_v0`，不是 `recursive_t`。**决定性证据＝`theta_v0/mod.rs:13-18` 源码自陈（首 commit `cc108f6c53` 06-25）：「`recursive_t`/`spiral`/`fugue_v3` 是**旧 ladder**，本模块是**独立的新引擎**，**不复用旧 ladder**（**避免语义漂入**）」。①**互斥全定义策略确实存在且确为数学推导**＝`formal/Origin/MutexFinalTheorem.lean:2` M29 + `MutexExhaustive`/`MutexRecursive`/`OperationRole18`，全链 **06-28**，同日实装落 `theta_v0/strategy/`；确在策略/操作层（P1..P10 全动作谓词）；限定：`mutex.rs:144` 自陈「互斥性是定义内蕴的**同义反复**」＝**动作语法非 alpha 发现**。②**时序否决**：`recursive_t` 生于 06-18，早 10 天；其内 14 处「互斥」无一指向该策略、零 import `OperationRole`；自陈权威源是**第 65 课 aₙ=f(aₙ₋₁)**且明写「**不**含操作/仓位」。③**「操作层不是重复」不成立**——`theta_v0/strategy/` **44663 行**与 `recursive_t` 操作层逐项对应。**★ 订正 [#761](https://github.com/xy7365527-lang/NewChanlun/issues/761)/[#762](https://github.com/xy7365527-lang/NewChanlun/issues/762)**：2611/12062 是「standalone T 算子 / T 引擎」**不是**信号层/操作层（`rec_stream.rs:23` 自己 `use super::iterate`）——**[#800](https://github.com/xy7365527-lang/NewChanlun/issues/800)/[#805](https://github.com/xy7365527-lang/NewChanlun/issues/805) 沿用的那个切分是误标。**④**与 #800 并存，是同一次换轨的两半**：**06-25 同日**发生 `theta_v0` 诞生 / `recursive_t` 最后功能 commit / 预注册协议落地；#800 答「为什么停」，本票补「活去哪了、为什么不在原地改」。⑤**订正 [#789](https://github.com/xy7365527-lang/NewChanlun/issues/789)**：「17383 行是最大重复」**不成立**——`theta_v0` 自身 **170506 行**才是最大的一处再实现；**但那处有书面裁定与理由＝「合理重复」**，不属本图「不知道已经有什么」的病灶。⑥**★ 多空双开不是第三种级别关系**——`rec_engine.rs:2156-2172` `prove_pair_isolation` **panic 守卫明文禁止跨级写**；唯一跨级耦合是 `highest_active_long()` **标量 argmax** + 序比较 `k<核心`，**零数据流动**。独立背书 `chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md` §9.1 逐字「互斥……**它从来不是跨级别约束**……执行层＝净额合成」。⟹ **#805「LegPair 是复制不是递归」的更强佐证：不是碰巧没耦合，是被守卫禁止耦合。****追加三件**：⑦**口径有过一次方向反转**——06-12 `bidirectional_nested_fugue_research.md`**物理双开被否决**+净额等价定理（m≤Q 时双开≡trim，**alpha 增量恒 0**）→ 06-14 `slow_bull_vs_bh_research.md:341` 逐字「就是当前 trim 系统」→ 06-23 LegPair 实装 + Face A 默认开启。⑧**现行口径**＝`face_a()`；**`rec_engine.rs:771` 那条注释是「不完整」不是「过期」**（写于 `5eb02e9685` 后一字未改，比生产默认少两层：核心多腿会翻空、次级别空腿受 `k<核心` 硬门且实测 `short_pnl +9623→−106256` 明确不开）；**执行层从未物理双开**（`ffi.rs:283` 输出 `long−short` 净敞口，NT NETTING）；「吃所有级别涨跌幅」那句与 06-14 在册 L0 定理表面冲突，**是否对过账＝【未找到】**。⑨**与既有图无关**——#106/#529/#695/#737/#743 全部创建于 07-21 之后，晚互斥链 23–32 天。**⚠️ 编号陷阱（净发现）**：`recursive_t` 里的 `#57/#69/#106/#110/#113/#164/#547` 是**当年 kimi 任务号/谱系号**，与当前 GitHub issue **号段碰撞**（`rec_engine.rs:417`「mid-scale #106」与 GH #106 **毫无关系**）——**读 2026-06 期注释时任何 `#nnn` 都不得当 issue 解。**未跑编译/测试/回测。`research/i807-recursive-t-genesis-mutex` @ `59ba13ce6d`
- [research：外部方法论的适用性重估](https://github.com/xy7365527-lang/NewChanlun/issues/806) — **[#801](https://github.com/xy7365527-lang/NewChanlun/issues/801) 核心结论维持（无候选改判为该引机制），但其靶子只覆盖上行一层，且 `formal/` 是整块盲区。**C1 的主否决理由「不在节点轴」**直接为假**（下行 `descend.rs:115` 四行 μF 析构子 + `econ_positive.rs:815-843` 真自调用**就是节点轴**），**但结论仍维持**、理由第二次更换。纵向对口的不是 ana/apo 而是 **hylo**，不引因 Rust 生态无 apo/futu 实装 + **核心卖点仓内 Lean 已有更强版本**（`WellFoundedRank.rank_acc` 明文免 fuel）。回溯侧（`extend_typed_upward` 五平行栈手工 pop）#801 零考察，补 LogicT/分支限界仍判不引，但记一条实账：**五栈同步已漏过一次**（`nest.rs:1005` 自陈 p118 关④断链点）。算子 N 侧 #801 **无一对口＝缺项非判错**，补出分区精化（Hopcroft/Paige-Tarjan，与「配置→角→板块」最贴）等——**据此弱化 [#805](https://github.com/xy7365527-lang/NewChanlun/issues/805) 的 D6**：事实维持但「因此拿不到强判据」撤回（**有限全集上 ⊆ 本身即良基偏序**），真正缺的是 D1 步函数不存在。**★★ 最大改判：验收判据应是三档不是两档——Lean 定理 > 测试锁 > grep gate。** `formal/` 六项装置在案：`WellFoundedRank.lean`（良基秩+严格递减+`Acc` 终止，零 sorry 零 axiom）、`RMoveCompose.lean:123 descend_composeMove`（**对偶律 `rfl` 机器证明**）、**`RecursiveLevelSystem.lean` + `TrendCompleteClassification.lean:137`（「一个算子跑所有级别」的抽象接口本身就在仓里，带缠论实例与自相似律）**、`RStarNonSpecial.lean`（`operate_blind_to_topness` ＝ **#805「第四条」的机器证明版，#805 那句应订正**）、`SubLevelDescent.lean`、`RecursiveConstruction.lean:160 WellFormed`（含「≥3 段」，五条判据没有）；Rust/CI 八项含 `nest_isolation_guard.rs`（**比 #801 提议的 grep gate 成熟一代**）、`ci.yml:36` G1 grep gate 先例。**一二档的锁都已打好、没接进任何 gate ＝ 本图「没人知道已经有什么」的第三个标本。**另：五条验收判据**只覆盖 L-构造一层**，下行侧与无序侧各缺一套；载体缺口 `src/newchan/core/recursion/recursive_stack.py`（第四个 L-构造载体 + 第六种终止界）#801/#805 均未提；**对偶记号订正**——`compose ⊣ descend` 偏强，单边 `descend∘compose=id` 不足以 establish 伴随，精确名分是 optics 的 **`Prism`**（section-retraction）。`research/i806-external-methodology-reassessment` @ `144c0865b4`
- [research：三处递归是不是同一个逻辑](https://github.com/xy7365527-lang/NewChanlun/issues/805) — **三者不同构，主控「一个算子三实例」初判推翻；编排者「不一样」成立但归因（因为层不同）双向证伪。**实测要划**四层**（尺子＝这一层的「空间」是什么类型的对象）：L-构造（有序单元序列，`apply_t`）/ L-判定（`RMove` 子树，`descend`）/ L-操作（`[T; MAX_LEVEL]` 的**下标**，`sink`）/ L-选标的（**无序** `frozenset[str]`，算子 N）。找到 **5 个**「一步」不是 3 个。**三个本质卡点**：D2 **方向相反且「变小」是两种关系**（上行＝**商** / 下行＝**子对象嵌入**；仓内已有名分 `descend ∘ compose = id`——**是对偶不是同一个**）；D3 输出的「确认」是 **discover/verify/select 三种东西**；D6 **无序集合上没有比基数更强的收缩序**。⟹ 能落地的最强统一 ＝ **两个对偶算子（compose ⊣ descend）+ 一个待补步函数**。**★ 算子 N 只有不变量没有步函数**（声明的步函数全仓不存在，`NestingPath` 零生产调用者；唯一生产载体 `horizontal.advance` 既无 level 也无 search_space）。**★ 真判别变量＝方向 + 级别是不是递归参数**，不是层——反例 A：`apply_t` 与 `descend` 同层而方向/空间关系/输出全相反；反例 B：操作层 `sink` 有 `prove_sink_descends` 守卫（panic 文案逐字「区间套要求 sub<parent 向心下沉」）**与 `descend` 同形**。**★ 订正 [#803](https://github.com/xy7365527-lang/NewChanlun/issues/803) 一处读法**：`LegPair` 是**复制**不是**递归**（`Vec<LegPair>` 按 k 下标访问、零自调用）——**自相似 ≠ 递归**。**★ 算子 N 三条 check 不通用**：`check_level_monotonicity` 在 (a) 上**反向**、对每条合法塔返回 False，`check_termination` 在 (a) 上为假——**这是不同构的独立佐证**；跨 (a)(b) 真通用的是三条里没写的第四条＝**级别差异只进数据不进控制流**（即 [#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 正在裁的那条）。未做（090）：未跑编译/测试；`nest_lifecycle.rs` 6386 行只据模块头判定未逐函数核。`research/i805-three-recursions-isomorphism` @ `1f529685e2`
- [task 探针：区间套的纵向下钻是不是真递归](https://github.com/xy7365527-lang/NewChanlun/issues/802) — **总判：机制层是真递归，消费层退化。**强假说（纵向没实现成递归）**推翻**；弱形式（入口查一次、下面全放行）对 π 门 Type2/3 rung 链**逐字成立**。①rung 区间**逐级真取**（各级 `partition_point` 找含点段 `:1030-1064`），非抄上级；但判定分叉——Type1 每级真跑 `div_cand`，**Type2/3 字面 `true`（`:961`）**，且 `is_sub` 由塔不变量结构性恒成立 ⟹ rung 链**零判别力**。**仓内注释早已实测在案（`:361-364`）：BTC 300K 中 95.36% 通过门信号 rungs 空、仅 4.64% 真跨级且 max 深度=1**，与 [#796](https://github.com/xy7365527-lang/NewChanlun/issues/796) 真链 `chain_pass=1/52` 同向。②**下钻方向用了嵌套结构**（`descend_type1_anchor_depth` 走 `RMove::Compose.subs` 的携坐标侧车，带测试锁的真自递归）；上行 rung 链**确用塔索引**——「横向表示做纵向事」属实但**无害**（bottom-up 真嵌套对照 BTC 三窗 **bit-exact 差异 0**）。③第 17 课 L60 **只撑基例存在性、撑不了逐级免判**（Type3 援引错位），但恒真有 673-fix + [#97](https://github.com/xy7365527-lang/NewChanlun/issues/97) D1 两道裁决背书，与 P2 已结算的 `Cand^δ` 定义式**三道共存，待 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 收敛**。**★ 真空洞在消费侧三处：深度全弃（生产只吃 `is_some()`）、`lvl==0` 免门、Type1 从不下钻。****与 [#737](https://github.com/xy7365527-lang/NewChanlun/issues/737) 是两个机制**（其 78% 属链臂基例缺证书），**不改判**。`probe/i802-nest-descend-recursion` @ `37ed2a8b10`
- [task 探针：三阶段是单例还是自相似](https://github.com/xy7365527-lang/NewChanlun/issues/803) — **反假说被否证，但否证方式指出了真答案。**当年「每级独立自我复制」（`aa45a76d1f` 06-18 23:33）**复制的是流程不是钱包**——三阶段会计 39 小时后才落地（`5a0ad774f8` 06-20 14:16），彼时仓内根本没有三阶段；现金池 `free` 恒单数，「每实例独立池」在 `architecture_v2.md:432` §8.7 被编排者显式否决从未落码。**`sink=0` 与资金无关**（`sink()` 零现金门、配额取自父级持仓股数、断点是 `candidate@core=0`；最硬一条：`sink=0` 实测时 per-instance 三阶段正在生效，时序自证）。**[#800](https://github.com/xy7365527-lang/NewChanlun/issues/800) 结论层维持**：三阶段单例是**原文裁定非工程妥协**（第 31 课「三阶段总体+成本穿 0 全局门」、chan99 `0033:11`「**级别只决定量**」）。**★ 净发现：真正每级一份、真正自相似的「仓位生命周期」在仓里确实存在——载体是 `LegPair` + `leg_open_units`（`rec_engine.rs:149`/`1026`/`2002`），不是三阶段；把二者当同一个东西是反假说的偷换点。**⟹ [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 重心从「拆不拆资金池」（已是拆开的）移到「**已经拆出来的 `LegPair` 为何没人当它是答案**」。`probe/i803-three-stage-singleton` @ `396d0a0473`
- [research：递归自相似模块化方法论](https://github.com/xy7365527-lang/NewChanlun/issues/801) — **外部没有成名的「递归自相似模块化」方法论（照实）；对口的那套就在仓里。**首选 = `recursive_t` 信号层的朴素形态（纯函数 + 判定作参数 + **级别差异进数据** + 测试 gate，**零 trait 零泛型零宏**）。**★ 实测 `theta_v0` 已有 90% 同形态**——`compose_level` 已把中枢构造函数当值传入、`divergence`/`decompose`/`bsp` 零 level 分叉、`RMove` 已是 μF；**只缺三件收口**：① `is_l0` 分叉收敛到唯一缝（现外泄 `classifier/mod.rs` **24 处**，即 [#789](https://github.com/xy7365527-lang/NewChanlun/issues/789)「同塔 L0 与 L≥1 中枢定义不同」的机制源头）② 实验箱禁止重抄口径（`theta_v0/backtest/wverify_run.rs:2944` `c327_weak_*` 一族**即 40 套口径的生成机制**）③ 补「同码全级」回归测试。**recursion schemes 机制判为不适用**——本仓递归在**级别轴不在节点轴**，连 cata 基座都不对形（中枢横向三窗连 paramorphism 都不覆盖）；tagless final / visitor / DDD 同判不适用，APoSD **证实只覆盖半个问题**，MRA 只借三条纪律无新架构。**「取思想不取机制」成立**：机械保证的载体可以是**测试 gate 而非类型系统**。⟹ **[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 该拍的不是「选哪套」，是「把样板的机械锁补进 `theta_v0` 并立为 gate」**（报告 §6 给五条可机械检查的验收判据）。**适用域：仅结构判定层；操作层有 [#800](https://github.com/xy7365527-lang/NewChanlun/issues/800) 的 L3 否定铁证，须另裁。**`research/i801-recursive-modularity-methodology` @ `a3e5806cca`
- [task 探针：recursive_t 失败复盘](https://github.com/xy7365527-lang/NewChanlun/issues/800) — **本仓已有一个做成了的递归自相似样板，且用的是朴素手段。**`recursive_t` 两层结论相反：**信号层（2611 行）四条模块化判据全过且做到自相似复用**——纯函数 `apply_t(units, level, mode)`、判定与调度分离（`iterate` 28 行只做循环）、**级别是参数不是分支**（对 level 的比较 grep 命中 0）、可脱引擎单测，`operator.rs:198` 有回归测试锁「所有级别用同一套代码」，**无 F-代数无 HKT**；**操作层（12062 行）不模块化**（零 trait 零泛型、22 开关 + 24 处 env::var、`push_bar` 413 行 63 分支），其真障碍非架构表达力而是「三阶段资金战役本质单例」。**卡点定案：验收语法换轨，非技术失败**——2026-06-25 预注册协议落地（看结果前冻结/事后判据无效）与其「改码→看收益→再改」的开发方式不相容，同日最后一个功能 commit 后停摆、5 周零演进；判据不对/性能/接不进生产三项均被反证（已接入 NT）。**与编排者「不成熟」定性有出入，照实记录不调和**：准确说法 = 执行到位、验收语法过时。**对 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 的决定性含义：落在矩阵两格**，首选候选应是「移植本仓已有样板」而非引入外部方法论。五概念对照：两套引擎无一同名同义，第二类买卖点定义直接冲突（代数结构相反），去向 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793)。`probe/i800-recursive-t-postmortem` @ `4c8458f20c`

<!-- 已决票索引：一行一票，gist + 链接 -->

- [task：认知对账](https://github.com/xy7365527-lang/NewChanlun/issues/788) — 以为没做其实做了 4 / 反向 1；**#524 订正只做了一半**（`nautilus/` 四文件头注释至今写「骨架」，`mod.rs` 对其零 cfg 门控、一直在编译）＝ 本图立图起源的直接来源；ROADMAP 支柱 IV 称风控/执行器未实现，实为 risk.rs 2230 行 + exec.rs 492 行；报告 `55b0562feb`
- [task：疆域测绘](https://github.com/xy7365527-lang/NewChanlun/issues/790) — 六区 + 10 边（活 8 / 死候选 1），零环零双向边；**最重信号是两处「该有边却没有」**：theta_v0（17 万行）→Rust nautilus→CLI 与 trading_system→Python nautilus→实盘壳**互不相通**（theta_v0 零 PyO3 出口，`lib.rs:76` 自陈）；`src/newchan` ↔ rust 顶层散件仅文档级「移植血缘」零代码依赖；报告 `55b0562feb`
- [task：线的终点与重叠矩阵](https://github.com/xy7365527-lang/NewChanlun/issues/791) — **簇 C：#106→#126→#278→#529 四条线反复重开「级别归属该怎么判」＝「兜圈子」第一份实证**；簇 A：#106 身份桥 vs #529 TowerChainCertificate 两套跨级确认链**并存生产**、退役时点未定；残雾失落 #743→#529 三片维持判定，#654→#529 复核后撤回（已落 `.chanlun/definitions/beichi.md:298` 只是没回写 issue）；报告 `0181e9f824`
- [task：重复实现普查](https://github.com/xy7365527-lang/NewChanlun/issues/789) — **甲层裁决：必须重裁**。70+ 实体 / **约 40 套独立教义口径**，七概念全部有教义层分歧（20 条带理由）。最烂＝区间套（4 坐标系 + 4 停止级 + `Cand^δ_ℓ` 源码自陈无定义 + **零对拍**）；中枢 Rust `<` / Lean `≤` 且 L0 与 L≥1 定义不同；背驰五种不相容力度。**CI `cargo test` 零命中**⟹34 个 parity 断言只编译不执行、对拍全靠合成正弦数据。正面样板两条（相切边界一次统一三侧 / 三类点判据从未分歧）证分歧可裁。报告 `b7eaff1097`
- [task：端到端跑法实测](https://github.com/xy7365527-lang/NewChanlun/issues/792) — 初报「零条端到端 + 缺数据」**两条均订正**（数据 314MB 在 gitignore 内实存；首测跑了全量未用日期窗）。补测 BTC 一周窗**跑通并贯通真实 Nautilus BacktestEngine**（orders 2754 / positions 177 / PnL −23.06% / L2 生产引擎贯通验证通过）。硬缺口＝实盘段：`LiveNode` 全仓 3 处**全为注释**、零代码使用。**新捞：两套口径数值分叉未解释**（in-crate −10.25%/胜率 .2752 vs nautilus −23.06%/胜率 .01）；报告 `b7eaff1097`

- [task：回测链口径归属普查](https://github.com/xy7365527-lang/NewChanlun/issues/794) — **那个 −23% 内部自带中枢口径分叉且零裁定**：同一座塔 L0 走 `center_from_segments`（含方向交替，9700 次）、L1/L2 走 `center_from_window`（**不看方向**，8956 次），上层吃下层产物。**区间套一次没跑**（env `THETA_NEST_CERT_GATE` 门关）。七概念**全部只吃 `theta_v0/`**，而 CI 唯一在跑的九组对拍守的是 `rust/src/*.rs` 顶层散件 ↔ Python——**测试守的是这条链一行不走的代码**。一类买卖点决策恒 0、`depth>0` 嵌套产出恒 0、2754 单全多。插桩实测，前后 −23.06% 逐位不变；报告 `5c80ab3773`

- [task：22 张图裁定的落点核对](https://github.com/xy7365527-lang/NewChanlun/issues/795) — 在链上 8 / **在旁支 5** / 不在树 8（多为流程文档图，归宿正确）/ 查不到 1。**核心：旁支 5 条非各自跑偏，是集中堵在两个卡口**——①区间套证书门关闭（#106/#126 落 `admission.rs` 受其门控）②N7 消费点未接（#529 的 `chain_cert::` 全部使用点在 `#[cfg(test)]` 内，生产链零消费，主控验证）。**工作都在、都编译、都有单测，缺的是合闸。** 另：#379/#566 在链上但 `runner.rs:728` 自陈 EarningShares 终态生产从不进入（GAP3 老账再撞）。加测：近两天 46 个实质 commit 中 147 处改动落 `theta_v0`（84%）；报告 `820fff69b7`

- [task：开区间套证书门跑一次](https://github.com/xy7365527-lang/NewChanlun/issues/796) — **通电了但通的不是那四条线**。⑤ 段订单 3136→1794、亏损 −10.25%→−3.22%，跑通无 panic；**但腰斩是 L2 旧臂 Xzd 干的**（52 候选中真链仅 Pass 1，51 个走 NoChain 回退）。**★typed 真链证书索引造出来是空的**（`index_builds=5` 而 `assembled=0/indexed=0`）＝ [#737](https://github.com/xy7365527-lang/NewChanlun/issues/737) 的题目带读数撞上。**★第四个卡口**：⑥ 段真实 nautilus 引擎**一次没查过这个门**（`nest_cert_gate_enabled()` 是 `pub(super)`，`nautilus/` 够不着，主控验证）。#597 零触达、#529 卡口②独立未解。**⟹「先收敛还是先开门」两难当场解决：光开门没用。** 报告 `5984ab0d8e`

- [grilling：教义收敛的方法与次序](https://github.com/xy7365527-lang/NewChanlun/issues/793) — **四裁全出，另加两条当场订正。**①**收敛单位＝先裁总缝再逐概念**，第一批只开一张（[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804)），七张概念票不预切——靶子已是「设计一套能每级复用的」而非投票，40 套里一大类是**同一个毛病**。②**权威分层不排名次**：原文管语义 / **Lean 管边界**（原文是讲课稿，端点根本没处理）/ 生产代码**只有否决权**（可用实测读数打回，不得以「现在就这么跑」为由支持）⟹ **[#321](https://github.com/xy7365527-lang/NewChanlun/issues/321) 那次是裁错了层**，不只是跟进不及时。③**关票判据＝文字正本 + 受影响代码清单**（点名到行号）；**「注释里登记口径分歧」不再算合规收尾**，注释必须带票号（实测仅 1 处孤例 `center.rs:13`，恰证无机制会清它）。④**载体＝`.chanlun/definitions/` 补名分**——**当场撞出：15 份 4295 行定义文件而 `AGENTS.md` 零提及**（`:23` 指的是 `CONTEXT.md`+`docs/adr/`），照正本入口进场的人根本读不到＝**立图那个病的又一标本**。**★⑤ 订正 [#789](https://github.com/xy7365527-lang/NewChanlun/issues/789)「同名异物」误判**：`src/newchan/nesting/` 实为编排者自写的**统一区间套算子 N**（纵向级别下钻 / 横向搜索空间收缩是其两个实例，源 `[新缠论] levels-bsp-v2`）——**第三个递归方向，且仓里已有人试着统一过两个**，「没人知道已经有什么」第三次现形。**★⑥ 登记（非本票裁定）**：编排者表态**单点核心必须有宽度**＝严格 `<`＝Rust 现状正确，为中枢概念票既定前提；按分层要动的是 **Lean `≤`→`<`**，代价是依赖弱不等式证过的 Lean 定理可能要重证。落文档：`AGENTS.md` 新增「缠论教义正本」节（②③④）

- [grilling：递归式统一怎么落地](https://github.com/xy7365527-lang/NewChanlun/issues/799) — **十一条裁定，四项剩余全清。**⑥**例外＝先划管辖再配退场条款**：模块化只管**生产判定路径**，对照臂/观测器属「**不适用**」非「例外」⟹ 前提①「无例外」一字不改，ADR-0005 不冲突。⑦**管辖载体＝目录即层，重开 ADR-0005 搬家案**——当年「否于成本而非原理」，而本图目标本就是模块化，搬家是顺路；ADR 自陈「唯一根治法是把 nest 三件搬出判据目录」。搬完 `GUARD-ROLE` 自述豁免可整个拆掉（连带解「自述可骗」+「一记号两用」）。⑧**抓手③＝一半有意一半遗漏**：`build_nest_certificate` 有 ADR 明文（有意）／`nest_cert_gate_enabled` 零书面裁定、`pub(super)` 是可见性副产物（遗漏），二者只是恰好同修饰符——**门属于哪一层从来没人裁过**，为⑦背书。**★⑨→⑩ 走图中途被编排者的教义记忆推翻主控原判**：原判「两条并列通道互为退路」，经「小转大也需要区间套」+ 核实**改判为「同一判断的严格版/宽松版」**——`div_cand`（四条件含 **Extreme**）与 `sublevel_diverges`（**无 Extreme**，力度原语同族）是同一个次级别背驰判定的两份实现，`econ_positive.rs:1687` 的 C1 规定前者失败（`descend anchor None`）才走后者 ⟹ **[#796](https://github.com/xy7365527-lang/NewChanlun/issues/796) 的 51/52 有了解释：51 次都是严格版降级到宽松版放行的**。裁：C1 违规 + 两份须收成一份 + 收到哪套归背驰概念票。⑪**验收三档按对象性质分派**（结构性质→Lean／行为不变式→测试锁／禁令→grep gate），每条判据须声明属哪类；不取「能上高档必须上高档」（成本失控）、不取「三档等价」（全躺 grep，而 **grep 抓不到另写一份同义代码**——两套背驰零共同标识符）。**★净发现：CI 根本不跑 `cargo test`**（三 job＝pytest+一条 grep gate／`cargo check`+`fmt`／fixture-drift）⟹ `nest_isolation_guard.rs` 与 34 个 parity 断言**只编译从不执行**，[#806](https://github.com/xy7365527-lang/NewChanlun/issues/806)「十四件锁没接进 gate」成因定死；立原则「**判据必须真被 CI 执行才算数，只编译不执行按无锁计**」。**★第三个「答案已在仓里没被读出来」标本**：[#166](https://github.com/xy7365527-lang/NewChanlun/issues/166) 早在 07-23 就查到 C2 内嵌次级别背驰，但它喂的是 [#134](https://github.com/xy7365527-lang/NewChanlun/issues/134)，没人把它与区间套判据摆一起比。落文档：`AGENTS.md` 教义正本节新增「**收敛通则：同一判断不得有宽严两档**」（含三条限定词 + 判别式「看两套判据是不是接在同一个 if-else 上」），commit `9dc71234fd`。未估搬家成本、未裁 Extreme、未跑编译/测试/回测。

- [task：recursive_t 的退场条款](https://github.com/xy7365527-lang/NewChanlun/issues/808) — **供电清单：生产 0 个**（Rust 侧零内部调用；Python 6 直接+3 间接＝回测 3 文件 2 链 / 一次性研究 9 文件 / 已死 1 导出 `run_t_fugue` 零调用）。**`rec_t_strategy.py` 不在生产路径**四条举证（唯一 importer 是回测 CLI；`live/runner.py:32` 装配全在 docstring、`:61` print「阶段4骨架」`:69` return 1；`:57` 点名 `ChanlunStrategy` 非 `RecTStrategy`；唯一能下单的 `hl_verify_nautilus.py` 零 `newchan_rust`）——与 [#792](https://github.com/xy7365527-lang/NewChanlun/issues/792) 一致，**订正 [#762](https://github.com/xy7365527-lang/NewChanlun/issues/762) 名分表「NT 生产策略」措辞**（不影响现役判定）。**★现在就能接＝0 项**：`theta_v0` **零 PyO3**（主控复核 `grep -rc pyo3` 零命中），消费者 100% 是 Python 进程 ⟹ **[#807](https://github.com/xy7365527-lang/NewChanlun/issues/807) 的「44663 行逐项对应」是语义层对应、不是接口层可替换性——对应了也接不上**；接不了 4 项举证到字段（`push_bar` 净敞口 `lu-su` vs `Order`/`StrictAction` 契约、`finish_full()` 九字段零命中、`mode`/`a0` 两自变量零命中、对照另一方是 `fugue_v3`），没人要 4 项。**条款**：①死于「消费者一个不剩」不死于「theta_v0 更好」（D1 继任者 PyO3 出口现 0/2；兜底挂**本图结项复核**不挂日历）②**执刀＝写 `theta_v0` PyO3 出口的那张票，建新与杀旧不得分票分人，该票为本图关图前置**③冻结、只修编译不修逻辑、**且跨切面口径变更不同步**。**★④ 模板（对 destination 的交付）**：「另起一套」声明＝**立新/判死/执刀/看守/兜底**四句，缺一不成立；判死只能写在旧的**消费者**上（「等 Y 成熟」不是判据），执刀默认＝写 Y 的人，**不连写「口径不同步」的冻结是假冻结**，兜底挂仓内事件；判别式「三个月后有人问『旧的还要不要跟着改』，答案是否已写在纸上」；挂靠世代宪法**加一列「退场条款：`<票号>`」不新增第六态**，该列为空 ⟹ 声明不成立。**★净发现：「停止开发」≠「零维护」**——35 天吃了 **3 次跨切面口径税且是逻辑层**（`36092eab8e` #322 中枢延伸谓词 `center.rs` +214/−4、`3186bd104d` #338 +38/−4、`77040317f2` #288 相切=重合 `ffi.rs` +6，主控逐个复核确实碰 `recursive_t/`）⟹ 没写后半句的账单不是「留着不管」，是**每次教义收敛都要多改一个实现多做一次评审**；条款③「不同步」正由此推出。主控 5 条抽核全过。未跑编译/测试；「没人要了」全是【推断】；模板未合并进 `generation-constitution.md`（归正本落盘时做）。`research/i808-recursive-t-exit-clause` @ `c84e42f64e`

## Not yet specified

<!-- 2026-07-30 第一批勘察（#788-#792）回报后重写。甲层那条已毕业成 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793)，此处清除。 -->

- **乙层三件的形态**（身份键正本 / 目标模块图 / 证书链归属）——#743 移交失落那三片。方法已由 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 定下（先裁总缝再逐概念 / 权威分层 / 正本落 `.chanlun/definitions/`），**总缝票 [#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 已裁完（2026-07-30），依赖顺移到 [清点票 #809](https://github.com/xy7365527-lang/NewChanlun/issues/809)**：模块图的划分依据是甲层口径，剩余分歧面没清点完就不知道要画几块，画不出。
- **丙层的切法**——线按什么切、一条线管到哪、新线开在哪；既有六条线（#529/#737/#695/#485/#620/反向根）怎么归位或合并。依赖乙。**已有两个具体待处置**：簇 A 两套跨级确认链（#106 身份桥 vs #529 TowerChainCertificate）并存生产、退役时点未定；簇 C 四条线反复重开「级别归属」。
- ~~两套 fill 口径数值分叉~~（**已由 [#794](https://github.com/xy7365527-lang/NewChanlun/issues/794) 定性，2026-07-30**：不是两套 fill，是**两套决策层**——⑤ 走 `coverage::pi_theta_step`、⑥ 走 `strategy::recognize_nested`+`plan_orders`，实测互相零调用，分类层同源。**残留待裁**：两条独立决策路径并存是否本身即需收敛，归丙层。）
- **合闸的真实前置：四个卡口，次序已由实测定死一半**（[#796](https://github.com/xy7365527-lang/NewChanlun/issues/796) 把原「两个卡口」订正为四个）：
  ① 区间套证书门关闭 —— **已实测可开**，开后 ⑤ 段有反应但真链净贡献仅 1 admit/52；
  ② N7 消费点未接（#529）—— 独立未解，接它是实装；
  ③ **真链产不出证书**（`assembled=0`）—— **这是关键路径**，正是 [map #737](https://github.com/xy7365527-lang/NewChanlun/issues/737) 的题目，#737 由此从「尚未开工」升为**下一个该走的图**；
  ④ **⑥ 段真实 nautilus 引擎够不着这个门**（`pub(super)` 可见性）—— 结构性，非配置；成因（设计有意 vs 接线遗漏）未判。
  **「先收敛口径 vs 先开门」的两难已解**：光开门没用，必须先解③。待裁的只剩「③ 归 #737 自解还是并入本图」以及「④ 的成因与去向」。
- **中枢口径分叉的处置**（自 [#794](https://github.com/xy7365527-lang/NewChanlun/issues/794) 新出）：L0 含方向交替 vs L1/L2 不看方向，同塔混用且零裁定票——这是甲层收敛的**最急一块**，方法已由 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 定下，但按其①「第一批只开总缝票、七张概念票不预切」，**中枢那张属第二批**，须待 [清点票 #809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 报回剩余分歧面后再开。**[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 已对这条缝当场表态（2026-07-30）**：两函数签名逐字相同、只差 `dir_alternates`，`compose_level:304` 以 `if is_l0` 分派 ⟹ 正踩 #799 判别式；其「诚实有效域」举证书**理由够格但缺退场条件、载体在注释非定义文件 ⟹ 现状不合规**，中枢票须一并补齐（**收敛到哪一套仍未裁**）。**已有既定前提入账**：编排者已表态「单点核心必须有宽度」（严格 `<`），中枢票开出来时不再重议。
- **区间套零执行的处置**（自 [#794](https://github.com/xy7365527-lang/NewChanlun/issues/794) 新出）：`THETA_NEST_CERT_GATE` 门长期关闭 ⟹ #789 判定最烂的那块口径连暴露机会都没有。待裁：开门跑一遍取读数（探针票），还是先收敛口径再开门。
- **第二批概念票的范围与次序**（自 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 新出）：总缝规则会把 40 套里「按级别分两份」的一批直接判出局，**剩下多少分歧面、够切几张票、按什么顺序**——[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 已裁完（2026-07-30），**清点这半已毕业成 [#809](https://github.com/xy7365527-lang/NewChanlun/issues/809)**（按四类过筛 + 报三个数）；**留在雾里的余量＝拿到清点结果后「开几张、按什么次序」那一拍**。已知至少三块必进：中枢（同塔 L0/L≥1 分叉）、区间套（含算子 N 的统一性主张）、背驰（五种不相容力度）。**[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 追加两条必答项**：背驰票须裁「**次级别背驰要不要 Extreme 条件**」（`div_cand` 要 / `sublevel_diverges` 不要，收敛到哪套未定）；区间套票须裁「**小转大与区间套下钻是不是同一件事**」（教义侧第 43 课「c 中出现 1 分钟背驰」+ 第 53 课「二类点构成含次级别一类点」已在案，见 [#166](https://github.com/xy7365527-lang/NewChanlun/issues/166)）。两票均受 `AGENTS.md` 新立的「同一判断不得有宽严两档」通则约束。
- **`.chanlun/definitions/` 15 份文件的三行头**（自 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 新出）：规范已立进 `AGENTS.md`，但各文件的头须由该概念的裁定票落地时填（现在填「最后裁定票号」「受影响代码清单指针」只能编）。**不是雾也不是欠账，是随第二批逐票兑现的挂账**——记在此处防它变成第二个「写了但没人知道」。
- **测试与生产错位的处置**（自 [#794](https://github.com/xy7365527-lang/NewChanlun/issues/794) 新出，与既有「分歧不被发现」那条合并考虑）：CI 唯一在跑的九组对拍守 `rust/src/*.rs` 顶层散件 ↔ Python，而生产链只吃 `theta_v0/`，二者零 crate 内依赖。**修它是实装**，本图不带实装，故只登记去向待裁。
- **「分歧不被发现」的机制缺口**（**[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 已把成因定死并立原则，2026-07-30**：`ci.yml` 三 job 实查＝pytest+一条 grep gate／`cargo check`+`fmt`／fixture-drift，**无 `cargo test`** ⟹ `nest_isolation_guard.rs` 那道守卫与 34 个 parity 断言**只编译从不执行**，守卫文件头「违规即测试红」在 CI 里是空话；立原则「**验收判据必须真被 CI 执行才算数，只编译不执行的锁按无锁计**」。**去向已兑现：已挂成图外 triage 票 [#810](https://github.com/xy7365527-lang/NewChanlun/issues/810)（2026-07-30），与本图主线并行、不占其路径**。**★ 该票产出复核当场查出第二种空转形态（2026-07-30，主控本地实测）**：`ci.yml` 触发器挂 `main-rewritten`，而该镜像分支**落后 `main` 545 个提交** ⟹ **CI 当前跑的全部检查测的都是 545 提交前的代码**。故 #799 附带原则须加一句——不只是「必须被 CI 执行」，还得是「**在当前代码上**执行」；[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 对拍锁的落地前提同步升级。**同批查出**：当前线上 `theta_v0_classifier_parity` 有 **2 条 Lean↔Rust 一类买点对拍断言为红**（复跑稳定，非本会话引入），已立 [#811](https://github.com/xy7365527-lang/NewChanlun/issues/811) 并标注**教义分歧禁止直接修绿**，**去向＝第二批买卖点概念票的必答项**。**CI 分支归属（同步镜像 vs 改挂 `main`）涉及 push，待编排者拍**——原「下游实施链第一条动作＝让 CI 跑 `cargo test`」，否则 #799 裁的任何判据落地即空转）：CI 只跑 `cargo check` 从不跑 `cargo test`（34 个 Lean↔Rust parity 断言只编译不执行）；`fixture-drift` 仅证 Lean→fixture，fixture→Rust 半环断开；对拍证据全来自合成正弦数据，真实行情用例被 `-m "not slow"` 摘掉。**这是本图全部发现的共同成因，但修它是实装**——本图不带实装，故此处只登记去向待裁：走 triage 挂 `debt`，还是作为正本落地后实施链的第一条。
- **实盘段空缺的去向**：`LiveNode` 三处注释承诺、零代码；`theta_v0` 零 PyO3 出口故到不了 Python 实盘壳；`trading_system/live/` 唯一能真下单的脚本不消费任何缠论信号。**属架构事实而非本图 destination**——是登记进正本的现状章，还是另开一条实施线，待丙层裁。
- **~~同名异物的处置~~ → 统一区间套算子 N 的归属**（**[#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 已订正**，2026-07-30）：「`src/newchan/nesting/` 与缠论区间套无关」**判定撤回**——该目录实为编排者自写的**统一区间套算子 `N : (SearchSpace, Level) -> (Target, BSP_confirmation)`**，纵向（级别下钻）与横向（搜索空间收缩：配置→角→板块→标的）是它的**两个实例**，源 `[新缠论] 全球资本流转拓扑：级别与买卖点 v2`。**处置已定**：算子 N 的统一性主张进**第二批的区间套概念票**（该票须回答「区间套正本是不是算子 N」）；应用域（选标的）仍在 Out of scope 内不动。**未毕业的余量**：这是第三个递归方向，与 Destination 的「双向递归」前提是什么关系——待 [#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 与区间套票合读后才谈得上具体化。

- **ADR-0005 搬家案的落地**（自 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定七新出）：管辖边界载体已裁为「**目录即层**」——nest 三件（`nest.rs`/`nest_index.rs`/`turn_class.rs`）搬出判据目录 `classifier/`，搬完 `GUARD-ROLE` 自述豁免机制整个拆除。**原理侧无争议**（ADR 自陈「唯一根治法」），**成本当年未估、#799 也没估**。**属实装，本图不带实装** ⟹ 登记去向待裁：并进正本落地后的实施链，还是单开一条线。同批待处置：`nest_cert_gate_enabled` 的 `pub(super)`（[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定八判为接线遗漏）随搬家一并按「门属于哪一层」定可见性，不单独打补丁。

- **`theta_v0` 的 PyO3 出口票**（自 [#808](https://github.com/xy7365527-lang/NewChanlun/issues/808) 裁定新出，**本图关图前置**）：[#808](https://github.com/xy7365527-lang/NewChanlun/issues/808) 条款②把「执刀人」定为**写这张票的人**，且**建新与杀旧不得分票分人**。而 [#808](https://github.com/xy7365527-lang/NewChanlun/issues/808) 实查 `theta_v0` **零 PyO3 出口**、继任判据 D1 现 **0/2** ⟹ 这张票现在**还不存在、没有主**。**它同时是 Destination 关图条件②（下游实施线已开、有票号、有主）的一个具体落点**——与「三条实装欠账」（ADR-0005 搬家案 / CI 补 `cargo test` / `nest_cert_gate_enabled` 可见性）同批，待甲层收敛走到一半时一并开线。
- **正本必收条目的挂账**（防「写了但没人知道」，与 `.chanlun/definitions/` 三行头那条同型）：已裁定但正文仍只存在于票内、须在正本落盘时收录的条目——① [#808](https://github.com/xy7365527-lang/NewChanlun/issues/808) 的「另起一套」四句模板（另需并进 `docs/agents/generation-constitution.md` 名分记录加一列「退场条款」）；② [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十一的验收三档分派表；③ [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定六/七的「生产判定路径」管辖划法与目录即层。**不是雾，是随正本兑现的挂账**。

## Out of scope

- **支柱 II PH 拓扑、支柱 III K4 选股**——编排者 2026-07-30 划界：「那些是后面的东西，我现在做的阶段是缠论的量化交易系统」。本图只管缠论引擎与它的交易这一条链。
- **`nautilus_trader/`（4062 文件）上游框架本身**——集成方向是本仓 crate 依赖它，非它依赖本仓（实测 NT 内对 theta_v0/newchan/chanlun 引用 0 命中）。本仓自写的适配层 `theta_v0/nautilus/` 在疆域内。
- **[#743](https://github.com/xy7365527-lang/NewChanlun/issues/743) 落地后的存量票据漂移处置**——已有专票 [#786](https://github.com/xy7365527-lang/NewChanlun/issues/786)，与本图正交（那张管票面订正，本图管架构统一）。




