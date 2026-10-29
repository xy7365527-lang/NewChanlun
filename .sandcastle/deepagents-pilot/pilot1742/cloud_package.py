from __future__ import annotations

import hashlib
import io
import json
import re
import shutil
import subprocess
import tarfile
from pathlib import Path, PurePosixPath

EXCLUDED = {"node_modules", "__pycache__", ".pytest_cache", ".ruff_cache", ".mypy_cache", ".git", ".venv"}
PSTACK_TARGET = "/opt/newchanlun/pstack"
PROJECT_TARGET = "/workspace/repo"
RULES_TARGET = "/opt/newchanlun/rules"
MAX_FILE_BYTES = 32 * 1024 * 1024


def git(repo: Path, *args: str, input: bytes | None = None) -> bytes:
    return subprocess.run(["git", "--no-replace-objects", "-C", str(repo), *args], input=input,
                          check=True, capture_output=True).stdout


def safe_path(raw: str) -> PurePosixPath:
    path = PurePosixPath(raw)
    if not raw or not path.parts or raw.endswith("/") or path.is_absolute() or ".." in path.parts or "\\" in raw or "\0" in raw:
        raise ValueError("云产物路径不是安全相对路径")
    if any(part in {".git", ".agents", ".codex"} for part in path.parts):
        raise ValueError("云产物触及受保护路径")
    return path


def skill_files(directory: Path, root: Path, ancestors: frozenset[Path] = frozenset()):
    resolved_directory = directory.resolve(strict=True)
    if not resolved_directory.is_relative_to(root) or resolved_directory in ancestors:
        raise ValueError("技能目录链接越界或形成循环")
    for path in sorted(directory.iterdir()):
        if path.name in EXCLUDED:
            continue
        resolved = path.resolve(strict=True)
        if not resolved.is_relative_to(root):
            raise ValueError("技能引用越出已声明根：" + str(path))
        if resolved.is_dir():
            yield from skill_files(path, root, ancestors | {resolved_directory})
        else:
            yield path


def copy_skill_tree(source: Path, destination: Path, replacements: dict[str, str]) -> list[dict]:
    rows = []
    root = source.resolve()
    for path in skill_files(source, root):
        relative = path.relative_to(source)
        resolved = path.resolve(strict=True)
        if not resolved.is_relative_to(root) or not resolved.is_file():
            raise ValueError(f"技能引用越出已声明根：{relative}")
        data = resolved.read_bytes()
        if len(data) > MAX_FILE_BYTES:
            raise ValueError(f"技能文件过大：{relative}")
        if path.suffix in {".md", ".json", ".ts", ".mts", ".js", ".py", ".sh", ".yaml", ".yml", ".toml"}:
            text = data.decode("utf-8")
            for old, new in replacements.items():
                text = text.replace(old, new)
            data = text.encode()
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        target.chmod(resolved.stat().st_mode & 0o777)
        rows.append({"path": str(relative), "bytes": len(data), "mode": resolved.stat().st_mode & 0o777,
                     "sha256": hashlib.sha256(data).hexdigest()})
    return rows


def export_repository(repo: Path, head: str, destination: Path) -> None:
    if not re.fullmatch(r"[0-9a-f]{40}", head):
        raise ValueError("源码身份需要完整SHA")
    destination.mkdir(parents=True)
    archive = git(repo, "archive", "--format=tar", head)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar.getmembers():
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts or member.isdev() or member.islnk():
                raise ValueError("源码归档含不安全成员")
            if member.issym():
                target = (destination / member.name).parent / member.linkname
                if not target.resolve().is_relative_to(destination.resolve()):
                    raise ValueError("源码符号链接逃出运行包")
        tar.extractall(destination, filter="data")
    git(destination, "init", "-q", "--template=")
    parents = git(repo, "show", "-s", "--format=%P", head).decode().split()
    trees = git(repo, "rev-list", "--objects", "--no-object-names", *(item + "^{tree}" for item in [head, *parents])).decode().split()
    objects = "\n".join(dict.fromkeys([head, *parents, *trees])) + "\n"
    pack = git(repo, "pack-objects", "--stdout", input=objects.encode())
    git(destination, "index-pack", "--stdin", input=pack)
    if parents:
        (destination / ".git/shallow").write_text("\n".join(parents) + "\n")
    git(destination, "symbolic-ref", "HEAD", "refs/heads/codex/cloud-worker")
    git(destination, "update-ref", "HEAD", head)
    git(destination, "read-tree", head)
    if git(destination, "rev-parse", "HEAD").decode().strip() != head:
        raise ValueError("源码包HEAD不符")


def build_skill_bundle(pstack: Path, models: Path, destination: Path) -> dict:
    """Freeze pstack skills and model rules once per run; every worker package copies this bundle."""
    if destination.exists():
        raise FileExistsError(destination)
    destination.mkdir(parents=True)
    replacements = {str(pstack): PSTACK_TARGET, str(models): RULES_TARGET + "/pstack-models.md"}
    copy_skill_tree(pstack / "installed/skills", destination / "pstack/installed/skills", replacements)
    refs = {pstack / "aliases.json", pstack / "upstream/LICENSE"}
    for path in (pstack / "installed/skills").rglob("*.md"):
        if any(part in EXCLUDED for part in path.parts):
            continue
        for match in re.finditer(r"(/Users/[^`\s)]+)", path.read_text()):
            raw = match.group(1).rstrip(".,")
            if raw.startswith(str(pstack / "upstream") + "/"):
                refs.add(Path(raw))
    for path in refs:
        if not path.is_file() or not path.resolve().is_relative_to(pstack.resolve()):
            raise ValueError("pstack外部引用不可封包")
        target = destination / "pstack" / path.relative_to(pstack)
        target.parent.mkdir(parents=True, exist_ok=True)
        text = path.read_text()
        for old, new in replacements.items():
            text = text.replace(old, new)
        target.write_text(text)
    (destination / "rules").mkdir()
    (destination / "rules/pstack-models.md").write_bytes(models.read_bytes())
    references = []
    for document in (destination / "pstack").rglob("*"):
        if not document.is_file() or document.suffix not in {".md", ".json"}:
            continue
        text = document.read_text()
        if str(pstack) in text:
            raise ValueError("生成技能包残留已知Mac根")
        for match in re.finditer(r"/opt/newchanlun/pstack/[^`\s)\"<>]+", text):
            ref = match.group(0).rstrip(".,;:")
            if any(char in ref for char in "*$[]{}"):
                continue
            ref = ref.split("#", 1)[0]
            if not (destination / "pstack" / ref.removeprefix(PSTACK_TARGET + "/")).exists():
                raise ValueError("生成技能引用不可达：" + ref)
            references.append(ref)
    files = [{"path": str(path.relative_to(destination)), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
             for path in sorted(destination.rglob("*")) if path.is_file()]
    manifest = {"schema": 2, "pstack_root": PSTACK_TARGET + "/installed/skills/", "rules_root": RULES_TARGET + "/",
                "pstack_entries": len(list((destination / "pstack/installed/skills").glob("*/SKILL.md"))),
                "files": files, "local_references": sorted(set(references))}
    manifest["id"] = hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
    (destination / "bundle.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    return manifest


def build_package(repo: Path, head: str, destination: Path, bundle: Path, pstack: Path, models: Path) -> dict:
    if destination.exists():
        raise FileExistsError(destination)
    destination.mkdir(parents=True)
    export_repository(repo, head, destination / "repo")
    shutil.copytree(bundle / "pstack", destination / "pstack", symlinks=False)
    shutil.copytree(bundle / "rules", destination / "rules", symlinks=False)
    project = copy_project_skills(destination / "repo", destination / "project-skills", repo, pstack, models)
    bundle_manifest = json.loads((bundle / "bundle.json").read_text())
    manifest = {"schema": 2, "head": head, "skill_bundle": bundle_manifest["id"],
                "pstack_root": PSTACK_TARGET + "/installed/skills/", "project_skills_root": PROJECT_TARGET + "/.agents/skills/",
                "rules_root": RULES_TARGET + "/", "pstack_entries": bundle_manifest["pstack_entries"],
                "project_entries": project["entries"], "project_files": project["files"], "project_skill_id": project["id"],
                "capabilities": {"shell": "hardened Docker", "mcp": "未配置，使用相关技能前须核可用工具",
                                 "macOS": "仅本机认证，云工蜂不可调用"}}
    (destination / "package.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    return manifest


def copy_project_skills(snapshot: Path, destination: Path, repo: Path, pstack: Path, models: Path) -> dict:
    skills = snapshot / ".agents/skills"
    skills.mkdir(parents=True, exist_ok=True)
    replacements = {str(pstack): PSTACK_TARGET, str(models): RULES_TARGET + "/pstack-models.md", str(repo): PROJECT_TARGET}
    rows = copy_skill_tree(skills, destination, replacements)
    for document in destination.rglob("*"):
        if document.is_file() and document.suffix in {".md", ".json"} and any(
                root in document.read_text() for root in replacements):
            raise ValueError("项目技能包残留已知Mac根")
    return {"entries": len(list(skills.glob("*/SKILL.md"))), "files": rows,
            "id": hashlib.sha256(json.dumps(rows, sort_keys=True).encode()).hexdigest()}


def build_project_context(repo: Path, head: str, destination: Path, pstack: Path, models: Path) -> dict:
    destination.mkdir(parents=True)
    paths = ("AGENTS.md", "CONTEXT.md", "docs/agents", ".sandcastle/docs", ".agents/skills")
    present = [path for path in paths if git(repo, "ls-tree", head, "--", path)]
    snapshot = destination / "repo"
    snapshot.mkdir()
    if present:
        archive = git(repo, "archive", "--format=tar", head, "--", *present)
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            for member in tar.getmembers():
                path = PurePosixPath(member.name)
                if path.is_absolute() or ".." in path.parts or member.isdev() or member.islnk():
                    raise ValueError("项目快照含不安全成员")
                if member.issym() and not ((snapshot / member.name).parent / member.linkname).resolve().is_relative_to(snapshot.resolve()):
                    raise ValueError("项目快照符号链接越界")
            tar.extractall(snapshot, filter="data")
    manifest = {"head": head, **copy_project_skills(snapshot, destination / "project-skills", repo, pstack, models)}
    (destination / "context.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    return manifest
