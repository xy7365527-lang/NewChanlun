"""#1742 U6：用已审 pilot1742 App 客户端的只读 installation token 读 DeepAgents 交付分支的实时 ref 与 PR。

只发 GET，不写 GitHub，不输出 token。PYTHONPATH 须指向含 pilot1742 的目录，私钥只按绝对路径传给客户端。
"""
import argparse
import json
import re
import sys
from pathlib import Path

from pilot1742.github_app import READ_PERMISSIONS, AppIdentity, GitHubAppClient, IdentityError

SHA = re.compile(r"[0-9a-f]{40}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="只读读取 codex/deepagents/tN 的远端 ref 与 PR")
    parser.add_argument("--private-key", required=True, type=Path)
    parser.add_argument("--branch", required=True)
    args = parser.parse_args(argv)
    if not re.fullmatch(r"codex/deepagents/t[1-9][0-9]*", args.branch):
        print("分支不是 DeepAgents 交付分支", file=sys.stderr)
        return 2
    try:
        identity = AppIdentity(5486877, 175617978, 1150335160, "xy7365527-lang/NewChanlun",
                               "newchanlun-deepagents-1742", args.private_key)
        repo = identity.repository
        with GitHubAppClient(identity, permissions=READ_PERMISSIONS) as client:
            found = client.api("GET", f"/repos/{repo}/git/ref/heads/{args.branch}", tolerate=frozenset({404}))
            ref = None
            if found.status != 404:
                obj = (found.data or {}).get("object") or {}
                if found.data.get("ref") != f"refs/heads/{args.branch}" or obj.get("type") != "commit" \
                        or not SHA.fullmatch(str(obj.get("sha"))):
                    raise IdentityError("远端分支读回不符")
                ref = obj["sha"]
            pulls = client.paginate(f"/repos/{repo}/pulls", {"head": f"{repo.split('/')[0]}:{args.branch}", "state": "all"})
    except IdentityError as error:
        print(str(error), file=sys.stderr)
        return 2
    print(json.dumps({"branch": args.branch, "ref": ref, "pulls": [
        {"number": p.get("number"), "state": p.get("state"), "merged": bool(p.get("merged_at")),
         "head": (p.get("head") or {}).get("sha"), "head_ref": (p.get("head") or {}).get("ref")} for p in pulls]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
