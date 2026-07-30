# #801 递归自相似系统的模块化方法论调研——判定过适用性的候选 + Rust 承载物 + 验收判据

- 日期：2026-07-30
- 票据：[#801](https://github.com/xy7365527-lang/NewChanlun/issues/801)（parent [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)，blocking [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799)）
- 性质：**只调研，不动代码，不做选型裁定**（选型归 #799）。
- 分支：`research/i801-recursive-modularity-methodology`（自本地 `main` @ `f6d000fed2` 切——注意 `origin/main` 落后本地 3215 commit 且不含 `rust/`，本报告全部代码锚以本地 `main` 为准）
- 标注约定：**【事实】**＝代码/文档/外部文献可直接核对；**【推断】**＝由事实推出；**【未找到】**＝090 照实的空手而归项。
- 中途输入：主控初判追问（recursion schemes 三点假说）+ #800 复盘结果（仓内已有成功样板）——两者均已正面处理，见 §4、§5。

---

## 0. 一页速查

| 候选 | 是不是本仓的问题 | Rust 落得下吗 | 增量还是改造 | 判定 |
|---|---|---|---|---|
| **C0 仓内朴素样板**（`recursive_t` 信号层形态：纯函数 + 级别作参数 + 判定与调度分离 + 回归测试锁） | **正是** | **已落过**（就在仓里，2026-06-18 起保留至今） | **天然增量**——theta_v0 已有 90% 同形态，缺的是收口 | **首选** |
| C1 recursion schemes / F-代数 | 思想对、机制形状**不对**（本仓递归是"层序列"型，不是"树节点"型） | 思想已在仓（`RMove` μF + `descend∘compose=id`）；完整机制要 HKT 模拟 + 变体动物园，生态无 para/histo 现成件 | 机制路线＝**改造**，是"第三套"的经典触发 | **取思想、弃机制** |
| C2 APoSD 深模块 | 半个问题（模块该多深/缝在哪），不讲递归复用 | 已在用（`codebase-design` skill） | 增量 | **保留作互补**，不单独够用 |
| C3 多分辨率 / 小波 MRA / 金字塔 | 组织范式对口（同一算子跑所有尺度、尺度差异进数据），但**不带来 C0 之外的新承载物** | 落下去就是 C0 的形状 | 增量（只借词汇和不变量清单） | **部分对口**：借三条纪律，不引架构 |
| C4 tagless final | 不对口（解"同一语法多解释器"，非"多级复用"） | 在 Rust 塌缩成普通 trait | — | **不适用** |
| C5 visitor / 泛型遍历 | 不对口（OO 双分派仪式，Rust 惯用 enum+match 已覆盖） | — | — | **不适用** |
| C6 DDD bounded context | 对递归复用不对口；对"40 套口径"病理有**命名价值**（无 context map 的多语境并存） | — | — | **不适用**（借一个诊断词） |

**对 #799 的一句话**：方法论找到了，而且**就在仓里**——`recursive_t` 信号层的朴素形态（§2 C0）。外部方法论里没有任何一个在"增量性"上打得过它；recursion schemes 的价值收敛为一条纪律（递归展开与每步判定分离），该纪律 C0 已内含。验收判据见 §6。

---

## 1. 靶子形状（实测，两套引擎各一份）

### 1.1 theta_v0（主战场）——判定已参数化，但 L0/L≥1 分叉外泄

**【事实】** 逐条锚：

- 塔驱动是显式 level 循环：`rust/src/theta_v0/classifier/mod.rs:509,1009` `for level_idx in 0..=l_max`，每级调 `compose_level(&units, &moves_tower[..], is_l0, level)`（`mod.rs:526`）。
- **判定已经作为参数传入驱动**：`recursive_tower.rs:298 compose_level` 内部把中枢构造函数当值传给探测器——
  ```rust
  let build = if is_l0 {
      super::center::center_from_segments   // L0：方向交替 ∧ 严格核心非空（完整判据）
  } else {
      super::center::center_from_window     // L≥1：仅几何（全三段核心非空）
  };
  let (windowed, ..) = detect_centers_windowed_resume(units, build, 0);
  ```
  两个构造函数**同签名** `(&UnitRange,&UnitRange,&UnitRange)->Option<Center>`。
- **L0/L≥1 中枢定义不同是教义有据的，不是漂移**：`classifier/center.rs:233-241`（`center_from_window` doc）明写"上级单元是中枢外缘区间，**无内在缠论方向**——Origin 的完整判据 `CenterConfirmedComplete`（方向交替）定义在 L0 Segment 上；上级只保留几何判据，**这是 Origin 上级发展态判据的有效域，非省略完整判据**"，且有 Lean 锚（`Origin.CenterComplete` / `Origin.CenterStates.classifyDevelopment`）。
- **真正的病不在"定义不同"，在分叉的表达方式与外泄**：`is_l0` 这个 bool 在 `mod.rs` 出现 **24 处**、`recursive_tower.rs` 6 处；`center_from_segments/window` 在 center.rs 之外有 **十余个调用/重抄点**——回测箱自己挑构造函数（`backtest/econ_positive.rs:1628`）、甚至**逐字重抄弱口径变体**（`backtest/wverify_run.rs:2944 c327_weak_center_from_segments` / `:2963 c327_weak_center_from_window`）。这正是 #789 "40 套口径"的生成机制：**分叉选择权在调用方手里，每个实验箱都可以再选一次、再抄一份**。
- 判定文件本身反而干净：`classifier/divergence.rs` / `decompose.rs` / `bsp.rs` 内按 level 分叉 grep **命中 0**。
- 仓里已有 μF 形态：`classifier/descend.rs::RMove`（`Segment | Compose{subs,centers,level}`）自述是 Lean `Move` μF 逐字段镜像，`descend ∘ compose = id` 是登记在案的 L0 对偶律（`recursive_tower.rs` 头注）。

**【推断】** theta_v0 离"判定写一次、每级复用"**只差收口**：判定已是参数、判定文件已无 level 分叉、递归对象已是初始代数形状。缺的是 (a) `is_l0` 分叉收敛到唯一缝、(b) 实验箱禁止重抄构造函数、(c) 一条机械锁（回归测试 + grep gate）防退化。

### 1.2 recursive_t 信号层——#800 判定的独立复核（本票被要求照实推翻，结果是：**复核通过**）

**【事实】** 我对 #800 四条判据逐条重验：

1. 调度 28 行只做循环：`recursive_t/mod.rs:96 iterate` 循环体唯一实质语句 `let out = apply_t(&current, k, mode);`，深度由数据涌现（`trends.is_empty()` / `next.len()<3` 停），`k>64` 仅安全阀。✅
2. 级别是参数不是分支：`grep "level ==|level <|level >|match .*level"` 在 `center.rs`/`trend.rs`/`operator.rs`/`types.rs` **命中 0**（我重跑，与 #800 一致）。✅
3. 回归测试锁在案：`operator.rs:198 fn apply_t_泛型_同一套代码作用于level1单元()`——把 level-1 单元喂给**同一个** `apply_t` 产 level-2 结构。✅
4. 唯一裂缝如实：`divergence.rs:347 has_nest` 按**数据属性**（`inner_zhongshu_count>0`）分叉真背驰/类背驰，实践上与 L0/L≥1 一一对应；另 `divergence.rs:389 t.level < 9` 仅诊断门。✅

**一处口径出入（不影响结论）**：#800 报信号层 2611 行；我 wc `mod+center+trend+operator+divergence+types` = **3032 行**。差异应为文件圈定口径不同（如是否计 types.rs），四条判据均不受影响。

**【事实】** 该形态的承载物清单（这就是"仓内样板"的全部技术含量）：
- 一个纯函数 `apply_t(units: &[Unit], level: usize, mode: PerfectionMode) -> TLevelOutput`（`operator.rs:94`），输入不可变（`operator.rs:190` 有"不改输入"测试）；
- 单元类型自带级别差异所需的**数据**（`Unit.inner_zhongshu_count`——L0 恒 0、L≥1 携真中枢数），判定读数据不读级别号；
- 调度器一个循环，判定经参数（`mode`）与数据（`units`）进入；
- 一条"同一套代码作用于所有级别"的回归测试作机械锁。

---

## 2. 候选逐判（每个按票面六问）

### C0 仓内朴素样板（`recursive_t` 信号层形态）——**首选**

1. **是不是本仓的问题**：**【事实】** 是，同一个问题在同一个仓被解过一次：立项文档 `docs/unified_recursive_operator_T.md:1`"用一个递归算子 T 取代手动分层 ladder"，动机就是消灭"判定绑死在具体层"。信号层做到并保留至今（§1.2）。
2. **Rust 承载物**：纯函数 + 参数化判定 + 数据携带级别差异 + 回归测试锁（§1.2 清单）。**零 trait、零泛型、零宏**——不是没能力用，是不需要。
3. **落到 theta_v0 长什么样 / 代价**：**【推断】** 不是移植代码（两套教义口径冲突在案，#800 §2.1），是移植**形态**，且 theta_v0 已有 90%（§1.1）。剩余工作三件：
   - a) `is_l0` 分叉收口到唯一缝——最小方案甚至不用改类型：`compose_level` 已是唯一生产选择点，把 `mod.rs` 里其余 23 处 `is_l0` 消化掉、把实验箱改成 import；更严的方案是把方向维度上载到单元类型（如 `enum LevelUnit { Directed(UnitRange), Bare(..) }` 或 `UnitRange.direction: Option<Direction>`），让分叉由**类型/数据**驱动而非调用方 bool——与 recursive_t 用 `inner_zhongshu_count` 携带级别差异同构；
   - b) 实验箱重抄口径（`c327_weak_*` 一族）逐个改 import 或显式登记为"口径变体"；
   - c) 加 `operator.rs:198` 式回归测试 + grep gate（见 §6）。
   代价：改动集中在 `mod.rs` 与若干回测箱，判定文件基本不动；风险是回测 bit-exact 锁多、动 `mod.rs`（5389 行）需小步。
4. **不适用的地方（诚实）**：**【事实】** 样板只覆盖**结构判定层**。#800 铁证：操作层（仓位三阶段战役）per-level 自我复制被 L3 打回、编排者裁决回退（`docs/recursive_t_deadlock_escalation.md §10`"不要自创约束"）——**操作层不属于本方法论的适用域**，#799 裁定时须按层分开。另：样板的"级别差异进数据"不能消灭 L0 的真实特殊性（L0 才有方向维度，教义如此），只能把特殊性**收口到一处**。
5. **增量还是改造**：**天然增量**——它就在仓里，theta_v0 大半形态已同构，可改一处验一处。

### C1 recursion schemes / F-代数、初始代数——**取思想、弃机制**

（主控初判三点的逐点裁定在 §4，此处按票面六问给总判。）

1. **是不是本仓的问题**：思想是（"递归怎么展开"与"每一步做什么"分离——catamorphism 一族的全部要点，见 Meijer/Fokkinga/Paterson, *Functional Programming with Bananas, Lenses, Envelopes and Barbed Wire*, FPCA 1991）。但**机制形状不对**：标准 cata 的合同是"每个节点只看直接子节点的折叠结果"，而本仓每级的判定单位是**整条同级序列**（滑动三窗找中枢、前后段力度比较、中枢链分组）——递归发生在**级别轴**上（`Vec<Unit_k> -> (判定_k, Vec<Unit_{k+1}>)` 迭代到不动点），不是在语法树节点上。这是 unfold-over-levels，不是 fold-over-nodes。
2. **Rust 能不能落 / 承载物**：**【事实】** 生态现状——[`recursion`](https://crates.io/crates/recursion) crate（Inanna Malick，v0.5.4，2026-07-23 仍在维护，[docs.rs](https://docs.rs/recursion)）：核心 trait `MappableFrame`（≈Functor）/`Collapsible`（≈cata）/`Expandable`（≈ana），显式栈机保证栈安全；用户须为每个递归类型手写 partially-applied frame 类型（`ExprFrame<A>`）并实现三 trait。设计文档为作者三篇博文（[recursion.wtf](https://recursion.wtf/posts/rust_schemes_2/)）。**只有 cata/ana，无 para/histo/zygo 现成件**。大规模生产使用证据：**【未找到】**（检索到的实例是作者自己的 filetree 搜索工具示例）。另有更早的 `recursion-schemes` crate（同作者，Haskell 味更重）。
3. **落下去长什么样 / 代价**：为 `RMove` 写 frame 类型 + 三个 trait impl 只覆盖"树上折叠"，而中枢/背驰/走势判定还需要横向窗口、历史、跨级下沉——每样都要在无 HKT 的 Rust 里手造变体（付出 GAT/关联类型样板代码），或把上下文塞进 carrier 里（等价于回到普通函数）。对一个 40 套口径、以中文教义注释为主要接口的仓库，可读性成本致命：**看不懂的抽象会被绕过，绕过就是第 41 套口径**。
4. **不适用的地方**：机制整体（理由如上）。特别指出：**仓里已经有了它的思想产物**——`RMove` 是 Lean `Move` μF（初始代数）的镜像，`descend∘compose=id` 就是 cata/ana 对偶律的实例。再引 crate 是给已有的思想买第二份机制。
5. **增量还是改造**：机制路线＝**改造**（frame 类型 + trait 层渗透所有判定签名），且正中 #800 查实的"第三套"触发形状。思想路线＝已含于 C0，零额外引入。

### C2 A Philosophy of Software Design 深模块——**证实主控怀疑：只覆盖半个问题**

1. **是不是本仓的问题**：半个。APoSD（John Ousterhout, *A Philosophy of Software Design*, ch.4 "Modules Should Be Deep"、ch.7 "Different Layer, Different Abstraction"）讲接口该多小、复杂度藏哪、缝放哪——正是 §1.1 "把 `is_l0` 收口到唯一缝"这一步要用的语言。但全书**没有一章讲"递归结构怎么复用判定"**；它的分层观（每层不同抽象）甚至与本仓"每层同一抽象（自相似）"的诉求方向相反——需要 C0 来补"层间同构"这半边。
2. **Rust 承载物**：无新增（它是设计语言不是机制）；仓内 `codebase-design` skill 已提供词汇（module/interface/depth/seam/adapter/leverage/locality），#743 全程在用。
3. **落下去长什么样 / 代价**：继续当审查词汇用，零代价。
4. **不适用的地方**：不回答"判定写一次、每级复用"如何机械保证——那要靠 C0 的测试锁。
5. **增量还是改造**：增量（已在用）。

### C3 多尺度 / 多分辨率组织范式（小波 MRA、金字塔算法）——**部分对口：借纪律，不引架构**

1. **是不是本仓的问题**：对口——这门工程几十年就在做"同一套变换跑在所有尺度上"。范式内核三条：(i) **一个分析算子 + 一个尺度调度器**，尺度差异由**数据**（降采样后的信号）携带，不由代码分支携带——金字塔构造就是对同一算子的迭代（Burt & Adelson, *The Laplacian Pyramid as a Compact Image Code*, IEEE Trans. Communications COM-31(4), 1983；工程实体如 OpenCV `cv::pyrDown`/`buildPyramid`——同一函数迭代成塔，[docs.opencv.org](https://docs.opencv.org)；ITK `itk::MultiResolutionPyramidImageFilter`——per-level 参数以 **schedule 矩阵（数据）**表达而非代码分支）；(ii) MRA 公理化了"层间关系"（嵌套子空间 V_{j+1}⊂V_j，同一 scaling 函数伸缩生成所有层——Mallat, *A Theory for Multiresolution Signal Decomposition*, IEEE TPAMI 11(7), 1989；教科书 Mallat, *A Wavelet Tour of Signal Processing*, ch.7）；(iii) **完美重构不变量**：分解∘重构=恒等，是整个滤波器组工程的验收判据（同书 ch.7；lifting 形态见 Sweldens, *The Lifting Scheme: A Construction of Second Generation Wavelets*, SIAM J. Math. Anal. 29(2), 1998）。仓内已有"缠论递归 ≅ 小波 MRA"类比记载（memory `滤波器比喻核心目标`）。
2. **Rust 承载物**：**没有超出 C0 的新承载物**——"同一算子迭代 + 尺度进数据"落到 Rust 就是 `iterate/apply_t` 那个形状；"schedule 作数据"对应 per-level 参数表（若将来需要 per-level 阈值，进 config 表而非 `match level`）；"完美重构"对应仓里已有的 `descend∘compose=id`。
3. **落下去长什么样 / 代价**：借三条可检查纪律（写进 §6 验收判据），零架构改动。
4. **不适用的地方（诚实）**：(a) 小波各层信号**同型**是构造出来的，缠论 L0 真特殊（方向维度只在 L0 存在）——MRA 消不掉这个，工程金字塔同样对第 0 层做特殊处理（初始平滑/边界），所以它不能替 #799 回答"L0 特殊性怎么表达"，只能支持"特殊性收口到一处"的方向；(b) 小波的算子是线性滤波、层数由分辨率上限定，缠论判定是非线性谓词、层深由数据涌现——类比到公理层就断了，**不要把 MRA 当形式化依据用，只当组织范式用**。
5. **增量还是改造**：增量（只借词汇与判据）。

### C4 / C5 / C6 快扫（均判不适用）

- **C4 tagless final**（Kiselyov, *Typed Tagless Final Interpreters*, 2012 lecture notes）：解的是"同一语法、多个解释器"（求值/打印/优化共用一套项构造），依赖对解释器 carrier 的高阶抽象；Rust 无 HKT，落下来就塌缩成普通 trait + 关联类型。本仓的痛不是"一套判定要多种解释"，是"一套判定要在多级复用"——**问题维度不同，不适用**。
- **C5 visitor / 泛型遍历**：OO 语言里分离"数据结构 vs 施于其上的操作"的手段；Rust 的 enum + match / 高阶函数参数已原生覆盖（`detect_centers_windowed(units, build)` 就是它的 Rust 惯用形态）。引入 visitor 双分派仪式无增益——**已被 C0 吸收，不单列**。
- **C6 DDD bounded context**（Eric Evans, *Domain-Driven Design*, 2003, Part IV "Strategic Design"）：解的是组织尺度上多模型并存与映射，不提供递归机制——对票面问题**不适用**。唯一带走的：它给 #789 的病理一个准确命名——"40 套口径"＝40 个**没有 context map 的隐式 bounded context**；已有的教义裁定票（#321/#322/#290 一族）实质上就是在人肉维护 context map。此观察移交 #799 参考，不构成引入建议。

---

## 3. 【未找到】清单（090）

- **未找到**任何"递归自相似系统模块化"的**成名成套方法论**（像 TDD/DDD 那样有名字、有书、有社区的）——这个问题在文献里被拆散在三处：递归模式（FP）、多分辨率（信号处理）、深模块（软件设计），没有一家独占。本报告的"方法论"是这三处思想被仓内样板实证过的交集，不是某本书的转述。
- **未找到** `recursion` crate 的大规模生产使用实证。
- **未找到**能一并解决操作层（仓位战役单例）的方法论——#800 的证据指向那一层的问题不是架构表达力，本票不越界。

---

## 4. 主控初判专节裁定（recursion schemes 三点假说）

**总判：成立（含一处修正加重）。**

**① "缠论判定形状超出标准 catamorphism"——成立，且比初判更甚。** 初判问三样是否对应 para/histo/hylo 变体：
- 中枢看同级相邻三段：**连 paramorphism 都不够**——para 给的是"子树 + 子树折叠结果"，仍是纵向；同级横向窗口在树折叠里要靠 zipper 或把整层序列做 carrier，后者等价于放弃节点局部性；
- 背驰比前后段力度：histomorphism 方向（携历史标注），Rust 生态无现成件；
- 区间套下沉：anamorphism/hylomorphism 方向，`recursion` crate 有 `Expandable` 但与上两样组合无现成方案。
**代价判断**：cata/ana 基线 = 每个递归类型一个 frame 类型 + 3 个 trait impl（可承受）；每加一个变体 = 在无 HKT 的语言里手造该变体的 trait 与驱动（无 crate 可抄，成本约等于自研一个小型库）。三个变体叠加后，机制的总代码量与认知负担**超过被它抽象掉的重复本身**——负杠杆。**但更根本的是修正**：本仓递归的自然单位是"每级一次整序列变换、迭代到不动点"（§1.1/§1.2 两套引擎驱动器实测同形），即递归在**级别轴**而非节点轴——所以问题不是"要几个变体"，是**基座就不对形**。
**② Rust 无 HKT、机制代码重难读——成立。** 生态实查见 §2 C1.2：可用 crate 存在且在维护，但只覆盖 cata/ana、要求手写 frame 类型、无生产实证；"看不懂就绕过去自己写一套"对本仓是实证过的行为模式（40 套口径即其沉积）。
**③ "漂亮框架 + 难套用的旧库 = 第三套"——成立，且 `recursive_t` 的来路即样本。** #800 查实：recursive_t 落地 commit 自述"standalone 架构（不依赖 v3 nucleus）"——上一次就是"带着更漂亮的统一观从零起"。机制路线的 recursion schemes 完全复刻此形状。

**核心区分（取思想 vs 取机制）——区分成立，中间路不是自欺，有实证。** "只取思想"要成立须回答：不靠完整机制，"天然复用、不靠人记得"能不能机械保证？答案在仓里：`recursive_t` 信号层用**回归测试**（`operator.rs:198`：把上级单元喂同一函数断言产出）+ **零 level 分叉的 grep 事实**顶住了六周以上无退化——机械保证的载体是**测试与 gate，不必是类型系统**。类型机制给的是编译期保证，测试 gate 给的是提交期保证；对本仓（判定口径本来就要经教义裁定频繁改）提交期保证已够，编译期保证的溢价买不回它的可读性成本。**结论：中间路成立，且是唯一同时满足票面判据 1（拆开）与硬防线 5（增量）的路。**

---

## 5. 给定仓内已有成功样板，外部方法论还有没有引入的必要（#800 触发的直答）

**部分有——但"引入"的内容是纪律和验收语言，不是任何机制或依赖。** 分三条：
1. **机制层面：没有必要。** 三个外部候选（C1 机制、C4、C5）全部判负；C3 落到 Rust 后与仓内样板同形。任何外部机制在增量性上都打不过"样板就在仓里"这一事实。
2. **纪律层面：有必要，三条外部来源的纪律值得成文**（它们解释了样板**为什么**成立，光抄样板不知其所以然会在下次压力下丢掉）：(i) 展开与判定分离（recursion schemes 的思想内核）；(ii) 尺度差异进数据/参数表、不进代码分支（金字塔/MRA 的 schedule 纪律）；(iii) 层间往返恒等作为不变量测试（完美重构纪律，仓内 `descend∘compose=id` 已是实例）。
3. **回答"为什么样板没传到 theta_v0"**（本票范围内的证据推断）：**【推断】** 不是技术障碍——theta_v0 独立长出了 90% 同形态（判定作参数、判定文件零 level 分叉），说明形态本身会被重新发明；没传过去的是**那条机械锁**（`operator.rs:198` 式测试在 theta_v0 无对应物）和**分叉收口纪律**（`is_l0` 外泄 24 处、实验箱重抄口径无 gate 拦截）。这与 #787 根因判断（"没人知道已经有什么"）一致：样板的四条判据从未被提炼成可检查的验收句，所以第二套引擎只能靠巧合复现其中三条。**#799 真正要拍的不是"选哪个方法论"，而是"把样板的第四条（机械锁）补进 theta_v0 并立为 gate"。**

---

## 6. 验收判据（喂 #799——全部可机械检查）

达成"判定写一次、每级复用、不靠人记得"，当且仅当以下五条全绿：

1. **判定文件零级别分叉**：`rust/src/theta_v0/classifier/{center,divergence,bsp,decompose}.rs` 内 `grep -E "level\s*(==|<|>|>=|<=)|is_l0|match .*level"` 命中 0（诊断计数器可白名单，须逐条登记）。级别号只允许出现在驱动层与坐标/ID 层（`recursive_tower.rs` 的 `ElementId` 一类）。
2. **L0/L≥1 分叉唯一缝**：`center_from_segments`/`center_from_window` 的**生产**选择点全仓恰 1 处（现为 `compose_level`）；其余出现处只许是 (a) center.rs 定义、(b) import 后直调且不做二次选择、(c) 测试。检查：`grep -rn "center_from_(segments|window)"` 人工分拣 ≤ 一屏。**更强版本**（#799 可选裁）：方向维度上载进单元类型（`Option<Direction>` 或双 variant enum），bool 消失，分叉由数据驱动——与 recursive_t `inner_zhongshu_count` 同构。
3. **同码全级回归测试**：theta_v0 侧存在 `operator.rs:198` 的对应物——手搓 L1 形态的 `UnitRange` 序列喂**同一个**判定入口，断言产出 L2 结构；测试名里写明其锁的命题（"所有级别用同一套代码"）。
4. **口径重抄清零**：实验箱/回测箱内与 classifier 判定同名异实现的本地函数（现存 `wverify_run.rs:2944,2963 c327_weak_*` 一族）逐个：改 import，或在文件头登记为"教义口径变体 + 裁定票号"。检查：`grep -rn "fn .*center_from\|fn .*_weak_center" rust/src --include="*.rs"` 除 center.rs 外命中均须携带登记注释。
5. **驱动器极小**：级别循环体 ≤ 30 行、只做"调判定、收产物、喂下级、判终止"四件事（recursive_t `mod.rs:96` 与 theta_v0 `mod.rs:509` 现状均接近达标，防的是未来判定逻辑回渗驱动器）。

**适用域声明**（防 #799 误用）：五条只覆盖**结构判定层**。操作层（仓位三阶段战役）有 #800 的 L3 否定性铁证在案，不适用本判据，须另行裁定。

---

## 7. 结果包六要素

1. **结论**：见 §0 表 + §4/§5 直答。首选＝仓内朴素样板（C0），外部方法论只取纪律不取机制；主控 recursion-schemes 初判成立（含"基座不对形"修正加重）。
2. **依据**：代码锚——`rust/src/theta_v0/classifier/{center.rs:210-268, recursive_tower.rs:298-311, mod.rs:377-409,509,526}`、`rust/src/theta_v0/backtest/{econ_positive.rs:1628, wverify_run.rs:2944-2963}`、`rust/src/recursive_t/{mod.rs:96-124, operator.rs:94,190-198, divergence.rs:347,389, types.rs:147}`；仓内文档锚——`docs/unified_recursive_operator_T.md`、`docs/recursive_t_deadlock_escalation.md`、`.chanlun`(#800 分支)`/chanlun/review-results/i800-recursive-t-postmortem-20260730.md`、`~/.claude/skills/codebase-design/SKILL.md`；外部——Meijer/Fokkinga/Paterson FPCA 1991；[`recursion` crate](https://crates.io/crates/recursion) v0.5.4 + [docs.rs](https://docs.rs/recursion) + [recursion.wtf 三篇](https://recursion.wtf/posts/rust_schemes_2/)（在线核验）；Ousterhout *APoSD* ch.4/ch.7、Burt & Adelson 1983、Mallat TPAMI 1989 / *A Wavelet Tour* ch.7、Sweldens SIAM 1998、Kiselyov 2012、Evans *DDD* 2003（标准文献，凭书目引用，章节号按通行版本，**未逐页在线核验**）。
3. **边界**：只调研不改码；未编译未跑测；#800 复核仅覆盖其信号层四判据（复核通过，行数口径差 2611/3032 登记在案），其余章节按原报告采信；`origin/main` 与本地 `main` 谱系分裂（3215 commit）是环境事实，本报告锚定本地 `main`。
4. **下游**：#799 按 §6 五判据裁；建议其把问题从"选方法论"改写为"补机械锁 + 收口分叉"；操作层另行开题。
5. **谱系**：`project_unified_architecture_map_787`、`project_recursive_t_architecture_v2`、`project_unified_recursive_operator_T`、`滤波器比喻核心目标`；票 #787/#789/#799/#800/#743。
6. **影响**：新增本报告一份，零代码改动。
