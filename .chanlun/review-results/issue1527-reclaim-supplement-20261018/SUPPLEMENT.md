# #1527 五结束工位封包缺口增量补核

五处继续 **HOLD**；可删候选 **0**，`deleteAuthorized=false`，实际释放 **0 B**。本轮解除精确 DevSkim 成员、共享 LFS 存储副本哈希和 Unicode 小样验证缺口；没有将旧报告改成通过。

产物名分为绑定 #1527 的工作草稿/证物。固定工作 HEAD 为 `44a977568948542aa6d00ecab50e2d468e20b76c`。当前 `python-r4-worktree` 是复用补核工位，**不在五个删除候选内**。本外部会话由运行时登记为 `01a14d97-74c9-7c00-9f53-7ed4dc2ff54b`；没有声称注册成父线程原生子代理。

## 已完成且限定范围的证据

1. 精确成员 `.chanlun/review-results/issue1371-tb01-b/postmerge-devskim/evidence/rules/secrets.json`、Git blob `ed44c86593eefebbc7e1061157628e360fba203c`、2888 B、SHA256 `efbb24386803bd7a8af3ddcabfd31bfd02097863592fe2696367bd7b4a3ef85e`，五处源文件均匹配固定字节和原属性，各有新增 PAX 小包与新恢复 hash。总正文 14,440 B。`MEMBERS.json`、`member-restore/RESTORE-RECEIPT.json` 与独立 `INCREMENT-VERIFY.json` 可追溯。未打印正文，没有将豁免推广到其他同名文件，也没有因 Microsoft 比对未做而重挂此 HOLD。
2. 两份共享 LFS 对象分别为 125,404,332 B、179,067,660 B，合 **304,471,992 B**，各只做一次存储副本流式 SHA256；五份原 `CONTENT-RESOLUTION.json` 的 pointer blob/sha/size 一致。`LFS-VERIFY.json` 保留精确路径、pointer、前后 inode/size/mtime/ctime 和实读量。未复制 LFS，没有全库重 hash；首工位展开文件的源 hash 仍缺。
3. `UNICODE-TOY.json` 使用原 `Collège` 整条相对路径字节建合成小样。PAX头与 libarchive 解码/解压名称可有不同规范形态；解压后用清单给出的精确 NFD→NFC 映射经 ASCII 临时名做两步 rename，再用 `os.listdir(bytes)` 核每级原路径字节。正文与 mode/uid/gid/mtime/xattr/ACL/flags 均匹配。规范化相等不算成功，真实整棵 base 仍未重建。
4. `metadata-only` 直解覆盖合成正文，确实将文件截为 0 B。原禁令继续有效。另用 `copyfile(COPYFILE_METADATA | COPYFILE_NOFOLLOW)` 对普通文件、目录、symlink 做独立小样，正文 hash 不变、目标属性一致，见 `METADATA-REPLAY-TOY.json`。这是未来安全回放的方法验证，不是全 base 验收。
5. 经理 `reclaim-frozen/FREEZE.json` 指定的旧输出包 SHA256 为 `9a3d4f8a10f25f9c23c154974106cbba18c043dc2bf0171017ab6124abaff176`，37,124,031 B。独立核清单 **546** 项内容/链接，另列 **547** 个 AppleDouble 属性伴随项，所有旧源输出字节仍匹配冻结清单。未将仓外包 hash 当作完整 Unix metadata 恢复；恢复老输出时也须在新目录实核。

原五 HEAD/index/unique 及 tail 原三报告仍匹配。只发现已在原封包阶段产生的 tail `.chanlun/review-results` 目录 mtime 变化：原清单 1792299027898345162，当前 1792301748825448150；inode、mode、size不变，原 `PRE-OUTPUT-PARENT-METADATA.json` 保留旧值。本轮不重写它来制造“无变化”。

## 活读取依赖

定向 lsof 的 PID **97760 node** 仍打开：

- FD20：`worktree/.chanlun/review-results/issue1527-formal-recovery-20261018/IMPLEMENTATION.md`。
- FD31：`r4-candidate-worktree/.chanlun/review-results/issue1527-r4-anchor-20261018/MODEL-STDOUT.txt`。
- FD24：`r4-candidate-worktree/rust/src/bin/s_session_v3/admission_anchor.rs`（本次额外观察）。

这是“仍有打开的 FD”证据，不能据此宣称进程此刻执行 read。未中断或发信号给任何 PID。实际 argv/时间/退出码在 `ACTIVITY.json`。阴性 lsof 不覆盖未来按路径读取，**no-reader 仍为 false**。

本轮采样的在跑工蜂：

- `b1-v3-contract-run`，session `01a14d99-1f81-7040-8413-9824ce4737c1`，状态文件 `running`。
- `legacy-job-proposal-run`，session `01a14d95-52d7-75c1-8323-b5e4a6d4ff20`，状态文件 `running`。
- `python-r4-r2-review-run`，session `01a14d95-53f5-7f10-a20d-79d1c96370b1`，状态文件 `running`。
- `reclaim-supplement-run`，session `01a14d97-74c9-7c00-9f53-7ed4dc2ff54b`，状态文件 `running`。
- `rust-h1m02-repair-run`，session `01a14d95-5354-7f33-bca4-aa9248e46d19`，状态文件 `running`。
- `storage-ordinary-contract-run`，session `01a14d95-5495-7d11-a281-3fb675c4a67e`，状态文件 `running`。

活 prompt/新报告源码的定向字面引用中，本补核的 `prompt.md:16` 与 `common.py:14` 明确依赖旧 `review-tail-worktree/.../issue1527-reclaim-seal-20261018`。其他在跑工蜂在本扫描范围未见指向原五候选的具体 path；它们可经共享任务文档或未来工具调用间接读取，不能写成无依赖。父目录历史任务仍引用 `worktree/.../IMPLEMENTATION`、`review-worktree-01/.../REVIEW.md` 和 `r4-repair-worktree/.../MANIFEST.json`，清单区分活工蜂归属和仅历史文本命中。没有把历史任务当作正在执行。经理需取得活读者退出/依赖改指向独立封存物的证明，再做最终无读者判断。

## 可实际批准的最小后续范围（未执行、未批准）

精确合同见 `NEXT-ACTION-PLAN.json`；本段仅请求范围，不转授运行或删除。

| 候选 | 源普通文件逻辑 B | 本次 lstat 分配 B | 补核后仍缺源 hash 的普通文件 |
|---|---:|---:|---:|
| worktree | 1,358,696,372 | 1,377,173,504 | 8,251 |
| r4-candidate-worktree | 1,443,613,852 | 1,463,316,480 | 0 |
| review-worktree-01 | 1,443,472,714 | 1,463,140,352 | 0 |
| r4-repair-worktree | 1,443,715,859 | 1,463,431,168 | 0 |
| review-tail-worktree | 1,443,492,830 | 1,463,160,832 | 0 |

**A：只补首工位缺 hash。** 源 `/Volumes/AgentStorage/issue1527-sandcastle-manager-20261018/worktree` 固定 HEAD `3315ed1d59ba31ffbad3f0b959702eaa92f4ddbd`；只读 `SOURCE-INTEGRITY-AND-BUDGET.json` 首项枚举的 **8251** 个普通文件，总 **1,358,527,370 B**，其中包括该源的两份 LFS 展开正文。已经有 hash 的 69 个原条目和本轮规则成员不重复读。每个文件 NOFOLLOW、原 inode/size/mtime 核对、读前后签名一致，再算 SHA256 和 Git blob SHA1；LFS例外仅比固定 pointer。逐行落盘，遇变停止，不丢已完成清单。输出 `/Volumes/AgentStorage/issue1527-sandcastle-manager-20261018/python-r4-worktree/.chanlun/review-results/issue1527-reclaim-supplement-20261018/FIRST-MISSING-HASH-RESULTS.jsonl`，新输出预留16 MiB。脚本 `hash_missing_proposal.py` 已做语法检查，**未执行**；明确根批准后才使用 `--execute-approved-first-hash`。参数不是授权凭据。

**B：只重建首工位一个完整被动文件树。** 精确目标 `/Volumes/AgentStorage/issue1527-sandcastle-manager-20261018/reclaim-frozen/base-validation-20261018/worktree/files`，必须不存在；metadata-stage、git-private、旧输出恢复位置均见 JSON。同目录一次只放一份完整 base；不 clone、不 checkout、不接入共享 worktree 注册、不改 Git 配置。其余四份完整重建不在这次最小申请里。

操作顺序：

1. 核新目标空、原固定 HEAD/ref/index、小包/清单/本轮精确成员 hash。共享 Git `/Users/silencehan/Projects/NewChanlun/.git` 与 `LFS-VERIFY.json` 两条精确对象路径必须保留且只读；禁止任何懒抓取或网络。恢复原封包新输出到 JSON 指定的新目录并验546项、属性及原路径；不能再次依赖待删除的 tail 路径。
2. 按原 `source-tree0` 的原始路径字节经只读 `git cat-file --batch` 输出固定 blob，直接写新树，不运行 clean/smudge/filter/hooks。Git 返回 blob 重算对象 SHA1，恢复结果对原清单 SHA256（首工位需A补齐）。两份指针替换为已核共享 LFS 正文，读量 304,471,992 B，边写边核。精确规则从本轮小包恢复。可见 dirty/未跟踪/ignored 由原 unique包逐成员覆盖；本批 ignored=0，tracked删除=0，原patch为空，但按固定清单处理，不以空状态代替校验。
3. 原 `.git` 普通指针与原 Git-private/index 原字节一并保留作**未激活证物**，私有 index复制后核原 SHA256；对 source/index/工作树三者比较使用显式 `GIT_INDEX_FILE` 与只读共享对象库，不在新树直接运行会沿旧指针写共享库的 Git。若要求把新树激活成可用 worktree，必须另裁重绑定路径；不能同时声称原绝对指针字节未改且新位置已激活。
4. metadata-only包只解到独立空 `metadata-stage`。按冻结清单校验并恢复原始路径字节；普通文件/目录/symlink一一对应后，用已试验的 COPYFILE_METADATA 与 NOFOLLOW 回放属性，**不含 COPYFILE_DATA**。先正文后属性，目录 mtime 最后从叶到根回放。原 hardlink类=0；symlink按原目标字节，不能解引用。此批ACL全空/fflags全零，xattr值来自PAX，不能拿清单中的hash造值。若新遇未列明类型/ACL/flags，停止而不是降级。
5. 独立逐路径核正文SHA、完整原路径字节、mode/uid/gid/mtime、ACL/xattr/flags、symlink目标；原index hash/标志、dirty/ignored分类也对拍。inode/dev/ctime是源身份观测，不可在新文件系统复造；atime/birthtime虽原清单有观测，本轮小样与原验证器未证明其回放，若根要求这两项也逐位恢复须先补 setattrlist/utime小样并纳入完整验收，不能暗称已满足。文件hash、对象存在、干净status都不替代这一步。

读量与空间：首工位新树普通正文 **1,358,696,372 B**，原实际 `st_blocks*512` 合 **1,377,173,504 B**；按4KiB逐文件取整 **1,377,173,504 B**；另私有Git普通正文 **1,067,469 B**。metadata-stage普通正文0 B，预留每清单项64KiB（共 589,889,536 B）覆盖小文件/目录/xattr开销，规划新分配预算 **2 GiB**。这是保守作业预算，非已证明的 APFS 总元数据硬上界；执行器须在逐文件写前计量、遇空间不足即停，不承诺超界后轮询补救。旧源与原仓外包始终保留。

B无需读取旧工位正文：Git解码返回非LFS/unique/规则正文上界 1,054,055,378 B，LFS复制304,471,992 B、unique约 166,114 B、规则2888 B；独立目标校验再读 1,358,696,372 B。原Git pack压缩/增量解码的实际设备读量未测，不能把返回正文量叫物理I/O上界。A+B总源补读仍只有A的精确1.358GB；没有授权重读五源6.64GiB。完整B执行器尚未交付/运行，本轮小恢复脚本不能冒充它。

## 本轮输出保全与恢复条件

`REPORT-CAPSULE.pax.tar.gz` 将保存本轮新报告、脚本、五小包、验证收据及小样；`REPORT-CAPSULE.json` 给出容器和索引hash，`CAPSULE-RESTORE-VERIFY.json` 给出本地新目录实恢复结果。后两份为包外运输收据，避免自引用；经理收包必须一起保留，且核 `MANIFEST.json` hash。报告包恢复仅验本轮输出，不代表五工位完整base已恢复。

本次允许的新写只有 `/Volumes/AgentStorage/issue1527-sandcastle-manager-20261018/python-r4-worktree/.chanlun/review-results/issue1527-reclaim-supplement-20261018`。因此**没有越界写仓外目录，也不声称已完成本轮仓外保全**。经理在本工蜂退出后，将包与两份运输收据复制到 `/Volumes/AgentStorage/issue1527-sandcastle-manager-20261018/reclaim-frozen/supplement-20261018`（须新建不覆盖），复核包hash，再用 `capsule.py restore --archive <仓外包> --sha256 <固定hash> --index-sha256 <固定indexhash> --destination <仓外新目录>` 实恢复并验内容/Unix属性/原路径字节；恢复脚本所用属性读取器是旧脚本逐字副本，已随包保存，无需旧tail目录。`original_attribute_reader.py` 仅作导入属性读取器，不运行其原封包main。

旧 `reclaim-frozen` 包保全已实核；当前新输出的仓外持久化仍是实际缺项。即使全部新小包恢复通过，缺少 no-reader、首工位源hash或完整base验收时仍 HOLD；所有删除、源移动、push/merge/commit/main/关票均未获授权。
