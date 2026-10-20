# #1488 主控第二批兼容接线

日期：2026-10-17。绑定 #1488 的工作草稿；未编译、Rust 测试 NOT_RUN。

在 `resume-1488` 的 `native_owned.rs` 为 String/ContentRef 新增具体 `from_unscoped`，仅移动旧无作用域入口已经拥有的值，不分配、不授予预算资格；只读 `is_native_admitted` 供兼容路径识别已有许可。没有新增公开泛型包装器、裸移出或无条件深复制。

`q_runtime.rs` 的无 allocation 控制实例 ID 和 `q_http_input.rs` 的普通错误/无 Owned 预算文本构造转用 `TextValue::from`；没有把该接口用于 Native 主链。Q 的测试 oracle 从 `PlainJson.as_ref()` 借 Value。Owned 分支与 HTTP 错误路径判据未改。

三个文件单文件 rustfmt 检查通过。本次只有兼容接线，不覆盖其余生产消费者、源码类型检查、六成功合同或全图验收。库文件后续增补由 A 工位负责，本文不作为其最新字节指纹。

## 来源身份核上游接口

`v2/content_producer.rs` 新增 `Cc004Producer::for_source_audit`，在原初始化位置先签 profile/rule 文本和两个 Rc<()> 控制块的原结构上界，再构造只读 `SourceIdentityProducer`。wrapper 无 DerefMut，inner 在原许可之前销毁；不将它称为可变 producer 所有后续缓存的费用凭证。来源审计两个调用点由 C1a 工位替换，并删除原 native_admit，不能新旧双收。

`v3/generation.rs` 新增 `witness_identity_native`：原流式 SHA 配方提取为固定栈上20字节输出，普通 String 与 NativeValue<String> 共享同一核。Native 输出在最终字符串分配前取得原许可，随返回值持有；旧 String 接口保持兼容，不冒称其所有消费者已迁移。

这两个文件 rustfmt 与限定 diff 检查通过；未编译、测试 NOT_RUN。须随 C1a 来源消费候选独审，不能套用前一片静态通过结论。
