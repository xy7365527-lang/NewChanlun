# #1419 Native 语义适配候选交付

> 工作草稿，绑定 #1419。仅完成本轮源码适配与轻量检查，未编译、未运行 Rust，未完成整票验收。

已在固定 `5cb7253d46d973669e7d10cd3331e12c9ef313b1` checkout 留下实质差异。语义来自 `634f62cddfaddf656a35ca9b9db20cb716b62bff` 的必要调用闭包；没有切换基线、覆盖其他工位、认领/评论 tracker、commit、push、merge 或开启工蜂。源码包含 40 个 Rust 文件及 6 份直接引用夹具。`SOURCE.patch` 包含全部新增文件；`SOURCE-MANIFEST.json` 逐文件保存 baseline/donor/current SHA256 与 Git blob。

## 已接入的行为

- `/2` 笔端点使用 raw_first/raw_last，成员覆盖终点极值区末根，事件代表仍为首根；同型同价保留在先者进入 evidence，不转成等待理由。`segment-second/2` 使用 donor 同一第二序列折叠与 K1 恢复核。
- generation 在同次 parser 事实上构造 C/D，再将真实段对象身份交给 E/F。E/F 使用共享中心扫描、SeedRule、CenterRoles、PairEndpoint、PairLeaf；成员区间、连续性、方向及塔 L0 对照失败均拒绝。错误收据保留 check/position/expected/actual 字段。
- 当前完整规则实际连接构造、prior、prepared、payload/links/full 来源审计、增量贡献与待定请求、22 轴目录。请求保留主体身份、对象槽、具名判定槽和精确来源；三族版本对象绑定全 raw 来源。旧规则仅可按既有历史合同读取，写者及 recover 在接受输入前拒绝旧域。
- 所有本次结构请求与等待原因收集后再建立 CC-054；目录按请求的具名轴归属，C/D/E/F 的 delivered/deferred 范围保留票号。

## Native 资源边界

新增输出数组、段映射、prior 根数组、请求键、字段文本、codec 中间数组均使用当前 `CodecVec`、`NativeValue` 或自带许可的局部包装，扩容先准入。新 typed 结构对象兼容字段的真实字符串容量先准入，许可随返回对象存活；结构贡献继续使用当前 Map/Tree 的 retained 账。K1 输出用借用序列化视图与检查点迭代，避免新增 `Vec<serde_json::Value>` 中间树；B 的 /2 端点/条件/对/确认也走同一持费事实编码核。整数仍编码为文本，bool/null 区分保留，浮点拒绝。

完整与 bound 两入口在实际 store 租约内采集结构 prior；原两段 prepared 租约、FixedRadix、SequenceMemo 失效及失败释放协议保留。新增有序插入使用 `try_insert_native`，不以普通插入绕过 Native 绑定。

资源结论限于新增投影与返回 owner。parser 的折叠状态、恢复重折叠及 center_scan 的纯算法工作区沿用 donor 普通容器；既有 B 包装路径也未整体重写。它们未被伪装成 Native retained 账。本轮没有运行真实分配/资源帽验收，不声称全生成过程的所有分配已证明有账；远端须对最终组合树验证此边界。

## 共享面与未结项

`parser/inclusion.rs` 字节未改，未碰 #1393 的初始化/全包含语义。共享接口是 `parse_layer_with_group_evidence_store`，以及 content_bi、tb02b、入口规则；根组合 #1393 时须保留其包含逻辑与本票 /2 笔段合同。未纳 #1394 99e819，未借 #1488 新 run 或 b46 财务原件验收当前变更树。

真实 raw→正式 S3/Q 的非空中心/对、修订、独立双跑、恢复、浏览器及根准入仍由同票 raw 工蜂和根完成。C…F 的 deferred、terminal/γ/K/G、≥9 升级、上级构造、趋势实现、破坏及 R5 结算按源码标记保留，不能据此宣称完整 Classification 或全图完成。K1 `from_wire` 的内容重折叠核未替换；本轮只增借用 wire 视图，不以恢复成功文本代替真实恢复运行。

## 已做检查与执行边界

`CHECK-RESULTS.json`：40 文件 rustfmt 解析/格式检查通过，`git diff --check` 通过，fixture include 闭包无缺项，inclusion 未改，K1 16 个字段映射检查通过，420 组独立 Python 极值区间 toy 通过，完整 patch 的反向只读 apply 检查通过。`COMMANDS.json` 保存实际 argv/exit/stdout/stderr；初次格式检查及检索表达式漏多行字段的失败原输出保留在 `COMMANDS-initial-static.json`，随后修正。

新增/移植的 Rust 定向锁包括：同价多根与成员边界、第二序列恢复、中心扫描/五叶对照、请求错绑与三域载荷、完整生成与 Native 返回 owner 对拍、Native 容量失败/异父插入不改值、K1 wire 编码对拍、历史库拒写无副作用。这些是**测试源码，均未执行**。rustfmt 不做名称解析、类型检查或借用检查；远端编译仍可能发现接口或生命周期错误。

`EXECUTION-CANDIDATES.json` 列精确测试命令、同树 SHA 前提和 `execution_authorized=false`。不得用本报告或旧 donor 绿灯当作本候选的编译/运行许可。
