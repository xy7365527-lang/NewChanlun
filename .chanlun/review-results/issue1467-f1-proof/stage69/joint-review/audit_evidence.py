#!/usr/bin/env python3
"""核封存清单和已算证书，不重新计算候选窗口。"""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = HERE.parents[1]
A = E / 'stage69/joint-author'
OLD = E / 'stage68/joint-author'


def load(path: Path):
    return json.loads(path.read_text())


def sha(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(name: str, data: object) -> None:
    with (HERE / name).open('x') as stream:
        json.dump(data, stream, ensure_ascii=False, indent=2)
        stream.write('\n')


def main() -> None:
    before = load(A / 'run/inputhash-before.json')
    after = load(A / 'run/inputhash-after.json')
    final = load(A / 'inputhash-final.json')
    assert before == after == final
    comparisons = []
    for item in final:
        path = Path(item['path'])
        got = sha(path)
        assert got == item['sha256'] and path.stat().st_size == item['bytes']
        comparisons.append({'path': str(path), 'sha256': got, 'match': True})
    old_review = E / 'stage68/joint-review/FINAL.json'
    old_review_final = load(old_review)
    assert sha(old_review) == load(A / 'stage68-seal-audit.json')['review_final_sha256']
    assert sha(OLD / 'FINAL.json') == old_review_final['frozen_author_final_sha256']
    assert sha(OLD / 'manifest.json') == old_review_final['frozen_author_manifest_sha256']
    assert sha(OLD / 'Report.md') == old_review_final['frozen_author_report_sha256']
    old_entries = []
    for entry in load(OLD / 'manifest.json')['entries']:
        path = OLD / entry['path']
        got = sha(path)
        assert got == entry['sha256'] and path.stat().st_size == entry['bytes']
        old_entries.append({'path': str(path), 'sha256': got, 'match': True})
    windows = load(HERE / 'run/independent-windows.json')
    coordinates = {(w['raw_start'], w['raw_stop']): w for w in windows}
    ids = {w['id']: w for w in windows}
    obstruction = load(A / 'finite-obstruction.json')
    assert obstruction['all_root_windows'] == [w for w in windows if w['raw_start'] == 1]
    budget = load(A / 'run/budget-obstruction.json')
    assert obstruction['no_pp_length_plans'] == budget
    for plan in budget['plans_with_at_least_three_and_structural_NoPP']:
        assert plan['raw_used'] == sum(plan['lengths'])
        assert plan['all_JEnd68'] == all(ids[name]['JEnd68'] for name in plan['windows'])
        for failure in plan['factor_failures']:
            w = ids[failure['id']]
            expected = {'id': w['id'], 'rejections': w['rejections'], 'kind': w['kind'],
                        'own': w['own'], 'whole': w['whole'], 'cores': [c['core'] for c in w['cores']],
                        'b_ref': w['b_ref'], 'c': w['c'], 'L_b': w['b_force']['L'], 'L_c': w['c_force']['L'],
                        'prior_extreme': w['prior_whole_extreme'], 'end_price': w['end_price'],
                        'happened': w['candidate_happened'], 'known': w['candidate_known']}
            assert failure == expected
    graph = load(A / 'run/graph.json')
    assert graph['edges'] == [w for w in windows if w['JEnd68']]
    assert graph['root_attempts'] == obstruction['all_root_windows']
    assert graph['root_raw_index'] == 1 and graph['raw_prelude'] == 'S0' and graph['root_end'] == 21
    internal = load(A / 'internal-chain.json')
    assert internal['raw_vertices'] == [4, 9, 18, 27]
    assert internal['all_three_evidence_known'] == 558
    assert internal['unowned_root_gap'] == {'raw_segments': ['S1', 'S2', 'S3'], 'events': [22, 81],
                                          'segment_count': 3, 'minimum_whole_segment_count': 5,
                                          'repair_inside_frozen_grammar': False}
    assert internal['root_changed'] is False and internal['rooted_F2_admission'] is False
    assert internal['original_F2_admission'] is False
    save('evidence-audit.json', {'created_at': datetime.now(timezone.utc).isoformat(),
                               'author_input_before_after_final_equal': True,
                               'author_recorded_inputs_currently_match': comparisons,
                               'stage68_author_seal_matches_prior_independent_final': True,
                               'stage68_manifest_count': len(old_entries), 'stage68_entries': old_entries,
                               'root_obstruction_six_full_field_equal': True,
                               'all_13_budget_plan_factor_failure_fields_equal': True,
                               'graph_all_edge_and_root_fields_equal': True,
                               'internal_gap_and_admission_fields_equal': True,
                               'candidate_window_evaluations': 0,
                               'author_python_review_conclusions_read': False})
    print('evidence audit passed; no candidate evaluation')


if __name__ == '__main__':
    main()
