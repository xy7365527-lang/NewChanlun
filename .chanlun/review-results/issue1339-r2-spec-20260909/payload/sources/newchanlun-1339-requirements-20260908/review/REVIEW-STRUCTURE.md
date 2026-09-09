# #1339 结构需求专项独立复核

结论：44 条结构需求完成专项复核，初审 **6 M / 3 L** 全部修正并复核关闭；当前 **0 H / 0 M / 0 L**。这不是全包审结或运行验收。

本专项属于#1323整图当前#1339需求票的结构分表；完成本专项不表示整图goal、Wayfinder、spec、Sandcastle实施或harvest验收完成。

最终绑定：`STRUCTURE.md` SHA256 `5addccea0f8c8697274a439b46fd18fc7243e2c7dd07acec940e539f62077cdc`；`STRUCTURE.json` SHA256 `d10aa63899bd489b546c5f5746ddd5bebebfc41c23215fe6c898d21f54f55166`。

初审发现绑定：MD `b5bbaa3862fe7a3a1b8ddff87828cd750d08a9e67893bf4cfd272390757b470f`；JSON `30e2c45a083163431e12803030bb47f33bfbff2cf5516d581d650ba572f04004`。下列缺陷和旧行号保留修订轨迹，链接指向当前修正条目。

完整缠论结构观察与完整多重赋格的结构接口；按用户纠正排除独立仓库 K4/拓扑，不恢复 ST-039..042/049，不引入固定重数量或多重非空经营门。

## 初审发现与修正结果

### STR-M01 · M · ST-025 · 已修复

初审问题：**MACD 趋势背驰遗漏 T4 零轴前提**。适用域只要求比较对确定和数据可用，验收要求任一 OR 分支成立按规则处理；当黄白线未回抽零轴而单一 OR 分支成立时，不能据此通过适用的 MACD 趋势背驰判定。全稿没有零轴条件。

初审位置：MD 533–543；JSON `/requirements/24/applicable_domain`、`/requirements/24/criterion_and_boundaries`、`/requirements/24/acceptance_scenarios_and_counterexamples/0`。

来源：[beichi.md:810–810](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/beichi.md:810)；[beichi.md:830–830](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/beichi.md:830)。

修正并复核：[当前 ST-025](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:733) 已落实以下要求：单列适用的 T4 黄白线回抽零轴条件、证据和否例；保留 OR 组合，不把 T4 塞入精确 L 定义。

### STR-M02 · M · ST-024 · 已修复

初审问题：**端点速度不同不推出力度不同**。验收写价格总幅度相等但首末速度不同应给不同 L。端速 (1,3) 与 (2,4) 的净增量同为 2；幅度相等不能补足这个推理。

初审位置：MD 522–522；JSON `/requirements/23/acceptance_scenarios_and_counterexamples/0`。

来源：[beichi.md:338–338](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/beichi.md:338)；[beichi.md:359–363](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/beichi.md:359)。

修正并复核：[当前 ST-024](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:704) 已落实以下要求：改为速度净增量不同则 L 不同；补端速整体平移但净增量相同时 L 相同的反例。

### STR-M03 · M · ST-029 · 已修复

初审问题：**类一例外的级别写反**。小级类一把正本的超大级别例外写反，并漏掉无明显趋势、只有一个中枢的完整限定。

初审位置：MD 619–629；JSON `/requirements/28/applicable_domain`、`/requirements/28/acceptance_scenarios_and_counterexamples`。

来源：[maimai.md:379–379](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/maimai.md:379)。

修正并复核：[当前 ST-029](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:853) 已落实以下要求：恢复周线以上超大级别、无明显趋势、只有一个中枢的类第一类观察域；与正式 Type1 分开，并补相应观察验收。

### STR-M04 · M · ST-030 · 已修复

初审问题：**二类点遗漏首次且紧随关系**。后续回调可能把任意较晚回调误当第二类；正本要求第一次次级别回调，且首个后继走势 i2=i1+1。

初审位置：MD 642–651；JSON `/requirements/29/criterion_and_boundaries`、`/requirements/29/required_backend_output_draft`、`/requirements/29/acceptance_scenarios_and_counterexamples`。

来源：[maimai.md:215–215](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/maimai.md:215)；[maimai.md:227–227](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/maimai.md:227)。

修正并复核：[当前 ST-030](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:883) 已落实以下要求：明写首次且紧随并输出关系依据；补首个回调不成立而以后回调成立也不得回填二类的反例；保留可破一类极值条款。

### STR-M05 · M · ST-028 · 已修复

初审问题：**六谓词关系表漏已定互斥**。正文只收录二三类可重合，却未收录第一与第二、第一与第三类不可重合及卖侧对偶。谓词独立全定义不等于任意组合可达。

初审位置：MD 600–609；JSON `/requirements/27/criterion_and_boundaries`、`/requirements/27/acceptance_scenarios_and_counterexamples`。

来源：[maimai.md:174–180](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/maimai.md:174)。

修正并复核：[当前 ST-028](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:824) 已落实以下要求：在同点、同级别、同适用结构域明确 1B/2B 与 1B/3B 互斥及卖侧对偶，并补反例；保留二三类重合与跨级不误去重。

### STR-M06 · M · ST-009 · 已修复

初审问题：**同级读法的已定行为被概括成职责不同**。当前仅写构造和读法对延伸、盘整连接有不同职责，验收也仅保证多读法不改塔。fenjie 的正本指针明确给出条件 seed、恒三格、禁延伸、允许盘整加盘整及唯一性；缺少这些语义的可判定对照验收。

初审位置：MD 198–206；JSON `/requirements/8/criterion_and_boundaries`、`/requirements/8/required_backend_output_draft`、`/requirements/8/acceptance_scenarios_and_counterexamples`。

来源：[fenjie.md:17–18](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/fenjie.md:17)；[fenjie.md:25–29](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/fenjie.md:25)；[0011-operation-decomposition-layer.md:29–35](/Users/silencehan/Projects/NewChanlun/docs/adr/0011-operation-decomposition-layer.md:29)。

修正并复核：[当前 ST-009](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:271) 已落实以下要求：明写读法口径与构造口径的行为边界，补六段延伸在操作读法下可读成两个盘整、构造身份不变的验收及错误回流反例；不预选旧模块、部署或存储形状。

### STR-L01 · L · ST-024 · 已修复

初审问题：**四个端点误写成四种端点形态**。来源的四是 b初、b末、c初、c末四个端点，未建立四种形态分类。

初审位置：MD 514–514；JSON `/requirements/23/criterion_and_boundaries`。

来源：[beichi.md:344–350](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/beichi.md:344)。

修正并复核：[当前 ST-024](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:704) 已落实以下要求：改为四个形态学确定的端点及各自身份、锚定依据。

### STR-L02 · L · ST-024 · 已修复

初审问题：**力度端点未定项收录不全**。仅写发展中笔端点值，未保留段初取第一笔还是零，以及段末取当下笔还是最后完成笔的未定口径。

初审位置：MD 525–525；JSON `/requirements/23/undecided`。

来源：[beichi.md:463–466](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/beichi.md:463)。

修正并复核：[当前 ST-024](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:704) 已落实以下要求：逐项保留首末速度的未定值域，避免由输出字段暗选第一笔或已完成笔。

### STR-L03 · L · ST-046 · 已修复

初审问题：**具名共振与更宽同步观察名分未分开**。来源具名共振明确说不同级别同时第一类点；本条扩展到六谓词任意同步和跨塔对照，却统称同步共振。扩展观察本身可保留，来源不足以把它全称为已定教义。

初审位置：MD 896–896；JSON `/requirements/41/object_and_axis`、`/requirements/41/criterion_and_boundaries`。

来源：[maimai.md:283–283](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/maimai.md:283)；[zoushi.md:302–309](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/zoushi.md:302)。

修正并复核：[当前 ST-046](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1238) 已落实以下要求：将具名多级别共振限定为来源的第一类点范围；任意六谓词同步与跨塔对照标为本票更宽观察草案，仍保持同知时点规则。

## 逐条覆盖与局限

| 条目 | 结论与局限 |
|---|---|
| [ST-001](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:52) | 闭包含、方向折叠与初始无方向未定分开；不把对称包含不传递错当单向包含不传递。 |
| [ST-002](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:80) | 组首锚、组号与组内极值根分开；重建对拍为行为要求，不认领已实现。 |
| [ST-003](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:108) | 四分类限良构三根 merged K；严格双高低与右邻获知时间准确。 |
| [ST-004](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:135) | 新笔双坐标两门齐全；生产新笔为现行选择，旧笔不兜底。 |
| [ST-005](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:162) | 同性去重、末笔延伸、缺映射不可换档明确；历史前缀稳定不是任意行情修订保证。 |
| [ST-006](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:189) | 笔破坏必要非充分与线段终结分开；起始三笔与至少三笔保持。 |
| [ST-007](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:216) | 方向性特征序列包含与分型识别有据；未改为一般 K 包含。 |
| [ST-008](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:244) | 独立第二特征序列及同向任意分型有据；未将缺口回补单独作为终结。 |
| [ST-009](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:271) | 通过修订复核。条件seed、恒三格、禁延伸、并列盘整、固定读法唯一性及六段对照验收均已补齐；不要求旧部署形状。 |
| [ST-010](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:303) | 结构判据、诊断与准入分开；八列历史收口不证明运行。 |
| [ST-011](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:331) | 连续三次级别单元、严格核心与连接段无遗漏；类中枢名分未混。 |
| [ST-012](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:360) | 冻结核心与动态外缘 G/D 分开；全观察要求已提供消费者名分。 |
| [ST-013](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:388) | 延伸与首次回试破坏分开；触及边界依闭区间口径。 |
| [ST-014](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:416) | 扩展、新生与趋势形成分开；中枢核心和外缘职责不同。 |
| [ST-015](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:444) | 九段为三加六，升级重切不能继承旧核心；无波动计算保留未定。 |
| [ST-016](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:475) | 续扫锚及最后中枢到两侧连接来源可追；历史待落地不作本轮状态。 |
| [ST-017](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:503) | 盘整与趋势的同级相对计数及同向外缘条件准确。 |
| [ST-018](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:531) | 盘整无教义方向与技术 break_direction 分开，例外名分明确。 |
| [ST-019](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:559) | 形成、构件完成与走势转折分层；后裁语义覆盖旧完成段表述。 |
| [ST-020](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:588) | 级别身份随塔与基底，不把相同整数或时间周期当同一级别。 |
| [ST-021](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:618) | 统一判据、构造递归与读法非递归分开；同输入语义对拍要求保留。 |
| [ST-022](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:647) | PH 仅名分边界，明说不新增任务；没有恢复 K4/拓扑独立任务。 |
| [ST-023](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:674) | 候选集合与正式比较对的来源张力公开；不能称已证明两套范围兼容。 |
| [ST-024](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:704) | 通过修订复核。端速差反例、四个端点与首末取值未定域均已核正；只是定义代入与需求审阅，没有程序验收。 |
| [ST-025](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:733) | 通过修订复核。趋势T4与后续OR分层，前提失败否例和输出证据已补；不把T4加入正式L。 MD主段完整输出已与JSON一致。 |
| [ST-026](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:764) | Div/NE/Turn 与未来验证隔离基本正确；建议保留完整 NE:Div⇒¬Ext 命题写法。 |
| [ST-027](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:793) | 必要非充分和下钻失败补集区别准确；验收未逐写末次级中枢买卖方向对偶。 |
| [ST-028](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:824) | 通过修订复核。同点同级一二/一三互斥、二三可重合及卖侧对偶均已纳入。 |
| [ST-029](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:853) | 通过修订复核。超大级别且无明显趋势、一个中枢的类一观察域与正式Type1分开。 |
| [ST-030](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:883) | 通过修订复核。首次紧随后继i2=i1+1及禁止跳到后续回调回填二类的否例已补，越极值不否决仍保留。 |
| [ST-031](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:913) | 首对失败不可回填、冻结核心、无幅度阈值有据；等号冲突保留，未执行 Lean。 |
| [ST-032](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:943) | 确认映射与结构/操作分离准确；修订协议明示本票草案。 |
| [ST-033](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:972) | 向下定位与向上构造、构造顺序与求值顺序分开，未改成竞争方向。 |
| [ST-034](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1000) | raw/merged 显式坐标、闭包含与真实父子来源有据；跨级价格挂法仍未裁。 |
| [ST-035](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1030) | 元素谓词、集合非空、条件化选择和独立力度门区分准确；最近同向冲突保持。 |
| [ST-036](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1059) | 链身份、连续性、N/A与三类深度分开，布尔证书不越权；Confirm内外保留为未定。 已补正本1106的N/A证书处理未裁边界，显示N/A不暗选true/false。 |
| [ST-037](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1090) | 成本与塔底先到者停止；阶段进阶需不确定性及到底证据；未认领历史读数。 |
| [ST-038](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1119) | EmptySubs/NoAlign/DivFalse 与知识包装状态独立；未知不扩宽准入。 |
| [ST-043](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1149) | 生成态与#659降级头有效，未从无独立风控模块口号预选架构；完整经营侧由FG承接。 |
| [ST-044](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1177) | 全对象关系与变化契约明确草案身份，未把字段直接冻结为API。 |
| [ST-045](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1209) | 强弱/中继分型保留为描述，未偷偷设阈值或改变基本准入。 |
| [ST-046](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1238) | 通过修订复核。具名共振限不同级别同时第一类点，其他六谓词并列明确草案名分，删除跨塔扩张。 |
| [ST-047](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1266) | 通过补充复核。盘背回拉、后继三类继续原向与新次级点的同知时点观察已经补齐；未冒称未来事实已发生。 |
| [ST-048](/tmp/newchanlun-1339-requirements-20260908/author/parts/STRUCTURE.md:1300) | 截断扫描与完整语义分开，50或tail7不是教义常数；资源停止与自然停止分别可见。 |

12 条（ST-023–032、046–047）由只读子复核 structure_predicate_check 独立对照来源后整合；其他 32 条由本独评核查。九项修订及两个覆盖补充由本独评重新对照当前作者字节与来源确认。

## 机械检查与证据边界

44 个活跃 ID 与 44 段 Markdown 对应；对象、域、正文、输出、前端、验收、未定和证据状态字段在 MD/JSON 逐项一致，重复补充段已移除。112 组条目引用范围均在当前源文件内；18 份读取清单的 SHA256、字节数和行数全部匹配。范围内 15 份定义有映射，另外 3 份明确为已排除的历史材料。

- 本报告只审 STRUCTURE 两份文件，不是全包审结；完整经营与最新 RF-01 由 FG/总稿承接。
- 作者44条逐项审查；引用按承重段落核验。未声称独评人完整重读18份定义的7432行。
- 18份来源SHA、bytes、line_count与当前工作区一致，说明文件绑定可靠，不证明作者实际阅读或语义完整性。
- 风控和交易体系作者全读声明来自赋格作者协作清单；本独评只核其状态头及ST-043引用段，不取代完整FG独评。
- 未回查博文作者归属，未读取旧候选方案作答案，未运行引擎/Lean/前端/回放/经济或交易验收。
- 实际只读了ADR0011语义职责以核fenjie指针，未把旧模块形状或部署作为新架构要求。

## 覆盖补充与保留边界

- ST-047 已补盘背回拉、后继三类继续原向、新次级点与逆转持续的关系观察及当时可知验收，未把来源保证性话语冒充已经发生的事实。
- ST-036 已明写正本 qujiantao:1106 的 N/A 证书处理仍未裁，显示不适用不暗选 true/false。未裁语义仍属于后续决策，不能由需求字段替代裁定。

没有运行引擎、Lean、传输、前端、回放、经济或交易验收，也未修改代码或执行 Git/GitHub 写入。文件数、字段齐全和哈希一致只能证明这一版本的文档一致性。
