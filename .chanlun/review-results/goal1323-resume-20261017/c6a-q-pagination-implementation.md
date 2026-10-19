# C6a Q 分页实施记录

工作草稿，绑定 #1406 / #1323；2026-10-17。用户已批准固定 cut 字节分页提案。仅在恢复树 `resume-1406-c6a` 实施，未改主树、资金写路径，未启动真实 HTTP/socket/服务、Rust 编译或远端运行。

## 当前切片

- `s_session/economic_query.py`：共享缓存按实际完整 canonical 逻辑正文计费，含 `snapshot_token`；token 摘要仍基于原 body。snapshot 与活跃 watch 共用原数量/字节帽；活跃传输 pin 防淘汰，无法准入时先拒绝，不部分淘汰。过期 watch 释放，snapshot 保留原 token 缓存。
- 内部 snapshot/watch 接受传入绝对 deadline 并取较早者，准备分页时不重开逻辑窗；旧单帧入口继续检查原帧帽。
- 专属测试 `s_session/tests/test_c6a_economic_pagination.py`：当前7项纯内存测试通过，覆盖真实正文计费、原token、共享数/字节、pin失败原子性、watch释放、old_fixed token、边界和绝对截止。

## 尚未完成

等待主控冻结版本化分页字段后接 Q 三操作、HTTP 路由和 owner 重组客户端。以上仅内部准备通过，不是分页实现完成或真实 41 步验收。
