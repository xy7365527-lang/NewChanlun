# #1456 交付封包预检独立审查

结论：**PASS_BOUNDED_INDEPENDENT_REVIEW**。本轮范围内四项原问题均已修复，独立五例运行得到预期结果；没有剩余可操作的阻断发现。本结论不构成合入授权。

审查者：`/root/delivery_bundle_preflight/preflight_independent_review`。工作树 W6：`/Users/silencehan/Projects/NewChanlun-1456-sandcastle`。

## 最终文件绑定

记录时间：2026-09-13T21:52:49.293Z；当时 HEAD：`abf5c0f8491fc066a0f2bc47ab4defdb120035c1`。

三文件仍为 untracked 新生实现；此处绑定文件字节，不声称它们已在该 HEAD。

| 文件 | SHA-256 |
|---|---|
| `.sandcastle/delivery-bundle.ts` | `cc4d43e45a8e32eaacfed939263f6133acc9e305dacf57b64600888167a02d20` |
| `.sandcastle/delivery-bundle.mts` | `ef3dfa70769cc6e8479a1ab77a8494404029d10a8d80acc677b1e6ba34c74b5c` |
| `.sandcastle/delivery-bundle.test.ts` | `571bfdbed8c91c3585cf16c85292d979c56c78b024138dd3179cf3e9f67c47bb` |

绑定说明：绑定报告生成时 W6 三文件的最终字节。此前五例执行时未同步留存源码 SHA-256，故不声称具有当时原子指纹见证；本次复读最终修复和测试，并按指令复用原结果、不重跑。

## 四项原问题与修复核验

### IR-01 · HIGH / P1 · 未参与交付 diff 的产品可被索引跳过标记隐藏脏改动

位置：`.sandcastle/delivery-bundle.ts`，行 141、143、144、145。

原问题证据：原实现仅逐字节验证 actual productDiff；首尾 git status 不能发现未变更产品路径上的 assume-unchanged/skip-worktree 脏改动。本项修前未单独保存运行输出。

修复：全索引 ls-files -v -z 检出小写 assume-unchanged 或 S skip-worktree 即 INDEX_FLAGS_HIDE_DIRTY；不修改索引。

独立核验：`hidden_unchanged_product` 返回 **BLOCKED**；状态为 `FIXED_AND_INDEPENDENTLY_VERIFIED`。

### IR-02 · MEDIUM / P2 · core.filemode=false 隐藏未改产品文件的执行权限变化

位置：`.sandcastle/delivery-bundle.ts`，行 124、141、311。

原问题证据：helper.js 在 HEAD 为 100644，工作树 chmod 755 后设置 core.filemode=false，status 为空，修前结果 READY_FOR_APPROVAL。该命令批次之后的另一项试验失败，不作为此项结果的通过依据；此项已单独输出完整结果。

修复：所有 Git 调用强制 core.filemode=true；同时关闭 core.fsmonitor，以真实状态作为冻结检查依据。

独立核验：`hidden_executable_bit` 返回 **BLOCKED**；状态为 `FIXED_AND_INDEPENDENTLY_VERIFIED`。

### IR-03 · HIGH / P1 · diff.ignoreSubmodules=all 隐藏已提交 gitlink 变更

位置：`.sandcastle/delivery-bundle.ts`，行 141、151、154、155、161、311。

原问题证据：真实临时嵌套 Git 仓库提交子模块更新后，默认 diff 隐去 submod，显式 diff.ignoreSubmodules=none 能看到 submod；修前预检仍 READY_FOR_APPROVAL。该批次之后的非 UTF-8 文件创建被 macOS 拒绝，不计为任何发现；子模块项已输出完整结果。

修复：status/diff 显式 --ignore-submodules=none；非 blob 对象及 gitlink 明确 UNSUPPORTED_PRODUCT_FILE，不作自动就绪判断。

独立核验：`hidden_committed_submodule` 返回 **BLOCKED**；状态为 `FIXED_AND_INDEPENDENTLY_VERIFIED`。

### IR-04 · HIGH / P1 · refs/replace 替换真实 head 树并隐藏未评审源码

位置：`.sandcastle/delivery-bundle.ts`，行 124、147、158、161。

原问题证据：base→benign→rawHead（新增 hidden.js），随后 git replace rawHead benign 与 git read-tree --reset -u benign；HEAD 仍 rawHead，status 为空，普通 diff 看不到 hidden.js，而 git --no-replace-objects diff 能看到；修前预检 READY_FOR_APPROVAL。

修复：统一 Git 入口加 --no-replace-objects，ancestry、ls-tree、show、diff、status 均对原始对象执行。

独立核验：`replacement_hides_product` 返回 **BLOCKED**；状态为 `FIXED_AND_INDEPENDENTLY_VERIFIED`。

## 已执行的五例独立验证

执行目录：`/Users/silencehan/Projects/NewChanlun-1456-sandcastle`。

实际入口命令：

```sh
./node_modules/.bin/tsx "$review_dir/repro.mts"
```

临时 repro.mts 由独审者编写，内容保存在同目录 [PREFLIGHT-INDEPENDENT-REVIEW.json](PREFLIGHT-INDEPENDENT-REVIEW.json) 的 verification.scriptSource；脚本 SHA-256：`d0aad10dfb49f9072d41b2b8a928f35410ae89669ea24d95abff4e4735add285`。该 JSON 同时保留创建/执行/清理脚本的命令模板。报告编写阶段未重跑或扩测。

真实工具输出块：`761bd8`；命令批次返回 exit=0。工具未单独保存 tsx 子进程退出码，结论依据完整五行实际 stdout 逐项核对，不能仅以清理命令的返回码作为测试通过依据。

```jsonl
{"case":"clean_control","status":"READY_FOR_APPROVAL","codes":[]}
{"case":"hidden_unchanged_product","status":"BLOCKED","codes":["INDEX_FLAGS_HIDE_DIRTY"]}
{"case":"hidden_executable_bit","status":"BLOCKED","codes":["DIRTY_WORKTREE"]}
{"case":"hidden_committed_submodule","status":"BLOCKED","codes":["UNSUPPORTED_PRODUCT_FILE","UNSUPPORTED_PRODUCT_FILE","UNSUPPORTED_PRODUCT_FILE","PRODUCT_DIFF_MISMATCH","REVIEW_COVERAGE_MISSING","CHECK_SOURCE_DRIFT"]}
{"case":"replacement_hides_product","status":"BLOCKED","codes":["DIRTY_WORKTREE","PRODUCT_WORKTREE_DRIFT","PRODUCT_DIFF_MISMATCH","REVIEW_COVERAGE_MISSING","CHECK_SOURCE_DRIFT"]}
```

正常对照保持 READY_FOR_APPROVAL；隐藏未改产品、隐藏执行位、隐藏已提交子模块和 replace 引用替换真实 head 四例全部 BLOCKED。脚本打印结果，核对由独审者完成；不把它说成内含五个自动 assert 的测试套件。

## 证据来源与其他审查结果

本会话真实工具输出与 functions store 中保留的已执行脚本原文；现回填为持久实体报告。

独立五例原先直接输出至工具结果，没有另存日志文件；临时目录已清理。此 JSON 保留实际输出和原执行脚本，未虚构原日志路径或哈希。

当前源码指纹来自已执行的 `shasum -a 256 .sandcastle/delivery-bundle.ts .sandcastle/delivery-bundle.mts .sandcastle/delivery-bundle.test.ts`（工具输出块 e7f088），写报告时再次逐项核对且不运行测试。最终测试文件已只读复核（工具输出块 e844e6）。

- 报告数据豁免已收为 .chanlun/review-results 下已声明的数据文件；.txt/.mdx 不能按文档后缀豁免。
- 范围内未发现其他可操作的路径穿越、符号链接报告替代、命令注入或扫描例外静默放行问题。
- 原审查加尾部审查可以逐文件 before/after hash 与 mode 连续覆盖；接受此已声明边界。

## 限制

- 本报告不证明异构报告语义真实性、审查人独立性、声明命令确实执行或 scanner 发现真伪；只审工具的已声明绑定及阻断逻辑。
- 五例中的报告、扫描和检查元数据是测试夹具；该独立脚本没有实际执行其中声明的编译命令或安全扫描，不能充当项目编译/scanner 运行凭据。
- 独立脚本打印结果后由审查者逐项核对；未声称它自身含五个自动 assert。
- 未重复实现方整包测试、类型检查或全仓安全扫描；其结果应另由实际执行者交付。
- 没有跨进程原子锁证明；不会把本次有界审查称为整个仓库安全证明或交付/合入授权。
- 本次只写 P6 中指定的 Markdown 与 JSON 两份报告，未修改 W6 源码、未合入、未发布外部消息。

本报告 JSON 实体 SHA-256：`ae366880f7621c2bd95a8202e3783549bfac1e4bcf584b1bb5d505f5ba0341f3`。原临时脚本和 Git fixture 已清理；没有可列出的原始独立测试日志文件路径及哈希。
