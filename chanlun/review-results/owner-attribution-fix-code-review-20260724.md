# #218 两轴 code-review 结论（Standards + Spec #219 逐条符合性）

- 日期：2026-07-24 ｜ 票据：issue #218（spec #219 v2）｜ 评审面：本票改动集（见 impl §1 表，
  他线未提交改动不属评审面）｜ 方法：自审逐条对账 + codex（gpt-5.5）独立只读评审
- 输入证据：`cargo test --lib`（1802 passed / 1 既线失败）、两接缝 scoped 测试（8/8、84/84）、
  `cargo check --bins`（0 错误）、`cargo clippy --lib`（本票改动面零新增警告）、wf7 重放
  校验 25 项断言退出码 0、上游红线/不变量/翻转清单落档（readings §1-§5）

## A. Standards 轴（本仓纪律符合性）

| 纪律 | 结论 | 证据 |
|---|---|---|
| no-patch / 090（声明=能力） | ✓ | 核化触及处 doc 全部如实改写（impl §5 清单 + level_view.rs `b_center_start`「判同基准」旧表述随判同机制换锚如实改写为「查找键」）；未验证项逐条如实标（readings §6：p3fold/wf8 未跑、scan_pts 无对照面、Exact 臂仅单测、B 查无防御臂、研究 bin 降级）；无声明膨胀（三类点带判同事实、二类 0/40 如实报负向结论） |
| v3 硬禁令（无概率推断；极值价精确等值无容差） | ✓ | 两族判同全部整数/tick 精确等值（(zd,zg) 带等、(极值价,组锚) 锚等），零容差零概率；计数全量无抽样 |
| #206 身份教义（身份=同点递归，锚=（极值价,合并组锚），禁序号判同） | ✓ | 序号（start_index）在两族判同中只当查找键（一/三类 B 查出）或完全不出现（二类锚判同）；身份判据 = 带内容 / 两元锚 |
| 禁第二查法 / 禁 fork 判定核 | ✓ | 判定单一来源 `terminal_bits_in_book_core`；oracle 单一构造子 `projection::anchor_resolver`（与本文件 :143-145 同一查法）；#214 私有核+双薄入口结构不动（共核单次遍历产判定+探针）；build 循环只换 ctx 构造；生产薄包装/测量入口/研究 bin 全委托同一核；bin 侧 `bin_anchor_ctx()` 是同一核的参照包注入非第二查法（事件锚缺失如实降级并注释） |
| TDD / 最小接缝 | ✓ | 接缝 = spec 钉案两处（构造器 `extract_second_signals`、主接缝 `build_nest_certificate_index`）；测试先行（面 A 3 新 RED→GREEN、面 B i218 9 新 RED→GREEN）；只测外部行为（合成账本/事件/oracle 进出，断言判定与计数） |
| bit-exact / GOLDEN 受控 | ✓ | 判定集合按裁定改变 = 故意面；digest guard 既线失败在案 + 面 A 二次漂移登记（readings §5，per-case 对拍全绿证内容不变）；无隐藏 Debug 掩盖 |
| 最小改动 / 不碰他线 | ✓ | 改动面 = impl §1 表内文件 + 产物；level_view.rs/incremental.rs/l3_*/wverify_run.rs/config.rs/exit.rs/overlay_state.rs/fractal.rs/inclusion.rs/rmove_compose.rs/descend.rs/types.rs 等他线文件零触碰（level_view.rs 仅 doc 一处随判同机制换锚如实改写，行为零改动） |
| 临时诊断码 | ✓ 已清零 | NEST218_DEBUG 探针（核内 band_cmp/t2_cmp 行 + gate 塔溯源 flip 探针 + debug_tower 字段）全部删除，grep 零残留；终版重放 stderr 零诊断行（`grep -c NEST218 = 0`） |

## B. Spec #219 逐条轴（v2 符合性）

| spec 条目 | 结论 | 证据 |
|---|---|---|
| ID-1 面 A（二类载体 = 该走势一类点身份锚；识别层零改动；零新入参；不加字段优先） | ✓ | `make_second_point` 载 `OwnerRef::Type1Anchor(index_of(m1))`（m1 = i1 第一类离开走势终点 = 该走势终点极值点）；c1/i1/i2/背驰/second_point/bits 逐字不动（构造器接缝 8 测试锁定 + 拒收测试零改动）；坐标经既有 `index_of` 闭包零新入参；**未加字段**——既有 `center` 字段类型适配为锚可比形态 `Option<OwnerRef>`（spec 首选路径）；消费面逐点复核落票（impl §4 表）；一/三类载体零改动 |
| ID-2 面 B（两族精确等值；oracle 单源进核；事件侧零改动；保留 owner_center_start；不动清单） | ✓ | 中枢判同 `point.center.(zd,zg) == B.(zd,zg)`（B 由 b_center_start 当查找键在账本 centers 查出，同一快照内一致）；点判同 `anchor_at(一类点坐标) == (event.extreme_price, event.group_anchor)`（oracle = T1 供给线 projection.rs:143-145 同款单查，仅供二类；`build_nest_certificate_index` 加 oracle 入参；事件侧直接用 Ext 已带两元锚经 gate `anchor_by_id` 透传——`NestCandidateEvent`/Ext 未加字段、tower_events.jsonl 逐字节不变）；锚不可解 = 诚实判负 + 子计数单列；`owner_center_start` 保留（归因快照，Center 载体 start_index / Type1Anchor 如实 None，消费侧零读者复核在案）；`Exact` 臂/窗口/方向/最早性/点类分域/`turn_source` 逐字保留（i218 不变量测试 + F 组断言） |
| ID-3 正交性（面 A digest 面 / 面 B 判定及下游） | ✓ | 面 A 影响面 = 信号流 digest（GOLDEN 登记归因面 A）；面 B 影响面 = 背书判定与下游（NEST_GATE/chain_dump/trades 对账）；叠加 = 裁定全量 |
| ID-4 装置同步（探针换锚口径；锚不可解重定；桶名改；平账保留） | ✓ | 探针与生产同一次遍历副产品（共核）；`owner_anchor_missing_pts` 语义重定 = 锚不可解（含事件侧缺失）；`owner_real_neq_pts` = 两侧可判且实不等；桶名「owner 锚不等」（schema 演进随票登记）；完备性平账断言保留（B 组全过）；按级 × kind 分解保留（C 组全过） |
| ID-5 验收口径（上游红线 + 不变量 + delta 落账 + 反向分量单列 + 逐事件清单） | ✓ 逐项落账 | 上游红线：tower_events MD5 ✓、base_events ✓、候选流行键集 ✓（E 组）；不变量：异向 13/窗口外 1/Pan 72/768/c_i total 8/40/124 逐项一致（F 组）；delta：owner 桶 8→10、success 16→14、assembled 88→86 与预期方向相反，**机制解释闭合**（readings §4：5 件二类真锚收紧 − 3 件一/三类带放宽 = −2）；反向分量（success→owner 5 件）逐件单列归因（readings §3.1）；逐事件双向翻转清单按事件身份键落账（readings §3，塔溯源探针产生、聚合交叉验证闭合） |
| ID-6 GOLDEN 受控流程 | ✓ | 事前基线复用 #214 留档；诚实重算（全量测试 + wf7 重放实际值）；逐项归因（面 A → digest 二次漂移；面 B → 下游行族/chain_dump/trades 对账，零变化项记零变化）；登记落档（readings §5）；与 #110 既线失败切分（勿修勿归因 + 二次漂移归面 A，per-case 对拍全绿证内容不变） |
| ID-7 重放与产物 | ✓（按范围裁定） | wf7 必跑 ✓（校验 25 项退出码 0）；p3fold/wf8 **未跑**（范围裁定 2026-07-24，如实标）；产物三件套落 `chanlun/review-results/`（20260724 戳）：impl + readings + verify 脚本/输出 |
| Testing Decisions（接缝清单 1/2/3） | ✓ | 主接缝 9 新测试（两族判同行为、序号漂移双向用例、四桶平账、点级计数、不变量、构造先例复用 book2/pt/ptc/typed_event/trend_ev 族）；构造器接缝 3 新测试（载体锚 == 一类点锚 + 识别行为不变断言）；产物校验接缝 = verify 脚本 25 断言 |
| 既有测试处置 | ✓ | owner 相关机械改写逐个归因（impl §3 清单）；非 owner 测试（Pan/窗口/方向/Exact）零改动通过（签名机械适配不计语义改动）；无红旗 |
| Out of Scope 全部条目 | ✓ 遵守 | 确认层零改动；一/三类载体零改动；路线 A/值桥层未触；`turn_source` 不动；`level_origin` 未触（归 #213）；`Exact` 臂不动；生产路径非授权面（base 事件供给/tower 扫描/候选流/Pan 域/`n_delta`/装配几何/止损判据）一个 bit 不动（红线 + 不变量 + trades 一致佐证）；descend/XZD 未引；`NestCandidateEvent`/Ext 未加字段；map Destination 线未设（只供实测 delta）；既线失败勿修勿归因；逐事件 dump jsonl 未引（flip 清单为 stderr 临时探针，已删，方法留档） |

## C. 评审发现（LOW 级，均不挡验收）

0. **codex 独立评审（gpt-5.5，2026-07-24）采纳项 1 件（HIGH→已修复）**：初评 FAIL——
   「Type1Anchor 臂不检查 `b_center_start`：None 时锚相等仍可判等，违反『B 身份缺失 ⟹
   无合法点』的合一诚实判负纪律」。复核：真实路径全不可达（生产 `b_center_start` 为
   `usize` 恒 Some；研究 bin 事件锚恒 (None,None) ⟹ 二类本已锚不可解），但 doc 声称的
   合一 fail-closed 语义在该合成边角确实被破（doc/行为不一致）。已按采纳修复：
   Type1Anchor 臂加 `b_center_start.is_none() ⟹ Unresolvable`（与中枢判同族同一纪律，
   生产行为中性可证——恒 Some 不触发）；补回归测试
   `i218_b_identity_missing_fails_closed_both_families`（两族逐一锁 fail-closed +
   锚不可解子计数）。修复后：nest 84/84、全量 1802 passed / 1 既线失败、wf7 校验
   25 项断言退出码 0（产物读数不变）。

1. **spec v2 措辞 vs 几何核实**：ID-1「i1 第一类离开走势的**起点**」与括注「该走势终点极值点」
   在字面有张力；按括注 + 事件侧 seg_c.1（离开段终点 = 趋势终点极值）+ `index_of`（end_index
   语义）+ 识别测试夹具（m1 向下离开，终点 = B1）三重核实，实装取 **m1 终点**（若取起点则
   与事件锚永不匹配，教义不成立）。建议 spec 修订时把「起点」勘为「终点」。（LOW，已在
   readings/impl 留痕，归编排者确认）
2. **遗留 stale doc（#214 已登记、本票不顺手清）**：`assemble_typed_certificate` doc 的
   「Trend = 破 B 一类 ∧ owner=B」为 P1 重议前旧表述；p125 bin 的 P125_RULE 串同为历史
   审计元数据。沿 #214 先例登记留归后续（LOW）。
3. **研究 bin ×8 锚供给降级**：未接事件锚账本 ⟹ 二类判同锚不可解诚实判负（一/三类带判同
   全功能）。归档研究工具语义降级已在各 bin 注释 + impl §4 表末行注册（LOW）。
4. **mod.rs 零改动说明**：任务提示曾列「mod.rs 一处实参」为改动面；spec ID-1「零新入参」
   成立后该处无需改动（c1 实参仍供识别层，doc 已核无冲突）。按 spec 钉案执行并留痕（LOW）。

## D. 结论

两轴评审**通过**（codex 独立评审 1 项 HIGH 已采纳修复并回归锁定）：Standards 全项符合；
Spec #219 v2 逐条符合（ID-7 按范围裁定 wf7 单窗）。
wf7 实测 delta 与 spec ID-5 预期方向相反一事**不是实现缺陷**——机制归因闭合（readings §4），
属路线 B 在该窗的真实测量结果，作为 map #126 Destination「显著上升 ✓ 或如实判定」的
如实判定分支裁定输入上报。
