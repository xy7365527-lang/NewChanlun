#!/usr/bin/env python3
"""封存独立审查；只核 hash 和既有结果，零窗口计算。"""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
A = HERE.parent / 'joint-author'


def load(path: Path):
    return json.loads(path.read_text())


def sha(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(name: str, value: object) -> None:
    with (HERE / name).open('x') as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write('\n')


def main() -> None:
    original = load(HERE / 'run/inputhash-before.json')
    assert original == load(HERE / 'run/inputhash-after.json')
    for item in original:
        path = Path(item['path'])
        assert sha(path) == item['sha256'] and path.stat().st_size == item['bytes']
    save('inputhash-final.json', original)
    result = load(HERE / 'run/result.json')
    save('command-receipt.json', {
        'main_command': ['python3', str(HERE / 'independent_check.py'), '--out', str(HERE / 'run')],
        'main_actual_cwd': load(HERE / 'run/receipt.json')['cwd'],
        'main_exec_tool_chunk_id': 'c260e6', 'main_exit_code': 0,
        'main_H2_candidate_executions': 1, 'main_old_control_executions': 1,
        'preexecution_AST_compile': {'tool_chunk_id': '419e04', 'exit_code': 0, 'candidate_executions': 0},
        'supplementary_evidence_audit': {'command': ['python3', str(HERE / 'audit_evidence.py')],
                                         'tool_chunk_id': '2c4ce3', 'exit_code': 0, 'candidate_executions': 0},
        'new_histories': 0, 'R_W_runs': 0,
        'io_stalls_in_independent_execution': 0,
        'limitation': 'Author historic global run count remains a package process statement; no author session was read.'})
    save('provenance.json', {
        'role': 'fresh independent mathematical and implementation review of frozen Stage69 JointCut H2',
        'author_root': str(A), 'write_root': str(HERE),
        'frozen_author_final_sha256': sha(A / 'FINAL.json'),
        'frozen_author_manifest_sha256': sha(A / 'manifest.json'),
        'frozen_author_report_sha256': sha(A / 'Report.md'),
        'h2_identity': result['h2_hashes'], 'primary_sources': 'run/sources.json',
        'independent_code': 'independent_check.py', 'independent_results': 'run/',
        'read_memory': False, 'read_research_progress': False, 'read_author_conversation': False,
        'read_author_python_helper_review': False, 'author_self_check_counted_independent': False,
        'author_functions_used_as_oracle': False, 'author_candidate_functions_executed': 0,
        'ordinary_author_import_only': True,
        'new_history_or_R_W_runs': 0,
        'source_certificate_scope': 'Existing independently audited H2 raw identities, reinforced by source quantity and stable pen checks; parser was not re-proved.',
        'skills': [str(Path('/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.agents/skills/rigorous-open-math-research/SKILL.md')),
                   '/Users/silencehan/.agents/skills/unslop/SKILL.md']})
    entries = [{'path': str(path.relative_to(HERE)), 'sha256': sha(path), 'bytes': path.stat().st_size}
               for path in sorted(HERE.rglob('*')) if path.is_file() and path.name not in {'manifest.json', 'FINAL.json'}]
    total = sum(item['bytes'] for item in entries)
    assert total < 96 * 1024 * 1024
    save('manifest.json', {'created_at': datetime.now(timezone.utc).isoformat(), 'entries': entries,
                           'artifact_bytes_before_manifest': total, 'artifact_limit_bytes': 96 * 1024 * 1024})
    save('FINAL.json', {
        'status': 'independent_review_complete_finite_H2_root_obstruction_accepted',
        'review': 'review.md', 'report_sha256': sha(HERE / 'review.md'),
        'manifest_sha256': sha(HERE / 'manifest.json'),
        'frozen_author_final_sha256': sha(A / 'FINAL.json'),
        'frozen_author_manifest_sha256': sha(A / 'manifest.json'),
        'frozen_author_report_sha256': sha(A / 'Report.md'),
        'finite_evidence_approval': 'Accept', 'finite_python_script_approval': 'Approve',
        'required_author_repairs': [], 'accept_original_F1_F2_qualification': False,
        'layers': result['layers'], 'windows_all_fields_equal': 78,
        'prefix_paths_classes_consumers_counts_all_fields_equal': 583,
        'old_control': {'raws': 21, 'windows': 40, 'all_path_and_class_prefixes': 443},
        'root': 'S1 after fixed S0 prelude', 'root_k_coverage': [1, 2, 3, 4, 5, 6],
        'rooted_nonempty_paths': 0, 'rooted_three_factor_paths': 0,
        'internal_numerical_triples': 1, 'original_admitted_triples': 0,
        'internal_chain': [4, 9, 18, 27], 'internal_core': [50000, 80000],
        'internal_known_at': 558, 'root_gap_events': [22, 81], 'NoPP_length_types': 13,
        'new_histories': 0, 'R_W_runs': 0, 'independent_H2_candidate_runs': 1,
        'max_rss_bytes': result['max_rss_bytes'], 'artifact_bytes_before_manifest': total,
        'inputs_unchanged': True, 'old_sealed_evidence_preserved': True,
        'remaining_obligations': ['source compatibility', 'GeneralDiv and original Complete',
                                  'RootArm roles', 'Outer0 interpretation', 'Owner and Next',
                                  'coverage and uniqueness', 'NE over all legal extensions'],
        'A64_global_incompatibility_revived': False, 'original_H2_legality_revoked': False,
        'physical_future_read_isolation_claimed': False,
        'author_self_checks_counted_independent': False,
        'outside_review_files_modified': False,
    })
    print(json.dumps({'FINAL_sha256': sha(HERE / 'FINAL.json'), 'manifest_sha256': sha(HERE / 'manifest.json'),
                      'review_sha256': sha(HERE / 'review.md'), 'artifact_bytes': total}, indent=2))


if __name__ == '__main__':
    main()
