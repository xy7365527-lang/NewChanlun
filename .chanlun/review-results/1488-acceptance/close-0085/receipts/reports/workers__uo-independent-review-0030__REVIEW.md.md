# 收据 workers/uo-independent-review-0030/REVIEW.md

- 原件：`workers/uo-independent-review-0030/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`4bbfccae62dd353b41f3b2694a03c9b8e18938fc225a2dfd4ff4ffe028d769f5`
- 字节数：16808；行数：223

## 关键行（行号: 原文）

```
1: # #1488 统一所有权预算补丁复审（0030）
4: - 返修基线：`4022f12079`（0028 评审对象）
10:   - #1488 主控裁定：P2-A；fixed_radix :755/:775 追认；0028 两条请裁的 (a)(b)；P2-β 须查明来源
13: ## 结论：**PASS_WITH_FINDINGS**
15: 0028 的 P1-α 已真正闭合，并有负控。P2-β 的 16,640B 已查明，并写入提交说明和 r14 断言。P3-β、P3-γ、P3-δ 均按要求落地。P1-1 依主控裁定 (a) 以「物理 Ledger 组 U + 来源不明拒绝 + E 侧保守」作为本票终态，其前提「P1-α 真拒并有负控」已经满足。
17: 本轮没有 P1 或 P2 级问题，只剩下 P3 观察项。其中 P3-α 改了一半：Ok 路径下实测值在结构上恒为 0，负控只模拟了「无持有者」这一种形态。
19: ## 一、0028 各条状态
23: | P1-α discard 后 refresh 洗白 stray | **RESOLVED** | 详见下文 |
24: | P1-1（0028 PARTIAL 余项） | **RESOLVED（按主控裁定 (a) 终态）** | 详见下文 |
25: | P2-β 16,640B 来源 | **RESOLVED** | 详见下文 |
29: | P3-δ 冷根残留 fail-closed | **RESOLVED（按主控裁定 (b)）** | 详见下文 |
31: **P1-α**
42: **P1-1（0028 PARTIAL 余项）**
43: - 主控 (a) 的前提是 P1-α 修到位且有负控，已满足（见上）。
47: **P2-β 16,640B 来源**
48: - 镜像 `NativeNodeBeforeP2a`（s_content.rs:3820 起）与 b6969af4cc 的 `NativeNode` 逐字段一致：tag、body、source_permit:Option\<NativeReservation\>、proof、owner_dirty、owner_next。
50: - 实测（20261020-034415-e2cc1bdf10.log）：R14_P2B_DROP 为 index 234,000B / 1040 槽 / 225→241，layout_drop=16,640。claims 111,334 + 16,640 = 527,048 − 399,074。
71:   1. 注入锁：driver.rs:1176，reruns=0 / 对照=1，错误文本包含「不自动重试」「重启writer进程」，CI PASS。
77: ## 二、0026 各条状态（于 62bad42a1b）
81: | P1-1 U 聚合残差 / 同父来源不明须拒 / 同父拒绝保留池 | **RESOLVED（按主控裁定 (a) 终态；逐项映射转 #1572）** | 详见下文 |
82: | P1-2 keeper 父≠候选父按物理 live 计 | **RESOLVED** | 详见下文 |
83: | P2-A 正文双计 | **RESOLVED** | 详见下文 |
89: **P1-1 U 聚合残差 / 同父来源不明须拒 / 同父拒绝保留池**
91: - 来源不明须拒：seal 中 attested≠L0 即拒（request_audit.rs:205-209）。经 discard 重试也不再被洗白（P1-α）。
94: **P1-2 keeper 父≠候选父按物理 live 计**
96: - 锁 p1_2（audit_owner.rs:3574）在 CI 与本地均 PASS。
98: **P2-A 正文双计**
110: **P3-b start_on 结果类别**：本轮未改动；锁 p3b_start_on_keeps_pre_p2_2_result_categories 在 CI PASS。
116: ## 三、Findings
118: ### P1
121: ### P2
124: ### P3
129: - 被漏检的形态：若来源物理许可以 P2-A 前的真实形态回归，即由 NativeNode 持有、列入 `native_permit_inventory`，那么 returned==held，指标报 0，C5 不报错。
134: - 位置：audit_owner.rs:3920-3936，`SEALED_BEFORE_P2A=527_048`。
137: **P3-ζ 冷根残留报错后 writer 不自动退役（观察，不违反裁定）**
139: - 现象：残留分支返回错误后，进程继续接收后续请求。泄漏的许可挂在已析构的旧根 Ledger 上，不进入新根的 T。处置完全依赖运维按说明重启。主控 (b) 只禁止自动重试，没有要求退役。如果希望把「重启」变成机械保证，可以在该分支 `retired.set(true)`，这属于后续工作，不在本票范围。
144: ## 四、评审项 1–7 重做
146: 1. **反例 A–F**：PASS。
148:    - C（来源不明须拒）在直接分区和 driver discard 重试两条路径上都会拒绝（P1-α 锁加负控）。
149: 2. **前轮 findings**：PASS。
150:    - 0028：P1-α、P2-β、P3-β、P3-γ、P3-δ RESOLVED；P1-1 按裁定 RESOLVED；P3-α PARTIAL（P3）。
152: 3. **assert 保护**：PASS。
155: 4. **r14 恒等**：PASS_WITH_FINDINGS。
158:    - 三条原负控未改，CI PASS。
159: 5. **业务语义**：PASS。生产行为变更全部在 native feature 内：
164:    - 新增「S3WriterResourcesInvalid 冷根残留分支」与「start_on 结果类别」延续 0028 结论，本轮未改顺序。
166: 6. **冷根重跑**：PASS。
170: 7. **测试证据**：PASS。
171:    - CI run 39321742113：headSha=62bad42a1b3a…（gh 核对）。
172:      - job `uo (s_session)` success，locks.tsv 为 1 PASS / 53 NOT_LISTED；
173:      - job `uo (s_session,s_capture_native_testonly)` failure，54 锁 50 PASS / 4 FAIL(101)；
192: ## 五、cross_request_source_cache.rs:486 门控核查
201: - 结论：该核对与被核对的对象处在同一构建面，不存在「生产有 native 捕获却缺核对」的组合。命名中的 `_for_testonly` 只是沿用了 feature 名，并非仅测试可用。
203: ## 六、证据清单
214:   - 20261020-033252-df48e8eb6c（P1-α）
218: - 实测日志：034415-e2cc1bdf10（R14_P2B_DROP）、035403-97ef7714e9（P24_SEAL_CENSUS 四组）
```
