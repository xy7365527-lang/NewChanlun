# #1504 固定只读 FD / 私有 VFS 组件原型

名分：`proto/1504` 原型源码。**当前仅静态准备，Rust 编译、SQLite 夹具和全部正负控均 `NOT_RUN`。** 没有接入生产 admission、launcher 或正式 S/R4。最终运行结论须另附精确提交、远端执行收据和独立审查。

原型只回答：本仓锁定的 bundled SQLite 3.53.2 是否能通过固定 `O_RDONLY` main/WAL FD、非默认私有 VFS、`io_methods` v1 和第一次 schema 读之前的 `locking_mode=EXCLUSIVE`，读取无 SHM 的真实非空 WAL；路径换位后是否仍读旧 FD；错误与域外请求是否拒绝且保源。

## 源码分工

- `../p1504_fd_wal.rs`：具名小夹具生成、默认 VFS 独立副本 oracle、逐字段比较、路径换位、负控、资源和文件前后证据。它会在新建私有目录中写入夹具/报告，oracle 也会在其独立副本中建立 SHM；这些行为不属于只读 VFS。
- `vfs.rs`：main/WAL 捕获、非默认 VFS、C ABI 回调、初始化顺序、SQL authorizer、显式关闭/注销。无默认 VFS 文件回退、无底层源写/删/截短/同步调用，无 WAL/page/B-tree 解释器。
- `Cargo.toml`：单独 `prototype_1504` feature，不引入 `s_session` 或项目生产 lib 调用。

## 必要假设与没有证明的内容

驱动单线程，持有新建的 `0700` 夹具目录；冻结夹具之后没有其他进程写该 inode、替换初始捕获路径或添加侧文件。VFS 的虚拟锁只描述单连接 SQLite 状态，不提供进程间排他。捕获前 `NOFOLLOW`/单 link/regular file 校验和回调中的 fstat 不冒充防同 UID 并发写者。路径换位案例只证明已持 FD 不随祖先路径变化，**不关闭正式源 RW 最后复核到 pathname open 的窗口**。

VFS 源 FD 只有读权限。`xWrite`/`xTruncate`/`xDelete` 拒绝；`xSync` 为只读无写 no-op。仅 `MAIN_DB` 与已经捕获的 `WAL` 对象可打开；已有 journal/SHM 输入域具名拒绝，未知/null/temp/URI/跨域名称拒绝。ATTACH（包括内存库）、扩展、写入、临时表与更改锁模式由 authorizer 拒绝。私有注册名不修改默认 VFS。

`Registration` 的 Box 地址稳定，`Session` 借用注册对象；Statement 继续借用 Connection。先关闭 SQLite 文件，再注销 VFS，再释放源 File。若破坏生命周期而仍有活文件，Drop 泄露活对象而不释放 SQLite 的裸指针；这是故障保命，不是验收成功。正常路径要求 active=0、注销后查找不到注册名；关闭回调失败也必须向调用方报错。C ABI 有状态路径捕获 Rust unwind；实际 allocator abort、内核卡顿、OS kill 不能由此恢复。NOMEM 负控在 xOpen 的明确故障缝注入，不声称复现 Rust 全局分配器 OOM。

## 六个具名案例（全部待运行）

| 案例 | 必须取得的证据 |
|---|---|
| committed | SQLite 生成 48 条、每条 2048 B payload、1024 B page、三次提交；main 保持 checkpoint 前像；WAL-only marker 与完整行逐字段等于独立默认 VFS 副本。缺 SHM，真实 WAL 打开/读取发生在 exclusive 之后。 |
| uncommitted-tail | SQLite 真事务与 cache spill 产生未提交尾，捕获时无并发 writer 变动；默认 oracle 和私有 VFS 均只返回已提交的 48 条。 |
| bad-tail | 只翻转 WAL 最后一个不透明字节，不解释文件格式；实际 SQLite oracle 决定可见提交，逐字段比较且查询必须成功。 |
| truncated-tail | 只截去 WAL 最后 7 B，不手写期望事务；与实际 oracle 全字段比较且查询必须成功。 |
| pathname-swap | 固定 FD 后整体更名源目录，在原位置建立另一 sentinel SQLite；私有查询仍返回旧源，旧源和 sentinel 全部文件/哈希不变。 |
| denials | null/temp、真实外域 sentinel、URI、短 buffer、负/溢出 offset、短读零填、EINTR/EIO、受控 panic、xOpen NOMEM、写删截、未知 control、关闭错误、第二连接、迟设 exclusive、SQL authorizer、已有 journal 拒绝；源/外域文件不变。 |

正控拒绝“双失败相等”通过。SQL 负控除返回错误，还要求 authorizer 拒绝计数增加。手工 callback 测试只传合法的本地指针和足长缓冲区，不用提前 close Rust File 的裸 FD 制造所有权违规或 UB。

## 远端执行接口

本机不得执行以下命令。独立源码/unsafe/C ABI 审查通过后，由根绑定精确候选、沿用既有超时并受监督地远端运行：

```sh
cd rust
cargo build --locked --no-default-features --features prototype_1504 --bin p1504_fd_wal
target/debug/p1504_fd_wal suite /absolute/canonical/new/evidence-directory
```

该路径必须是尚不存在的目录，父目录已存在且规范化。不要复用生产 DB、外盘或用户数据。成功要求退出码 0、六案例 PASS、实际 source-id、源文件哈希/存在性、真实 callback 顺序和完整清理均满足；单独看到 `RESULT.json` 不能验收。失败时保留已创建夹具，外层 CI 必须上传后回收任务私有目录/进程，不能延长批准窗口。

资源报告包含 main/WAL 字节、SQLite 当前/高水位 heap、进程峰值 RSS、VFS trace 实际 capacity 与字节。WAL index 包含在 SQLite heap，未从私有内部状态单独归因；v1+exclusive+真实 WAL 读+无 SHM 的轨迹须和锁定 SQLite 源码链一起审。32,768 个 trace 槽只限定本具名夹具的证据存储，满即报错；不是全进程 heap 帽。没有通用 512 MiB/4 GiB 限额，RAM 不计作磁盘 staged。

即使六项成功，结论仍仅 `PASS_D1_COMPONENT_ONLY`。D2 hot-journal、正式 producer/R4/CLI、完整 S store baseline、同步 I/O 硬期限、正式 RW 连续性、49 GB 历史、64 GiB/store、148 GiB 双跑门与全图资金/隔离/恢复验收都保留；v4 继续 `BLOCK_NOT_MERGEABLE`，#1405/#1420/#1279 不关闭，前端 #1460，K4 不并入。
