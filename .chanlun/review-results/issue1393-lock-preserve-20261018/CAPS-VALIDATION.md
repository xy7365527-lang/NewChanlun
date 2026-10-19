# #1393 根具名更窄帽：作者验证记录（非独审）

环境：macOS 本机，Python 3.14.6，仅标准库 unittest（另用 PyYAML 解析 workflow）；零网络、零真实子进程（测试将 `subprocess.Popen` 置为断言失败），未运行 cargo/rustc/rustup/lake/Lean/产品/浏览器。

命令（工位根目录）：

    python3 -B .github/scripts/test_issue1393_lock_prepare.py -v > .chanlun/review-results/issue1393-lock-preserve-20261018/CAPS-toy-final.stdout 2> .chanlun/review-results/issue1393-lock-preserve-20261018/CAPS-toy-final.stderr

结果：`Ran 80 tests ... OK`（完整输出见同目录 CAPS-toy-final.stderr/stdout）。run16 原有 47 项全部保留；其中 4 项按新语义改写而未放宽：记录完整性改为从摘录偏移取原文并对拍原 cache 偏移、摘录与原 cache SHA256；原「单文件/总读帽」两项改为「读上限先于打开」「registry 正文 total/files/items 边界」并新增「非 registry 原件不计入 registry 帽」；故障注入的 parse 阶段改挂流式扫描函数。

新增 33 项覆盖：根数值与保持值逐项（AST 读字面量）；合同逐层类型严格（single+1、float、True、0 代 False、未知/缺失嵌套字段、upload 计数口径、可变 @v4、compression≠0、消息帽改动、旧 schema/旧收据 schema、未知顶层字段）；runner 无网络导入、扫描函数无整读/子进程；单条原文等于帽完整与超 1 字节 proof_gap 且摘录无截断内容；大于 total 的 cache 文件因只摘所需记录仍完成且 `NOT_ARCHIVED`；total/files/items 等于帽通过、少 1 拒绝；跨文件累计项数；截断、非所需重复版本、缺 validator、空文件、坏 magic；扫描中途截止与信号；约束父与计数；容器公式对 Python ZIP_STORED 实测不低估；禁字符/非 UTF-8/ZIP64 门拒绝；收据预留为事前常数；目录/symlink 拒绝；写入超合计帽在打开文件前拒绝、等于帽通过；封包预核失败不写尾件不删原件；upload-gate 等于帽通过、少 1 拒绝且不写 FROZEN、请求在目录外；receipt-gate 合计等于/超 1、ACTUAL 超 4096、门后新增或改动散件、无冻结清单时仅主包未上传才放行；False 门、近截止、未知模式拒绝；workflow 两处上传固定 commit、compression 0、门调用与收据三件路径、无 RESOURCE_CAPS 文本解析、默认门 False。

另核：workflow YAML 可解析；两份 Python 源 AST 可解析；`CONTRACT-DRAFT.json` 字段集合与 FIXED+Git/摘要字段完全一致（无未知、无缺失），runner/workflow 摘要已更新，approval/review/prepared_* 保持 UNBOUND。
