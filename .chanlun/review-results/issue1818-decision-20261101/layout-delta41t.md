# #1818 / PR #1826 增量独审：BLOCK

- 精确 head：`07fb67f116c002f6604214f10ddebb55e58a6500`；基线：`163fb363a290ecf210a265058f1af71105ca73ce`。
- 只读工位：`/Users/silencehan/.codex/worktrees/layout-publish41e/NewChanlun`。本地 HEAD 与 GitHub PR head 一致；差异仅 ADR 0059。
- 41R 的普通 E/B socket 反例已闭合；新阻断是 socket 最终悬空链接在 macOS 的实际创建语义。它仍属于已经纳入的 E/B bind 路径，不是扩大未知足迹范围。

## F1 / P2：Q₄ 把 Linux 式最终链接语义误当成跨 Unix 事实

位置：`docs/adr/0059-production-session-layout-contract.md:63`、`:77`、`:107`，影响 `:108–111` 拒绝集合和验收 `:140–146`。

ADR 声称最终条目存在（含符号链接）必报 `EADDRINUSE`，因而只算 `Q₄ = canon(parent(k))`。Apple 发布的 XNU 源码明确相反：bind 会跟随最终符号链接，悬空目标可在其目标父目录创建 socket。

### 可复查的静态证据

本次读取 Apple 官方仓库，`git ls-remote` 将 main 固定为 `f6217f891ac0bb64f3d375211650a4c1ff8ca1ea`，随后按该 commit 重读以下片段：

1. [uipc_usrreq.c:1189–1236](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/uipc_usrreq.c#L1189-L1236)：`unp_bind` 以 `NDINIT(... CREATE, OP_MKFIFO, FOLLOW | LOCKPARENT, ...)` 调 `namei`。只有解析后 `vp != NULL` 才报 `EADDRINUSE`；目标不存在则对解析后的 `dvp` 调 `vn_create`，类型 `VSOCK`。
2. [vfs_lookup.c:154–160](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/vfs/vfs_lookup.c#L154-L160) 明写 FOLLOW 控制最终路径分量的链接跟随，其余分量一直跟随；[:1040–1044](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/vfs/vfs_lookup.c#L1040-L1044) 实际设置 `ISSYMLINK`；[:1601–1609](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/vfs/vfs_lookup.c#L1601-L1609) 说明创建目标不存在时返回 `ni_vp=NULL` 并保留目标父目录。
3. 本 head 的 `rust/src/session_transport.rs:634–636` 直接 `UnixListener::bind(socket)`，不预先拒绝最终链接、不 unlink。Rust 标准库 [listener.rs:76–114](https://github.com/rust-lang/rust/blob/master/library/std/src/os/unix/net/listener.rs#L76-L114) 把路径装为 sockaddr 后直接调 `libc::bind`。
4. 本 head 的 `economic_session/store.rs:157–183,281–296` 只要求 socket 绝对路径及互不重复，并按 chong_id 选 B 路由；`service.rs:1500–1524,1560–1581` 没有 socket 链接拒绝。`production_session.rs:455,467,480` 在 S 后立即拉起 E/B，不等 S ready。

这是 Apple 已发布实现的静态证据，不是本机并发实测。本机为 Darwin 25.6.0 / xnu-12377.161.14；该精确内核 tag 未能从公开仓库取得，故不声称上述 commit 等于本机二进制。合同没有 Linux-only 限定，且经济域显式包含 macOS 分支，不能用不分平台的“不跟随”断言排除该创建落点。

### 静态反例

P=`/abs/s`，A=`/abs/s-state`；所有库与锁在安全目录。经济 manifest `e.socket=/abs/e-run/alias.sock`，它是预置悬空链接，指向不存在且非 S 族名字的 `/abs/s/e.sock`；各 B socket 不重复。目录与权限均正常，无并发改链接、无 TestOnly、无需请求。

现规则下 Q₁/Q₂/Q₃ 安全，Q₄=`/abs/e-run` 也安全，launcher 放行。按上述 XNU 创建链，E bind 跟随 `alias.sock`，在 P 中创建 `e.sock`，而非对 alias 条目报 EADDRINUSE。若发生在 S 固定 stamp 与打开前复核之间，P 的 stamp 改变，仍可触发原 `SourceIdentityChanged`。现验收 13–16 只覆盖直接目录和父目录链接，全部通过仍抓不到最终悬空链接。

**最小修订建议**：原 owner 修正平台语义；保留 socket 词法父目录，并将最终链接解析后潜在创建目录纳入同一 M 拒绝集合，例如同时检查 `canon(parent(k))` 与 `parent(canon(k))`，canon 沿用已定义的悬空链接规则。不要为此新增宽松兜底、清理 socket 或改启动排序。补安全目录中的 E/B 最终悬空 socket 链接指向 P/A 的启动前拒绝验收，点名 socket 字段和 S 字段、目标未建、任何域文件未建；保留安全 socket 与 s.socket 正控。若选择平台特化，须说明支持平台和实际 bind 语义，不可继续写成所有 Unix 相同。实现与运行证据仍归 #1781。

## 本轮已通过与复用

- 原 41R F1 的直接 P、A、族目录和父目录链接反例现在均由 Q₄ 命中；B 按 chong_id 映射而非数组序号假定，符合 `Config::load`。错误字段及 load_manifest 返回 Ok 前只读检查可实施。
- P 下既有非族、非 A 子目录不命中；s.socket 明确不进入 E/B Q₄，S 在 driver open/finish_startup 后 bind 的证据保持成立。
- 未变 Q₁/Q₂/Q₃、L₁/L₂ 与 bundled SQLite Unix 侧件结论复用 `/Volumes/AgentStorage/layout-review41r/REPORT.md`，保留其 macOS/Linux O_NOFOLLOW 和默认 VFS/编译配置适用范围。
- R2/R4/R5/R6/R7 复用 41R 的未变结论。当前 F1 是技术事实修正；不因发布返修次数自动转 P，不改已有 owner 与依赖边。
- TestOnly marker、SQLite 临时文件、金融读子进程、未追模块仍是 ADR:33–39 的未核项；本轮没有把它们另立为阻断。

| 项 | 裁决 | 本轮依据 |
|---|---|---|
| R1 引用与事实 | BLOCK | “最终 socket 链接不跟随”的跨平台断言被 XNU 源码推翻。 |
| R2 候选与权威 | PASS | 未变，复用 41R。 |
| R3 二值验收 | BLOCK | 新 13–16/18–19 可实施，但缺最终悬空链接反例。 |
| R4 落点与政策 | PASS | load_manifest 与 #1781 落点不变，未引入资金/隔离新政策。 |
| R5 单一判据 | PASS | 同一 M 拒绝集合，无宽严兜底；缺的是实际创建目录。 |
| R6 图与 owner | PASS | 本轮未改边，复用原审，不声称重新核了 live 全图。 |
| R7 T/P | PASS | 复用 41R 对 land_t 后发布返修的结论。 |
| R8 考古 | BLOCK | 已纳入 bind 路径的最后一跳在 macOS 判错；不是要求穷举所有未知模块。 |

## 检查与交回

`Prove It Works`：回查精确 head 源码映射、调用顺序与 Apple 创建调用，静态代入拒绝集合；未运行 Rust、CI、socket 实验或并发复现。机械 head/干净状态不作为语义通过证据。纯 ADR 无新增代码注释，no-comments 不适用。只写本目录 REPORT.md 与 review.json。请主控把 F1 交原 owner 修正文书和验收后，再审新精确 head；本轮不是 #1781 实现或运行验收。
