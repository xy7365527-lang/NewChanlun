# #1588 devskim 分片：DevSkim 门禁红灯处置台账

- 分支：`eng-debt-1588/devskim`（首轮基于 `origin/main` 1d70db893f；第三轮 rebase 到 6c6acd5fe0）
- 触发事实：main 上 DevSkim run [39379637868](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39379637868)（head d2f20aaca8）失败；门禁写入的 step summary 约 6.3 MB，超出 GitHub 1 MiB 上限，摘要页无法渲染。
- 输入（不入仓，只记录校验和）：
  - SARIF `/tmp/eng1588/sarif1/devskim-results.sarif`：sha256 `a1b1dee8…2901`，共 91,438 条结果；
  - 新增清单 `devskim-new-findings-run39379637868.txt`（格式 `rule|path|line`）：sha256 `fb2c24fd…f5e`，去重后 55,146 个键不在基线内。
- 本目录文件：
  - `DEVSKIM-TRIAGE.md`：本台账；
  - `classify_excluded.py` / `classify-excluded-output.txt`：对拟排除目录内全部 DevSkim 结果按规则和匹配形状分类，只输出计数与键；
  - `scan_credentials.py` / `credential-scan-output.txt`：对拟排除目录做全量凭据形状扫描，只输出路径、模式组和计数，不输出命中值。

## 0. 结论速览

| 类别 | 键数 | 处置 |
|---|---:|---|
| `.chanlun/review-results/**` 归档 + `s_session/tests/fixtures/**/*.json{,l}` 测试数据 | 见 §7 | **第三轮按用户裁定（2026-10-21）改为：现有文件的现有告警按精确键 `rule\|path\|line` 追加入基线，不做目录排除，新写入文件仍被扫描**（§2、§7）；全量凭据扫描 0 命中 |
| 源码面（含 fixtures 目录里的 1 个 `.py`） | 81 | 66 条行内抑制、4 条改写过时注释、11 条按精确键入基线（§1） |
| step summary 超限 | — | 门禁新增 `--summary-max-new`（默认 200），摘要只列前 200 条并指向 job log 与 artifact；stdout 保持全量；判红逻辑与退出码不变（§3） |

真问题：81 条源码面告警中没有安全缺陷。2 条 DS176209 是真实的待办项，已抑制并转票（§5：strategy.rs:314 由 #1385 承接，replay_comparator.rs:151 转 #1607）。**未发现真实凭据。**

## 1. 源码面 81 条逐条处置

### 1.1 复现

```bash
F=devskim-new-findings-run39379637868.txt
grep -v -E '^\S+\|(\.chanlun/review-results/|s_session/tests/fixtures/.*\.jsonl?\|)' "$F" | sort -u | wc -l
# 81
```

规则分布：DS162092 ×27、DS173237 ×23、DS172411 ×16、DS176209 ×6、DS137138 ×3、DS148264 ×3、DS197836 ×3。

### 1.2 按规则的根因

| 规则 | 现象 | 根因 | 处置原则 |
|---|---|---|---|
| DS162092（localhost/127.0.0.1） | 本地服务、测试 harness 和启动脚本里出现回环地址 | S 会话是本机只读服务，测试必须连回环；不是调试残留 | 同行抑制 |
| DS173237（疑似密钥的长串） | 测试向量和常量中出现 64/40 位十六进制 | 内容是 SHA-256 摘要或 Git 提交号，不是凭据 | 同行抑制；字节被锁的文件入基线 |
| DS172411（setTimeout 字符串求值） | 浏览器端和测试代码调用定时器 | 回调是函数字面量，不存在字符串求值 | 同行抑制；生成或被锁的 JS 入基线 |
| DS176209（TODO） | 注释里含 TODO 字样 | 4 条是已兑现的过时措辞；2 条是真实待办 | 过时措辞直接改写；真实待办抑制并转票（#1385 / #1607） |
| DS137138（不安全 URL） | 测试用 `http://…invalid` 代理地址 | 保留 TLD `.invalid` 不可解析，是故意构造的失败代理 | 同行抑制 |
| DS148264（弱随机数） | 统计采样用 `random` | 固定种子，目的是可复现的统计采样，不涉及安全 | 同行抑制 |
| DS197836（对时间取哈希） | `hashlib` 与 time 变量同行 | 实际是内容寻址摘要，没有对时间值取哈希 | 同行抑制 |

抑制注释一律写在**同一行行尾**（`  # DevSkim: ignore DSxxxxxx — 理由` 或 ` // DevSkim: ignore DSxxxxxx — 理由`），不新增行，避免同文件其他基线键的行号漂移。唯一例外是 `rust/src/theta_v0/parser/stroke_tb02v_tests.rs:580`：加注后行长超限，`cargo fmt` 规则要求把参数拆成两行，常量与注释随之移到 581 行。DevSkim 按匹配所在行判定，所以抑制仍然生效。

### 1.3 逐条表

| 规则 | 位置（run 39379637868 行号） | 处置 |
|---|---|---|
| DS137138 | `.sandcastle/codex-child.test.ts:137` | 行内抑制：测试用不可解析的 .invalid 代理地址 |
| DS137138 | `.sandcastle/codex-child.test.ts:139` | 行内抑制：测试用不可解析的 .invalid 代理地址 |
| DS137138 | `.sandcastle/codex-child.test.ts:217` | 行内抑制：测试用不可解析的 .invalid 代理地址 |
| DS148264 | `analysis/p3_random_gate_control.py:197` | 行内抑制：固定种子统计采样，可复现，非安全用途 |
| DS148264 | `analysis/p3_random_gate_control.py:251` | 行内抑制：固定种子统计采样，可复现，非安全用途 |
| DS148264 | `analysis/p3_random_gate_control.py:344` | 行内抑制：固定种子统计采样，可复现，非安全用途 |
| DS162092 | `.sandcastle/codex-child.test.ts:138` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `.sandcastle/codex-child.test.ts:87` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `.sandcastle/codex-child.ts:23` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/launch_s.sh:270` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/launch_s.sh:271` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/launch_s.sh:272` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/launch_s.sh:273` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/s_readonly_server.py:1234` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/s_service_control.py:100` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/s_service_control.py:636` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/s_service_control.py:98` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/r9_contract.py:73` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/r9_contract.py:84` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/r9_contract.py:91` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb01c_query_tests.py:361` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb01c_query_v2.py:125` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb02a_runtime.py:129` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb02b_independent_harness.py:500` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb02b_runtime.py:199` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb02c_runtime.py:38` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb02d_runtime.py:98` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb02w_runtime.py:282` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb03b_runtime.py:198` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb03b_runtime.py:379` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/tb03b_runtime.py:58` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/test_tb01c_service_control.py:170` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS162092 | `s_session/tests/test_tb01c_service_control.py:538` | 行内抑制：有意使用本机回环（本地服务/测试），非调试残留 |
| DS172411 | `.sandcastle/codex-child.test.ts:350` | 行内抑制：回调为函数字面量而非字符串 |
| DS172411 | `.sandcastle/codex-child.test.ts:522` | 行内抑制：回调为函数字面量而非字符串 |
| DS172411 | `.sandcastle/codex-child.test.ts:535` | 行内抑制：回调为函数字面量而非字符串 |
| DS172411 | `.sandcastle/codex-child.test.ts:564` | 行内抑制：回调为函数字面量而非字符串 |
| DS172411 | `.sandcastle/codex-child.ts:395` | 行内抑制：回调为函数字面量而非字符串 |
| DS172411 | `.sandcastle/codex-child.ts:406` | 行内抑制：回调为函数字面量而非字符串 |
| DS172411 | `.sandcastle/codex-child.ts:410` | 行内抑制：回调为函数字面量而非字符串 |
| DS172411 | `s_session/browser/q3-consumer.js:1739` | 入基线 |
| DS172411 | `s_session/browser/q3-consumer.js:1838` | 入基线 |
| DS172411 | `s_session/browser/q3-consumer.js:2378` | 入基线 |
| DS172411 | `s_session/browser/tb01c-client.js:4970` | 入基线 |
| DS172411 | `s_session/browser/tb01c-client.js:5069` | 入基线 |
| DS172411 | `s_session/browser/tb01c-client.js:5609` | 入基线 |
| DS172411 | `s_session/browser/tb01c-client.js:6617` | 入基线 |
| DS172411 | `s_session/browser/tb01c-client.js:6855` | 入基线 |
| DS172411 | `s_session/browser/tb03a-client.js:531` | 行内抑制：回调为函数字面量而非字符串 |
| DS173237 | `rust/src/economic_session/financial_content.rs:2004` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:6116` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:6130` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7043` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7047` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7051` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7055` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7059` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7063` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7067` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7071` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7075` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7079` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/s_content.rs:7084` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/session_protocol.rs:656` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/src/theta_v0/parser/stroke_tb02v_tests.rs:580` | 行内抑制：SHA-256 摘要/测试向量，非凭据（rustfmt 后位于 581 行） |
| DS173237 | `rust/tests/s_content_contract.rs:19` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `rust/tests/s_content_sequence.rs:23` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `s_session/browser/tb01c-client.js:6431` | 入基线 |
| DS173237 | `s_session/browser/tb01c-client.js:6551` | 入基线 |
| DS173237 | `s_session/tests/fixtures/tb02d/generate-input.py:282` | 入基线 |
| DS173237 | `s_session/tests/tb02b_independent_harness.py:42` | 行内抑制：SHA-256 摘要/测试向量，非凭据 |
| DS173237 | `s_session/tests/tb03b_runtime.py:91` | 行内抑制：Git 提交号，非凭据 |
| DS176209 | `analysis/p3_random_gate_control.py:141` | 改写注释（去 TODO 字样） |
| DS176209 | `rust/src/theta_v0/nautilus/strategy.rs:104` | 改写注释（去 TODO 字样） |
| DS176209 | `rust/src/theta_v0/nautilus/strategy.rs:113` | 改写注释（去 TODO 字样） |
| DS176209 | `rust/src/theta_v0/nautilus/strategy.rs:19` | 改写注释（去 TODO 字样） |
| DS176209 | `rust/src/theta_v0/nautilus/strategy.rs:314` | 行内抑制：已登记待办（限价腿 entry 价回填，#916 订正段），由 #1385 OE09 承接（第二轮改写理由） |
| DS176209 | `rust/src/trading/replay_comparator.rs:151` | 行内抑制：已知简化版待办（#1518/#1304），已转票 #1607（第二轮改写理由） |
| DS197836 | `rust/src/economic_session/service.rs:125` | 行内抑制：内容寻址摘要，非对时间值取哈希 |
| DS197836 | `s_session/tests/tb02b_independent_harness.py:747` | 行内抑制：内容寻址摘要，非对时间值取哈希 |
| DS197836 | `s_session/tests/tb02w_runtime.py:185` | 行内抑制：内容寻址摘要，非对时间值取哈希 |

改写的 4 处（文字已不含 TODO，语义不变）：
- `strategy.rs:19`：「非 TODO。」改为「非待办项。」
- `strategy.rs:104`：「原 TODO 已兑现」改为「原待办已兑现」
- `strategy.rs:113`：「原「标接入点（TODO）」」改为「原「标接入点」待办」
- `analysis/p3_random_gate_control.py:141`：文档字符串「[TODO — 由用户实现]」改为「[已实现 — 原占位「由用户实现」，采用下方推荐实现]」。函数体早已实现，这里只更正占位措辞。

### 1.4 入基线的 11 条（`.github/devskim-baseline.json` 末尾追加，`updates[-1].entries` 逐键写理由）

| 键 | 为何不行内抑制 |
|---|---|
| `DS172411\|s_session/browser/q3-consumer.js\|1739/1838/2378` | q3-consumer.js 是 tb01c-client.js 生成段的输入，由 `s_session/tests/build_tb01c_browser.cjs` 按逐字节 SHA-256 锁定漂移。改字节就要重生成打包产物，而该目录正被 #1394/#1488 线改动 |
| `DS172411\|s_session/browser/tb01c-client.js\|4970/5069/5609/6617/6855` | 同一文件含生成段，且属在途改动面 |
| `DS173237\|s_session/browser/tb01c-client.js\|6431/6551` | 64 位十六进制是浏览器入口的内容摘要常量 |
| `DS173237\|s_session/tests/fixtures/tb02d/generate-input.py\|282` | 40 位十六进制是本仓 Git 提交号（`git log` 可解析）。文件字节被 `s_session/tests/fixtures/tb02d/MANIFEST.json` 的摘要锁定 |

追加方式：原有 `findings` 顺序不变，新键只追加。理由文本刻意避开会触发规则的英文词。基线文件自身唯一的既有告警在 860 行，追加不改变它的行号。

## 2. 归档与测试数据目录：按精确条目入基线（第三轮，用户裁定）

**裁定（用户 2026-10-21，经主控转达）**：不做整目录排除。`.chanlun/review-results/**` 与 `s_session/tests/fixtures/**/*.json{,l}` 中**现有文件**的现有告警，按现行基线键格式 `rule|path|line` 逐条追加进 `.github/devskim-baseline.json`（新开一条 `updates` 记录，只追加不重排）。这两个目录今后新写入的文件和新出现的告警仍被扫描、仍会判红。理由：与 #1474「不能只凭字段名、目录或 40/64 位形状放行」一致——放行对象是具体的已知条目，不是目录。

处置：
- `.github/workflows/devskim.yml`：删除首轮加的 5 条 glob（`**/.chanlun/review-results/**` 与 fixtures 下根层/子目录的 `*.json`、`*.jsonl`），#1111 原有排除不动；注释改为写明本裁定。
- 新增键取自 rebase 后 HEAD 的真实扫描结果（PR 上的 DevSkim run 的 `devskim-sarif` artifact），生成与断言见 §7。

首轮（已撤回）的方案是 ignore-glob 整目录排除，下面 §2.1–§2.2 的理由与凭据核实仍然有效，作为这批条目入基线的依据。

### 2.1 理由

- `.chanlun/review-results/**`：冻结的评审/证据归档，是绑定工单的工作底稿（宪法 §5），不被任何 CI job 构建、导入或执行。其中 DS173237 占 88,595 条。除 2 条外，匹配文本全是纯十六进制（SHA-256、Git 对象号，形状分布见 `classify-excluded-output.txt`）。剩下 2 条（`issue1340-publication-20260909/payload/inputs/LEGACY-TICKET-DISPOSITION.json:4238`、`RESIDUAL-TICKET-PLAN.json:3362`）已人工看过上下文，均为 `sha256` 字段。其余规则（DS162092 1,467、DS117838 889、DS197836 85 等）出现在归档的报告正文与日志里，同样不执行。
- `s_session/tests/fixtures/**/*.json{,l}`：机器生成、受哈希锁的测试数据（manifest、oracle、schedule），不可手改；行内注释也会破坏 JSON 格式和摘要锁。

### 2.2 入基线前凭据核实（全量，不是抽样；首轮完成）

```bash
python3 .chanlun/review-results/eng-debt-20261020/devskim/scan_credentials.py .
```

覆盖：git 跟踪的 1,812 个文件原文、15,130 个 zip 成员、535 个 base64(+xz/gzip) 附件（全部解码成功）。检查项为强特征（GitHub/Anthropic/OpenAI/AWS/Slack/Google/GitLab 固定前缀、PEM 私钥头、JWT）和弱特征（键名=值形态）。结果：**强特征 0，弱特征 0**。扫描器先用植入的假值自测，确认能命中。

分类阶段曾有 2 个前缀命中（`issue1371-tb01-b/native-codex-r13-r14/EVIDENCE-ATTACHMENTS.json`，DS117838/DS197836 各 1）。核实后它们都落在随机 base64/xz 附件数据里，是偶然出现的前缀。剔除 blob 并解码附件后重扫，结果为 0。另用宽松的键名扫描做了一遍，只找到领域 token（如 `snapshot_token`、`begin_token`、`token_schema`、`forged-token`），都不是凭据。本节只报路径与规则，不报任何值。

### 2.3 与 #1474 的口径张力（第三轮已裁定：选精确条目入基线）

#1474（OPEN）已获批的方向是「稳定告警身份」，并写明「不能只凭字段名、目录或 40/64 位形状放行」。本分片按任务书要求，对归档与测试数据用**目录级** ignore-glob。为不越过 #1474 的口径，排除前对两个目录做了全量（非抽样）凭据扫描，并在 workflow 注释里逐类写了理由。即便如此，这仍是按目录放行：今后写进这两个目录的新文件不会再被 DevSkim 扫描。建议由 #1474 的稳定身份方案接管这两个目录的长期策略（例如在提交钩子里对归档目录运行同类凭据扫描）。

第二轮（主控指示）：本 PR 不改方案，由主控交用户裁定。两种选择：

| 选择 | 内容 | 代价 |
|---|---|---|
| A：合入本 PR 的目录排除 | 两个目录按 ignore-glob 排除；排除前已全量凭据扫描 0 命中；DevSkim 在 main 立即转绿 | 今后写进 `.chanlun/review-results/**`、`s_session/tests/fixtures/**/*.json{,l}` 的新文件不再被 DevSkim 扫描（凭据类也不扫），与 #1474「不能只凭目录放行」的口径冲突，需要另设补偿控制（如提交钩子对新增归档跑 `scan_credentials.py` 同类扫描） |
| B：等 #1474 稳定告警身份方案 | 撤掉本 PR 的两条目录 glob，保留其余改动（源码面处置、summary 截断）；归档与 fixtures 的告警按稳定身份逐条放行 | 在 #1474 落地前，main 的 DevSkim 持续红（约 55,065 个归档/测试数据键超出基线），红灯期间源码面的新增真问题会被淹没在噪声里；若临时把这些键灌进现行带行号的基线，会再制造一批随行号漂移失效的键 |

## 3. step summary 超限

现象：run 39379637868 的门禁把 55,146 行新增清单全部写入 `$GITHUB_STEP_SUMMARY`，共 6.3 MB，超过 GitHub 1 MiB 上限。

修复（`scripts/devskim_sarif_gate.sh`）：
- 新增可选参数 `--summary-max-new COUNT`（默认 200，必须是非负整数，否则 exit 2）。
- stdout（job log）保持全量。写入 `--summary` 文件的副本在新增数超过上限时只列前 COUNT 条，并插入一行说明：已列多少条、省略多少条，全量见本 step 的 job log 和 `<artifact>` artifact（原始 SARIF）。
- 新增数 ≤ 上限时，摘要与 stdout 逐字节一致。`errors` 数组、判红逻辑和退出码都没有改动。

用 run 39379637868 的真实 SARIF 和原基线重放（本机）：

| | 修前 | 修后 |
|---|---:|---:|
| 摘要大小 | 6,452,476 B | 21,739 B |
| 摘要列出的新增行 | 55,146 | 200 + 截断说明 |
| stdout | 6,452,476 B（全量） | 6,452,476 B（全量，55,146 行） |
| 退出码 | 1 | 1 |

测试（`tests/test_devskim_sarif_gate.py`）：
- 改了 #1372 的大差集用例：stdout 仍断言 12,000 行完整有序；摘要文件改为断言保留既有内容前缀、只列前 200 行（等于 stdout 前 200 行）、含截断说明和 artifact 名、结论为 FAIL、总大小 < 1 MiB。
- 新增 4 个用例：上限恰好等于新增数（摘要 == stdout）；超出 1 条（只截摘要）；上限 0（仍判红、不列行）；非法上限（exit 2）。
- 本机（Bash 3.2）：`python3 -m pytest -q tests/test_devskim_sarif_gate.py` 结果为 9 passed。Bash 5 以 CI 为准。

### 3.1 第二轮：评审 N1（前导零按八进制解析）

- 现象（非作者评审 `1593-claude-fable-5.md` N1，Codex P2 同点）：校验正则 `^[0-9]+$` 放行 `08`/`0200`，但 `(( ))` 把前导零当八进制。`--summary-max-new 08` 在比较处报 `value too great for base`，截断不生效，摘要仍写入约 6.4 MB；`0200` 被当作 128 条。
- 根因：bash 算术上下文对 `0` 前缀数字按八进制求值。
- 处置：通过校验后立即执行 `summary_max_new=$((10#$summary_max_new))`，按十进制规范化。判红路径与退出码不变；workflow 不传此参数（默认 200），CI 行为不变。
- 测试：新增 `test_summary_cap_leading_zero_is_decimal_not_octal`（9 条、上限 `08`：只列前 8 条，stderr 为空）和 `test_summary_cap_0200_means_two_hundred`（201 条、上限 `0200`：只列前 200 条）。对旧脚本实测这 2 例失败（旧脚本 `0200` 输出「listed the first 0200 … 73 more omitted」，即按 128 截断），新脚本 11 passed（本机 Bash 3.2）。
- N2（`strategy.rs:314` 抑制注释位于 `///` 文档注释内，会进入 rustdoc 文本）：按主控指示维持现状。挪到普通 `//` 行要新增一行，会让行号漂移；待 #1385 清理该待办时一并去掉。

## 4. 提交前模拟

本机 docker daemon 不可用，无法运行扫描镜像。所以用的是模拟方法：从真实 SARIF 中去掉排除目录与已抑制的行，按 rustfmt 后的行号修正 tb02v（580 改为 581），再用新基线重放门禁。结果只剩 4 条新增，正是 §1.3 改写的 4 处注释；模拟无法反映文字改写，扫描时这 4 条将不再命中。其余全部通过，`Fixed since baseline: 7944`。最终以 PR 上的 DevSkim run 为准（见 PR 描述）。

## 5. 遗留与建议转票

1. `rust/src/theta_v0/nautilus/strategy.rs:314/317`：限价腿 entry 价回填待办（#916 订正段）。**第二轮查重订正：已有票承接，不另开。** #1385（OPEN，TB-10-B）2026-09-12 的正文补记写明「`DS176209|rust/src/theta_v0/nautilus/strategy.rs|314` 由本票 OE09 旧策略入口/订单归属核查承接」。首轮写的「#1385 只是宽泛命中」不准确。314 的抑制理由已改为引用 #1385，317 是原有基线键。
2. `rust/src/trading/replay_comparator.rs:151`：完整 CERT/DIV/TERM 对拍，**已转票 #1607**（task）。查重：#1518 已关；#1532 把 `trading::replay_comparator` 归为冻结影子，去向归 #1304；#1528 扩展的是 `run_replay_double` 的业务对拍项，对象不同，不重复。151 的抑制理由已改为引用 #1607。
3. 基线键带行号，会随行号漂移失效。本次模拟显示 7,944 个基线键已失效（`Fixed since baseline`），需要清理，归 #1474 的稳定身份方案。
4. §2.3 的口径已由用户 2026-10-21 裁定：按精确条目入基线，不做目录排除。代价是基线文件变大、这批键同样带行号（归档文件一旦被改写，对应键会失效并以新行号重新判红），长期仍归 #1474 的稳定身份方案。

## 6. 希望非作者评审重点看的地方

- `scripts/devskim_sarif_gate.sh`：截断时用数组切片与插入说明行，对 Bash 3.2 和 Bash 5 都要兼容；同时确认 `errors` 与判红路径完全没动。
- 第三轮：workflow 只删了本 PR 加的目录 glob、#1111 原有排除未动；基线新增条目是否全部落在两个目录内（基线文件自身的例外见 §7）、是否只追加未重排。
- 对 #1372 测试断言的改动：全量断言保留在 stdout，摘要断言改为截断版本。这是有意变更，不是缩小覆盖。
- `.github/devskim-baseline.json` 的 11 条 entries 理由，以及 `generate-input.py:282` 的提交号核实。
- 66 处行内抑制的理由是否逐条成立，尤其是 DS162092 中 echo/日志行的措辞。

## 7. 第三轮：精确条目入基线的生成与断言

- 截至 main：`6c6acd5fe0`（PR 分支 rebase 后 head `49d33f0fc7`，即 main 加本 PR 的前两轮改动和第三轮 workflow 回撤）。
- 扫描来源：PR #1593 上 DevSkim run [39540955835](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39540955835) 的 `devskim-sarif` artifact（不入仓）。workflow 已无目录排除，结论 FAIL，基线外 55,147 个键（结果总数 91,458）。
- 生成与断言脚本：本目录 `exact_baseline.py`。键格式与门禁一致，只输出计数和分布，不输出匹配文本。

### 7.1 基线外键的分布（run 39540955835，入基线前）

```
$ python3 exact_baseline.py report devskim-results.sarif .github/devskim-baseline.json
sarif unique keys: 63335; baseline keys: 8212; outside baseline: 55147
outside baseline, in the two directories: 55101
  rule DS117838: 863
  rule DS137138: 20
  rule DS148264: 3
  rule DS149435: 1
  rule DS162092: 1426
  rule DS172411: 3
  rule DS173237: 52692
  rule DS176209: 7
  rule DS187371: 1
  rule DS197836: 85
  dir fixtures: 411
  dir review-results: 54690
outside baseline, the baseline file itself: 0
outside baseline, elsewhere: 46
```

### 7.2 两个目录内的 55,101 键：按用户裁定入基线

`exact_baseline.py write` 将这些键排序后追加到 `findings` 末尾，并新开一条 `updates` 记录：issue 1588，PR 1593，`source_head` 49d33f0fc7，`actions_run` 39540955835，`added_findings` 55101，`rule_counts` 同上，reason 写明用户裁定，并指向本文件、`classify-excluded-output.txt`、`credential-scan-output.txt`。原有条目未重排；`git diff --numstat` 只有新增行，外加 1 行（原末元素补逗号）。

本目录本轮新写入的文件（本台账和 `exact_baseline.py`）在本轮扫描中没有告警，基线中不含这两个文件的条目。（订正：原句称这批键覆盖了它们，与事实不符，见 `reviews/1593-tail-b3b1a7707c.md` N3。）

### 7.3 目录外的 46 键（rebase 带来，非裁定范围，单独一条 updates 记录）

rebase 后 main 新合入的代码有：#1406 `75022b3b9c`（TB-03-C）、#1493 `24c2fa068f`（TB-02-X）。它们在两个目录外新增了 46 个告警，main 自身的 DevSkim 因此也是红的（如 run 39540603883 @ 6c6acd5fe0）。逐键核实全是误报：

| 文件 | 规则与条数 | 判定 |
|---|---|---|
| `.github/scripts/run-1406-c6a.py` | DS173237 ×6，DS440010 ×1 | 40 位常量经 `git cat-file -t` 核实为本仓 commit/tree；64 位是被评审输入的摘要；DS440010 那一行比较的是两个提交号环境变量，与 TLS 版本无关 |
| `.github/scripts/run-c7a-remote.py` | DS173237 ×4 | 同上（commit/tree 已核实；文件字节被 `goal1323-resume-20261017/c6a-contract-recovery-v2.md` 引用锁定） |
| `rust/src/economic_session/tb03c_book_tests.rs`、`tb03c_c5_frozen_tests.rs`、`rust/src/theta_v0/parser/tb1493_replay_tests.rs` | DS173237 ×3 | 测试期望摘要 |
| `rust/src/economic_session/service.rs` | DS197836 ×1 | 取摘要的对象是镜像字节，并列字段不参与摘要 |
| `s_session/browser/q3-consumer.js`、`tb01c-client.js` | DS172411 ×8，DS173237 ×2 | 与第二轮入基线的 11 条是同一批位置，#1493 改了这两个文件导致行号漂移；仍受打包摘要锁 |
| `s_session/s_socket_client.py`、`s_session/tests/test_tb03c_query_runtime_contract.py` | DS197836 ×2，DS162092 ×2 | 摘要对象是请求体/空串；测试使用回环端口 |
| `s_session/tests/tb03c_query_runtime.py`、`tb02x_runtime.py` | DS162092 ×8，DS173237 ×3 | 测试 harness 回环端口；40 位为已核实的 commit/tree；输入摘要锁 |
| `s_session/tests/test_tb03c_query.py` | DS117838 ×1，DS173237 ×5 | 测试期望摘要 |

处置：按精确键写进单独一条 `updates` 记录（`actions_run` 39540955835，`added_findings` 46，`entries` 逐键写理由），不改这些文件的字节。理由有三：
1. 它们由他线在当天合入；
2. 部分文件带评审绑定的 commit/tree 和摘要锁，行内加注会改变被锁字节；
3. 同期仍有他线在改，改字节容易产生冲突。

第一轮「源码面 81 条」的处置不变。

### 7.4 断言

```
$ python3 exact_baseline.py check devskim-results.sarif .github/devskim-baseline.json
sarif unique keys: 63335; baseline keys: 63359; outside baseline: 0
outside baseline, in the two directories: 0
outside baseline, the baseline file itself: 0
outside baseline, elsewhere: 0
last update added: 55101; keys outside the two directories (excluding the baseline file itself): 0
CHECK PASS
```

用同一份 SARIF 和新基线本机重放门禁，结果 `Gate conclusion: PASS`。`Fixed since baseline: 24` 是行号已漂移的旧键（包括第二轮那 11 条中被 #1493 改动挪位的）；只追加不删除，失效键的清理归 #1474。

唯一允许的目录外例外是**基线文件自身**：新键的路径文本可能被规则命中，例如既有的 `.github/devskim-baseline.json|860`。这类条目只有在下一次扫描后才能得知。若出现，用 `exact_baseline.py write … --self-only` 再追加一条记录；最终 PASS 的 run 和 check 输出见 PR #1593 的评论。
