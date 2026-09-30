# #1394 正式 Controller Q3 入口工程审查

结论：**PASS**。固定候选 `71e473bd54f46c31159a742a2b3ac706ac04a7da` 相对 `08241aa12fa429245ba5132e6b6c2480f40029e4` 的三个文件已完整审阅。未发现有实际反例支持的 CRITICAL、HIGH 或 MEDIUM 工程缺陷。本报告仅为独立 Standards 轴，不替代 Spec、正式运行验收或合入批准。

审查者与作者、Spec 审查者分离；产品文件只读。依据为已提供 AGENTS.md、固定合同 §5.5/5.6.1、现有 Controller 生命周期合同及 code-review Standards 口径。图工具未索引本工位，故回退固定文件和 Git diff。无全仓扫描、服务启动、Browser、Rust 构建或 9011 操作。

- `s_service_control.py:469-498`：真实 fd 经 NONBLOCK 打开后以 fstat 验证普通文件；读取与严格 JSON 解码均在本次直接子进程中。目录、字符设备、指向 FIFO 的链接、缺失文件、非法 UTF-8、小数、NaN 和空对象全部拒绝，11 个独立探针全部通过、11 个 reader 全部回收。
- `:477-490`：使用当前 `sys.executable`，将候选模块所在绝对目录置于导入首位。恶意 cwd 同名模块未被导入。路径作为 argv 传递，无 shell 插值；没有断言承担准入。
- `:488-498`：父进程只认可三个完整固定 stdout 标记。它们只用于分派，不证明 Q21 合法；原资源路径交给原 Rust 入口冻结和校验。非零退出和 SIGTERM 被转换为拒绝，未触发环境、服务启动、Ready，也未改既存 q 收据。
- `:490,495,502-526`：使用原绝对 deadline 的剩余额度。已按 SHA 核验并复用 Spec 的真实 FIFO、CPU 解码超时及四阶段同 deadline 原件。另直接检查本机 Python 3.14 `subprocess.run`：POSIX TimeoutExpired 对本次 Popen 执行 kill、wait 后重抛；未使用进程组或扫描其他 PID。`:606-611` 已捕获 SubprocessError 并关闭控制器。
- `:506-525`：显式 /3 选择 Rust query-serve 和原 runtime_environment；无版本原文件保留 Python、Browser 与 economic 参数；未知版本拒绝；/3 与 economic 组合拒绝。无失败后换旧入口重试。所有既有 Controller 方法除 start_q 外均与基线 AST 相同，包括 nonce/PID、status、Ready、epoch/session、stop。

独立增量验证命令为 `PYTHONDONTWRITEBYTECODE=1 python3 .scratch/issue1394-q3-controller-entry-20261001/revision1/STANDARDS-probes.py`。原件见同目录 `STANDARDS-probes-output.json`。两个修改 Python 文件 AST 解析通过，fresh `git diff --check BASE CANDIDATE` 通过；ruff/mypy/pylint/black 的命令和模块均不可用，未安装。作者同字节 30/30 单测及 compile 证据复用，未为计数重跑。Spec SHA 索引内 13 项全部匹配，旧 FAIL、Spec 和作者原件均未修改。

固定输入 SHA-256：产品 `bac0956929d861b8a359ac187e2fd1d2dbd6adb96f2271701f9eaca860c6f1f9`；测试 `82618c3338fd8187cbde292265cff02762c784e1d34a77b270531ee4b173e000`；已批准合同 `fab912c6ebe3613969d65fc924368a1d5c9b343f0c3f7a123241645413771d30`。本报告不宣称完整 Consumer、完整资源域、S16、9011 或整图完成；根代理的 fresh 正式 start_q 实测另行验收。
