# PR #1593 非作者独立评审（#1588 devskim 分片）

- PR：https://github.com/xy7365527-lang/NewChanlun/pull/1593（head `f89ad7cbad`，base `main`）
- 评审模型：claude-fable-5
- 评审日期：2026-10-20
- 评审方式：只读。拉 PR diff 与描述，读 `.github/devskim-baseline.json`、`.github/workflows/devskim.yml`、`scripts/devskim_sarif_gate.sh`、`tests/test_devskim_sarif_gate.py`、台账 `DEVSKIM-TRIAGE.md` 及全部被改源码；下载 main 失败 run 与本 PR run 的 SARIF artifact 独立重放门禁；本机（Bash 3.2）重跑门禁测试。未改代码、未推送、未评论 PR。

## 结论：PASS-WITH-NITS

PR 做到了票面要求的「修真问题或按规则排除归档文件，排除须附理由」，且没有掩盖问题、没有降低覆盖、没有误改行为。本 PR 的 DevSkim run 已转绿（0 条基线外新增），门禁判红路径与退出码未动，stdout 全量清单逐字节不变。两条 nit 不阻塞合入，可随后续票处理。

## 轴①：正确性 / 风险

### 没有掩盖问题
- 81 条源码面告警我用 main run 39379637868 的真实 SARIF 独立重算，规则分布与台账完全一致（DS162092 27、DS173237 23、DS172411 16、DS176209 6、DS137138 3、DS148264 3、DS197836 3）。逐条看了抑制理由：
  - DS173237 23 条全是 SHA-256 测试向量 / 内容摘要常量 / Git 提交号；`tb03b_runtime.py:91` 的 40 位值 `efc1ddc4…` 和 `generate-input.py:282` 的 `f95a7f3e…` 我用 `git cat-file -t` 核实都是本仓 commit。
  - DS162092 27 条全是本机回环地址（本地只读服务默认 host、测试 harness 绑端口、启动脚本打印 URL），不是调试残留。
  - DS172411 16 条的 `setTimeout` 回调都是箭头函数字面量，无字符串求值。
  - DS148264 3 条 `random.Random(BASE_SEED + k)` 固定种子统计采样。
  - DS176209：4 处改写只动措辞（「非 TODO」→「非待办项」等），语义不变；2 处真实待办（`strategy.rs:314`、`replay_comparator.rs:151`）保留原文并抑制，台账 §5 建议转票，没有偷偷删除待办。
- 没有关闭 DevSkim、没有删规则、没有放宽门禁判定。
- 11 条入基线键：全部存在于 main SARIF，且在 PR run 的 SARIF 中**仍被扫到**并被基线吸收（说明这三个文件没有被 glob 误排）。`generate-input.py` 的字节锁主张成立：`tb02d/MANIFEST.json` 的 `generator_sha256` 等于该文件当前 sha256（`a8ef0768…`）；`q3-consumer.js` 的锁在 `build_tb01c_browser.cjs:15-36`（逐输入 SHA-256 写入打包产物）。
- 目录级 ignore-glob：排除前的全量凭据扫描脚本与输出已入仓（强/弱特征均 0）。票面 #1588 明确允许「按规则排除归档文件，排除须附理由」，workflow 注释与台账 §2 均给了理由。作者主动在 §2.3 指出与 #1474「不能只凭目录放行」的张力并请主控裁定，这是诚实披露，不是回避。

### 没有降低覆盖
- `#1372` 大差集测试的改动：stdout 仍断言 12,000 行完整有序；摘要改为断言「前 200 行 == stdout 前 200 行 + 截断说明 + FAIL + < 1 MiB」。新增 `cap == count` 用例断言摘要与 stdout 逐字节相同。覆盖是扩展不是缩小。
- fixtures 目录只排 `*.json/*.jsonl`；目录内唯一的 `.py` 在 PR run SARIF 中仍出现（`DS173237|s_session/tests/fixtures/tb02d/generate-input.py|282`）。

### 没有误改行为
- 用 main run 的真实 SARIF + 旧基线分别跑旧/新门禁：**stdout 逐字节相同**（`cmp` IDENTICAL，6,452,417 B，55,146 行），exit 都是 1；新门禁摘要 21,680 B，含 200 行 + 截断说明行。
- 用 PR run 的 SARIF + 新基线跑新门禁：exit=0，`Fixed since baseline: 7944`，与 CI 输出一致；未超限时摘要与 stdout `cmp` 相同。
- 截断切片 `"${error_lines[@]:0:kept_end}"` / `"${error_lines[@]:rest_start}"` 在 Bash 3.2 下实测正确（9 个用例通过）；CI ubuntu（Bash 5）也通过。`errors`、`has_errors`、`gate_status` 的赋值路径未动，只是把输出拆成 `report`（stdout）与 `summary`（文件）。
- 行号漂移：抑制一律同行尾注，不新增行。唯一例外 `stroke_tb02v_tests.rs:580→581`（rustfmt 拆行），PR run SARIF 中该文件已无任何键，抑制生效。被改文件中其他旧基线键（如 `launch_s.sh:234-237`、`strategy.rs:12/52/109/…`）在改动前就已漂移失效，属于 7,944 条 dead key，台账 §5.3 已登记归 #1474。
- `bash -n launch_s.sh`、`node --check tb03a-client.js`、`py_compile` 7 个 py 文件本机通过；rust-check（含 `cargo fmt --check`）CI 通过。

## 轴②：是否满足 #1588 票面验收（本分片范围）

| 验收项 | 状态 |
|---|---|
| DevSkim 在 main 转绿（修真问题或按规则排除归档文件，排除须附理由） | **满足**：本 PR run 39383843793 PASS，0 条基线外新增；排除理由在 workflow 注释 + 台账 §2 + 全量凭据扫描证据 |
| LEDGER.md 逐条「现象、复现命令与输出、根因、处置」 | 本分片台账 `DEVSKIM-TRIAGE.md` 已具备四要素（§1.1 复现命令、§1.2 根因表、§1.3 逐条处置）；总 LEDGER.md 汇总由主控合并，不属本 PR 范围 |
| 每个修复非作者独审；squash 合入 | 本评审即独审；作者已声明不开自动合入 |

其余验收项（基线失败测试、CI 时长、内存）不在本分片范围。

## 逐条发现

### N1（nit）`scripts/devskim_sarif_gate.sh:109-113` / `:249` — `--summary-max-new` 前导零未按十进制规范化
- 理由：校验正则 `^[0-9]+$` 放行 `08`/`0200`，但 `(( ))` 算术按八进制解释。实测：`--summary-max-new 08` 时第 249 行报 `value too great for base`，截断条件不成立，摘要仍写入 6.4 MB（exit 仍为 1，而非参数错误的 2）；`--summary-max-new 0200` 被当作 128 条截断。
- 影响：workflow 不传此参数（用默认 200），CI 实际不受影响；只是手工调用的边角。Codex 也报了同一点（P2）。
- 建议：校验后加 `summary_max_new=$((10#$summary_max_new))`，并补一个 `08` 用例；或在正则里拒绝前导零。

### N2（nit）`rust/src/theta_v0/nautilus/strategy.rs:314` — 抑制注释写进了 `///` 文档注释
- 理由：`/// 映射（TODO）。… // DevSkim: ignore DS176209 …` 中的 `// DevSkim…` 会原样进入 rustdoc 渲染文本。功能无影响，DevSkim 也按行生效。
- 建议：转票清理该待办时顺手去掉；或把抑制挪到该段 doc 之后紧邻代码的普通 `//` 行（但那会新增一行并漂移行号，所以当前写法在本 PR 语境下是合理取舍）。

### 观察（不计发现）
- `chanlun/review-results/`（无点前缀，319 个 git 跟踪文件）不在新 glob 覆盖内，PR run 中仍有 24 条键，全在基线里。不影响本 PR，但主控裁定 #1474 目录策略时可一并考虑这个历史目录。
- 新 glob 同时把本 PR 新增的 `scan_credentials.py`（内含凭据前缀正则）排除出扫描，与排除归档的决定自洽。
- 台账 §3 的字节数（6,452,476 / 21,739）与我重放的（6,452,417 / 21,680）差 59 B，是 `--baseline`/`--retention-days` 等参数字符串不同造成的摘要头差异，不是逻辑差异。

## 我实际复核过的事实与命令

1. `gh pr list --head eng-debt-1588/devskim` → PR #1593；`gh pr view 1593 --json body,files`；`gh issue view 1588`；`gh pr checks 1593`（DevSkim pass 55s、rust-check pass、test pass、s-observer pass、fixture-drift pass）；`gh run view 39383843793` → event=pull_request、head `f89ad7cbad`、success。
2. `gh run download 39379637868 -n devskim-sarif` → `main/devskim-results.sarif` sha256 `a1b1dee8d321…2901`（与台账一致）；`gh run download 39383843793 -n devskim-sarif` → PR SARIF。
3. 用门禁同款 jq 键提取：main 91,438 条结果 / 63,333 唯一键；对旧基线 `comm -13` → 55,146 新增；去掉 review-results 与 fixtures json → 81 条，规则分布与台账 §1.1 一致。
4. `comm -13 old-baseline new-baseline` → 11 键；这 11 键在 main SARIF 与 PR SARIF 中均存在。81 − 11 = 70 条抑制/改写键在 PR SARIF 中 0 条残留。PR SARIF 对新基线：0 新增、7,944 fixed。
5. 本机 Bash 3.2.57：`/usr/bin/python3 -m pytest -q tests/test_devskim_sarif_gate.py` → 9 passed；`python3 -m unittest` 同样 OK。
6. 真实 SARIF 重放：新门禁（main SARIF + 旧基线）exit=1，stdout 6,452,417 B / 55,146 行，摘要 21,680 B / 200 行 + 截断说明；旧门禁同输入 stdout 与新门禁 `cmp` IDENTICAL。新门禁（PR SARIF + 新基线）exit=0、`Fixed since baseline: 7944`、摘要 == stdout。
7. 边角实验：`--summary-max-new 08` → 第 249 行八进制报错、摘要 6,452,412 B；`0200` → 按 128 截断（N1）。
8. 事实核实：`git cat-file -t f95a7f3ebc0ce84ec2d2f8a8d71b5030b19f89b9` → commit（#1473 合并提交）；`git cat-file -t efc1ddc4d6c015f4f8c6d44f904d704a88308b46` → commit；`tb02d/MANIFEST.json.generator_sha256` == `shasum -a 256 generate-input.py`；`build_tb01c_browser.cjs:15-36` 对 `q3-consumer.js` 做 SHA-256 锁。
9. `bash -n s_session/launch_s.sh`、`node --check s_session/browser/tb03a-client.js`、`py_compile` 7 个 py 文件均通过。
10. 读 `gh api pulls/1593/comments`：Codex 唯一发现即 N1，已独立复现。
