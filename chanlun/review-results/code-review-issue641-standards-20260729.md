# #641 实装收尾 · Standards 轴评审

**审查对象**: `git diff 039cf86974...HEAD`（ticket-641-n3，2 commit：`785732b3ba` / `be69a870db`；
`rust/src/theta_v0/classifier/chain_cert/{mod,tests}.rs`、`classifier/mod.rs`、
`rust/src/bin/{issue550_event_battery,p123_fast_replay}.rs`，+1977/-4）
**统计口径**: 不适用（本报告无统计结论，只作静态标准比对）
**标准源**: 任务书指定的 `.claude/rules/common/` **在本工作面不存在**——`AGENTS.md` 正本声明
「`.claude/rules/`（ECC 注入机制）已归档下线」。故退化为仓内现行标准：`AGENTS.md` +
`docs/agents/{generation-constitution,delivery-discipline,stat-provenance}.md` + Fowler 12 条基线。
CI 无 clippy/fmt gate（`.github/workflows/ci.yml` 只跑 `cargo check --all-targets`），故未跳过格式类。

## (a) 违反文档标准处

**MED-1 悬空对照物指针** — `rust/src/theta_v0/classifier/chain_cert/mod.rs:59`
写「真实窗口读数见 `chanlun/review-results/issue641-n3-chain-impl-20260729.md`」，该文件
**不存在**（207 份归档中零命中）；`rust/src/bin/issue550_event_battery.rs:49` 更只写「见实施报告」
无任何指针。`delivery-discipline.md` §开票门：「验收项若写……对照说法，开票当场就附上能找到 X 的
指针……附不出来……要么改写成不依赖缺失对照物、可以直接判定的条件，要么明确标『对照后补』」。
此处该指针是「复杂度可接受」这一判断的**唯一**依据，属对照型声明。整改：补落该报告，或改写为
「对照后补」。

**LOW-1 名分层提示** — `chain_cert` 自述「纯产出零消费」（`mod.rs:5`）。按
`generation-constitution.md` §1，它有非测试调用者（两个 bin）故判**现役**，不违规；但宪法首要功能
是「让精力不再流向死代码」，1400 行零生产消费对象宜在票上显式登记其消费路线，避免后续被误判。

## (b) 基线臭味

**MED-2 空转测试（违 diff 自设「非真空锁」纪律）** — `p123_fast_replay.rs:4245`
`chain_dump_cadence_advances_on_beat_and_on_last_bar`：`cache = TowerCache::new()` ⟹
`candidate_streams()` 恒空 ⟹ `advance` 恒返回空 Delta。两条断言（sink 为空、`seq == 0`）在
「节拍正确」「永不推进」「每根都推进」三种实现下**全部通过**——测试名承诺的节拍语义无一被夹住。
同 diff 在 `classifier/mod.rs:4455/4469/4478` 等六处反复自设「非真空绿等于没锁」，此处自破。

**MED-3 Duplicated Code + Repeated Switches（Fowler #2/#12）** — `ChainStatus → &str` 的
match 写了两份：`issue550_event_battery.rs:232` `chain_status_name` 与
`p123_fast_replay.rs:631` `chain_dump_line`（`use ...ChainStatus` 后就地 match）。`ChainStatus` 无
`Display`/`as_str`，新增变体须改两处（Shotgun Surgery）。同族：两个 bin 各自重算
alive/falsified/absent 与 adjacent/skip/fact 计数；p123 用了 `certificate.fact_edge_count()`，
battery 却手写 `!edge.is_segment()`——同一量两种拼法，其一类型已提供。整改：计数上提为
`TowerChainCertificate` 方法 + `ChainStatus::as_str()`。

**LOW-2 Data Clump（Fowler #10）** — `TowerChainCertificate` / `ChainProjection` /
`ChainObservation`（`chain_cert/mod.rs:224/255/291`）重复同一组 7 字段
（extends/root_level/leaf_level/nodes/edges/extendable/status）。附带代价：`projection()`
深拷贝 `nodes`+`edges`，而 `apply`（`:607-624`）每路径每次推进调用两次，嵌在已是 O(n²) 的
覆盖边循环内。抽 `ChainPayload` 内嵌可同时消掉三处声明与两次深拷贝。

**LOW-3 Long Function + 命令查询混用** — `issue550_event_battery.rs:150` `print_chain_summary`
约 85 行、14 个可变累加器、三段 `println!`；且函数名为 `print_*` 却在 `:207` 处执行
`book.advance(...)` 改变簿状态做幂等自检。建议拆 `collect_chain_stats()`（纯）+ 打印，
幂等自检单列具名函数。

## 结论

**PASS**（可合入，无阻断项）。整改项：MED-1 / MED-2 / MED-3 建议本票内闭合，LOW-1~3 可挂后续。
未发现违反 `AGENTS.md` 语言约定、`stat-provenance` 标注口径、或 `generation-constitution`
名分/入仓条款的硬伤。
