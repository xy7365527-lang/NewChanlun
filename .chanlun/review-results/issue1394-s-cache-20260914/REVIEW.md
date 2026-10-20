# #1394 S 审计缓存独立审查

结论：**approve_bounded**。未发现本次 cache-only 修复的正确性或篡改安全阻断项；真实全量输入成本验收仍由主控完成，不能从本报告推导 9011 源输入已经通过。

- 审查者：`/root/tb02b_incremental_review`；2026-09-14。
- 固定 HEAD：`a6a83440e418016c575d1f6d75bfcd8da72debd7`。
- 唯一产品差异：`rust/src/bin/s_session_v2/cache.rs`，未提交，314 行新增、42 行删除。
- 最终 source SHA256：`468d24143ddf6a63cffd66059ce69dce31c3eeb6a1028a8e26e9bc4baa4aa254`。
- 此次 git diff SHA256：`ca314b6ace9aacbfd6e40560ad1d494fa16f29f7a9b5f382974338647dd881f0`。
- 本审查只新增 `.scratch/issue1394-s-cache/REVIEW.md`，未改产品、编译、启动服务或派发子代理。

## 概念层质询结果

通过。本差异处理进程内缓存保留策略，不改变缠论定义或公开/持久事实。授权口径是一个规范源字节总上限，root 优先，其余空间保留 decoded；它不是 RSS/allocator 限额。

预算不变量逐分支核对成立：`store_batches` 只用 `limit-root.bytes` 挑选当前 capture 中的条目（cache.rs:151）；可装入的新 root 安装后，`trim_batches` 驱逐至剩余预算（:216）；新 root 单独超限时保留上限内、identity 相同、全部 DeltaRow 前缀相同的旧 root，否则只移除旧 root（:202）。该分支保持此前已成立的总量约束；configure 本身会清空状态，未发现缩小 limit 后残留超限缓存的路径。limit=0 在 configure 和两个 store 入口均禁用缓存。

同一 id 的优先级不会复活旧值：`select_batches` 只从当前 incoming 取条目，旧缓存只提供排序优先级。单条装不下时继续试装后续条目，未再引入整池清空。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

**旧前缀与篡改安全。** `decoded` 仍比较当前捕获的精确原字节（cache.rs:85）；缓存未命中时，调用方仍校验长度、内容寻址、strict JSON。当前事务始终执行 `capture_reachable_batches`、`verify_control`、元数据/索引发布代/原始输入检查与 `AuditWalk::capture`（s_structure_session.rs:768、:817、:841、:899）。`resume` 仍要求 identity、全 DeltaRow 前缀和 `AuditWalk::resume_from` 同时通过（cache.rs:172）；后者核当前 raw、独立索引内容和出生代、对象原字段，全部成功后才移动游标（audit.rs:189）。随后仍运行 audit.verify/finish 和当前末代元组检查，成功后才 store_root（s_structure_session.rs:1010、:1055）。

新 root 超限时未重新调用 resume_from 来决定“是否保留旧 root”，不会形成绕过：保留只影响缓存驻留，下一次实际复用仍重新经过上述 fresh 审计门；旧 walk 不匹配只会回完整核验。decoded 在 root 被废弃时可以保留，因为其信任仍独立绑定当前原字节。未发现 Begin、ghost、损坏或跨连接检查被本差异跳过。

**复杂度与实现范围。** decoded 选择和 root 驱逐在 k 个条目上为 O(k log k) 级，包含 id 比较成本；没有逐条重复排序或历史值深复制。新增的成本函数、选择函数和驱逐函数服务于同一预算策略，未引入额外配置、抽象层或通用缓存框架，未发现需要阻断的过度实现。每次 fresh 全域捕获、root_cost 中 walk.source_bytes 的规范化遍历、旧前缀比较和失效后的完整审计仍存在；不宣称整体 O(1)、O(page) 或不受历史长度影响。

## 验证依据及限制

本次实际执行了 git 状态/差异、source SHA256 与 `git diff --check` 核验，并定点阅读 cache.rs、audit.rs、当前调用链和既有跨连接篡改测试。图工具对目标测试无命中后采用文件检索。

实施报告 `report.md` 记录 cargo fmt/build 成功以及 cache 过滤 **8 passed / 0 failed**（6 新测试 + 2 既有测试）；后来单独执行的两个旧测试是重复验证，不相加为 10。新增测试覆盖共享预算、单条过大跳过、root 增长驱逐、超大新 root 保留旧前缀、字节/identity/前缀失配、零容量与移出可达集。既有篡改测试代码实际覆盖跨连接修改/删除 raw、独立索引、Delta、batch、meta、控制消息及 foreign schema，并要求 accept/advance/recover 拒绝且持久内容 dump 不变（s_session_v2/mod.rs:1572）。上述测试运行结果引用实施方报告，本审查未重跑。

小型空 AuditWalk 测试只验证缓存分支，不能代替真实 gen58/268MB 临界或完整 9011 源输入。主控仍需完成相同输入、配置和正式入口下的新旧成本与公开/持久结果对照；本报告不以缩小输入、复用旧二进制或局部测试作为整范围完成。RSS、极端条目规模、整个服务路径的延迟上界均未证明。

结论：**PASS / approve_bounded（代码审查）**；没有要求修改产品源码的 blocking finding。
