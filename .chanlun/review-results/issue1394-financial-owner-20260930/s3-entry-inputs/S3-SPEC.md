# #1394 S3 正式入口增量 Spec 复核（revision1）

**PASS：本增量合同反例 0 项。** 私有 SHA 通道闭合已报告的“child 加载被修改七值后，文件恢复原值而 parent 不知”的绑定缺口；本结论不表示真实服务验收。

- 最终候选：`46d183a20afe2c68e01e26555531663a7810f065`
- 增量基线：`b86c0c291952879af67882e79d9687f51ce1741f`
- 完整基线：`7d6dd63aeb6bb6861a9b91d794225d93818ad317`
- tree：`0c261fb81f8d45acb2442453f75c40f2edc1c09c`
- 增量 diff SHA：`c7b3ecc587ea94e849784d31d79baa6a1dc75137a6a2c3a4fefa920a2fd6ded9`
- 完整 diff SHA：`a32c03adb35e8e92798e0c413f325314fa8939d1aa62f27c24bbb2906f0e6b62`
- 新冻结 binary SHA：`5437fbb5d581d1b48f6a1d3405cae67327113e5a2c7fedd7ce3a1d9ce1e18e91`
- 作者结果 SHA：`85a96ac74bbd9b7b9fd6b2a8b901c18ab1b9109f85dacc9d44c7bd24486a8591`
- source manifest SHA：`8a331bebd86b998090fdb8926c1dfd3c618f025e66e641eb7f2f133ae8340abc`

两份 patch 均与现场相应 git diff 字节一致。新旧 manifest 无增删，仅指定三个文件变化，其 SHA 已现场重算一致；其他 571 项与旧已核 manifest 相同，本轮未重扫全树。新 binary、旧 binary archive、API、patch、manifest、作者全部日志 hash 均核对一致。旧 SPEC-REVIEW.md 保持 SHA fa0622e3…，没有覆盖。批准 C 源不在增量内，沿用上一轮实核 fab912c6…771d30。

| 合同 | 核验 |
|---|---|
| 只由本次已核身份生成私有声明 | PASS — s_service_control.py:408–427：先拒绝活旧PID，再 validate_resource_identity(...require_current=True)，复制环境，清除 ambient S_CONTROL_RESOURCE_SHA256，仅有资源身份时注入其 SHA。原环境 dict 不修改；原 nonce/argv/deadline 保持。 |
| 实际加载值与声明绑定 | PASS — writer_cli.rs:437–474：SResources::load 生成规范 digest；比较同一个 SResources 后，transport_bounds 与 audit cap 均从该对象获取。文件在 child load 后恢复原值不能改变内存值；被换成不同七值时规范 SHA 不同，明确拒绝。 |
| 非法/缺失声明的精确语义 | PASS — expected=None 仅保留 standalone CLI 原显式资源文件合同，不默认任何值。有声明则严格 64 bytes lowerhex，空串、长度错误、非十六进制、大写拒绝；std::env::VarError::NotUnicode 明确拒绝，未以 .ok() 吞掉。规范 JSON 等价格式共用 digest。 |
| 拒绝先于服务副作用 | PASS — serve 内顺序为资源 load→读取/核私有声明→ClockPlan load→checked_profile→原七门→Driver open→identity→serve_unix。ServeOptions::parse 先做原有路径/flag只读校验；声明错误不会到持锁、DB open/write 或 listen。此项为源码与 helper 证据，本工位未运行真实 CLI 负例。 |
| 旧2/Q/Economic兼容 | PASS — start_s 旧2分支、start_q、economic_service_control.py:92 的 start_owner 都不传 resource_identity，共同清除该私有键；Economic 继续使用 SESSION_CONTROL_INSTANCE_ID，原命令/配置/nonce字段不变。不要求其配置具有 S3 七字段。 |
| 无公共或资源扩域 | PASS — 增量仅3文件95行；没有新公开JSON字段、CLI flag、端点、默认数字、量纲或业务条件。Driver/state/原七门映射不变，保留上一轮复核射程。 |

## 验证与限制

独立执行新增 Python 环境来源/清理 unit：1 passed，0.003 秒，退出 0；该测试 mock child，覆盖 S3 注入已核 SHA、ambient 覆盖、旧2不传、原环境不被修改及 record SHA 相同。没有启动 S3 服务。

新作者日志逐份核 SHA 与摘要：Controller 42 passed；Rust writer 3 passed/0 failed/0 ignored；non-test check/build 均有 Finished，作者记录退出 0。本工位未重跑全组或 build。旧 Driver19/state7 源文件无增量，保留原证据射程，不将其记作本轮新执行；上一轮零测试、ENOSPC、夹具断言失败与后续通过记录全部保留。

非UTF-8拒绝及 before-effects 顺序由当前源码明确验证；作者真实 CLI 负例脚本只经语法检查，本工位未执行，不把脚本存在视为实测通过。Standalone CLI 无私有声明仍须显式合法七字段资源文件；该分支不是对错误声明的 fallback。

旧生命周期、完整审计与第七资源零跨请求保留沿用上一轮未变部分，本轮不重新裁量其口径。不声明真实socket七门、生产容量、9011、整图、C09条件执行、Standards评审或main合入完成。

机器记录见同目录 SPEC-REVIEW.json；旧报告保留在上一级。
