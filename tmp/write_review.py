from pathlib import Path
Path("tmp/codex-traverse-impl-review.md").write_text(open("tmp/review_content.txt", encoding="utf-8").read(), encoding="utf-8")
print("done")
