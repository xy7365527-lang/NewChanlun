**PR #1449 固定提交 DevSkim 有界分诊**

结论：现有门禁确实失败，原始 16,195 条扫描结果不等于 16,195 个新漏洞。按门禁实际语义，基线外有 **7,961 个唯一位置键**：**7,950 个新增文件键、3 个原有代码行号漂移键、8 个未变文件旧键**。此次静态分诊对 7,960 个键的具体扫描声明有反证；另 1 个旧限价腿映射待办保留 `needs_review`，没有据此确认安全漏洞。没有发现本次新增的真实敏感凭据用途或弱加密调用。

本结论不改变执行门禁、不等于全仓无秘密，也不允许把失败检查当成成功。用户对固定 head 的“检查通过且内容不变后合入”条件尚未满足；本报告不授权调整 baseline、ignore、扫描政策或合入。其他 CI / CloudCode 问题由主控按各自证据独立判定。

**固定输入与清点口径**

- 基准：`2212eba31e0b7419907e622e79f02f7c2cc37706`；head：`750f933bddffc8251a880555b9beae2cdce75690`。本次只读 `git rev-parse` 确认工作树 HEAD；49 个 PR 文件范围来自已保存 `PR1449-FILES.json`。
- Actions：run `34706796160`；扫描及普通 artifact 上传成功，门禁最终失败（主控已核 live 状态）。本工位未重跑扫描器或产品测试。
- SARIF：52,355,864 字节；SHA256 `ec12437325f9f198e23ed3b803a0e21580564445ff4c1ebf723984e811f96e2b`，本次实算一致。
- 门禁读取 `scripts/devskim_sarif_gate.sh` 和 `.github/devskim-baseline.json`；机器报告保留两文件 SHA256。键为 `ruleId|第一个位置的URI仅去前导file://|startLine`，缺省为 `?` / `0`。不含列号、消息或严重性，不解码 URI，不因静态分诊判作误报自动豁免。
- raw **16,195** → unique **16,150**，同键多列等重复合并 **45** 条。baseline **8,201** 条且无重复，其中当前仍出现 **8,189** 键；差集 **7,961** 键对应 **7,975** 条 raw。baseline 中本次未出现 **12** 键只作信息，不自动等同真实修复。

| 规则 | 新增文件 | 旧行漂移 | 未变文件旧项 | 基线外合计 |
|---|---:|---:|---:|---:|
| `DS117838` | 417 | 0 | 0 | 417 |
| `DS148264` | 0 | 0 | 3 | 3 |
| `DS162092` | 38 | 3 | 0 | 41 |
| `DS173237` | 7,492 | 0 | 0 | 7,492 |
| `DS176209` | 0 | 0 | 5 | 5 |
| `DS187371` | 1 | 0 | 0 | 1 |
| `DS197836` | 2 | 0 | 0 | 2 |
| 合计 | 7,950 | 3 | 8 | 7,961 |

JSON 的 `raw_result_inventory` 保留全部 16,195 条到唯一键的映射；`new_gate_keys` 全枚举 7,961 键及原 raw 索引；`findings` 对 7,975 条 raw 各保留一项，不丢重复列命中。每项含字段、实际位置、类别、证据及摘要验证层级，未复制疑似凭据原值。

**逐类证据与判断**

| 类别 | 具体用途 | 唯一键 / raw | 静态结论 |
|---|---|---:|---|
| G01 | 归档 SHA256 / Git 对象标识 | 7,491 / 7,500 | `not_actionable` |
| G02 | 附件索引 artifact_key | 413 / 413 | `not_actionable` |
| G03 | 单行压缩证据容器被宽匹配 | 1 / 1 | `not_actionable` |
| G04 | 归档 Git 提交日志被宽匹配 | 2 / 2 | `not_actionable` |
| G05 | 捕获HTTP响应 fixture 内摘要、正文或回环地址 | 3 / 8 | `not_actionable` |
| G06 | GUI / 历史TestOnly测试回环地址记录 | 34 / 34 | `not_actionable` |
| G07 | 原有localhost代码仅行号移动 | 3 / 3 | `not_actionable` |
| G08 | R9隔离测试自建HTTP进程 | 3 / 3 | `not_actionable` |
| G09 | 压缩证据文本中的XEX及低熵哈希模式 | 3 / 3 | `not_actionable` |
| G10 | 未变的退役统计实验随机数 | 3 / 3 | `not_actionable` |
| G11 | 未变的历史TODO或已完成订正文字 | 4 / 4 | `not_actionable` |
| G12 | 未变的限价腿entry映射待办 | 1 / 1 | `needs_review` |

`DS173237`：差集中的全部 raw 命中均为带引号的 40 / 64 位十六进制。40 位共 **105 个唯一值**，全部通过 `git cat-file --batch-check` 证明是现存 Git 对象（22 commit、80 blob、3 tree），不是把外形像 SHA 当成证明。64 位共 **2,876 个唯一值**；逐项字段用途为 SHA256、来源/报告/二进制/manifest 摘要，或文件路径到摘要的映射。数组中的裸值已核在 Git 命令 argv、profile_hash 比较或捕获响应中。具名依据见 `native-codex-r7-r12/README.md:3、:7`、`COMMIT-DECLARATIONS.json:6`、`r7/READING-LOG.json` 的 `human_code_full` / `read_events`。

“用途已核”与“原字节重算”分开：从 R7 / R12 阅读清单所指的 4,548 个现有具名文件及附件，补做重算覆盖 **2,611 / 2,876 个唯一 SHA256 值**，对应 **6,537 / 7,065 个 64 位 raw 命中**。其余 265 个唯一值仅按明确摘要字段和归档上下文分类；没有声称这些历史摘要原字节全部重新验证，也没有因此扩张到历史全图重审。逐 raw 的 `digest_verification` 标明差别。

`DS117838`：413 键在 `EVIDENCE-INDEX.json` 的 `artifact_key`。已逐行验证 **artifact_key == sha256**，且映射到附件，不存在额外秘密前缀。另 1 键跨越 `EVIDENCE-ATTACHMENTS.json:1` 序列化容器；285 个 xz+base64 附件全部解码，合计 **84,509,910 原字节**，字节数、SHA256 与内容地址键全部一致。`review-r3.json:323` 和 `review-r4.json:25` 的 2 键跨越归档 Git 日志，完整读取后确认是提交元数据和说明。剩余 1 键跨越 `s_session/tests/fixtures/r12_consumer.json:1` 的捕获响应 JSON，属于 G05。

`DS162092`：新增文件的 38 键分成 GUI 记录 33、历史 TestOnly argv 1、R9 测试代码 3、fixture 同行键 1（含 6 个 raw 回环地址）。`r9_contract.py:70、:77、:81` 分别在自建测试端口绑定、就绪轮询及请求；`:89` 开始终止自己创建的进程。fixture 的 `kind` 明写捕获响应、不执行新网络请求，六项正文 SHA256 全数重算相等；`r12_consumer.cjs:10、:16、:46` 读这些字节并用本地 fetch 返回对象。`GUI-RESULT.json:49` / 各 `url` 和 `review-r5.json:555` 起的具名 TestOnly argv 是观察记录，不是新的生产远程 debug 入口。

剩余 3 个 `DS162092` 基线外键来自旧行漂移，已用 base→head 全文件 equal-block 对拍确认文本逐字不变，且对应旧键在当前 baseline：

| 文件 | baseline 原行 | 当前行 | 实际用途 |
|---|---:|---:|---|
| `s_session/launch_s.sh` | 236 | 238 | echo 展示快照 API 回环地址 |
| `s_session/launch_s.sh` | 237 | 239 | echo 展示状态 API 回环地址 |
| `s_session/s_readonly_server.py` | 351 | 1223 | 既有 `--host` 默认 127.0.0.1 |

`DS187371` 唯一键和 `DS197836` 的 1 键都在 `review-r3.json:1674` 的 `gzip+base64` 证据 data。前者仅匹配其中 `XEX` 三个字符；后者跨越编码文本，位置不是加密模式或对时间作哈希的调用。具名 `probe-results.json` 解压后 **4,595,732 字节**、SHA256 一致并可解析 JSON。另一个 `DS197836` 键跨越 `EVIDENCE-ATTACHMENTS.json:1` 中的 SHA256 解码说明及压缩文本，同样没有可执行哈希调用。这里证明的是所报词法命中的性质，不声称压缩证据内容已接受完整秘密审计。

**8 个未变文件旧项**

| 精确键 | 当前静态判断 |
|---|---|
| `DS148264|analysis/p3_random_gate_control.py|197` | 未变的退役统计实验随机数；`not_actionable` |
| `DS148264|analysis/p3_random_gate_control.py|251` | 未变的退役统计实验随机数；`not_actionable` |
| `DS148264|analysis/p3_random_gate_control.py|344` | 未变的退役统计实验随机数；`not_actionable` |
| `DS176209|analysis/p3_random_gate_control.py|141` | 未变的历史TODO或已完成订正文字；`not_actionable` |
| `DS176209|rust/src/theta_v0/nautilus/strategy.rs|104` | 未变的历史TODO或已完成订正文字；`not_actionable` |
| `DS176209|rust/src/theta_v0/nautilus/strategy.rs|113` | 未变的历史TODO或已完成订正文字；`not_actionable` |
| `DS176209|rust/src/theta_v0/nautilus/strategy.rs|19` | 未变的历史TODO或已完成订正文字；`not_actionable` |
| `DS176209|rust/src/theta_v0/nautilus/strategy.rs|314` | 未变的限价腿entry映射待办；`needs_review` |

`analysis/p3_random_gate_control.py:30` 已声明退役、不参生产准入，`random.Random` 用于 bootstrap / 择时统计；`:141` 的文档 TODO 后 `:175` 起已有实际实现。[#1316](https://github.com/xy7365527-lang/NewChanlun/issues/1316) 本次只读确认 CLOSED，承接的是冻结声明，不是 DevSkim 豁免。

`strategy.rs:19、:104、:113` 命中“非 TODO / 原 TODO 已兑现”等历史订正。`:314` 所指限价 entry 映射确为既有未完成能力，函数 `:316` 起恒返回 None；保持 `needs_review`，不把它宣告为安全漏洞或已修复。[#1385](https://github.com/xy7365527-lang/NewChanlun/issues/1385) 本次只读确认 OPEN，可作为旧策略迁移上下文，但不等于该扫描项已销项或能由 #1371-B 包办。

**覆盖边界与可审阅处置**

已全枚举原始结果、唯一键、双向差集、所有基线外 raw 对应行存在性、所有基线外类别和字段用途。人工阅读按具名上下文和类别进行，没有把脚本核对说成 7,492 处逐行人工安全审计。已基线化的 8,189 个当前键只清点，不再重审。根 `SECURITY.md` 是未填写完成的模板，未拿它臆定部署边界；分类依据是本次具名文件和入口用途。

本报告可供后续审阅形成 **7,950 个新增文件精确键**的误报登记候选，以及 **3 个旧行映射**候选。8 个旧键保持单列；特别是限价腿待办不应顺手吸收为“安全已解决”。这些候选不是修改授权，本工位未写 baseline / ignore / 源码，未改 Git / GitHub 状态，未重跑扫描器或产品测试。当前 gate 会继续因 7,961 键失败；未经相应批准及真实检查成功，不可在固定 head 上声称合入前件完成。

如果后续决定改精确基线或代码，会形成新 head，需要按更新后的具体授权处理；重跑完全相同检查本身不会改变已有差集。任何新政策取舍均未在此裁定。

机器报告：`PR1449-DEVSKIM-TRIAGE.json`。该文件保留完整映射与逐项静态证据，MD 为其可读摘要。
