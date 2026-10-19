"""汇总增量证据；历史报告不回写。报告属于 #1527 工作草稿。"""
import datetime
import stat
from common import *


def main():
    original = read_json(OLD / 'MANIFEST.json')
    integrity = read_json(OUT / 'SOURCE-INTEGRITY-AND-BUDGET.json')
    activity = read_json(OUT / 'ACTIVITY.json')
    members = read_json(OUT / 'MEMBERS.json')
    first = integrity['sources'][0]
    first_inventory = read_json(OLD / 'worktree/INVENTORY.json')['entries']
    private = read_json(OLD / 'worktree/GIT-PRIVATE-INVENTORY.json')
    private_bytes = sum(r['size'] for r in private if stat.S_ISREG(r['attrs']['mode']))
    regular = [r for r in first_inventory if stat.S_ISREG(r['attrs']['mode'])]
    unique_bytes = sum(r['size'] for r in regular if r['category'] in ('untracked', 'ignored', 'git-pointer'))
    block_rounding = sum((r['size'] + 4095) // 4096 * 4096 for r in regular)
    plan = {
        'state': '待根另准；本轮未执行', 'authorization_granted': False,
        'phase_a': {'source': first['source_path'], 'head': first['head_expected'],
                    'selection': 'SOURCE-INTEGRITY-AND-BUDGET.json sources[0].missing_hash_files',
                    'regular_files': 8251, 'source_read_bytes': 1358527370,
                    'output': str(OUT / 'FIRST-MISSING-HASH-RESULTS.jsonl'),
                    'argv': ['python3', str(OUT / 'hash_missing_proposal.py'), '--execute-approved-first-hash'],
                    'new_logical_allocation_budget_bytes': 16 * 1024**2,
                    'source_lfs_bytes_included': 304471992,
                    'shared_lfs_rehash_required': False},
        'phase_b': {
            'source': first['source_path'], 'fixed_head': first['head_expected'],
            'target': str(PARENT / 'reclaim-frozen/base-validation-20261018/worktree/files'),
            'metadata_stage': str(PARENT / 'reclaim-frozen/base-validation-20261018/worktree/metadata-stage'),
            'private_git_evidence': str(PARENT / 'reclaim-frozen/base-validation-20261018/worktree/git-private'),
            'old_output_restore': str(PARENT / 'reclaim-frozen/base-validation-20261018/original-seal-output'),
            'base_regular_logical_bytes': first['source_regular_logical_bytes'],
            'source_regular_allocated_bytes_observed': first['source_regular_allocated_bytes'],
            'target_regular_4096_rounding_bytes': block_rounding,
            'private_git_logical_bytes': private_bytes,
            'metadata_stage_file_body_bytes': 0,
            'planning_filesystem_metadata_allowance_bytes': len(first_inventory) * 65536,
            'new_allocated_space_budget_bytes': 2 * 1024**3,
            'budget_is_not_a_measured_apfs_hard_bound': True,
            'git_blob_decoded_bytes_upper_excluding_lfs_rule_unique': first['source_regular_logical_bytes'] - 304471992 - unique_bytes - 2888,
            'lfs_copy_bytes': 304471992,
            'unique_payload_bytes': unique_bytes,
            'rule_payload_bytes': 2888,
            'restored_regular_verification_read_bytes': first['source_regular_logical_bytes'],
            'source_worktree_content_read_bytes': 0,
            'git_pack_physical_read_bytes': '未测；delta解码/缓存会使其不等于返回正文量，不声称物理读上界',
            'git_dependency': original['shared_git_prerequisite'],
            'lfs_dependency': read_json(OUT / 'LFS-VERIFY.json')['objects'],
            'write_git_common_or_activate_old_pointers': False,
            'new_output_permitted_only_after_root_approval': True,
            'runner': '流程和接口已具体化；尚无完整base执行器，不能把小包恢复脚本当完整base执行器'},
        'all_other_candidates': '本次精确申请不包含其完整base；需要时同方法逐个另准，禁止五份并落或重扫6.64GiB',
        'deleteAuthorized': False}
    dump(OUT / 'NEXT-ACTION-PLAN.json', plan)
    released = ['五处唯一path/blob名称阻断：正文与固定blob、源属性、封包恢复均匹配',
                '两份共享LFS副本独立SHA256：累计304471992B，与五份原pointer记录匹配',
                'Unicode原始路径字节小样：两步改名后逐字节匹配，非规范化相等',
                '旧封包输出已退出并由经理仓外冻结：546清单项内容/链接核对通过；不是完整base恢复']
    remaining = ['PID97760仍打开worktree和r4-candidate原文件；全局no-reader证明缺失',
                 '五处真实完整base及全量Unix属性/原路径字节恢复未执行',
                 '首工位8251份普通文件的原source逐文件hash仍缺；共享LFS对象hash不补源文件hash',
                 '本轮最终输出的仓外复制尚未执行：仅可写指定补核目录；需经理在退出后收包',
                 '共享Git对象库及两份LFS仍是恢复前置；文件hash/存在性不构成独立bundle']
    checks = {'ticket': 1527, 'overall': 'BLOCKED', 'scoped_increment': '已完成获准小范围核验',
        'baseline': '44a977568948542aa6d00ecab50e2d468e20b76c',
        'released_scoped_holds': released, 'remaining_holds': remaining,
        'exact_member_restore_pass_count': 5, 'member_payload_bytes': 14440,
        'lfs_independent_hash_objects': 2, 'lfs_bytes_read': 304471992, 'lfs_read_upper_bound': 304471992,
        'unicode_exact_bytes_toy_pass': True, 'metadata_only_direct_extract_forbidden': True,
        'metadata_copy_without_data_toy_pass': True,
        'old_freeze_archive_hash_and_546_records_pass': True,
        'old_freeze_full_unix_restore_verified': False,
        'original_five_heads_indexes_and_unique_content_unchanged': all(r['head_match'] and r['index_match'] and not r['unique_content_mismatches'] for r in integrity['sources']),
        'original_tail_three_reports_unchanged': all(r['supplement_match'] for r in integrity['original_tail_reports'].values()),
        'old_metadata_exception': 'review-tail/.chanlun/review-results 的mtime是原封包创建时变化；本次未写旧目录，差值保留',
        'old_reports_rewritten': False, 'first_source_missing_hash_read_executed': False,
        'full_base_restore_executed': False, 'no_reader_proven': False,
        'open_dependencies': activity['observed'],
        'increment_verification': 'INCREMENT-VERIFY.json',
        'report_capsule_verification': '最终结果由包外CAPSULE-RESTORE-VERIFY.json给出；与包内报告分层避免循环hash',
        'supplement_outside_checkout_preserved': False,
        'current_python_checkout_is_deletion_candidate': False,
        'currently_deletable_candidates': [], 'deleteAuthorized': False, 'actualReleasedBytes': 0,
        'credentials_read': False, 'network_or_build_or_sqlite_or_subworker': False,
        'git_commit_push_merge_branch_config_write': False,
        'prior_attempts': ['原包首次核验把547个AppleDouble属性伴随项当未知，原失败记录保留；后核magic/对应成员并单列hash',
                          'Unicode小样首次os.setxattr在此Python不可用；改用系统xattr',
                          'Unicode第二次证明PAX头为NFC但解压实际为NFD；按已冻结精确映射修复后通过'],
        'not_run': ['全6.64GiB源二读', '完整base落地', '首工位缺hash补读', '仓外写入', 'Git全库fsck', 'Microsoft逐字网络比对（已裁不需要）']}
    dump(OUT / 'CHECKS.json', checks)
    candidates = []
    for source in original['sources']:
        candidates.append({'name': source['name'], 'path': source['path'], 'source': source['source'],
                           'preserved_ref': source['preserved_ref'], 'state': 'HOLD',
                           'currentlyDeletable': False, 'deleteAuthorized': False})
    dump(OUT / 'CANDIDATES.json', {'candidates': candidates, 'excluded_python_checkout': str(ROOT),
                                 'currently_deletable': [], 'deleteAuthorized': False})
    rows = '\n'.join(f'| {r["candidate"]} | {r["source_regular_logical_bytes"]:,} | {r["source_regular_allocated_bytes"]:,} | {r["missing_hash_regular_count_after_rule_supplement"]:,} |'
                     for r in integrity['sources'])
    active_rows = '\n'.join(f'- `{Path(r["run_directory"]).name}`，session `{r["external_session_id"]}`，状态文件 `{r["status"]}`。' for r in activity['running_from_state'])
    text = f'''# #1527 五结束工位封包缺口增量补核

五处继续 **HOLD**；可删候选 **0**，`deleteAuthorized=false`，实际释放 **0 B**。本轮解除精确 DevSkim 成员、共享 LFS 存储副本哈希和 Unicode 小样验证缺口；没有将旧报告改成通过。

产物名分为绑定 #1527 的工作草稿/证物。固定工作 HEAD 为 `44a977568948542aa6d00ecab50e2d468e20b76c`。当前 `python-r4-worktree` 是复用补核工位，**不在五个删除候选内**。本外部会话由运行时登记为 `{next(r['external_session_id'] for r in activity['runs'] if Path(r['run_directory']).name == 'reclaim-supplement-run')}`；没有声称注册成父线程原生子代理。

## 已完成且限定范围的证据

1. 精确成员 `{RULE}`、Git blob `{BLOB}`、2888 B、SHA256 `{RULE_SHA}`，五处源文件均匹配固定字节和原属性，各有新增 PAX 小包与新恢复 hash。总正文 14,440 B。`MEMBERS.json`、`member-restore/RESTORE-RECEIPT.json` 与独立 `INCREMENT-VERIFY.json` 可追溯。未打印正文，没有将豁免推广到其他同名文件，也没有因 Microsoft 比对未做而重挂此 HOLD。
2. 两份共享 LFS 对象分别为 125,404,332 B、179,067,660 B，合 **304,471,992 B**，各只做一次存储副本流式 SHA256；五份原 `CONTENT-RESOLUTION.json` 的 pointer blob/sha/size 一致。`LFS-VERIFY.json` 保留精确路径、pointer、前后 inode/size/mtime/ctime 和实读量。未复制 LFS，没有全库重 hash；首工位展开文件的源 hash 仍缺。
3. `UNICODE-TOY.json` 使用原 `Collège` 整条相对路径字节建合成小样。PAX头与 libarchive 解码/解压名称可有不同规范形态；解压后用清单给出的精确 NFD→NFC 映射经 ASCII 临时名做两步 rename，再用 `os.listdir(bytes)` 核每级原路径字节。正文与 mode/uid/gid/mtime/xattr/ACL/flags 均匹配。规范化相等不算成功，真实整棵 base 仍未重建。
4. `metadata-only` 直解覆盖合成正文，确实将文件截为 0 B。原禁令继续有效。另用 `copyfile(COPYFILE_METADATA | COPYFILE_NOFOLLOW)` 对普通文件、目录、symlink 做独立小样，正文 hash 不变、目标属性一致，见 `METADATA-REPLAY-TOY.json`。这是未来安全回放的方法验证，不是全 base 验收。
5. 经理 `reclaim-frozen/FREEZE.json` 指定的旧输出包 SHA256 为 `9a3d4f8a10f25f9c23c154974106cbba18c043dc2bf0171017ab6124abaff176`，37,124,031 B。独立核清单 **546** 项内容/链接，另列 **547** 个 AppleDouble 属性伴随项，所有旧源输出字节仍匹配冻结清单。未将仓外包 hash 当作完整 Unix metadata 恢复；恢复老输出时也须在新目录实核。

原五 HEAD/index/unique 及 tail 原三报告仍匹配。只发现已在原封包阶段产生的 tail `.chanlun/review-results` 目录 mtime 变化：原清单 1792299027898345162，当前 1792301748825448150；inode、mode、size不变，原 `PRE-OUTPUT-PARENT-METADATA.json` 保留旧值。本轮不重写它来制造“无变化”。

## 活读取依赖

定向 lsof 的 PID **97760 node** 仍打开：

- FD20：`worktree/.chanlun/review-results/issue1527-formal-recovery-20261018/IMPLEMENTATION.md`。
- FD31：`r4-candidate-worktree/.chanlun/review-results/issue1527-r4-anchor-20261018/MODEL-STDOUT.txt`。
- FD24：`r4-candidate-worktree/rust/src/bin/s_session_v3/admission_anchor.rs`（本次额外观察）。

这是“仍有打开的 FD”证据，不能据此宣称进程此刻执行 read。未中断或发信号给任何 PID。实际 argv/时间/退出码在 `ACTIVITY.json`。阴性 lsof 不覆盖未来按路径读取，**no-reader 仍为 false**。

本轮采样的在跑工蜂：

{active_rows}

活 prompt/新报告源码的定向字面引用中，本补核的 `prompt.md:16` 与 `common.py:14` 明确依赖旧 `review-tail-worktree/.../issue1527-reclaim-seal-20261018`。其他在跑工蜂在本扫描范围未见指向原五候选的具体 path；它们可经共享任务文档或未来工具调用间接读取，不能写成无依赖。父目录历史任务仍引用 `worktree/.../IMPLEMENTATION`、`review-worktree-01/.../REVIEW.md` 和 `r4-repair-worktree/.../MANIFEST.json`，清单区分活工蜂归属和仅历史文本命中。没有把历史任务当作正在执行。经理需取得活读者退出/依赖改指向独立封存物的证明，再做最终无读者判断。

## 可实际批准的最小后续范围（未执行、未批准）

精确合同见 `NEXT-ACTION-PLAN.json`；本段仅请求范围，不转授运行或删除。

| 候选 | 源普通文件逻辑 B | 本次 lstat 分配 B | 补核后仍缺源 hash 的普通文件 |
|---|---:|---:|---:|
{rows}

**A：只补首工位缺 hash。** 源 `{first['source_path']}` 固定 HEAD `{first['head_expected']}`；只读 `SOURCE-INTEGRITY-AND-BUDGET.json` 首项枚举的 **8251** 个普通文件，总 **1,358,527,370 B**，其中包括该源的两份 LFS 展开正文。已经有 hash 的 69 个原条目和本轮规则成员不重复读。每个文件 NOFOLLOW、原 inode/size/mtime 核对、读前后签名一致，再算 SHA256 和 Git blob SHA1；LFS例外仅比固定 pointer。逐行落盘，遇变停止，不丢已完成清单。输出 `{OUT / 'FIRST-MISSING-HASH-RESULTS.jsonl'}`，新输出预留16 MiB。脚本 `hash_missing_proposal.py` 已做语法检查，**未执行**；明确根批准后才使用 `--execute-approved-first-hash`。参数不是授权凭据。

**B：只重建首工位一个完整被动文件树。** 精确目标 `{plan['phase_b']['target']}`，必须不存在；metadata-stage、git-private、旧输出恢复位置均见 JSON。同目录一次只放一份完整 base；不 clone、不 checkout、不接入共享 worktree 注册、不改 Git 配置。其余四份完整重建不在这次最小申请里。

操作顺序：

1. 核新目标空、原固定 HEAD/ref/index、小包/清单/本轮精确成员 hash。共享 Git `{original['shared_git_prerequisite']}` 与 `LFS-VERIFY.json` 两条精确对象路径必须保留且只读；禁止任何懒抓取或网络。恢复原封包新输出到 JSON 指定的新目录并验546项、属性及原路径；不能再次依赖待删除的 tail 路径。
2. 按原 `source-tree0` 的原始路径字节经只读 `git cat-file --batch` 输出固定 blob，直接写新树，不运行 clean/smudge/filter/hooks。Git 返回 blob 重算对象 SHA1，恢复结果对原清单 SHA256（首工位需A补齐）。两份指针替换为已核共享 LFS 正文，读量 304,471,992 B，边写边核。精确规则从本轮小包恢复。可见 dirty/未跟踪/ignored 由原 unique包逐成员覆盖；本批 ignored=0，tracked删除=0，原patch为空，但按固定清单处理，不以空状态代替校验。
3. 原 `.git` 普通指针与原 Git-private/index 原字节一并保留作**未激活证物**，私有 index复制后核原 SHA256；对 source/index/工作树三者比较使用显式 `GIT_INDEX_FILE` 与只读共享对象库，不在新树直接运行会沿旧指针写共享库的 Git。若要求把新树激活成可用 worktree，必须另裁重绑定路径；不能同时声称原绝对指针字节未改且新位置已激活。
4. metadata-only包只解到独立空 `metadata-stage`。按冻结清单校验并恢复原始路径字节；普通文件/目录/symlink一一对应后，用已试验的 COPYFILE_METADATA 与 NOFOLLOW 回放属性，**不含 COPYFILE_DATA**。先正文后属性，目录 mtime 最后从叶到根回放。原 hardlink类=0；symlink按原目标字节，不能解引用。此批ACL全空/fflags全零，xattr值来自PAX，不能拿清单中的hash造值。若新遇未列明类型/ACL/flags，停止而不是降级。
5. 独立逐路径核正文SHA、完整原路径字节、mode/uid/gid/mtime、ACL/xattr/flags、symlink目标；原index hash/标志、dirty/ignored分类也对拍。inode/dev/ctime是源身份观测，不可在新文件系统复造；atime/birthtime虽原清单有观测，本轮小样与原验证器未证明其回放，若根要求这两项也逐位恢复须先补 setattrlist/utime小样并纳入完整验收，不能暗称已满足。文件hash、对象存在、干净status都不替代这一步。

读量与空间：首工位新树普通正文 **{first['source_regular_logical_bytes']:,} B**，原实际 `st_blocks*512` 合 **{first['source_regular_allocated_bytes']:,} B**；按4KiB逐文件取整 **{block_rounding:,} B**；另私有Git普通正文 **{private_bytes:,} B**。metadata-stage普通正文0 B，预留每清单项64KiB（共 {plan['phase_b']['planning_filesystem_metadata_allowance_bytes']:,} B）覆盖小文件/目录/xattr开销，规划新分配预算 **2 GiB**。这是保守作业预算，非已证明的 APFS 总元数据硬上界；执行器须在逐文件写前计量、遇空间不足即停，不承诺超界后轮询补救。旧源与原仓外包始终保留。

B无需读取旧工位正文：Git解码返回非LFS/unique/规则正文上界 {plan['phase_b']['git_blob_decoded_bytes_upper_excluding_lfs_rule_unique']:,} B，LFS复制304,471,992 B、unique约 {unique_bytes:,} B、规则2888 B；独立目标校验再读 {first['source_regular_logical_bytes']:,} B。原Git pack压缩/增量解码的实际设备读量未测，不能把返回正文量叫物理I/O上界。A+B总源补读仍只有A的精确1.358GB；没有授权重读五源6.64GiB。完整B执行器尚未交付/运行，本轮小恢复脚本不能冒充它。

## 本轮输出保全与恢复条件

`REPORT-CAPSULE.pax.tar.gz` 将保存本轮新报告、脚本、五小包、验证收据及小样；`REPORT-CAPSULE.json` 给出容器和索引hash，`CAPSULE-RESTORE-VERIFY.json` 给出本地新目录实恢复结果。后两份为包外运输收据，避免自引用；经理收包必须一起保留，且核 `MANIFEST.json` hash。报告包恢复仅验本轮输出，不代表五工位完整base已恢复。

本次允许的新写只有 `{OUT}`。因此**没有越界写仓外目录，也不声称已完成本轮仓外保全**。经理在本工蜂退出后，将包与两份运输收据复制到 `{PARENT / 'reclaim-frozen/supplement-20261018'}`（须新建不覆盖），复核包hash，再用 `capsule.py restore --archive <仓外包> --sha256 <固定hash> --index-sha256 <固定indexhash> --destination <仓外新目录>` 实恢复并验内容/Unix属性/原路径字节；恢复脚本所用属性读取器是旧脚本逐字副本，已随包保存，无需旧tail目录。`original_attribute_reader.py` 仅作导入属性读取器，不运行其原封包main。

旧 `reclaim-frozen` 包保全已实核；当前新输出的仓外持久化仍是实际缺项。即使全部新小包恢复通过，缺少 no-reader、首工位源hash或完整base验收时仍 HOLD；所有删除、源移动、push/merge/commit/main/关票均未获授权。
'''
    (OUT / 'SUPPLEMENT.md').write_text(text)
    # 清单绑定核心报告/脚本/证物；运输包、索引和最终恢复收据在外层绑定，避免循环hash。
    exclude = {'MANIFEST.json', 'REPORT-CAPSULE.pax.tar.gz', 'REPORT-CAPSULE.json',
               'REPORT-CAPSULE-MEMBERS0', 'REPORT-CAPSULE-INDEX.json', 'CAPSULE-RESTORE-VERIFY.json'}
    artifacts = []
    for path in sorted(OUT.rglob('*')):
        rel = path.relative_to(OUT).as_posix()
        if rel.split('/')[0] in exclude or rel.startswith('capsule-restore/'):
            continue
        st = path.lstat()
        if stat.S_ISREG(st.st_mode):
            artifacts.append({'path': rel, 'bytes': st.st_size, 'sha256': sha(path)})
    dump(OUT / 'MANIFEST.json', {'ticket': 1527, 'parent_task': '01a12e01-45a1-7173-a3e8-d338d108fb93',
        'manager': '/root/manage_1527_sandcastle', 'created_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'fixed_head': checks['baseline'], 'artifact_class': '绑定#1527的增量工作草稿/证物',
        'original_manifest': str(OLD / 'MANIFEST.json'), 'original_manifest_sha256': sha(OLD / 'MANIFEST.json'),
        'original_freeze_receipt': str(FROZEN / 'FREEZE.json'), 'original_freeze_receipt_sha256': sha(FROZEN / 'FREEZE.json'),
        'original_sources_unmodified': original['sources'], 'artifacts': artifacts,
        'transport_envelope_excludes_from_core_manifest': sorted(exclude),
        'report_capsule_restore_proof': 'CAPSULE-RESTORE-VERIFY.json',
        'supplement_outside_checkout_preserved': False, 'current_checkout_excluded_from_deletion': str(ROOT),
        'complete_base_restore_verified': False, 'deleteAuthorized': False})
    print('SUPPLEMENT/CHECKS/MANIFEST与最小后续合同已生成。')


if __name__ == '__main__':
    main()
