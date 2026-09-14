# PR #1453 最终候选 DevSkim 差异分诊

**候选 `e7fe54e8cd041e18ea8e473a41a5621cbe90b693` 的真实 gate 为 FAIL；与上一候选 `4408b0c233` 的全部 34,783 条 result 多重集完全相同，新增/消失均为 0。** 本次不是直接套用旧扫描：已独立下载新 run 的完整 SARIF、日志、check/job 元数据，并核 checkout tree 和日志全部基线外键。

[DevSkim run 34765582923 / job 103745837887](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34765582923/job/103745837887) 的 scanner、artifact 均成功。gate 于 2026-09-13 15:30:46–15:30:53 UTC 运行 **7 秒**，因 **26,521 个现役 baseline 外键**失败；全部日志键与完整 SARIF 差集按排序逐项相同。随后 summary 上传另报上限 1024k、实际 3122k，发生在 gate exit 1 之后；修复显示容量本身不会使告警门通过。

实际 checkout `22f7e0101503c4a1ce4e958400ac53271c84ca92` 的父提交恰为已合入 base `65298686cf3e7c9e94e0b9ceeb83aebad2b5530b` 与本 head；GitHub commit API 的 tree `c906d23276d6263a67363b146b8f70958a77b81b` 与固定 head tree 完全相同。两次 scanner 都是 `1.0.90+fb2d676ce4`；DevSkim gate/baseline/workflow/Dockerfile/entrypoint 五件在 652、4408、e7fe 三提交均逐字一致。

| 对比 | 完整结果 | gate 键 | 新增 / 消失 |
|---|---:|---:|---|
| 4408 → e7fe | 34,783 → 34,783 | 34,708 → 34,708 | 0 / 0 |
| 652 → e7fe | 34,663 → 34,783 | 34,589 → 34,708 | 121 键 / 2 键；122 条 / 2 条 |
| 现役 baseline 外 | 26,402 → 26,521 | 净增 119 键 | 旧 26,400 键原样，另 2 键行移 |

完整 result 比较只统一 URI 的 `file://` 前缀，message、properties、全部 region 坐标、snippet 等其余字段完整参与，保留重复次数。最新两条变更路径为 `rust/src/theta_v0/classifier/bsp.rs` 与 `CI-ADDENDUM.md`，**两文件均无结果命中**，测试增加六行未产生 finding 行移；两文件之外全部 34,783 条结果也相同。相对 652 的 31 个变更路径之外，34,659 条结果完全相同。

相对 652 的 122 条新增结果，与上一份 `ci-devskim/TRIAGE-ROWS.json` 的完整结果摘要多重集逐项相同；六个命中文件在 4408/e7fe 也逐字相同。因此复用的是已核具体命中的用途证据，不是旧门禁例外：

- **DS173237：118 键 / 119 条。** 证据索引 113 个直接文件 SHA256、2 个教义 fixture SHA256、目录完整性 2 个摘要字段、2 个 Git 身份。113 份直接证据文件和 2 个教义 blob 已重算；116 项目录公共静态摘要已重算。目录 `source_sha256` 仅绑定已签元数据，未递归重算原 SPEC；没有把这项用途绑定写成全来源复算。
- **DS162092：1 个新 TestOnly 本机地址。** `s_session/tests/tb02a_runtime.py:129` 连接自己的 `127.0.0.1` Q，端口已有严格配置验证。另 1 键为 `s_readonly_server.py:1153→1187` 原样行移。
- **DS172411：1 个原样行移。** `tb01c-client.js:188→413` 是 `setTimeout(() => abort.abort(), ...)` 函数回调，没有字符串代码执行。

摘要字段精确定位、原件路径和逐条验证见既有冻结 `ci-devskim/TRIAGE-ROWS.json`；新 `REUSED-TRIAGE-BINDING.json` 固定其 SHA、122 条完整结果相等及六个源文件相同的证据。两个消失键就是上述旧行移，不能当作修复了两个安全问题。

**本次相对 652 的新增中，未识别实际安全缺陷或凭据泄露；新增未分类数为 0，无证据支持当场修改产品来修复这些告警。全仓实际风险仍 UNKNOWN，继承 needs_review 保持原状态。** 删除/变形摘要、扩 baseline/ignore 或修改规则不能冒充安全缺陷修复。旧 PR 或此前候选的批准不自动覆盖本次 head；仍由用户对本候选及真实红灯作具体决定。本报告不替代产品独评、main 验收或批准。

原始 SARIF 来自 artifact `10320392046`，130,736,419 字节，SHA256 `a5dc0bcb6fe487a017e8dcb2ad0276db29fa464b1377e0f99efdb08fdf7d2007`。容器原字节与 4408 不同；完整 result 多重集相同不等于容器 SHA 相同。原始日志/SARIF/ZIP 与下载命令、UTC、exit 和 SHA 全保留在本目录，`EVIDENCE-INDEX.json` 索引，`REPORT-FREEZE.json` 固定报告。大型原件留仓外，无需为归档重复入 Git或再触发扫描。本次未修改源码、配置、忽略规则、旧冻结件或 GitHub，未重跑/取消 workflow，未启动产品服务或测试。
