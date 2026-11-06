import json

import pytest
import test_external_review as _external_review

from pilot1742.external_review import unwrap_json_fence

lane = _external_review.lane

# 与 .sandcastle/codex-child.test.ts 的 FENCE_CASES 同一组用例。
CASES = [
    ('{"a":1}', '{"a":1}'),
    ('```json\n{"a":1}\n```', '{"a":1}'),
    ('  ```\n{"a":1}\n```\n', '{"a":1}'),
    ('```json\r\n{"a":1}\r\n```', '{"a":1}'),
    ('说明\n```json\n{"a":1}\n```', '说明\n```json\n{"a":1}\n```'),
    ('```json\n{"a":1}\n```\n补充', '```json\n{"a":1}\n```\n补充'),
    ('```json\n{"a":1}\n```\n```json\n{"b":2}\n```', '{"a":1}\n```\n```json\n{"b":2}'),
]


@pytest.mark.parametrize(("text", "expected"), CASES)
def test_only_a_whole_single_fence_is_unwrapped(text, expected):
    assert unwrap_json_fence(text) == expected


@pytest.mark.parametrize(("fence", "verdict"), [("whole", "PASS"), ("prose", "FAILED")])
def test_fenced_final_message_is_validated_like_plain_json(lane, fence, verdict):
    (lane.state_root / "runs").mkdir(parents=True, exist_ok=True)
    (lane.state_root / "runs" / "fake-fence").write_text(fence)
    result = lane.review_pr(1810)
    assert result["verdict"] == verdict
    if verdict == "FAILED":
        assert "不符" in result["reviews"][0]["reason"]
        json.dumps(result)  # 失败结论原样可序列化
