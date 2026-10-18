# #1527 纯旧验证器迁移、首错窄修与 M-03 调用对齐

本轮已完成授权的五文件候选改动及定向静态/小模型验证，保留未提交差异交管理者冻结、独审。**不宣称 I1 完成，不授旧域生产资格；Rust 构建、测试、SQLite 均 NOT_RUN。** 名分为绑定 OPEN #1527 的工作草稿。

管理者 `/root/manage_1527_sandcastle`，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`。本工位为 Sandcastle 外部 Codex 工蜂，真实 session_id 由运行时登记；不声明已注册为父任务原生子代理。本轮无其他工蜂，无票务、联网、安装、索引、凭据读取或 Git 写操作。

固定 source 为 `9d8e5c328a4a15aad1e53dac1a875f788e30b05b`，tree 为 `87167df04264e1287f5e6810b60e41241144fd99`，实际以父目录 `LEGACY-PURE-BASE.json` 为准。开工工作树干净，分支仍为 `codex/1527-formal-i1-integration-20261018`。产品自 `ce3dd877359e4ccec76b54f1d3d0f3585d85d78f` 未变；未读取或改动其他工位活 WIP。已读 AGENTS、ROOT-I1-IMPLEMENTATION-SCOPE、I1-RESPONSIBILITY-AND-REUSE、原设计 §1–6，以及固定 `c1c156ed5851c36395fed35e0dc763f5d5b248e5` 的 PROPOSAL/interfaces。M-03 来自固定 `b228d8ec05173127f17c4af777b2df51c65aa425` 的独审 REVIEW。固定 9d8 旧域独审的 3H/1M 是全接线设计缺口，本次不据其阻断已准先行片，也不把本片等同关闭这些问题。

当前工具表无可调用代码图；故回退固定对象及任务路径的文本读取。父目录已有 XERJ checkpoint 查询缓存命中为零，PRIOR-R4-PROVENANCE 给出设计来源；本轮只复用这些缓存与已冻结 Git 对象，不查询网络、不建索引。MANIFEST 绑定全部实际取证源、父输入和旧报告摘要。

**#1527 本轮根裁合同原文。** 以下逐字引自父目录 `ROOT-LEGACY-G1-G4-DECISION.md:5`；根裁定随本合同入包，历史提案不回写：

> 根已读旧域161行草案，技术选择如下（请原文落票/合同，按最小写集独审）：G1选固定legacy-evidence/1＝1+原payload.len，GraphV2/3共享旧编码域，LegacyBlob真实无图节点为0，S-content保持原帧；同Job聚合两域nodes/node_bytes但集合物理隔离。既有数值授权不得静默扩义：生产启旧域须在具名新准入合同显式绑定该计量域及统一上限，未具名授权仍拒绝；本裁定不填任何数值、不自动改持久v1。G2准纯旧验证器迁入库＋opaque checked证明、bin共用同一算法；新增持久集合/凭据必须先有具名同父布局许可，未有许可不可裸分配后补费，该桥等B1/1488冻结接口。G3选择每次实际取到六/七族成员为访问，缓存规范成员再次参与独立比较也算实际访问，字段/头/控制行不另算；未发布raw只有实际被审计作为raw_history语义成员读取时计record，不把SQL元数据计入；全同类遍历入口先清单核覆盖，禁len预收。G4采第一实际观察错误＋已接受账/拒收实量/未知后缀分明，非约束路径同原算法逐字对拍，有限预算中途停不推后缀。准修:96/:172后到收尾盖首错，仍执行清理/保次错。可先实施不依赖新资源父的纯验证器迁移与首错窄修，冻结后独审；其余源接线以完整schema/同父闭包审查为门，勿去现有拒绝或开产品运行。不得与现Rust独审WIP并写，复用干净工位。

实际落实范围如下；G1、G3 和 G2/G4 的其余全接线责任只继承上述门，不借本轮扩权。

| 片 | 实际产品文件与当前行号 | 结果 |
|---|---|---|
| A | `rust/src/s_content/legacy_evidence.rs:8`、`:16`、`:43`、`:47`；`rust/src/s_content.rs:52`；`rust/src/bin/s_session_v2/evidence_store.rs:30` | node_id/verify_node_row/hex/hex_bytes 四个函数体逐字迁移，bin import/re-export 共用唯一算法。 |
| A 凭据 | `legacy_evidence.rs:53`、`:61` | CheckedLegacyEvidenceRow 的 id/kind/payload_len/declared 都为私有固定栈字段；verify 先执行原 verify_node_row，成功才构造；只读 getter 不产生费用或来源许可。 |
| B | `rust/src/bin/s_session_v3/admission_private/mod.rs:34`、`:131`、`:177`、`:219`、`:304`、`:323` | 私有固定阶段首错载体；回调在 bool/SQL 包装前保存原因，业务结果在收尾前保存；最终收尾后封口。 |
| M-03 | `rust/src/bin/s_structure_session.rs:5580` | 仅 cfg(test) 区新增三个薄业务夹具并替换旧调用，生产命令及 scope 门逐字不变。 |

A 的旧函数锚为固定原 `evidence_store.rs:856`、`:2223`、`:2385`、`:2389`。完整文件按确定性移位重建后与新 bin 逐字相同，包含 SQL/Reader/owned 路径，故原 `i64/BLOB/i64` 类型拒绝位置、finalize 与原行检查先后都未挪动。当前普通 Reader::load 在 `:2108`，admitted 读行在 `:2186`。原 kind → hash → 非负 len → 叶 len 的判据及错误中文逐字保留；SEQ 仍只在本层核行，不把行凭据冒称展开/闭包/来源证明。没有把 TEXT/REAL 转成合法输入，没有去重命中跳验证，没有旧域消费口或新集合。

库/bin 属不同 crate：`rust/src/lib.rs:86` 的 s_session 门暴露 s_content；模块及迁移函数用 pub，bin 的 hex_bytes 继续以 pub(super) re-export 供旧调用方使用。`rust/Cargo.toml:78` 的同 feature 已启 sha2，`:94` 要求该 bin 启同 feature；Cargo、lib 和依赖逐字未变。S-content 原文件只增加模块声明，job.rs 全文件不变；原帧配方、集合、费用和判定均未动。这里是静态可见性/feature 核对，未代替 Rust 类型/链接验收。

**B 的单线程顺序和所有权。** validate 仍先用原 account 过门并取得原 grant；其栈上的一个 RefCell<ObservedErrors> 借给 SharedAccounting，后者仅短借原 AdmissionAccounting。没有 Job/Rc/Clock 的新建、替换或重置，也没有向调用方公开可变 Job/错误槽位。

1. 正常运行原业务及 VFS/progress。既有 before_io/precheck/charge_wire/节点/record 门实际返回错误时，observed 立即锁存原错误 String，然后原 Result 才交回 VFS 转 bool/SQL。此处没有事后调用 check_deadline 来倒推先后。预算拒绝先发生时，其原因先于 SQL wrapper。
2. 原业务 closure 返回后，在 rollback 前保存该 Result。若此前没有回调错误，原业务 String 成为首错；若有，则保留为独立的业务返回次因。quick_check/连接错误也在显式 close/registration finish 前记录。
3. 撤下原 VM progress 门，执行原 tx.rollback；回滚后执行原 precheck；随后 session.close、registration.finish、原 account.reconcile(grant, 0)、原 check_deadline 逐项执行。所有 Result 先求值再记录，失败不跳过后继收尾。不存在的事务/连接沿原失败路径析构，不虚构一次已执行的 rollback。
4. 最终 finish 只取已保存的第一实际原因，追加具名阶段次错与拒收实量。业务先错、后到期限不会覆盖；早到 VFS/Job 错、晚到 SQL 包装也不会颠倒。成功必须没有任何已观察失败。无次错的非约束路径返回原业务错误串逐字不变；有次错时原串保持为返回文本前缀，追加诊断使整体 String 有意变化。

载体只有固定七阶段，每阶段分别保存首个回调次因和首个返回次因；重复的错误传播不生成无限轨迹。它保存错误文本所需的 String，不引入长期 Vec/Arc/集合、父 RAM 许可、持久 schema 或跨 scope 生命周期，不声称错误文本的 heap 已获 B1 全资源证明。首错保持原文；次错按阶段列出而非冒充每次重复回调的完整时序日志。原 `fd_wal.rs`（包括回调、错误/重试/关闭算法）及 IoAccounting trait 全部不改；本片处理的早到 VFS 首因是其原记账/progress 门已经实际返回的错误，不扩成全 OS I/O 错误溯源或完整 H-03 收据协议。

`after_read` 只使用原 JobSnapshot 的 wire 前后差：`accepted_delta = after - before`，`unaccepted = actual - accepted_delta`（非负截断）。cap 越限那笔按原 Job 先入账，不能再记作拒收；已停止后确实交到该回调的返回量才记拒收。toy 的 4 字节越过 cap=3、随后已有 3 字节观测，结果为已接纳4/拒收3/总观测7，绝不是4或11。拒收计数不是新余额，不允许继续业务 I/O；累计 u128 诊断溢出会显式标识，不暗称精确。原节点、record、request、stopped、起钟与预留仍属原 Job；本轮未新增旧域 record/request 事件钩子，也不猜尚未访问后缀的错误或费用。原 `admission.rs:146` 的失败分支继续在本次 validate 封口后附原账快照，终端生命周期维持既有 B2/M-02 实现。

**M-03 的覆盖边界。** 根测试中 188 个 cmd_accept/cmd_advance/cmd_recover 调用位于 72 个直接改动函数；沿已有 helper 调用关系，涉及78个测试及4个辅助函数。完整名称、固定源行号、逐命令调用次数和传递名单见 M03-TEST-INVENTORY.json。新增 accept_business_file 只读既有输入文件后调原 accept_core；advance_business/recover_business 分别调原 core，保持原 Option 参数及 Result<(), String> 的薄映射。逆向移除新 helper 并还原三类调用名后，整个旧测试区与固定源逐字相同，原断言、测试数据和业务分支覆盖均保留。

这些断言检查业务/持久状态或原业务拒绝，不检查这三条命令外壳的 stdout 生命周期；原测试里调用 Python 的几处仍是原 oracle，未运行。未新增生产 None 兜底、TestOnly 准入或旧域费用绕口。根生产区逐字不变，read_current_text/write_current_result 的 StartupScopeMissing 拒绝不变。对应独审反例 tb02b_revision_of_inserted_identity_opens_another_generation 现经 advance_business → 原 advance_core；这关闭的是静态调用前置冲突，尚未声称原业务测试实际运行通过。

**验证收据。** CHECKS.json 记录47项静态/模型/打包检查；数量不是 Rust 测试数。已实际完成四函数体逐字比较、整个旧 evidence_store 的迁移重建、根测试逆替换、保护源与34份旧 #1527 报告逐字/hash 核验、五份增量 Rust 的 rustfmt stdin 解析、8项旧行小模型、10项首错/收尾事件模型、1项部分实读计量反例及五文件产品 patch 零 fuzz 内存重建。旧提案27项/160组输入及其他历史套件未重跑、未算作本轮结果。首轮核验器在 M-03 逆替换中多删一个空行，修正核验器后通过；产品无需为该误报修改。收包阶段只补库存分类及未跟踪写集核验，不重复已通过模型。

新增5个 Rust 测试源码（新旧行凭据1个、首错/收尾3个、原 Job 拒收增量1个）及既有共享账测试的构造适配均为 NOT_RUN；M-03 全部原测试同为 NOT_RUN。未执行 cargo/rustc/build、Rust 类型/借用/ABI/Drop/链接检查、SQLite、正式 S/Q、旧 binary、历史重型、真实恢复或 OS EINTR/取消。小模型及 rustfmt 不替代原 Rust 验证器运行对拍或真实生命周期验收。

包内 IMPLEMENTATION.md、CHECKS.json、MANIFEST.json、PRODUCT.patch、verify.py、MODEL-RESULTS.json、M03-TEST-INVENTORY.json、VALIDATION.log 全部明确交包，MANIFEST 不自我 hash。PRODUCT.patch 只含上述五个产品文件，含新模块；报告文件单独由 manifest 绑定。管理者应以本 checkout 差异及 manifest 冻结，再交独审，本工位不自行 commit/push/merge/main/关票。

**保留义务：B1、资源父/同父布局、旧域全接线、I1最终验收、I2–I5、完整9011、64GiB/store、AgentStorage至少148GiB卷及同卷槽位身份、金融/隔离/真实恢复独立双跑、全部六目标均 OPEN；#1405/#1420生产v4仍 BLOCK_NOT_MERGEABLE。** 49GB仍只是含WAL库布局估算，不能除二成74GiB许可。本轮不删 LegacyAdmissionBudgetAdapterMissing，不填预算数值，不选择 allocator/helper/schema，不迁锚，不代签其它独审。
