# Secure Enclave 真人签批（#1742 D8）

执行器需要人批时写出一份待批请求。你运行 `newchanlun-approve sign`：终端先显示仓库、票号、范围、完整目标（commit SHA 或 SPEC 正文摘要）和批准或拒绝，系统随后要你用 Touch ID 或登录密码签名，再弹出 macOS 管理员认证窗口，认证后把这份决定发布到 root 所有的权威记录。**只有发布成功、且从权威记录读回确认的决定才生效。** 签名认证取消就不签名；签了但管理员认证取消或失败，终端会醒目地提示“未发布，尚未生效”，执行器也看不到它。

签名钥在 Secure Enclave 内生成，不可导出，访问控制是 `privateKeyUsage + userPresence`。每次签名新建 `LAContext`，`touchIDAuthenticationAllowableReuseDuration = 0`，签完即 `invalidate`。

权威记录在 `/Library/Application Support/NewChanlun/approval-receipts/<request_nonce>/<序号>.json`。目录和文件归 root 所有，只有以 root 运行的 `publish` 能往里追加。`publish` 先在同一 root 目录下的 `.staging/` 写临时文件，再原子改名进链目录，所以链目录里除了 `<序号>.json` 不该有任何东西；它只接受用信任公钥签过、恰好接在当前链头之后的下一条。执行器从这里读整条链，不以 GitHub 或任何用户可写文件为准。

## 一次性安装（由你本人操作）

1. 从已通过独审的那个提交构建，不从可被代理修改的工作树直接构建。把 `<COMMIT>` 换成独审报告里的完整 SHA。

   ```sh
   COMMIT=<COMMIT>
   SRC=$(mktemp -d)
   git -C /Users/silencehan/Projects/NewChanlun archive "$COMMIT" .sandcastle/approval-signing | tar -x -C "$SRC"
   cd "$SRC/.sandcastle/approval-signing/swift" && swift build -c release --product newchanlun-approve
   ```

2. 安装签批工具，建好 root 所有的权威记录目录。之后一律用这个绝对路径运行它。

   ```sh
   sudo -k install -d -o root -g wheel -m 0755 "/Library/Application Support/NewChanlun"
   sudo -k install -d -o root -g wheel -m 0755 "/Library/Application Support/NewChanlun/approval-receipts"
   sudo -k install -o root -g wheel -m 0755 .build/release/newchanlun-approve \
     "/Library/Application Support/NewChanlun/newchanlun-approve"
   ```

3. 生成签名钥，并把信任配置直接写进 root 所有的位置。信任配置来自本进程刚生成的钥，中间不落用户可写文件。

   ```sh
   "/Library/Application Support/NewChanlun/newchanlun-approve" enroll --label "Mac Studio Secure Enclave" \
     | sudo -k tee "/Library/Application Support/NewChanlun/approval-trust.json" >/dev/null
   sudo -k chmod 0644 "/Library/Application Support/NewChanlun/approval-trust.json"
   ```

   每条都用 `sudo -k`，每次要密码，安装过程不留 sudo 凭据缓存。终端打印的 `key_id` 应与 `cat` 出来的配置里的一致（配置是 0644，读它不需要 sudo）。钥句柄存于 `~/Library/Application Support/NewChanlunApproval/signing-key.se-handle`，只能在本机 Secure Enclave 里用，每次用都要你认证。

## 每次签批

```sh
APPROVE="/Library/Application Support/NewChanlun/newchanlun-approve"
"$APPROVE" status --request <请求文件>                    # 看这一请求当前生效的决定
"$APPROVE" sign --request <请求文件> --decision approve   # 或 reject
```

`sign` 自动接在权威记录当前链头之后，后一个决定覆盖前一个。发布通过 macOS 管理员认证窗口（AppleScript `do shell script … with administrator privileges`），窗口里写明票号、批准或拒绝、目标和序号；这份授权只属于那一个 osascript 进程，每次发布都重新认证。看到 `已发布到权威记录……现在生效` 才算生效。若看到 `未发布`，回执存在 `~/Library/Application Support/NewChanlunApproval/pending/`，可按提示运行 `"$APPROVE" publish-pending --receipt <文件>`，它先在终端显示回执内容，再弹出同样的管理员认证窗口。只在你自己刚发起签批时完成 Touch ID 或管理员认证，任何你没有发起的弹窗都取消。

## 首次真机验收

主控给你一份验收用的请求（`run_id=enrollment-check`，随机 nonce，任何执行器都不会发出同一 nonce，所以它不放行任何真实动作）。依次做：

1. `status`：应显示还没有已发布决定。
2. `sign --decision approve`，Touch ID 弹窗时取消。应无回执、不出现 sudo 提示；`status` 仍为空。
3. `sign --decision approve`，完成 Touch ID，管理员认证窗口出现时点取消。应提示“未发布，尚未生效”；`status` 仍为空。
4. 按提示 `publish-pending --receipt <待发布文件>`，核对内容后在管理员认证窗口认证。`status` 显示当前生效为批准（第 1 条）。
5. `sign --decision reject`。这一次必须再次弹出 Touch ID；若没有弹窗就签出来了，验收失败。完成管理员认证。`status` 显示当前生效为拒绝（第 2 条）。
6. 在本目录下运行 `python3 -m approval_signing status <请求文件>`，应输出 `rejected`，这是执行器读到的结果。

## 执行器接入

`approval_signing/decision.py` 的 `decide` 是唯一入口：

```python
decide(request, current_target, *, now, trust_path=DEFAULT_TRUST_PATH, authority_root=AUTHORITY_ROOT) -> Evaluation
```

- `request` 是执行器发出并保存的 `ApprovalRequest`：`repository_id=1150335160`、`ticket`、`run_id`、`claim_generation`、`scope`、`target`、`request_nonce=secrets.token_hex(16)`、`expires_at`（UTC 整秒，2000–2099 年）。`request.encode()` 写成请求文件交给你签。目标或 run、generation 变了就发新请求、新 nonce。
- `current_target` 是动作此刻真正作用的对象：`CommitTarget(<40 位 SHA>)`，或 `spec_target(spec_issue, GitHub 返回的正文原始字节)`。
- 返回的 `releases` 为真才放行。`rejected` 表示最新已发布决定是拒绝；`awaiting_human` 表示还没有已发布决定、批准已过期或目标已变；`needs_reconciliation` 表示权威记录有异常，停下交人。

`decide` 读该 nonce 的整条已发布链。路径任何一级不归 root 所有、组或其他人可写、或为符号链接，目录里有任何不是 `<序号>.json` 的条目（包括点文件），回执不是规范格式，签名无效，文件位置与内容不符，缺号或断链，一律返回 `needs_reconciliation`，从不跳过。信任配置缺失或不受 root 控制直接抛错。GitHub 可以转载回执供人查看，但执行器不以它为准。

## 字节格式 v1

```text
newchanlun-approval/v1
repository_id=1150335160
ticket=1742
run_id=t1742-run.1
claim_generation=3
scope=candidate_head
target=commit:120421fe338a9fcf8467b63aa7d6a28e3f871b8b
decision=approve
request_nonce=0123456789abcdef0123456789abcdef
sequence=1
previous_receipt=none
expires_at=2099-01-01T00:00:00Z
```

ASCII，LF 结尾，字段顺序固定，十进制无前导零，十六进制小写，`expires_at` 限 2000–2099 年。`spec_authorization` 的目标写作 `spec:<SPEC 票号>:sha256:<正文摘要>`。请求文件用表头 `newchanlun-approval-request/v1`，去掉 `decision`、`sequence`、`previous_receipt` 三行。解析后重新编码必须与原字节完全一致。回执是 JSON：`format`、`algorithm`（`ecdsa-p256-sha256`）、`key_id`（公钥 SPKI DER 的 SHA-256）、`payload` 与 `signature`（DER）的标准 base64。回执必须是规范形式：键按字母序、无空白、斜杠不转义、末尾一个 LF，即 Python `json.dumps(obj, sort_keys=True, separators=(",", ":")) + "\n"`，所以重复键或任何其他写法两端都拒。`receipt_id` 是载荷字节的 SHA-256。

## 能防什么，不能防什么

代理和工蜂与你同一个 macOS 账户运行，可以读写你的文件、启动程序，但 `sudo` 要你的密码。

能防的：代理不能自己产出生效的决定，因为签名要你在场认证，发布要你在 macOS 管理员认证窗口里当场认证；那份授权只属于发起它的 osascript 进程，别的进程拿不到。手工安装的 `sudo -k` 不用也不留缓存；平时别在代理也在用的终端里留 sudo 缓存。它改不了 root 所有的信任配置、签批工具和权威记录，所以删不掉、也回滚不了已发布的拒绝；在记录里塞进或改动任何文件都需要 root。换钥句柄只会得到验不过的回执。它诱导你在 Touch ID 弹窗上点了通过，得到的也只是一份未发布、不生效的回执。

不能防的有三类。第一，你运行的终端、Swift 工具链和 Python 环境与代理同一账户；构建产物在复制进 root 目录之前、执行器自己的代码和它保存的请求，都可能被同账户程序改动。固定提交构建、root 安装和执行器核对请求字段缩小了这个面，没有消除它。第二，`sudo` 密码或 root 权限一旦被别的程序拿到，权威记录就不再可信；root 删掉链尾也无法被发现。第三，代理可以用你的钥句柄触发 Touch ID 弹窗，也可以自己调用 osascript 弹出管理员认证窗口，两种弹窗的说明文字都由调用方写；在不是你发起的管理员窗口里认证等于交出 root。所以只认自己发起的弹窗。

## 开发检查

```sh
cd .sandcastle/approval-signing/swift && swift build -c release
./.build/release/approval-core-check                    # Swift 侧检查；加目录参数会重写跨语言夹具
cd .. && python3 -m pytest -q                            # 需要 cryptography 与 pytest
```

`approval-core-check` 用软件测试钥，在临时目录里演练权威记录的发布与篡改；签批工具本身不链接任何软件钥。本机命令行工具链没有 XCTest，Swift 侧检查因此做成可执行程序。
