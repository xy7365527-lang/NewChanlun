#!/usr/bin/env python3
"""恢复三处旧擦源表达式，供 CI 证明真实预算失败的旧红行为。"""
from pathlib import Path

base = Path("rust/src/bin/s_session_v3")
path = base / "generation.rs"
text = path.read_text()
old = '.map_err(|error| bi_error(error).context("GenError：上一代未定原件不可核："))'
new = '.map_err(|error| format!("GenError：上一代未定原件不可核：{error}").into())'
assert text.count(old) == 1
path.write_text(text.replace(old, new))
path = base / "object_index.rs"
text = path.read_text()
old = """    store.native_codec_room(targets, 1).map_err(publish_err)?;
    targets.try_push(target).map_err(publish_err)"""
new = """    store.native_codec_room(targets, 1).map_err(err)?;
    targets.try_push(target).map_err(err)?;
    Ok(())"""
assert text.count(old) == 1
path.write_text(text.replace(old, new))
