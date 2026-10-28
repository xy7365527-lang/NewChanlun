//! #1657：操作员签发/撤销；与 R4 只读消费分离，共享已审合同与身份规则。
use super::super::admission::Lock;
use super::*;
use newchan_rust::session_protocol::canonical_json;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Write;

const VERSION: &str = "s-admission-small/1-awaiting-measurement";
const CONTRACT: &str = include_str!("../../../../s_session/contracts/admission-small-v1.json");
const REVOKING: &str = ".s-admission-revoking";

type Result<T> = std::result::Result<T, String>;

struct Contract<'a> {
    version: &'a str,
    canonical: &'a str,
}
const CURRENT: Contract<'static> = Contract {
    version: VERSION,
    canonical: CONTRACT,
};

impl Contract<'_> {
    fn authorize(&self, version: &str, sha: &str, timeout: u64) -> Result<JobLimits> {
        if version.is_empty() {
            return Err("AdmissionContractVersionMissing".into());
        }
        if version != self.version {
            return Err("AdmissionContractSupersededOrUnknown".into());
        }
        let node = strict_json(self.canonical.as_bytes())?;
        let canonical = canonical_json(&node);
        if sha256_hex(canonical.as_bytes()) != sha {
            return Err("AdmissionContractUnapproved".into());
        }
        if node
            .as_object()
            .is_some_and(|m| m.values().any(|v| v == "AwaitingMeasurement"))
        {
            return Err("AdmissionContractAwaitingMeasurement".into());
        }
        let limits = JobLimits::from_canonical_json(&node).map_err(|e| e.to_string())?;
        if timeout < limits.max_elapsed_ms {
            return Err("AdmissionStartupWindowTooShort".into());
        }
        Ok(limits)
    }
}

fn io_error(e: io::Error) -> String {
    format!("AdmissionIo:{:?}", e.kind())
}
fn sidecar(db: &Path, suffix: &str) -> PathBuf {
    let mut s = db.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}
fn identity(file: &File) -> Result<Value> {
    let m = file.metadata().map_err(io_error)?;
    Ok(json!({"device":m.dev(),"inode":m.ino(),"uid":m.uid(),"mode":m.mode()&0o7777}))
}
fn create_file(dir: &File, name: &OsStr) -> Result<File> {
    let name = c_name(name).map_err(io_error)?;
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(io_error(io::Error::last_os_error()));
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn write_file(file: &mut File, raw: &[u8]) -> Result<()> {
    file.write_all(raw).map_err(io_error)?;
    file.sync_all().map_err(io_error)
}
fn mkdir(dir: &File, name: &str) -> Result<File> {
    let c = c_name(OsStr::new(name)).map_err(io_error)?;
    if unsafe { libc::mkdirat(dir.as_raw_fd(), c.as_ptr(), 0o700) } != 0 {
        return Err(io_error(io::Error::last_os_error()));
    }
    dir.sync_all().map_err(io_error)?;
    open_at(dir, OsStr::new(name), true).map_err(io_error)
}
fn move_new(from: &File, name: &str, to: &File, target: &str) -> Result<()> {
    let a = c_name(OsStr::new(name)).map_err(io_error)?;
    let b = c_name(OsStr::new(target)).map_err(io_error)?;
    #[cfg(target_os = "linux")]
    let rc = unsafe {
        libc::renameat2(
            from.as_raw_fd(),
            a.as_ptr(),
            to.as_raw_fd(),
            b.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(target_os = "macos")]
    let rc = unsafe {
        libc::renameatx_np(
            from.as_raw_fd(),
            a.as_ptr(),
            to.as_raw_fd(),
            b.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let rc = -1;
    if rc != 0 {
        return Err(io_error(io::Error::last_os_error()));
    }
    from.sync_all().map_err(io_error)?;
    to.sync_all().map_err(io_error)
}
fn read_small(dir: &File, name: &OsStr, uid: u32) -> Result<(File, Vec<u8>, FileStamp)> {
    let mut file = open_at(dir, name, false).map_err(io_error)?;
    let m = file.metadata().map_err(io_error)?;
    if !m.is_file()
        || m.nlink() != 1
        || m.uid() != uid
        || !matches!(m.mode() & 0o7777, 0o600 | 0o644)
    {
        return Err("AdmissionControlIdentity".into());
    }
    let stamp = FileStamp::of(&m);
    let mut raw = Vec::new();
    (&mut file)
        .take((CONTROL_BYTES + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(io_error)?;
    if raw.len() > CONTROL_BYTES || FileStamp::of(&file.metadata().map_err(io_error)?) != stamp {
        return Err("AdmissionControlChanged".into());
    }
    Ok((file, raw, stamp))
}

struct Site {
    db: PathBuf,
    state_path: PathBuf,
    parent: PinnedDirectory,
    state: PinnedDirectory,
    db_file: File,
    db_identity: Identity,
    locks: Vec<Lock>,
    uid: u32,
}
impl Site {
    fn open(db: &Path, state: &Path, service_uid: u32) -> Result<Self> {
        let uid = unsafe { libc::geteuid() };
        if uid != service_uid {
            return Err("AdmissionForeignUid".into());
        }
        let db = std::fs::canonicalize(db).map_err(io_error)?;
        let state_path = state
            .to_str()
            .and_then(absolute_path)
            .ok_or("AdmissionStatePath")?;
        let parent = PinnedDirectory::new(
            open_directory_path(db.parent().ok_or("AdmissionDbPath")?).map_err(io_error)?,
        )
        .map_err(io_error)?;
        let state = PinnedDirectory::new(open_directory_path(&state_path).map_err(io_error)?)
            .map_err(io_error)?;
        if state.identity.uid != uid {
            return Err("AdmissionForeignUid".into());
        }
        let db_file = open_at(
            &parent.file,
            db.file_name().ok_or("AdmissionDbPath")?,
            false,
        )
        .map_err(io_error)?;
        let m = db_file.metadata().map_err(io_error)?;
        if !m.is_file() || m.nlink() != 1 || m.uid() != uid || m.mode() & 0o022 != 0 {
            return Err("AdmissionDatabaseIdentity".into());
        }
        let locks = vec![
            Lock::acquire(&state_path.join(".control"), ".lock")?,
            Lock::acquire(&db, ".control.lock")?,
            Lock::acquire(&db, ".writer.lock")?,
            Lock::acquire(&db, ".maintenance.lock")?,
        ];
        let site = Self {
            db,
            state_path,
            parent,
            state,
            db_file,
            db_identity: Identity::of(&m),
            locks,
            uid,
        };
        site.check()?;
        Ok(site)
    }
    fn check(&self) -> Result<()> {
        for lock in &self.locks {
            lock.check()?;
        }
        let p = open_directory_path(self.db.parent().unwrap()).map_err(io_error)?;
        let s = open_directory_path(&self.state_path).map_err(io_error)?;
        if !self.parent.matches(&p) || !self.state.matches(&s) {
            return Err("AdmissionDirectoryChanged".into());
        }
        let current = open_at(&p, self.db.file_name().unwrap(), false).map_err(io_error)?;
        for f in [&current, &self.db_file] {
            let m = f.metadata().map_err(io_error)?;
            if !m.is_file() || m.nlink() != 1 || Identity::of(&m) != self.db_identity {
                return Err("AdmissionDatabaseChanged".into());
            }
        }
        Ok(())
    }
    fn owner_name(&self) -> OsString {
        sidecar(&self.db, ".control-owner.json")
            .file_name()
            .unwrap()
            .to_os_string()
    }
    fn owner(&self) -> Result<Option<(Value, FileStamp)>> {
        let name = self.owner_name();
        if !entry_exists(
            &self.parent.file,
            name.to_str().ok_or("AdmissionOwnerName")?,
        )
        .map_err(io_error)?
        {
            return Ok(None);
        }
        let (_, raw, stamp) = read_small(&self.parent.file, &name, self.uid)?;
        let value = strict_json(&raw)?;
        let object = value.as_object().ok_or("AdmissionOwnerInvalid")?;
        if !(object.len() == 2 || object.len() == 3 && object.contains_key("admission"))
            || object.get("db") != Some(&json!(self.db))
            || object.get("state_dir") != Some(&json!(self.state_path))
        {
            return Err("AdmissionOwnerConflict".into());
        }
        Ok(Some((value, stamp)))
    }
    fn set_owner(&self, value: &Value, previous: Option<FileStamp>) -> Result<()> {
        self.check()?;
        if self.owner()?.map(|(_, s)| s) != previous {
            return Err("AdmissionOwnerChanged".into());
        }
        let name = self.owner_name();
        let mut temp = name.clone();
        temp.push(".issuance-pending");
        let mut file = create_file(&self.parent.file, &temp)?;
        write_file(&mut file, canonical_json(value).as_bytes())?;
        self.check()?;
        if self.owner()?.map(|(_, s)| s) != previous {
            return Err("AdmissionOwnerChanged".into());
        }
        if previous.is_none() {
            move_new(
                &self.parent.file,
                temp.to_str().ok_or("AdmissionOwnerName")?,
                &self.parent.file,
                name.to_str().ok_or("AdmissionOwnerName")?,
            )?;
        } else {
            let a = c_name(&temp).map_err(io_error)?;
            let b = c_name(&name).map_err(io_error)?;
            if unsafe {
                libc::renameat(
                    self.parent.file.as_raw_fd(),
                    a.as_ptr(),
                    self.parent.file.as_raw_fd(),
                    b.as_ptr(),
                )
            } != 0
            {
                return Err(io_error(io::Error::last_os_error()));
            }
            self.parent.file.sync_all().map_err(io_error)?;
        }
        Ok(())
    }
}

fn prepare(
    site: &Site,
    contract: &Contract<'_>,
    version: &str,
    input: &[u8],
    timeout: u64,
) -> Result<Value> {
    let node = strict_json(input)?;
    let raw = canonical_json(&node);
    let sha = sha256_hex(raw.as_bytes());
    contract.authorize(version, &sha, timeout)?;
    site.check()?;
    for name in [PUBLISHED, PENDING, REVOKING] {
        if entry_exists(&site.state.file, name).map_err(io_error)? {
            return Err("AdmissionExistingAnchor".into());
        }
    }
    let owner = site.owner()?;
    if owner
        .as_ref()
        .is_some_and(|(v, _)| v.get("admission").is_some())
    {
        return Err("AdmissionExistingAnchor".into());
    }
    let pending = mkdir(&site.state.file, PENDING)?;
    let mut limits = create_file(&pending, OsStr::new(LIMITS_NAME))?;
    write_file(&mut limits, raw.as_bytes())?;
    let mut binding_file = create_file(&pending, OsStr::new(BINDING_NAME))?;
    let binding = json!({"db":site.db,"path":site.state_path.join(PUBLISHED).join(LIMITS_NAME),"sha256":sha,
        "file_identity":identity(&limits)?,"binding_identity":identity(&binding_file)?,"directory_identity":identity(&pending)?});
    write_file(&mut binding_file, canonical_json(&binding).as_bytes())?;
    pending.sync_all().map_err(io_error)?;
    site.set_owner(
        &json!({"db":site.db,"state_dir":site.state_path,"admission":binding}),
        owner.map(|(_, s)| s),
    )?;
    site.check()?;
    move_new(&site.state.file, PENDING, &site.state.file, PUBLISHED)?;
    Ok(
        json!({"ok":true,"contract_version":version,"sha256":sha,"path":site.state_path.join(PUBLISHED)}),
    )
}

pub(crate) fn dispatch(args: &[String]) -> Result<bool> {
    dispatch_with(args, &CURRENT)
}

/// 生产只经 `dispatch` 传入编译期 `CURRENT`；测试借此用已实测的测试合同走同一参数与判据路径。
fn dispatch_with(args: &[String], contract: &Contract<'_>) -> Result<bool> {
    let command = args.get(1).map(String::as_str).unwrap_or("");
    if !matches!(
        command,
        "admission-prepare" | "admission-revoke" | "admission-contract"
    ) {
        return Ok(false);
    }
    let allowed: &[&str] = match command {
        "admission-contract" => &["--contract-version", "--limits-sha", "--startup-timeout-ms"],
        "admission-prepare" => &[
            "--db",
            "--state-dir",
            "--service-uid",
            "--contract-version",
            "--contract",
            "--startup-timeout-ms",
        ],
        _ => &["--db", "--state-dir", "--service-uid", "--revocation-id"],
    };
    let mut fields = BTreeMap::new();
    let mut chunks = args[2..].chunks_exact(2);
    for pair in &mut chunks {
        if !allowed.contains(&pair[0].as_str())
            || pair[1].is_empty()
            || pair[1].starts_with("--")
            || fields.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("AdmissionArgumentsInvalid".into());
        }
    }
    if !chunks.remainder().is_empty() {
        return Err("AdmissionArgumentsInvalid".into());
    }
    let required = |key| {
        fields
            .get(key)
            .copied()
            .ok_or_else(|| format!("AdmissionArgumentMissing:{key}"))
    };
    let result = if command == "admission-contract" {
        let version = required("--contract-version")?;
        let sha = required("--limits-sha")?;
        let timeout = required("--startup-timeout-ms")?
            .parse()
            .map_err(|_| "AdmissionTimeoutInvalid")?;
        contract.authorize(version, sha, timeout)?;
        json!({"ok":true,"contract_version":version,"sha256":sha})
    } else {
        // 未批准的合同先拒绝，不创建锁或 pending。
        let input = if command == "admission-prepare" {
            let version = required("--contract-version")?;
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
                .open(required("--contract")?)
                .map_err(io_error)?;
            if !file.metadata().map_err(io_error)?.is_file() {
                return Err("AdmissionContractNotFile".into());
            }
            let mut raw = Vec::new();
            file.take((CONTROL_BYTES + 1) as u64)
                .read_to_end(&mut raw)
                .map_err(io_error)?;
            if raw.len() > CONTROL_BYTES {
                return Err("AdmissionContractTooLarge".into());
            }
            let value = strict_json(&raw)?;
            let canonical = canonical_json(&value);
            let timeout = required("--startup-timeout-ms")?
                .parse()
                .map_err(|_| "AdmissionTimeoutInvalid")?;
            contract.authorize(version, &sha256_hex(canonical.as_bytes()), timeout)?;
            Some((version, raw, timeout))
        } else {
            None
        };
        let uid = required("--service-uid")?
            .parse()
            .map_err(|_| "AdmissionUidInvalid")?;
        let site = Site::open(
            Path::new(required("--db")?),
            Path::new(required("--state-dir")?),
            uid,
        )?;
        if let Some((version, raw, timeout)) = input {
            prepare(&site, contract, version, &raw, timeout)?
        } else {
            revoke(&site, required("--revocation-id")?)?
        }
    };
    println!("{}", canonical_json(&result));
    Ok(true)
}

fn revoke(site: &Site, id: &str) -> Result<Value> {
    if id.is_empty() || id.len() > 64 || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return Err("AdmissionRevocationIdInvalid".into());
    }
    site.check()?;
    let archive_name = format!("s-admission-revoked-{id}");
    if entry_exists(&site.state.file, &archive_name).map_err(io_error)? {
        return Err("AdmissionRevocationAlreadyArchived".into());
    }
    let exists = entry_exists(&site.state.file, REVOKING).map_err(io_error)?;
    let (archive, intent) = if exists {
        let archive = open_at(&site.state.file, OsStr::new(REVOKING), true).map_err(io_error)?;
        let m = archive.metadata().map_err(io_error)?;
        if m.uid() != site.uid || m.mode() & 0o7777 != 0o700 {
            return Err("AdmissionRevocationIdentity".into());
        }
        let (_, raw, _) = read_small(&archive, OsStr::new("intent.json"), site.uid)?;
        (archive, strict_json(&raw)?)
    } else {
        let owner = site.owner()?;
        let mut containers = serde_json::Map::new();
        for name in [PUBLISHED, PENDING] {
            if entry_exists(&site.state.file, name).map_err(io_error)? {
                let dir = open_at(&site.state.file, OsStr::new(name), true).map_err(io_error)?;
                let m = dir.metadata().map_err(io_error)?;
                if m.uid() != site.uid || m.mode() & 0o7777 != 0o700 {
                    return Err("AdmissionRevocationIdentity".into());
                }
                containers.insert(name.to_string(), identity(&dir)?);
            }
        }
        if containers.is_empty()
            && owner
                .as_ref()
                .is_none_or(|(v, _)| v.get("admission").is_none())
        {
            return Err("AdmissionAnchorAbsent".into());
        }
        let intent = json!({"id":id,"db":site.db,"state_dir":site.state_path,
            "owner":owner.as_ref().map(|(v,_)|v),"containers":containers});
        let archive = mkdir(&site.state.file, REVOKING)?;
        let mut file = create_file(&archive, OsStr::new("intent.json"))?;
        write_file(&mut file, canonical_json(&intent).as_bytes())?;
        archive.sync_all().map_err(io_error)?;
        (archive, intent)
    };
    if intent["id"] != id
        || intent["db"] != json!(site.db)
        || intent["state_dir"] != json!(site.state_path)
    {
        return Err("AdmissionRevocationConflict".into());
    }
    let expected = intent["containers"]
        .as_object()
        .ok_or("AdmissionRevocationInvalid")?;
    for name in [PUBLISHED, PENDING] {
        let active = entry_exists(&site.state.file, name).map_err(io_error)?;
        let saved = entry_exists(&archive, name).map_err(io_error)?;
        match expected.get(name) {
            None if active || saved => return Err("AdmissionRevocationChanged".into()),
            None => {}
            Some(wanted) => {
                if active == saved {
                    return Err("AdmissionRevocationChanged".into());
                }
                let dir = if active { &site.state.file } else { &archive };
                let object = open_at(dir, OsStr::new(name), true).map_err(io_error)?;
                if identity(&object)? != *wanted {
                    return Err("AdmissionRevocationChanged".into());
                }
                site.check()?;
                if active {
                    move_new(&site.state.file, name, &archive, name)?;
                }
            }
        }
    }
    let owner = site.owner()?;
    let empty = json!({"db":site.db,"state_dir":site.state_path});
    let current = owner
        .as_ref()
        .map(|(v, _)| v.clone())
        .unwrap_or(Value::Null);
    if current != intent["owner"] && current != empty {
        return Err("AdmissionRevocationOwnerChanged".into());
    }
    // 签发/撤销在 owner 原子替换之前中断的临时原件也归档，不删证据。
    let mut temp = site.owner_name();
    temp.push(".issuance-pending");
    if entry_exists(
        &site.parent.file,
        temp.to_str().ok_or("AdmissionOwnerName")?,
    )
    .map_err(io_error)?
    {
        read_small(&site.parent.file, &temp, site.uid)?;
        move_new(
            &site.parent.file,
            temp.to_str().unwrap(),
            &archive,
            "owner-pending.json",
        )?;
    }
    if current != empty {
        site.set_owner(&empty, owner.map(|(_, s)| s))?;
    }
    site.check()?;
    move_new(&site.state.file, REVOKING, &site.state.file, &archive_name)?;
    Ok(json!({"ok":true,"revocation_id":id,"archive":site.state_path.join(archive_name)}))
}

#[cfg(test)]
#[path = "../../../../s_session/tests/issue1657_admission.rs"]
mod tests;
