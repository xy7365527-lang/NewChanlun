# #1394 完整数组线性枚举候选

完成局部候选；未提交，未操作 tracker、S/Q、恢复或 9011 验收。Root 须独评后重新 freeze。

固定 HEAD：e4ea36fbd16c3784b846092b6c28b976ff6343a5。基线 Rust tree：7196a77fef56cf87cbe91dc4b135326a142e2395。
候选 Rust tree 虚拟 SHA：c60452f606202ed6b4754ce29bd17698d115ed32（只计算，未写 Git 对象）。
完整差异 SHA256：8e67ee4fe48625e61e83a6bb56fe18743774784df40c908db7cfe554222034c9；见 CANDIDATE.patch 和 CANDIDATE-BINDING.json。

生产仅五文件：s_content/path.rs、s_content.rs，以及 s_session_v3/history.rs、source_audit.rs、object_index.rs。
测试新增 tests/s_content_array_scan.rs，并在 history.rs 增补两个定向测试。无其它生产差异。

新增 doc(hidden) Rust API path::lookup_array_items(source, reference) -> Result<Vec<ContentRef>, CodecError>。
单次 DFS 栈只保存 digest/count/base_index，每个逻辑位置独立核序列 count/k 和实际值头；无 body map、不跨调用复用。
返回成员 Ref，不展开值闭包。普通 lookup_path、fragment、公开 wire 不变。
NodeSource::audit_full_array_scan 默认 false；history 仅 read_indexed_history_for_audit 的 BudgetNone 显式 opt-in。
source_audit 的完整来源审计在原 sc::verify 之后局部选择 BudgetNone；Some 强制旧逐 Index 路径。
对象成员和形态来源数组仅在明确 opt-in 时完整枚举，Some 继续原读取/业务检查交错顺序。

新/旧 root[] 均使用 Value 角色，不能从裸 Ref 推测 JsonInner。公开 0x08 与 JsonInner 的 0x07/0x08
拒绝仍由原上下文路径和上层完整 verify 执行，没有用 header 代替闭包。

## 定向验证

58 个不同测试通过、0 失败、0 忽略：array_scan 4、path 11、fragment 12、history 19、source_audit 4、object_index 2、audit_budget 6。
新反例先失败：n=16 旧枚举 256 次读取，线性要求 48。修复后 0/1/3/7/16/65/257 元素逐 Ref/order/kind/extent 对拍。
复杂度锁：n>0 实际读取恰为 3n；空数组实际读根与空序列 2 次；嵌套值不展开，重复子树按逻辑位置读。
真实 SQLite n=65：旧 1607、新 195（独立两次）；同事务修改 body 或删节点后，仍 NODE_DIGEST/NODE_MISSING。
Some 实际 SQL/F/预算门事件逐项与旧循环相等，包括 warm 源、双 F、前/后门拒绝和期限先于预算/SQL。

rustfmt 只处理新差异块和新测试文件；新测试 rustfmt --check、git diff --check 通过。
标准定向 Clippy 被未修改 state.rs:60 的既有 absurd_extreme_comparisons 阻断，不能宣称标准 Clippy 全绿。
另做单独禁用该旧 lint 的差异诊断运行通过，没有修改生产规则；原失败日志保留。

## 同材料 pending 只读 pilot

管理者提供副本包含 27 已发布 cut、28 raw、1 AcceptedPending；再复制为局部 query_only 输入。
基线与候选各三次完整 state::audit，均通过。每次节点 SQL：8792023 -> 8090429，减少 701594（7.9799%）。
每次 F：3402722 -> 3400571。返回根/raw/对象身份指纹与全部 963 个阶段事件顺序逐次一致，overlay 读/写计数一致。
每次保留 27 个来源闭包、来源核对、目录核对阶段。提供副本和局部输入哈希前后不变。
完整原始样本、阶段、输入及二进制绑定见 pending/SUMMARY.json、candidate.log、baseline.log、build-binding.json。
SQL 只计 content_store 节点 SELECT，不是全部 SQL 或磁盘 I/O；F 包含读核、写入与派生 hash。
耗时仅 debug/cfg(test) 观察，OS 缓存/外部负载未控制。Some 的全 pending 审计未测，仅其定向读取与门序对拍已过。

另有归档 S16 局部观测：每次 3082463 -> 2939779 节点 SQL（4.6289%），见本目录 SUMMARY.json；当时已无 pending。

heap/RSS 峰值均 UNMEASURED；新增对象索引成员 Ref 向量的临时内存也未做峰值测量。
未运行真实 S/Q、pending 恢复、Root 新 Controller freeze、全 9000/9011 重放或资源/端到端验收。
