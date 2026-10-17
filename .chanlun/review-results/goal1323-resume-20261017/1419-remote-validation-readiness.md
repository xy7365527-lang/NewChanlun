# #1419 当前完整源码封包与远端验证准备资格

2026-10-17；绑定 #1419 的工作草稿。结论：**源码材料具备交给具名远端编译准备的资格，尚不具备声称远端已就绪、已编译或运行通过的资格。** 当前修改与未跟踪原件均未提交；现有远端 ref 无法代表这些字节。本报告没有执行 commit、push、dispatch、PR、main 合入或关票，也没有运行本机 cargo/rustc、服务、HTTP、浏览器、重型回放或访问外盘。

## 完整恢复包

目录：`.chanlun/review-results/goal1323-resume-20261017/snapshot-1419-source-v2-reviewed/`。

- 源工作树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1419`。
- 分支：`codex/1419-resume-20261017`。
- 基线 HEAD：`018c6542e5ce24d56fde354330d2571f64ffaefc`；基线 tree：`2b33aa2fa8bb715445f86faefca283f72529b21a`。
- 17 个 tracked 修改、2 个未跟踪原件，共 19 个完整文件，2,413,662 字节。文件逐一记录路径、身份、大小和 SHA256，全部位于 `rust/src/` 或 `s_session/tests/fixtures/`；不含凭据、`.git` 或构建缓存。
- `working.patch` 是相对 HEAD 的完整 binary/full-index 差异，含可能暂存和未暂存面。SHA256：`a31a80e124701e20d5364fb3d2e63f18fc4d935570ddefa41c12097524103e2b`。
- `MANIFEST.json` SHA256：`71273d6a824d11d295310c0ee327b6aaa08eb809cdec43e2614be3e98f31f0a9`。`SHA256SUMS` 覆盖 manifest、patch 和所有原件。
- 未跟踪 `source_structure.rs` 和真实 `btc-tail50k-first600.json` 均已单独复制。fixture 14,396 字节，SHA256 `0df2091a8723b040988527bcd05612c9643530f9724be68edb52acbcf77c108a`，不是 LFS 指针。
- 封包后第二次读取源 HEAD、git 状态、完整 diff 和每个源文件，均与首读及副本完全相同；`git diff --check HEAD --` exit 0。五个来源文件与 `1419-source-kernel-rereview-v2.md` 的独审指纹逐项一致。

“完整”指**指定基线 + 全部当前差异 + 全部未跟踪源码/fixture**可恢复当前候选；不指复制整个仓库，也不指整票验收。恢复者必须持有该基线对象，先应用 patch，再放入两个未跟踪原件，按 manifest 校验。不能只拿 patch，也不能只拿旧 `1419-working.patch`。

## 旧失败的当前静态状态

旧 run `38854239929` 绑定 `121949ee99959cf728408faa6b8478d9abc27935`；`ci-121949-compiler-errors.txt` 含 16 条 coded error 记录（其中 Serialize 错误重复出现）与 1 条缺 fixture 错误。本轮未重新拉取远端日志。

| 旧故障 | 当前源码事实 | 结论边界 |
|---|---|---|
| fixture include_str 文件不存在 | 同一路径真实原件已恢复并封包，但仍是 untracked | 文件缺失已本地消除；提交必须显式包含它 |
| content_structure 旧 JSON 桥：不存在的 render、DataUnavailable、Boolean、SignedInteger、错误 Object 容器、TypedObject.kind、直接 Serialize codec Value | 旧桥已替换直接 C/D/E/F 构造；当前文件使用真实 codec Value/TypedObject 工厂及 render_canonical，原错误调用形状消失 | 只能说旧报错位置已被移除/改写，不能证明新实现无类型错误 |
| content_structure 把 round_budget 的 u32 当 SecondFoldStore，恢复 clone | 当前 StructureEvidence 接收 `&SecondFoldStore`，同次 parser 的真实 store 从 generation 传入 | 旧错误路线已替换；运行正确性待验 |
| content_bi Endpoint.raw_position 不存在、把 usize 传 Option<usize> | 当前 :1754 起使用 raw_first/raw_last；:1805 将真实 raw_between_actual_extrema 转文本 | 已存在于恢复基线的修复保留，不重复计本轮成果 |
| tb02e 第十参数缺失、Result 当 tuple | 当前 :1442 同次 parser/store，:1463 传 &second_store，之后 unwrap_or_else | 静态接口形状已修，测试 NOT_RUN |
| s_structure_session 第十参数缺失、Result 当 tuple | 当前 :3323 起传 &second_store 并用 ? 传播错误 | 基线已带修复，不能宣称本轮新修 |

来源 v2 两 HIGH 已由原审限定静态 PASS；请求四轴专项亦已限定静态通过。旧早期桥报告仍是历史结论，不能冒充当前整树独审。本包并不扩大任何审查的覆盖范围。图索引不含恢复树新符号（精准恢复路径检索 0），据此使用精确源码/差异读取。

## 既有 workflow 能覆盖什么

恢复树 `.github/workflows/ci.yml` 未修改，SHA256 `98145e236cd0d8761a82971923d38bea13c7714d26571558a9bc3f159bdf557d`。其 `workflow_dispatch` 可选择已发布远端 ref；`actions/checkout@v4` 默认检出事件对应源码。普通 `codex/*` 分支 push **不会**触发本文件（push 仅 main）；面向 main 的 PR 会触发。

`rust-check` :265 的正式 S bin 步执行：

```text
cargo test --locked --features s_session --bin s_structure_session --jobs 1 -- --skip v3::financial_session
```

该步 `if: always()`，可在前序默认目标失败后继续；它编译整个 S test bin，并执行未被 ignore/过滤排除的新增结构/来源测试。它不是“只跑 #1419”：其他旧模块失败可使 job 红。#1488 Native job 的现有精确 filter 只跑自身 fields/union/path 等，**不构成 #1419 Native 完整 owner 路径验收**。来源核组件测试即使未来执行成功，也仍不等于 full/native 发布/恢复通过。

手动运行现有 CI 还会启动 Python、Lean、#1488 和 `macos-session` 等 jobs；macOS job 的职责是财务/会话读数，既有 60 分钟限制。此广泛运行范围必须被具名运行安排覆盖，不可把“已有 workflow”当成只做小型编译。现有 CI 不自动执行本票真实 full/native 发布/查询/恢复、原帽成功合同、完整历史双跑，也不替代 C6a 的专用分支协议。

## 下一步的具体门与证据

1. 主控核当前 open blockers、认领和源基线资格。当前 HEAD 是历史组合恢复基线；现役线仍为 main。只读本地 `HEAD...origin/main` 是 313/4，merge-base `a54d1ced816745748b5482229c748be05cc75749`，仅是本地 tracking 快照、**不是实时远端验收**。不能将恢复基线称最新 main；最终交付前仍需按当时 main 再整合并复核。
2. 若执行具名远端编译，先在隔离分支形成包含这 19 个文件的明确源码提交（fixture 和 source_structure 必须 tracked），记录 commit/tree 与本包逐文件哈希；任何变动产生新指纹，必要时补独审。提交与推送由主控按已有授权及具体批准门决定，本工位未执行。
3. 远端 ref 存在后核准确 commit，确认该 ref 的 workflow/脚本字节；若使用现有 CI，明确接受其广泛 job 范围。触发后核 run.headSha 和 checkout SHA 等于该源码提交，收全部编译诊断、测试逐项结果、跳过/未执行项、工具链与资源日志。不拿旧 run 的成功 job 抵扣本次未运行项。
4. 编译成功之后，仍补非空来源十关系正负控、真实生成 A…F 的 full/native 同核、完整 owner 保留旧 change/删除或替换端点、发布/查询/恢复及资源帽内原成功合同；具名小范围测试与重型全历史窗口分别记证据。不得移除资金、隔离、恢复、≥148GiB 完整双跑或其他现有门。

**准备状态：PACKAGE_STABLE / OLD_ERROR_SHAPES_REPAIRED_STATIC / REMOTE_SOURCE_COMMIT_PENDING / RUST_COMPILE_NOT_RUN / RUNTIME_ACCEPTANCE_NOT_RUN。** #1419、#1323、#1279 不得据此关闭。
