//! #1568：`s-session/3` init 的「暂存建库 → 原子发布」。
//!
//! 目标路径只会出现两种状态：**不存在**，或**已完整提交、已回读核对的库**。建库全程发生在同目录
//! 暂存文件 `.<库名>.s3-init.<pid>.<nanos>.tmp`（及其 SQLite 旁文件 `-wal/-shm/-journal`）上；
//! 核对完毕、连接关闭（WAL 检查点并删除）后 fsync，再用**不覆盖**的原子 rename 发布到目标：
//! Linux `renameat2(RENAME_NOREPLACE)`，macOS `renamex_np(RENAME_EXCL)`。目标此时若已被他人
//! 创建，rename 以 EEXIST 失败，既有文件一个字节都不动。
//!
//! 失败分三类：
//! - 进程内可观察的失败（磁盘满 EFBIG/ENOSPC、fd 耗尽等）：守卫删除本次暂存文件，目标不存在；
//! - 进程被杀（SIGKILL/SIGXFSZ 等）：只会留下可识别的暂存文件，目标仍不存在；
//! - 下一次 init 在持有同一把域进程锁之后，**只**清扫严格匹配本库名暂存格式的残留——锁保证
//!   同库没有活着的 init 在用它们；库名不同、格式不符的文件一律不碰，更不碰目标本身。

use std::ffi::{OsStr, OsString};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const STAGING_TAG: &str = ".s3-init.";
const STAGING_EXT: &str = ".tmp";
/// SQLite 可能为暂存库生成的旁文件后缀（附加在暂存文件名之后）。
const SIDE_SUFFIXES: [&str; 3] = ["-wal", "-shm", "-journal"];

fn parent_dir(db: &Path) -> &Path {
    db.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn staging_prefix(db: &Path) -> Result<OsString, String> {
    let name = db
        .file_name()
        .ok_or("InvalidDomain：数据库路径缺文件名")?;
    let mut prefix = OsString::from(".");
    prefix.push(name);
    prefix.push(STAGING_TAG);
    Ok(prefix)
}

/// 判定 `entry` 是否是 `db` 的暂存文件或其 SQLite 旁文件：`<prefix><digits>.<digits>.tmp[-wal|-shm|-journal]`。
pub(super) fn is_staging_entry(db: &Path, entry: &OsStr) -> bool {
    let Ok(prefix) = staging_prefix(db) else {
        return false;
    };
    let entry = entry.as_bytes();
    let Some(rest) = entry.strip_prefix(prefix.as_bytes()) else {
        return false;
    };
    let rest = SIDE_SUFFIXES
        .iter()
        .find_map(|s| rest.strip_suffix(s.as_bytes()))
        .unwrap_or(rest);
    let Some(core) = rest.strip_suffix(STAGING_EXT.as_bytes()) else {
        return false;
    };
    let mut parts = core.split(|b| *b == b'.');
    let digits = |p: Option<&[u8]>| p.is_some_and(|p| !p.is_empty() && p.iter().all(u8::is_ascii_digit));
    digits(parts.next()) && digits(parts.next()) && parts.next().is_none()
}

/// 清扫本库名下死去 init 留下的暂存残留。调用方必须已持有该库的域进程锁。
pub(super) fn sweep_stale(db: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = parent_dir(db);
    let mut removed = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("读目录 {} 失败：{e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("读目录项失败：{e}"))?;
        if !is_staging_entry(db, &entry.file_name()) {
            continue;
        }
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| format!("读 {} 类型失败：{e}", path.display()))?;
        if !kind.is_file() {
            // 名字像暂存但不是普通文件：不认识的东西不删，交由人工处理。
            continue;
        }
        std::fs::remove_file(&path)
            .map_err(|e| format!("清扫暂存残留 {} 失败：{e}", path.display()))?;
        removed.push(path);
    }
    Ok(removed)
}

/// 一次 init 的暂存文件；未发布即 drop 时删除暂存文件与旁文件（绝不触及目标）。
pub(super) struct Staging {
    target: PathBuf,
    path: PathBuf,
    published: bool,
}

impl Staging {
    pub(super) fn new(db: &Path) -> Result<Self, String> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("读系统时钟失败：{e}"))?
            .as_nanos();
        let mut name = staging_prefix(db)?;
        name.push(format!("{}.{nanos}{STAGING_EXT}", std::process::id()));
        let path = parent_dir(db).join(name);
        if path.exists() {
            return Err(format!("暂存路径已存在（{}）", path.display()));
        }
        Ok(Self {
            target: db.to_path_buf(),
            path,
            published: false,
        })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    fn side(&self, suffix: &str) -> PathBuf {
        let mut name = self.path.as_os_str().to_os_string();
        name.push(suffix);
        PathBuf::from(name)
    }

    /// 发布：调用方必须已关闭全部连接。旁文件残留（WAL 未检查点）即拒绝发布。
    pub(super) fn publish(mut self) -> Result<(), String> {
        for suffix in ["-wal", "-journal"] {
            let side = self.side(suffix);
            match std::fs::metadata(&side) {
                Ok(m) if m.len() > 0 => {
                    return Err(format!(
                        "StorageUnavailable：暂存库关闭后仍有未合并的 {}（{} 字节），拒绝发布",
                        side.display(),
                        m.len()
                    ))
                }
                _ => {}
            }
        }
        std::fs::File::open(&self.path)
            .and_then(|f| f.sync_all())
            .map_err(|e| format!("StorageUnavailable：fsync 暂存库失败：{e}"))?;
        rename_noreplace(&self.path, &self.target).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                super::existing_db_error(&self.target)
            } else {
                format!("StorageUnavailable：原子发布暂存库失败：{e}")
            }
        })?;
        self.published = true;
        self.remove_sides();
        std::fs::File::open(parent_dir(&self.target))
            .and_then(|d| d.sync_all())
            .map_err(|e| format!("StorageUnavailable：fsync 目录失败：{e}"))?;
        Ok(())
    }

    fn remove_sides(&self) {
        for suffix in SIDE_SUFFIXES {
            let _ = std::fs::remove_file(self.side(suffix));
        }
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if !self.published {
            let _ = std::fs::remove_file(&self.path);
            self.remove_sides();
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn c_path(path: &Path) -> std::io::Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "路径含 NUL"))
}

/// 不覆盖的原子 rename；目标已存在时返回 `AlreadyExists`。
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rename_noreplace(from: &Path, to: &Path) -> std::io::Result<()> {
    let (from, to) = (c_path(from)?, c_path(to)?);
    #[cfg(target_os = "linux")]
    let rc = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(target_os = "macos")]
    let rc = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn rename_noreplace(_from: &Path, _to: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "本平台无不覆盖原子 rename，s-session/3 init 拒绝执行",
    ))
}
