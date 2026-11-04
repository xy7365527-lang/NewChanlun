# #1419 raw/driver v5 同票修复交付

F1、F2、F8 已实际修复。最终源的 **26 项 Python 内存/fake 定向测试通过，193 条观察记录**；仍待管理者冻结后独立评审。这里没有整票验收或后续执行许可。

本件是 #1419 开放票工作草稿。固定基线 `46a11d1cbdeecab3444efbfd324be29a73ecb293`，沿用 `codex/1419-raw-fix-v5-20261018`。身份为 Sandcastle 外部 Codex 工蜂，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`、管理者 `/root/manage_1419_sandcastle`；真实 session_id 由运行时事件登记，不声明原生子代理注册。

## 修复与证据

| 发现 | 最终行为 | 实际验证 |
|---|---|---|
| F1 | `driver/driver_v2.py:73` 在 `plain()` 前遍历原始公共 payload，任意对象/列表深度禁止 JSONText/0x07；非几何字段、非受测 kind 也不例外。检查复用 graph 的 WorkBudget，访问和容器分配前收费。`records:95` 仍核八类受测 kind 的完整对象集合、公共 data 对象形状和内部字段拒绝。 | 八种 kind × 六种布局；40 个单层 0x07 布局经合法重哈希闭包后由公共类型门拒绝，8 个 JSONText 套 JSONText 由既有闭包层先拒绝。另含原 `geometric_end` 反例、未知 kind 两负控/普通值正控、全集多余对象、原 8×5 缺失/非法载荷、internal 冲突、耗尽及到期预算。 |
| F2 | `fixed_cut_driver.py:120` 构造即必传 WorkBudget，身份复制不再先于逻辑准入；`:144` 在序列化/写出回复元数据前，核 status、headers 全部键值与嵌套和 raw_reply。分页/watch 标量、具名绑定、可信切面及状态复制均走原共享预算。整数按位长先收费。 | 300 帽下空 header 通过，2,000,000 字符 header 在 used=109、requested=1955 时拒绝，headers 序列化次数为 0，只留下已准入请求 319 B。键、值、多层列表、碎片及大整数、已到期 callback 均测；同 4096 字符整体/列表/嵌套费用 14/23/47，拆分不能降费。 |
| F8 | `fixed_cut_driver.py:281` 的 `bind_watch(fixed,cursor)` 一次绑定完整可信切面及初始游标；`:307` 跨调用保存上次已验证完整 fixed、cursor 和提交去重账。八个固定身份始终与初始信任根、上一切面一致；其余八个字段按 delta/代际检查。整次调用成功才推进状态，失败不回滚费用，不允许重绑。 | 八个固定身份分别在首代/后代、合并/分次中变更，32 例均拒绝；profile toy→alien→third 只接受第140代。单页三批与三次调用的正控结果/最终状态相同，跨epoch 1/65/65 对应139/204/269，`seq_range.to=7`未替代前沿。另测999对139冲突、旧游标重试、深复制隔离、提交重复、失败后的状态与预算。 |

八个固定身份是 `session_id/session_generation/catalog_revision/profile_id/profile_hash/scope/rule_revision/history_mode`。八个逐代字段是 `structure_cut/cut_generation/index_frontier/input_frontier/seq_range/catalog_run_status/catalog_evidence/as_of_generation`；逐项与 delta 或当前 generation 绑定。`catalog_evidence` 的内容根可以随代变化，不能误当跨代常量；catalog 身份仍由 `catalog_revision` 及 profile/scope 等约束。完整映射在候选合同 `watch_contract` 中。

`bind_watch` 的“可信”是调用者已核验快照的入口前件，不是由当前 accepted_cut 自签产生。本轮 fake 只测试绑定检查，不证明来源真实性或正式可达。

## 测试原件与范围

最终命令：

```text
python3 -B .chanlun/review-results/issue1419-raw-oracle-v2-20261018/fixes-v5/run_checks.py verified
Ran 26 tests in 38.788s
OK
重复全图fake拒绝 {'calls': 45, 'work': 1000000, 'limit': 1000000}
分块完整图fake通过 {'calls': 182, 'work': 603763, 'limit': 1000000}
```

逐条原输出为 `tests.verified.stdout`，实际源 hash 和观察值在 `verified-observations.json`。测试期间文件输出、临时目录、exchange 均为内存替身；只由 runner 写本轮具名报告，不启动真实进程或传输。

保留了前轮 22 项中的 **12 项必要回归**：公共对象正/负控与全集、decode/plain/records 共预算、Collector 不重置帽、三项 watch 原门、两个领域适配用例、完整 fake 收集和禁止 live。fake 收集重跑是因为构造合同和元数据收费影响整个采集累计值；仍使用原 100 万帽。未重跑独立原始几何推导、F5 头/ContentRef、大 hex、单独 Graph 建图/重复节点/跨页检查或真实 toy 监督。

本轮试跑过程全部保留：

| 原件标签 | 实际结果 | 说明 |
|---|---|---|
| initial | 3 项，3 失败 | 修复前三个独审反例均复现。F2 本最小请求体仍为550 B回复，因 route/请求不同，元数据为2,000,272 B、work=112；不把它写成独审的2,000,284 B/118。 |
| repaired | 3 项通过 | 首次修复后的原三个反例。 |
| acceptance | 24 项，8 errors | 测试误把 JSONText 套 JSONText 当作可通过闭包的输入；实际被原有 `json_text内非普通JSON` 门提前拒绝。未改闭包判据或提高帽。 |
| final | 24 项通过 | 测试明确区分既有闭包拒绝和公共类型拒绝。 |
| accepted | 25 项通过 | 增补未知 kind 公共类型及单页多批/分次等价。 |
| verified | 26 项通过 | 增补分页/watch 外来大整数在字符串分配前准入，最终代码身份。 |

`TEST-SOURCE-REVISIONS.json` 逐次将报告源 hash 定位到固定 Git 原件、当前文件或已核 hash 的历史源码副本。`driver/test_v2.py` 仅适配 v5 的显式工作预算构造，帽仍为100万；v4 测试源码未改，watch 用 `RetainedV4` 明示新调用身份。旧失败、旧已封报告均未倒改。

## 来源、合同与交付

已全文读取原 `RAW-V4-INDEPENDENT-REVIEW.json` 的发现和验证；其原件为 `V4-INDEPENDENT-REVIEW.reference.json`。原run `3ec92c77-dadc-47ef-a202-17e705859f81` 于 `2026-10-18T05:53:34.413Z` code 0 结束，但结论为 CHANGES_REQUIRED，未把退出成功当评审通过。

0x07 限定依据为固定2694的 `object_records.rs:38`；固定46a的 q3 FIXED 字段、已安装 Snapshot 才可 watch、逐字段 Delta/Snapshot 对应、最终游标等摘录连同 blob/文件/范围 hash 见 `STATIC-WIRE-SOURCES.json`。只是静态协议依据，产品 source/tree/binary/frozen rule 仍 PENDING_SOURCE。

已先读取旧 XERJ 请求失败记录、响应及参考命中，沿用原件；本轮没有新 XERJ 成功检索、重装或建索引。图工具不可用，代码定位回退固定 Git 原件和限定文件读取。指针及 hash 见 `XERJ-AND-PRIOR-READS.json`。

`METADATA-ENTRYPOINTS.json` 用 AST 穷举 Collector 和具名绑定的26个 canonical/deepcopy/write_bytes 入口，逐处列前置准入。`FIX-MATRIX.json` 逐项映射三发现与测试。候选 `EXACT-RUN-PROPOSAL.json` / `DRIVER-CONTRACT.schema.json` 升为/5，JSON schema 明说无法代替原始内容 tag 检查。

最终封包由 `seal_v5.py` 生成 `SCOPE.json`、`STATIC-CHECK.json`、`changes.patch` 及根 MANIFEST/SHA256SUMS：相对46a的全部新增/修改，补丁排除自身及可再生根清单；根清单记录其他每份产物 hash，SHA256SUMS另覆盖MANIFEST。排除项、重建命令和反向check结果明列，不制造自引用哈希。`V4-MANIFEST.json.reference` / `V4-SHA256SUMS.reference` 保留旧封包清单。

## 保留的未验范围

authorization、execution_authorized 均为 false；known_at、AsKnown、entity_revision、raw_history、withdrawn_objects 等仍 UNVERIFIED。未泛称 MATCH。原raw/expected/commands/cuts/replay/资金与资源帽保持原值。

正式源/类型/接线、22轴、实际生命周期、市场来源与原量、完整历史、非空程序 full-group、正式双跑、持久恢复、浏览器、OS硬帽及整票仍未验。未运行 cargo/rustc/Lean/lake、产品/S3Q/服务/DB/Long/历史/真实传输或真实进程信号；未认领、评论、commit、push、PR、merge、切分支、改Git配置、删工位或开启其他工蜂。
