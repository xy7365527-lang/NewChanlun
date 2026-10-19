# #1393 根具名更窄帽实施评审报告

**评审工蜂**: Claude Opus 5.5  
**评审对象**: commit 5583547bdae20df917de60790834d34a8197ff82（相对起点 d06b12743db549389cbd2f8aa8cf4d86ac84d3b8）  
**评审时间**: 2026-10-19  
**评审模式**: 只读、零网络、零 cargo/Lean/产品运行  

---

## 逐项判定

### 1. 五个帽数值与根原件逐字一致；明称根本次新定、未称既有帽；保持值原义未变

**判定**: ✓ **PASS**

**证据**:
- 根帽原件 `/Volumes/AgentStorage/issue1393-sandcastle-20261018-management/ROOT-REGISTRY-CAPS-20261018.json` 定义:
  - `registry_evidence.single_bytes = 65536`
  - `registry_evidence.total_bytes = 2097152`
  - `registry_evidence.files = 128`
  - `registry_evidence.items = 512`
  - `upload_bytes = 33554432`

- 实现 `.github/scripts/issue1393_lock_prepare.py:47-48`:
  ```python
  RESOURCE_CAPS = {'single_bytes': 65536, 'total_bytes': 2097152, 'files': 128, 'items': 512,
                   'upload_bytes': 33554432}
  CAPS_SOURCE = 'ROOT_NEW_NARROWER_CAPS_20261018'
  ```

- 明确声明本次新定（:44-46 注释）：  
  > 根 2026-10-18 本次新定的更窄上限（不是「既有帽」）

- 保持值原义（:62-64）:
  - `MAX_MESSAGE_BYTES = 32768` (保持)
  - `MAX_EVENT_BYTES = 1048576` (保持)
  - `total_seconds = 900` (保持)
  - `jobs = 1` (保持)
  - `initial_disk_bytes = 2147483648` (保持，仅启动门)

- 合同 `CONTRACT-DRAFT.json` 与代码 FIXED 一致，schema 升级为 `issue1393-lock-preserve/2`。

---

### 2. 计数对象正确

**判定**: ✓ **PASS**

**证据**:

**single_bytes（单条版本 JSON 原文）**:
- `:604-609` `scan_sparse_cache` 的 `limit()` 函数:
  ```python
  return RESOURCE_CAPS['single_bytes'] if current in wanted else 0
  ```
  只对所需版本段缓冲，非所需版本只计长度 (`:628-634 feed()`)

- `:653-661` 单条超限记 `proof_gap`，不截断:
  ```python
  if overflow:
      gaps.append({'version': current, 'reason': '单条版本原文超 single_bytes；不截断冒充完整', ...})
  ```

**total_bytes/files（registry 证据正文）**:
- `:445-447` 明确定义计数范围:
  ```python
  def is_registry_evidence(name):
      """registry 证据正文的唯一命名口径：四个 registry 帽只计这些文件。"""
      return name == 'registry.json' or name.startswith(REGISTRY_PREFIX)
  ```

- `:522-527` `EvidenceIO.check()` 分别统计:
  ```python
  registry = [r for r in rows if is_registry_evidence(r['name'])]
  state = {'registry_files': len(registry), 'registry_bytes': sum(r['bytes'] for r in registry), ...}
  require(state['registry_files'] <= RESOURCE_CAPS['files']
          and state['registry_bytes'] <= RESOURCE_CAPS['total_bytes'], ...)
  ```

- 工具链日志、旧源等不进入 registry 帽（测试 `test_non_registry_originals_not_counted_in_registry_caps` 验证）

**items（保留版本记录）**:
- `:544-549` `take_items()` 每保留一条即计:
  ```python
  def take_items(self, count):
      require(self.items + count <= RESOURCE_CAPS['items'], 'registry 版本记录累计项数帽')
      self.items += count
  ```
- `:660` 扫描中调用 `io.take_items(1)`

**upload_bytes（全部容器外层合计）**:
- `:528` 检查主包上界 + 收据预留:
  ```python
  require(state['main_bound'] + RECEIPT_RESERVE <= RESOURCE_CAPS['upload_bytes'], ...)
  ```
- `:461-478` `container_bound()` 计算 ZIP store 外层开销，每容器 98B + Σ(size + 128 + 2·UTF8名)
- `:982-983` `upload-gate` 复核主包 + 收据合计

**不把整份 cache 当原文**:
- `:693-695` meta 明写 `scope: 'WHOLE_CACHE_FILE_NOT_ARCHIVED_ONLY_IDENTITY_AND_SELECTED_RECORD_RAW'`
- 摘录文件 `:749-750` 只写 magic + validator + 所选版本段

---

### 3. cache-only：无联网；缺失/异常均 proof_gap；流式同一截止

**判定**: ✓ **PASS**

**证据**:

**无网络导入**（测试 `test_runner_has_no_network_or_http_fetch_path` :1112-1122）:
- AST 扫描 runner 源码，禁止导入 `socket/urllib/http/requests/ssl/subprocess`
- 扫描 `scan_sparse_cache` 函数体，禁止 `read_text/json.loads/Popen/urlopen` 等无界操作

**流式扫描受同一截止**:
- `:668,670` 每块前后 `check_budget(io.budget, io.supervisor)`
- `:616-617` 每段缓冲区限于 `single_bytes/VERSION_SEGMENT_MAX/VALIDATOR_MAX`
- `:691-692` 扫描结束后核文件未变

**缺失/异常记 proof_gap**:
- `:640,645,658,680,692` 多处 `require` 失败拒绝
- `:653-655` 单条超限记 gap 不截断
- `:805-807,815-816,831` 缺 cache/版本/约束父记 gap
- `:835` 有 gap 即拒绝 `complete=True`
- `:842-855` 异常写 `registry-failure.json` 和 `complete=False`

**重复/格式错**:
- `:646-647` `current not in seen` 禁止重复版本
- `:658-659` 核 `name/vers` 字段
- `:680` 核 sparse magic

**截止与信号**:
- `:891-892` `check_budget()` 核 `supervisor.interrupted` 和剩余时间

---

### 4. 上传：事前可证明保守上界；门后写入拦截；固定 commit/compression 0

**判定**: ⚠️ **PASS_LIMITED_STATIC**（静态核查通过，但 **FINDING-H1** 存在覆盖缺口）

**证据**:

**事前上界公式与固定 commit 原码一致**:
- `:461-478` `container_bound()` 公式:  
  每容器 = 98 + Σ(size + 128 + 2·UTF8名)
- 对照 `upload-artifact-reference/dist-upload-index.js:3268-3309`:
  - `:3304` `zipUploadStream.pipe(uploadStream)` 直传 blob
  - `:3309` `blockBlobClient.uploadStream(uploadStream, ...)` 无再包
- compress-commons 6.0.2（BOUND-CANDIDATE.md 引用）:
  - LFH 30B、CFH 46B、数据描述 24B（ZIP64）、ZIP64 extra 28B
  - EOCD 22B + ZIP64 EOCD/locator 76B = 98B
  - 128 = 30+46+24+28 覆盖每项元数据
- 名字含前导 `/`（:474），比实际多 1B，保守

**固定 action commit 与 compression 0**:
- `:52` `UPLOAD_ACTION = 'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02'`
- workflow `:94,139` 两处上传均 `uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02`
- `:100,147` 两处 `compression-level: 0`
- `:936` Python ZIP 用 `zipfile.ZIP_STORED`

**事前核定**:
- `:917-933` `package()` 在写 FILES/ZIP 前预核:
  ```python
  projected = inner + [('evidence.zip', zip_bound), ('PACKAGE.json', PACKAGE_JSON_MAX)]
  require(container_bound(projected) + RECEIPT_RESERVE <= RESOURCE_CAPS['upload_bytes'], ...)
  ```
- `:976-987` `upload-gate` 在上传前冻结清单、核上界
- `:989-1013` `receipt-gate` 核门后目录未变、收据合计

**超限停止、不删原件**:
- `:931-933` 封包超限拒写 PACKAGE，不删散件
- `:983` 上传门超限 `allowed=false`，workflow `:92` 条件跳过上传
- `:1036-1040` `failed_package()` 保留原件

**收据大小事前有限**:
- `:74` `RECEIPT_LIMITS = {REQUEST: 4096, FROZEN: 65536, ACTUAL: 4096}`
- `:482` `RECEIPT_RESERVE = container_bound(RECEIPT_LIMITS.items())` 固定常数
- `:982,1007` 核实际字节 ≤ 上限

**workflow 步骤顺序**:
- `:66-88` `archive` 步在上传前补缺失、调 `upload-gate`
- `:86` 注释明确"此后不再写入上传目录"
- `:90-100` `payload` 步 `if: steps.archive.outputs.allowed == 'true'`
- `:102-133` `receipt` 步写 ACTUAL 后调 `receipt-gate`
- `:136-147` 收据上传 `if: steps.receipt.outputs.allowed == 'true'`

**rootDirectory 推断与 path**:
- `:97` 主包 `path: ${{ runner.temp }}/issue1393-lock-evidence/`（单个目录，rootDirectory = 该目录）
- `:142-145` 收据包 `path:` 多行（三个文件，rootDirectory = `runner.temp`）
- 收据三件在 `temp/` 下（`:984,1005,1026`），与冻结清单名字一致

**⚠️ FINDING-H1: 变异测试2发现 `EvidenceIO.check()` 中的合计上界检查未被有效测试覆盖**（详见"缺陷"节）。

---

### 5. 合同封套：schema/2、逐层严格类型、拒绝旧值/未知字段

**判定**: ✓ **PASS**

**证据**:

**schema 升级为 /2**:
- `:84` `FIXED['schema'] = 'issue1393-lock-preserve/2'`
- `:160` 批准/独审 schema 核 `'issue1393-lock-preserve-' + kind + '/2'`

**逐层类型严格**:
- `:139-147` `strict_equal()` 递归核类型:
  ```python
  if type(left) is not type(right):
      return False
  if type(left) is dict:
      return left.keys() == right.keys() and all(strict_equal(left[k], right[k]) for k in left)
  ```
- `:154` `require(strict_equal(contract[key], expected), ...)` 拒绝 0/False、1.0/1

**旧 F/schema/operation/ref 拒绝**:
- `:153-156` 核 `FIXED` 每个字段严格相等
- `:193` 核 `source_commit` 固定
- `:163` 旧批准/独审拒绝

**错数值/未知字段拒绝**（测试 `:1036-1087` `test_wrong_caps_values_types_unknown_fields_and_old_schema_rejected`）:
- 单项测试 single+1、float、True/0 混用、未知嵌套、缺字段等 16 种变异均拒绝
- 顶层未知字段拒绝

**CONTRACT-DRAFT 与代码 FIXED 一致**:
- `:84-98` `FIXED` 定义完整字段
- 读取 `CONTRACT-DRAFT.json`:
  ```python
  import json
  draft = json.load(open('.chanlun/review-results/issue1393-lock-preserve-20261018/CONTRACT-DRAFT.json'))
  assert draft['schema'] == 'issue1393-lock-preserve/2'
  assert draft['resource_caps']['source'] == 'ROOT_NEW_NARROWER_CAPS_20261018'
  ```

**批准/独审/prepared 仍 UNBOUND**:
- CONTRACT-DRAFT `:4,6,19,21` 均为 `"UNBOUND"`
- `:159-163` 核批准/独审 JSON 结构，但当前值为 UNBOUND 会拒绝运行

---

### 6. run16 原测试未被削弱

**判定**: ✓ **PASS**（4 项改写保持或加强断言）

**对比分析**（d06b1274 vs 5583547b）:

**改写的 4 项测试**:

1. **`test_full_round_trip_and_constraint_provenance`** (`:647-665` → `:647-667`):
   - **旧**: 直接从记录取 `raw_json` 字符串和 `cache_artifact`
   - **新**: 从摘录文件按 `evidence_offset/raw_json_bytes` 取原文，对拍 `cache_offset`
   - **强度**: ✓ **加强**（验证摘录偏移正确、原文未截断、与原 cache 字节一致）

2. **`test_caps_preflight_before_read_and_registry_total_files_items`** (`:726-758` → `:736-760`):
   - **旧**: 测"读上限先于打开"用 `single_bytes`、"累计"用 `total_bytes`
   - **新**: "读上限"改测 `io.read(path, root, limit)` 的第三参数、"累计"测 `is_registry_evidence()` 区分
   - **强度**: ✓ **保持**（仍核先于打开，但改为函数参数而非帽值；新增非 registry 原件不计入测试）

3. **`test_non_registry_originals_not_counted_in_registry_caps`** (`:761-765`):
   - **新增**测试：工具链日志不占 registry 帽
   - **强度**: ✓ **新增覆盖**

4. **`test_registry_read_parse_hash_write_expiry_and_exception`** (`:768-801` → `:762-797`):
   - **旧**: 故障注入 hook `sparse_records` 解析函数
   - **新**: hook `scan_sparse_cache` 流式扫描函数
   - **强度**: ✓ **保持**（仍测解析阶段故障，但换为实际入口函数）

**原 47 项测试全部通过**（运行输出 `Ran 80 tests ... OK`），新增 33 项均为扩充覆盖。

---

### 7. 其它正确性/安全缺陷

**判定**: ⚠️ **3 个 HIGH 级别 FINDINGS**

#### **FINDING-H1: `EvidenceIO.check()` 合计上界检查覆盖缺口** ⚠️ HIGH

**发现**: 变异测试2破坏 `:528` 的合计上界检查后，80 个测试仍全部通过。

**根因**: 测试套件未包含"主包散件 + 收据预留逼近 32 MiB 上限"的边界场景。

**影响**: 若该 `require` 被意外删除或条件改错，运行时不会被测试拦截，可能导致实际上传时才发现超限。

**修复建议**: 添加测试：构造总计 33554432 - RECEIPT_RESERVE - 200 字节的散件，断言封包成功；再加 300 字节断言封包拒绝。

---

#### **FINDING-H2: workflow 的 `path` 多文件 rootDirectory 推断未在 runner 测试** ⚠️ HIGH

**发现**: 收据包上传（`:142-145`）用多行 `path:`，action 会推断 `rootDirectory` 为共同父目录 `runner.temp`。若三件中一个路径写错（如 `issue1393-lock-evidence/UPLOAD-ACTUAL.json`），rootDirectory 会错误上移或失败。

**根因**: runner 测试全部 mock Popen，无真实 action 调用；workflow YAML 解析测试（`:11`）只核 valid YAML，不核 `path` 字段语义。

**影响**: 若收据三件路径写错，workflow 能通过 YAML 校验但运行时失败或上传错误内容。

**修复建议**: 
1. 在 workflow 测试中解析 `:142-145`，断言三个路径均在 `${{ runner.temp }}/` 下且不含子目录
2. 或添加注释/ADR 明确说明 rootDirectory 推断依赖所有路径在同一目录

---

#### **FINDING-H3: 帽值只在 Python 字面量，缺 CI/ADR 锁定** ⚠️ HIGH

**发现**: 五个帽数值只在 `:47-48` 定义为 Python dict 字面量。合同、workflow、测试均引用该模块变量。但没有独立的 CI 检查或 ADR 文档锁定这些值不被意外改动。

**根因**: ADR 0012（若存在）未找到；CI 的 fixture drift 检查不覆盖 Python 常量。

**影响**: 若开发者误改 `:47` 的某个数值（如 `65536 → 131072`），测试会用新值通过，但与根帽原件静默分叉。

**修复建议**:
1. 添加 CI 步骤：读取 `ROOT-REGISTRY-CAPS-20261018.json` 与 runner 的 AST 字面量，断言逐字一致
2. 测试 `test_real_caps_are_root_named_values_and_preserved_limits` (`:1035`) 已读 AST，但运行在本地不在 CI
3. 或将帽值移至独立 JSON 文件，runner 运行时加载并核 SHA256

---

#### **FINDING-M1: `container_bound()` 公式注释与实现有偏差** ⚠️ MEDIUM

**发现**: `:462-468` 注释写"每项 = 原文 + 128 + 2·UTF8名"、"每容器 + 98"，但实现 `:476` 只对名字长度 `len(encoded)` 乘 2，未区分 LFH/CFH 各用一次名字。

**分析**: 实际公式已保守（名字含前导 `/` 多 1B），但注释可更精确描述"128 含两次名字开销的预留，最终 `2·len(encoded)` 覆盖"。

**影响**: 注释读者可能误以为公式不保守。

**修复建议**: 修改注释为"每项固定元数据 128B（含 LFH 30 + CFH 46 + 描述 24 + extra 28）+ 2·UTF8名字节（LFH/CFH 各一次）"。

---

#### **FINDING-L1: 扫描中文件变化只事后检测，无 inotify** ⚠️ LOW

**发现**: `:691-692` 扫描结束后核 `path.lstat().st_size == info.st_size`，但扫描期间（可能数秒）无实时监控。

**分析**: 任务书未要求 inotify，且 sparse cache 在独占临时目录由单一 Cargo 进程生成，并发写入概率极低。事后检测已符合"扫描期间变化拒绝"。

**影响**: 理论上若扫描期间 cache 被外部改写又恢复原大小，会漏检。但实际场景不存在该威胁源。

**修复建议**: 保持现状（文档注明"独占临时环境"）或添加 fcntl 文件锁。

---

## 变异实验记录

| 实验 | 破坏点 | 预期 | 实际 | 说明 |
|------|--------|------|------|------|
| 1 | `scan_sparse_cache` single_bytes 改为 999999999 | 变红 | ✓ 变红 | `test_single_record_equal_cap_complete_and_over_cap_proof_gap_untruncated` 失败 |
| 2 | 移除 `EvidenceIO.check()` 的 upload_bytes 合计检查 | 变红 | ✗ 仍绿 | **FINDING-H1**: 测试覆盖缺口 |
| 3 | 测试中注释掉 `compression_level=6` 检查 | 测试减少 | ✓ 仍绿 | 验证测试可被移除（不是变异实验失败） |

---

## 测试运行记录

**命令**: `python3 -B .github/scripts/test_issue1393_lock_prepare.py`

**结果**: `Ran 80 tests in 1.825s OK`

**未运行项**（按任务书）:
- Cargo/rustc/rustup/lake/Lean
- 真实 GitHub Actions
- 网络/联网取证
- fixture drift 检查
- 产品/浏览器
- push/dispatch/merge

---

## 结论

**总体判定**: ⚠️ **PASS_LIMITED_STATIC**（有条件通过静态核查）

**通过项**: 1-3, 5-6（五个帽逐字一致、计数对象正确、cache-only 实现完整、合同严格、测试未削弱）

**有限通过项**: 4（上传事前上界静态正确，但 FINDING-H1/H2 存在运行时覆盖缺口）

**阻断条件**: 存在 **3 个 HIGH 级别 findings**，需修复或明确接受后方可授权运行：
1. **H1**: 补充合计上界边界测试
2. **H2**: 明确 workflow `path` 多文件的 rootDirectory 约束
3. **H3**: 添加 CI 锁定帽值与根原件一致性

**独审建议**: 在修复上述 findings 或管理者明确接受残留风险后，方可进入真实工具链运行与独审阶段。当前实现**不建议直接激活 `ISSUE1393_LOCK_EXECUTE: 'True'`**。
