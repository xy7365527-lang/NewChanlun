"""定向只读取证；无 ps 扫环境、无信号发送、无全库正文重读。"""
import datetime
import re
import stat
from common import *


def parse_lsof(raw, allowed):
    pid = command = fd = None
    rows = []
    for line in raw.decode(errors='replace').splitlines():
        if line.startswith('p'): pid = line[1:]
        elif line.startswith('c'): command = line[1:]
        elif line.startswith('f'): fd = line[1:]
        elif line.startswith('n') and line[1:] in allowed:
            rows.append({'pid': pid, 'command': command, 'fd': fd, 'path': line[1:]})
    return rows


def activity():
    inventories = {n: read_json(OLD / n / 'INVENTORY.json') for n in NAMES}
    files = [str(PARENT / n / r['path']) for n, inv in inventories.items() for r in inv['entries']
             if r['category'] in ('untracked', 'ignored') and stat.S_ISREG(r['attrs']['mode'])]
    tail = read_json(OLD / 'SOURCE-FINAL-INTEGRITY.json')['original_tail_reports']
    files = sorted(set(files) | {str(PARENT / 'review-tail-worktree' / rel) for rel in tail})
    argv = ['/usr/sbin/lsof', '-nP', '-Fpcfn', '--', *files]
    p = run(argv, check=False)
    old_targets = [row['path'] for source in read_json(OLD / 'ACTIVITY-FINAL.json')['sources']
                   for row in source['observed']]
    pidargv = ['/usr/sbin/lsof', '-nP', '-a', '-p', '97760', '-Fpcfn', '--', *old_targets]
    pp = run(pidargv, check=False)
    # 只枚举经理父目录的一层 run/state.json；不读会话认证、日志正文或环境变量。
    runs = []
    for child in sorted(PARENT.iterdir()):
        if not child.is_dir() or 'run' not in child.name or not (child / 'state.json').is_file():
            continue
        state = read_json(child / 'state.json')
        if state.get('ticket') != 1527:
            continue
        runs.append({k: state.get(k) for k in ('run_id', 'status', 'process_pid', 'ended_at',
                     'external_session_id', 'expected_head')} | {'run_directory': str(child),
                     'checkout': (state.get('checkout_before') or {}).get('root'),
                     'state_sha256': sha(child / 'state.json')})
    active = [r for r in runs if r['status'] == 'running']
    # 只扫描具名任务文本与活工位任务所指的新报告/必要源码，检索旧候选路径字面量。
    text_files = set(PARENT.glob('TASK-*.md')) | set(PARENT.glob('REVIEW-*.md'))
    text_files |= {PARENT / 'ROOT-I1-IMPLEMENTATION-SCOPE.md', PARENT / 'ROOT-R4-ENGINEERING-SCOPE.md',
                   PARENT / 'I1-RESPONSIBILITY-AND-REUSE.md'}
    for row in active:
        prompt = Path(row['run_directory']) / 'prompt.md'
        if prompt.exists(): text_files.add(prompt)
        if row['checkout']:
            checkout = Path(row['checkout'])
            # 新成果报告/源码；不递归读取历史封包和 toy。
            report_names = set(re.findall(r'\.chanlun/review-results/(issue1527-[a-zA-Z0-9-]+)/',
                                         prompt.read_text())) if prompt.exists() else set()
            for dirname in report_names:
                folder = checkout / '.chanlun/review-results' / dirname
                if folder.is_dir():
                    text_files |= {p for p in folder.iterdir() if p.is_file() and p.suffix in ('.md', '.py', '.rs')}
            for relative in ('rust/src/bin/s_session_v3/admission.rs',
                             'rust/src/bin/s_session_v3/admission_provider.rs'):
                path = checkout / relative
                if path.is_file(): text_files.add(path)
    hits, scanned, total = [], [], 0
    # 路径名边界避免把 formal-i1-worktree 误认成 worktree。
    pattern = re.compile(r'(?<![A-Za-z0-9_-])(?:' + '|'.join(re.escape(n) for n in NAMES) + r')/[^\s`\"\'，。；（）<>]+')
    for path in sorted(text_files):
        raw = path.read_bytes()
        total += len(raw)
        scanned.append({'path': str(path), 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()})
        for lineno, line in enumerate(raw.decode(errors='replace').splitlines(), 1):
            for match in pattern.finditer(line):
                hits.append({'file': str(path), 'line': lineno, 'candidate_path_reference': match.group(0)})
    for hit in hits:
        hit['active_owners'] = [Path(r['run_directory']).name for r in active
                               if hit['file'] == str(Path(r['run_directory']) / 'prompt.md')
                               or (r['checkout'] and hit['file'].startswith(r['checkout'] + '/'))]
    dump(OUT / 'ACTIVITY.json', {
        'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'target_files': files, 'lsof_argv': argv, 'lsof_rc': p.returncode,
        'observed': parse_lsof(p.stdout, set(files)),
        'lsof_stderr_bytes': len(p.stderr), 'lsof_stderr_sha256': hashlib.sha256(p.stderr).hexdigest(),
        'pid97760_argv': pidargv, 'pid97760_rc': pp.returncode,
        'pid97760_observed': parse_lsof(pp.stdout, set(old_targets)),
        'pid97760_stderr_bytes': len(pp.stderr),
        'runs': runs, 'running_from_state': active, 'scanned_text_files': scanned,
        'text_bytes_read': total, 'old_candidate_path_references': hits,
        'scope': '父目录当前run状态、具名prompt/任务/新报告及两份admission源码；不覆盖所有未来工具调用、间接引用或其他会话',
        'no_reader_proven': False, 'negative_lsof_is_not_no_reader_proof': True,
        'pids_interrupted': False, 'deleteAuthorized': False})
    print({'active_runs': len(active), 'path_hits': len(hits), 'lsof_hits': len(parse_lsof(p.stdout, set(files)))})


def source_integrity_and_budget():
    sources = read_json(OLD / 'MANIFEST.json')['sources']
    results = []
    for s in sources:
        name = s['name']
        inv = read_json(OLD / name / 'INVENTORY.json')
        regular = [r for r in inv['entries'] if stat.S_ISREG(r['attrs']['mode'])]
        missing = [r for r in regular if r['category'] == 'tracked' and 'sha256' not in r and r['path'] != RULE]
        unique = [r for r in regular if r['category'] in ('untracked', 'ignored', 'git-pointer')]
        source = PARENT / name
        old_seal = read_json(OLD / name / 'SEAL.json')
        head = git('rev-parse', 'HEAD', root=source).decode().strip()
        index = Path(s['git_private_dir']) / 'index'
        # 只 stat 已冻结清单（不读 base 正文）。原输出父目录 mtime 变化在前轮已登记。
        changes, allocated = [], 0
        for r in inv['entries']:
            path = source / r['path']
            st = path.lstat()
            if stat.S_ISREG(st.st_mode): allocated += st.st_blocks * 512
            if (st.st_mode, st.st_size, st.st_mtime_ns, st.st_ino) != (r['attrs']['mode'], r['size'], r['attrs']['mtime_ns'], r['ino']):
                changes.append({'path': r['path'], 'kind': 'lstat', 'actual': [st.st_mode, st.st_size, st.st_mtime_ns, st.st_ino],
                                'expected': [r['attrs']['mode'], r['size'], r['attrs']['mtime_ns'], r['ino']]})
        unique_changes = [r['path'] for r in unique if sha(source / r['path']) != r['sha256']]
        original_index_hash = old_seal['index_sha256']
        rows = {'candidate': name, 'source_path': str(source), 'head_expected': s['source'],
                'head_actual': head, 'head_match': head == s['source'],
                'index_path': str(index), 'index_sha256': sha(index), 'index_expected_sha256': original_index_hash,
                'index_match': sha(index) == original_index_hash,
                'unique_files_checked': len(unique), 'unique_content_mismatches': unique_changes,
                'metadata_deltas': changes, 'source_regular_logical_bytes': sum(r['size'] for r in regular),
                'source_regular_allocated_bytes': allocated, 'regular_entries': len(regular),
                'all_entries': len(inv['entries']),
                'missing_hash_regular_count_after_rule_supplement': len(missing),
                'missing_hash_bytes_after_rule_supplement': sum(r['size'] for r in missing),
                'known_hashed_bytes_not_requiring_source_reread': sum(r['size'] for r in regular if 'sha256' in r),
                'missing_hash_files': [{'path': r['path'], 'bytes': r['size'], 'git_expected': r.get('git_expected')} for r in missing]}
        results.append(rows)
    tail = read_json(OLD / 'SOURCE-FINAL-INTEGRITY.json')['original_tail_reports']
    for rel, rec in tail.items():
        rec['supplement_actual'] = sha(PARENT / 'review-tail-worktree' / rel)
        rec['supplement_match'] = rec['supplement_actual'] == rec['expected']
    dump(OUT / 'SOURCE-INTEGRITY-AND-BUDGET.json', {'sources': results, 'original_tail_reports': tail,
        'scope': '原HEAD、原index、独有小成果复核，base仅lstat；不含新封包目录',
        'current_checkout_head': git('rev-parse', 'HEAD').decode().strip(), 'deleteAuthorized': False})
    print([{k: r[k] for k in ('candidate', 'head_match', 'index_match', 'source_regular_logical_bytes',
                            'source_regular_allocated_bytes', 'missing_hash_regular_count_after_rule_supplement',
                            'missing_hash_bytes_after_rule_supplement')} for r in results])


if __name__ == '__main__':
    activity()
    source_integrity_and_budget()
