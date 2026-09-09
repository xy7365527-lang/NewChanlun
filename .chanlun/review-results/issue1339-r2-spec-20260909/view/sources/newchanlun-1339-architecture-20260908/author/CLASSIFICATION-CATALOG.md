> 链接转换阅读版；正文语义与原稿一致。签署原字节：[原稿](../../../../payload/sources/newchanlun-1339-architecture-20260908/author/CLASSIFICATION-CATALOG.md)，SHA256 `41fe65519c011fc173b47af5ee3cc721288b2de62219059fa9e69822135eccf8`。当前批准状态见归档根APPROVAL.json；原稿中的待审状态保留为发生时记录。

# #1339 完全分类目录（结构范围，r2）

本目录为当前批准结构范围的完全分类义务目录，包含已定义分区、完整逻辑签名族、允许组合和转换约束；不是原始行情到所有可达语义状态的已完成形式证明。

本稿有 **62 个稳定分类/关系/观察轴记录**，覆盖 **44 个结构聚合需求**。叶分支使用显式枚举或有限可生成公式族；不是用62或44当作完备证据。35FG及RF-01由经营工作线给完整目录，本稿仅处理结构接口。

原文管语义，Origin定义式管端点与退化；已有精确定义/裁定优于旧文字未裁登记。代码历史注记不充当当下实现事实。

## 验证边界

不能以80聚合条目、62轴数量、Boolean函数total/unique或笛卡尔积编码代替定义域覆盖、语义互斥、对象唯一和转换可达性证明。

除附录明确有限枚举自检，本轮未运行lake/cargo/项目测试、真实交易或全量回放；未写仓内定义、GitHub或候选封存原稿。

最需要避免的三种误读：64个买卖谓词签名不等于64个语义合法态；给定Context六态唯一不等于原始行情投影已证；没有未来充分预测条件不等于已发生转折的分类缺失。中枢形态和分型强弱描述保留可见性，但不会由本稿升格为新的强制互斥数值档位。

## 关键订正

- 三笔相切是已裁闭重叠；正式中枢仍须严格非退化。
- 第三类端点在Origin已有严格定义；旧inclusive文字/实现属于待对齐张力。
- ForceVelocity已有速度及力度定义；剩余是实际结构样本投影。
- 二类点必须保留小转大“无同级一类”的原文支，不能用常规afterTypeOne路径拒绝。
- 区间套1317价格挂法与1106不适用rung处理是具体残留，不能被1266“未裁为空”总标题覆盖。

## 完整轴目录

### CC-001　有序值的原子三分

**范围**：ST-001, ST-003, ST-004, ST-006, ST-011, ST-014, ST-024, ST-030, ST-031, ST-034。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：同一单位、坐标与版本下两个可比较值x,y；NaN、缺值、币种或坐标不一致不在D。

**完整分支 / 生成族（数量 3）**：

- `LT`：{"id":"LT","predicate":"x<y"}
- `EQ`：{"id":"EQ","predicate":"x=y"}
- `GT`：{"id":"GT","predicate":"x>y"}

**合法组合 / 排除**：任意复合不等式必须是这三支的显式并集；≤=LT∪EQ，>=EQ∪GT。

**转换**：数值随新事实可在任意三支间迁移；已封存版本值不回写，修订生成新版本。

**后端输出 / 前端观察**：返回比较两值、单位、坐标、等号命中及源对象；UI不得把相切画成严格分离。

**证明状态**：ALGEBRAIC_PARTITION_ONLY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/结构判据.md:9-24；formal/Origin/SegmentFeatureSeq.lean:101-127。

### CC-002　两闭区间全端点关系

**范围**：ST-001, ST-007, ST-008, ST-011, ST-013, ST-014, ST-034。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：a.low<=a.high且b.low<=b.high，四端点同有序域；允许单点区间。

**完整分支 / 生成族（数量 26）**：

四端点的有序等价类：r∈{0,1,2,3}^4，已用rank为0..max(r)，r0<=r1且r2<=r3；每一叶按rank指定全部=和<关系。

26个叶的四端点rank向量：(0,0,0,0)；(0,0,0,1)；(0,0,1,1)；(0,0,1,2)；(0,1,0,0)；(0,1,0,1)；(0,1,0,2)；(0,1,1,1)；(0,1,1,2)；(0,1,2,2)；(0,1,2,3)；(0,2,0,1)；(0,2,1,1)；(0,2,1,2)；(0,2,1,3)；(0,3,1,2)；(1,1,0,0)；(1,1,0,1)；(1,1,0,2)；(1,2,0,0)；(1,2,0,1)；(1,2,0,2)；(1,2,0,3)；(1,3,0,2)；(2,2,0,1)；(2,3,0,1)。等价类序列和13个strict子域叶已逐项存JSON。

**合法组合 / 排除**：严格非退化子域r0<r1且r2<r3恰13叶；闭重叠=a.low<=b.high且b.low<=a.high，缺口为其补；包含为端点内含。三者不是一个互斥枚举。

**转换**：端点变化按全部26×26有序叶对记录；是否可由同一结构对象的合法延展到达另验构造规则，不能由几何可能推出可达。

**后端输出 / 前端观察**：返回全部四端点顺序叶，以及可同时成立的contains/overlap/gap投影和作用域。

**证明状态**：ALGEBRAIC_PARTITION_ONLY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：formal/Origin/SegmentFeatureSeq.lean:101-145；.chanlun/definitions/zhongshu.md:66-101。

### CC-003　相邻K线包含和无包含方向

**范围**：ST-001。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：时序相邻、同坐标、high>=low的K线a,b。

**完整分支 / 生成族（数量 9）**：

- **generator**：(cmp(b.high,a.high),cmp(b.low,a.low))∈{LT,EQ,GT}²，9叶。
- **projection**：GT/GT=向上非包含；LT/LT=向下非包含；其余7叶=包含。EQ/EQ两向包含同时真；不可先选一向后删除事实。

**合法组合 / 排除**：包含域才做合并；非包含双高双低域上，单条件与双条件方向等价。任一同高/同低均属于包含，不能另设平K方向。

**转换**：按左到右处理；非包含追加并更新方向；包含合并后重检相邻，禁止任意括号重排。

**后端输出 / 前端观察**：显示原始两K和9叶、包含方向两个位、有效合并方向及来源。

**证明状态**：ALGEBRAIC_PARTITION_ONLY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/baohan.md:86-144。

### CC-004　包含折叠输入及方向来源

**范围**：ST-001, ST-002。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：D是一次可定位的左折叠处理步：同代际有效K线前缀长n；n=0/1为初始化，n>=2须给当前待处理K线k、此前已处理部分的有效非空累加器尾a及方向来源dir∈{UP,DOWN,None}。dir由既有非包含方向或已显式声明的初始化策略给出。不能取得a的未完成请求走CC-054，不虚构合并状态。

**完整分支 / 生成族（数量 7）**：

- `EMPTY`：{"id":"EMPTY","predicate":"n=0","output":"空累加器"}
- `SINGLE`：{"id":"SINGLE","predicate":"n=1","output":"第一根原样成为累加器"}
- `APPEND_UP`：{"id":"APPEND_UP","predicate":"n>=2且a,k非包含且k.high>a.high且k.low>a.low","output":"追加k为新merged bar，建立/更新dir=UP"}
- `APPEND_DOWN`：{"id":"APPEND_DOWN","predicate":"n>=2且a,k非包含且k.high<a.high且k.low<a.low","output":"追加k为新merged bar，建立/更新dir=DOWN"}
- `MERGE_UP`：{"id":"MERGE_UP","predicate":"n>=2且a,k包含且dir=UP","output":"高高合并，保留明确方向来源"}
- `MERGE_DOWN`：{"id":"MERGE_DOWN","predicate":"n>=2且a,k包含且dir=DOWN","output":"低低合并，保留明确方向来源"}
- `INITIAL_DIRECTION_UNESTABLISHED`：{"id":"INITIAL_DIRECTION_UNESTABLISHED","predicate":"n>=2且a,k包含且dir=None","output":"未具备唯一合并方向；显式记录初始化选择/待建立，不默填UP为教义结果"}

**合法组合 / 排除**：CC-003已证明非包含域只有双高UP/双低DOWN，所以n>=2先按包含/非包含互补域切开；UP/DOWN合并只在包含域。dir=None不阻止非包含追加建立方向。包含域dir=None时不能先合并再冒称已有方向；默认UP若使用须显式列为初始化策略。

**转换**：EMPTY→SINGLE；每次n>=2步在APPEND_UP/APPEND_DOWN/MERGE_UP/MERGE_DOWN/INITIAL_DIRECTION_UNESTABLISHED恰一支。非包含追加更新方向并可解除尚未建立方向；包含合并保留组成员和既定方向来源；方向未建立请求只有得到有依据的初始化/追加状态后继续。

**后端输出 / 前端观察**：输出当前处理raw索引、累加器尾、包含判据、七叶、追加或合并动作、方向值及建立/策略来源；两根非包含K不得丢失或误报初始化缺口。

**证明状态**：DEFINED_WITH_UNINSTANTIATED_CHOICE。定位项：G-001。

**证据**：.chanlun/definitions/baohan.md:115-155。

### CC-005　三种坐标及组映射

**范围**：ST-002, ST-004, ST-034。**轴名分**：ROLE_OR_RELATION_SCHEMA。

**D**：同一原始行情代际内的一条包含组及其成员。

**完整分支 / 生成族（数量 3）**：

- `RAW_MEMBER`：{"id":"RAW_MEMBER","predicate":"原始K标识和时序"}
- `MERGED_GROUP`：{"id":"MERGED_GROUP","predicate":"包含处理后组标识和组内首根raw锚"}
- `ACTUAL_EXTREMUM_MEMBER`：{"id":"ACTUAL_EXTREMUM_MEMBER","predicate":"极值实际出现的raw成员集合"}

**合法组合 / 排除**：三支是坐标角色而非对象只能取其一；映射raw→group单值、group→members全量；组首raw锚不等价于极值raw根。极值多根必须保留集合直至身份选择有依据。

**转换**：组扩展更新成员映射；稳定raw身份不因merged重编号更换；修订重算要新代际。

**后端输出 / 前端观察**：所有索引声明raw/merged/极值角色，显示组成员并可定位实际极值；不能靠一个idx装三者。

**证明状态**：MAPPING_AND_IDENTITY_BRIDGE_REQUIRED。定位项：G-002。

**证据**：.chanlun/definitions/baohan.md:164-188；.chanlun/definitions/bi.md:275-316。

### CC-006　无包含三K的四种局部形态

**范围**：ST-003。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：三根顺序标准K；每对相邻均非包含、指标值完整。

**完整分支 / 生成族（数量 4）**：

- `RISING`：{"id":"RISING","predicate":"dir(a,b)=UP且dir(b,c)=UP"}
- `TOP`：{"id":"TOP","predicate":"dir(a,b)=UP且dir(b,c)=DOWN"}
- `BOTTOM`：{"id":"BOTTOM","predicate":"dir(a,b)=DOWN且dir(b,c)=UP"}
- `FALLING`：{"id":"FALLING","predicate":"dir(a,b)=DOWN且dir(b,c)=DOWN"}

**合法组合 / 排除**：顶=中K的high和low均严格高于两边；底完全对偶。等高/等低违反D中的非包含，进入输入域诊断，不造第五“其他分型”。

**转换**：滑窗a,b,c→b,c,d重新判；少于三K为输入不足。窗口判顶/底不等于后续形成有效笔。

**后端输出 / 前端观察**：显示四类之一、四个严格比较与对应三根原始/merged见证。

**证明状态**：ALGEBRAIC_PARTITION_ONLY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/fenxing.md:27-58；.chanlun/definitions/fenxing.md:94-126。

### CC-007　分型描述与已实现后续发展

**范围**：ST-045。**轴名分**：OBSERVATION_OR_KNOWLEDGE。

**D**：已经成立的顶或底分型；描述标签有原文依据，未来事件按当时可知。

**完整分支 / 生成族（数量 {"axis_A":2,"price_relation":3,"descriptive_partition":null}）**：

- **family_A**：顶/底两支（CC-006）；后续已观察的任意选定参考价关系用CC-001三支。
- **family_B**：强/弱、中继以及长阳/小阴阳是描述词，现正本没有使所有合法输入唯一落入这些词的数值阈值，不能给虚构有限叶数。

**合法组合 / 排除**：描述词不作生产准入；分型强弱和其后是否继续/反转不等价，也不以结果倒填当时强弱。 本轴不要求把强/弱/中继变为强制二分或三分；该描述保留可见性，不计作现行结构分类缺口。

**转换**：标签的获知时点和后续事件时点分开；无正式描述分类器前只能给原始证据或有出处的人工描述。

**后端输出 / 前端观察**：展示顶底已定义分类；强弱描述标明规则未形式化，不以Unknown补齐一个声称已完备的轴。

**证明状态**：OBSERVATION_ONLY_NOT_REQUIRED_EXHAUSTIVE_TAXONOMY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/fenxing.md:64-88。

### CC-008　新笔候选的完整条件签名

**范围**：ST-004。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：同代际、时间先后的分型端点f,g；先以端点同型/异型分域。

**完整分支 / 生成族（数量 18）**：

- **same_type**：TOP/TOP或BOTTOM/BOTTOM两叶，由CC-009处理，不进入生产成笔判据。
- **opposite_type**：BOTTOM/TOP或TOP/BOTTOM两方向；各方向按(m>=3, raw_between_actual_extrema>=3, top_price>bottom_price)∈{0,1}³生成8叶。111才成笔；其余7叶完整保留全部失败位。

**合法组合 / 排除**：m为merged中心距离，不共用三K；raw间隔排除实际极值两端；顶必须高于底。旧笔不能作为111失败后的fallback；参数3是原文标准，不能隐式调宽。

**转换**：同型去重后重新考察异型端点；新数据可改变候选尾部，已确认笔的变化遵CC-010和版本轴。

**后端输出 / 前端观察**：返回两个方向/同型叶、三个条件位、merged和raw距离、顶底价，全部失败原因而非firstfail。

**证明状态**：ALGEBRAIC_PARTITION_ONLY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/bi.md:100-121；.chanlun/definitions/bi.md:275-316。

### CC-009　同型分型去重与极值身份

**范围**：ST-005。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：同方向的两个候选顶或两个候选底，时序已知。

**完整分支 / 生成族（数量 6）**：

- **generator**：kind∈{TOP,BOTTOM}×cmp(new.extreme,old.extreme)∈{LT,EQ,GT}；6叶。
- **rule**：TOP取更高、BOTTOM取更低；EQ价格唯一但raw端点身份tie需单独说明选择证据。

**合法组合 / 排除**：同价不同时间不能以价格相等推身份相同；组内多次极值与同型端点并列是两种tie。

**转换**：严格更极端→替换当前候选；严格较弱→保留；EQ不得无来源地声称首/末唯一。

**后端输出 / 前端观察**：记录被保留和被替代候选、价格判据、身份tie与策略版本。

**证明状态**：PRICE_PARTITION_DEFINED_IDENTITY_GAP。定位项：G-002。

**证据**：.chanlun/definitions/bi.md:163-210；.chanlun/definitions/bi.md:380-420。

### CC-010　笔形成和确认

**范围**：ST-005, ST-019, ST-032。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：按生产新笔规则得到的有序笔实例；尾部可发展。

**完整分支 / 生成族（数量 2）**：

- `FORMED_UNCONFIRMED`：{"id":"FORMED_UNCONFIRMED","predicate":"成笔成立且对应终结/确认见证尚未成立"}
- `CONFIRMED`：{"id":"CONFIRMED","predicate":"成笔成立且对应确认见证成立"}

**合法组合 / 排除**：未成笔属于CC-008失败输入，非第三种笔。formed不等于confirmed；确认的共同语义必须给原始前缀不可逆见证，不能只因数组非末尾。

**转换**：合法追加下未确认可延伸并最终确认；同一代际已确认不得退回；行情纠错需要撤回/新版本，非语义倒退。

**后端输出 / 前端观察**：尾笔虚实、延伸端点、确认依据及known_at，提供前缀重放。

**证明状态**：DEFINED_LIFECYCLE_WITH_PREFIX_PROOF_REQUIRED。定位项：G-004。

**证据**：.chanlun/definitions/bi.md:163-210；.chanlun/definitions/level_recursion.md:329-340。

### CC-011　线段构造候选条件

**范围**：ST-006, ST-048。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：同代际的连续笔序列片段；使用P2特征序列正本。

**完整分支 / 生成族（数量 {"length":5,"conditional_masks":16,"valid_direction":2}）**：

- **length_partition**：n=0,1,2；n>=3偶数；n>=3奇数，共5叶。
- **for_odd_ge3**：(方向交替,前三笔闭公共重叠,顶>底,首尾同向)∈{0,1}⁴，共16条件签名；仅1111为有效候选，首尾条件可由奇数+交替推得但仍保留证明。
- **direction**：有效候选按首笔UP/DOWN两叶。

**合法组合 / 排除**：三笔闭公共重叠=max(low_i)<=min(high_i)，EQ相切已裁算重叠。正式中枢仍严格<，不得把两种对象的端点规则统一掉。无重叠只是不能作为该段起点，后续可重新起算。

**转换**：候选只有满足构造条件才成段；笔破坏/第一第二特征序列确认决定旧段终结，参CC-012..014。

**后端输出 / 前端观察**：列全部构造条件、前三笔共同区间及相切叶、方向、起止笔ID。

**证明状态**：PARTITION_DEFINED_RULING_SYNC_GAP。定位项：G-005。

**证据**：.chanlun/definitions/xianduan.md:23-76；.chanlun/definitions/xianduan.md:93-118；classification-evidence-issue-246.json；classification-evidence-issue-249.json。

### CC-012　特征序列作用域与标准化

**范围**：ST-007。**轴名分**：ROLE_OR_RELATION_SCHEMA。

**D**：一个已有方向的线段候选及所取特征元素。

**完整分支 / 生成族（数量 3）**：

- `FIRST_SEQUENCE`：{"id":"FIRST_SEQUENCE","predicate":"旧段取其反向笔；UP段取下降笔，DOWN段取上升笔"}
- `SECOND_SEQUENCE`：{"id":"SECOND_SEQUENCE","predicate":"第一序列有缺口时，从相应极值后的反向走势取其特征序列"}
- `CROSS_SEQUENCE`：{"id":"CROSS_SEQUENCE","predicate":"两元素不属同一序列，几何关系可算但包含合并不适用"}

**合法组合 / 排除**：只能在相同sequence_id内按同一包含规则合并；转折点两边不合并。第二序列不是第一序列剩余元素的改名，也不默认只找对偶分型。

**转换**：先按序列独立标准化→找分型；若第一有缺口才启动第二；新事实追加不跨作用域吸收。

**后端输出 / 前端观察**：显示first/second身份、极值锚、原笔映射和包含组，禁止UI只画一条混合特征线。

**证明状态**：SCOPE_DEFINED_PROJECTION_BRIDGE_REQUIRED。定位项：G-006。

**证据**：.chanlun/definitions/xianduan.md:118-187；.chanlun/definitions/xianduan.md:248-286；formal/Origin/SegmentFeatureSeq.lean:137-145。

### CC-013　线段终结的两情况与完整条件族

**范围**：ST-007, ST-008。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：足以观察当前第一特征序列窗口的线段候选；局部窗口合法。

**完整分支 / 生成族（数量 {"raw_masks":32,"semantic":5}）**：

- **generator**：原始逻辑编码(first_fractal,gap,second_fractal,top_gt_bottom,G4_closure_satisfied)∈{0,1}^5共32位签名；语义求值须带适用性：无第一分型时case选择不适用；无gap时第二序列不适用，且不凭虚填Bool扩大语义域。适用分支按下面5叶；G4准确封闭对象/阈值对齐欠证单列G-006。
- **semantic_branches**：[{"id":"NO_FIRST","when":"无第一序列方向要求的分型","effect":"旧段续"},{"id":"CASE_ONE","when":"第一分型且无缺口且顶>底","effect":"第一情况终结"},{"id":"CASE_TWO","when":"第一分型且有缺口且第二序列任意分型且顶>底且已满足G4封闭","effect":"第二情况终结"},{"id":"PENDING_CASE_TWO","when":"第一分型且有缺口且顶>底，且所需第二序列/封闭见证未齐","effect":"尚无终结见证"},{"id":"FAILED_GEOMETRY","when":"第一分型但顶>底不成立","effect":"不成为有效终結"}]

**合法组合 / 排除**：gap依据闭重叠的否定：正缺口strict，touch不是gap。第二序列可以顶或底分型，不加只对偶门；“第二序列不必封第一缺口”不等于省略已裁G4。掩码只在各条件适用域求值；不适用单列。 G4条款已决定strict不能省略；但本目录尚未提供其与完整两情况构造的统一谓词桥，不能以5叶布尔分配宣称线段识别已全证。

**转换**：NONE→case1终结，或→case2等待→case2终结；一次候选失效可重扫后续窗口，不能把未扫完当不存在。

**后端输出 / 前端观察**：输出选择两情况的缺口证据、两个序列的分型与封闭证据、终结获知时点及扫描范围。

**证明状态**：FINITE_LOGIC_FAMILY_SEMANTIC_ALIGNMENT_REQUIRED。定位项：G-006。

**证据**：.chanlun/definitions/xianduan.md:118-220；.chanlun/definitions/xianduan.md:248-305；.chanlun/definitions/xianduan.md:157-176。

### CC-014　笔破坏、段破坏、扫描完整性

**范围**：ST-008, ST-048。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：同一旧线段、给定行情前缀和扫描范围。

**完整分支 / 生成族（数量 {"raw_fact_signatures":4,"scan":2,"reachable_fact_count":"待域桥证明，不能称4种均可达"}）**：

- **facts**：(笔破坏,新段形成)∈{0,1}²；段破坏=新段形成；若新段形成⇒笔破坏适用于所用构造域，则排除01，否则须给该蕴含反例或修正域，不能先删。
- **scan**：COMPLETE / INCOMPLETE_RESOURCE_BOUND两支；未覆盖无限未来为自然当前前缀边界，并非资源失败。

**合法组合 / 排除**：只有笔破坏而未发展成段破坏，旧段继续，可出现古怪线段。终结不存在只对已完整扫描的当前前缀成立；固定50等资源限制是工程预算。

**转换**：已成新段→旧段终结；未形成则旧段续；INCOMPLETE允许后续补扫恢复COMPLETE。

**后端输出 / 前端观察**：分别显示笔破坏/段破坏、扫描起止/预算是否耗尽；禁止“扫描不足⇒没有段破坏”。

**证明状态**：DEFINED_FACTS_REACHABILITY_AND_SCAN_POLICY_GAP。定位项：G-007。

**证据**：.chanlun/definitions/xianduan.md:286-346；.chanlun/definitions/xianduan.md:360-400。

### CC-015　构造、操作读法与定位读法

**范围**：ST-009, ST-033。**轴名分**：ROLE_OR_RELATION_SCHEMA。

**D**：一个结构主塔及同源读取请求。

**完整分支 / 生成族（数量 3）**：

- `CONSTRUCT`：{"id":"CONSTRUCT","predicate":"唯一主塔构造；中枢按后裁动态延伸"}
- `OPERATE`：{"id":"OPERATE","predicate":"指定操作级别的同级别分解旁路；每中心seed固定三格、禁止延伸"}
- `LOCATE`：{"id":"LOCATE","predicate":"给定上级区间主动向下定位旁路"}

**合法组合 / 排除**：这是职责角色分区，不是三种可互换结构语义；CONSTRUCT恰一份，OPERATE可N份，LOCATE可N个查询。旁路输出不回流构造。盘+盘可出现在操作读法，不能用主塔默认排除吞掉。

**转换**：主塔生成后可派生两旁路；旁路读法变化只生成新读法实例和源版本绑定。

**后端输出 / 前端观察**：返回role、source_tower_version、operation_level或target_interval、完整分解成员与未分配范围。

**证明状态**：ROLES_DEFINED_DECOMPOSITION_BRIDGE_REQUIRED。定位项：G-008。

**证据**：.chanlun/definitions/fenjie.md:1-45；docs/adr/0011-operation-decomposition-layer.md:29-95。

### CC-016　正式中枢的seed判定

**范围**：ST-011。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：给定同级候选三个连续次级别走势单元；最低正式基底取线段。

**完整分支 / 生成族（数量 {"condition_masks":32,"core_geometry":3}）**：

- **generator**：(三个均已完成,连续,同次级别,方向交替, max(low)<min(high))∈{0,1}⁵，共32签名；11111方成正式中枢。
- **geometry_refinement**：ZD<ZG / ZD=ZG / ZD>ZG三叶，后两叶均不成中枢。

**合法组合 / 排除**：数量不足3为输入不足；不是candidate center。中枢+连接+中枢与三个相邻中枢是上级构造的不同成员途径，不是两个互斥造枢准则；同一级仍用同一谓词。

**转换**：仅11111建立新正式中枢身份；失败保留构造尝试证据，不能下放宽松几何条件重判。

**后端输出 / 前端观察**：输出三个次级单元ID、级别/完成/方向/连续见证、ZD/ZG精确值及相切排除。

**证明状态**：FORMAL_PREDICATE_DEFINED_INPUT_BRIDGE_REQUIRED。定位项：G-009。

**证据**：.chanlun/definitions/zhongshu.md:41-101；.chanlun/definitions/zhongshu.md:130-157；formal/Origin/CenterConstruction.lean:107-127。

### CC-017　中枢核心和外缘变量角色

**范围**：ST-012。**轴名分**：ROLE_OR_RELATION_SCHEMA。

**D**：严格seed产生的正式中枢及全部已纳入同向Zn。

**完整分支 / 生成族（数量 3）**：

- `CORE`：{"id":"CORE","predicate":"ZD=max(seed lows),ZG=min(seed highs)，初始化后冻结"}
- `OUTER`：{"id":"OUTER","predicate":"DD=min(all dn),GG=max(all gn)，随全部Zn演进"}
- `INNER_EXTREMA`：{"id":"INNER_EXTREMA","predicate":"D=max(all dn),G=min(all gn)，独立动态统计"}

**合法组合 / 排除**：DD<=ZD<ZG<=GG。D/G不等于ZD/ZG，且D>G可能，不能拿[D,G]冒充核心；所有Zn的成员身份需同源。

**转换**：纳入Zn时按min/max更新四外缘统计，核心保持不变；有新seed才新核心身份。

**后端输出 / 前端观察**：同时显示六变量及冻结/动态属性、成员贡献、上版差异和known_at。

**证明状态**：ALGEBRAIC_PARTITION_ONLY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/zhongshu.md:102-126；.chanlun/definitions/zhongshu.md:157-195。

### CC-018　单走势单元相对中枢核心的位置

**范围**：ST-013, ST-031。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：正式中枢[ZD,ZG]且ZD<ZG、一个完整值域[u.low,u.high]。

**完整分支 / 生成族（数量 5）**：

- `STRICT_BELOW`：{"id":"STRICT_BELOW","predicate":"u.high<ZD"}
- `LOW_TOUCH`：{"id":"LOW_TOUCH","predicate":"u.high=ZD"}
- `INTERIOR_CONTACT`：{"id":"INTERIOR_CONTACT","predicate":"u.high>ZD且u.low<ZG"}
- `HIGH_TOUCH`：{"id":"HIGH_TOUCH","predicate":"u.low=ZG"}
- `STRICT_ABOVE`：{"id":"STRICT_ABOVE","predicate":"u.low>ZG"}

**合法组合 / 排除**：若u是单点且落于core内部仍INTERIOR_CONTACT，标签指跨core边界具有内点接触，不声称u自身正长度。延伸判据闭重叠=中间三叶；严格离开=首尾两叶。离开≠中枢破坏。

**转换**：当前单元从core内到外可离开，后续返回是新相对事件；是否正式延伸成员须满足结构和计数域。

**后端输出 / 前端观察**：保留相切单独叶、离开方向、对应单元及核心版本；不由一根越界直接显示“中心死亡”。

**证明状态**：ALGEBRAIC_PARTITION_ONLY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/zhongshu.md:195-242；formal/Origin/CenterStates.lean:176-192。

### CC-019　中枢离开后首回试与破坏

**范围**：ST-013, ST-031。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：冻结核心、已完成次级离开以及其后首次回试对象身份。

**完整分支 / 生成族（数量 {"leaf_formula":"2×(2+3)=10"}）**：

- **generator**：side∈{UP,DOWN}×retest_status∈{未形成,形成未完成,完成}；完成时按cmp(UP回试low,ZG)或cmp(DOWN回试high,ZD)三分。
- **outcome**：UP的GT、DOWN的LT=三类成立并构成中枢破坏；其余含EQ不成立（Origin严格端点）。

**合法组合 / 排除**：单次离开不构成破坏；首次回试失败后不能拿更晚回试冒充first；二次回试属于另一结构上下文，需独立依据。

**转换**：leave完成→first-retest形成→完成后固定成立或失败；中枢成员入外缘规则不能因为一类点而统一终止同级所有枢。

**后端输出 / 前端观察**：同时画leave、first retest、完成、比较端点、B3/S3及中心破坏事件，含known_at。

**证明状态**：ENDPOINT_DEFINED_TEXT_SYNC_TENSION。定位项：G-010。

**证据**：.chanlun/definitions/zhongshu.md:216-252；.chanlun/definitions/maimai.md:178-211；formal/Origin/BspClassification.lean:113-127。

### CC-020　相邻正式中枢对的完整发展分类

**范围**：ST-014, ST-047。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：相应级别的两个正式中枢A,B，各DD<=ZD<ZG<=GG；当前pair的结构相邻性另作前提。

**完整分支 / 生成族（数量 5）**：

- `CORE_NOT_STRICTLY_SEPARATED`：{"id":"CORE_NOT_STRICTLY_SEPARATED","predicate":"not(B.ZG<A.ZD or B.ZD>A.ZG)"}
- `UP_NEWBIRTH`：{"id":"UP_NEWBIRTH","predicate":"B.DD>A.GG"}
- `DOWN_NEWBIRTH`：{"id":"DOWN_NEWBIRTH","predicate":"B.GG<A.DD"}
- `UP_EXPANSION`：{"id":"UP_EXPANSION","predicate":"B.ZD>A.ZG且B.DD<=A.GG"}
- `DOWN_EXPANSION`：{"id":"DOWN_EXPANSION","predicate":"B.ZG<A.ZD且B.GG>=A.DD"}

**合法组合 / 排除**：外缘严格分离蕴含core同向严格分离；core相切归首叶，外缘相切且core分离归扩展。首叶在CenterStates.development命名extension是pair几何，不自动等于CC-018某单元加入旧枢的延伸。

**转换**：新B确认后生成A→B关系；可作为高级构造输入，禁止lower输出自行强塞level_lift。各pair保留，不以优先级删除另一个pair关系。

**后端输出 / 前端观察**：显示5叶、core和outer全部比较、两个center ID及关系获知时间；分别显示新生和扩展方向。

**证明状态**：LEAN_PAIR_PARTITION_SOURCE_PRESENT_NOT_EXECUTED。定位项：G-009。

**证据**：.chanlun/definitions/zhongshu.md:243-298；formal/Origin/CenterStates.lean:215-250；formal/Origin/CenterStates.lean:326-375。

### CC-021　九段升级及构造尝试

**范围**：ST-015。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：某中枢自seed起所有参与波动的次级段集合及有序成员流；正式seed至少3段。

**完整分支 / 生成族（数量 {"threshold_branches":2,"recut_family":"有限窗口数m上的32^m条件向量，受连续性和共享成员约束，非所有向量可达"}）**：

- **count**：count<3不在正式中枢D；正式D内3..8 / >=9两支。
- **recut**：每一重切三段窗口按CC-016几何三叶和全条件签名分类；整组结果是全部窗口结果的有序向量，不只取第一个成功。

**合法组合 / 排除**：九段计全部段，不只同向Zn；>=9优先于继续把它当同级延伸。无波动不计，但精确无波动判法未实例化不能自加阈值。重切无strict核心就不产生对应枢；高级自然构造，不凭计数强制产center。

**转换**：3..8逐段增加→>=9触发升级处理；重切结果逐一记录，旧父子关系由新代际切换。

**后端输出 / 前端观察**：输出完整计数贡献、排除项理由、达到9的known_at、每个重切窗口成败与高级血缘。

**证明状态**：THRESHOLD_DEFINED_NO_FLUCTUATION_RULE_GAP。定位项：G-011。

**证据**：.chanlun/definitions/zhongshu.md:257-298；.chanlun/definitions/zhongshu.md:300-341。

### CC-022　中枢成员、连接与尚未归属单元

**范围**：ST-016。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：一条已明确构造读法的同级有序单元流。

**完整分支 / 生成族（数量 3）**：

- `CENTER_MEMBER`：{"id":"CENTER_MEMBER","predicate":"被该读法正式中枢构造纳入的成员"}
- `CONNECTOR`：{"id":"CONNECTOR","predicate":"连接相邻两个中枢而不属于二者成员"}
- `NOT_YET_ASSIGNED`：{"id":"NOT_YET_ASSIGNED","predicate":"当前完整前缀中尚未满足任何上述归属的单元"}

**合法组合 / 排除**：域是单一读法的单元归属；不同读法可有不同结果。连接段不重复计入两枢；尚未归属是构造输出覆盖状态，不能命名为候选中枢。若同单元属多个同级枢有独立正本依据则需按边关系分域，不以此表强删。

**转换**：扫完某枢后从last_member+2跳连接续扫；新事实使未归属→成员/连接，已定结构仍受版本单调边界。

**后端输出 / 前端观察**：前端可查每单元归属和续扫位置，明确未覆盖尾部，不能消失。

**证明状态**：DECOMPOSITION_TOTALITY_BRIDGE_REQUIRED。定位项：G-008。

**证据**：.chanlun/definitions/zhongshu.md:348-408；.chanlun/definitions/fenjie.md:29-45。

### CC-023　已完成走势类型三分类

**范围**：ST-017。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：相应递归级别q上已经完成、级别提升与分解已正确处理的良构走势类型T。

**完整分支 / 生成族（数量 3）**：

- `CONSOLIDATION`：{"id":"CONSOLIDATION","predicate":"恰1个相应级别中枢"}
- `UP_TREND`：{"id":"UP_TREND","predicate":">=2个相应级别中枢且相邻发展构成同向向上外缘严格分离"}
- `DOWN_TREND`：{"id":"DOWN_TREND","predicate":">=2个相应级别中枢且相邻发展构成同向向下外缘严格分离"}

**合法组合 / 排除**：零枢是未形成；未完成是输入包装；多枢非同向不能塞Other，须回到高级盘整/多走势连接/未完成的正确构造域。Lean chooseTrend给布尔输入唯一，不证明原始数据总落D；flat+2时consolidation需先升层桥。

**转换**：形成盘整可发展为趋势或高级结构；已完成旧T保持三类，后续T为新身份。禁止同级完成UP+UP或DOWN+DOWN当两个独立相邻类型。

**后端输出 / 前端观察**：输出三类、q、全中枢证据、分解身份、完成见证；无D证明时另报输入域未证而非第四类型。

**证明状态**：THREE_WAY_DOMAIN_CONDITIONAL_NOT_RAW_TOTAL。定位项：G-012。

**证据**：.chanlun/definitions/zoushi.md:90-138；.chanlun/definitions/qushi.md:40-114；docs/chanlun/text/blog/017-第17课.md:38-58；formal/Origin/TrendCompleteClassification.lean:34-88。

### CC-024　盘整语义方向与技术突破方向

**范围**：ST-018。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：CC-023三类之一和一个带适用域的技术突破信息。

**完整分支 / 生成族（数量 {"semantic":3,"technical":3,"cartesian_count":9,"reachable_count":"需结构事件域证明"}）**：

- **semantic**：UP_TREND方向UP；DOWN_TREND方向DOWN；CONSOLIDATION语义方向N/A。
- **technical**：突破方向UP/DOWN/尚无突破见证三支，由实际结构事件供给；不得由盘整标签反推出。

**合法组合 / 排除**：盘整没有多空语义方向。技术方向、trade side及BSP side是不同轴，可并存。旧兼容Compose.dir不构成教义。

**转换**：盘整内技术方向可随离开/回归更新；这不把盘整改成趋势。

**后端输出 / 前端观察**：趋势标签和break_direction分字段，盘整UI不画成必然多头/空头。

**证明状态**：SEMANTIC_DEFINED_COMPATIBILITY_SYNC_GAP。定位项：G-013。

**证据**：.chanlun/definitions/zoushi.md:138-180；.chanlun/definitions/qushi.md:179-200。

### CC-025　形成、完成、同级继任与中阴

**范围**：ST-019, ST-026, ST-044。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：相应级别一个构造中的或已形成走势实例及当前已知继任关系；原始前缀固定。

**完整分支 / 生成族（数量 4）**：

- `UNFORMED`：{"id":"UNFORMED","predicate":"尚无足够结构构成该级走势"}
- `FORMED_OPEN`：{"id":"FORMED_OPEN","predicate":"形成且尚未取得该实例完成见证"}
- `COMPLETED_NO_KNOWN_SUCCESSOR`：{"id":"COMPLETED_NO_KNOWN_SUCCESSOR","predicate":"已完成且当前未有已知同级继任实例"}
- `COMPLETED_WITH_SUCCESSOR`：{"id":"COMPLETED_WITH_SUCCESSOR","predicate":"已完成且存在不同身份的同级继任实例"}

**合法组合 / 排除**：“中阴”是形成/完成/继任当前知识关系的描述，不能另立一个已完成走势第四种类。Turn=旧T完成且新同级successor；不同identity约束不可省。Origin.Turn.Completed狭义=Div仅用于其窄TrendInstance，不能替代包括小转大的全部Move.settled。

**转换**：在无行情修订的合法前缀上UNFORMED→FORMED_OPEN→COMPLETED_*；出现继任可从无继任→有继任；不得把同一T自身当successor。即时可跨过中间观察状态但需有证据。

**后端输出 / 前端观察**：并列显示形成、完成依据、同级旧/新ID、known_at与发生位置；小转大完成不能强制要求本级Div。

**证明状态**：DEFINITIONS_PRESENT_GENERAL_COMPLETION_BRIDGE_GAP。定位项：G-014。

**证据**：.chanlun/definitions/level_recursion.md:329-356；.chanlun/definitions/beichi.md:504-584；formal/Origin/Turn.lean:40-105。

### CC-026　走势的六态观察

**范围**：ST-013, ST-018, ST-044。**轴名分**：OBSERVATION_OR_KNOWLEDGE。

**D**：TrendSixState.Context=(Option center,p,b3,s3)；center严格有效，b3/s3绑定该center和已知时间。

**完整分支 / 生成族（数量 6）**：

- `BOT`：{"id":"BOT","predicate":"center=None"}
- `INSIDE_Z`：{"id":"INSIDE_Z","predicate":"center=Some Z且ZD<=p<=ZG"}
- `ABOVE_NO3B`：{"id":"ABOVE_NO3B","predicate":"center=Some Z且p>ZG且not b3"}
- `ABOVE_B3`：{"id":"ABOVE_B3","predicate":"center=Some Z且p>ZG且b3"}
- `BELOW_NO3S`：{"id":"BELOW_NO3S","predicate":"center=Some Z且p<ZD且not s3"}
- `BELOW_S3`：{"id":"BELOW_S3","predicate":"center=Some Z且p<ZD且s3"}

**合法组合 / 排除**：六态是给定center/price/BSP事实的观察轴，非CC-023走势三类，也不是交易决策。无center时p/bsp不影响BOT；core内时b3/s3仍可作为独立历史事实保留，不被六态标签删除。未绑定center的任意bool不能充当前提。

**转换**：价格和已知BSP变化可引发六态迁移；全36有序状态对仅是表现变化空间，真实可达须由原始前缀与center身份规则证明，不能生成独立FSM控制交易。

**后端输出 / 前端观察**：同源返回6态及来源context；UI可展示六态、完整BSP事实和历史变化，无价格门反向准入。

**证明状态**：LEAN_TOTAL_UNIQUE_GIVEN_CONTEXT_NOT_PROJECTION。定位项：G-015。

**证据**：formal/Origin/TrendSixState.lean:1-185。

### CC-027　递归构造级别与停止

**范围**：ST-020, ST-021。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：从同一a0输入构造的一棵有限塔；q是递归阶数。

**完整分支 / 生成族（数量 {"level":2,"next_layer":2,"resource_bit":2}）**：

- **level**：q=0（Move0=线段） / q>0（Centerq由Move(q-1)构，Moveq由center构）两支。
- **stop**：下一层已settled单元数<3自然停止 / >=3可继续候选扫描；另资源预算是否耗尽独立bool。

**合法组合 / 排除**：递归级别、图表周期、经营重深度三个身份不互换。>=3仅足以尝试，不保证有strict center。max_levels是资源限制，不定义市场最高级别；局部确认无须先递归到底。

**转换**：下层settled对象供上层；数量/候选不足自然停止，后续新事实可恢复；资源中断须显式可续算。

**后端输出 / 前端观察**：每级给构造参数/基底、父子来源、已结算供给量、自然/资源停止理由。

**证明状态**：RECURSION_SCHEMA_DEFINED_DOMAIN_BRIDGE_GAP。定位项：G-009。

**证据**：.chanlun/definitions/level_recursion.md:16-69；.chanlun/definitions/level_recursion.md:117-178；.chanlun/definitions/level_recursion.md:223-297。

### CC-028　跨级同判据与真实父子

**范围**：ST-021, ST-034。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：要比较的同名判据两次调用及声明的父子边。

**完整分支 / 生成族（数量 {"same_input_comparison":2,"parent_relation":2}）**：

- **equiv**：相同规范输入+不同level参数时输出相同/不同两支；不同即违规线索，合法level差异只在输入值。
- **parent**：真实构造成员边 / 只有时间或价格几何包含但不是成员边两支。

**合法组合 / 排除**：对象不同的判据不强行同形；同输入同输出要测试/证明，不是grep归零。真实父子level差1且成员归属和端点覆盖；几何contains不能推出parent。

**转换**：追加产生新成员边；既有边的修订以结构代际表达；旁路不能改构造血缘。

**后端输出 / 前端观察**：给每级同输入对拍证据、parent_id和member证明；绘图连边区分构造和几何。

**证明状态**：INVARIANT_DEFINED_PROOF_REQUIRED。定位项：G-009。

**证据**：.chanlun/definitions/结构判据.md:9-39；.chanlun/definitions/level_recursion.md:117-178；.chanlun/definitions/qujiantao.md:245-309。

### CC-029　描述中枢形态与正式对象名分

**范围**：ST-022。**轴名分**：ROLE_OR_RELATION_SCHEMA。

**D**：正式center或另有来源的PH/类中枢对象。

**完整分支 / 生成族（数量 3）**：

- `FORMAL_CENTER`：{"id":"FORMAL_CENTER","predicate":"满足CC-016的缠论中枢"}
- `BASE_ANALOG`：{"id":"BASE_ANALOG","predicate":"线段以下独立声明的类中枢，不自动等于笔中枢"}
- `PH_ANALOG`：{"id":"PH_ANALOG","predicate":"PH拓扑对象独立对象类型"}

**合法组合 / 排除**：三角/奔走/平台是正式center的描述，不是互斥穷尽形态；“开放型中枢”未采纳。类比不获得正式center生产资格。 zhongshu:450-462明文排除中枢形态分类；不能把无形态阈值登记成用户本轮必须裁的新边界。

**转换**：名分不能通过渲染样式或相似几何升级；正式center必须有独立seed证据。

**后端输出 / 前端观察**：UI明确对象类型；形态只能附原文描述与证据，不能新增未定义生产分类。

**证明状态**：NOMINAL_DISTINCTION_DEFINED_SHAPE_OBSERVATION_ONLY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/zhongshu.md:344-363；.chanlun/definitions/zhongshu.md:450-474。

### CC-030　背驰比较对象与适用域

**范围**：ST-023。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：相应级别的同向结构片段，原始事实和结构上下文已具备。

**完整分支 / 生成族（数量 4）**：

- `TREND_PAIR`：{"id":"TREND_PAIR","predicate":"处于趋势；比较进入最后中枢B的b与其后满足结构条件的c"}
- `RANGE_FIRST_EXIT_PAIR`：{"id":"RANGE_FIRST_EXIT_PAIR","predicate":"盘整首次离开；比较同向进入段与该离开段"}
- `RANGE_REPEAT_EXIT_PAIR`：{"id":"RANGE_REPEAT_EXIT_PAIR","predicate":"盘整重复离开；比较最近上次同向跨边界离开段与本次"}
- `CENTER_INTERNAL`：{"id":"CENTER_INTERNAL","predicate":"对象仅为中枢内部波动，未形成上述跨界比较对；结构背驰判据不适用"}

**合法组合 / 排除**：趋势b是进入最后B的段，不是习惯写法a；盘背pair“最近同向跨界”与区间套候选集合不许按最近任取一个是不同对象。不能用全部中心内部任两段比较产正式背驰。

**转换**：只有结构pair成立才计算独立力度关系；无pair输出适用域不足，不能弱配对fallback。

**后端输出 / 前端观察**：标明TREND/RANGE和first/repeat、b/c对象及选择来源，显示被排除内部段。

**证明状态**：PAIRING_DEFINED_LIVE_PROJECTION_UNVERIFIED。定位项：G-016。

**证据**：.chanlun/definitions/qushi.md:216-237；.chanlun/definitions/beichi.md:261-284。

### CC-031　趋势背驰五结构条件签名

**范围**：ST-023, ST-029。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：一个声称为a+A+b+B+c的趋势比较上下文，值和级别完整。

**完整分支 / 生成族（数量 {"masks":32,"extreme_cmp":3}）**：

- **generator**：(确为趋势,A/B同级,c为次级且含对B三类,b.level<=c.level,c创新极值)∈{0,1}⁵；32签名，11111结构成立。
- **refinement**：c相对前极值LT/EQ/GT三分；UP要求GT，DOWN要求LT，EQ失败。

**合法组合 / 排除**：五项先于力度；Extreme在对应次级走势内部判，不能因降级通道保留其余条件删Extreme；下钻失败不能绕成小转大。

**转换**：结构条件随发展态可能逐项取得；只有当前齐备时才有当下结构背驰判断，未来成立不得回灌到过去。

**后端输出 / 前端观察**：输出5位及每位来源对象/known_at，所有失败原因和不适用项分开。

**证明状态**：STRUCTURAL_PREDICATE_DEFINED_BRIDGE_REQUIRED。定位项：G-016。

**证据**：.chanlun/definitions/beichi.md:504-534；.chanlun/definitions/beichi.md:286-303。

### CC-032　力度样本与数值全分类

**范围**：ST-024。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：接口域I：含全部价格/索引字段及单位、坐标、代际标识的有限笔记录列表S，允许空表、零/负时序差以及不一致记录；I内校验字段已可判。语义有效子域D_L：S非空、每笔startIndex<endIndex且全表时间次序、单位/坐标/代际一致。缺字段或尚不能核实一致性的请求不进已齐接口域I，按CC-054显式返回不足/域证明缺失。

**完整分支 / 生成族（数量 {"interface_I":4,"valid_length_subdomain_D_L":2,"value_on_D_L":3,"length_x_value_allowed_signatures":4,"comparison_on_D_pair":3,"invalid_reason_nonzero_masks":15}）**：

- **interface_partition**：[{"id":"EMPTY","predicate":"|S|=0","destination":"CC-054输入不足；无语义L，Lean impulse([])=0仅总化读数"},{"id":"NONEMPTY_INVALID","predicate":"|S|>0且WellFormed/一致有序校验至少一项失败","destination":"CC-054输入不一致/域外，返回全部失败位；不发布语义速度或L"},{"id":"SINGLE_VALID","predicate":"|S|=1且S∈D_L","destination":"有语义L=0"},{"id":"MULTI_VALID","predicate":"|S|>=2且S∈D_L","destination":"有语义L=v_last-v_first"}]
- **formula_on_D_L**：v_i=(p_end-p_start)/(index_end-index_start)，L=v_last-v_first；单筆首末相同，故L=0。
- **value_partition_on_D_L**：L<0 / L=0 / L>0；SINGLE_VALID只落L=0，MULTI_VALID的三数值支按实际样本判。不能把接口EMPTY总化0混入L=0语义叶。
- **comparison_on_D_pair**：L(c)<L(b) / = / > 三叶，只有c,b均满足D_L与同单位坐标前提时求值。
- **known_invalid_masks**：NONEMPTY_INVALID保留(存在非正时序笔,顺序不一致,单位/坐标不一致,代际不一致)四位全部失败原因；至少一位真。该原因向量不是市场结构新类型。

**合法组合 / 排除**：EMPTY与NONEMPTY_INVALID都不属于D_L，不能获得语义L标签。单笔有效L恰0；至少两笔有效才可进一步落三个符号叶。零时长除零总化0、负分母结果均只属总函数值，不能越过WellFormed语义前提。带符号，不取绝对值、不绕父除比、不用TV/振幅代理兜底；时间坐标共享不等于必须下钻到底。

**转换**：接口EMPTY可在追加有效样本后进入SINGLE_VALID/MULTI_VALID；无效输入校正须相应代际，记录旧失败。D_L内发展样本改变首/末边界可更新L；旧版本样本和known_at不回写。

**后端输出 / 前端观察**：输出interface_case和domain_status；只有D_L内输出semantic_L及首/末笔、四端点、Δindex、v首/末。空表和无效记录返回明确原因、semantic_L缺席；可附totalized_debug_value但不得绘成零力度。比较端仅接受两个语义有效L及同单位坐标。

**证明状态**：EXPLICIT_LEAN_DEFINITION_SAMPLING_BRIDGE_REQUIRED。定位项：G-017。

**证据**：formal/Origin/ForceVelocity.lean:1-68；.chanlun/definitions/beichi.md:336-373；.chanlun/definitions/beichi.md:463-469。

### CC-033　力度比较、结构背驰与代理状态

**范围**：ST-023, ST-024, ST-025。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：结构比较对已定义且两个语义力度样本有效。

**完整分支 / 生成族（数量 3）**：

- `WEAKER`：{"id":"WEAKER","predicate":"L(c)<L(b)"}
- `EQUAL`：{"id":"EQUAL","predicate":"L(c)=L(b)"}
- `STRONGER`：{"id":"STRONGER","predicate":"L(c)>L(b)"}

**合法组合 / 排除**：Div=结构前提∧WEAKER；结构失败和力度不弱独立位，不能只给Div=false吞原因。MACD辅助另轴，不可替换精确定义失败。

**转换**：L变化重新判3分；不能把候选内Cand的3条件和Weak合并成同一层门。

**后端输出 / 前端观察**：展示结构是否成立、L三分、Div结果和辅助读数分别字段。

**证明状态**：ALGEBRAIC_PARTITION_ONLY。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/beichi.md:504-584。

### CC-034　MACD辅助完整逻辑签名

**范围**：ST-025。**轴名分**：OBSERVATION_OR_KNOWLEDGE。

**D**：相应MACD序列完整、同采样口径且结构比较pair已给；本轴是辅助。

**完整分支 / 生成族（数量 {"trend":16,"range":8}）**：

- **generator**：(趋势适用的T4零轴前提,T6 DIF未创新极<=,T2同色带号面积严格<,T7 HIST未超<=)∈{0,1}⁴；16签名。
- **projection**：趋势辅助=T4∧(T6∨T2∨T7)；盘背辅助=(T6∨T2∨T7)，T4在盘整域不适用。

**合法组合 / 排除**：等号按各项不同规则；不能全部统一strict。面积同色带号和非Σ|hist|；T4影响趋势三项整体，不能仅限制面积。缺数/换周期/代理不可比均在数据状态轴，不转窄宽fallback。

**转换**：每次同一数据前缀更新三维和前提；每个proxy取值的采样窗口可追溯。

**后端输出 / 前端观察**：同时显示三维全部位与数值、T4、缺数理由；辅助结果不能冒充结构背驰结论。

**证明状态**：LOGIC_DEFINED_NO_RUNTIME_OR_PROXY_EQUIVALENCE_PROOF。定位项：G-018。

**证据**：.chanlun/definitions/beichi.md:792-830；.chanlun/definitions/beichi.md:374-396；.chanlun/definitions/beichi.md:1005-1019。

### CC-035　Div、NE、Turn及高级结果的分轴

**范围**：ST-026, ST-047。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：同一q、旧趋势T与当时事实前缀；NE涉及终结意义，Turn涉及同级继任。

**完整分支 / 生成族（数量 {"raw_signatures":16,"semantic_reachable_count":"未取得统一Completed/NE桥证明，不能宣布16合法或按旧窄Completed删小转大"}）**：

- **signature**：(Div_q(T,t),Completed(T),known_same_q_distinct_successor,NE(T))∈{0,1}⁴为16个命题签名；若统一语义实现满足Div⇒Completed及Completed⇒NE，则排除违背这两条的签名；这些桥当前不是给定bool枚举可自动证明。
- **turn**：Turn=Completed∧known_same_q_distinct_successor，独立于未来高级Outcome³。

**合法组合 / 排除**：Div为当下本级结构+力度；NE是不能继续同向该级中心的命题；Turn要求真正新同级继任。未来结果不能作为当时Div判据，EM桥不能循环假设“已转折所以背驰”。

**转换**：Div/完成/继任可同一known_at或分次获知；高级结果稍后归类，保留原时点未知事实的知识状态。

**后端输出 / 前端观察**：前端逐命题显示已知证据和关系，不把“有背驰”直接画作高级反趋势已完成。

**证明状态**：SEMANTIC_BRIDGE_THEOREMS_MISSING。定位项：G-014, G-019。

**证据**：.chanlun/definitions/beichi.md:504-584；formal/Origin/Turn.lean:40-105。

### CC-036　所有已发生转折的级别二分

**范围**：ST-027。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：已确认q级旧走势发生转折，已定位导致其转折的背驰级别r；r<=q。

**完整分支 / 生成族（数量 2）**：

- `SAME_LEVEL_DIVERGENCE`：{"id":"SAME_LEVEL_DIVERGENCE","predicate":"r=q"}
- `SMALL_TO_LARGE`：{"id":"SMALL_TO_LARGE","predicate":"r<q"}

**合法组合 / 排除**：r>q违反D；下钻无anchor不是r<q的证据。线段层“类小转大”须按特征序列对象处理，不能借上级走势的命名代替终结。它是已发生事件归因二分，不是未来预测器。

**转换**：只有q级转折证据和r级定位均有才确定分支；此前保留必要条件/局部发展，不预告SMALL_TO_LARGE。

**后端输出 / 前端观察**：显示旧q走势、新q继任、r级背驰位置、r/q、发生与获知时点；必要条件另标。

**证明状态**：ORDER_PARTITION_DEFINED_WITNESS_BRIDGE_REQUIRED。定位项：G-020。

**证据**：docs/chanlun/text/blog/043-第43课.md:18-26；.chanlun/definitions/beichi.md:127-145；.chanlun/definitions/beichi.md:164-199。

### CC-037　小转大必要条件和发生的合法组合

**范围**：ST-027。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：D是一个明确q级旧趋势及其小级别r<q背驰上下文，末次级中枢Z_last属于该q级旧趋势。X只表示该r<q背驰引起的q级小转大已经发生；同级r=q背驰转折由CC-036另一支处理，不属于X。N是原UP趋势Z_last出现第三类卖点，或原DOWN趋势Z_last出现第三类买点；与旧趋势反向，不能写成笼统同向三类。各事实有当时可知见证。

**完整分支 / 生成族（数量 {"valid":3,"rejected_signature":1}）**：

- `NO_N_NO_X`：{"id":"NO_N_NO_X","predicate":"not N and not X"}
- `N_WITHOUT_X`：{"id":"N_WITHOUT_X","predicate":"N and not X"}
- `N_AND_X`：{"id":"N_AND_X","predicate":"N and X"}
- `INVALID_OR_UNPROVEN_WITNESS`：{"id":"INVALID_OR_UNPROVEN_WITNESS","predicate":"X and not N","classification":"在本r<q小转大域中，X=true且N=false是必要条件见证不一致；先检查数据/证据完整性，不是第四合法市场态；不得据此否定r=q同级背驰转折。"}

**合法组合 / 排除**：本域X⇒N，N不⇒X；N可成立而原q趋势继续。原UP必要N是最后q-1中枢三卖、原DOWN必要N是三买。r=q的同级背驰转折不接受本轴X命名，也不由本轴强加N。没有保证未来的充分条件是原文明说的边界，不是分类缺口。

**转换**：在固定r<q上下文内，N可先成立而X仍false；只有小转大实际发生且见证足够时才进入N_AND_X。r=q转折通过CC-036.SAME_LEVEL_DIVERGENCE记录，不通过本轴制造X&&!N违规。

**后端输出 / 前端观察**：显示“必要条件已出现”与“小转大已发生”两个事实、各自known_at及见证。

**证明状态**：SOURCE_NECESSITY_DEFINED_NO_FUTURE_SUFFICIENCY_CLAIM。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：docs/chanlun/text/blog/044-第44课.md:24-30；.chanlun/definitions/beichi.md:127-145。

### CC-038　小背驰后的局部中枢出口和相邻关系

**范围**：ST-027, ST-047。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：r<q的小背驰后已形成级别>r的局部中枢C，最后q级中枢B已给。

**完整分支 / 生成族（数量 {"exit":3,"relation":2,"combinations":6}）**：

- **exit**：C当前尚无确认出口 / 向上确认出口 / 向下确认出口三支，出口根据结构事件不是单价越界。
- **relation**：C与B按CC-002端点族判闭重叠/无重叠；需同口径range才可比。
- **interpretation**：以原UP趋势为例：局部向下继续才有回B可能；向上可延续；C和B不重叠则原q走势可继续，重叠可能构造更高级中心。DOWN镜像。

**合法组合 / 排除**：局部出口、B/C重叠、q级转折是三个不同轴；不能只看“小背驰”二字把所有分支压成X。重叠只按明定范围，级别重建仍须CC-016，不任意升层。

**转换**：小背驰→形成C→出口和关系逐项获知；任一部分尚缺时记录知识而不填另一出口。

**后端输出 / 前端观察**：画小背驰、C、B及完整关系，对继续原趋势和更高级构造保留历史因果链。

**证明状态**：LOCAL_SOURCE_CASES_DEFINED_GLOBAL_REACHABILITY_UNPROVEN。定位项：G-020。

**证据**：docs/chanlun/text/blog/043-第43课.md:30-48。

### CC-039　小转大答疑与操作落点的观察轴

**范围**：ST-027, ST-030, ST-043。**轴名分**：OBSERVATION_OR_KNOWLEDGE。

**D**：已声明操作q、旧方向与参考高/低、最后次级中心；只提取结构事实，经营决策交FG。

**完整分支 / 生成族（数量 {"extreme_cmp":3,"center_touch":2,"div_bit":2,"raw_signature":12}）**：

- **relations**：对参考极值按LT/EQ/GT，是否触及最后次级center按CC-002闭关系，再与当前反弹/回调结构背驰位形成乘积。
- **source_cases**：强走势可不触最后次级中心；普通在参考极值内反弹/回调；突破参考后须看后续反弹/回调背驰或不再创极值；原文没有授权跌破即卖或无背驰仍强制反转。

**合法组合 / 排除**：这些价格关系只有作为原文给定结构载体；不得据“最强/普通/最弱”自行设全互斥数值档位。等参考极值单列，具体操作边界若无Origin定义不能静默取一侧。

**转换**：先后次序保留：突破、反弹、次级转折各不同事件，不能倒序解释旧信号。

**后端输出 / 前端观察**：输出参考对象、三分关系、center触及与背驰证据，经营接口接结构事实而非本目录决定仓位。

**证明状态**：OBSERVABLE_ATOMS_COMPLETE_OPERATION_BOUNDARY_UNSETTLED。定位项：G-021。

**证据**：docs/chanlun/text/blog/043-第43课.md:186-220；docs/chanlun/text/blog/044-第44课.md:34-38；docs/chanlun/text/blog/053-第53课.md:28-34。

### CC-040　背驰转折后的三个高级结果

**范围**：ST-047。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：已发生q级趋势背驰及其后足以完成分解的走势结果；结果按29课的分解层级识别。

**完整分支 / 生成族（数量 3）**：

- `LAST_CENTER_EXPANDS`：{"id":"LAST_CENTER_EXPANDS","predicate":"最后q级中枢扩展，原下跌等属于尚未完成的更高级走势内部"}
- `LARGER_CONSOLIDATION_CONNECTION`：{"id":"LARGER_CONSOLIDATION_CONNECTION","predicate":"已完成旧q走势+一个严格高于q的盘整走势相接"}
- `REVERSE_TREND_CONNECTION`：{"id":"REVERSE_TREND_CONNECTION","predicate":"已完成旧q走势+一个级别>=q的反向趋势走势相接"}

**合法组合 / 排除**：三分取决于“同一高级走势内部”还是“两个已完成走势相接”，不能只按反弹是否入核心。反趋势允许等于q；“必须>q”会漏同级反趋势。最弱可只触外缘DD/GG，不能把29课保证错写为必回ZD/ZG核心。

**转换**：结果尚未形成/完成时只标待观察，不能提前归三叶；完成证据出现后归类，并可解释此前局部状态。

**后端输出 / 前端观察**：显示Outcome³、分解边界、比较级别与完成见证、回拉关系另轴。

**证明状态**：SOURCE_TRICHOTOMY_STATED_REALIZATION_PROOF_MISSING。定位项：G-022。

**证据**：docs/chanlun/text/blog/029-第29课.md:16-52；.chanlun/definitions/beichi.md:649-658。

### CC-041　买卖点六谓词的完整签名

**范围**：ST-028。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：编码域U=Bool^6允许所有64位向量。语义视图域D_LevelView是Origin给定LevelView：cand1/cand2/cand3每类恰一个Option BspEndpoint，各候选有唯一side；不在此构造市场投影。σ(D_LevelView)必须满足每类Bk与Sk不能同真，属于U的至多27签名子集。原始行情/生产逐点BspBits列表不自动等于此LevelView域。

**完整分支 / 生成族（数量 {"encoding_U":64,"excluded_by_proven_same_class_buy_sell_exclusion":37,"abstract_allowed_upper_bound":27,"all_27_constructible":"NOT_PROVED_BY_THIS_CATALOG","market_reachable_count":"NOT_PROVED"}）**：

- **encoding_family_U**：向量(B1,B2,B3,S1,S2,S3)∈{0,1}^6，用sum(bit_i*2^i)生成0..63全部64编码；每个编码唯一并穷尽U。
- **known_semantic_exclusion**：∀k∈{1,2,3}, ¬(Bk∧Sk)。依据BuySellPredicate:82-97每类仅一个候选、:207-208同类买卖互斥定理；违反任一位对的37编码不能来自该LevelView。
- **allowed_signature_generator**：按类k独立取{NONE,BUY,SELL}三种抽象位对，共3^3=27个不违反上述已证排斥的编码；不同类别可以买/卖并存。只是已知约束允许集，不声称每一个都有满足厚谓词的LevelView见证或市场构造。
- **allowed_signature_codes**：[0,1,2,3,4,5,6,7,8,10,12,14,16,17,20,21,24,28,32,33,34,35,40,42,48,49,56]
- **semantic_local_endpoint_constraint**：同端同级同side的标签集合另受CC-042五集合约束；不同类别可投影不同端点，不能拿局部排斥替代或绕过同类买卖排斥。
- **production_carrier_boundary**：当前Rust是真实逐条BspPoint.bits；append+sort未构造每级每时刻唯一LevelView/SignalVector。Γ逐候选bsp_bits_class_index保留六位，不能当作D_LevelView生产投影已贯通。

**合法组合 / 排除**：同一LevelView每类cand_k仅一Option端点且side唯一，故Bk与Sk不能同真。不同类别的买/卖可分别来自不同端点，但仍须各厚谓词成立；同一端点还受CC-042约束。编码全集64和约束允许集27分开，不以数组顺序选择标签。不同实现载体不能无证明地共享27上界。

**转换**：每个新前缀若构成新的D_LevelView，输出必须仍满足三条同类排斥；保留原候选ID与投影版本。Rust逐点列表的增删/排序不是这一步投影的证明，需单独桥接；单一签名不作对象ID。

**后端输出 / 前端观察**：若输出名为LevelView签名，必须同时给cand1/2/3身份、side、厚谓词见证及三条互斥检查；逐点BspBits和Γ候选六位另标carrier_kind，不冒充每级每时刻投影。UI允许跨不同类别反向标签，拒绝同一类B/S同真却标成有效LevelView。

**证明状态**：64_ENCODING_TOTAL_KNOWN_SAME_CLASS_EXCLUSION_27_UPPER_BOUND_PROJECTION_UNPROVED。定位项：G-023。

**证据**：formal/Origin/BuySellPredicate.lean:385-429；.chanlun/definitions/maimai.md:106-147；formal/Origin/BuySellPredicate.lean:82-97；formal/Origin/BuySellPredicate.lean:169-208；formal/Origin/BuySellPredicate.lean:231-315。

### CC-042　同端同级同side买卖标签合法集合

**范围**：ST-028, ST-030。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：同一个语义端点、级别、side及结构上下文；而非整个LevelView。

**完整分支 / 生成族（数量 5）**：

- `NONE`：{"id":"NONE","set":[]}
- `TYPE1`：{"id":"TYPE1","set":[1]}
- `TYPE2`：{"id":"TYPE2","set":[2]}
- `TYPE3`：{"id":"TYPE3","set":[3]}
- `TYPE2_TYPE3`：{"id":"TYPE2_TYPE3","set":[2,3]}

**合法组合 / 排除**：排除{1,2},{1,3},{1,2,3}；2/3重合合法且最强描述独立。不同context、级别、side的同像素点不可按坐标去重。NONE是六谓词全假的精确定义，不是未知兜底。 若这些端点被汇成Origin LevelView，还须逐类满足CC-041的¬(Bk∧Sk)；局部同side五集合不能取消这条不同层级的限制。

**转换**：同一个端点新增证据可使NONE→某标签或TYPE2/TYPE3→TYPE2_TYPE3；确认后标签撤回只能由版本修订/证明推翻事件表达，不靠first-match。

**后端输出 / 前端观察**：重合同时画2/3、分别给理由；最多5集合的约束须由实际投影不变量证明。

**证明状态**：DOCTRINE_LEGAL_SETS_DEFINED_PROJECTION_PROOF_REQUIRED。定位项：G-023。

**证据**：.chanlun/definitions/maimai.md:106-147；.chanlun/definitions/maimai.md:277-309。

### CC-043　第一类点完整条件族

**范围**：ST-029。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：带side、核心及比较pair的同级端点。

**完整分支 / 生成族（数量 {"per_side":8,"with_side":16}）**：

- **generator**：(broken_core,trend_pair,Div)∈{0,1}³，8签名；111为Type1，方向long=buy、short=sell。

**合法组合 / 排除**：正式Type1要求趋势；盘背用于Type2/3下级完成确认或超大级类一描述不等于正式Type1。broken_core不是一根价格离开，即使独立参数是Bool也须有结构投影证据。

**转换**：只在全部结构条件当前成立时发布Type1；不把Turn逆推Div当证据。

**后端输出 / 前端观察**：显示三条件和同级core、趋势pair与力度、side和known_at。

**证明状态**：EXPLICIT_PREDICATE_INPUT_PROOF_REQUIRED。定位项：G-023。

**证据**：.chanlun/definitions/maimai.md:147-178；formal/Origin/BspClassification.lean:88-100。

### CC-044　第二类构成结构的次级序列与首个后继

**范围**：ST-030。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：本级parent的q级第二类结构候选；subs=descend(parent)是其q-1级成员序列。i1/i2只属于subs。q级转折锚有无同级Type1由CC-059独立展示。

**完整分支 / 生成族（数量 {"temporal":3,"extreme_annotation":3,"multiple_witnesses":"所有满足的i1组成有限见证集合；存在性判据不得被误称唯一见证。"}）**：

- **index_domain**：i1和i2是subs内局部序号，不是q级Type1事件表索引、raw索引或merged索引；m1=subs[i1]为次级一类离开，m2=subs[i2]为后继回拉。
- **temporal**：无满足SubLevelType1的m1 / 有m1但无i1+1成员 / 有m1且m2=subs[i1+1]三支；每个m1均枚举，不以任取首个隐藏其他结构候选。
- **constitutive**：完整结构=(descend parent)[i1]=m1 ∧ (descend parent)[i1+1]=m2 ∧ SubLevelType1(side,m1,c1,divPair)。m2反向语义、结束/确认以及真实q-1归属须提取桥落实。
- **price_annotation**：比较m2.extreme与m1.extreme为LT/EQ/GT三叶；全部只标注，不新增不破极值门。

**合法组合 / 排除**：i2=i1+1约束次级构成见证，不得误写为本级Type1事件的后继编号。SubOf须真实次级归属、反自反和同side；RMoveCompose已给实际subs索引定义，BspClassification的构成/可观测双向桥仍有限定。q级锚无一类不等于缺q-1级构成一类。 CC-059补q级转折锚的有/无同级一类轴，不把这里的次级m1一类和q级锚一类混同。

**转换**：取得m1次级一类→同subs首个后继m2→回拉完成见证→二类确认；不可把后续任意m2替掉不存在的i1+1。q级锚有无一类不改变这个索引域，但两者的具体对象对应须单独证。

**后端输出 / 前端观察**：列q级parent/转折锚、q-1的subs/m1/m2、i1/i2、c1/divPair、回拉方向与完成，显示实际索引域和全部见证。

**证明状态**：CONSTITUTIVE_INDEX_DOMAIN_DEFINED_EXTRACTION_BRIDGE_REQUIRED。定位项：G-024, G-032。

**证据**：.chanlun/definitions/maimai.md:209-227；formal/Origin/RMoveCompose.lean:199-249；formal/Origin/RMoveCompose.lean:517-555；formal/Origin/BspClassification.lean:131-179。

### CC-045　第三类点的严格端点与首回试

**范围**：ST-031。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：接口域I：针对冻结有效核心的一次离开/回试请求，可包含尚缺对象、价格、标识或完成证据。编码域E⊂I：所需对象和值已给且同源，side、left、first、leave_settled、retest_settled和极值比较均可求值。语义完成子域D_completed⊂E要求两次级走势均完成；正式首回试子域D_first⊂D_completed再要求left=first=true。不得把E的96编码都叫作完成语义域。

**完整分支 / 生成族（数量 {"raw_encoding_E":96,"unfinished_E_minus_D_completed":72,"completed_D_completed":24,"completed_non_first_or_not_left":18,"eligible_D_first":6,"confirmed_type3_signatures":2,"I_minus_E":"按CC-054缺失/一致性原因集，不计入96完整字段编码"}）**：

- **interface_readiness**：I\E包括缺leave、缺retest、缺价格/core、缺完成证据、同源/身份未核；保留全部缺项原因，交CC-054而非给缺值填false。
- **encoding_family_E**：side∈{BUY,SELL}×(left,first,leave_settled,retest_settled)∈{0,1}^4×cmp(retest_extreme,boundary)∈{LT,EQ,GT}；96原始完整字段编码；不承诺全部可达。
- **unfinished_E_minus_D_completed**：至少一个settled=false的72编码只表达发展/未完成输入；具体候选形成及确认按CC-019/CC-046，不得凭临时极值把正式三类当已确认，也不能把未完成直接当永久否定。
- **completed_domain_D_completed**：两settled均true，side×(left,first)×cmp共24编码；若left或first=false则不符合这次正式三类的结构要求，共18签名。
- **eligible_first_retest_D_first**：left=first=leave_settled=retest_settled=true，BUY/Sell各LT/EQ/GT共6叶；BUY.GT、SELL.LT成立，另4叶不成立且EQ明确拒绝。
- **formal_vs_wrapper**：Origin IsType3Buy/Sell的left∧first∧strict-price已定义；完成要求由此处结构投影/确认包装明确携带，不谎称Origin厚谓词本身读取settled位。

**合法组合 / 排除**：96只对E，24只对D_completed，6只对D_first，2只指正式三类的两个side比较签名；数字不可互换。I\E不丢弃，E中未完成72编码不发布正式三类确认。D_completed中left/first失败可明确否定本次请求；不设距离门，不以更远回试替首次失败，core保持冻结。Origin严格端点已有定义，旧inclusive张力按G-010处理。

**转换**：请求先补齐对象和值进入E；leave/retest各自完成使E进入D_completed；此时结构first/left和严格边界决定成立/失败。未完成时仅更新候选/发展事实；完成不得仅由当前价格位置代替，后续单元不更改first身份，修订另代际。

**后端输出 / 前端观察**：输出I/E/D_completed/D_first成员资格、全部完成/first/left证据和比较叶；数据不齐给精确缺项，未完成显示等待或发展，不显示已确认三类；最终确认以冻结core及完成次级对象绑定。

**证明状态**：EXPLICIT_STRICT_ENDPOINT_TEXT_IMPLEMENTATION_SYNC_TENSION。定位项：G-010。

**证据**：.chanlun/definitions/maimai.md:178-237；.chanlun/definitions/maimai.md:333-367；formal/Origin/BspClassification.lean:113-127。

### CC-046　买卖点确认与结构归因

**范围**：ST-032。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：一个已满足类型谓词的买卖点对象及对应走势组件。

**完整分支 / 生成族（数量 2）**：

- `FORMED_UNCONFIRMED`：{"id":"FORMED_UNCONFIRMED","predicate":"类型成立但对应Move未settled"}
- `CONFIRMED`：{"id":"CONFIRMED","predicate":"类型成立且对应Move已settled"}

**合法组合 / 排除**：confirmed必须绑定具体Move完成语义，不能因扫描遇到就盖章。行情更正与语义确认轴分开；完全分类声明不允许把数据失真当第三种BSP。

**转换**：合法追加下未确认→确认，确认不可退；证据失效通过撤回和新版本，不静默改原known_at。

**后端输出 / 前端观察**：确认旗标、完成见证、形成/确认各known_at与撤回因果可查询。

**证明状态**：CONFIRMATION_DEFINED_MONOTONICITY_PROOF_REQUIRED。定位项：G-004。

**证据**：.chanlun/definitions/maimai.md:367-408；.chanlun/definitions/level_recursion.md:329-340。

### CC-047　区间套坐标与几何包含

**范围**：ST-033, ST-034。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：同一行情代际的上下级已声明结构区间，index为统一raw锚闭区间。

**完整分支 / 生成族（数量 {"index":26,"price":26,"level":3,"raw_product":2028}）**：

- **index**：按CC-002的26端点叶细分；index_subset=(parent.i0<=child.i0且child.i1<=parent.i1)。
- **price**：按CC-002另外26叶，price_subset独立；真实descendant min/max聚合可推出price_subset，独立选父不能假设。
- **level**：child.level<parent.level / = / >三叶；真一层成员须差1。

**合法组合 / 排除**：闭subset允许共享端点，不要求strict缩短每个数值端点；结构级别必须下降。时间包含、价格包含、真实parent三事实不能互代。独立跨级NestInterval价格挂法未决定，不能用真实descendant证明套过来。

**转换**：向下定位逐级生成区间链；索引映射变更须同源版本重算，不把显示merged索引当raw比较。

**后端输出 / 前端观察**：显示区间端点、subset证据、真实parent或仅几何关系、完整祖先路径和坐标版本。

**证明状态**：GEOMETRY_PARTITION_COMPLETE_SELECTED_PARENT_PRICE_POLICY_GAP。定位项：G-025。

**证据**：.chanlun/definitions/qujiantao.md:147-214；.chanlun/definitions/qujiantao.md:245-309；.chanlun/definitions/qujiantao.md:1313-1318。

### CC-048　Cand元素三条件全签名

**范围**：ST-035。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：给定level/方向δ、上层约束区间和全部可比子候选；元素级。

**完整分支 / 生成族（数量 8）**：

- **generator**：(Dir,Comparable,Extreme)∈{0,1}³，8签名；111入元素候选集合。力度Weak为独立轴，不参与该Cand定义。

**合法组合 / 排除**：Extreme仍保留；把Weak并入rung Cand违规。Comparable的结构对象要与正本相符，不能只因价格同向就算。先按上层区间条件化集合，再选，不能全局取一个后裁剪。

**转换**：构造可以bottom-up形成条件候选；top-down求值传入所选上级区间过滤；两个方向是不同环节可并存。

**后端输出 / 前端观察**：返回8位分组的候选全集、被排除原因、上级条件和力度另轴。

**证明状态**：ELEMENT_DEFINITION_PRESENT_INSTANCE_BRIDGE_REQUIRED。定位项：G-026。

**证据**：.chanlun/definitions/qujiantao.md:729-775。

### CC-049　Cand集合非空和Sel选择

**范围**：ST-035。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：C=已按上层所选区间条件化的全部元素级111候选集合。

**完整分支 / 生成族（数量 3）**：

- `EMPTY`：{"id":"EMPTY","predicate":"|C|=0","effect":"Cand=false；Sel无值"}
- `SINGLE`：{"id":"SINGLE","predicate":"|C|=1","effect":"Cand=true；任何合法Sel只能取唯一成员"}
- `MULTIPLE`：{"id":"MULTIPLE","predicate":"|C|>1","effect":"Cand=true；Sel_Theta须取C内一个且保留完整C"}

**合法组合 / 排除**：元素Cand与集合Cand同名异物应分类型；“最近候选”不是教义强制，Sel是已承认工程选择，需要确定算法版本和tie。选中元素不能来自另一未条件化集合。 Sel_Theta先在同一条件化C上按固定策略版本确定s；随后只评价本请求真正适用的独立门G(s)。全候选G(c)观察允许保留，但不能以存在其他G(c)=true替换G(s)=false，更不能因选中失败再重选。每个rung是否适用该门须按合同/CC-051核，不新增逐rung Weak。

**转换**：C随上级区间和事实版本变化可0/1/多任意迁移；已封存查询的C与Sel不变，重算另版本。 一次请求内顺序C确定→s确定→适用独立门G(s)；失败仍保留s。新原始事实/合法策略新版本可形成新请求，但不是同请求失败fallback。

**后端输出 / 前端观察**：显示候选数/全集、选中ID、选择规则及参数，多个候选不能隐藏。 同时返回selected_id、selector_version、selected_gate_name/适用域、selected_gate_result；全部候选评估放只读observation字段，证书消费者不得读取any_success代替selected_gate_result。

**证明状态**：CARDINAL_PARTITION_DEFINED_SELECTOR_CHOICE_UNINSTANTIATED。定位项：G-026。

**证据**：.chanlun/definitions/qujiantao.md:760-819。

### CC-050　区间链的完整约束签名

**范围**：ST-036。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：给定n层非空区间链r0..r(n-1)，每层语义适用性和输入证据齐全。

**完整分支 / 生成族（数量 {"family":"n>=1、0<=k<=n，2^(3(n-1)+2k+1)原始签名；不是全部语义可达"}）**：

- **generator**：对每相邻边记(level_decreases,index_subset,true_parent)3位；对每个适用rung记Cand与选择成员性2位；另叶确认Confirm。全部签名族{0,1}^{3(n-1)+2k+1}，k为适用rung数；全规定条件满足才是对应完整有效链。
- **empty**：n=0独立输入域：Prop折叠可真、Bool实例可拒绝；非同一判据，不能仅比较空链结果称冲突。

**合法组合 / 排除**：三套Lean骨架有效域不同、未有跨骨架等价；Confirm放内外须具体证书合同。级别连续是构造不变量，不由n_delta只验rung布尔自动推出。不得以严失败转宽成功。 证书只消费相应已选对象的适用判据G(s)，不消费CC-052的ANY_GATE_TRUE；选中门失败不得以其他候选真补证。Weak是否适用于当前层按原定义域，不从观察合同新增。

**转换**：构造链→验完整rung/边→叶Confirm→证书；任一实际失败保留全失败位。链变更重新验证，不复用不匹配祖先/版本的证书。

**后端输出 / 前端观察**：全rung/边见证、有效域、证书版本、Confirm位置、完整失败集合和获知时间可下钻查看。 每个被消费的门结果必须带selected_id及predicate_id，不以全集存在性观察充当选中见证。

**证明状态**：FINITE_SIGNATURE_COMPLETE_CERTIFICATE_EQUIVALENCE_MISSING。定位项：G-027。

**证据**：.chanlun/definitions/qujiantao.md:599-689；.chanlun/definitions/qujiantao.md:883-912；.chanlun/definitions/qujiantao.md:1001-1014。

### CC-051　Type2/3的区间套适用域

**范围**：ST-036。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：一个π入口的point type及上级rung。

**完整分支 / 生成族（数量 3）**：

- `TYPE1_RUNG_APPLIES`：{"id":"TYPE1_RUNG_APPLIES","predicate":"Type1入口所需结构Cand适用"}
- `TYPE2_RUNG_NOT_APPLICABLE`：{"id":"TYPE2_RUNG_NOT_APPLICABLE","predicate":"Type2入口上级rung结构Cand非其适用域"}
- `TYPE3_RUNG_NOT_APPLICABLE`：{"id":"TYPE3_RUNG_NOT_APPLICABLE","predicate":"Type3入口上级rung结构Cand非其适用域"}

**合法组合 / 排除**：N/A不能填true；不得填假再触发宽通道。Type2/3仍有自己的下级完成/构成义务，不能因为rung不适用就认为所有结构验证免除。如何进入现n_delta证书需明确后续裁定/实例化，当前不得静默选择。

**转换**：按入口对象分派语义适用域，同一Type不会因rung失败改成另一Type。

**后端输出 / 前端观察**：UI展示“不适用：该点型无此rung要求”，并列真正适用的构成/完成证据。

**证明状态**：APPLICABILITY_RULING_PRESENT_CERTIFICATE_HANDLING_PENDING。定位项：G-028。

**证据**：.chanlun/definitions/qujiantao.md:1084-1111。

### CC-052　下钻选择后判定与全候选观察分轴

**范围**：ST-038。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：D为一次固定parent/level/方向、原始事实前缀与策略版本的下钻请求：真实subs已完整取得，条件化候选C已完整构造；C非空时Sel_Theta(C)=s已定义且s∈C；本阶段确实适用的独立门G及其数据已给。G可为该语境的完整背驰/独立Weak门，但不得把它强加给所有rung。缺数据、选择未实例化、门不适用或扫描未完成走CC-054/CC-051，不能填bool。

**完整分支 / 生成族（数量 {"selected_request_partition":4,"readonly_all_candidates_partition":4,"joint_when_same_G_and_complete_evaluations":5,"per_candidate_observation_family":"|C|=m时为有ID的{false,true}^m向量；向量观察不重选s。"}）**：

- **production_selected_partition**：[{"id":"EMPTY_SUBS","predicate":"subs=∅，因而C=∅","effect":"无可供选中的次级成员"},{"id":"NO_ALIGN","predicate":"subs≠∅且C=∅","effect":"无满足条件的对齐候选"},{"id":"SELECTED_GATE_FALSE","predicate":"C≠∅且G(Sel_Theta(C))=false","effect":"本请求选中对象的适用门失败；其他候选成功不改变此结果"},{"id":"SELECTED_GATE_TRUE","predicate":"C≠∅且G(Sel_Theta(C))=true","effect":"仅本请求这一适用门通过；不是完整区间链或交易证书成功"}]
- **readonly_candidate_observation**：[{"id":"EMPTY_SUBS","predicate":"subs=∅"},{"id":"NO_ALIGN","predicate":"subs≠∅且C=∅"},{"id":"ALL_GATES_FALSE","predicate":"C≠∅且∀c∈C,G(c)=false"},{"id":"ANY_GATE_TRUE","predicate":"C≠∅且∃c∈C,G(c)=true"}]
- **same_predicate_joint_valid_family**：使用同一个G且全集已完整评估时，(selected,observation)恰有5种允许签名：EMPTY_SUBS/EMPTY_SUBS；NO_ALIGN/NO_ALIGN；SELECTED_GATE_FALSE/ALL_GATES_FALSE；SELECTED_GATE_FALSE/ANY_GATE_TRUE；SELECTED_GATE_TRUE/ANY_GATE_TRUE。SELECTED_GATE_TRUE/ALL_GATES_FALSE不可能。
- **different_predicate_warning**：若另展示Div(c)而选中门是Weak(s)，必须给不同predicate_id；没有Div与Weak桥就不能套用上面的5组合约束，更不能将观察Div存在性作为选中Weak结果。
- **historical_name_boundary**：qujiantao:1047的EmptySubs/NoAlign/DivFalse只为历史None失败分因；不授权把∃Div变成新准入。生产SELECTED_GATE_FALSE在G=Div时可标DivFalse，在G=Weak时必须标WeakFalse。

**合法组合 / 排除**：选择判定与全集观察是不同对象/用途。允许C={a,b},s=a,G(a)=false,G(b)=true，此时生产SELECTED_GATE_FALSE、观察ANY_GATE_TRUE同时成立，绝不能救回选中失败。相同G时selected_true⇒any_true；不同predicate_id不自行推蕴含。任何失败不构成小转大。

**转换**：同请求先建立C、按固定Sel确定s，再评价适用G(s)；只读评价其他候选不得反馈重选。新事实可生成新版本请求，但旧s和结果不变。选中门真之后仍须继续其余链/叶确认，不能提前发布完整证书。

**后端输出 / 前端观察**：返回subs、完整C、selected_id、selector_version、selected_gate{predicate_id,applicability,result,evidence}及readonly_candidate_evaluations；证书/经营只能按合同消费selected_gate，UI并排显示选中失败和其他候选成功，不把ANY_GATE_TRUE渲染为本请求已通过。

**证明状态**：SELECTED_PREDICATE_DOMAIN_AND_READONLY_OBSERVATION_SEPARATED。定位项：G-026。

**证据**：.chanlun/definitions/qujiantao.md:1024-1049；.chanlun/definitions/qujiantao.md:760-775；.chanlun/definitions/qujiantao.md:807-819。

### CC-053　下钻成本门与塔底停止

**范围**：ST-037。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：当前下钻级别，成本估计和适用塔底参数均已提供。

**完整分支 / 生成族（数量 4）**：

- `CONTINUE`：{"id":"CONTINUE","predicate":"not cost_stop and not at_base"}
- `COST_ONLY`：{"id":"COST_ONLY","predicate":"cost_stop and not at_base"}
- `BASE_ONLY`：{"id":"BASE_ONLY","predicate":"not cost_stop and at_base"}
- `BOTH`：{"id":"BOTH","predicate":"cost_stop and at_base"}

**合法组合 / 排除**：停止=cost_stop∨at_base，“谁先到算谁”；两触发可同真，不用优先级消一条。当前基底线段，后续笔/成交阶段为已批方向待条件；本轮不换基底。成本参数属任务政策，不凭原文设新数值。

**转换**：CONTINUE可向下一层；任一停止位为真就停在当前合法端点，缺成本数据另知识状态，不虚报stop=false。

**后端输出 / 前端观察**：显示两停止位、成本读数/参数版本、底层身份及触发时间。

**证明状态**：BOOLEAN_PARTITION_DEFINED_COST_POLICY_INSTANCE_REQUIRED。定位项：G-029。

**证据**：.chanlun/definitions/qujiantao.md:408-449；.chanlun/definitions/qujiantao.md:477-518。

### CC-054　数据、适用域、计算完成和结果有效性

**范围**：ST-010, ST-038, ST-044。**轴名分**：OBSERVATION_OR_KNOWLEDGE。

**D**：每一个分类请求及其事实集；这是知识/计算元信息，不是市场结构额外类别。

**完整分支 / 生成族（数量 54）**：

- **axes**：{"input_quality":["SUFFICIENT","INSUFFICIENT","INCONSISTENT"],"applicability":["IN_DOMAIN","OUTSIDE_DOMAIN","DOMAIN_PROOF_MISSING"],"computation":["COMPLETE","INCOMPLETE"],"validity":["CURRENT","SUPERSEDED","WITHDRAWN"]}
- **generator**：3×3×2×3=54个原始元信息签名；具体字段含完整原因集合。结构结果只有input sufficient∧in_domain∧complete且语义证据齐全时才可标已确定；旧结果可superseded保留。

**合法组合 / 排除**：这些状态不能占用三走势、六观察态或中心类型的缺失分支。DOMAIN_PROOF_MISSING是证据不足，非断言数学域外；INSUFFICIENT不可转false。既有有效旧快照可存在且同时新请求计算未完成。

**转换**：输入补齐/证明补齐/继续计算可恢复；修订使旧结果superseded或withdrawn并链接新版本，不能逆写语义确认。

**后端输出 / 前端观察**：每条结果附四轴和精确原因；UI明确“结构不成立”“尚缺证据”“不适用”的区别。

**证明状态**：OBSERVATION_CONTRACT_DESIGN_NOT_NEW_DOCTRINE。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/结构判据.md:9-39；.chanlun/definitions/qujiantao.md:338-376；.chanlun/definitions/xianduan.md:360-400。

### CC-055　全对象关系与变更事件身份

**范围**：ST-044。**轴名分**：ROLE_OR_RELATION_SCHEMA。

**D**：结构对象集合O：bar/group/fractal/bi/segment/feature_sequence/center/move/div_pair/bsp/rung_chain及明确版本。

**完整分支 / 生成族（数量 {"relation_kinds":12,"delta_forms":4,"market_event_cardinality":"对象数随有限前缀增长，按全typed edges有限枚举"}）**：

- **relation_family**：typed edge(source_id,relation_kind,target_id,version)，relation_kind严格取本项relation_kinds具名列表；每种关系都绑定相应CC轴见证。具名列表是关系数量的唯一数据源，不凭数目声明市场本体封闭。
- **delta_family**：对任何固定对象键：不存在→存在(建立)、存在→同身份新版(发展)、存在→已完成版(确认/终结)、旧版→被撤回/替代(修订)；每类带before/after、event_at、known_at及因果。
- **relation_kinds**：["member_of","derived_from","successor_of","leaves","retests","returns_to","extends","newborn_after","expands_with","caused_turn_at","confirms","selected_from"]

**合法组合 / 排除**：相同时间/价不等于相同对象；事件记录关系成立/变更，不用最新状态覆盖历史小转大/中枢扩展。本目录具名关系由观察要求派生，其数量从relation_kinds列表计算；若扩展对象/关系必须补覆盖和版本说明，不宣称本列表封闭全部市场本体。

**转换**：建立/发展/确认依各CC轴前件；撤回只走版本和来源修订。一个输入可产生多个合法变化事件，全部发出，不全局first-match。

**后端输出 / 前端观察**：查询同源快照、全关系、按游标变化、撤回因果及按当时可知回放；前端只消费同源计算结果。

**证明状态**：REQUIRED_OBSERVABILITY_DESIGN_GLOBAL_TRANSITION_CLOSURE_UNPROVEN。定位项：G-030。

**证据**：.chanlun/definitions/结构判据.md:9-39；.chanlun/definitions/level_recursion.md:329-356；.chanlun/definitions/maimai.md:367-408。

### CC-056　唯一归类和对象唯一身份的不同义务

**范围**：ST-002, ST-005, ST-044。**轴名分**：ROLE_OR_RELATION_SCHEMA。

**D**：给定输入前缀I、规则版本V、参数P和读法R。

**完整分支 / 生成族（数量 {"version":3,"obligations":2}）**：

- **classification**：函数结果在D中的分支唯一是各轴互斥覆盖所得。
- **identity**：同(I,V,P,R)下同一对象必须稳定ID；候选极值ties、分解Sel、修订代际不由谓词唯一自动推出。
- **version**：同代际追加 / 原始事实修订 / 规则或参数更换三支。

**合法组合 / 排除**：不能用choose函数total性声称raw→唯一对象→唯一分区全部已证。R不同的旁路对象不可互相覆盖；main tower同(I,V,P)仅一份。

**转换**：追加只用已允许发展转换；修订或规则变化新代际，保存旧版和替代原因。

**后端输出 / 前端观察**：返回input_frontier、semantic_version、policy_version、view_role、object_generation，可比对同输入重放。

**证明状态**：IDENTITY_DETERMINISM_REQUIRES_TIE_AND_VIEW_PROOF。定位项：G-002, G-030。

**证据**：.chanlun/definitions/bi.md:380-420；.chanlun/definitions/fenjie.md:1-45；.chanlun/definitions/qujiantao.md:815-819。

### CC-057　多级联立与共振

**范围**：ST-046。**轴名分**：OBSERVATION_OR_KNOWLEDGE。

**D**：同一known_at、同一市场事实代际的m个有限级别；每级若为Origin LevelView须各自给cand1/2/3真实投影。若输入只是Rust逐点BspBits集合，须声明另一carrier_kind，不能自动套用每级一个LevelView。

**完整分支 / 生成族（数量 {"raw_encoding_family":"64^m","upper_bound_if_each_level_is_Origin_LevelView":"27^m","semantic_reachable_count":"未给构造与跨级可达证明；逐点列表不得直接套此上界"}）**：

- **generator**：编码空间每级64，联合为64^m；在各级确属Origin LevelView时，由每类买卖排斥进一步限制为A_27^m，至多27^m个抽象允许联合签名；再受同端标签、共享血缘、坐标及同知时点约束。27^m不是全可达数。
- **resonance**：至少两个不同级别同时Type1为原文共振描述；不是独立阈值评分或新增准入门。

**合法组合 / 排除**：每级独立Type1条件不可被高层通过代替；像素同点/历史曾同时不等同本次known_at同步。2/3重合与跨级共振不同轴，可共存。 每级LevelView都遵CC-041三条同类买卖排斥；若BSP聚合载体不同，先定义并证投影，不把27^m读成全部生产列表数。

**转换**：新级别事实到来产生新截面；未来确认不能回填过去同知时点，按旧前缀可复演。

**后端输出 / 前端观察**：以级别×事实矩阵显示完整联合向量及血缘关系；保留多个级别标签和各known_at。

**证明状态**：FINITE_PRODUCT_DEFINED_CROSS_LEVEL_REALIZABILITY_UNPROVEN。定位项：G-031。

**证据**：.chanlun/definitions/zoushi.md:298-313；docs/chanlun/text/blog/017-第17课.md:66-72；.chanlun/definitions/maimai.md:277-309。

### CC-058　结构到持久重经营的接口名分

**范围**：ST-043。**轴名分**：ROLE_OR_RELATION_SCHEMA。

**D**：已确认范围内的结构对象/变化和经营请求；只覆盖ST-043桥，35FG由经营目录给完整分类。

**完整分支 / 生成族（数量 3）**：

- `STRUCTURAL_FACT`：{"id":"STRUCTURAL_FACT","predicate":"上述各CC轴产出的类型/状态/关系/完成和变化"}
- `OPERATION_CONTEXT`：{"id":"OPERATION_CONTEXT","predicate":"固定Q及某持久重/Voice的结构引用、成本/容量/频率等已裁经营条件"}
- `ECONOMIC_EFFECT`：{"id":"ECONOMIC_EFFECT","predicate":"由经营权威根据事实和条件处理的动作/成交/账目，完整类别在FG目录"}

**合法组合 / 排除**：fengkong和trading-system生成态历史文字不能替代已裁持久重语义；“成本归0”是事件而非与正/零成本并排的第三状态。结构观察端不得写经济权威；此三支是角色，不是完整经济分类。

**转换**：结构变化以不可歧义引用输入FG流程；经营不会反向修改主塔定义或选择另一个较松结构判据。

**后端输出 / 前端观察**：每重可查采用哪些结构对象/事件及当时版本；UI同图呈现结构与经营而非靠价格重算结构。

**证明状态**：INTERFACE_SCOPE_ONLY_FG_CLASSIFICATION_EXTERNAL。定位项：无额外语义缺口登记；仍以本稿总体证明边界为限。

**证据**：.chanlun/definitions/fengkong.md:1-38；.chanlun/definitions/fengkong.md:243-286；.chanlun/definitions/chanlun-trading-system.md:1-35；.chanlun/definitions/chanlun-trading-system.md:76-113。

### CC-059　二类点转折锚有无同级一类的完整补充分支

**范围**：ST-027, ST-030, ST-043。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：已识别q级转折锚后的首次次级往返；参考极值、side及次级走势完成可核；不得把任意价高/低当转折锚。

**完整分支 / 生成族（数量 24）**：

- **generator**：has_same_q_type1∈{0,1}×side∈{BUY,SELL}×cmp(retrace_extreme,anchor_extreme)∈{LT,EQ,GT}×range_divergence∈{0,1}，24签名。
- **source_rule**：053:28：SELL为高点后一次级下、再一次级上，若不创新高(比较LT/EQ)或盘整背驰即二卖；BUY镜像：不创新低(GT/EQ)或盘整背驰。has_same_q_type1=0的小转大支明确存在。
- **obligation**：该原文完整语义须与次级一类构成定律、确认条件和常规CC-044桥接；本目录保留全部签名与依据，不以代码afterTypeOne=false直接拒绝整支。

**合法组合 / 排除**：有无同级一类分的是转折锚处，不是说整个q级历史从未出现一类。同级一类不存在≠下级一类不存在；越一类极值仍可经盘背成立，符合101最弱二类描述。原文“不创新极值”有EQ，不能取strict而漏边界。 i1/i2不属于这个q级锚轴；只在CC-044同一descend(parent)的q-1序列内使用。无同级Type1分支不能借一个虚构q级Type1事件去初始化i1。当前ST-030用词未显式禁止无q级一类，本目录是消歧和补显式分支，不宣称首阶段已违反。

**转换**：小转大实际转折→首次次级往返→相应不创新极值或盘背/下级构成见证齐备→二类确认；未有本级一类不阻断此支。

**后端输出 / 前端观察**：前端明确“同级一类有/无”、转折锚、小级背驰、首次往返、极值三分和盘背证据；不得为了单一路径生成虚构同级一类。

**证明状态**：SOURCE_BRANCH_PRESENT_COMMON_IMPLEMENTATION_BRIDGE_MISSING。定位项：G-024, G-032。

**证据**：.chanlun/definitions/maimai.md:120-133；.chanlun/definitions/maimai.md:277-280；docs/chanlun/text/blog/053-第53课.md:26-32；formal/Origin/BspClassification.lean:131-179；formal/Origin/RMoveCompose.lean:217-238。

### CC-060　点相对中枢的位置三态

**范围**：ST-013, ST-018, ST-031, ST-044。**轴名分**：OBSERVATION_OR_KNOWLEDGE。

**D**：给定Center与一个点p，Center.valid要求ZD<=ZG；正式center另需strict seed。

**完整分支 / 生成族（数量 3）**：

- `BELOW`：{"id":"BELOW","predicate":"p<ZD"}
- `WITHIN`：{"id":"WITHIN","predicate":"ZD<=p且p<=ZG"}
- `ABOVE`：{"id":"ABOVE","predicate":"p>ZG"}

**合法组合 / 排除**：该三态允许形式载体退化center，语义正式center是strict子域；相切两边均归WITHIN。它是CC-026有center时的几何投影，非三类走势或三类买卖点。

**转换**：任意新p可改变观察标签；三态静态证明不证明必须按BELOW→WITHIN→ABOVE顺次交易。

**后端输出 / 前端观察**：显示位置三态与p、ZD/ZG，保留等边界，供六态与第三类严格性复核。

**证明状态**：LEAN_TOTAL_DISJOINT_IFF_SOURCE_PRESENT_NOT_EXECUTED。定位项：G-015。

**证据**：formal/Origin/CenterStates.lean:50-112；formal/Origin/BspClassification.lean:183-216。

### CC-061　中枢结束后两种发展结果

**范围**：ST-013, ST-014, ST-031, ST-047。**轴名分**：SEMANTIC_STATE_OR_EVENT。

**D**：已由次级离开+首回试构成三类而结束的q级中枢；后续有足够结构形成结果。

**完整分支 / 生成族（数量 2）**：

- `LARGER_CENTER`：{"id":"LARGER_CENTER","predicate":"转成更大级别的中枢；各高级seed仍按同一构造判据证"}
- `NEW_SAME_LEVEL_CENTER`：{"id":"NEW_SAME_LEVEL_CENTER","predicate":"上涨/下跌发展直到形成新q级中枢；同向新生外缘分离条件另验"}

**合法组合 / 排除**：两分指该结束中枢后结果，不等于当前离开瞬间可预知；与29课三结果分的对象/分解语境不同，不能全局互斥。新q级中枢之后未来还可形成更高级，需固定本次观察终点及分解上下文。

**转换**：center三类结束→等待结构完成→相应结果；当下只能先发三类/结束事件，再发已形成结果。

**后端输出 / 前端观察**：同时显示中心结束事件、后续二分结果及构造证据，不直接把三类标签解释成趋势已成。

**证明状态**：SOURCE_DICHOTOMY_CONTEXT_AND_REALIZATION_BRIDGE_REQUIRED。定位项：G-022。

**证据**：docs/chanlun/text/blog/053-第53课.md:30-32；.chanlun/definitions/zhongshu.md:216-298。

### CC-062　同级已完成走势连接和读法约束

**范围**：ST-009, ST-017, ST-019。**轴名分**：FINITE_PREDICATE_OR_SIGNATURE_FAMILY。

**D**：给定同级分解读法R、相邻两个已完成走势实例T1,T2；两者身份不同、无未分配接缝。

**完整分支 / 生成族（数量 {"raw_signatures":9,"unconditionally_excluded":2,"potential_remaining":7}）**：

- **generator**：(type(T1),type(T2))∈{P,U,D}²，9签名；U→U和D→D被同级走势连接语义排除；余7签名必须再核R的构造/分解规则。
- **operation_view**：ADR0011固定三格禁延伸旁路允许P→P，不得以主塔直觉一概排除。
- **main_tower**：P→P是否由延伸/扩展收编须由主塔分解域决定，不能把操作读法七种潜在连接宣称全为主塔可达。

**合法组合 / 排除**：禁止U→U/D→D不意味U后必D，U→P合法；P后可U/D，旁路可P。最终连续序列的合法族为所有相邻边满足本读法约束的有限词，长度n按此边关系生成，不另列无限长清单。

**转换**：已完成旧T→新实例；同向延续应仍属同实例发展或正确更级分解，不为事件展示制造同级同向继任。

**后端输出 / 前端观察**：给连接两端类型、q、读法、边界和完成见证；图上旁路连法注明来源。

**证明状态**：FINITE_ADJACENCY_SIGNATURE_COMPLETE_REACHABILITY_BRIDGE_REQUIRED。定位项：G-008, G-012。

**证据**：docs/chanlun/text/blog/017-第17课.md:52-60；docs/chanlun/text/blog/043-第43课.md:18-20；.chanlun/definitions/fenjie.md:1-45；docs/adr/0011-operation-decomposition-layer.md:29-95。

## r2复核修订回执

前版JSON SHA-256：`7b954f7c1deb40db31f9339304688e6ac9da464c934534533ea7fdbb22e9c07a`；前版MD：`87963f91816da65f5423bf8f503cf98dc91244fa846b6ba66e9879e3d7e3cffa`。所有条目为作者已修、待主控关闭，不代独立复核结论。

- **RC-M01**（CC-004）：补UP/DOWN非包含追加，把两种有方向合并和未建方向限定包含域；共7叶。 自检：225个闭区间对×3种方向来源逐对恰一叶，另验空/单根及原审反例n=2非包含UP。
- **RC-M02**（CC-037）：把X限定为r<q已发生小转大，并明确N的旧趋势方向对偶。 自检：覆盖N/X四个签名；r=q转折标为本轴不适用而非X&&!N违例。
- **RC-M04**（CC-032, CC-045）：明确接口域、语义子域和不同数量射程；空样本、无效记录、缺数据及未完成签名均有明确去向。 自检：CC032接口4分区及有效长度×符号4签名；CC045枚举96=72未完成+24已完成，24=18非正式首回试+6首回试，6中2成立。
- **RC-M03**（CC-049, CC-050, CC-052）：分开固定Sel后门判定与全候选只读观察，禁止选中失败后择优重选；观察ANY_GATE_TRUE不能供证书消费。 自检：枚举1..3候选全部布尔向量及每个固定选择；selected=false且any=true反例保留，joint合法组合共5含空/无对齐。
- **RC-L01**（CC-055）：原有12个具名关系不变，数量改为从唯一列表len导出，删除错误13/14表述。 自检：assert len(relation_kinds)==len(set(relation_kinds))==branch_count.relation_kinds==12。
- **RC-M05**（CC-041, CC-042, CC-057）：加入LevelView每类买卖排斥：64编码→37已排除→至多27抽象允许；修订跨级上界与逐点BspBits/LevelView区分。 自检：枚举全部64向量，27满足∀k¬(Bk∧Sk)，37违反；只验证编码约束数量，不验证全部27语义构造。
- **SOURCE-CLAIM-MATCH**（CC-030, CC-031, CC-032, CC-034, CC-036, CC-037, CC-038, CC-052）：按当前beichi章节实质校正全部相关引用及G016/G017/G018/G020；保留649旧保证仅作冲突证据，移除它对局部出口的误支撑。 自检：每个现存beichi引用附具体主张、范围摘要SHA与文字锚；人工逐项核实，不以行存在替代。

## 当前实现的有界证据

采用主控已完成的有界只读源码核查；本作者通读报告，未重复其发现扫描。源文件字节绑定见该报告snapshot.files。

仅三个家族的计算、Classification承载与所核出口；dirty工作区静态字节，非全仓语义等价不存在证明，未编译或运行。

- **六态/位置观察**（CC-026, CC-060）：Rust rlevel_of纯函数存在；所核生产Classification装配没有六态字段/调用连接；断点在纯函数到生产context与装配，不是已计算后只被Python丢掉。 出口：所核PyThetaStream及Γ JSONL未输出六态。
- **中枢发展关系**（CC-020, CC-055）：classify_relation确在decompose_resume生产调用；非单点MoveBlock的span及(kind,dir,level_lift)和centers在完整Classification中保留关系信息。单中枢块不代表已有pair关系。 出口：newly_confirmed_step清空moves/centers，所核信号差分失去该轴；PyThetaStream与Γ JSONL未输出它。
- **逐点六位与每级LevelView**（CC-041, CC-042, CC-057）：endpoint_to_bsp生产真实置六位，Classification.levels[*].bsp[*].bits保留逐点位；append+sort没有建立每级每时刻唯一LevelView投影。 出口：Rust信号差分保留逐点bits，Γ的bsp_bits_class_index保留逐候选完整六位，bsp_class只是另一个主类字段；PyThetaStream不出该签名。

证据报告：`/tmp/newchanlun-1339-architecture-20260908/evidence/CLASSIFICATION-OUTPUT-PROBE.md`；JSON SHA-256：`fa163817464681376175a3b07cb86054d1c959906c6cdc43008b5042aceedeb6`。不据此补称全部叶的生产或运行验证。

## beichi引用实质匹配与残留权威回核

所有本稿当前beichi引用已逐项对照所承担主张；JSON的source_claim_match_receipts列对象、具体主张、来源角色、范围SHA及文本锚。这里不把行存在检查升级为全部来源已审计。

- CC-025 — `.chanlun/definitions/beichi.md:504-584`：Div结构+力度、NE、Turn、当下可知与尚缺EM验收桥（SUPPORTS_NAMED_CLAIM）。
- CC-030 — `.chanlun/definitions/beichi.md:261-284`：先结构配对后力度：趋势c对b，盘整首次进入段/重复最近同向跨界段（SUPPORTS_NAMED_CLAIM）。
- CC-031 — `.chanlun/definitions/beichi.md:504-534`：Div定义及五条结构前提；NE与高级结果分离（SUPPORTS_NAMED_CLAIM）。
- CC-031 — `.chanlun/definitions/beichi.md:286-303`：次级别Extreme不删除及禁止宽松fallback（SUPPORTS_NAMED_CLAIM）。
- CC-032 — `.chanlun/definitions/beichi.md:336-373`：L=末速度减首速度、笔速度坐标、四端点、符号值和不绕父（SUPPORTS_NAMED_CLAIM）。
- CC-032 — `.chanlun/definitions/beichi.md:463-469`：当下笔未完成样本的工程取值细节（SUPPORTS_NAMED_CLAIM）。
- CC-033 — `.chanlun/definitions/beichi.md:504-584`：Div结构+力度、NE、Turn、当下可知与尚缺EM验收桥（SUPPORTS_NAMED_CLAIM）。
- CC-034 — `.chanlun/definitions/beichi.md:792-830`：T4零轴前提与趋势域、三维OR和各维严格/非严格读数（SUPPORTS_NAMED_CLAIM）。
- CC-034 — `.chanlun/definitions/beichi.md:374-396`：MACD代理数学论证近似边界与TV已退役（SUPPORTS_NAMED_CLAIM）。
- CC-034 — `.chanlun/definitions/beichi.md:1005-1019`：版本登记：#985带号同色柱处置、#994TV退役及#814OR后裁；只作后裁历史指针（HISTORICAL_RULING_POINTER）。
- CC-035 — `.chanlun/definitions/beichi.md:504-584`：Div结构+力度、NE、Turn、当下可知与尚缺EM验收桥（SUPPORTS_NAMED_CLAIM）。
- CC-036 — `.chanlun/definitions/beichi.md:127-145`：转折r=q/r<q二分与小转大必要条件，主语及方向受限（SUPPORTS_NAMED_CLAIM）。
- CC-036 — `.chanlun/definitions/beichi.md:164-199`：小转大俗称名分、线段/走势两层替代物和不新增买卖点类型（SUPPORTS_NAMED_CLAIM）。
- CC-037 — `.chanlun/definitions/beichi.md:127-145`：转折r=q/r<q二分与小转大必要条件，主语及方向受限（SUPPORTS_NAMED_CLAIM）。
- CC-040 — `.chanlun/definitions/beichi.md:649-658`：旧文仍声称回到核心的张力证据，不能用作29课三结果或43课局部出口定义（DOCUMENTED_CONFLICT_ONLY）。
- G-014 — `.chanlun/definitions/beichi.md:504-584`：Div结构+力度、NE、Turn、当下可知与尚缺EM验收桥（SUPPORTS_NAMED_CLAIM）。
- G-016 — `.chanlun/definitions/beichi.md:261-284`：先结构配对后力度：趋势c对b，盘整首次进入段/重复最近同向跨界段（SUPPORTS_NAMED_CLAIM）。
- G-016 — `.chanlun/definitions/beichi.md:504-534`：Div定义及五条结构前提；NE与高级结果分离（SUPPORTS_NAMED_CLAIM）。
- G-017 — `.chanlun/definitions/beichi.md:336-373`：L=末速度减首速度、笔速度坐标、四端点、符号值和不绕父（SUPPORTS_NAMED_CLAIM）。
- G-017 — `.chanlun/definitions/beichi.md:463-469`：当下笔未完成样本的工程取值细节（SUPPORTS_NAMED_CLAIM）。
- G-018 — `.chanlun/definitions/beichi.md:792-830`：T4零轴前提与趋势域、三维OR和各维严格/非严格读数（SUPPORTS_NAMED_CLAIM）。
- G-018 — `.chanlun/definitions/beichi.md:374-396`：MACD代理数学论证近似边界与TV已退役（SUPPORTS_NAMED_CLAIM）。
- G-018 — `.chanlun/definitions/beichi.md:1005-1019`：版本登记：#985带号同色柱处置、#994TV退役及#814OR后裁；只作后裁历史指针（HISTORICAL_RULING_POINTER）。
- G-019 — `.chanlun/definitions/beichi.md:504-584`：Div结构+力度、NE、Turn、当下可知与尚缺EM验收桥（SUPPORTS_NAMED_CLAIM）。
- G-020 — `.chanlun/definitions/beichi.md:127-145`：转折r=q/r<q二分与小转大必要条件，主语及方向受限（SUPPORTS_NAMED_CLAIM）。
- G-020 — `.chanlun/definitions/beichi.md:164-199`：小转大俗称名分、线段/走势两层替代物和不新增买卖点类型（SUPPORTS_NAMED_CLAIM）。
- G-022 — `.chanlun/definitions/beichi.md:649-658`：旧文仍声称回到核心的张力证据，不能用作29课三结果或43课局部出口定义（DOCUMENTED_CONFLICT_ONLY）。

G-025/G-028另承接主控限定关键词与#817/#908评论回核：`/tmp/newchanlun-1339-architecture-20260908/evidence/PENDING-AUTHORITY-FOLLOWUP.md`。两项具体残留仍保留，closed不替代选择；不声称全仓不存在别名后续裁定。

## 跨轴合法组合合同

- **LC-01**（CC-001, CC-002, CC-003）：每一具体几何输入落唯一原子叶；包含、闭重叠、gap为该叶的确定投影，允许多标签，不以优先级删去包含且重叠。
- **LC-02**（CC-006, CC-008, CC-010, CC-011, CC-013）：四分型→新笔条件→成笔/确认→线段条件→两情况，每层只有先证明输入属D才判断下一层；上一层失败不能改用另一宽档。
- **LC-03**（CC-016, CC-017, CC-018, CC-019, CC-020, CC-021）：正式center只有strict seed；核心冻结；单元离开、首回试三类/破坏、pair扩展、新生、九段处理分对象可同次更新并发出，九段优先级只作用其已裁互竞构造决策，不抹别的关系事实。
- **LC-04**（CC-023, CC-025, CC-026, CC-060）：完成三类型、形成完成继任、六态、点位置三态分别字段；由同context一致性约束绑定。任何一个标签不替代另外三个轴。
- **LC-05**（CC-030, CC-031, CC-032, CC-033, CC-034, CC-035）：结构pair/五前提→精确力度→Div，代理只读；NE/Turn及高级结果有独立见证，不作Div循环前提。
- **LC-06**（CC-036, CC-037, CC-038, CC-039, CC-040, CC-061）：r=q/r<q只分类已发生q转折；末次级三类是必要而非充分；局部中心出口/重叠与高级结果独立，保留未发生转折继续原趋势分支。
- **LC-07**（CC-041, CC-042, CC-043, CC-044, CC-045, CC-046, CC-059）：Origin LevelView逐类满足∀k¬(Bk∧Sk)，64编码仅至多27个已知约束允许签名；同端同级同side仍仅空、1、2、3、2+3，两个限制作用域不同。不同类别可不同端点反向并存；无同级一类的二类支必须覆盖。Rust逐点bits列表、Γ逐候选六位不自动构成每级LevelView；其投影/可达性另证。
- **LC-08**（CC-047, CC-048, CC-049, CC-050, CC-051, CC-052, CC-053）：真实父子/几何包含→条件化Cand全集→固定Sel选中s→完整链及适用门G(s)/叶Confirm；全候选ANY_GATE_TRUE仅只读观察，不得替选中失败或触发重选。不是每个rung都新增Weak。不适用不填真、停止两触发可同真、下钻失败不是小转大。
- **LC-09**（CC-007, CC-029, CC-034, CC-039, CC-054）：观察描述、MACD、计算知识/有效性均不得填补市场定义分支或直接升级生产准入；保留可见性而不造新阈值。
- **LC-10**（CC-015, CC-055, CC-056, CC-057, CC-058, CC-062）：唯一主塔、多操作读法、定位查询的身份不混；所有结构/关系/变化同源版本+known_at；经营消费不会回写主塔，跨级联合签名必须保留全部见证。

## 已裁未同步、未证、工程实例与真正残留

“本轮未证”不等于“全仓无定理”。真正未裁/待处置只按原文和正本具体明示登记；观察描述不列为新增裁定义务。

- **explicit_unsettled_semantic_or_certificate_items**：G-025, G-028
- **explicit_pending_scan_choice_needs_followup_check**：G-007
- **already_defined_or_ruled_sync_or_model_alignment**：G-005, G-006, G-010, G-013, G-017, G-022, G-032
- **engineering_choice_or_predicate_instance**：G-001, G-002, G-008, G-011, G-026, G-029
- **observation_only_not_exhaustive_semantic_obligation**：G-003, G-021
- **proof_boundaries_not_a_claim_of_no_theorem_elsewhere**：G-004, G-009, G-012, G-014, G-015, G-016, G-018, G-019, G-020, G-023, G-024, G-027, G-030, G-031

### G-001　包含输入初始方向未建时的处理

**名分**：ENGINEERING_CHOICE_INSTANCE。涉及 CC-004。

**已有**：UP/DOWN合并规则与非包含方向已定义；初始默认UP在文字中是工程处理。

**剩余义务**：明确前缀建立方向或初始化策略及其输出影响；不得默默把默认UP当原文唯一方向。

**来源**：.chanlun/definitions/baohan.md:115-155。

### G-002　等价极值和端点身份唯一

**名分**：PROOF_OR_IDENTITY_INSTANCE。涉及 CC-005, CC-009, CC-056。

**已有**：最极端价格选择已定义，价格最值唯一不自动推出raw端点身份唯一。

**剩余义务**：极值同价多根、同型同价端点的稳定身份选择及原始/合并映射确定性。

**来源**：.chanlun/definitions/bi.md:163-210；.chanlun/definitions/bi.md:386-392。

### G-003　描述性强弱和中枢形态不升格分类

**名分**：OBSERVATION_ONLY_NOT_A_GAP。涉及 仅界限说明，无新增待裁轴。

**已有**：中枢形态正文明排除；分型强弱为描述性补充，不作为本轮强制穷尽标签。

**剩余义务**：保留来源和可观察证据；不要求本轮发明互斥数值分档。

**来源**：.chanlun/definitions/fenxing.md:64-88；.chanlun/definitions/zhongshu.md:450-462。

### G-004　确认的跨对象前缀单调桥

**名分**：PROOF_BOUNDARY。涉及 CC-010, CC-046。

**已有**：本轮未取得统一证明/实例化；不声称全仓不存在

**剩余义务**：以原始合法追加前缀证明笔/段/Move/BSP确认不倒退，修订另代际。

**来源**：.chanlun/definitions/bi.md:386-392；.chanlun/definitions/maimai.md:308-367；.chanlun/definitions/level_recursion.md:329-340。

### G-005　三笔相切已裁，旧正本登记未同步

**名分**：RULING_SYNC。涉及 CC-011。

**已有**：#246/#249明确相切算重叠且三笔起点<=；Origin缺口↔闭重叠已有定理。正式center非退化另域。

**剩余义务**：遵闭公共重叠；同步旧“未裁”登记及核对受影响实现，当前仅目录报告不改仓。

**来源**：classification-evidence-issue-246.json；classification-evidence-issue-249.json；.chanlun/definitions/xianduan.md:303-305；formal/Origin/SegmentFeatureSeq.lean:101-127。

### G-006　特征序列两情况、同向第二序列和G4封闭的统一桥

**名分**：DEFINED_SEMANTICS_PROJECTION_ALIGNMENT。涉及 CC-012, CC-013。

**已有**：P2两情况、同向第二序列、任意分型及strict封闭已裁；本轮只核到局部形式函数，未证明统一构造。

**剩余义务**：第二序列必须正确笔集合且接受任意分型；独立包含作用域和strict封闭对象须统一对齐；不能以旧提取revSeq或布尔case穷尽替代源语义。

**来源**：.chanlun/definitions/xianduan.md:118-176；formal/Origin/SegmentAutoConstruct.lean:130-195。

### G-007　第二特征序列扫描窗口

**名分**：EXPLICIT_PENDING_OR_STALE_IMPLEMENTATION_CHOICE。涉及 CC-014。

**已有**：xianduan现有文字明确“待裁”；其族I现役身份是历史陈述，本轮未把它当当前生产事实。

**剩余义务**：核查后续实施/裁定是否已统一50与完整扫描；任何预算耗尽都不得判为结构否定。

**来源**：.chanlun/definitions/xianduan.md:180-198；.chanlun/definitions/xianduan.md:327-340。

### G-008　唯一主塔与多读法的分解全覆盖

**名分**：PROOF_OR_ALGORITHM_INSTANCE。涉及 CC-015, CC-022, CC-062。

**已有**：本轮未取得统一证明/实例化；不声称全仓不存在

**剩余义务**：给主塔及固定三格旁路各自Φ、未分配尾部、无重复归属和可达连接证明，不要求二者逐字段相同。

**来源**：.chanlun/definitions/fenjie.md:1-45；docs/adr/0011-operation-decomposition-layer.md:29-95。

### G-009　真实递归输入到中枢与外缘的桥

**名分**：PROOF_BOUNDARY。涉及 CC-016, CC-020, CC-027, CC-028。

**已有**：本轮未取得统一证明/实例化；不声称全仓不存在

**剩余义务**：原始事实→真实完成次级单元→strict seed→全成员外缘→父子级差/覆盖→各级同判据；局部几何定理不能替这条链。

**来源**：formal/Origin/CenterStates.lean:36-41；formal/Origin/CenterStates.lean:215-250；.chanlun/definitions/level_recursion.md:117-178。

### G-010　第三类严格端点与旧文字/实现张力

**名分**：FORMAL_ENDPOINT_DEFINED_SYNC_TENSION。涉及 CC-019, CC-045。

**已有**：Origin已经定义B3 core.zg<retracePrice与S3 retracePrice<core.zd，并有reenter反例；本轮无新端点选择。

**剩余义务**：把Origin严格端点作为本目录边界，后续核并同步文字与实现；不得同时放行inclusive宽档。

**来源**：formal/Origin/BspClassification.lean:113-127；formal/Origin/BuySellPredicate.lean:419-429；.chanlun/definitions/maimai.md:136-149。

### G-011　升级计数的无波动排除实例

**名分**：ENGINEERING_PREDICATE_INSTANCE。涉及 CC-021。

**已有**：九段阈值和无波动不参与已裁；判据形状明归实施细节，不登记成教义未裁。

**剩余义务**：给无波动判法的可执行实例及其与源语义等价证据；不凭描述自定额外阈值。

**来源**：.chanlun/definitions/zhongshu.md:268-276。

### G-012　三走势的真实良构域覆盖

**名分**：PROOF_BOUNDARY。涉及 CC-023, CC-062。

**已有**：给定布尔/TrendSpec的total/unique不等于原始流构造域完备；本目录未提供后者证书。

**剩余义务**：证明完成对象总落恰一枢盘整/同向多枢趋势之一；多枢flat等输入必须正确升层或重分解。

**来源**：formal/Origin/TrendCompleteClassification.lean:34-88；.chanlun/definitions/zoushi.md:111-120。

### G-013　盘整无方向与Compose技术方向

**名分**：RULING_SYNC_OR_COMPATIBILITY_RETIREMENT。涉及 CC-024。

**已有**：盘整无语义方向已定；旧字段兼容不是另一教义。

**剩余义务**：技术break_direction与语义type分开，核既有兼容字段退场和下游消费者。

**来源**：.chanlun/definitions/qushi.md:179-200；.chanlun/definitions/zoushi.md:129-131。

### G-014　一般Move完成和Turn窄Completed的桥

**名分**：PROOF_BOUNDARY。涉及 CC-025, CC-035。

**已有**：本轮未取得统一证明/实例化；不声称全仓不存在

**剩余义务**：一般完成含小转大、盘整等语境；不能用Completed=Div的窄模型排除其他完成。继任须真实同级且不同身份。

**来源**：formal/Origin/Turn.lean:40-105；.chanlun/definitions/level_recursion.md:329-340；.chanlun/definitions/beichi.md:504-584。

### G-015　原始结构投影到位置/六态Context

**名分**：PROOF_BOUNDARY。涉及 CC-026, CC-060。

**已有**：位置三态、六态在给定context域上已有定义/证明源；本轮未运行Lean，也未证投影。

**剩余义务**：证明center,p,b3,s3同源、绑定正确center且当时可知；静态3/6态定理不提供顺序交易FSM。

**来源**：formal/Origin/CenterStates.lean:50-112；formal/Origin/TrendSixState.lean:1-185。

### G-016　b/c结构配对和发展态投影

**名分**：PROOF_BOUNDARY。涉及 CC-030, CC-031。

**已有**：本轮未取得统一证明/实例化；不声称全仓不存在

**剩余义务**：真实走势按最后中心进入b及后续c、盘整最近跨界对象配对；先结构后力度，当前前缀因果。

**来源**：.chanlun/definitions/beichi.md:261-284；.chanlun/definitions/beichi.md:504-534。

### G-017　力度已有定义，实际样本桥待证

**名分**：EXPLICIT_DEFINITION_SAMPLING_BRIDGE。涉及 CC-032。

**已有**：v=Δp/Δindex及L=v末−v首明确存在；不是力度未定义。

**剩余义务**：如何由每个实际发展段给首/末笔和四端点，并满足WellFormed和同坐标；空/零duration总化不得冒充语义样本。

**来源**：formal/Origin/ForceVelocity.lean:1-68；.chanlun/definitions/beichi.md:336-373；.chanlun/definitions/beichi.md:463-469。

### G-018　MACD辅助和精确力度的适用边界

**名分**：PROOF_BOUNDARY。涉及 CC-034。

**已有**：三维组合和不回退已裁；近似论证与历史实现情况不是当前实证。

**剩余义务**：三维OR、T4域、符号面积采样对齐；不能以近似来源声称MACD与Δv在任意行情等价。

**来源**：.chanlun/definitions/beichi.md:792-830；.chanlun/definitions/beichi.md:374-396；.chanlun/definitions/beichi.md:1005-1019。

### G-019　Div⇒NE/Turn的非循环实现桥

**名分**：PROOF_BOUNDARY。涉及 CC-035。

**已有**：本轮未取得统一证明/实例化；不声称全仓不存在

**剩余义务**：不得把NE/Turn当前/未来事实当前件循环定义Div；提供EM等实际桥的定义域及证据。

**来源**：.chanlun/definitions/beichi.md:504-584；formal/Origin/Turn.lean:40-105。

### G-020　小转大已经发生的定位见证

**名分**：PROOF_BOUNDARY。涉及 CC-036, CC-038。

**已有**：r=q/r<q二分明确；没有保证未来的充分条件是原文明说的边界，不是需要补的预测缺口。

**剩余义务**：真实q转折、r<q背驰、末次级中心相应三类必要条件均可定位；不能把候选/下钻失败当发生。

**来源**：docs/chanlun/text/blog/043-第43课.md:18-48；docs/chanlun/text/blog/044-第44课.md:24-30；.chanlun/definitions/beichi.md:127-145；.chanlun/definitions/beichi.md:164-199。

### G-021　操作描述的边界不冒充结构分类

**名分**：OBSERVATION_ONLY_OR_SEPARATE_OPERATION_CHOICE。涉及 CC-039。

**已有**：原文操作情形可作为完整原子关系观察；没有强制将所有行情分成这些描述词的教义义务。

**剩余义务**：参考价LT/EQ/GT全部可展示；本轮不立最强/普通/最弱交易档，也不在FG之外增加价格准入。

**来源**：docs/chanlun/text/blog/043-第43课.md:186-220；docs/chanlun/text/blog/044-第44课.md:34-38。

### G-022　高级结果分解和回拉外缘/核心

**名分**：PROOF_BOUNDARY_AND_SOURCE_TEXT_TENSION。涉及 CC-040, CC-061。

**已有**：原文已给分类和最弱边界；本轮未证从任意实际发展流识别结果的完备性。

**剩余义务**：兑现29三结果与53中心结束二结果的不同域；029:30最弱触DD/GG与旧beichi:651必回核心冲突按原文优先，不作同级整齐对齐。

**来源**：docs/chanlun/text/blog/029-第29课.md:16-52；docs/chanlun/text/blog/053-第53课.md:30-32；.chanlun/definitions/beichi.md:649-658。

### G-023　64编码、27约束允许上界及实际LevelView/逐点载体桥

**名分**：PROOF_BOUNDARY。涉及 CC-041, CC-042, CC-043。

**已有**：Origin LevelView每类一个Option端点；同类Bk/Sk互斥已有定理，因此64编码中仅27不违反这一约束。该27子集每项是否可由厚谓词构造，以及Rust逐点列表到每级LevelView桥，本轮未证明。

**剩余义务**：明确载体；LevelView必须保持∀k¬(Bk∧Sk)，并与局部同端同side五集合一致。补真实市场投影/逐点列表聚合与所有声称可达签名的见证，不能再说六位语义无条件独立。

**来源**：formal/Origin/BuySellPredicate.lean:385-429；formal/Origin/BspClassification.lean:218-242；.chanlun/definitions/maimai.md:172-178；formal/Origin/BuySellPredicate.lean:82-97；formal/Origin/BuySellPredicate.lean:169-208。

### G-024　二类构成形与可观测形双向桥

**名分**：PROOF_BOUNDARY。涉及 CC-044, CC-059。

**已有**：形式源明确将faithful参数与反向未证登记；本目录没有把参数当定理。

**剩余义务**：SubOf反自反/同side/真实次级归属；构成⇒观测和观测⇒构成皆需在实际实例证明，常规after&&notbroke不能替代原文所有二类。

**来源**：formal/Origin/BspClassification.lean:131-179；.chanlun/definitions/maimai.md:154-164。

### G-025　独立选父NestInterval的价格挂法

**名分**：EXPLICIT_SEMANTIC_CHOICE_PENDING。涉及 CC-047。

**已有**：同文件1266“未裁空”被1317明确“当前未裁”的追加残留覆盖；这是具体条款内部张力，不自称已无缺口。

**剩余义务**：按既有#817残留明确价格挂法，再验证跨级price-subset；不得用真实descendant的min/max单调性跨域。

**来源**：.chanlun/definitions/qujiantao.md:1317-1318。

### G-026　候选全集、条件集合与Sel实例

**名分**：PROOF_OR_SELECTOR_INSTANCE。涉及 CC-048, CC-049, CC-052。

**已有**：Cand三条件与集合非空已定义；Sel为已承认选择，其参数/算法实例不能凭教义留白捏造。

**剩余义务**：实例化Comparable/Extreme的对象投影、候选全集完整性与确定Sel；多候选全保留，选择不可越集合。 Sel后失败必须保留；候选全集任何真值观察不得重选或替换适用的selected gate。

**来源**：.chanlun/definitions/qujiantao.md:729-819。

### G-027　三套链骨架的有效域和证书桥

**名分**：PROOF_BOUNDARY。涉及 CC-050。

**已有**：本轮未取得统一证明/实例化；不声称全仓不存在

**剩余义务**：明确每套骨架D、Confirm内外和成员/层级不变量；跨骨架等价或退场条件需真正证明/验收。

**来源**：.chanlun/definitions/qujiantao.md:599-689；.chanlun/definitions/qujiantao.md:883-912。

### G-028　Type2/3上级rung不适用的n_delta处置

**名分**：EXPLICIT_CERTIFICATE_HANDLING_PENDING。涉及 CC-051。

**已有**：有效域Type1及Type2/3 N/A已裁；n_delta_rec如何处理N/A是正本明示残留，不因10.1总标题“未裁空”消失。

**剩余义务**：按1106仍需单独裁的处理来统一证书；不适用不得填true或因false翻掉所有Type2/3。

**来源**：.chanlun/definitions/qujiantao.md:1084-1111。

### G-029　成本停止门参数和基底进阶条件

**名分**：ENGINEERING_POLICY_INSTANCE。涉及 CC-053。

**已有**：stop两个触发与阶段方向已裁，不新增数值、资金政策或自动降到底门。

**剩余义务**：具体成本量与任务policy版本，当前底线段；阶段二必须满足原定两层条件，非本轮实施。

**来源**：.chanlun/definitions/qujiantao.md:408-449；.chanlun/definitions/qujiantao.md:477-518。

### G-030　全对象转换与版本因果闭包

**名分**：PROOF_BOUNDARY。涉及 CC-055, CC-056。

**已有**：本轮未取得统一证明/实例化；不声称全仓不存在

**剩余义务**：给每个可达转换的来源、不能发生转换的否证、事件多发不丢失、稳定身份和按当时可知回放；本目录仅列来源规则/必要约束，未证明全局可达图。

**来源**：.chanlun/definitions/结构判据.md:9-39；.chanlun/definitions/level_recursion.md:329-356；.chanlun/definitions/maimai.md:308-367。

### G-031　多级联合签名合法可达性

**名分**：PROOF_BOUNDARY。涉及 CC-057。

**已有**：本轮未取得统一证明/实例化；不声称全仓不存在

**剩余义务**：共享血缘和同知时点下，LevelView联合像必须先落A_27^m（每类买卖互斥），再证明其他跨级约束；原始编码64^m与上界27^m均不是可达性证明。生产逐点集合另需投影。

**来源**：docs/chanlun/text/blog/017-第17课.md:66-72；.chanlun/definitions/zoushi.md:298-313。

### G-032　小转大无同级一类仍有二类

**名分**：SOURCE_SEMANTICS_DEFINED_COMMON_MODEL_INCOMPLETE。涉及 CC-044, CC-059。

**已有**：053:28及maimai:280明确已有无q级一类支；RMoveCompose:217-238已将i1/i1+1定义在descend(parent)次级序列内。第一阶段ST-030没有显式禁止无同级一类，本项是层级消歧和覆盖桥，不是查实生产漏判。

**剩余义务**：分别给q级转折锚、q-1构成一类m1、同descend序列后继m2，以及053:28一般往返的对象映射；拒绝伪造q级Type1事件来初始化i1。

**来源**：docs/chanlun/text/blog/053-第53课.md:26-32；.chanlun/definitions/maimai.md:120-133；.chanlun/definitions/maimai.md:277-280；formal/Origin/BspClassification.lean:131-179。

## 44个ST对应

| ST | 需求 | 分类轴 |
|---|---|---|
| ST-001 | 包含关系、顺序合并与方向来源 | CC-001, CC-002, CC-003, CC-004 |
| ST-002 | 包含组锚、成员映射与原始极值定位 | CC-004, CC-005, CC-056 |
| ST-003 | 分型的局部分类与成立条件 | CC-001, CC-006 |
| ST-004 | 生产新笔的双坐标成立条件 | CC-001, CC-005, CC-008 |
| ST-005 | 笔序列、去重、延伸与确认边界 | CC-009, CC-010, CC-056 |
| ST-006 | 线段基本良构与正本范式 | CC-001, CC-011 |
| ST-007 | 特征序列构造、包含作用域与两类终结 | CC-002, CC-012, CC-013 |
| ST-008 | 缺口封闭、笔破坏与段破坏的独立事实 | CC-002, CC-013, CC-014 |
| ST-009 | 构造唯一与多读法分解分层 | CC-015, CC-062 |
| ST-010 | 结构判据与观察准入边界 | CC-054 |
| ST-011 | 中枢由三个连续次级别走势构成及严格核心 | CC-001, CC-002, CC-016 |
| ST-012 | 冻结核心与动态 GG/G/D/DD 全量可查 | CC-017 |
| ST-013 | 中枢延伸、离开与回归分别发布 | CC-002, CC-018, CC-019, CC-026, CC-060, CC-061 |
| ST-014 | 新生趋势与扩展关系的完整不等式 | CC-001, CC-002, CC-020, CC-061 |
| ST-015 | 九段升级计数、重切与高级血缘 | CC-021 |
| ST-016 | 中枢续扫起点、连接段归属与无候选态 | CC-022 |
| ST-017 | 走势三分类与相应级别的有效域 | CC-023, CC-062 |
| ST-018 | 盘整无语义方向与技术突破方向 | CC-024, CC-026, CC-060 |
| ST-019 | 构造、形成、完成、转折与过渡状态分轴 | CC-010, CC-025, CC-062 |
| ST-020 | 递归层级、基底、自然停止与三种级别身份 | CC-027 |
| ST-021 | 每级同一判据、构造父子不变量与可复验边界 | CC-027, CC-028 |
| ST-022 | 描述形态与PH类比结构名分分离 | CC-029 |
| ST-023 | 背驰先结构配对后力度及范围边界 | CC-030, CC-031, CC-033 |
| ST-024 | 力度精确定义、速度坐标与符号值 | CC-001, CC-032, CC-033 |
| ST-025 | MACD 辅助判据、数据不足与多代理禁兜底 | CC-033, CC-034 |
| ST-026 | Div、NE、Turn 命题与当时可知事实 | CC-025, CC-035 |
| ST-027 | 小转大为转折级别关系，必要条件与发生分离 | CC-036, CC-037, CC-038, CC-039, CC-059 |
| ST-028 | 六买卖谓词与可重合事实 | CC-041, CC-042 |
| ST-029 | 第一类点的背驰与本级核心破坏结构 | CC-031, CC-043 |
| ST-030 | 第二类点组成、回调越一类极值与非门化注记 | CC-001, CC-039, CC-042, CC-044, CC-059 |
| ST-031 | 第三类点首回试、冻结核心与等号待核 | CC-001, CC-018, CC-019, CC-045, CC-060, CC-061 |
| ST-032 | 买卖点确认单调性与数据修订分离 | CC-010, CC-046 |
| ST-033 | 区间套主动向下定位与向上构造分工 | CC-015, CC-047 |
| ST-034 | 区间套统一坐标、闭包含与真正父子来源 | CC-001, CC-002, CC-005, CC-028, CC-047 |
| ST-035 | Cand 元素、集合非空和独立力度门 | CC-048, CC-049 |
| ST-036 | 完整区间链、有效域、不适用与证书可复验性 | CC-050, CC-051 |
| ST-037 | 区间套成本停止、塔底及阶段进阶证据 | CC-053 |
| ST-038 | 下钻失败分类与知识状态独立于结构分类 | CC-052, CC-054 |
| ST-043 | 生成态风控/交易体系与经营结构关联 | CC-039, CC-058, CC-059 |
| ST-044 | 全对象关系和每次结构变化可追溯 | CC-025, CC-026, CC-054, CC-055, CC-056, CC-060 |
| ST-045 | 分型强弱、中继描述与其后发展分开 | CC-007 |
| ST-046 | 多级别联立与同步现象的同知时点 | CC-057 |
| ST-047 | 转折后的高级结果、回拉与中心生死关系 | CC-020, CC-035, CC-038, CC-040, CC-061 |
| ST-048 | 线段扫描上限、相切与结果完整性 | CC-011, CC-014 |

## 本目录自检

仅对目录内部引用、数量和有限公式进行自检；不充作项目验收或形式定理重编译。

```json
{
  "closed_interval_order_leaves": 26,
  "strict_interval_subset_leaves": 13,
  "interval_pairs_finite_sanity_cases": 225,
  "center_pair_five_way_finite_sanity_cases": 1225,
  "unit_vs_core_five_way_finite_sanity_cases": 150,
  "six_bsp_signatures": 64,
  "scope_44_st_covered": true,
  "all_axis_ids_unique": true,
  "all_source_ranges_exist": true,
  "all_gap_references_resolve": true,
  "r2_CC004_actual_fold_step_cases": 675,
  "r2_CC004_initialization_cases": 2,
  "r2_CC037_level_domain_cases": {
    "r_q_pairs": 14,
    "same_level_outside_X": 4
  },
  "r2_CC032_interface_cases": 8,
  "r2_CC032_length_value_signature_count": 4,
  "r2_CC052_nonempty_joint_signatures": 3,
  "r2_CC052_with_empty_and_noalign_joint_signatures": 5,
  "r2_CC052_selected_false_any_true_preserved": true,
  "r2_CC045_counts": {
    "E": 96,
    "unfinished": 72,
    "D_completed": 24,
    "complete_not_eligible": 18,
    "D_first": 6,
    "confirmed": 2
  },
  "r2_CC055_relation_count_from_single_list": 12,
  "r2_CC041_64_codes_partition": {
    "abstract_allowed": 27,
    "excluded": 37
  },
  "source_claim_matches_range": {
    "scope": "ALL_BEICHI_REFS_IN_CURRENT_AXES_AND_GAP_REGISTER",
    "receipt_count": 27,
    "all_references_accounted": true,
    "method": "manual named-claim match plus exact range anchor and SHA",
    "not_all_catalog_sources_reaudited": true
  },
  "r2_unchanged_44_ST_mapping": true,
  "not_a_lean_or_runtime_test": true,
  "not_semantic_reachability_proof": true
}
```

## 字节绑定与取证范围

JSON正本：`/tmp/newchanlun-1339-architecture-20260908/author/CLASSIFICATION-CATALOG.json`；SHA-256 `b8713384045dd3f15dad14cd5f77d89a92c4176bbe3487d6ba4a09432041106f`。机器可读稿逐项含D、分支、证明三义务状态、组合/转换、ST对应、源文件哈希和实际节选范围。

本作者取证为原文/正本/Origin定向节选，不冒称所有仓内实现逐行核实。第一阶段全文定义阅读及44ST覆盖来源单独在STRUCTURE.json绑定。已保存#246/#249/#1218/#1219的只读票面证据；后二者只作原文定位，不把其历史报告当现行教义。

最终验收须针对每条已声明语义域给覆盖、互斥、唯一证明，针对原始输入给构造域桥，针对跨轴与事件给合法组合/转换证据；本稿给出需要逐项履行的完整合同，没有以假定证明掩盖尚未完成的部分。
