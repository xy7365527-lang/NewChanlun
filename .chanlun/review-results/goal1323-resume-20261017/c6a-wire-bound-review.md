# C6a wire 容量约束独立审查

日期：2026-10-17。名分：#1406 / #1323 工作草稿。只读审查恢复树 `.sandcastle/worktrees/resume-1406-c6a`，并独立解析已经封存的两份约 2.7 MB wire。本次唯一写入是本报告。

## 概念层质询结果

**发现合同约束矛盾，原静态不相容证明成立；阻止将当前合同标为可运行成功合同。** 本结论不是生产实跑故障、不是最早失败步骤定位，也不是资金算法错误判定。按概念层先行纪律，不进入一般工程层质量审查。

被审报告为 `c6a-wire-positive-path.md`。未借用它的数字作为结论：独立精读当前 driver 的业务断言、正式 Q 编码/容量门、生产证明图像装配，并用标准库重算冻结 RAW 编码和封存 wire 路径计数。图工具查询 execute_we7/check_snapshot/We7Expected 返回零后，回退文件精读。根 CLAUDE.md、Serena manual 已读。

这里的概念是原件字节身份、完整证明图像、完整公共向量及响应容量，未涉及中枢/背驰等缠论教义判据。检索 `缠论知识库.md`、`definitions.yaml`、`docs/spec/`、`.chanlun/definitions/` 未找到改变本合同的相关定义；`.chanlun/genealogy/` 未找到 C6a/#1406 此项容量矛盾的既有裁定。没有将通用 JSONL 膨胀谱系当作本案授权或判据。

## 定义回溯和生产接线

下列行号以恢复树当前文件为准。

1. `s_session/tests/tb03c_query_runtime.py:1023` 的 execute_we7 读取冻结 RAW；实际 RAW 有六事件，前三件分别属于 chong-N1/N2/N3。`:1050` 至 `:1078` 前三件 Capture+单 B Apply，后三件 Capture+Resolve+三 B Apply，共 `3*2+3*5=21` 步，并显式断言 21。冻结 manifest 正好三个参与重。
2. `:902-914` 对 E/B recorded 精确数量、唯一 fact_key 集合逐项核验，每项 `raw_utf8 == self.raw`。`:981-1004` 核 E 累积 captures 和 B 累积 applies，不能只留最新事实。
3. `:885-895` 保存每次 Capture/Resolve 后完整 E 图像；`:919-930` 要求每条 B 原件的 Capture 完整证明等于当时 E 图像，后三件的 Resolve proof.image 也等于当时完整 E 图像。不能改成摘要或不同投影仍通过。
4. `:756-771` 每步请求当前及固定快照等；`:410-419` 要求 CompleteVector、全部域 Available，并对每域 image 与 owner 原图像做 canonical 完整字节相等。最终一步包含整四域图像，跨请求拆开不能通过现有断言。
5. `rust/src/economic_session/service.rs:145` 原件含原 raw_utf8；`:320-337` 在 B 原件装配 Capture 的完整 E commit image；`rust/src/economic_session/tb03c_allocation.rs:440` 装配 Resolve 完整 image，另有 previous_image 等本下界未计字段。`service.rs:714` ReadView 返回完整 image。
6. `s_session/economic_query.py:512` 保留完整 image，`:530-558` 合成完整 domains；`:515-519` 在缓存/回复前按 canonical(body) 检查 max_reply_bytes。`s_session/economic_http.py:104-109` 又对 canonical(reply_envelope) 检查同一正文帽。driver `:44-46` 固定 max_reply_bytes=1048576；`:702` 将 QUERY_RESOURCES 装入实际 Q 配置。不存在先压缩再绕过 `_store` 门的路径。

## 独立字节和互斥路径计数

冻结 RAW 文件 10,069 B，SHA256：

`5b56870f8801ffc205683972158c286cfea25bc7672efd732300ac97d89c84e9`

独立以 `json.dumps(raw_text, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")` 重算，结果 **11,389 B**，与 `s_socket_client.py:18-21` 正式 canonical 定义一致。包括字符串两端引号、原换行和引号的 JSON 转义；不包含 raw_utf8 字段名。这里未解析重写 RAW 内容，其原字节和摘要保持不变。

最终公共响应中四类路径互不重叠，`D` 表示 B 域，`i/j` 表示数组位置：

| 路径（均在 payload.domains 下） | 必需份数 |
|---|---:|
| `E.image.recorded[i].raw_utf8` | 6 |
| `D.image.recorded[i].raw_utf8` | 3*(1+3)=12 |
| `D.image.recorded[i].e_commit_proof.image.recorded[j].raw_utf8` | (1+2+3)+3*(4+5+6)=51 |
| `D.image.recorded[i].e_resolution_proof.image.recorded[j].raw_utf8` | 3*(4+5+6)=45 |

四类的叶路径不同。Capture 的 E 图像只含到该事件为止的 1…6 件原事实；Resolve 不增新原事实，其图像分别含 4/5/6 件。即使 Python 内存复用同一个 dict，JSON 树序列化也仍重复这些位置，不提供对象引用语法。

独立遍历两份原件，对每个 raw_utf8 值逐一断言等于冻结 RAW，按上述路径分类，没有未归类项：

| 封存替身原件 | 正文 B | E直接 | B直接 | Capture内 | Resolve内 | 全部原字符串 B |
|---|---:|---:|---:|---:|---:|---:|
| first.http-response.bin | 1,181,905 | 6 | 10 | 39 | 33 | 1,002,232 |
| maximum.http-response.bin | 1,533,387 | 6 | 12 | 51 | 45 | 1,298,346 |

maximum 的域计数为 E=6、N1=35、N2=36、N3=37，与源码推导独立相合。**计数佐证来自替身；成功合同的必需份数来自源码断言。** 没有假定替身包含全部正式字段或可通过真实 owner 准入。

## 最小不相容核及反例边界

完整下界为 `114*11389=1298346 > 1048576`，超正文帽 249,770 B；也超过含 HTTP 头的 1,064,960 B 接收上限。未计字段名、账目、收据、previous_image、身份或 HTTP 头。

更小的充分矛盾核甚至不需要 E/B 直接原件的 18 份：**Capture 内 51 份 + Resolve 内 45 份 = 96 份，仅字符串即 1,093,344 B**。其必要约束可缩为：

- 六事实/三 B 的冻结步骤与证明完整性，强制同一个响应里至少上述 96 个不同 JSON 字符串位置；
- 每个位置严格保留冻结 RAW，当前 canonical 表示每份 11,389 B；
- 一个成功正文容量最多 1,048,576 B。

在“只按原字符串份数”的整数下界中，最小超帽子集为 **93 份**：92 份=1,047,788 B，93 份=1,059,177 B。可从强制 96 份中任选 93 个不同路径，无须全部 114 份即可矛盾。这是该字节下界的最小份数核，**不是声称已证明整个应用约束集合的全局最小不可满足集**；其余字段可能让更少份数也超帽。

反例检查：第一份封存超限响应的原字符串只有 88 份，下界 1,002,232 B 尚未超过正文帽，所以本证明不能推出最早失败必在 we7-018。缩短身份或去除外层空白不改变 93 份下界；把原 RAW 重新格式化会违反原字节身份；PartialVector/错误响应不满足当前 CompleteVector 成功断言。仅 HTTP gzip 也无效，正式 `_store` 已在压缩之前检查未压缩 canonical(body)。

## 可保持资金、原件和帽的协议改造边界

以下是需独立协议裁定/实施/验收的方向，不是本审查授予改合同许可：

- 保持原件不可变及唯一权威来源，在 wire 层按内容摘要一次携带原字节，嵌套证明用有类型、有版本的引用。验证器必须逐项验证 hash/length、完整引用闭包、cut 和图像身份；缺项/错项/环/超深不得降级成功。不能把“有 hash”替代“拿到并核过原件”。
- 或采用固定向量/根绑定的分页或有界流式证明，所有页固定同一会话/代次/cut。重建结果仍须与原完整图像和原金融 oracle 精确一致，不能把跨页漂移拼成成功向量。当前单响应 snapshot/oracle 需要显式升级版本；旧协议退出条件一并定义。
- 每帧 1 MiB、已有总内存/缓存/时间/进程及隔离帽保持；新协议需要在这些帽内证明重建、验签/摘要、重试与恢复成本。不能靠一次性无界解压或把完整展开图偷偷移到预算外满足测试。若保留现有 `_store` 以展开 canonical body 计 1 MiB 的语义，单纯改线缆编码仍无解，必须明确改变存储/传输表示并独立核资源计账。
- 41 步业务序列、冻结 RAW/HAND、资金账目、逐事实完整来源及确定性要求不变。不得删字段、缩事实、降低 oracle、放宽帽或将拒绝改作成功。

当前继续保持 CONTRACT_READY=False；外层时钟锚、监督器、来源审计可独立推进，但不能替代这项协议闭合。本次未修改产品或 driver，没有 cargo/rustc、正式服务/HTTP、外盘访问或重型全集生成。

## 采样指纹及审查边界

| 文件 | SHA256 |
|---|---|
| s_session/tests/tb03c_query_runtime.py | bafc25a7bdeb899d32174e06f7c6ccfbc81d82f74d852876167bf22a12e4f395 |
| s_session/s_socket_client.py | 8fb63a14d131452e09b2ab3e4aeddef1c1cbf922ca641ec5a13a7db729b4a1e2 |
| s_session/economic_query.py | ab5801d8e0ca18a8d41473561b082546756f0e964e5495eb01bd9dab5b6cd373 |
| s_session/economic_http.py | 436a43c808a2d215029cd19bee7086d7ee7f3c72dd91d0a5ed80d61b47f3e630 |
| rust/src/economic_session/service.rs | 8caf49a3b3f8dffcdeca14d73ddf0ef3b5ef84e37bb352a89b5cd5cb45c54f2d |
| rust/src/economic_session/tb03c_allocation.rs | 42a4a361b3c63eee5c2d96b109ff8734dfcd9f49f495b3d2ef6fd102b5cfc114 |

driver 的时钟锚变更由其他代理持有，本审查只覆盖业务步骤与图像断言；以上为读取时指纹，不代表整文件独审通过。原报告的四个生产源码指纹均相同。未评估真实 earliest failure、产品运行时、Linux 资源/取消/恢复及全图完成资格。

结论：**静态不相容证明通过独立复核；当前成功合同 FAIL/BLOCK。工程层不适用，产品未实跑。**
