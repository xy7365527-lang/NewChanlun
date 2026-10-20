# C6a 分页实施核验补记

日期：2026-10-17。范围：已批准的固定 cut 字节分页协议；仅核验恢复树 `resume-1406-c6a`，不代表 #1323 全图或 #1279 可关闭。

## 已执行并通过

- `python3 -B -m unittest discover -s s_session/tests -p 'test_c6a_*.py' -v`：41 项通过。覆盖四种逻辑文档、跨页重组、UTF-8/转义边界、页/描述符/片摘要、缺页/重叠/重复/非法 offset、4 MiB 上限、Close 失败、根截止、snapshot token、watch 连续性、cache pin/淘汰、终态 Open 防复活。
- 在 `s_session/tests` 目录执行 `python3 -B -m unittest test_tb03c_query_runtime_contract.py -v`：62 项通过。覆盖外部时钟、共享期限、Q 启动/重试、supervisor、输入冻结、WE5/WE7 账及公开验收门。
- `python3 -B -m py_compile s_session/s_socket_client.py s_session/economic_query.py s_session/economic_http.py`：通过。
- `git diff --check`（owner/Q/client 变更文件）：通过。

## 本轮静态结论

- Q 的 `EconomicReadError` 保持版本化错误 payload；客户端页核心只接受精确的 `EconomicReadOpened/Page/Closed`，错误不会变成完整逻辑文档。
- Rust owner 的续页重核 session/domain/epoch/manifest、固定 cut/root；Close 对活动、未知和已释放 transfer 按合同处理，Open 失败保留终态摘要防止同消息身份复活。
- HTTP `/api/economic/read` 计算完整 reply envelope（含公共信封）和原 deadline；超帧/超时不交付成功。
- 旧单帧 snapshot/watch 入口仍保留其旧版本受限检查；分页路径没有把旧单帧失败改成宽松 fallback。

## 尚未证明

- 未运行 cargo/rustc、真实 Rust owner、Q HTTP/Unix socket 或完整 supervisor。
- 未运行完整真实 41 步产品 wire、独立 semantics 对真实产品、历史回放、恢复、双跑、RSS/磁盘/4 MiB/1 MiB 资源验收。
- 因而不能称 C6a 已完成，更不能称 #1323 全图完成或关闭 #1279。

冻结合同 SHA256：`c6a-pagination-contract-v1.md` = `93a3ffedfa21e72589fc90163855361b8d44a75db1e977b30f20811824428d83`。

## Rust owner 精准测试补记（2026-10-17）

在恢复树 `resume-1406-c6a` 的 `rust/` 下，先确认 `economic_session` 由 `s_session` feature 门控。未带 feature 的两次过滤命令各返回 0 tests，不能算验证；改用正确命令：

```bash
cargo test --features s_session --lib economic_session::service::read_page_tests -- --nocapture
```

结果：9 passed、0 failed、3380 filtered out，耗时 0.86s。覆盖 RFC4648 页面编码、UTF-8 字节切片与固定 cut 复核、分页/观察原件保持、跨页大文档帧界、Open/Page 重传不续租、Close 幂等及释放、身份/offset/cut 失败关闭、共享预算与 tombstone、ACL 隔离。命令首次编译 `s_session` 测试配置成功；Rust 编译共显示既有 warning，未见编译错误。

`git diff --check` 通过。该结果只证明本机 Rust owner 单元合同，不是 E/Q/Q HTTP/Unix socket/客户端/supervisor 跨进程链路。真实 driver 明确要求 Linux GitHub Actions、push 触发、clean 源码基线与完整源 SHA；当前宿主为 macOS，候选树含本轮工作改动且有未跟踪准备件，`verify_runtime` 仍为 PENDING，因此不能在此把本地测试包装成完整产品运行。先前记录中“未运行 cargo”已由上述命令更新；其余未证明项保持不变。
