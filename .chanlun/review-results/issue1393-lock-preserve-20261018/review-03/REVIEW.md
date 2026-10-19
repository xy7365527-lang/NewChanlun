# #1393 根具名更窄帽：第三轮独立评审

- 评审提交：`57f41578396d172d9a465b71fbdd87b988ae8d0d`（tree `ba94072c71a7a2e06927a3c26473f12746272648`）
- 对比基：`d06b12743db549389cbd2f8aa8cf4d86ac84d3b8`；diff 只动 runner、测试文件、workflow 与报告目录，其它代码未动
- 结论：**PASS_LIMITED_STATIC**。共 5 项发现，全部为 LOW，没有 CRITICAL、HIGH 或 MEDIUM。
- 依据：只读原件，以及本目录下的 toy 和变异实验。没有采用作者的自报。没有读 run17-partial、lock-prepare-20261018、resume-20261018 等禁读路径。

## 0. 基线

| 场所 | 命令 | 结果 |
|---|---|---|
| 工位（审前、审后各一次） | `python3 -B .github/scripts/test_issue1393_lock_prepare.py` | `Ran 86 tests OK`，`git status` 干净 |
| 副本 `review3/base/`（git archive 取 `.github`、`.chanlun/review-results`、`rust/Cargo.toml`、`rust/Cargo.lock`） | 同上 | 基线 `Ran 86 tests OK`；24 个变异结束后再跑仍 `OK` |

## 1. 帽数值与措辞

- 根原件 `ROOT-REGISTRY-CAPS-20261018.json`（SHA256 `d4c6e154…`）与仓内副本用 `cmp` 比对，逐字节一致。
- runner 的 `RESOURCE_CAPS`（`issue1393_lock_prepare.py:47`）取值：single 65536、total 2097152、files 128、items 512、upload_bytes 33554432。与根原件逐项相等。
- `CAPS_SOURCE='ROOT_NEW_NARROWER_CAPS_20261018'` 见 :49。:44 的注释写明这些「不是既有帽」，措辞没有把新帽冒充成旧帽。
- 保持不变的值：
  - MAX_MESSAGE_BYTES 32768、MAX_EVENT_BYTES 1048576，在 helper `issue1393_validate.py:32-33`。helper 本次未改，SHA `a98e7e6f…`。
  - 900/90/10 秒、jobs 1。
  - 2 GiB 只出现在 :1100，用作开工磁盘门，没有被挪作上传帽。
- 测试锁定：
  - `test_caps_match_in_repo_root_original`（:1068）用 AST 读出 runner 的字面量，并与 FIXED 合同和 CONTRACT-DRAFT 逐层做类型严格的对拍。
  - 变异 X1（single +1）被杀。

## 2. 各帽计数口径

- `is_registry_evidence`（:445）只认 `registry.json` 和 `registry-*`。变异 X10（只认 registry.json）被杀，24 项测试失败。
- files 和 total 只统计 registry 证据正文（`registry_usage` :863）。非 registry 原件不计入，测试为 `test_non_registry_originals_not_counted_in_registry_caps`。这是按根原件的 counts=`REGISTRY_EVIDENCE_BODY_AND_VERSION_RECORDS_ONLY` 改写的口径，不是放宽。
- items 只计保留下来的版本记录，跨文件累计。
- upload_bytes 计全部 artifact 容器的上界之和：主包加收据预留（`EvidenceIO.check` :519，`RECEIPT_RESERVE` :483）。变异 X8（去掉收据预留）被杀。
- single：一条原文恰好等于 65536 时记为完整，超 1 字节记为 proof_gap，且摘录里不出现被截断的内容。变异 X1 被杀。

## 3. 只用缓存与 proof_gap

- runner 不导入任何网络或子进程模块；扫描函数里没有整读、loads 或 Popen 调用（测试见 :1141-1145）。变异 X5（导入 urllib）被杀。
- `scan_sparse_cache`（:605-697）的做法：
  - 每次读 64 KiB，读前读后各调一次 check_budget（:669、:671）。
  - 结尾检查截断（:693）。
  - 遇到非所需的重复版本、缺 validator、坏 magic，一律拒绝。
  - 单条记录超帽时降为 gap，而不是截断后照收。
- 相关变异 M7、M8 被杀（详见第 6 节）。
- 缓存缺失、旧版本缺失、symlink、写入过期：都不会标记为 complete，测试为 RegistryTests 和 NewCapsRegistryTests 两组。
- 合同写的是 `registry_network_fallback: FORBIDDEN`，以及 `EXISTING_EXCLUSIVE_CARGO_CACHE_ONLY_..._MISSING_IS_PROOF_GAP`，与实现一致。

## 4. 上传上界与 workflow

### 上界公式

`container_bound` 在 :461-479，公式为：

    98 + Σ(size + 128 + 2·UTF8('/'+name))

按 upload-artifact@ea165f8d 的源码（dist SHA `0165b8a7…`，与参考 manifest 一致）逐项核对：

- **每项实际字节**：LFH 30+n，数据描述符 16，CFH 46+n，合计 92+2n，n 为名字长度。archiver 的 sanitizePath 会去掉开头的 '/'，所以名字字节比公式里少 1。EOCD 22 字节。
- **余量**：公式偏保守，每项多留 36 字节，加每个名字 1 字节；每个容器多留 76 字节。
- **不会落到更大的格式上**：
  - extra 和 comment 默认为空。
  - ZIP64 只在超过 65535 项或 4 GiB 时启用，32 MiB 帽内不会触发。
  - level=0 时 zip-stream 用 store；lazyReadStream 走数据描述符路径。
- **没有额外包装层**：`uploadZipToBlobStorage`（dist:3268）把 zip 直接 pipe 给 blob 上传流。
- **测试锁定**：变异 X2（每项开销 128 改 60）被杀，37 项测试失败。

### rootDirectory

- 主包 path 是单个目录 `${{ runner.temp }}/issue1393-lock-evidence/`，所以根就是这个目录。
- 收据的三个路径都直接位于 `runner.temp` 下。findFilesToUpload 取它们的最近公共祖先（getMultiPathLCA，dist:125447），结果是 runner.temp，destinationPath 为 '/'+名。这与公式、与门的计数一致。

### 步骤顺序与条件

- 步骤顺序是 archive → payload → receipt → 收据上传。
- archive 步先写 result、steps、PACKAGE 三类占位，再调 `upload-gate`；门之后不再有任何写入。变异 M10 被杀。
- payload 步：
  - `if` 为 `always() && env.ISSUE1393_LOCK_EXECUTE == 'True' && steps.archive.outputs.allowed == 'true'`。变异 M11 被杀。
  - `timeout-minutes` 来自 `steps.archive.outputs.minutes`。
  - action 钉在固定 commit。变异 X11（改成 @v4）被杀。
  - compression 为 0。
- receipt 步：
  - 先写 ACTUAL：超过 4096 字节就拒绝，文件用 'xb' 打开。然后跑 `receipt-gate`。
  - 如果 payload 没成功，这一步 exit 1。
- 收据上传：由 `steps.receipt.outputs.allowed` 门控，compression 0。变异 X3（改 6）被杀。
- GITHUB_OUTPUT 只写 allowed 和 minutes 两项。要求 minutes > 0，变异 X7 被杀。REQUEST 写在上传目录之外。
- 默认门为 `'False'`。

## 5. 合同封装

- CONTRACT-DRAFT 的 schema 为 `issue1393-lock-preserve/2`，收据 schema 也为 /2（`validate_contract` :150-163）。
- 以下摘要与 HEAD blob 的 SHA256 全部相等：
  - runner `d8ab8004…`、workflow `99cc0d1f…`、ci `b347f791…`、helper `a98e7e6f…`
  - telemetry `57ae7b0a…`、validation `47c31701…`、toolchain `736b421f…`
  - manifest `f1294401…`、old_lock `c5918583…`
- approval、review、prepared_* 三组字段保持 UNBOUND；status 为 `PREPARED_NOT_AUTHORIZED`；resource_caps 与 FIXED 相等。
- `strict_equal`（:139）要求类型也相同，所以 True 不能顶 1，0 不能顶 False，float 也不行。变异 X4 被杀。
- 未知字段、缺失字段、旧 schema、旧 GENERATE 命令、旧 ref、旧锁 SHA、网络 ALLOW、旧批准文本，全部被拒。测试为 :491 与 :1091。

## 6. run16 测试未被削弱，以及变异实验

逐项对比 d06b..57f4 的测试改动：

- provenance schema 从 /1 改为 /2：随 schema 升级而改。
- 记录完整性测试变强了：新增对摘录偏移、原 cache 偏移、SHA 和 NUL 结尾的核对。
- 原「单文件/总读帽」两项：改为「读上限先于打开文件」（`opened` 不被调用）和「registry 正文 total/files/items 的边界」，并补了写入侧的边界。
- 原「累计读字节/项数」：改为「非 registry 原件不计入 registry 帽」。这是按根口径换语义，不是放宽。
- 故障注入的 parse 阶段改挂到流式扫描函数上。
- F8 指出的 `opened.assert_not_called()` 已在 :761 恢复。
- 旧合同拒绝用例仍在（:491）。

结论：run16 原有断言没有被删除或放松。

### 变异实验

24 个单点变异，只改实现或 workflow，不删改测试，脚本为 `mutate.py`，原始输出在 `mutation-results.json`。结果：**22 个被杀，2 个存活**。

| 变异 | 内容 | 结果 |
|---|---|---|
| M4 | receipt-gate 不比摘要 | 杀（1） |
| M7 | 接受截断 | 杀（3） |
| M8 | 扫描循环内不核截止 | 杀（2） |
| M10 | 门移到占位写入之前 | 杀（1） |
| M11 | payload if 引用错步骤 | 杀（1） |
| F1a | bounded_registry_save 不核帽 | 杀（3） |
| F1b | 失败记录回退为无界 save | 杀（2） |
| F1c | upload-gate 删 registry 复核 | 杀（1） |
| F1d | receipt-gate 删 registry 复核 | 杀（1） |
| F1e | 新文件不计入 files | 杀（2） |
| F1f | 去掉最小记录降级 | **存活** → R3-1 |
| F1g | 已存在的 registry.json 不降级 | 杀（2） |
| F1h | 门用 < 代替 ≤ | 杀（1） |
| X1 | single +1 | 杀（1） |
| X2 | 每项开销 128→60 | 杀（37） |
| X3 | 收据 compression 6 | 杀（1） |
| X4 | strict_equal 不核类型 | 杀（2） |
| X5 | 导入 urllib | 杀（1） |
| X6 | workflow ACTUAL 上限 8192 | **存活** → R3-2 |
| X7 | 门不核剩余窗口 | 杀（1） |
| X8 | check 不含收据预留 | 杀（1） |
| X9 | receipt-gate 不核普通文件与上限 | 杀（1） |
| X10 | registry-* 不计入 | 杀（24） |
| X11 | 可变 @v4 | 杀（1） |

括号内为失败的测试数。review-02 中存活的 M4、M7、M8、M10、M11，本轮复跑全部被杀。

## 7. review-02 发现的闭合情况

| 编号 | 状态 | 证据 |
|---|---|---|
| F1 失败路径越 registry 帽 | 闭合 | 见下方说明 |
| F2 占位先于门 | 闭合 | M10 被杀 |
| F3 截断 | 闭合 | M7 被杀，`test_truncated_unwanted_tail_after_complete_wanted_rejected` |
| F4 循环内截止 | 闭合 | M8 被杀，`test_scan_deadline_and_signal_mid_stream` |
| F5 同长度改写 | 闭合 | M4 被杀 |
| F6 if 精确断言 | 闭合 | M11 被杀 |
| F7 seen 集合无帽 | 接受为 LOW | 合同和报告已写明口径，见 R3-3 |
| F8 断言恢复 | 闭合 | 测试 :761 |

F1 的闭合说明：

- **失败路径**：:841-858 改用 `bounded_registry_save`（:868-880）。
  - 判据为 `files + (old is None) <= files` 且 `size - (old or 0) + len(raw) <= total`。
  - 先尝试完整值，放不下再尝试最小记录；还放不下就不写。
- **门复核**：
  - upload-gate 在 :1003-1005 复核 registry 帽，超帽就拒绝，不写 FROZEN，原件保留。
  - receipt-gate 在 :1026-1028 复核。
- **测试**：`test_failure_path_never_exceeds_registry_files_or_total`（files=2、total=600 两种设置）和 `test_gates_refuse_registry_evidence_over_caps`。F1a 到 F1h 中只有 F1f 存活。
- **bounded_registry_save 的边界**：
  - 覆盖已有文件时不增加文件数，净增字节按差值计算。
  - 新文件按 +1 计入 files，F1e 被杀。
  - 判据是 ≤，等于帽时允许写，F1h 被杀。

## 8. 其它发现（全部为 LOW）

- **R3-1**：最小失败记录的降级分支没有可区分的测试（F1f 存活）。现有测试传入的失败值与最小记录逐字相同，所以回退分支从未被单独触发。实现逻辑本身正确：完整摘要必然大于最小记录，覆盖时净增字节为负，真实运行里不存在越帽路径。建议补一个「完整值放不下、最小记录放得下」的用例。
- **R3-2**：workflow 里 ACTUAL 的 4096 上限没有被测试锁定（X6 存活）。不过 receipt-gate 会按 RECEIPT_LIMITS 独立兜底（X9 被杀可证），超限的 ACTUAL 仍会被拒。这是纵深防御上的缺口，不会越帽。
- **R3-3**：F7 沿用。扫描条目数和 seen 集合只受 900 秒截止约束；每块读取前后都核截止，seen 里只存 ≤256 字节的版本字符串。根没有给扫描条目帽，报告已写明这一点。
- **R3-4**：主包上传期间如果目录内容变化，只能事后由 receipt-gate 发现。这是已知限制，实际风险低：
  - workflow 在门之后没有写入步骤（M10 锁定）。
  - telemetry 只在 runner 步内写入（`issue1393_validate.py:502`）。
  - receipt-gate 逐项比对名字、大小和 SHA（M4 锁定），发现变化就拒绝收据并让 job 失败。
- **R3-5**：bounded_registry_save 的返回值被忽略，放不下时会静默不写 registry-failure.json，失败结论由 result.json 承载。它写入时会跟随 symlink，但输出目录是 runner 自建的，两道门都会拒绝 symlink。

## 9. 未运行

cargo、rustc、rustup、lake、Lean、fixture drift、真实 GitHub Actions、网络、产品、浏览器、commit、push、dispatch。因此本结论只是有限的静态结论（toy 加读源），不能替代真实 Actions 运行的证据。
