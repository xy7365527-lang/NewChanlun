**结论：当前约束下不能把 DevSkim 红灯真实修成绿色；可按用户条件授权处理固定 head 的例外，不能称为已修复。**

对象是 PR #1449 的 `256cd822e0d872f1fb65d433ca0b15b01fa621a2`。已复用完成的 9,233 键分诊，限定核现役 gate/workflow、固定版本官方规则以及旧 entry 映射待办。没有重读 65 MB 全枚举、重扫、运行产品或扩大安全审计；未写产品、基线、Git/GH、评论或启动 Prime。

最小充分依据是：**已冻结的新 R13/R14 文档单独就有 1,272 个基线外键**。固定规则已经实际匹配这些字节；固定 baseline 不含它们；现行 gate 遇任一基线外键即失败。因此，只要冻结报告、scanner/ignore 与 baseline 键语义保持不变，即使另把旧业务缺口补齐，文档键也会让当前检查继续红。没有找到既符合这些条件又能真正修成绿色的最小补丁。

**没有发现可通过修正计算错误解决的 gate 故障。**

`scripts/devskim_sarif_gate.sh` 取 `ruleId|首位置URI去file://|startLine`，然后比较当前键与 `.github/devskim-baseline.json`。固定 baseline 的具名 #1194 元数据明确：已有键冻结、只拦基线外新增、缺席键只作信息。这里的“新增”是相对这份已提交 baseline，不是相对上一版 PR head，也不是只看本 PR 修改行。当前 9,233 键和真实 run 日志一致；扫描/上传均成功，失败原因是差集非空。#1194/#1385 的在线票面由根并行核，本工位没有冒称独立读过最新全文。

把门禁改成只扫描 PR diff、忽略 `ManualReview`、自动信任摘要字段、过滤 SARIF 或用内容替代行号身份，都会改变现行判定。新增 baseline、批量 ignore、continue-on-error 或改检查状态也不构成代码缺陷修复。本次唯一行移已具备精确证明，但按现有语义仍是新增键；修正行移政策也不能消除上述 1,272 个不变文档键。

**官方固定规则解释了噪声，没有证明扫描器偏离规则执行。**

核对版本为 CLI `1.0.90` / SARIF driver `1.0.90+fb2d676ce4` 对应官方 commit `fb2d676ce475a47c0338966bebd97a47ae566572`，不是以最新版规则替代现役版本。

- DS173237 匹配引号内至少 30 位十六进制，只列全零串和两种 `InternalsVisibleTo` 场景等例外，不按 `sha256` 字段用途自动排除。DS117838 将含 secret/license/key/pass 的同行内容与长十六进制结合；`artifact_key` 和单行压缩容器会满足它。这与已核具名摘要、内容地址键及宽跨度相符。[官方 secrets.json](https://github.com/microsoft/DevSkim/blob/fb2d676ce475a47c0338966bebd97a47ae566572/rules/default/security/privacy/secrets.json)
- DS197836 用大小写不敏感的哈希算法名、贪婪中段和 Time 作匹配，并不证明存在哈希函数调用；现有新增命中的起止均位于 base64 数据内部。这是规则对证据文本的误报，不是低熵密码学执行已被发现。[官方 hash_algorithm.json](https://github.com/microsoft/DevSkim/blob/fb2d676ce475a47c0338966bebd97a47ae566572/rules/default/security/cryptography/hash_algorithm.json)
- DS176209 是 `ManualReview` 的 TODO 等可疑注释规则，其含义是可能存在未完成能力，不是已证实安全漏洞。[官方 todo.json](https://github.com/microsoft/DevSkim/blob/fb2d676ce475a47c0338966bebd97a47ae566572/rules/default/security/hygiene/todo.json)

改进这些启发式规则当然可能降低误报，但需要改变 scanner/规则并验证漏报边界；“所有 sha256 字段一律可信”也可能隐藏命名成摘要的真实凭据。把贪婪匹配换成非贪婪最多改变容器跨度，不会消灭 `artifact_key` 的既有词法匹配。当前没有证据指向一项不改既定规则政策、又能消除红灯的上游实现修复。不能把误报直接说成 gate 算错。

**`strategy.rs:314` 不能无新交易语义决定就关闭。**

固定源码 `rust/src/theta_v0/nautilus/strategy.rs:312–317` 明写 Order 没带 entry，`entry_tick_for` 实际恒返回 None。`:207、:234` 将其传给 `order_adapter::to_order_intent`；`order_adapter.rs:81–95` 在 Buy/Add/Sell 意图中直接令 `price_tick = entry_tick`。从 None 改为 Some(entry) 因而会改变市价/限价订单行为，不是注释整理。

更不能随手把 decisions 与 orders 按下标配对：`strategy.rs:217–218` 已明确二者非一一对应。真正修复至少要保留可追踪的 decision→order entry 归属，明确多对一/冲突及动作范围，并按既有契约核缺失 entry 的处理和订单意图验证。本票 TB-01-B 的持久结构会话工作没有提供这些交易接口的完成契约。删掉 TODO、换个词或仍返回 None 都不能关闭实际待办；任意取首个 entry、最近价格或统一限价则是没有依据的新决定。该项继续 `needs_review`，由原业务承接处理；这里既不确认安全漏洞，也不宣告实际影响已排除。

**可执行的处置边界**

范围内的诊断已足以排除“修个 gate 小错误即可真实变绿”这一方案；没有可提交的范围内产品修复补丁。保存固定 head、真实 DevSkim FAIL、9,232 个有反证键及 1 个旧 needs_review，再由根按用户最新“修不了则批准”的条件授权处理本次例外，是与当前证据一致的路径。若将来治理 baseline 或规则，应明确作为新政策/扫描器工作，不回写为本次代码缺陷已修复。

“新增 confirmed 扫描声明为 0”与“实际安全风险为 0”不同；后者在本报告保持未知。旧待办、基线内和扫描未覆盖面没有因例外授权自动关闭。

机器记录见 `DEVSKIM-REPAIR-FEASIBILITY.json`；固定上游规则原件及摘要位于 `work/DEVSKIM-REPAIR-FEASIBILITY/`。
