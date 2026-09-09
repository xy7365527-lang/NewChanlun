# #1370 / PR #1447 DevSkim 候选基线独立复核

结论：**PASS；0H / 0M / 0L**。本结论限误报登记候选；原 gate 仍以旧 8 项返回 **FAIL / exit 1**。

固定源提交 `17e9afb6fbadcc3057da8f6b3b9f9df57e5b7358`；W38 对照 `38a6b3f6f7d6118ffeb22c49c4a67a736be1ab92`。候选 SHA256：`df25ff2ec9a0898de03a4450fc3506444f08100ca0657a32665086991e202595`。

1. 候选保留原 5,587 键及顺序、历史 metadata；仅在末尾新增 2,614 个唯一精确键，总计 8,201。第 860 行逐字不变，原 `DS148264|.github/devskim-baseline.json|860` 自绑定键未移动。
2. 两份真实 SARIF 独立重算：17e9 为 8,231 结果 / 8,200 唯一键，W38 为 5,598 / 5,586；新增恰 2,614、删除 0。新增集合与候选后缀及逐键分类清单相等，原 8 项没有被吸收。全部新增键的固定 git blob SHA256、源行、重复数和 SARIF UTF-16 区间字节一致。
3. 误报分类全部核过匹配内容。2,608 项为验收 JSON：身份/收据来自业务字段，摘要/Git 标识记录来源与内容绑定，回环地址是验收命令、结果或浏览器记录。涉及 408 个不同冻结 Rust 文件的告警摘要已与 `76a641606d70` 对应字节重算；另核 profile 规范摘要和已签目录来源摘要。其他历史二进制/数据库摘要核其记录语义，不声称重新生成过历史工件。
4. 6 项运行源/目录误报：`launch_s.sh:234–237` 仅 echo 本地访问地址，PORT 与 `:222` 启动参数一致；启动未覆盖 host，服务 `s_readonly_server.py:351,355` 以默认回环地址绑定 HTTP，只读库连接见 `:32–34`。`signed-catalog.json:4` 的来源摘要与已签原始 JSON 重算相等，未充认证凭据。
5. DevSkim workflow、scanner 入口/镜像配置、gate、旧 baseline 和原 8 项两个源文件相对 W38 全部无 diff；候选没有新增 suppression 或放宽规则。一般 `.github/workflows/ci.yml` 存在本片既有修改，不能扩大声称所有 workflow 都未变化。
6. 候选新增 metadata 无完整 SHA、自摘要或新 TODO/HACK/FIXME/XXX 文本；旧 metadata 保持。配套说明的数量、#1316 冻结研究和 #1385 TB-10-B / OE09 归属均与证据一致。

实际命令：`python3 /tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/candidate-review/verify.py`，exit 0。该脚本直接运行固定提交逐字相同的原 gate，参数为真实 SARIF、候选基线、`scanner-outcome=success`、`artifact-outcome=success`、`devskim 1.0.90+fb2d676ce4`，**gate exit 1，恰剩原 8 项**。完整命令、逐键结果与退出码见同名 JSON；原始输出见 `/tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/candidate-review/gate.log`。DevSkim 配置及旧源对照 `git diff --exit-code W38 HEAD -- <具名路径>` 为 exit 0。

旧 8 项：`analysis/p3_random_gate_control.py` 的 DS148264 197/251/344 和 DS176209 141；`rust/src/theta_v0/nautilus/strategy.rs` 的 DS176209 19/104/113/314。没有修改或消隐这些项。

未执行新 DevSkim 扫描，不能据现有 SARIF 重算断言候选新提交扫描结果。未运行 cargo、Python suite、数据库或服务；未改仓、GitHub 或生产状态。main 合入、#1370 和父图完成不由本复核放行。
