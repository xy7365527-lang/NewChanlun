**结论：PASS**

未发现 HIGH / MEDIUM 问题；下列 LOW 项不阻塞合并，供作者酌情处理。

## HIGH

无。

## MEDIUM

无。

## LOW

1. **L4 留档不覆盖 HEAD 的第二个提交，也没有 macOS 分支证据**
   `.chanlun/review-results/issue1666-20261021/atomic-init-tests-nocapture-73bbf098d9.log:1-3`
   日志跑的是 73bbf098d9（已核实：该提交与 ae3a38fc74 在三个票面文件上逐字一致，其余差异是基线 rebase）。但评审工蜂的第二个提交 238630af12（`fault::with` 改 Drop 守卫）在留档之后，且留档只在 Linux/WSL 跑过，`rename_einval_classification_follows_platform_manual` 的非 Linux 分支（`atomic_init_tests.rs:443-447`）没有任何运行记录。改动都是 `cfg(test)` 的测试基础设施，风险低，但严格说"留档 ≠ HEAD"。日志头还提到"编译警告段已略，均非本票文件"——CI 对 `cargo check --all-targets --features s_session` 开 `-D warnings`，若那些警告在本树真实存在会红，我无法离线核实。

2. **非 Linux/macOS 平台：stub 文案说"拒绝执行"，实际会走 link 回退；新测试在那类平台的 else 分支必失败**
   `atomic_init.rs:359-365` 的 stub 返回 `ErrorKind::Unsupported`，而 `rename_flag_unsupported`（`atomic_init.rs:313`）把 `Unsupported` 判为"回退 link"，所以 stub 消息"s-session/3 init 拒绝执行"与行为不符（这是 #1568 既有问题，本 PR 未引入）。本 PR 新增的测试 `rename_einval_classification_follows_platform_manual` 用 `cfg!(target_os = "linux")` 二分，else 分支假定 macOS；在其他 unix 上注入点不在 stub 里，会走回退成功、`unwrap_err()` panic。因全文件已依赖 `std::os::unix` 且 CI 只跑 Linux/macOS，仅记录。

3. **`fault::with` 嵌套不还原前值**
   `atomic_init.rs:243-253`：Drop 守卫把 `ACTIVE` 置 `None` 而非还原进入前的值。当前无嵌套用法，无实际影响；若日后嵌套会静默丢掉外层注入。

4. **`Linked { unlink: Err(NotFound) }` 的告警措辞会失真**
   `atomic_init.rs:158-163`：若 `remove_file(from)` 返回 NotFound（锁保护下理论上不会发生），告警仍声称"该名与库是同一文件的硬链接……请手工删除"。可忽略。

5. **告警只在 stdout JSON 回执里，`ok` 仍为 `true`**
   `mod.rs:443-446`、`mod.rs:208-209`：回执按原样 `println!` 输出，`publish_warnings` 可见；但只看 `ok` 字段的调用方不会察觉 `PublishedWithLeftover`。票面明确允许"返回成功并附告警"，故合规，仅提示该字段目前只在 `atomic_init.rs:15-17` 模块文档里有描述。

## 六个审查点逐项结论

**1. L2 成功附告警是否有误报成功的路径** — 没有。`publish()`（`atomic_init.rs:131-179`）里，旁文件残留、fsync 暂存库失败、rename 非"不支持"类错误、link 回退里 `hard_link` 本身失败（`atomic_init.rs:287` 的 `?`）全部在目标名出现前返回 Err，`published` 仍为 false，Drop（`atomic_init.rs:188-195`）删暂存。只有 `Published::Renamed` 或 `hard_link` 已成功的 `Linked` 才继续，此时目标是已 fsync 的完整库同一 inode。`Published` 枚举把"目标已出现"和"删暂存名结果"显式分离，`self.published = true` 紧跟其后（`atomic_init.rs:169`），不会误删已发布的目标。EEXIST 在 rename 与 `hard_link` 两条路都映射到 `existing_db_error`（`atomic_init.rs:164-166`），有 `link_fallback_never_overwrites_existing_target` 覆盖且断言既有字节不变。`publish_warnings` 仅在非空时加入（`mod.rs:444-446`），`build_domain` 的其余字段（`mod.rs:698-711`）不变；`out` 是 `json!({...})` 对象，`IndexMut` 插键不会 panic。调用方只有 `init_v3` 一处。

**2. 故障注入隔离** — `mod fault`、`fn injected` 与三个注入点全部在 `#[cfg(test)]` 下（`atomic_init.rs:224, 260, 266-271, 288-295, 329-338`），生产二进制无注入面。`with` 先 `set(Some)` 再构造 `_reset`（非 `_`，不会立即 drop），`f()` 作尾表达式，正常返回与 panic 展开均触发 Drop 置 `None`；测试里的 `unwrap_or_else(panic!)` 都在闭包外，闭包内 panic 也能撤销。`thread_local!` + libtest 每个测试独立线程，`init_v3`→`publish` 全程同线程调用注入点，子进程是全新进程不带注入，无串扰。可见性 `pub(in super::super)` 指向 `v3`，`atomic_init_tests` 是 `v3` 的子模块（`mod.rs:40-41`），编译通过（留档已证实）。

**3. L1 命中判定** — 命中条件（`atomic_init_tests.rs:273-276`）= 被 SIGKILL 终止 ∧ 目标不存在 ∧ 本次独占目录里有暂存残留。暂存文件只在 `build_domain` 打开 SQLite 到 rename 之间存在（出错由 Drop 删，成功由 rename 消失），三者同时成立是"init 中途被杀"的充分条件，不会恒真：子进程自然退出则 `signal() != SIGKILL`，已发布则 `db.exists()`，未开始/已出错则无残留。恒假风险低：先轮询到暂存出现（200 µs 粒度），两个 0 ms 落点加 1/2/5 ms，而建库含 WAL FULL 同步、约 300 KB 写入，远长于 200 µs；留档显示 8 次中 6 次命中。`try_wait` 已回收后 `kill` 的 Err 被忽略、`wait` 返回缓存状态，不会 panic。

**4. L3 EINVAL 归类** — `atomic_init.rs:319-323`：Linux 为 `[EINVAL, ENOTSUP, ENOSYS]`，非 Linux 为 `[ENOTSUP, ENOSYS]`；`ErrorKind::Unsupported` 在 Rust std 的 unix errno 映射里只对应 ENOSYS（和 stub 自造错误），没有隐式放宽。其余 errno（测试用 EACCES）不回退，`rename_einval_classification_follows_platform_manual` 覆盖了 Linux EINVAL 回退 / 非 Linux EINVAL 不回退 / EACCES 不回退三种。Linux `renameat2(2)` 的 EINVAL 语义与手册一致；macOS 的 ENOTSUP 说法与我对 XNU 默认 vnop 返回 ENOTSUP 的记忆一致，但离线无法对照手册（见"未核实"）。link 回退用 `std::fs::hard_link`，目标存在即 EEXIST，绝不覆盖；测试用真实 `hard_link`（只注入 rename）验证。

**5. L4 / L5** — 日志含 `S1568_NOFILE_FAILURES` 四档命中明细（SQLite 打开、读目录、设置 WAL、开 init 事务）及 SIGKILL 命中列表，符合 L4 要求（覆盖范围见 LOW-1）。L5：`existing_db_error`（`mod.rs:365-371`）新文案"中断的 init 不会留下目标文件……已发布的库或他人文件"与 `mod.rs:22-25`、`atomic_init.rs:3-17` 的"原路径可直接重试"叙事一致，也覆盖 `PublishedWithLeftover` 后同路径重试被识别为既有库的情形（`link_fallback_unlink_failure_is_success_with_leftover_warning` 末段断言）。全仓无测试/脚本依赖旧文案"请换新路径"。

**6. 范围与告警** — 改动全部落在 L1–L5；SIGKILL 延迟表从 `[0,2,5,10,20,40,80,160]` 改为 `[0,0,1,2,5,10,20,40]` 配合前置轮询，属 L1。未见未用导入/变量：`Published::Renamed` 有构造，`fault::active` 三处使用，`injected` 三处使用，三个 `Fault` 变体测试里均用到；`#[cfg(not(target_os = "linux"))]` 分支不再引用 `libc::EINVAL`，不会 dead-code。CI 只开 `-D warnings`（未见 clippy），未发现新告警面。

## 实际看过 / 未核实

**看过**：`git diff 5797d2269e...HEAD` 全文；`atomic_init.rs` 全文、`atomic_init_tests.rs` 全文、`mod.rs` 的 1-60、133-231、270-460、698-711 行；留档日志全文；`git diff --quiet 73bbf098d9 ae3a38fc74` 证实三个票面文件与留档提交逐字一致；全仓 grep `publish_warnings`/`existing_db_error`/旧文案/`init_v3` 调用点；`.github/workflows/ci.yml` 中 `-D warnings` 相关行。

**未核实**：未跑 cargo（按约束），未在 macOS 本机执行任何测试，故 `rename_einval_classification_follows_platform_manual` 的 macOS 分支与 Drop 守卫提交后的全套测试均未亲自验证；macOS `renamex_np(2)` 手册对 ENOTSUP/EINVAL 的原文未对照（离线）；留档日志提到的"编译警告段"内容未见、无法判断是否影响 CI；`session_transport::process_lock` 内部未展开阅读（假定不跨线程调用发布路径）。
