# Secure Enclave 真人签批（#1742 D8）

执行器需要人批时写出一份待批请求。你在本机运行 `newchanlun-approve sign`，终端显示仓库、票号、范围、完整目标（commit SHA 或 SPEC 正文摘要）和批准或拒绝，随后系统弹出 Touch ID 或登录密码认证。认证通过才产生一份签名回执；取消或失败不产生任何文件。执行器只认 root 所有的信任配置里那把公钥签出的回执。

签名钥在 Secure Enclave 内生成，不可导出。钥的访问控制是 `privateKeyUsage + userPresence`，每次签名新建 `LAContext`，`touchIDAuthenticationAllowableReuseDuration = 0`，签完即 `invalidate`，所以每份回执都对应一次单独的本人认证。

## 一次性安装（由你本人操作）

1. 从已通过独审的那个提交构建，不从可被代理修改的工作树直接构建。把 `<COMMIT>` 换成独审报告里的完整 SHA。

   ```sh
   COMMIT=<COMMIT>
   SRC=$(mktemp -d)
   git -C /Users/silencehan/Projects/NewChanlun archive "$COMMIT" .sandcastle/approval-signing | tar -x -C "$SRC"
   cd "$SRC/.sandcastle/approval-signing/swift" && swift build -c release
   ```

2. 把签批工具装成 root 所有。之后一律用这个绝对路径运行它。

   ```sh
   sudo install -d -o root -g wheel -m 0755 "/Library/Application Support/NewChanlun"
   sudo install -o root -g wheel -m 0755 .build/release/newchanlun-approve \
     "/Library/Application Support/NewChanlun/newchanlun-approve"
   ```

3. 生成签名钥，并把信任配置直接写进 root 所有的位置。信任配置来自本进程刚生成的钥，中间不落用户可写文件。

   ```sh
   "/Library/Application Support/NewChanlun/newchanlun-approve" enroll --label "Mac Studio Secure Enclave" \
     | sudo tee "/Library/Application Support/NewChanlun/approval-trust.json" >/dev/null
   sudo chmod 0644 "/Library/Application Support/NewChanlun/approval-trust.json"
   ```

   终端会打印 `key_id`。核对 `sudo cat` 出来的配置里是同一个 `key_id`。钥句柄保存在 `~/Library/Application Support/NewChanlunApproval/signing-key.se-handle`，它只能在本机 Secure Enclave 里用，每次用都要你认证。

## 每次签批

```sh
APPROVE="/Library/Application Support/NewChanlun/newchanlun-approve"
"$APPROVE" sign --request <执行器给出的 request.txt> --decision approve --out <回执.json>
```

拒绝把 `approve` 换成 `reject`。对同一请求改主意时，用 `--previous <上一份回执>` 接在后面签，新决定覆盖旧决定。只在你自己刚发起签批时完成认证；任何你没有发起的 Touch ID 或密码弹窗都取消。

检查回执：

```sh
cd /Users/silencehan/Projects/NewChanlun/.sandcastle/approval-signing
python3 -m approval_signing verify <回执.json>
```

## 首次真机验收

主控给你一份验收用的 request（`run_id=enrollment-check`，随机 nonce，任何执行器都不会发出同一 nonce，所以它不放行任何真实动作）。依次做：

1. `sign --decision approve`，弹窗时取消。应无回执文件。
2. 再 `sign --decision approve`，完成认证。得到回执 1。
3. `sign --decision reject --previous 回执1`。这一次必须再次弹窗；若没有弹窗就签出来了，验收失败。
4. `verify` 两份回执都应为 VALID。

## 执行器接入

`approval_signing/protocol.py` 定义字节格式、回执和信任配置，`approval_signing/decision.py` 判定当前决定。

1. 每个需要人批的动作生成一份 `ApprovalRequest`，`request_nonce = secrets.token_hex(16)`，目标取动作此刻真正作用的对象：`CommitTarget(<40 位 SHA>)`，或 `spec_target(spec_issue, GitHub 返回的正文原始字节)`。目标或运行状态变了就发新请求、新 nonce。
2. 用 `load_trust()` 读信任配置。它拒绝任何非 root 所有、组或其他人可写、或经过符号链接的路径。
3. 每份回执经 `verify_receipt` 验签。
4. 动作前调用 `evaluate(request, current_target, receipts, now=..., watermark=..., receipts_complete=...)`。只有 `releases` 为真才放行。先持久化返回的 `watermark` 再行动。读不全回执来源时传 `receipts_complete=False`，此时只会拒绝或等待，不会放行。

判定规则：同一 nonce 的回执按 `sequence`/`previous_receipt` 组成一条链，最新一条生效，后来的拒绝覆盖先前的批准。分叉、断链、同 nonce 却字段不同、比已见水位短（远端删除）都返回 `needs_reconciliation`，交人对账。批准过期或目标已变返回 `awaiting_human`。

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

ASCII，LF 结尾，字段顺序固定，十进制无前导零，十六进制小写。`spec_authorization` 的目标写作 `spec:<SPEC 票号>:sha256:<正文摘要>`。请求文件用表头 `newchanlun-approval-request/v1`，去掉 `decision`、`sequence`、`previous_receipt` 三行。解析后重新编码必须与原字节完全一致，所以每个载荷只有一种编码。回执是 JSON：`format`、`algorithm`（`ecdsa-p256-sha256`）、`key_id`（公钥 SPKI DER 的 SHA-256）、`payload` 与 `signature`（DER）的标准 base64。`receipt_id` 是载荷字节的 SHA-256。

## 能防什么，不能防什么

代理和工蜂与你同一个 macOS 账户运行，可以读写你的文件、启动程序，但 `sudo` 要你的密码。

能防的是代理自己产出批准。它不能在没有你在场认证的情况下让 Secure Enclave 签名；它改不了 root 所有的信任配置和签批工具；它换掉钥句柄只会得到验不过的回执；它删掉 GitHub 上的拒绝回执，执行器的水位会发现链变短并停下。

不能防的有三类。第一，代理可以用你的钥句柄在任意时刻触发一次系统认证弹窗，弹窗文字由调用方决定；只要你在没有自己发起签批时通过了认证，就可能签下它准备的内容。所以只认自己发起的弹窗。第二，你运行的终端、Swift 工具链和 Python 环境与代理同一账户，被篡改后显示内容可以与签名内容不一致；固定提交构建和 root 安装缩小了这个面，没有消除它。第三，拥有 root 或你的密码的程序不在防护范围内。

## 开发检查

```sh
cd .sandcastle/approval-signing/swift && swift build -c release
./.build/release/approval-core-check ../tests/fixtures/swift   # Swift 侧检查并重写跨语言夹具
cd .. && python3 -m pytest -q                                    # 需要 cryptography 与 pytest
```

`approval-core-check` 用软件测试钥生成夹具，它是独立的开发程序，签批工具本身不链接任何软件钥。本机命令行工具链没有 XCTest，Swift 侧检查因此做成可执行程序。
