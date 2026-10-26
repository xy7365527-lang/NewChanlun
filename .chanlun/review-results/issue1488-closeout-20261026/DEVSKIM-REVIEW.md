# #1725 的 36 项 Git 对象误报登记

本候选沿用 #1488 已有的完整 SARIF result 身份与文件字节绑定机制，仅新增两文件、36 项精确身份，待非作者复核。不改 baseline、扫描范围、规则或 gate 实现。

## 分类依据

[DevSkim run 40245119196](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40245119196) 的 job `121324945011` 扫描成功，gate 报 36 项新增 DS173237。规则原文为 “Do not store tokens or keys in source code”，其说明限定 “If this represents a secret”。原始 SARIF 从该 run 的 `devskim-sarif` artifact 读取。

逐项核 SARIF 的文件、行、片段和 JSON 字段，21 项均为干净工位 `head`，15 项均为退役工位 `stash_oid`。全部是 40 位 Git 对象标识，`git cat-file -t` 对 36 个值均返回 `commit`。HEAD 从独立 `restore.git` 核验；stash 从各自 common dir 核验，ID41 使用其已登记的独立 clone common dir。它们是恢复证据，不是认证令牌或密钥。

- `1488-CLEAN-LINKED-RELEASE26.json` 的命中行为 8、16、24、32、40、48、56、64、72、80、88、96、104、112、120、128、136、144、152、160、168。
- `1488-RETIRED15-RELEASE26.json` 的命中行为 10、23、92、129、142、155、168、181、194、207、220、233、246、259、272。

误报登记在 `.github/devskim-reviewed-1488.json`，每项仍绑定完整 result 的规则、全部位置、片段、消息与属性，文件仍绑定 SHA256 字节。新增两项文件登记之外，既有 74 项文件登记结构和值均不变，baseline 字节不变。

## 修订后的原件重新扫描

同一提交删去干净批次 `projection_note` 中不准确的“和环境扫描”片语。该变化会移动后续字符偏移，所以没有照抄旧 SARIF 的身份。使用现有 `devskim-scanner:1.0.90` 镜像重新扫描修订后的两文件，SARIF driver 为 `1.0.90+fb2d676ce4`，仍有 36 项。未变动的退役文件，其 15 项完整身份与 GitHub 原 SARIF 逐项一致。

本地 Docker 无法看到宿主绑定路径，首次绑定扫描未产生结果，不计验证。随后使用无网络临时容器，以 `docker cp` 放入两份原件，扫描后取回 SARIF 并回收该容器。没有改 Docker 权限或全局配置。

## 验证结果与边界

- 本地两文件真实 SARIF 经既有 gate 回灌，精确匹配并排除 36 项，PASS。传给 gate 的 artifact 成功值用于本地回灌，不代表发生 GitHub 上传；本地两文件扫描也不代表全仓扫描完成。
- 将一条真实 result 的起始列加 1，既有分类器保留该条为未审结果。没有按文件、行号或目录通配放行。
- `python3 -m unittest discover -s tests -p 'test_devskim*py' -v` 为 22/22 PASS，覆盖原件漂移、缺失、同旧键新片段、规则变化及 scanner/upload/tool 失败。
- `git diff --check` 通过。没有改原 OID、删除恢复证据或重新生成历史 baseline。

新 head 仍须经 GitHub 全仓 DevSkim 和非作者复核。本报告不宣布全 CI 通过，也不改变 #1488 剩余容器处置条件。
