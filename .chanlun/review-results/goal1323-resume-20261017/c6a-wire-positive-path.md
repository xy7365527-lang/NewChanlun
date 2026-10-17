# #1406 C6a 全 41 步 wire 正控可行性只读审计

日期：2026-10-17。名分：绑定 #1406 / #1323 的工作草稿。对象：`.sandcastle/worktrees/resume-1406-c6a` 当前源码及已封存 `c6a-semantics/synthetic-wire-boundary/` 两份响应。

**结论：在冻结 RAW、41 步顺序、现有完整图像 oracle 和 1 MiB 正文容量均不变的条件下，全 41 步合法成功 wire 正控无法构造。** 依据是这些条件自身要求的最少原字符串字节数，非把替身总大小当成产品结果。最终 WE7 公共快照至少重复冻结 RAW 114 次；仅这些 JSON 字符串值就是 1,298,346 B，大于正文上限 1,048,576 B，也大于含 HTTP 头的 raw 上限 1,064,960 B。仍未运行真实 owner/Q，不能宣称观测到具体生产失败步骤、错误、耗时或恢复结果。

## 实测与可复核推导

只对已封存约 2.7 MB 原件做标准库 JSON 解析，没有重造约 144 MB 全集，也没有执行产品、网络、cargo/rustc 或访问外盘。

冻结 `s_session/tests/fixtures/tb03c_allocation/RAW.json` 为 10,069 B，SHA256 `5b56870f8801ffc205683972158c286cfea25bc7672efd732300ac97d89c84e9`。按正式 `s_socket_client.py:18` 的 canonical 编码，作为一个 JSON 字符串值为 **11,389 B**（包括引号及原换行/引号所需转义）。重排外层键或去空白不能缩减这部分；压缩 RAW 本体会改变冻结原字节及摘要。

| 原件 | 正文字节 | E / N1 / N2 / N3 的 raw_utf8 次数 | 仅 raw_utf8 值字节 |
|---|---:|---|---:|
| first.http-response.bin，we7-018 | 1,181,905 | 6 / 35 / 23 / 24 | 1,002,232 |
| maximum.http-response.bin，we7-020 | 1,533,387 | 6 / 35 / 36 / 37 | 1,298,346 |

两份原件中每一个 `raw_utf8` 的 SHA256 均等于上述冻结 RAW。首个的字符串下界本身尚未过帽，**不能用本证明断言最早失败必在 we7-018**；最终的下界已独立过帽。

### 114 的来源不依赖替身私有字段

`tb03c_query_runtime.py:1023` 的 `execute_we7` 读取冻结 RAW，依次处理六事件：前三个分别 Capture 后应用到对应一个 B；后三个分别 Capture、Resolve、应用到三个 B。`:1080` 一带锁定 21 主请求。`:981` 的 `We7Expected.check` 核 E 和每个 B 的精确事实集合，`:900` 的 `records` 核每条原件的 `raw_utf8 == self.raw`，并核 B 的 Capture 完整图像、Resolve 完整图像等于先前实读 E 图像。`:410` 的 `check_snapshot` 又要求公共每域图像和 owner 完整图像 canonical 字节相等。

因此，在最后一步成功后的公共完整向量内，必需份数如下（只数互不重叠的 `raw_utf8` 值）：

| 位置 | 份数推导 | 份数 |
|---|---|---:|
| E 当前 recorded | 六次 Capture | 6 |
| 三个 B 当前 recorded | 每个 B 一次开账加三次净额应用 | 12 |
| B 原件的 Capture 证明 image.recorded | 开账三次分别含 1、2、3 条；后三事实的 Capture 图像含 4、5、6 条，分别进入三个 B | `(1+2+3)+3*(4+5+6)=51` |
| B 原件的 Resolve 证明 image.recorded | 三次 Resolve 分别含 4、5、6 条，分别进入三个 B | `3*(4+5+6)=45` |
| 合计 | `6+12+51+45` | **114** |

`114 * 11389 = 1298346 > 1048576`。此下界还没有计算 `raw_utf8` 字段名、括号、其他原事实字段、账目、收据、cut、公共身份、摘要、snapshot_token 或 HTTP 头。真实生产附加字段也完全不计入这个下界。因此这一证明无须假设替身的全部图像、金额或收据已通过生产准入。

## 生产和替身的具体差异

1. 正式 `economic_session/service.rs:145` 构造 E 原件还含 `source_received_time`、`source_producer`、`origin_message`、`origin_request_hash` 等；替身 `test_tb03c_query_runtime_contract.py:818` 一带只造精简原件，`test_semantics.py:33` 的 Backend7 再补 token/来源时间/回执域等。它不是正式 owner 序列化全字段副本。
2. 正式 `service.rs:336` 将 Capture 对应完整 E commit image 附为 `e_commit_proof.image`，与上述最小计数一致。正式 `tb03c_allocation.rs:440` 的 Resolve 证明除 image/receipt，还含 commit_seq、previous_root、commit_ns、previous_previous_root、previous_commit_ns、**previous_image**、request；替身 `test_tb03c_query_runtime_contract.py:850` 只存 image/receipt。因此替身缺少真实证明材料，不能通过“补齐正式字段”把已有响应变小。下界没有计 previous_image，避免依赖额外生产细节。
3. 正式 `service.rs:714` 的 ReadView 返回选择 cut 的完整 image；`economic_query.py:512` 保留该 image，`:530` 的 snapshot 组合完整 domains，`:517` 的 `_store` 核完整 body 容量；`economic_http.py:107` 又核整个 reply envelope 的上限。没有读取到“正式 Q 自动去重/只回摘要而 oracle 仍接受”的路径。
4. 替身 `test_semantics.py:206` 一带直接从 owner views 拼完整公共响应，用正式 canonical/reply_envelope 编码，但不先走正式 Q 的 `_store` 容量拒绝；所以它能落下超限成功字节供拒绝测试。正式 Q 若走到同一完整成功形状，容量门不会允许将它作为合法成功响应发送。**这属于条件式源码判断，尚非产品实跑失败证据。**

## 可行推进边界

本次没有发现保持全部冻结条件的合法全 41 步正控补法。不可通过更短随机身份、去空白、补真实证明字段、替换时钟或生成另一份完整替身消除原字符串下界；返回 PartialVector/Unavailable/容量错误也不能满足 `check_snapshot` 与全 41 步成功合同。去重引用、分页或有压缩层的 wire 会要求同步修改现有 decode、完整 image 对拍或公共协议，不能暗作测试工厂内部优化；缩减 RAW/计划、删除字段或放宽帽均越过本次约束。

建议将此**冻结输入 × 完整图像协议 × 既有容量**的冲突提交原合同责任面裁定，继续保持 `CONTRACT_READY=False` / 全 41 步成功未证。在裁定前可以完成独立时钟锚、hash-before-exec、监督器等互不依赖工作；重跑同一 144 MB 替身或反复扩大合成工厂没有证据收益。没有自行变更冻结合同或授予运行/关票资格。

## 本次源码指纹与未验证范围

| 源码 | SHA256 |
|---|---|
| s_session/economic_query.py | `ab5801d8e0ca18a8d41473561b082546756f0e964e5495eb01bd9dab5b6cd373` |
| s_session/economic_http.py | `436a43c808a2d215029cd19bee7086d7ee7f3c72dd91d0a5ed80d61b47f3e630` |
| rust/src/economic_session/service.rs | `8caf49a3b3f8dffcdeca14d73ddf0ef3b5ef84e37bb352a89b5cd5cb45c54f2d` |
| rust/src/economic_session/tb03c_allocation.rs | `42a4a361b3c63eee5c2d96b109ff8734dfcd9f49f495b3d2ef6fd102b5cfc114` |

driver/contract tests 有独立时钟锚工位在飞；本次仅引用当前业务断言，未修改、未冻结其整文件新哈希。没有重跑已完成的 10/14 测试，没有启动正式服务、HTTP、Rust 编译、重型回放。全 41 步真实执行、Linux 资源/取消/恢复与全图验收均未验证。本次唯一写入为本报告。
