# #1419 raw/driver v3 八项修复交付

八项可在本工位独立修复的缺陷已落码，定向回归通过。当前状态为 **FIXES_IMPLEMENTED_TARGETED_CHECKS_PASS_REVIEW_PENDING**，等待管理者对本轮差异另会话独审并冻结。此结论不是独立评审通过，不是 #1419 完成，也不批准产品执行。

当前 checkout 为 `codex/1419-raw-fix-v3-20261018`，固定 HEAD 为 `7cc205c8a83a683fd0ce7c84dfe753a198c5dc92`；差异未提交。工蜂是 Sandcastle 外部 Codex，管理者 `/root/manage_1419_sandcastle`，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`；真实 session_id 由运行时登记，本文不声称原生子代理注册。名分为开放票工作草稿。

`authorization=false`、`execution_authorized=false`；source/tree/binary/frozen rule 仍为 `PENDING_SOURCE`。没有修改生产 Rust、src、formal 或配置，没有 cargo/rustc/Lean 构建，没有产品、服务、真实数据库、Long/历史回放或浏览器运行。没有 tracker 写入、claim、commit、push、merge、切分支、Git 配置写入或开启其他工蜂。没有读取认证文件、凭据或密钥。

## 修复与实测

| 项 | 实际修复 | 具体触发与新输出 |
|---|---|---|
| F1 | 比较低级端点几何及笔 formation/formed_evidence；段引用绑定实际 bi entity；完整核 geometric_end、construction、strict、fact_vector（含破坏及终结证书）、trigger；seed 五条件、核心贡献、单元 object_id、中心 seed_attempt、连接单元都接入 | 239 个逐叶字段篡改，每次重建合法内容闭包，再被领域比较以 ValueError 拒绝。包括端点价/锚、布尔条件、证书嵌套字段、根集、引用等。新静态正控为 10 完成段、2 中枢、1 对；结果名改为 `SCOPED_STATIC_FIELDS_VERIFIED_LIFECYCLE_UNVERIFIED`，不再泛称 MATCH |
| F2 | plain 每次访问、分配前扣 Graph 同一累计工作预算；records 先选原始对象，再展开受测 payload；Collector 的临时 Graph 与最终闭包共用预算和截止回调 | 独审同型 16 层共享 DAG：51 节点、2380 body 字节，decode=166；plain 在 used=1001、limit=1000 确定拒绝。records 内重复子图也在 1001 拒绝；未受测三族保留共享结构；截止回调拒绝也被测试 |
| F3 | 枚举原始 objects，先核 kind 及受测 payload，不再过滤缺失/null 后计数；八种受测 kind 的 data 必须为对象 | 8 kinds × 5 种缺失/null/标量/缺data/非法data，共40个合法闭包负控拒绝，含额外 center_pair |
| F4 | 可空引用必须实际存在，预期 None 时 wire 必须严格 null；非空 ID 必须存在且映射到预期目标，成员、连接两端、pair 两端复用检查 | 成员/连接/尾部三种身份 × 三个引用字段 × 三种非法值共27项拒绝；另核 pair 两端、缺 null 字段、中心 members/Zn 悬空 |
| F5 | close_projection 建图后，对原 snapshot catalog、catalog_evidence、每个 record 原始 ContentRef 执行 checked | 三种位置 × kind/extent/format 九个负控，仅改 snapshot 元数据、节点与 root 不变，均拒绝 |
| F6 | Popen 成功立即进入 try/finally；支持未创建 selector、仅注册部分管道、仅打开部分日志；kill、wait、各句柄关闭分别执行 | 纯 fake 注入 selector 构造、set_blocking、第一次/第二次 register、第一次/第二次 open 失败，均 killpg=1、wait=1、stdout/stderr关闭；另复验原六条固定 Python toy 正常/取消路径。所有 fake 信号均 mock，不接触真实 PID |
| F7 | 每推进一个特征元素先检查无包含域；不足三元素、没有 strict 分型与 pending 分支也检查 | 原独审19bar反例，两个反向区间 [1200,1600]、[1280,1520]，现在报“域外：特征序列有包含”。另测第三元素才发生包含、无 strict 分型；原四份固定输入只复验新增域门不误拒 |
| F8 | input_frontier 解析规范非负整数字符串、同会话单调；每代与具名提交的接纳事件数差值相等；新增原始请求到实际成功 cut 的绑定接口 | 138→100、负数、前导零、小数、JSON number/bool/null、接纳计数不符均拒绝。跨 epoch 正控 138→139→204→269 与1/65/65事件一致；affected seq_range.to=7，不作消费前沿 |

逐项机器矩阵为 `FIX-MATRIX.json`；每个触发、拒绝字符串和计数见 `CHECK-OUTPUT.json`、`SUPPLEMENT-OUTPUT.json` 与 stdout 原输出。

## 正控、独立性与原件身份

`independent_fixture.py` 只读取 `../RAW-INPUT.json`，从实际 OHLC 三 K 取得端点与笔区间，宏观段见证索引复用外部独审已核结果；它不读取 INDEPENDENT-WITNESS-LEDGER、expected、derive 或 domain_fields。此夹具是独立于被比较账簿的静态 wire 正控，**不是正式 S/Q 输出，也不是另一次独立评审**。原 `test_v2.domain_records` 已适配字段集合，但只作旧 API 自洽回归，不列作独立 oracle。

外部独审完整原件保存为 `V2-INDEPENDENT-REVIEW.reference.json`（原会话 `01a14d4d-ddae-7e30-ab34-1933824dc229`，2026-10-18 04:47:34.913Z 退出、code 0、结论 CHANGES_REQUIRED）。该审查独立核完全部139前缀及几何/两中枢/16格、修订与撤回数值；本轮不重跑其完整几何穷举。原 raw、expected、独立账簿及定义引文逐字节不变，不用修改答案消除缺陷。

`../validation/` 旧输出、旧 PID/时间与旧树身份全部保留；根报告的旧 `5cb7253d…` 身份没有改写为本轮结果。`V2-MANIFEST.json.reference` 和 `V2-SHA256SUMS.reference` 保存旧封包身份。`initial-run-sources.json` 绑定首轮14项所用代码；`final-run-sources.json` 绑定尾差复验时的代码。新工具和报告的最终字节身份由当前根 MANIFEST/SHA256SUMS 统一覆盖。

`STATIC-WIRE-SOURCES.json` 保存只读静态字段对照的文件哈希与逐行摘录，来自 `adapt-v2-worktree` 的 `2694afcf147f3ba4114fde386470886c2428efc0`。此身份只证明字段对照对象，不替代本轮候选最终 source/tree/binary 冻结。已复用既有来源/索引失败证据，没有重装或重建索引。

## 逐项未验证量词及接点

`compare_domain` 的通过范围仅限返回值 `verified_subquantifiers`。以下字段无论 payload 内是否出现，都会单列 UNVERIFIED；故伪造 terminated_known_at 不会得到“获知时点通过”的结果。

| 量词 | 当前状态与可接接口 |
|---|---|
| first_visibility | UNVERIFIED。`compare_first_visibility(traces, ledger, bars, prefix_bindings=...)` 接全部139前缀的 close_projection 输出；逐条绑定实际 session_id/session_generation/cut_generation/input_frontier 和 raw 前缀 SHA。已测两前缀接口及错 cut/raw/定义前沿拒绝；真实139前缀产品输出未接入。该接口只验可见身份前沿，不认证接纳动作、产品 known_at 或每个中间切面的全部领域值 |
| known_at | UNVERIFIED。未来需真实对象 formed_known_at/terminated_known_at/settled_known_at/known_at 与正式接纳轨迹绑定；不把定义 first_known_input_count 写成产品 generation |
| AsKnown | UNVERIFIED。未来同源 `fixed_cut.history_mode=AsKnown` 的跨代固定切面完整原件；价格恢复相等不能代替不可回填检查 |
| entity_revision | UNVERIFIED。未来 bi/segment/center/pair 的实际修订号与先前对象、变化原因绑定；当前没有编造修订号正控 |
| raw_history / withdrawn_objects | 各自 UNVERIFIED。闭包核到了六族的内容完整性，但没有把其记录内容与270接纳事件/具名撤回逐项接线；空或任意历史记录不获得生命周期通过资格 |
| 低级 source_coords/sealed_at、confirmation | UNVERIFIED。当前核端点几何、根集、双坐标条件、笔及段引用；端点封口来源、笔不可逆确认历史仍需正式字段与接纳切面 |
| feature_sequence_objects、完整目录轴、product_source_binding | 各自 UNVERIFIED。strict/fact_vector 内的元素与证书已核；独立序列对象、22轴全目录及最后源规则绑定另需实际输出 |

watch 的新接点为 `bind_named_submissions(submissions, accepted_cuts)`：输入是原始具名请求及其实际成功后固定切面，函数核请求 payload_hash、会话/代际唯一性，按 `raw_input.events` 求事件数；`Collector.watch(..., accepted_commits=...)` 逐代核前沿差值。禁止猜测 cut_generation。测试中800系列 cut只为接口夹具，没有写入产品或候选原件；已从冻结 epoch2 后缀末三条验证1/65/65。

当前固定输入的 seed 尝试均为五条件全真。无独立失败 seed 排除原因见证时直接报域外，不能把实际 exclusion_reason 当预期自签。本次没有扩大包含处理域或引入已裁规则的宽松兜底。

## 定向验证记录

首次 `python3 -B fixes-v3/test_fixes.py`：14项通过，50.758秒；输出 `tests.initial.stdout`。除239次字段篡改外，覆盖F2–F8指定反例及必要旧 API 回归、六个固定 Python toy。没有运行旧全套测试或此前23项。

尾差 `python3 -B fixes-v3/test_supplement.py`：前5项通过，第6项失败，完整输出 `tests.supplement.stdout`。失败是新测试错从 `messages.base.jsonl` 尾部读取修订，实得 `[1,1,1]`，不是要求的 `[1,65,65]`。修为原有 `messages.recovery-epoch2.jsonl` 末三条后，仅复跑 `test_f8_bind_frozen_named_messages`，`tests.f8-recheck.stdout` 通过。未改事件数判据、raw 或 expected，失败原件未覆盖。最终20个具名顶层测试都有通过证据；嵌套复验不另算独立测试数。

`delivery-check.stdout` 复核原件与旧资源；`static-check.stdout` 核77份旧输入/expected/来源/验证原件字节、Python AST、已签 AC 量词不变、原帽不变、无越界差异，以及 `--live` 返回1、明确 PENDING_SOURCE 拒绝。只运行与变动逻辑相关的验证。

## 封包与仍欠范围

本轮候选合同为 `EXACT-RUN-PROPOSAL.json`（路径字段仍相对父产物目录，文件内已明写）。它完整继承原输入、expected、commands、固定切面、双跑/恢复计划和原 S7/Q21/累计资源提案；只更新准备基线、范围声明和未准入门。全部授权 false；固定 toy 仍只允许监督器内置的四种 Python 字符串，没有任意产品启动能力。

`changes.patch` 包含全部新增实体和已有文件修改。为避免哈希自引用，补丁不装入它自身及两个可再生根清单 MANIFEST.json/SHA256SUMS；`seal_v3.py` 生成完整当前清单，清单覆盖补丁、旧原件、新代码、报告和全部输出。管理者可先核清单再收差异；本工蜂没有 commit 或 Git 写入。封包输出及逆向 dry-run 的结果由最终工具输出给管理者，未写入会造成自引用的本报告。

受控 raw 只作 AC-DOMAIN-MATRIX 列明的局部行为锁。真实市场源认证、市场原量、完整历史、正式非真空 S3Q、全级递归、下游消费、完整生命周期、正式双跑、持久恢复、浏览器与全票成功仍欠。同步 IO 硬截止、产品后代追踪、OS 内存/磁盘硬帽仍 UNQUALIFIED；本轮 toy 不抵扣这些义务。最终冻结与独审验收由管理者负责。
