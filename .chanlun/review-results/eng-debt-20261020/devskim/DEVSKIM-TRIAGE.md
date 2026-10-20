# #1588 devskim 分片：DevSkim 门禁红灯处置台账

- 分支：`eng-debt-1588/devskim`（基于 `origin/main` 1d70db893f）
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
| `.chanlun/review-results/**` 归档 + `s_session/tests/fixtures/**/*.json{,l}` 测试数据 | 55,065 | ignore-glob 排除（§2），排除前全量凭据扫描 0 命中 |
| 源码面（含 fixtures 目录里的 1 个 `.py`） | 81 | 66 条行内抑制、4 条改写过时注释、11 条按精确键入基线（§1） |
| step summary 超限 | — | 门禁新增 `--summary-max-new`（默认 200），摘要只列前 200 条并指向 job log 与 artifact；stdout 保持全量；判红逻辑与退出码不变（§3） |

真问题：81 条源码面告警中没有安全缺陷。2 条 DS176209 是真实的待办项，已抑制并建议转票（§5）。**未发现真实凭据。**

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
| DS176209（TODO） | 注释里含 TODO 字样 | 4 条是已兑现的过时措辞；2 条是真实待办 | 过时措辞直接改写；真实待办抑制并建议转票 |
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
| DS176209 | `rust/src/theta_v0/nautilus/strategy.rs:314` | 行内抑制：已登记待办（限价腿，#916 订正段），建议转票 |
| DS176209 | `rust/src/trading/replay_comparator.rs:151` | 行内抑制：已知简化版待办（#1518/#1304），建议转票 |
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

## 2. 归档与测试数据目录的 ignore-glob

新增 glob（`.github/workflows/devskim.yml`，注释已逐类写理由并指向本文件）：

```
**/.chanlun/review-results/**
**/s_session/tests/fixtures/*.json, **/s_session/tests/fixtures/*.jsonl
**/s_session/tests/fixtures/**/*.json, **/s_session/tests/fixtures/**/*.jsonl
```

不确定 DotNet glob 的 `**/` 是否匹配零层目录，所以根层与子目录两种写法都加了。fixtures 只排除 `.json/.jsonl` 数据文件，目录内代码（`tb02d/generate-input.py`）仍在扫描范围内。

### 2.1 理由

- `.chanlun/review-results/**`：冻结的评审/证据归档，是绑定工单的工作底稿（宪法 §5），不被任何 CI job 构建、导入或执行。其中 DS173237 占 88,595 条。除 2 条外，匹配文本全是纯十六进制（SHA-256、Git 对象号，形状分布见 `classify-excluded-output.txt`）。剩下 2 条（`issue1340-publication-20260909/payload/inputs/LEGACY-TICKET-DISPOSITION.json:4238`、`RESIDUAL-TICKET-PLAN.json:3362`）已人工看过上下文，均为 `sha256` 字段。其余规则（DS162092 1,467、DS117838 889、DS197836 85 等）出现在归档的报告正文与日志里，同样不执行。
- `s_session/tests/fixtures/**/*.json{,l}`：机器生成、受哈希锁的测试数据（manifest、oracle、schedule），不可手改；行内注释也会破坏 JSON 格式和摘要锁。

### 2.2 排除前凭据核实（全量，不是抽样）

```bash
python3 .chanlun/review-results/eng-debt-20261020/devskim/scan_credentials.py .
```

覆盖：git 跟踪的 1,812 个文件原文、15,130 个 zip 成员、535 个 base64(+xz/gzip) 附件（全部解码成功）。检查项为强特征（GitHub/Anthropic/OpenAI/AWS/Slack/Google/GitLab 固定前缀、PEM 私钥头、JWT）和弱特征（键名=值形态）。结果：**强特征 0，弱特征 0**。扫描器先用植入的假值自测，确认能命中。

分类阶段曾有 2 个前缀命中（`issue1371-tb01-b/native-codex-r13-r14/EVIDENCE-ATTACHMENTS.json`，DS117838/DS197836 各 1）。核实后它们都落在随机 base64/xz 附件数据里，是偶然出现的前缀。剔除 blob 并解码附件后重扫，结果为 0。另用宽松的键名扫描做了一遍，只找到领域 token（如 `snapshot_token`、`begin_token`、`token_schema`、`forged-token`），都不是凭据。本节只报路径与规则，不报任何值。

### 2.3 与 #1474 的口径张力（请主控裁定）

#1474（OPEN）已获批的方向是「稳定告警身份」，并写明「不能只凭字段名、目录或 40/64 位形状放行」。本分片按任务书要求，对归档与测试数据用**目录级** ignore-glob。为不越过 #1474 的口径，排除前对两个目录做了全量（非抽样）凭据扫描，并在 workflow 注释里逐类写了理由。即便如此，这仍是按目录放行：今后写进这两个目录的新文件不会再被 DevSkim 扫描。建议由 #1474 的稳定身份方案接管这两个目录的长期策略（例如在提交钩子里对归档目录运行同类凭据扫描）。

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

## 4. 提交前模拟

本机 docker daemon 不可用，无法运行扫描镜像。所以用的是模拟方法：从真实 SARIF 中去掉排除目录与已抑制的行，按 rustfmt 后的行号修正 tb02v（580 改为 581），再用新基线重放门禁。结果只剩 4 条新增，正是 §1.3 改写的 4 处注释；模拟无法反映文字改写，扫描时这 4 条将不再命中。其余全部通过，`Fixed since baseline: 7944`。最终以 PR 上的 DevSkim run 为准（见 PR 描述）。

## 5. 遗留与建议转票

1. `rust/src/theta_v0/nautilus/strategy.rs:314/317`：限价腿 entry 价回填待办（#916 订正段）。tracker 没有专票（#1385 只是宽泛命中），建议转票。314 已行内抑制，317 是原有基线键。
2. `rust/src/trading/replay_comparator.rs:151`：完整 CERT/DIV/TERM 对拍。#1518 已关，母图 #1304 仍 OPEN，建议在 #1304 下开子票。
3. 基线键带行号，会随行号漂移失效。本次模拟显示 7,944 个基线键已失效（`Fixed since baseline`），需要清理，归 #1474 的稳定身份方案。
4. §2.3 的目录级排除口径，请主控结合 #1474 裁定。

## 6. 希望非作者评审重点看的地方

- `scripts/devskim_sarif_gate.sh`：截断时用数组切片与插入说明行，对 Bash 3.2 和 Bash 5 都要兼容；同时确认 `errors` 与判红路径完全没动。
- workflow ignore-glob 的范围：是否误排了代码文件；`**/x/**/*.json` 是否匹配零层目录。
- 对 #1372 测试断言的改动：全量断言保留在 stdout，摘要断言改为截断版本。这是有意变更，不是缩小覆盖。
- `.github/devskim-baseline.json` 的 11 条 entries 理由，以及 `generate-input.py:282` 的提交号核实。
- 66 处行内抑制的理由是否逐条成立，尤其是 DS162092 中 echo/日志行的措辞。
