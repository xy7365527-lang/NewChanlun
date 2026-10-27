"""只跑本轮选择的 Python 内存/fake；记录源 hash 和实际测试原输出。"""
import hashlib
from contextlib import nullcontext
import json
from pathlib import Path
import sys
import unittest
import tempfile
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import test_v5
from test_v2 import V2Tests


def identities():
    paths = [*sorted((ROOT/'driver').glob('*.py')), *sorted(HERE.glob('*.py')),
             ROOT/'fixes-v4/test_v4.py', ROOT/'fixes-v3/domain_fields.py',
             ROOT/'fixes-v3/independent_fixture.py']
    return {str(p.relative_to(ROOT)):dict(bytes=p.stat().st_size,
             sha256=hashlib.sha256(p.read_bytes()).hexdigest()) for p in paths}


def main():
    label = sys.argv[1]
    target = HERE/(label+'-observations.json')
    assert not target.exists(), '旧输出禁止覆盖'
    before = identities()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(test_v5.Tests)
    retained_v4 = ['test_f1_public_positive_and_geometry','test_f1_conflicting_internal_fields',
                   'test_f3_public_complete_set','test_f2_decode_plain_records_same_budget',
                   'test_f2_collector_budget_survives_second_snapshot','test_f8_frozen_named_messages',
                   'test_f8_cut_frontier_conflict','test_f8_identity_and_cross_epoch']
    retained_v2 = ['test_domain_adapter_positive_and_zn_mutation_negative',
                   'test_domain_withdrawal_rejects_stale_pair',
                   'test_driver_collects_fixed_pages_and_full_content','test_live_is_unconditionally_denied']
    suite.addTests(test_v5.RetainedV4(name) for name in retained_v4)
    suite.addTests(V2Tests(name) for name in retained_v2)
    with patch.object(Path, 'mkdir'), patch.object(Path, 'write_bytes', return_value=0), \
         patch.object(tempfile, 'TemporaryDirectory', side_effect=lambda:nullcontext('/memory')):
        result = unittest.TextTestRunner(verbosity=2).run(suite)
    assert identities() == before
    output = dict(schema='issue1419-v5-targeted-observations/1', tests=result.testsRun,
                  failures=len(result.failures), errors=len(result.errors), observations=test_v5.OBS+test_v5.v4.OBS,
                  retained_v4=retained_v4, retained_v2=retained_v2,
                  source_identity=before, authorization=False, product_run='NOT_RUN',
                  qualification='实施工蜂定向内存/fake，非独立评审')
    target.write_text(json.dumps(output, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps({k:output[k] for k in ('tests','failures','errors')}, ensure_ascii=False))
    return int(not result.wasSuccessful())


if __name__ == '__main__':
    sys.exit(main())
