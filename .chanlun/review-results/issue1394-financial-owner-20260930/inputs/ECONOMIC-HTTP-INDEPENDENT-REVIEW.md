# #1394 经济只读 HTTP 实际证据独立事实补审

结论：本封包支持“新冻结只读 Q 身份在具名 TestOnly economic-session/2 实例上完成六次实际 HTTP 观察”这一限定事实，PASS。没有发现该限定声明中的具体错误或阻断遗漏。代码评审已经通过，此补审只核证据与事实范围，不重复审函数签名，不重跑服务、测试或浏览器。

固定根提交 `976702750dfd7e89706223216ae8e0e19f2b9c87`。原封包 `.scratch/issue1394-economic-observer-actual-20260930/ECONOMIC-OBSERVER-ACTUAL-MANIFEST.json` SHA256 `53fe8cda6fa5fae453ac36b1d7a24e35027994973b6dd63068fffd6f2e8b4523`，首尾相同。封包引用的 47 份原件逐项独立核过文件 SHA256 与长度，包括 native bin、来源、六次请求/响应和六个磁盘 image。未重新散列 ZIP，不修改 EVIDENCE、RESULTS、源码或其它原件。

## 来源与权限

根四文件 SHA256 与已独评候选 `d34dd1e0d2e4600e460d8d13e158a3a2cdaea3f1` 一致。economic_query 的 SHA 为 `b785373284ee7aac5c786dfb0acc39ce0bff5c40ddf197f4c647c1a3d5b09b1e`。源码构建绑定记录源提交 `219c4f2ab14c55c262a46d65665889e4e7a3abb6`；该提交至上述根提交在 rust/s_session 无差异。独立对拍私有编译目录中的 store/service/finance 及 E/B CLI entry 与当前根逐字一致，finance_tests 唯一差异是加入私有观察测试 include。不是生产 reducer 返修。

已核 E native bin SHA `30e66463f55460ff7eef05a7389f2bdca0937503b68465eddf76ebef47d00525`，B native bin SHA `6c3cb2b38b83747bcf29a32870413d98c9d0b202230769187657afd57426d048`；进程 argv 与这些绝对路径、冻结 manifest、各自独立 HFS+ DB 配对。

case2 `evidence/manifest.json` SHA `bb44978f91d9f3086d244e166d706abcd5a4b4488b1b0cbb1cc6ff73191a0602`。独立标准库 canonical JSON 重算得到同一 manifest_hash，economic-config 与 HTTP snapshot 均绑定它。原 writer 仍为 test/one/unit/1；新 Q 是 testonly.economic-observe/1/Q:issue1394-economic-observer/1，只含 ReadView、Watch、QueryFact、QueryAllocation、QueryVoice、QueryDecision、QueryGrant 七项。两主体四元身份不同，coordinator_source 仍属原 writer。

冻结发生于三个新 owner init 前，见 FAULT-private.rs:309-317；并由 FORMAL-CLI-private.rs:218-227 的实际正式财务测试断言核过。HTTP 客户端头是消费者身份；Q 向 owner 使用配置中这份只读源，不能把消费者头当成写授权。

写拒绝实际负控原件来自 **case1** 的 `owner-probe-attempt2/q-write-denied.reply.json`，SHA `4e42a2f8e1f15ce39c0297bf6d4e0e02e07b769bc7f0117b5fb0738e5c3f0aa3`，该 Q 来源对 B:alpha 的 PrepareDecision 返回 `InvalidDomain：来源未获准执行该操作`。case2 的两个授权项与 case1 逐项相同，但没有再次实际发送这条写负控。case2 只读资格由配置、economic_query.py:85-98 及 Rust store.rs:330-368 的按主体与操作同时匹配门支持，不声称 case2 已独测写拒。

## 六次实际 HTTP 与内容

不是只读 RESULT 的 PASS 文本。本工位读六份原请求/响应，以独立标准库配方重算请求 payload_hash、响应 payload_hash、reply message_id 和精确 causal_refs，核完整公共头字段集、economic-session/2、session/generation、Q producer_id、economic-observe/replies 与 epoch。六项均一致。q-http.stderr 中另有六次对应 POST 的 200 记录，进程实际使用正式 s_readonly_server.py。

| 请求 | HTTP | 响应字节 | 实测耗时 ms |
|---|---:|---:|---:|
| snapshot-current | 200 | 921564 | 2722.936 |
| snapshot-token-replay | 200 | 921569 | 11.636 |
| snapshot-fixed-latest | 200 | 921569 | 2677.973 |
| watch-from-init | 200 | 1078412 | 15568.529 |
| original-decision-alpha | 200 | 16552 | 5059.297 |
| original-grant-e | 200 | 5195 | 2217.688 |

Current 是 E、B:alpha、B:beta 三域 CompleteVector，均 Available。独立核各 image_hash 等于完整 image canonical JSON Hash，并与 `observer-cli-results.json` 的原生 ReadView 的完整 image、image_hash、cut、root_hash 一致。三个最新 commit_seq 均为 4；最新 root 为 E `8feb56d48b3ec1c097b55f5ac01e18de2f9015dba1190135d429bafb994184d2`、alpha `556ec074eb237a4e48d761649e23bbbd0954666c2a68dd2f652b8af24f68dc5b`、beta `89cc24bef22721356b5a878ce58f9b7af496c5653d4e932df972eaf39083841a`。本补审比较与实际原生 cut 对应的 root，没有重新构造 DB 全历史审计。

token replay 的整个 payload 与 Current 相等；token 按去除 snapshot_token 后的完整 payload 独立重算相同。固定最新 cut 返回 AsKnown，三域完整视图与 Current 相等。Watch 从各域创世 cut 至最新 cut，每域四项非空 changes，全部变化图像摘要独立重算通过，最后图像等于 Current。

两个 Original 分别与原生 Unix QueryDecision[alpha]／QueryGrant[E 第一份 grant] 的全对象逐值相等，保留完整 cuts 向量，而非仅对 ID 或摘要。FinancialBasisEvidence 中的 ContentRef 仍是原 wire 表达；此次全对象相等不代表消费者已经拉取并展开其内容闭包。

实际 finance 都为 tb03b-finance/2，E responsibility amounts 总和 90；alpha/beta 决定均 Committed，locked_amount 为 30/40，execution_eligibility 均为 None。HTTP 观察前后记录的 seq/root/commit_ns/receipts/stage_rows 完全相同。核的是这组具名 owner 状态字段与只读合同，不把它扩大为任意外部系统均无副作用的证明。

原生财务测试日志为 1/1 pass，35.71s，财务 Prepare alpha/beta → E Reserve → B Commit alpha/beta 使用真实 Unix serve。Book/Voice/existing20 的前缀准备仍是 library/TestOnly setup，没有包装成全系统生产经营路径。

## 资源与两次路径结果

新经济 Q 的八项资源从 case1 原生只读 owner 测量形成，并在 case2 逐项保持：max_frame_bytes=1048576、timeout_ms=10000、operation_timeout_ms=30000、max_reply_bytes=2097152、max_cached_bytes=4194304、max_snapshots=2、max_domains=3、max_changes=8。case1 owner 测量最大 451637 字节、2525.782 ms、每域含创世五项变化；case2 HTTP 最大 1078412 字节、15568.529 ms，位于声明的 HTTP reply 与操作预算内。缓存、域数与变化数由此样本推导，有测量依据，不是所有边界的压力证明。

Q19 冻结文件与旧正本 `s_session/tests/fixtures/tb01c/query-resources.json` **逐字一致**，SHA `36b188ae42be8e24e7fdbb44e7d3a7068200f7c942f50975337d07595ac62e64`。它不是 Q3 的新 wrapper/limits 合同；本轮未替换其字段或上限。经济 Q 新配置与既有 Q19 各自资源门仍独立。

case1 只得到 PartialVector，三域明确 `AF_UNIX path too long`；配置短路径是 symlink，Path.resolve 后变成长 scratch 路径 129/133/132 UTF-8 字节。失败 response 与 failure.json 保留，owner 状态字段未变。case2 在 init 前使用实际短 socket 目录，resolve 到 /private/tmp 后 E/alpha/beta 为 32/36/35 字节。六次成功只支持此真实短路径部署。没有修改产品 Path.resolve 或资源来隐藏 case1，没有声称任意路径都可用。

## 清理与最终边界

两 case 各有三个正式 owner 进程 SIGKILL/wait 结果，HTTP 阶段各有 Q/E/alpha/beta 四个 SIGTERM/wait、exit=-15、reaped=true 记录。两份 mounts-state.json 共六卷 detach_exit=0，输出均为 ejected；封包中磁盘 image 摘要一致。短 socket 目录在补审时也已不存在。本工位依据退出/等待及卸载原件核清理，没有重新启动、挂载或运行任何服务。

本次限定事实 PASS 不包含任意路径保证、所有资源边界、旧结构 Q3 /api/ready、browser／#1460 前端、消费者闭包读取完成、冷 S 内容全量导入、9011 全量双跑、跨域联合故障或外效执行资格。未运行 lint/mypy 等静态工具，本轮没有产品差异可触发重复静态扫描。联合验收与 #1394 完成状态仍由总图的其它独立证据承担。
