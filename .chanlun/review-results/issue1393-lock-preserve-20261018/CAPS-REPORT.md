# #1393 保旧解析入口：根具名更窄帽实施报告（作者自述，未独审）

接续负责人 /root/owner_1393。工位 `/Volumes/AgentStorage/issue1393-narrowcaps-20261019`，分支 `codex/1393-narrowcaps-20261019`，起点 run16 `d06b12743db549389cbd2f8aa8cf4d86ac84d3b8` / tree `f2cb21f005016418d1d10934073df5682515018c`。按任务书 `TASK-RUN17-NARROW-CAPS.md`（SHA256 `d7161b14…`）从 run16 起点重新实施；run17 partial（`RUN17-PARTIAL-FREEZE.json`，INCOMPLETE_UNREVIEWED_NOT_AUTHORIZED）及其后基于该 partial 的 resume 封包均未读取、未复制。本报告是作者检查，不算独审；approval/review/prepared_* 仍 UNBOUND。

## 帽的来源与计数对象

`RESOURCE_CAPS` 现绑定根 2026-10-18 本次新定的更窄上限（`ROOT-REGISTRY-CAPS-20261018.json`，SHA256 `d4c6e154…`），代码与合同均明称 `ROOT_NEW_NARROWER_CAPS_20261018`，不称既有帽：

| 帽 | 值 | 计数对象 |
|---|---|---|
| single_bytes | 65536 | 每一条所需版本 JSON 原文字节（不是整个 sparse cache 文件） |
| total_bytes | 2097152 | registry 证据正文累计：`registry.json` 与 `registry-*`（config 原件、逐 crate 摘录、失败记录）；工具链日志、旧源、lock 等不计入 |
| files | 128 | 上述 registry 证据文件数 |
| items | 512 | 保留的 registry 版本记录条数（扫描中每保留一条即计，超即拒） |
| upload_bytes | 33554432 | 主包 + 收据包两个上传容器外层字节合计 |

保持原义：MAX_MESSAGE_BYTES 32768、MAX_EVENT_BYTES 1048576、900 秒共享截止、90 秒终段预留、10 秒监督、jobs 1、2 GiB 仅作启动磁盘门。合同 `resource_caps` 段完整列出上述数值、`mode=EXISTING_EXCLUSIVE_CARGO_CACHE_ONLY`、`additional_network_for_evidence=false`、固定 action 与 compression 0；校验对每个固定字段做逐层类型相等（拒 0/False、1.0/1、True/1），schema 升为 `issue1393-lock-preserve/2`，批准/独审收据 schema 升为 `/2`，旧 schema、旧 operation/ref、错数值、未知或缺失字段均拒绝。

## registry 取证：仅独占 cache，记录原文完整、整 cache 不归档

`scan_sparse_cache` 以 64 KiB 块流式读 Cargo sparse cache v3（magic `03 02 00 00 00`），每块前后核同一截止和信号；只缓冲当前所需版本段（≤ single_bytes）与短 version（≤256B）/validator（≤4096B）段，非所需版本只计长度不入内存。所需版本原文超过 single_bytes 记 proof_gap（不截断、不冒称完整）；缺 cache、缺版本、截断（末段未以 NUL 收尾）、未知 magic、缺 validator、重复版本（含非所需版本）、名/版本不符、扫描期间文件变化都进入 proof_gap 或拒绝，最终 `complete=false`。没有任何 HTTP/下载/补 cache 路径；runner 不导入 socket/urllib/http/ssl/subprocess。Cargo update 本身的官方 registry 访问保持原合同。

每个 crate 写一份 `registry-cache-<i>.raw` 摘录：原 magic + 原 validator + 所选 `version\0json\0` 原段（按原 cache 顺序）。`registry.json` 每条记录给出原 cache 相对路径、原 cache 偏移、原文字节数与 SHA256、摘录文件及偏移；每个 crate 给出原 cache 全文件字节数与 SHA256（流式算出）和 validator。`scope` 字段明写 `WHOLE_CACHE_FILE_NOT_ARCHIVED`，区分「所选记录原文完整」与「整个 cache 未归档」。约束父（含根 manifest 与 registry 父）照旧逐边记录。

## 上传容器的事前上界

依据管理者取得的官方固定 commit 原件（`upload-artifact-reference/`，dist SHA256 `0165b8a7…`；compress-commons 6.0.2 `zip-archive-output-stream.js` SHA256 `262cce58…`），本人核实：`createZipUploadStream` 用 `archiver.create('zip', {zlib:{level}})`，level 0 时 zip-stream 置 store；`uploadZipToBlobStorage` 直接上传该 zip 流，没有再包一层。compress-commons 每项写 LFH 30B + 名 + extra、流式条目数据描述 16B（ZIP64 形式 24B）、CFH 46B + 名 + extra + comment；extra/comment 默认空，只有 ZIP64 时 central extra 加 28B；结尾 EOCD 22B，ZIP64 时再加 56 + 20B。因此每容器上界取 `98 + Σ(size + 128 + 2·UTF8('/'+name))`，128 = 30+46+24+28，名字按 destinationPath 含前导 `/` 计（比实际多 1B）。实现拒绝会进入 ZIP64 的情形（项数 ≥65535、容器 ≥4 GiB），拒绝 upload-artifact `validateFilePath` 禁字符、非严格 UTF-8 名、子目录、symlink、非普通文件。

流程：runner 每次写入前核「当前全部散件的主包上界 + 收据容器预留 ≤ 33554432」；`package()` 在写 FILES/ZIP 之前按「散件 + FILES + evidence.zip 上界 + PACKAGE ≤4096」预核，超限不写尾件、不删原件。workflow 的 archive 步先补缺失占位原件，此后不再写入上传目录，调用 `upload-gate`：逐件取字节与 SHA256 冻结清单（写在目录外），核主包上界 + 收据容器上界（REQUEST ≤4096、FROZEN ≤65536、ACTUAL ≤4096，属合计帽内部切分）≤ 33554432 且剩余窗口足够，才置 allowed。收据步写 ACTUAL（>4096 拒写），`receipt-gate` 复核上传目录与冻结清单逐件一致、按收据三件实际字节重算合计上界后才允许第二次上传。两个上传均固定 `actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02` 与 `compression-level: 0`，不再使用可变 `@v4`。

## 已知限度

上界只覆盖 action 生成的 zip 字节流；GitHub 服务端之后的存储形态不在其内，此处以 bundle 证据（直传 blob、finalize 报告的 size 即上传字节）为依据。上传期间目录被改动只能在 receipt-gate 事后发现并拒绝收据上传，不能事前阻止；runner 结束后已无写入者。冻结清单 65536B 上限下约可容纳数百个散件，超出即拒绝上传（BLOCKED 而非扩帽）。仍 NOT_RUN：Cargo/rustc/rustup、真实 Actions、Lean、fixture、产品与浏览器；未 push、未 dispatch、未启 True、未合 main、未关票。
