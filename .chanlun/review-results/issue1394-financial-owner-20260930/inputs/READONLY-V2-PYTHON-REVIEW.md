# #1394 经济只读 /2 适配独立 Python 评审

结论：Spec 未发现本次差异引入的行为缺陷；Standards 有 1 项 HIGH，按本工位明确的 Python 类型注解规则暂为 Block。此结论仅评价四文件候选补丁，不是 #1394 联合验收，也不是 main 合入许可。

固定基线 `909ccff13a`，候选 `abada33caefef1e3e03d1592352101b99c03ca1e`，差异为四文件 235 行新增、8 行删除。起始 `git diff -- '*.py'` 为空；实际评审使用已提交候选相对固定基线的差异。另核对根工位旧文件，生产四路径的旧实现与基线一致。评审不修改源码，不派下级，不运行正式服务。

## Spec

依据为根工位 `.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md:319-366`、`SPEC-ADDENDUM.md:7-9,29-33`，以及旧批准 `issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md:9-15,40-41,64-66`。根 Rust 的 `economic_session/store.rs:62-80,97-112` 与 `finance.rs:25-41,47-62` 用于跨实现配对。

- `economic_query.py:46-60,76-78` 先校验冻结 manifest 摘要，再选择版本。选择子 absent/null 保持 `/1`，只有两个已知文本版本可用，显式 `/2` 必须配 `tb03b-finance/2`，未知类型、未知值、跨版配对在 owner I/O 前拒绝。与 Rust 域版本和财务版本配对同义；Q 没有接管财务原件或资金政策校验。
- `economic_query.py:84-97,159-174` 保留精确获准源匹配及只读操作白名单，新增协议版本不借用 writer 身份。所有 owner 操作仍经同一 request/validate_reply 路径。
- `s_socket_client.py:70-105` 扩展 `/2` 的生产者域分派，保留 E、各不透明 B、Q 的 namespace、session/generation、epoch、payload_hash、causal_refs、message_id 全绑定。新增未知版本拒绝，未放宽 strict JSON、单帧、字节上限或总期限。
- `economic_query.py:196-211` 对 image 的财务版本使用冻结选择结果，保留 exact 外层、数组类型及完整 image_hash 核验；没有改写历史、证据或 root 的字节。
- `economic_http.py:14-33,53-59,103-107` 按该 Q 域版本核请求，响应沿原公共头配方生成。既有 finance_revision 如实反映 `/2`，仍是 `economic-discovery/1`，字段集未新增。
- `/1` 的 absent/null/显式选择、正常 `/1` 财务图像和旧传输判据均保留。拒绝未知或混搭配置是明确版本门，不是热替换旧账。

Spec findings：0。没有因为这 36 个测试而认定真实 Q/B 已接通，也没有授权或实施 browser/frontend 改动。

## Standards

[HIGH] 新增公共函数缺少输入和返回类型注解
File: s_session/economic_query.py:46
Issue: 新函数 `economic_revisions(manifest)` 的参数及二元组返回值均无类型注解。本工位 Python review 指令将“公共函数没有类型注解”列为 HIGH；该问题只针对新增函数，不追溯旧函数。版本选择与配对是跨语言合同入口，明确其输入对象及返回顺序有助于静态核查后续调用。
Fix: 增加明确输入类型及 `tuple[str, str]` 返回注解，例如 `def economic_revisions(manifest: dict[str, object]) -> tuple[str, str]:`；如采用只读映射类型则使用 `Mapping[str, object]`。这是工程类型缺口，未声称已有运行反例或安全绕过。

未发现本差异新增的注入、eval/unsafe load、秘密、资源泄漏、吞错、可变默认参数或无锁共享状态。协议输入的 exact `type(...) is ...` 判据用于禁止 bool 等子类混入，不要求机械改成 isinstance 而放宽 wire 合同。没有以函数长度或协议字面值单独制造行为缺陷。

## 已有验证与限制

复用实施者 `.scratch/issue1394-economic-readonly-v2-20260930/final.stderr`，末尾明确 `Ran 36 tests in 0.232s`、`OK`，包含新增 /2 正负例、旧 /1、HTTP 配方和原单帧/期限负例。审阅了日志与对应测试源码；遵从无新行为疑点不重复跑这 36 项。`compile.stderr` 为空，但单凭空 stderr 不认定独立静态分析通过。

本工位查 PATH 及当前 python3 模块，ruff、mypy、pylint、black、bandit 均不可用，未安装或运行这些工具。未运行正式 Q/B、财务三阶段、六故障、9011 双跑、浏览器或全仓扫描。工具缺失单列为验证限制，不能伪报 lint/typecheck 已通过。

## 四文件冻结指纹

以下 SHA256 在评审开始与结束两次核对一致，工作树相对候选四文件差异为空。

| 文件 | SHA256 |
|---|---|
| s_session/s_socket_client.py | 8fb63a14d131452e09b2ab3e4aeddef1c1cbf922ca641ec5a13a7db729b4a1e2 |
| s_session/economic_query.py | 089b284f3e30ce526274344f8a9fee2b9375d14c913de06a8895151c8a9768ed |
| s_session/economic_http.py | 1c88fac07338258fae1b18cf9e13e8eed5d0b6e4c38fe1393ac9f7ebc275fedd |
| s_session/tests/test_tb03b_query.py | de8770fe1776ac77d28e7d983e031ffab101cd077da51d8bf918bef5f4800653 |

Spec：0 findings；Standards：1 HIGH 类型注解缺口。后续补注解须另核固定候选；此报告保持原候选结论。
