# #1419 本轮写集与共享调用

工作草稿。以下仅指 2694 起点后的增量，不把已继承的 5cb→2694 适配列为本轮新接口。

| 修改文件（均在 rust/src/） | 内容 | 消费者/许可变化 |
|---|---|---|
| `bin/s_session_v3/source_contributions.rs` | touched 的 Tree 泛型及两处借用查询 | remove_request 的三个调用点仍在 request_phase；两个 touched.insert 继续 owned→native_text；无新增 reserve/WorkScope API。 |
| `bin/s_session_v2/content_producer.rs` | 组标签借用迭代；增加一个对拍场景 | generation→build_group_object→group_payload_value。输入原因由原调用者持有，输出旧 CValue 消费后析构；不新增投影 owner。 |
| `theta_v0/backtest/econ_positive/tests.rs` | 22 构造补 None | 原测试不变。 |
| `theta_v0/backtest/runner_tests.rs` | 4 构造补 None | 原测试不变。 |
| `theta_v0/classifier/cand_predicate.rs` | 3 构造补 None | 原测试不变。 |
| `theta_v0/classifier/signal/cert.rs` | 2 构造补 None | 原测试不变。 |
| `theta_v0/classifier/voice_eat.rs` | 2 构造补 None | 原测试不变。 |
| `theta_v0/classifier/recursive_tower.rs` | donor/有效域及 C 续扫引用 | 注释修改；构造、缓存、续扫行为不改。 |
| `theta_v0/classifier/center.rs` | Zn donor 绑定与 ≥9 排除 | 注释修改；is_zn 不改。 |
| `theta_v0/classifier/descend.rs` | 同类 Move.interval 来源订正 | 注释修改；lo/hi 不改。 |
| `theta_v0/classifier/tests/center_pair_1424.rs` | 导出模块与缺生成器界限 | 注释修改；8 例及夹具字节不改。 |
| `theta_v0/parser/second_fold.rs` | from_wire 改借用核验；旧 normalized 仅测试；新增差分锁 | 外部签名均不变；SecondFoldStore::fold 和 tb02w 的 from_wire 调用继续有效。 |
| `theta_v0/parser/second_fold/wire_match.rs`（新增） | 私有 serde 比对核 | 唯一生产调用是 from_wire→matches；权威字段由原 wire_view 序列化，不另造预算/许可。 |
| `theta_v0/parser/tb02w_tests.rs` | 新增非空恢复语义锁 | NOT_RUN；不宣称 Native 资源锁已成立。 |

本轮新增 `WorkScope`/`reserve_native`/`try_*_native` 调用点数 **0**；枚举以 `DELTA.patch` 的新增行搜索这些名称为界。原 source_contributions 两处 owned 调用没有增加或改变；统一到 TextValue 后其文本 owner 能留在 touched 中。Native map/tree 本身的会计接口仍由 #1488 负责。

尚待 #1488 的 K1 共享变更点：`PlainJson` 按值承接接口、恢复索引持费容量、`SecondFoldStore` 的 owned 恢复入口、`restore_second_cursors` 的消费式传递，以及必要的 generation Native 构造。当前未新增这些接口/调用；精确边界、失败释放、Clone 限制和全调用图见 `K1-OWNER-HANDOFF.md`。不拿本表中的 0 新调用假称 K1 已有账。

根组合 #1393 的 exact rule 核对入口：

- 当前完整结构 rule 位于 `bin/s_structure_session.rs:111`，精确值 `s2-axis-quantifiers+new-bi/2+segment-second/2+center-seed/1+center-pair/1`；本轮未改。
- `tb02b::contract_for_rule` 将该完整规则映射 CURRENT_CONTRACT；`tb02a_facts::CatalogAxisTable::for_rule_revision` 将它映射 CenterPair（22 轴）。当前 S2 的 RULE_REVISION 仅到 segment-second/2，不可当作完整结构域。
- 目录具名映射在 `tb02a_facts.rs:1080`：CC-011→seed、CC-012→feature_sequence、CC-013/014→segment，CC-016→center、CC-017→center_roles、CC-018→unit_core_position、CC-020→center_pair、CC-022→unit_assignment；CC-055/056 保变化/版本，delivered/deferred 不改。
- #1393 的 inclusion 初始化/全包含与 build(previous) 尚未进入本树；最终 rule 字符串、W 事实与上述 C…F 构造、prior、requests、catalog、source_audit 和 writer/recover 必须在根的同一最终源逐项核对。本轮没有声明那份待交增量已映射。

完整文件与字节 hash 见 `WRITESET.json`、`SOURCE-MANIFEST.json`。报告文件列表见 `REPORT-MANIFEST.json`。
