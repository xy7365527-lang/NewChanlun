# 第六轮：现有入口不能直接补齐F₂，连接单元还需正确的层级证书

日期：2026-10-03。绑定[订单基底候选构造与合法性证据](https://github.com/xy7365527-lang/NewChanlun/issues/1467)。本轮承接Stage5，没有改变R_W-DC-v0、研究目标或验收。

本轮在已定位的四条路径中，没有找到可直接替换Stage5适配并同时满足九段重切、成员覆盖和完成语义的实现。这个结论仅覆盖下表所列入口和冻结main，不是“全仓或所有未合入分支都不存在实现”。新发现的关键义务是：连接片段必须有正确的递归层级与完成依据；仅把它补进几何数组，仍不够。

## 一、复用范围与源码证据

已核研究HEAD `6fba9f8adbbf693dd0538d990dc27f20930c8468`，远端main仍为 `6f99b36c64e0560febdb08e5d1f8e781d2af9299`。图工具先行，返回的过期位置/无效过滤只作定位线索，结论回到实际工作树文件。

| 路径 | 当前查到的事实 | 可复用边界 |
|---|---|---|
| theta分类管线 | `classifier/pipeline.rs:1083`调用 `compose_level_resume`，`:1538`调用 `project_to_units_resume`；`recursive_tower.rs`全量/resume复用同一扫描器 | 更换为resume并未换掉Stage5反例中的重切与投影规则；缓存、frontier和来源管理可另作适配研究 |
| 正式S结构会话 | `s_structure_session.rs:2121–2160`建立ParseLayerIncr/包含结果，再调用 `classify_local_shape_sliding`；当前对象发布主线不提供本研究所需的完整F₂替代 | 修订、事件身份、first_known、撤回和恢复载体值得复用，但“正式会话”名称不证明递归语义已接入 |
| Python RecursiveLevelEngine/Stack | 过滤settled后调用 `zhongshu_from_components`；该函数延伸到不重叠为止，无九段重切。Stack还有默认最大层数6及PH停止。所依赖 `a_level_protocol.py`、`a_move_v1.py`明示 #1286 历史对照、退出生产准入 | 本轮只读核查，不安装依赖或修复历史路径，不作生产替代 |
| Rust standalone T | `find_centers → segment_into_trends → judge_divergence → encapsulate`；只封装completed走势，`iterate`用next_units驱动下一层 | 有独立完成过滤，但本轮实际9单元输入没有九段重切；不是合格的直接替换 |

现有[中枢seed/成员实施票](https://github.com/xy7365527-lang/NewChanlun/issues/1419)、[九段升级/自然递归实施票](https://github.com/xy7365527-lang/NewChanlun/issues/1429)、[扫描与恢复实施票](https://github.com/xy7365527-lang/NewChanlun/issues/1420)仍OPEN，所查评论列表为空，票体是待验收行为合同。不能据此断言别处没有在制实现，但也不能把规划或勾选框当成当前main上的可用交付。研究没有改这些实施票的状态、依赖或验收。

Rust T模块有自身调用者与历史名分声明，本轮不因它不是所选theta路径就把整个模块判为死代码。这里只检验其是否满足本研究的已裁契约。

## 二、实际T入口的两个限定执行

新增 `alternative_f2_probe.rs`直接调用实际crate的 `recursive_t::apply_t` 和 `iterate`，不复制它们的算法。沿用Stage5整数观察列，经同一DC-v0构造及独立参考核对后适配 `recursive_t::Unit`。本例整数均可由f64精确表示；这不验证一般价格精度。area字段为0，仅作该接口要求的辅助载荷，不据此改变力度或资金规则。

| 输入 | 单级结果 | 全迭代结果 |
|---|---|---|
| G1：11观察、9个完成DC单元 | 1个中枢，成员0–8全部被吸收；核心[10027,10033]、外缘[10025,10041]；1个未完成Consolidation，next_units为空 | 1层，随后自然停止 |
| G2：9观察、7个完成DC单元 | 2个中枢，成员分别0–2、4–6；前Consolidation标completed且跨度0→4，后Consolidation未完成；只输出1个上级单元 | 1层，因上级不足3项停止 |

G1在真实单级入口和全迭代入口都没有重切成3个子窗。源码 `recursive_t/center.rs:85–138`确实只做三段seed与持续延伸，没有九段分支；`operator.rs:95–131`后续分组/确认/封装并未补上该行为。这否定本入口可直接解决Stage5重切义务的假设。

G2展示不同的成员处理：连接源区间3→4被纳入前一个走势跨度。它与Stage5“每中枢一上级单元”的窗口包装不同。不能仅凭两条路径最后都没有上级中枢就宣称它们语义相同，也不能将不同输出当作选更宽口径的理由。T的completed判定是否覆盖原义全部条件未在本轮独立证明。

## 三、上一轮的几何连接诊断不能直接变成修补方案

Stage5的 `center_from_segments(left, connector, right)`只检验价位、方向和时间几何。实际类型中，left/right包装对象的 `ElementId.level=1`，被插入的连接单元来自 `level=0`。`UnitRange`不带层级，单看这个函数检查不出混级。

因此“把缺失的一条DC单元插回数组”不能被当作已经构造了同级次级走势，也不能只修改level字段使它过关。现有 `Formal.RecursiveConstruction.WellFormed`对Compose要求至少3个下级成员、正确的逐级关系、非空且来自成员的中枢，以及下级递归良构。

本轮 `ConnectorLevelBoundary.lean`直接导入现有定义并证明：

```lean
∀ u centers level, ¬ WellFormed (Move.compose [u] centers level)
```

实际定理名为 `singleton_is_not_wellformed_upper`。证明只展开现有WellFormed并读取成员数量要求，没有增加前提或改定义；Lean4.31通过，依赖标准 `propext/Quot.sound`，没有sorryAx。初次编译需要显式展开递归定义才能取合取项，修复仅改证明体。

这否定单成员直接升格的朴素修补，不否定所有连接走势构造。合法连接可能需要不同的走势划分、真实内部构成及完成证书；若改F₁本身，则必须另立候选版本并重新满足原义务，不能沿用DC-v0证明的名字。

## 四、对后续研究的约束

现在要研究的承重对象应明确为“已完成的次级走势及其构成证书”。中枢窗口、价格包络、连接片段、完整走势是不同角色。每中枢包装一个对象再把全部对象上送，不足以证明F₂已经在同一个语义域内闭合。

下一步沿已裁正本建立可独立检查的同级走势组装关系：成员分配与层级、核心/外缘来源、连接的内部构成、活动尾部以及完成依据一起验收。现有会话持久载体和frontier可复用，但不能把它们的存在换算成结构定理已证。只修Stage5的数值核心同样不能补齐这些义务。

这条上层义务对投影路线和直接事件路线都适用。R_D-v2的限定反例保留；R_W-DC-v0的存在唯一性/因果/几何定理也保留。没有以替代算法失败来否定订单递归整体，更没有开始真实市场收益选优。

## 五、复现和边界

实际exit=0的新增检查：

```sh
cargo run --offline --locked --manifest-path .chanlun/review-results/issue1467-f1-proof/stage6-probe/Cargo.toml --target-dir /tmp/nc1467-cargo-target --bin order-base-alternative-f2
cd formal
lake build Formal.RecursiveConstruction
lake env lean ../.chanlun/review-results/issue1467-f1-proof/ConnectorLevelBoundary.lean
```

原始输出为 `alternative-f2-stdout.txt` 和 `connector-level-boundary-stdout.txt`；源码、相关合同与证据hash见 `source-sha256-stage6.json`。没有重跑Stage4一般定理或Stage5已有核的同一反例；新执行针对另一套实际入口及新的层级边界。Rust格式、旧研究锁与差异检查通过。

未审计全部未合入工作树、全部历史入口或所有正式经营消费者；未验证这个研究适配的经济含义、实盘执行、费用、吞吐和尾延迟。没有生产源码或formal源修改，无需重生fixture。用户另行配置的未跟踪skills/研究工具/工作流不在本轮提交范围。独立跨代理审查仍未完成。候选票及goal保持活动。
