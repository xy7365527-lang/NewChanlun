# #1394 独立审阅03（正式BLOCK）

固定提交6cb6bbf33eb079f639abae2449ccc9b98d81a11c；真实Sandcastle run 4fc774b9-b14d-4924-bed5-feff2ef581ab，external session 01a14d70-516c-7e62-929e-7d7f8b0b1d37，2026-10-18T05:20:55.885Z实际退出，入口/worker均0。执行完成，内容结论BLOCK。

复审完成，结论阻断。固定提交6cb6bbf33eb079f639abae2449ccc9b98d81a11c相对dfdeda34886d4a8f0a3bfa7bc8e3f3930aab28f0的九项中，R-F3、R-F4、R-F6、N1、N3、N4、N5闭合；R-F5、N2各有一处可复现残余。F1/F2及原P0保护保持。本结论仅覆盖工具与本轮尾差，不代表整票或P11通过。

## 正式发现

- [MEDIUM][R-F5残余] s_session/tests/issue1394_acceptance/supervisor.py:240：每处理一条消息就立即采样，仍会误拒队列中尚未处理的合法登记。内存复现一：使用test_review01.setup_core，令101已出现，队列依次为合法HEARTBEAT、ACK_REQUEST、COMPLETE，时钟推进0.25秒；仅receive一次即报“存在未登记后代”，ACK尚在队列，owners为空、101仍存活且未被回收。复现二：101已登记，其102、103两个后代的合法REGISTER均排队；只登记102便因103未登记而失败，103仍存活。原work/heartbeat截止均未到。建议在原截止内建立明确的排队登记处理边界，再做后代全集采样；同时处理ack内部强制采样路径，保持身份验证及陌生PID拒绝，补上述两个顺序用例。
- [HIGH][N2残余] s_session/tests/issue1394_acceptance/captured_cut.py:256、294：闭包原回复被解析后丢失原始长度，单请求max_reply_bytes只与重新规范化后的长度比较。内存复现：取test_capture_bridge.fixture首个financial-closure，将请求帽设为原回复长度5940，在回复前加100个空格并更新捕获清单及合同摘要；回复仍为合法JSON且只有末尾一个LF，实际6040字节，导出却返回UNAUTHENTICATED_FACTS。5940字节正例也通过。session/reader/Job帽充足不能替代该请求自身的5940字节帽。建议保留原回复字节长度并按请求帽实核；若协议只接受规范编码，则同时验证原字节确为规范JSON加LF，不能以重新序列化缩小后的长度代替原件。

## 实际验证

- 前后HEAD均匹配指定提交，工作区无未提交改动；工具差异git diff --check退出0。全程未修改checkout或原封包，未执行Git写操作、开启工蜂或读取凭据。
- 已读取指定工作流文件、返修报告、九项矩阵、接线合同、来源角色与依赖交接。正式REVIEW-02及5条原探针的SHA分别准确匹配8d90c9c037aeaea13d826cf57c77d62f7d6fb8e04010b64d5f2dbac062baf7f5、51c16f90e41be4a66e3eb288b603c4cc14c29a5615858c8a2560b3a06d39642b，并读取原命令和输出。
- DELIVERY-FILES共52项的长度、权限和SHA全部匹配；25个Python文件匹配作者验证前后指纹并通过内存compile。普通/-O原输出摘要匹配，各含68个成功方法记录；本轮未重跑这些方法。
- TEST-REUSE列出的10个保护文件与dfdeda逐字一致；原driver 41、scheduler 27个测试方法均保留。F1固定合同量词及F2逐owner原请求有序全集校验未变，旧保护未发现回归。
- R-F3静态核实三处B提交时间先一致，再与独立E Reserve时间比较；绑定自检覆盖9/9/11拒绝与合法时间通过。R-F4核实解锁后接受、序列化后截止检查、持锁失败仅撤自身candidate、consume不退款；重开隔离全部candidate，正常活调用者可继续。其代价明确为不能自动恢复接受资格，不构成已实现崩溃恢复。
- R-F6核实先向全部已登记owner发TERM，再沿同一绝对期限wait，必要KILL及非阻塞reap不因前一owner迟到而跳过；迟到、异常或无法证明reap保留INCOMPLETE。复用已绑定的任意位置超期、已退出、存活及需KILL测试证据，未认证真实OS Probe。
- N1核实空串首游标、null拒绝与后页顺序；b46和固定C1的financial_reader.rs:745均要求文本。N3核实各capture显式绑定Q或session/reader及具名Job，4Q与17financial分账，ReadBudget独立。N4已移除捕获路径借用Voice帽，Voice原文件不变。N5仅取当前stage、whole及所属Job适用截止，历史stage不参与最早截止。
- 来源31项核对全部匹配，包含7个pin、19份协议快照及固定C1源码。3315、b46、C1均未冒充统一新product source；1488→1419→组合C1→新SHA及financial_reader组合语义重审要求保持。
- 仅对新疑点执行内存定向探针：两种排队登记顺序均复现误拒；闭包单请求帽的5940字节正例通过，6040字节超帽原件也被接受。未运行68/91/82/host123集合或旧ps/socket三项，未运行cargo/rustc、真实S/E/B/Q、Long、数据库、网络、浏览器或资金动作。
- execution_authorized仍为false。真实统一源、两角色ARTIFACTS、同域terminal/完整K/Voice认证、逐对象资源与准入，以及全历史、A/B、恢复、资金、隔离和独评仍未验收；这些既有产品义务未重列为新代码bug。前端归#1460，K4排除，相关图票及#1279保持开放。
