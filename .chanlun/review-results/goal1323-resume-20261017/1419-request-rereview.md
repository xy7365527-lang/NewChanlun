# 代码审查报告：#1419 具名请求四轴与 oracle 规则投影复审

日期：2026-10-17。绑定 #1419 的工作草稿。工作树 `.sandcastle/worktrees/resume-1419`。只读复审，无源码修改；没有运行 cargo/rustc、服务、外盘或远端。

## 概念层质询结果

**本次限定范围静态通过。未发现超过 80% 置信度的新矛盾。** 先核已签 `SPEC-COVERAGE-INPUT.md:185-196/228` 的具名子轴，再核 `.chanlun/definitions/xianduan.md` §5.1 的语义状态与扫描覆盖正交、`.chanlun/definitions/zhongshu.md` 的 #1478 同次 Zn/核心合同。这里不裁定未来 #1493/#1429 的实现，也不把此次对象和请求覆盖当作整个父轴完成。

| 请求 | 实际实现及核查结论 |
|------|------------------|
| CC011 length / construction_conditions / direction | `content_structure.rs:184`。seed、候选 construction、每个实际 termination_attempt 各构造具路径请求；length 已知即 COMPLETE；四位向量含 false 仍完整，已知偶数/不足三笔为条件轴域外，通过完整条件前方向轴域外；奇数域内缺向量才数据不足/域证据缺失/未完成。没有把 false 向量改成缺数据。 |
| CC013 semantic_termination_result | `:809-839`。从同次 candidate.result 与 second.resource_truncated/matched 取状态；当前前缀已求出的 NO_FIRST/PENDING/KILLED 与扫描资源未完成区分；已知几何失败、已确认/失效不被资源状态遮蔽。资源未完成保 SUFFICIENT/IN_DOMAIN，仅 computation=INCOMPLETE。此处没有新判定器或降级重判。 |
| CC014 known_fact_vector / scan_coverage | `:840-879`。两个 Option 命题均 Some 才算完整，Some(false) 不缺证据；None 不填 false。已核共享 BreakFacts：stroke_broken 是已求 bool，new_segment_formed 与 segment_broken 同源于证书/扫描完整性（parser/segment.rs:295-350）。scan_coverage 请求描述已知覆盖事实，资源不足本身是已求出的覆盖状态，故该请求 COMPLETE 不等于结构扫描已完成。 |
| CC016 seed_condition_vector / core_order | `:1487-1521`。同次 scan 的完整三单元五位向量与核心比较；strict Some(false) 是完整否定。输入仅来自已核完成段 UnitInputs，condition_vector(SeedInput::TOWER) 的 strict 前件一致。core_order 从同份三个完整区间计算，未用成立与否替代几何比较。 |
| CC018 unit_core_position | `:2024`。消费同次 tested_positions，主体携冻结 strict 核心、该单元及真实源并集；位置 COMPLETE 不升级为中心破坏，destruction 仍保留待 #1434 的合同。 |
| CC020 center_pair_development | `:2206`。Evaluated 才完整；≥9 PendingUpgrade 明确 INSUFFICIENT/DOMAIN_PROOF_MISSING/INCOMPLETE，不发叶或矩阵。上级 realization 未构造与已求相邻 pair 分开，不把独立延后义务错误传播成关系未求值。 |
| CC022 assignment_status | `:1866`。从同次 unit_facts 的 member/connector/not-yet 和进行中单元明确状态发布；已知 NOT_YET_ASSIGNED 为当前固定读法已求出的归属状态，未混作缺计算，也未造 member_of。 |

上述请求只声明本次输出的 CURRENT 有效版本，不重写 prior/历史对象。CC012/017/055/056 没有被机械强造分类请求。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | 未发现 |
| HIGH | 0 | 本次范围未发现 |
| MEDIUM | 0 | 先前 oracle 域不一致已静态消除 |
| LOW | 0 | 未报风格项 |

结论：**PASS，仅本次静态范围；运行验证 NOT_RUN。** 上轮报告 HIGH-1 的来源审计扩域及已有 rule 缺参由另一工位修复，本报告不复审其在飞代码、不撤销该未验收项。

### 请求身份、来源及唯一聚合

`classification_request`（:144）使用 `[object_fact_key, [cc, axis, "structure-classification/1", path, obj3]]`；object_fact_key 复用真实 kind/slot，主体是本次实际 TypedObject.object_id。多个 construction/termination_attempt 的路径区别明确。请求含固定的五个字段及完整四轴；source_coords 使用与主体发射相同的来源集，经共享 producer 取真实源节点。

`content_producer::explicit_request`（:1322）在调用方 NodeStore 写并渲染含 slot Edge 的键。上轮发现的五元键拒绝和私有 scratch MissingNode 两项均已消除；没有修改 A 的既有 request_value 启发式来套 C/D/E/F。

`generation.rs:1051-1089` 只将 `StructureOutput.requests` 汇入 requests，然后生成 CC054；没有再从 TypedObject.request 收第二次。目录仅在新 CenterPair 域与明确 structure-classification/1 版本下按 cc 分轴，因此 CC011/016 前缀和 CC013/014 同主体不同轴不再串用。

旧完整 JSON builder 只在 `cfg(test)` 的有界 oracle 中；生产事实走共享 typed 核，request key 的规范文本属于身份协议，不是旧 JSON 业务构造回流。

### oracle 规则投影修复

`generation.rs:3663-3715` 的 `restrict_oracle_to_tb02ab` 先断言输入 batch.rule_revision==RULE_REVISION，再把 A/B 投影 metadata 显式映射到历史 V3_RULE_REVISION。原库、all_events、effective 和保留对象的业务载荷不改；沿用原 A/B 对象/关系过滤与 NewBi 13 轴重算。

检查过 `delta_full_cold_matches_independent_sealed_v2_oracle`（:2410）、`oracle_prefix_parity_two_prefixes`（:4183）及 `oracle_incremental_second_generation_parity`（:4293）：runtime、GenerationInput/BoundGenerationInput 均使用投影后的 rule；多代的两份 oracle 各自从真实库新读、各映射一次，gen1/2 同一 runtime 不发生规则身份冲突。身份变动仍由既有 obj-/obj3-/wit 映射处理，未删全文字段对拍。

因此上轮 MEDIUM-1 的生产 17 轴/测试 13 轴不一致已静态消除。它是明确的历史 A/B 投影测试，不能称作未变更封存字节，也不证明新 A…F 域与完整历史等价。

## 验证状态与后续门

- **NOT_RUN**：所有 Rust 类型检查、单元/集成测试、同源发布、重启、远端 CI、历史回放、资源约束。
- 已静态阅读三项新测试：construction 三类状态；真实 parser C/D 对拍与请求主体/来源；E/F 前缀对拍、≥9 pending 和请求个数。它们确实调用真实工厂和含 Edge 请求键，不是只检查字符串。
- 资源截断分支虽由共享事实条件静态核对，本轮没有运行证据证明测试样本非真空命中该分支；远端验收应保留具名预算截断反例，不能用当前全为未执行的测试文字计通过。
- 限定三文件 `git diff --check` exit 0；staged diff 为空。未运行额外构建，也未更改源码或票状态。
- 新请求最终能否经来源审计及发布/查询/恢复，还取决于另一工位的稳定修复及独审，不能据本报告关闭 #1419/#1323/#1279。

## SHA256

| 文件 | SHA256 |
|------|--------|
| s_session_v3/content_structure.rs | 1b3001a12ffd822700a14475b6b90650707242301cd329ca50deb53f2da9bda3 |
| s_session_v2/content_producer.rs | 57c02cf2c4a8810e4031048533826f5174f80c88af97fe6154718bfd4ee0eacd |
| s_session_v3/generation.rs | 471825f38f5a41796346b286386d64f5ad25e06bfbd01ac51ea77248bdda4c0f |
| s_session_v2/tb02a_facts.rs | f33aa9e513191f689b18539b06faf97b159484bbe9f8d88aefef5f8a92d4c5b4 |
