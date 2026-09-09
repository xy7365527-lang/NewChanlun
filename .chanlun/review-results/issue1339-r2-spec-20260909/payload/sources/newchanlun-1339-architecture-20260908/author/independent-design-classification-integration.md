# 独立方案的完全分类整合合同

> #1339 R1；目录 r2-reviewable，SHA-256 `b8713384045dd3f15dad14cd5f77d89a92c4176bbe3487d6ba4a09432041106f`。本附录完整承接62轴、10组合、32定位项，未宣称全分类已证明。

静态分区、互斥、实现等价与构造不变量按语义版本、程序哈希及数值/策略版本验证成 ProofPackage，版本不变即可复用。每次推进核具体输入的域/见证、依赖与所用包，真正计算全部对象/分类/关系并产覆盖收据；不要求每bar跑Lean或重证全域。程序/规则变化需重新验证受影响证明包。

完整域、逐叶/公式族、来源与合法转换原文在绑定 [目录](CLASSIFICATION-CATALOG.json)；本附录指定它们在新生产架构中的计算、持久输出与验证责任。下面每一行均对应同号目录条目，不能按当前数据有无实例裁剪。

所有轴经同一结构与操作读法 Module 的具名调用计算；catalog、snapshot(axis)、对象关系、变化及explain同源输出。经营只消费适用且具版本的证据，前端不补算。角色/变量schema可同时出现，不用“62互斥市场类别”误述目录。对于实际分区轴必须分别核覆盖、两两互斥和唯一归类；角色schema逐关系满足其域/关联约束。

| 轴 | 计算责任/调用 | 持久输出 | 必须核的具体义务 |
|---|---|---|---|
| CC-001 有序值的原子三分 | 有序几何 / `exact_compare(a,b)` | 精确LT/EQ/GT；等号见证保留 | 三分覆盖/互斥与数值投影等价，不能epsilon |
| CC-002 两闭区间全端点关系 | 有序几何 / `interval_relation(I,J)` | 26端点秩叶及包含/重叠/gap多投影 | 退化闭区间全26叶；严格子域13叶；同一原子叶推出各关系 |
| CC-003 相邻K线包含和无包含方向 | 包含构造 / `bar_pair_signature(a,b)` | 9比较签名及有包含/无包含方向 | 完整9签名与方向来源；包含投影不first-match丢重叠 |
| CC-004 包含折叠输入及方向来源 | 包含构造 / `fold_prefix(prefix,init_policy)` | 7叶：空/单根/非包含UP追加/非包含DOWN追加/UP合并/DOWN合并/包含未建方向 | 初始方向无证据时具体等待G001；非包含建立方向后才能用其合并 |
| CC-005 三种坐标及组映射 | 身份 / `coordinate_map(group)` | raw/merged/代表根三个角色与成员映射 | 三角色不当互斥市场态；等价极值tie身份G002 |
| CC-006 无包含三K的四种局部形态 | 分型 / `fractal(local_triple)` | 无包含三K四局部形态 | 证明输入已去包含，逐个满足形态判据 |
| CC-007 分型描述与已实现后续发展 | 只读描述 / `describe_fractal(fractal,prefix)` | 实体/影线及后续已发生事实 | 不创造强弱阈值叶；描述不准入 |
| CC-008 新笔候选的完整条件签名 | 新笔 / `new_bi_candidates(fractals,coordinates)` | 18条件签名及原始/合并距离 | 双坐标域完整且不以一种坐标替另一种 |
| CC-009 同型分型去重与极值身份 | 新笔 / `deduplicate_same_type(candidates)` | 同型六支、同价tie全候选和选根 | 极值价格唯一不推出根唯一；G002定实例 |
| CC-010 笔形成和确认 | 笔确认 / `bi_lifecycle(prefix)` | 形成/确认及所依赖后续对象 | 原始合法追加确认单调，修订单独代际G004 |
| CC-011 线段构造候选条件 | 线段构造 / `segment_seed(bis)` | 五长度域/16条件签名、闭公共重叠 | 三笔相切闭边界已裁；不得套正式center严格端点 |
| CC-012 特征序列作用域与标准化 | 特征序列 / `feature_sequences(segment)` | 三序列角色及各自成员/包含域 | 第二序列同向正确成员；角色共存不当互斥 |
| CC-013 线段终结的两情况与完整条件族 | 线段终结 / `segment_termination(features)` | 32条件签名及5语义处理结果/两情况 | 两情况/任意分型/G4 strict封闭统一投影G006 |
| CC-014 笔破坏、段破坏、扫描完整性 | 线段扫描 / `scan_destruction(prefix,scope)` | 笔破坏/段破坏独立事实与扫描完整性 | 窗口G007具名；耗尽预算不等无终结 |
| CC-015 构造、操作读法与定位读法 | 读法分层 / `views(classification,role,policy)` | 主构造/操作/定位三个具名读法 | 唯一主塔+每读法唯一分解，不要求旁路等同主塔G008 |
| CC-016 正式中枢的seed判定 | 中枢构造 / `center_seed(real_children)` | 32条件签名、核心<=>三几何叶 | 输入真次级/完成/连续，strict seed；无候选冒充center |
| CC-017 中枢核心和外缘变量角色 | 中枢发展 / `center_geometry(center,members)` | 冻结ZD/ZG、全成员GG/G/D/DD及成员 | 六变量角色并列；D>G合法，不误作核心非空 |
| CC-018 单走势单元相对中枢核心的位置 | 中枢关系 / `unit_vs_core(unit,core)` | 单元对核心5位置关系 | 闭边界全部；单元与点位置轴不同 |
| CC-019 中枢离开后首回试与破坏 | 中枢发展 / `leave_first_retest(center,path)` | 10离开/首回试情形及三类/破坏事实 | 第一次真实完成回试与严格三类端点G010 |
| CC-020 相邻正式中枢对的完整发展分类 | 中枢对 / `center_pair(left,right)` | 5对分类及完整外缘不等式 | core未分离与向上/下新生/扩展，关系不冒充高级对象 |
| CC-021 九段升级及构造尝试 | 升级构造 / `nine_unit_recut(center,units)` | 九段计数向量、所有重切子窗与高级构造 | 无波动排除实例G011；每个子窗仍需strict seed |
| CC-022 中枢成员、连接与尚未归属单元 | 分解归属 / `assign_units(view,units)` | 成员/连接/尚未分配，续扫起点 | 全覆盖且不重复归属；Unassigned真实尾部不是完成走势类 |
| CC-023 已完成走势类型三分类 | 完成走势 / `classify_completed(move)` | 盘整/上涨/下跌恰一类 | 真实完成对象落良构域G012；多枢flat须正确分解/升层 |
| CC-024 盘整语义方向与技术突破方向 | 走势方向 / `semantic_and_break_direction(move)` | 盘整无语义方向，技术突破字段单列 | 旧兼容字段及消费者退场G013 |
| CC-025 形成、完成、同级继任与中阴 | 转折 / `completion_successor(move,prefix)` | 形成、完成、真实同级继任和中阴分轴 | 一般Move与Turn窄Completed桥G014；不能把Div当所有完成 |
| CC-026 走势的六态观察 | 六态观察 / `six_state(same_context)` | 六标签及完整center/p/B3/S3证据 | 给定Context∃!不证raw→Context；36转移编码非可达性G015 |
| CC-027 递归构造级别与停止 | 递归构造 / `compose_next(real_level)` | 基底/上载/停止及三种级别身份 | 级别差、真实血缘及自然停止；资源耗尽独立知识状态 |
| CC-028 跨级同判据与真实父子 | 跨级判据 / `judge_same_predicate(real_units)` | 各级同一输入判定及父子见证 | 真实构造/包含/级差与同输入同输出G009 |
| CC-029 描述中枢形态与正式对象名分 | 只读名分 / `describe_center_or_analogy(object)` | 正式/基底类比/PH描述具名角色 | 形态和PH类比无生产权，不立额外穷尽分档 |
| CC-030 背驰比较对象与适用域 | 背驰配对 / `structural_pairs(view)` | 趋势b/c、盘整首/重复比较和内部段语境 | 先结构后力度，完整候选配对G016 |
| CC-031 趋势背驰五结构条件签名 | 背驰结构 / `trend_pair_conditions(pair)` | 5条件全32签名及失败集合 | Extreme等逐项保留，不从未来NE倒推条件 |
| CC-032 力度样本与数值全分类 | 力度 / `force_velocity(real_strokes)` | 接口空/非空无效/有效单笔/有效多笔四分；仅有效域给首末端点和L | 接口域I→语义D_L→比较D_pair分开；有效单笔L=0；有效多笔符号三支，空/无效无语义L |
| CC-033 力度比较、结构背驰与代理状态 | 力度比较 / `compare_force(pair)` | LT/EQ/GT与独立Div/代理结果 | L相对变化不等绝对强度；代理失败不可换判断 |
| CC-034 MACD辅助完整逻辑签名 | MACD观察 / `macd_aux(pair,context)` | 趋势16/盘整8签名，各维OR/T4适用域 | 同色带符号面积采样；非sumabs；不得声称与Δv普适等价 |
| CC-035 Div、NE、Turn及高级结果的分轴 | 转折因果 / `div_ne_turn_evidence(move)` | 16原始签名及各独立证据/获知时点 | Div⇒NE/Turn非循环桥G019，非16均市场可达 |
| CC-036 所有已发生转折的级别二分 | 小转大 / `realized_turn_relation(q_turn)` | r=q或r<q及真实q/r对象 | 仅给已发生转折；不要求预知未来的充分条件 |
| CC-037 小转大必要条件和发生的合法组合 | 发生 / `necessary_outcome_pair(target)` | r<q上下文三合法N/X组合；r=q不属此X；UP末中枢三卖、DOWN三买 | X⇒N仅该小转大域，N不推出X；同级转折不能因无N被误拒 |
| CC-038 小背驰后的局部中枢出口和相邻关系 | 小转大局部 / `local_exit_overlap(small_div)` | 3出口×2重叠局部签名 | 本地几何与真实q转折证据分开G020 |
| CC-039 小转大答疑与操作落点的观察轴 | 操作观察 / `reference_price_observations(turn)` | 全部12参考价原子关系 | 原子比较可展示；不发明最强/最弱交易准入档 |
| CC-040 背驰转折后的三个高级结果 | 转折结果 / `realized_outcome(turn,reading,horizon)` | 三结果、高级对象、回拉极值/外缘关系 | 反向趋势level≥q；最弱可只触DD/GG；固定分解语境G022 |
| CC-041 买卖点六谓词的完整签名 | 买卖点 / `all_six_predicates(level_view)` | 64编码、37同类B/S同真排除、至多27抽象允许；逐点和LevelView分载体 | LevelView每类一个候选且Bk/Sk互斥；27不等全部可构造，逐点列表须另证投影G023 |
| CC-042 同端同级同side买卖标签合法集合 | 买卖点组合 / `local_bsp_set(endpoint,q,side)` | 空/1/2/3/2+3五合法集合 | 约束仅同端同级同side，不全LevelView排斥 |
| CC-043 第一类点完整条件族 | 一类点 / `type1_conditions(candidate)` | 每方向8条件签名及本级核心/背驰 | 全部真实结构条件，不以Weak代理替完整一类 |
| CC-044 第二类构成结构的次级序列与首个后继 | 二类点 / `type2_descend_sequence(parent)` | q−1 m1/i1、i2=i1+1/m2及全部构成见证 | 索引属于descend(parent)；faithful和双向桥G024/G032未证 |
| CC-045 第三类点的严格端点与首回试 | 三类点 / `type3_conditions(center,leave,retest)` | 接口I/完整编码E96/完成域24/正式首回试6/确认2及strict端点 | 72未完成码不确认；18已完成非首/非离开不成立；2严格成立码，缺值走CC054 |
| CC-046 买卖点确认与结构归因 | 确认因果 / `bsp_confirmation(prefix)` | 未确认/确认及所依赖完成对象 | 前缀单调与修订代际分开G004 |
| CC-047 区间套坐标与几何包含 | 定位坐标 / `interval_nesting(parent,child)` | 26索引×26价格×3级别几何码、实际血缘 | 几何码不是全部可达；独立选父价格挂法G025未裁 |
| CC-048 Cand元素三条件全签名 | 定位候选 / `candidate_elements(conditioned_domain)` | Comparable/Direction/Extreme全8签名 | Cand不含Weak；输入先按上层选区间条件化 |
| CC-049 Cand集合非空和Sel选择 | 定位选择 / `select_from_complete_candidates(C,policy)` | 空/单/多及完整C、Sel和版本 | 固定Sel先选s再判适用G(s)；其他候选成功只读，不重选或救回失败G026 |
| CC-050 区间链的完整约束签名 | 定位证书 / `validate_chain(chain,context)` | 公式化全边/rung签名、Confirm、失败集合 | 空链另域；三骨架和N/A处理不能严失败转宽G027 |
| CC-051 Type2/3的区间套适用域 | 定位适用性 / `rung_applicability(point_type)` | Type1适用/Type2不适用/Type3不适用 | N/A不是true；仍验证真正次级构成义务G028 |
| CC-052 下钻选择后判定与全候选观察分轴 | 定位知识 / `complete_descent_result(request)` | 选中请求四分、全集观察四分、同G时五合法联合；带predicate_id | selected=false/any=true合法并存；证书仅消费selected_gate，不从观察any成功补证 |
| CC-053 下钻成本门与塔底停止 | 定位停止 / `descent_stop(cost,base)` | 继续/仅成本/仅基底/两者四签名 | OR双触发不first-match；成本政策未齐单列G029 |
| CC-054 数据、适用域、计算完成和结果有效性 | 结果元信息 / `result_qualification(request)` | 输入3×域3×完成2×有效性3元信息 | 54原始元码不填市场叶；旧有效切面与新未完成可并存 |
| CC-055 全对象关系与变更事件身份 | 全对象变化 / `typed_relations_and_delta(revision)` | 12具名关系族/4delta形式与全部前后对象 | 逐轴见证和全事件闭包G030；不把关系名enum当本体穷尽 |
| CC-056 唯一归类和对象唯一身份的不同义务 | 对象身份 / `object_version_identity(input,role)` | 追加/输入修订/规则参数变化及稳定ID | 分支唯一与对象ID唯一不同；tie/读法证明G002/G030 |
| CC-057 多级联立与共振 | 跨级观察 / `joint_level_facts(cut,levels)` | 编码64^m；各级均属LevelView时最多27^m抽象允许；载体和共振见证 | 每级同类B/S互斥后再核共享血缘同知G031；逐点列表不能直接套27^m |
| CC-058 结构到持久重经营的接口名分 | 结构经营桥 / `consumption_links(structure,decisions)` | 结构事实/经营上下文/经济外效角色及全引用 | 三角色可共存，仅接口；35FG与工程状态另承接 |
| CC-059 二类点转折锚有无同级一类的完整补充分支 | 二类补充分支 / `type2_anchor_case(q_turn,sub_roundtrip)` | 有无q一类×side×比较×盘背24签名 | 053:28不创新极值含EQ或盘背；不伪造q一类，真构成桥待证 |
| CC-060 点相对中枢的位置三态 | 位置观察 / `point_position(center,p)` | Below/Within/Above且边界含等 | 形式Center.valid可退化，正式center仍strict；非交易FSM |
| CC-061 中枢结束后两种发展结果 | 中心后续结果 / `center_end_outcome(center,reading,horizon)` | 高级中枢/新同级中枢结果及构造证据 | 三类先结束后果再形成；不与29三结果全局互斥G022 |
| CC-062 同级已完成走势连接和读法约束 | 读法连接 / `completed_adjacency(T1,T2,reading)` | 9类型对、禁U→U/D→D、余7及本读法证据 | 操作旁路P→P保留；主塔可达另证G008/G012 |

每行还须完成 raw→良构域、实现↔谓词、合法组合可达、转换排除/可达、同输入身份与回放的各自证据。现有局部Lean声明、枚举或测试样例不代替这些义务；本轮未运行项目/Lean。

## 组合及发布

| 合同 | 完整组合约束 |
|---|---|
| LC-01 | 每一具体几何输入落唯一原子叶；包含、闭重叠、gap为该叶的确定投影，允许多标签，不以优先级删去包含且重叠。 |
| LC-02 | 四分型→新笔条件→成笔/确认→线段条件→两情况，每层只有先证明输入属D才判断下一层；上一层失败不能改用另一宽档。 |
| LC-03 | 正式center只有strict seed；核心冻结；单元离开、首回试三类/破坏、pair扩展、新生、九段处理分对象可同次更新并发出，九段优先级只作用其已裁互竞构造决策，不抹别的关系事实。 |
| LC-04 | 完成三类型、形成完成继任、六态、点位置三态分别字段；由同context一致性约束绑定。任何一个标签不替代另外三个轴。 |
| LC-05 | 结构pair/五前提→精确力度→Div，代理只读；NE/Turn及高级结果有独立见证，不作Div循环前提。 |
| LC-06 | r=q/r<q只分类已发生q转折；末次级三类是必要而非充分；局部中心出口/重叠与高级结果独立，保留未发生转折继续原趋势分支。 |
| LC-07 | Origin LevelView逐类满足∀k¬(Bk∧Sk)，64编码仅至多27个已知约束允许签名；同端同级同side仍仅空、1、2、3、2+3，两个限制作用域不同。不同类别可不同端点反向并存；无同级一类的二类支必须覆盖。Rust逐点bits列表、Γ逐候选六位不自动构成每级LevelView；其投影/可达性另证。 |
| LC-08 | 真实父子/几何包含→条件化Cand全集→固定Sel选中s→完整链及适用门G(s)/叶Confirm；全候选ANY_GATE_TRUE仅只读观察，不得替选中失败或触发重选。不是每个rung都新增Weak。不适用不填真、停止两触发可同真、下钻失败不是小转大。 |
| LC-09 | 观察描述、MACD、计算知识/有效性均不得填补市场定义分支或直接升级生产准入；保留可见性而不造新阈值。 |
| LC-10 | 唯一主塔、多操作读法、定位查询的身份不混；所有结构/关系/变化同源版本+known_at；经营消费不会回写主塔，跨级联合签名必须保留全部见证。 |

这些约束在 ClassificationRevision 原子发布前核；一个输入产生的多项合法关系全部保留。违法组合或证明缺失按明确失败/未证状态暴露，不能从first-match剩下一个标签就称互斥。合法转换逐轴承接目录 legal_transitions 字段，具体转换的可达性和排除证明仍是下表义务。

## 32项证明、同步及选择定位

| 项 | 名分 | 本方案承接与限制 |
|---|---|
| G-001 包含输入初始方向未建时的处理 | ENGINEERING_CHOICE_INSTANCE | 明确前缀建立方向或初始化策略及其输出影响；不得默默把默认UP当原文唯一方向。 |
| G-002 等价极值和端点身份唯一 | PROOF_OR_IDENTITY_INSTANCE | 极值同价多根、同型同价端点的稳定身份选择及原始/合并映射确定性。 |
| G-003 描述性强弱和中枢形态不升格分类 | OBSERVATION_ONLY_NOT_A_GAP | 保留来源和可观察证据；不要求本轮发明互斥数值分档。 |
| G-004 确认的跨对象前缀单调桥 | PROOF_BOUNDARY | 以原始合法追加前缀证明笔/段/Move/BSP确认不倒退，修订另代际。 |
| G-005 三笔相切已裁，旧正本登记未同步 | RULING_SYNC | 遵闭公共重叠；同步旧“未裁”登记及核对受影响实现，当前仅目录报告不改仓。 |
| G-006 特征序列两情况、同向第二序列和G4封闭的统一桥 | DEFINED_SEMANTICS_PROJECTION_ALIGNMENT | 第二序列必须正确笔集合且接受任意分型；独立包含作用域和strict封闭对象须统一对齐；不能以旧提取revSeq或布尔case穷尽替代源语义。 |
| G-007 第二特征序列扫描窗口 | EXPLICIT_PENDING_OR_STALE_IMPLEMENTATION_CHOICE | 核查后续实施/裁定是否已统一50与完整扫描；任何预算耗尽都不得判为结构否定。 |
| G-008 唯一主塔与多读法的分解全覆盖 | PROOF_OR_ALGORITHM_INSTANCE | 给主塔及固定三格旁路各自Φ、未分配尾部、无重复归属和可达连接证明，不要求二者逐字段相同。 |
| G-009 真实递归输入到中枢与外缘的桥 | PROOF_BOUNDARY | 原始事实→真实完成次级单元→strict seed→全成员外缘→父子级差/覆盖→各级同判据；局部几何定理不能替这条链。 |
| G-010 第三类严格端点与旧文字/实现张力 | FORMAL_ENDPOINT_DEFINED_SYNC_TENSION | 把Origin严格端点作为本目录边界，后续核并同步文字与实现；不得同时放行inclusive宽档。 |
| G-011 升级计数的无波动排除实例 | ENGINEERING_PREDICATE_INSTANCE | 给无波动判法的可执行实例及其与源语义等价证据；不凭描述自定额外阈值。 |
| G-012 三走势的真实良构域覆盖 | PROOF_BOUNDARY | 证明完成对象总落恰一枢盘整/同向多枢趋势之一；多枢flat等输入必须正确升层或重分解。 |
| G-013 盘整无方向与Compose技术方向 | RULING_SYNC_OR_COMPATIBILITY_RETIREMENT | 技术break_direction与语义type分开，核既有兼容字段退场和下游消费者。 |
| G-014 一般Move完成和Turn窄Completed的桥 | PROOF_BOUNDARY | 一般完成含小转大、盘整等语境；不能用Completed=Div的窄模型排除其他完成。继任须真实同级且不同身份。 |
| G-015 原始结构投影到位置/六态Context | PROOF_BOUNDARY | 证明center,p,b3,s3同源、绑定正确center且当时可知；静态3/6态定理不提供顺序交易FSM。 |
| G-016 b/c结构配对和发展态投影 | PROOF_BOUNDARY | 真实走势按最后中心进入b及后续c、盘整最近跨界对象配对；先结构后力度，当前前缀因果。 |
| G-017 力度已有定义，实际样本桥待证 | EXPLICIT_DEFINITION_SAMPLING_BRIDGE | 如何由每个实际发展段给首/末笔和四端点，并满足WellFormed和同坐标；空/零duration总化不得冒充语义样本。 |
| G-018 MACD辅助和精确力度的适用边界 | PROOF_BOUNDARY | 三维OR、T4域、符号面积采样对齐；不能以近似来源声称MACD与Δv在任意行情等价。 |
| G-019 Div⇒NE/Turn的非循环实现桥 | PROOF_BOUNDARY | 不得把NE/Turn当前/未来事实当前件循环定义Div；提供EM等实际桥的定义域及证据。 |
| G-020 小转大已经发生的定位见证 | PROOF_BOUNDARY | 真实q转折、r<q背驰、末次级中心相应三类必要条件均可定位；不能把候选/下钻失败当发生。 |
| G-021 操作描述的边界不冒充结构分类 | OBSERVATION_ONLY_OR_SEPARATE_OPERATION_CHOICE | 参考价LT/EQ/GT全部可展示；本轮不立最强/普通/最弱交易档，也不在FG之外增加价格准入。 |
| G-022 高级结果分解和回拉外缘/核心 | PROOF_BOUNDARY_AND_SOURCE_TEXT_TENSION | 兑现29三结果与53中心结束二结果的不同域；029:30最弱触DD/GG与旧beichi:651必回核心冲突按原文优先，不作同级整齐对齐。 |
| G-023 64编码、27约束允许上界及实际LevelView/逐点载体桥 | PROOF_BOUNDARY | 明确载体；LevelView必须保持∀k¬(Bk∧Sk)，并与局部同端同side五集合一致。补真实市场投影/逐点列表聚合与所有声称可达签名的见证，不能再说六位语义无条件独立。 |
| G-024 二类构成形与可观测形双向桥 | PROOF_BOUNDARY | SubOf反自反/同side/真实次级归属；构成⇒观测和观测⇒构成皆需在实际实例证明，常规after&&notbroke不能替代原文所有二类。 |
| G-025 独立选父NestInterval的价格挂法 | EXPLICIT_SEMANTIC_CHOICE_PENDING | 按既有#817残留明确价格挂法，再验证跨级price-subset；不得用真实descendant的min/max单调性跨域。 |
| G-026 候选全集、条件集合与Sel实例 | PROOF_OR_SELECTOR_INSTANCE | 实例化Comparable/Extreme的对象投影、候选全集完整性与确定Sel；多候选全保留，选择不可越集合。 Sel后失败必须保留；候选全集任何真值观察不得重选或替换适用的selected gate。 |
| G-027 三套链骨架的有效域和证书桥 | PROOF_BOUNDARY | 明确每套骨架D、Confirm内外和成员/层级不变量；跨骨架等价或退场条件需真正证明/验收。 |
| G-028 Type2/3上级rung不适用的n_delta处置 | EXPLICIT_CERTIFICATE_HANDLING_PENDING | 按1106仍需单独裁的处理来统一证书；不适用不得填true或因false翻掉所有Type2/3。 |
| G-029 成本停止门参数和基底进阶条件 | ENGINEERING_POLICY_INSTANCE | 具体成本量与任务policy版本，当前底线段；阶段二必须满足原定两层条件，非本轮实施。 |
| G-030 全对象转换与版本因果闭包 | PROOF_BOUNDARY | 给每个可达转换的来源、不能发生转换的否证、事件多发不丢失、稳定身份和按当时可知回放；本目录仅列来源规则/必要约束，未证明全局可达图。 |
| G-031 多级联合签名合法可达性 | PROOF_BOUNDARY | 共享血缘和同知时点下，LevelView联合像必须先落A_27^m（每类买卖互斥），再证明其他跨级约束；原始编码64^m与上界27^m均不是可达性证明。生产逐点集合另需投影。 |
| G-032 小转大无同级一类仍有二类 | SOURCE_SEMANTICS_DEFINED_COMMON_MODEL_INCOMPLETE | 分别给q级转折锚、q-1构成一类m1、同descend序列后继m2，以及053:28一般往返的对象映射；拒绝伪造q级Type1事件来初始化i1。 |

G-003 为观察名分而非缺失分类；G-021 不要求新交易档。已裁的三笔闭重叠、严格三类端点与力度公式继续遵从原裁，仅保留同步/实例桥。G-025独立选父价格挂法、G-028 N/A证书处理等具体残留不由本方案自选默认。以上32项不等于32个新取舍。

本附录只完成设计绑定和消费路径。当前生产入口是否实际计算、逐字段前端能否达到、全转换和原始域完整性均未由本轮运行验证。工程协议另见 [状态合同](independent-design-state-contracts.md)，不把CC-058的三种角色充当35FG经营全分类。
