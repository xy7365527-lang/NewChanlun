# 核验独立 GitHub App 身份

此命令检查 NewChanlun DeepAgents 1742 能否用自己的 installation 身份只读访问 NewChanlun。成功时输出 App、installation、仓库 ID、只读权限和到期时间，不输出 token。它不启动模型或工蜂，不读取旧 run 的批准，不执行发布。

## 运行核验

把用户下载的 PEM 留在仓库外。不要把文件内容放进命令、环境变量、聊天或配置。命令只接受文件的绝对路径，且要求当前用户持有普通文件，文件权限严格为 600 或 400。任一祖先目录存在 .git 文件或目录时拒绝，不只限当前 worktree。

在仓库根运行：

```sh
cd /Users/silencehan/.codex/worktrees/deepagents-pilot-1742/NewChanlun
VENV=/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv
PYTHONPATH=.sandcastle/deepagents-pilot "$VENV/bin/python" -m pilot1742.github_app \
	--private-key "/用户明确提供的仓库外路径/app-private-key.pem"
```

现有环境已包含锁定的依赖。新环境先安装本目录的 `requirements.lock`。

成功输出含 `"verified": true`。缺私钥、权限过宽、身份不符、仓库不符或 GitHub 拒绝时退出 2。命令不尝试用户凭据。若私钥权限不符，只调整该明确文件的权限后再试，不修改整个下载目录。

## 身份与权限

缺省参数绑定本票用户创建的 App。它们是公开标识，不是凭据。

- `--app-id` 为 `5486877`。
- `--app-slug` 为 `newchanlun-deepagents-1742`。
- `--installation-id` 为 `175617978`。
- `--repository-id` 为 `1150335160`。
- `--repository` 为 `xy7365527-lang/NewChanlun`。

客户端先用明确 PEM 签名的 App JWT 核 App 和 installation，再请求限定到该仓库的 installation token。核验用 token 将全部六项权限缩到 read，即 actions、metadata、contents、issues、pull_requests、statuses。客户端随后核 token 实际可访问的仓库只有目标仓库，并回读仓库身份。

JWT、PEM 字节与 installation token 只在调用进程内存。token 到期前 60 秒重新认证，客户端关闭时丢弃内存 token。客户端不把 token 写到文件、环境或子进程，也不修改共享 gh 登录。GitHub token 最长有效期由服务端决定，核验输出包含到期时间。

HTTP 只访问 `https://api.github.com`，禁用环境代理和 netrc，拒绝重定向。TLS 使用显式 SSLContext 和 certifi CA，保留证书与 hostname 校验，不读取 SSLKEYLOGFILE。HTTP 错误仅显示状态码，不打印响应正文或请求认证头。此入口不调用 gh、git、SSH 或 keychain，`GH_TOKEN` 和 `GITHUB_TOKEN` 对它无效。

## 本单元之后

此入口是迁移接入的身份核验，不给模型增加写工具。新执行循环、真实人批认证、旧消费者交接、精确 head 发布和 Studio 写操作仍按迁移 SPEC 独立验收。旧 v1 的 run、CLI 与 pilot_acceptance 合同保持原样。App 安装和本命令通过均不构成 main 合入或生产切换批准。

认证流程依据 [GitHub installation 认证说明](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/authenticating-as-a-github-app-installation)。
