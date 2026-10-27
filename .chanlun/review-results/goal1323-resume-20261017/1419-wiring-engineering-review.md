# 代码审查报告：#1419 新结构域与生产接线

日期：2026-10-17。工作树：`.sandcastle/worktrees/resume-1419`，恢复基线 `018c6542e5`。只读审查；本报告是绑定 #1419 的工作草稿。未运行 cargo/rustc、服务、外盘或远端；没有修改生产源码。

## 概念层质询结果

本次限定接线范围通过静态质询：新域独立声明，不回填旧域；同次 parser/store、已核活动 prior 与唯一 CC-054 请求聚合的方向符合现行合同。中枢定义核对 `.chanlun/definitions/zhongshu.md` 的 #1478 修订及已签 SPEC-COVERAGE-INPUT.md:228/230；本工位不重复并行工位对逐请求四轴、具体 C/D/E/F 判据的专项裁定，也不把它们的未决项判为通过。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | 未发现 |
| HIGH | 1 | HIGH-1 未修；HIGH-2 已静态复读消除 |
| MEDIUM | 1 | 确定的测试域不一致 |
| LOW | 0 | 未报风格项 |

结论：**WARNING；当前不能作为可发布或验收通过的候选。** HIGH-1 是真实生产链拒绝，不能用生成根闭包成功代替发布验证。HIGH-2 经本次即时返修已消除静态缺陷，仍未运行。另有已存在于 HEAD 的编译阻断，详见下文。

### HIGH-1：新域生成 C/D/E/F 后，发布前来源审计仍只接受 A/B

`source_audit.rs:1700-1709` 的 typed kind 白名单只有 CC004/005/007/054 与 `tb02b::KINDS`。新域 `generation.rs:1034-1075` 无条件纳入 C/D/E/F，至少三种 CC056 版本根即使没有完整段/中心也会产生。首代真实发布在 `prepared.rs:906` 调 `source_audit::audit_generated`，经 `source_audit.rs:2231` 的 `audit_object_record` 到该白名单，必以“未声明typed事实种类”拒绝。Native 增量路径 `prepared.rs:623` → audit_native_source_cut → `source_contributions.rs:1023` 调同一 `audit_object_record_with_slots`，也不会绕开。

最小反例：新 `init_v3` 库接纳短 OHLC 前缀，生成 `CC-056.segment_version`，进入完整来源审计；该 kind 不在现白名单。新增的 generation 单元锁只调用 `verify_new_closure`，不调用发布前来源审计，因此不能捕获该失败。

修复依据：C/D 为 `tb02c.rs:59` 的 7 个 KINDS，头为 `s-segment/3` / `CONSTRUCTION_RULE` / `seed_closed_strict_second`；E 为 `tb02e.rs:18` 的 7 个 KINDS，头为 `s-center/1` / `center-seed/1` / `strict_seed_zn_outer_c_resume`；F 为 `tb02f.rs:15` 的 5 个 KINDS，头为 `s-center-pair/1` / `center-pair/1` / `zn_outer_five_leaf_pending_ge9`，view_role 均 main。精确允许当前结构规则域及这些 kind，核 payload.slot 与规范 fact_key，并保持当前通用 source_references → raw 原件、source_coords 相等、obj3 身份校验。不要以 starts_with 放宽所有未来对象，也不要重跑 parser 取代原件审计。

旧 `source_audit_oracle.rs` 未包含 C/D/E/F kind；未找到可直接复用的旧 C/D/E/F 独立来源审计。旧 tb02c/e/f builders 是构造器而不是审计核。C/D/E/F 的内部实体/对象子边、版本见证仍需依据其 schema 核认，不能只把白名单扩开就声称完整源证明。

更正：早期即时消息猜测 CC055 会误套 B 头，已撤回。实际结构关系种类是独立的 `segment_relation`、`center_relation`、`pair_relation` 等，当前同样在白名单外。

### HIGH-2：新结构分类请求与既有二元槽位审计不兼容

**本批次最终复读：静态缺陷已消除。** `content_producer.rs` SHA256 `57c02cf2c4a8810e4031048533826f5174f80c88af97fe6154718bfd4ee0eacd` 的 explicit_request 已在传入同一 NodeStore 写入并规范渲染 key，不再使用私有 scratch；`content_structure.rs` 最终读取 SHA256 `1b3001a12ffd822700a14475b6b90650707242301cd329ca50deb53f2da9bda3` 保持规范二元键。以下为缺陷到返修的证据轨迹，不代表仍开放；四轴语义与运行验证仍属于专项/远端门。

**返修复读：二元封装与目录归属已修；当前仍有 key 外部值边渲染阻断。** 新 `content_structure.rs` SHA256 `67a2c564130011fddcdebad025d4ab6ac22520eefcb2a33f57c40a29c906d689` 已改为 `[ [kind, Edge(slot)], [cc, axis, version, path, obj3] ]`。新 `tb02a_facts.rs` SHA256 `f33aa9e513191f689b18539b06faf97b159484bbe9f8d88aefef5f8a92d4c5b4` 仅在 CenterPair 新域和明确请求版本下按第二元 cc 归轴，已静态核吻合，旧解析不变。新增分轴测试源码未运行。

但 `content_producer::explicit_request` 仍调用 `canonical_text(&key)`；该函数（content_producer.rs:2983）新建私有 MemoryNodes 再写/渲染，外部 store 的 slot 节点未在此 scratch 中，故真实请求键含 Edge 时会 MissingNode，未能生成请求。已直接通知 bridge 作者：应使用传入真实 store 规范渲染含边 key，或使用真正自足的小值；不得把该故障误报为二元协议仍不通过。以下保留初次发现证据及已修说明。

审查中的 bridge 快照 `content_structure.rs` SHA256 `bbdb94edd416bf654362bbe2885f251843c7eeec059a1100995d80cfaf08da86`，`classification_request` 使用五元键 `[structure-classification/1, cc, axis, subject, path]`。

`source_pending.rs:615` 在区分 CC004/非 CC004 之前要求规范二元数组，否则报“知识请求键不是规范二元槽位”。所以一旦新域有实际分类请求，修复 HIGH-1 后仍会在该审计失败。此问题已通知作者；作者正在改规范二元封装，本报告只绑定上述快照，不能冒称后续改稿仍失败。

另需同步目录的请求归属：`tb02a_facts.rs:1030` 仅接受 `key[0] == kind || key[0][0] == kind`，五元键不会归属任一结构轴；把第一元改为对象 fact_key 后，CC011/CC016 的前缀 kind 仍不能用精确相等匹配，CC013/CC014 共用 segment kind 也不足以区分子轴。建议保规范二元请求身份，并由目录显式读取受版本约束的 cc/axis，不为迁就目录改变对象身份。版本根 waiting_reasons 聚合不能代替逐请求原因的正确轴归属。

### MEDIUM-1：历史 A/B oracle 比较仍固定 13 轴，生成端现按 V2 规则选择 17 轴

`generation.rs:324` 新增按 meta.rule_revision 选表。现有 `OracleDb::new`（:3724）使用根 `tests::init_test_db`（s_structure_session.rs:5386）建立 V2 oracle，batch.rule_revision 是 RULE_REVISION。`restrict_oracle_to_tb02ab`（generation.rs:3663-3710）只去 C/D 对象并固定重算 NewBi 13 轴，未改变该规则字段；`generate_draft`（:3798）仍把 oracle 的 RULE_REVISION 传给生成器，`finish_axes` 因而选 SegmentSecond 17 轴。`oracle_prefix_parity_two_prefixes`（:4176）等完整状态逐字段比较会出现 13/17 轴不同。

这是静态可确定的 fixture/适配域不一致，尚未执行测试。修复时明确 A/B oracle 投影的逻辑域及身份映射，不能改生产目录回固定 13 轴，也不要删除对拍或把不一致当预期。

## 既有 HEAD 编译阻断，恢复时必须处理

`source_audit.rs:1766` 和 `:1791` 引用 `rule`，但所在 `audit_object_record_with_slots`（:1644）的形参、局部作用域均未定义该变量；调用者也未把 rule 传入。`git show HEAD:...` 可见同样代码，故非本次 diff 新引入，仍会阻断恢复树编译。应把真实冻结 rule 沿 full/native 两条同核调用链传递，不能固定成新域。没有运行 rustc 来冒充已复现。

## 已核可保留部分及验证边界

- `init_v3` 写新域；writer gate 只接受新域，旧 A/B 常量不改。`tb02b::contract_for_rule` 与目录表已有新域精确映射。
- `prepared` 的 full/bound 两入口均由同一 audited.objects 选择活动 prior；重复段/center/pair 版本拒绝；构造器另核内容身份。这里未发现绕开发布授权的高置信路径。
- 恢复 SecondFoldStore 在唯一 parser 调用前；C/D/E/F 从同次 evidence/store 读取。结构失败不切旧域重判。
- `requests.extend(output.requests)` 为唯一结构请求聚合，CC054 在结构之后构造并插回原历史槽位；四轴语义归专项独审。
- 22 轴目录按真实对象引用、保 delivered/deferred，未将 E/F 父义务声明为已实现/已证明。
- 限定文件 `git diff --check` exit 0；静态检查不证明类型检查、运行、恢复、原始历史或资源帽通过。没有关闭任何票。

## 快照哈希（SHA256）

| 文件 | SHA256 |
|------|--------|
| s_structure_session.rs | fc75d59e8a81cde6ebef41fca035b0b599bc68b2c00e71f5f18e9aae48439907 |
| s_session_v3/mod.rs | 7c7483132ae093cbf14c28fa50debcccafbbd25d3e6bd0d5cab7789df5bc0022 |
| s_session_v2/tb02b.rs | 4d4db9df229bb5768ba5890e445cae272977694349c37128fec4fe32bb81732f |
| s_session_v2/tb02a_facts.rs | 64bd1f1e7e0abc902f3206668067941fbf91222f7d308faec88908b1bc4f812e |
| s_session_v3/generation.rs | 027d9253747b75994c433e40f225c3936a803bcdb22985889074ce6e06eb01cd |
| s_session_v3/object_index.rs | 6dc03d398393c8f2705aae5815ad82fd2ae0c2fbe94f59a05ec811bbd36a2f87 |
| s_session_v3/prepared.rs | dc1cdab4f60bd804ebe679e006ff790a9ca253f0d4a29ba97b9f4ff73893b97a |
| s_session_v3/source_audit.rs | 24fe89596e4ca369d4a46512b362dcd5a806a31e56dbd1cb4c0d2a731e55c0a0 |
| s_session_v3/source_pending.rs | bae94f3aaaca5d55df71b695cd9cdff7bfe38034263a9e42774fee10177ecf53 |
