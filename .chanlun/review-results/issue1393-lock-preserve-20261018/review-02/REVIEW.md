# #1393 根具名更窄帽：第二轮独立评审（review-02）

- 评审者：Claude Opus 5.5（CLI 工蜂），只读静态评审，另做 Python 标准库测试、变异实验和 toy 实验
- 评审对象：`d75d28a4502f39f2ed650f6048a78e22fd484071`（tree `ac47838d1c09f0541c5bb758f387ae15d1e83c06`），对比起点 `d06b12743db549389cbd2f8aa8cf4d86ac84d3b8`
- 副本：`review2/base/`，由 `git archive` 导出 `.github`、两个 review-results 目录、`rust/Cargo.toml` 和 `Cargo.lock`。基线 82 项测试 OK。
- 未读取的作废材料：`issue1393-lock-prepare-20261018`、`issue1393-resume-20261018`、`RUN17-PARTIAL-*`、`run-17-partial-frozen/` 均未读取，也未作为对照。

## 结论：PASS_LIMITED_STATIC

没有 CRITICAL 或 HIGH 级问题。首轮 review-01 的 3 项 HIGH 和 1 项 MEDIUM 均已闭合。本轮新发现 4 项 MEDIUM、4 项 LOW，都不破坏帽值语义或上传字节上界，但其中 F1 是一条实际存在的越帽路径，建议在根签批前返修。

「有限静态」的含义是：本结论不覆盖真实 Actions 运行、真实 cargo cache、真实 upload-artifact 行为、cargo/Lean/fixture drift。

---

## 1. 帽值与根原件一致、措辞、保持值 —— 通过

- 根原件副本是 `.chanlun/review-results/issue1393-lock-preserve-20261018/ROOT-REGISTRY-CAPS-20261018.json`，SHA256 为 `d4c6e154…`，与管理目录原件一致。
- runner `issue1393_lock_prepare.py:44-65` 中的 `RESOURCE_CAPS` 为：single_bytes 65536、total_bytes 2097152、files 128、items 512、upload_bytes 33554432，与根原件逐项相等。
- 措辞方面：`CAPS_SOURCE='ROOT_NEW_NARROWER_CAPS_20261018'`；第 44 行注释写「根 2026-10-18 本次新定的更窄上限（不是「既有帽」）」；合同 `decision=PREPARATION_ONLY_NEW_NARROWER_CAPS`。CAPS-REPORT 也明说「不称既有帽」。
- 保持值不变：
  - `existing_preserved` 为 MAX_MESSAGE_BYTES 32768、MAX_EVENT_BYTES 1048576、900/90/10 秒、jobs 1、initial_disk_bytes 2147483648。
  - `issue1393_validate.py:32-33` 的 32768/1048576 未改动，该文件在 d06b→d75d 的 diff 中为空。
  - 2 GiB 只在 `:1070` 用作启动磁盘门。
- `test_caps_match_in_repo_root_original` 钉住根原件 SHA，并对字面量、FIXED 和 CONTRACT-DRAFT 做逐层类型严格对拍。

## 2. 计数对象 —— 通过

- `counts=REGISTRY_EVIDENCE_BODY_AND_VERSION_RECORDS_ONLY`。
- `is_registry_evidence`（`:445`）只把 registry 正文和摘录归入 files/total。
- `take_items` 只在每条版本记录处计 1。
- 非 registry 原件（lock、manifest、日志等）不计入 registry 帽，由 `test_non_registry_originals_not_counted_in_registry_caps` 锁定。
- 上传帽单独按「全部 artifact 容器合计」计数（`upload_counting=ALL_ARTIFACT_CONTAINERS_COMBINED_INCLUDING_MAIN_AND_RECEIPT_OUTER_OVERHEAD`）。
- 例外是失败路径，见 F1。

## 3. cache-only 与 proof_gap —— 通过

- `mode=EXISTING_EXCLUSIVE_CARGO_CACHE_ONLY`，`additional_network_for_evidence=false`。runner 没有任何网络 import；cargo 环境为 `CARGO_NET_RETRY=0`，且没有新增取证网络。
- `scan_sparse_cache`（`:605-697`）的行为：
  - 以 64 KiB 分块流式读取，按 v3 magic、validator、成对的 version/json 逐段解析；
  - 单条记录超过 single 帽时记为 proof_gap，不截断写入；
  - 所需版本缺失时记为 proof_gap；
  - 遇到非所需的重复版本、坏 magic、空文件或缺 validator 时拒绝；
  - scope 为 `WHOLE_CACHE_FILE_NOT_ARCHIVED…`，即只摘录所需记录，不归档整个 cache。
- `snapshot_registry:836` 要求没有 proof_gap 才能声称完整。变异 M6（删除该 require）杀死了 22 项测试。

## 4. 上传上界、workflow 顺序、if、GITHUB_OUTPUT、rootDirectory —— 通过（附 F2、F6）

上界公式为 `container_bound = 98 + Σ(size + 128 + 2·len(UTF8('/'+name)))`。我对照依赖源码逐项核实（compress-commons 6.0.2 / zip-stream 6.0.1 / archiver / upload-artifact `ea165f8d` dist）：

| 部件 | 依赖中的实际字节 | 公式中的覆盖 |
|---|---|---|
| LFH | 30 + n | 128 中的一部分，外加一次名字 |
| 数据描述符 | 16（ZIP64 时 24） | 128 中的一部分 |
| CFH | 46 + n，comment 为空 | 128 中的一部分，外加一次名字 |
| ZIP64 extra | 28，仅 isZip64 或偏移超过 4 GiB 时出现 | 128 中的一部分；32 MiB 帽下不会触发 |
| EOCD | 22 | 98 中的一部分 |
| ZIP64 EOCD 与 locator | 76，仅条目数 >65535 时出现 | 98 中的一部分 |

另外三点：
- `compression-level` 由 parseInt 解析为 0，配合 `forceZip64` 为 false，zip-stream 走 store 模式。
- dist 第 3268 行起，zip 流直接 pipe 给 `uploadStream`，没有二次包装。
- `setName` 会去掉前导 '/'，所以公式按 '/'+name 计数偏保守，不会低估。

workflow 方面：
- 顺序为：archive 写占位 → upload-gate → payload；payload 的 if 含 `steps.archive.outputs.allowed == 'true'`；之后是 receipt：ACTUAL 超过 4096 时拒写 → receipt-gate → conclusion。
- `GITHUB_OUTPUT` 由 gate 写入 allowed/minutes。
- 两处上传都使用固定 commit `ea165f8d…`，`compression-level: 0`。
- rootDirectory：收据上传用三行 `${{ runner.temp }}/<名>`。按 dist 125447/125489，多 path 时取 LCA，即 runner.temp，destinationPath 为 '/名'，与上界计数和门计数一致。主包是单目录 path，rootDirectory 即该目录。
- run16 中按 AST 读帽和 UPLOAD-REQUEST 的旧路径已删除。

## 5. 合同封套 —— 通过

- `validate_contract` 和 `strict_equal`（`:139-163`）逐层严格比较类型，拒绝 0 与 False、1.0 与 1、True 与 1 互相冒充，也拒绝未知或缺失的嵌套字段。
- 合同 schema 为 `issue1393-lock-preserve/2`，批准和独审收据 schema 均为 `/2`；旧 schema 被拒绝。
- CONTRACT-DRAFT 的 `resource_caps` 与 FIXED 严格一致，字段集合等于 FIXED 加上 hash/UNBOUND 字段。
- 各摘要与 HEAD 实际 blob 一致：runner `60a29eb9…`、workflow `99cc0d1f…`、ci `b347f791…`、helper `a98e7e6f…`、telemetry `57ae7b0a…`、validation `47c31701…`。
- approval、review、prepared_* 均为 UNBOUND。
- `status=PREPARED_NOT_AUTHORIZED` 与 FIXED 的 `BOUND_FOR_ROOT_REVIEW` 不同，这是有意设计，与 run16 一致。

## 6. run16 测试未削弱与变异实验 —— 通过（附 F3、F4、F5、F8）

- 测试数从 47 增至 82。run16 原有的 47 项中有 2 项改名并改写语义：
  - `test_caps_preflight_before_read_and_single_total_items` 改为 `…registry_total_files_items`；
  - `test_cumulative_read_bytes_and_items` 改为 `test_non_registry_originals_not_counted_in_registry_caps`。根口径下读取本就不计入累计帽，所以这是口径变更，不算削弱。
- files 帽构造器分支去掉了 `opened.assert_not_called()`，属轻微削弱（F8）。
- 其余改动包括：provenance 改为 /2，故障注入挂点改为 `scan_sparse_cache`，round_trip 测试有所加强。

变异实验共 13 项（`mutate.py`，结果见 `mutation-results.json`），每项都是单点真实破坏后跑原测试：

| ID | 破坏 | 结果 |
|---|---|---|
| M1 | single 边界放宽 1 字节 | 杀死 |
| M2 | 上界中名字只计一次 | 杀死 |
| M3 | upload-gate 不含收据预留 | 杀死 |
| M4 | receipt-gate 只比名字和长度，不比摘要 | **存活**（F5） |
| M5 | 删除 receipt-gate 的目录变化比对 | 杀死 |
| M6 | 有 proof_gap 仍称完整 | 杀死（22 项） |
| M7 | 删除 cache 末段截断检查 | **存活**（F3） |
| M8 | 扫描循环内不核截止和信号 | **存活**（F4） |
| M9 | items 不计 | 杀死 |
| M10 | workflow 中 upload-gate 移到占位写入之前 | **存活**（F2） |
| M11 | payload 的 if 引用错误步骤 id | **存活**（F6，属 fail-closed） |
| M12 | 放行非所需重复版本 | 杀死 |
| M13 | 删除 package 预核 | 杀死 |

任务要求的四类真实破坏均被杀死：single 边界（M1）、合计上界（M2、M13）、upload-gate 和 receipt-gate（M3、M5）、cache-only 与 proof_gap（M6、M12）。

## 7. 其它缺陷

见下方 F1–F8。

---

## 首轮 review-01 返修核实

| 项 | 状态 | 证据 |
|---|---|---|
| H1：合计上界检查无测试 | 闭合 | 新增 `test_check_refuses_existing_output_over_combined_cap_equal_passes`；M3、M13 被杀死 |
| H2：收据路径的 rootDirectory 未测 | 闭合 | workflow 测试逐字断言三件路径和主包 path；我对照 dist 独立核实了 LCA 推断 |
| H3：帽值与根原件没有一致性锁 | 闭合 | `test_caps_match_in_repo_root_original` 钉住 SHA，并做三方严格对拍 |
| M1：注释 | 闭合 | `container_bound` 注释已改为「固定 128B + 名字 LFH/CFH 各一次」 |

---

## 发现

### F1 MEDIUM：registry 失败路径绕过 files/total 帽，upload-gate 不复核 registry 帽

- 问题：`snapshot_registry` 的 except 分支用 `save()` 写入 `registry-failure.json` 和 `registry.json`，不经过 EvidenceIO 的 registry 帽。`gate('upload-gate')` 只核上传容器上界，不复核 registry files/total。
- 复现（toy：`toy_failure_path_caps.py`，输出见 `.out`）：在 files=1 时走失败路径，结束后 registry 证据有 3 个文件（registry-config.raw、registry-failure.json、registry.json）。此时 `EvidenceIO.check` 拒绝，但 `upload-gate` 输出 `allowed=true`、`minutes=14`。
- 缓解因素：complete 仍为 false，上传字节仍受 32 MiB 上界约束，所以降为 MEDIUM。
- 修复：失败记录也走有界写；或在 upload-gate/receipt-gate 复核 registry 帽，超帽时只标记失败；或在合同中明文规定「files 帽预留 2 个失败记录名额」，并补测试。

### F2 MEDIUM：「占位写入先于 upload-gate」的 workflow 顺序未被测试锁定

- 问题：M10（把 upload-gate 移到占位写入之前）存活。
- 修复：workflow 测试断言 archive 步中占位写入的行先于 `upload-gate` 调用。

### F3 MEDIUM：截断检查缺少可区分用例

- 问题：M7 存活。被截断的所需版本会因版本缺失记为 gap，所以现有测试测不出区别；但被截断的非所需末段会被接受。
- 修复：增加「所需版本齐全，cache 末尾有一段被截断的非所需段」的用例，并断言拒绝。

### F4 MEDIUM：扫描循环内的截止和信号检查缺少可区分用例

- 问题：M8 存活，因为扫描结束后的检查兜住了。
- 修复：测试断言截止或信号在下一次 `read` 之前即被拒，并断言 read 次数有界。

### F5 LOW：receipt-gate 未测「长度不变、内容改变」

- 问题：M4 存活。
- 修复：增加同长度改写散件的用例。

### F6 LOW：payload 的 if 未精确断言引用 `steps.archive.outputs.allowed`

- 问题：M11 存活。后果是 fail-closed：主包永远不会上传。
- 修复：测试精确断言 if 表达式。

### F7 LOW：seen 集合和扫描条目数没有项数上限

- 问题：这两者只受时间截止约束。
- 修复：可选，加一个扫描条目上限，或在合同中写明「仅受截止约束」。

### F8 LOW：run16 files 帽测试去掉了 `opened.assert_not_called()`

- 修复：恢复这条断言。

### 备注（不计为缺陷）

- 上传期间目录被改动，只能事后由 receipt-gate 检测，这是已知限度。
- CONTRACT-DRAFT 的 status 与 FIXED 不同，属有意设计。

## 已运行与未运行

- 已运行：
  - `python3 -B .github/scripts/test_issue1393_lock_prepare.py`：工位与副本均为 Ran 82 tests OK；
  - `mutate.py`（13 个变异）；
  - `toy_failure_path_caps.py`。
- 未运行：cargo、rustc、rustup、lake、Lean、fixture drift、真实 GitHub Actions、网络、产品、浏览器、push、dispatch。
