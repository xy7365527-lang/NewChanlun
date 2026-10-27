# #1488 publication 的 raw 父账路由核对

日期：2026-10-17；只读静态核对，Rust NOT_RUN。工作树 `resume-1488`。

`audit_owner.rs` 中旧 `with_bound_raw_advance` 仍包含 `Budget::new`，不能仅凭该函数存在判正式 publication 又建了第二父账。当前 `AuditConnection::publish` 在 `complete.is_some() && context.source_cache_enabled()` 时签发 `PublicationResources`，其 budget 克隆的是 `candidate.resources`；在该资源的 `work.run` 内把 `Some(&parent)` 传入 `publish_body`。后者以相同条件签 raw advance，走 `with_bound_raw_advance_in_publication` 的 Some 分支。None 的旧支路及测试直调仍存在。

这只解释当前正式 publication 选择父账的控制流；不证明冷候选、恢复、跨事务 previous、跨请求池或新 token 与旧 charge 之间都已正确交接。没有修改旧方法或将其报成已删除，也没有把 raw 新建事实三轮测试当作 warm restore 证据。后续 owner/SQL 整体迁移必须继续审这些边界。
