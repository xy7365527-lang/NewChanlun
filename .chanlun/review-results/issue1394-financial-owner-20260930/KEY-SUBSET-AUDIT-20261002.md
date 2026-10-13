# #1394 Subset 短键复用与同材料验收

本增补是 open #1394 工作草稿，承接已批准 C 及 #1323 后端总图。只改 path.rs 的 Subset 短键重复解码，以及 history/source_audit 的期限 opt-in 门。字段序列原本已是 DFS；现在仅在完整审计、无预算且无期限时，在当次调用保留前/当前两份实际读出并完整核验的短键。长度资格使用原 CHUNK_BYTES，核 STRING、唯一空/单叶分解、真实块长度和全部 UTF8；长键仍走原严格流式比较，不加资源数字。未选值不展开，全部业务、来源、索引、clock、三次完整 audit 保留。

Exact、默认/普通 lookup/fragment、预算 Some 保留原路径；期限 Some 明确关闭局部优化，恢复逐项访问。原单独 Subset 比较可在首差早返，新 None 合格短键会更早发现畸形；完整审计本已要求独立全闭包，不声称单访问器首错或瞬时错误触发位置全等，不以访问器代替完整证明。

51项定向测试报告通过，Root已核固定源码、日志、补丁与构建绑定，独立概念/Spec/Standards通过。合法两夹具节点读分别199→100、244→130。新增 Some 测试曾误将无预算同素材复用的F次数写成199；实际 SQL199/F91，预算Some SQL199/F398。失败原件保留，修正仅该测试参照，并新增成功断言及完整 old/new 返回、digest、F、预算/错误/期限轨迹对拍。完整Some fixture仍151351真实读取/F170797及原指纹。原 state.rs:60 Clippy deny继续阻断，未加allow/删测试/ignore；check、release build、fmt、diff通过。

同raw28/cut27/AcceptedPending1三文件副本只读两遍。Root逐类型重新计算完整四成员及每cut全部字段全等、指纹42444317d86a03e066867bdfccfd0bb9f07010a1a2655483fdc377b7a115b8a0、963阶段顺序及每阶段业务overlay全等。每遍SQL 7753079→6977348，少775731，减少10.01%；F3400571、unique66953不变。两遍观察10.138091917/10.036378500秒，不将跨时负载差异归因；首遍与来源定向测试曾并行。实际S请求内部耗时、严格heap/RSS/IO峰值和全9011仍未证明。原输入三件bytes/stat不变；副本SHM时间变化如实记录。

非test release CLI已由移除诊断后的固定产品字节构建并保留，SHA c133ed89d9fda4c332f8c5730c53393fcd50ceca4a97f28b593b6530900bbe43，源码/补丁/编译与原HEAD构建依赖绑定见JSON。Root集成的Rust子树为5da6fc6caec9cb6a80569ff9497a2a5e461e5e12。诊断已移除。工蜂实际退出0、PID消失，原生管理者接回后停止；未合main、未push。

一次收齐必要报告、冻结输入与外部大实体SHA。独评读取时的CHECKS原件在最终追加build/fmt/diff机械收据，原审查快照已按旧SHA精确重建冻结，原审查产品及结论未变。稀疏工位私有bridge只允许固定root/HEAD/rules和真正缺席的excluded S项，其余隐藏flag拒绝；共享bridge不动，构建依赖均为HEAD原字节。

当前局部减读不证明30秒服务域、全9011双跑、crosswalk、联合故障/资源负载、Client真实allocation、C或六Destination。下次正式验证须冻结新Root HEAD/子树/最新CLI、原完整源/clock/S7/Q21与全新物理域，并满足原6GiB空闲启动门；不重投或删除原pending28。#1460前端仍独立。
