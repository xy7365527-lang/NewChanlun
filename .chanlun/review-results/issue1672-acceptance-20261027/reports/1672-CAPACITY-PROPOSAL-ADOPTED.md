# #1672：FIN/2 TestOnly 载体容量修订提案

状态：待用户 spec 批准；本提案未修改代码、spec、票据或卷。日期：2026-10-25。

## 建议：每个 TestOnly 卷由 16 MiB 改为 32 MiB

仅修订 #1672 的 E + 3B 四个独立 UDIF/HFS+ 测试卷，合计名义容量从 64 MiB 到 128 MiB。保留现有 8 倍预留公式、cleanup/checkpoint 余量检查和全部生产资源帽；不修改业务输入、WE7、30 秒期限或 reader 的任何资源限额。批准后由实施工蜂改驱动并在精确候选上完整重跑，32 MiB 是有限的一次修订，不是已证明足够的全程容量。

选择 32 MiB 的关键理由：现有测试 manifest 的 `max_staged_bytes` 与 `max_store_bytes` 已各为 33,554,432 B；该选择无需连带扩大它们，又为剩余提交和恢复留出约一倍数据库页空间。当前读数不足以证明 20/24 MiB 的最小可行性；为省 32～48 MiB 总镜像容量反复试探，不如在既有 staged/store 边界内一次完成验收。

## 1. 证据与适用范围

- 已批准 spec：`/Volumes/AgentStorage/issue1673-fin2-spec-revision-20261025/spec-candidate.md`，§7.2 每卷 16 MiB，§7.3 不足即停；§4.4 资源预检、§14 独审与合入门继续有效。
- 当前诊断候选：`ccc115ddfb029adba8aa8fa40f65805328e58302`；macOS run：`40178218217`。
- 当前读数来自同目录 `IMPLEMENTATION.md`：6/6 Capture、3/3 Resolve、9/12 Apply；第十次 Apply m-19 在 B0 容量失败，后续三次 Apply（含失败的 m-19）、四域恢复及 21 次重送未完成。
- `FAILURE-m19.md` 属较早候选 `f43df8ab…`、run `40146774020`，只有初步错误，不与当前候选的页数诊断混为同一次运行。
- 本提案核读了上述候选 Git 对象中的 `content_store.rs`、`store.rs`、`tb03c_fin2_tests.rs` 和 HFS driver；未重跑 Rust、未创建或扩展卷。

## 2. 16 MiB 为什么只允许 507 页

`rust/src/economic_session/content_store.rs:323–342` 的公式是：

```text
P = page_size = 4096 B
C = statvfs.f_blocks × statvfs.f_frsize = statfs 总容量
Nmax = floor(C / [8 × (P + 24)])
```

24 B 是每个 WAL frame 的头，不是数据库页的有效负载；8 倍因子给失败事务与独立 cleanup/checkpoint 留空间。另有 `cleanup_room_metered`（同文件 :151–180）按 `Nmax×(P+24)+65536` 的 2/4 倍检查实际可用空间，不能只验页帽。
真实 HFS 总容量为 `16344 KiB = 16,736,256 B`，小于镜像名义 `16,777,216 B`：

```text
8 × (4096 + 24) = 32,960 B/获准页
floor(16,736,256 / 32,960) = 507 页
507 × 4096 = 2,076,672 B 数据库页上限
```

失败前 `page_count=326`、`freelist_count=0`；DB/WAL/SHM 实占合计 `1,368,064 B`。现场无可清理暂存行，cleanup 前后实占不变。checkpoint 能把 WAL 页写回并截断 WAL，不能提高 `max_page_count`，也不能删除 `commits` 中必须保留的历史图像。因此单独 checkpoint 不是本次修复。

## 3. 容量缺口：严格结论与估算分开

确定结论是：当前 507 页预算不能完成 m-19，更不能完成整轮。只从已观察到的页帽拒绝，可保守地说该路径至少需要超过 507 页；若仅增至 508 页，容量公式所需总容量已是 `508×32960=16,743,680 B`，比当前 C 至少多 `7,424 B`。这是极弱的门槛下限，不是可运行尺寸，也不代表磁盘物理写入只差这些字节。

m-19 待提交图像 `771,028 B`，原报告的整页追加计算为：

```text
ceil(771,028 / 4096) = 189 页
326 + 189 = 515 页
515 - 507 = 8 页 = 32,768 B 数据库页空间
515 × 32,960 = 16,974,400 B 获准载体总容量
16,974,400 - 16,736,256 = 238,144 B 载体总容量增量
```

**515 页是整页追加估算，不是已经测得的峰值。** SQLite 一个大记录的本地 payload 可以使用已有叶页剩余空间，overflow 页另有 4 B 链指针；`freelist=0` 不等于所有叶页无剩余空间。SQLite 格式 §1.6–1.7 给出：4096 B 页的表叶最大本地 payload 为 4061 B，overflow 页最多容纳 4092 B；该图像单独至少涉及 `ceil((771028−4061)/4092)=188` 个 overflow 页，另须容纳记录本地部分。现有汇总读数没有逐页占用和事务期间复用信息，不能把 189 当作严格的新增页数证明。

因此原报告“至少缺 8 页”应在呈现时称为**图像整页追加估算的缺口**。facts、applications、messages、receipts、索引、证据导入及恢复写入均未计全；禁止将其称为整个运行只缺 8 页。SQLite 格式依据：https://www.sqlite.org/fileformat.html#b_tree_pages 、https://www.sqlite.org/fileformat.html#cell_payload_overflow_pages 。

当前源码 `store.rs:740–754` 向 `commits` 追加新图像，不是用 771,028 B 覆盖旧的 480,103 B。不能仅按图像差值 `290,925 B` 配空间。
## 4. 32 MiB 的余量与不确定性

下表仅用于尺寸比较，假定镜像与 HFS 总容量差仍是本次的 40,960 B。新卷必须实测 `statfs/statvfs`，不能把这些估算常量当成验收读数。

| 每卷名义尺寸 | 估算页帽 | 估算数据库页空间 | 相对 515 页估算剩余 |
|---|---:|---:|---:|
| 16 MiB | 507 | 2,076,672 B | −8 页 |
| 20 MiB | 635 | 2,600,960 B | 120 页 |
| 24 MiB | 762 | 3,121,152 B | 247 页 |
| **32 MiB** | **1016** | **4,161,536 B** | **501 页** |

32 MiB 相对 m-19 图像追加估算还有约 2,052,096 B 数据库页空间。剩余 m-20/m-21 分属另外两个独立 B 卷，不能简单把三张新图像叠加到 B0；反过来，也不能把四卷剩余容量合并借给一卷。E 已完成 Capture/Resolve，但仍需恢复和审计。

不能给出全流程严格容量上界：当前缺 B1/B2 末轮候选大小、全部表与索引实际增长、各域恢复峰值以及 cleanup 可用空间读数。故 32 MiB 是首选的受限验证规格，不声称已证明足够或数学最小。它与当前 fixture 已有的 staged/store 32 MiB 上限兼容（`tb03c_fin2_tests.rs:26–42`）；生产每库 64 GiB、双跑 148 GiB、帧/文档/缓存帽均不因本提案改变。

## 5. 批准后实施与关票证据

实施方只把 #1672 driver 的每卷名义尺寸改成 32 MiB，保留 E/3B 独立设备、挂载根、HFS+、UDIF 和资源/失败语义；记录新候选 SHA、实际四卷容量、每域最大 page_count/max_page_count、freelist、DB/WAL/SHM 分配与 cleanup 可用余量。保留失败原件，另建成功运行证据。

复跑必须证明：

1. 精确最终 candidate 的 macOS CI 实跑 `fin2_we7_capture_resolve_apply_recover_and_resend` 与 `fin2_capacity_shared_carrier_and_non_hfs_are_rejected` 两个 ignored 测试；各自实际通过，不接受 skipped 或 0 passed。
2. 全部 6 Capture、3 Resolve、12 Apply 成功，逐轮 HAND 对拍，最终 B 数量 `[2,5,5]`、K `[204,506,506]`。
3. 四域按关闭→recover→重开→audit 执行，writer epoch 前进，最后提交图像逐字节一致，暂存无残留。
4. 21 次原消息逐字重送返回原收据，只有已批准的 epoch 字段合法变化；commit seq、next、messages、receipts、applications 与图像不增长/不变更，不产生第二 application。
5. 容量、共享载体、非挂载根、非 HFS 负控仍拒绝，FIN/1 WE7 相关回归不退化；四卷实际回收且原件归档完整。
6. 同 head 的相关 CI 和非作者整票独审通过，再交用户 main 合入门并核关票门。#1672 通过不等于 #1673/#1406 或全图通过，#1279 保持打开。

若 32 MiB 再次超任何页/物理/reader/期限帽，立即保存失败操作、逐域页数、表/图像大小与峰值并回 spec。**不授权继续翻倍，不放宽 8 倍预留，不删除历史，不裁 WE7，不修改输入或生产资源帽。**
## 6. 建议用户批准文本

> 批准仅将 #1672 FIN/2 TestOnly 的 E/3B 四个独立 UDIF/HFS+ 卷由每卷 16 MiB 修订为 32 MiB，并据此实施及完整复跑。保留原 8 倍页预算、cleanup/checkpoint 预留、现有 reader 全部资源帽、WE7 全输入、12 Apply、四域恢复、21 次原消息重送、两个 macOS ignored 测试实跑及完整原件要求；所有生产资源、资金、隔离与期限要求不变。32 MiB 再不足即停并回报，不授权自动继续扩卷。本批准不含 main 合入、关票或任何真实资金/场所动作。
