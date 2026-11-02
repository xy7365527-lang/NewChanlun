//! #1394：显式Q3命令的配置冻结、监听与信号收尾。
//! `query-serve --protocol-revision s-session/3`只服务Q API，不改旧Python入口或旧S命令。
//! 启用新服务前仍须具名实测资源；本模块不提供任何Q21资源缺省值。
use super::{q_listener::Listener, q_resources::QResources, q_runtime::Runtime, PROTOCOL_REVISION};
use newchan_rust::session_protocol::positive;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

const FLAGS: [&str; 6] = [
    "--protocol-revision",
    "--db",
    "--resource-config",
    "--producer-epoch",
    "--host",
    "--port",
];
struct Options {
    db: PathBuf,
    resources: PathBuf,
    epoch: String,
    host: String,
    port: u16,
}
impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        if args.get(1).map(String::as_str) != Some("query-serve") {
            return Err("Q命令必须为query-serve".into());
        }
        let mut values = BTreeMap::new();
        let mut index = 2;
        while index < args.len() {
            let key = args[index].as_str();
            if !FLAGS.contains(&key) {
                return Err(format!("Q参数不支持：{key}"));
            }
            let value = args
                .get(index + 1)
                .filter(|v| !v.is_empty() && !v.starts_with("--"))
                .ok_or_else(|| format!("Q参数缺值：{key}"))?;
            if values.insert(key, value.as_str()).is_some() {
                return Err(format!("Q参数重复：{key}"));
            }
            index += 2;
        }
        let required = |key| {
            values
                .get(key)
                .copied()
                .ok_or_else(|| format!("Q缺必填参数：{key}"))
        };
        if required("--protocol-revision")? != PROTOCOL_REVISION {
            return Err("query-serve只接受显式s-session/3".into());
        }
        let epoch = required("--producer-epoch")?;
        positive(epoch, "Q producer_epoch")?;
        // 只沿旧HTTP入口的地址缺省；Q资源文件21项仍全部显式且冻结。
        let port = values
            .get("--port")
            .copied()
            .unwrap_or("8787")
            .parse::<u16>()
            .map_err(|_| "Q端口须为0..65535".to_string())?;
        Ok(Self {
            db: required("--db")?.into(),
            resources: required("--resource-config")?.into(),
            epoch: epoch.into(),
            host: values.get("--host").copied().unwrap_or("127.0.0.1").into(),
            port,
        })
    }
    fn load(&self) -> Result<(PathBuf, Arc<QResources>), String> {
        let resources = Arc::new(QResources::load(&self.resources)?);
        let db = self
            .db
            .canonicalize()
            .map_err(|e| format!("Q只读数据库路径不可用：{e}"))?;
        if !db.is_file() {
            return Err("Q只读数据库必须是已有文件".into());
        }
        Ok((db, resources))
    }
}

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let options = Options::parse(args)?;
    let (db, resources) = options.load()?;
    let _signals = Signals::install()?;
    let listener = Listener::bind(&options.host, options.port, &resources)?;
    let address = listener.local_addr()?;
    let runtime = Runtime::new(db.clone(), Arc::clone(&resources), &options.epoch)?
        .with_control_instance_id(std::env::var("S_CONTROL_INSTANCE_ID").unwrap_or_default())?;
    eprintln!(
        "[q3] {}",
        serde_json::json!({"listen":address.to_string(),"db":db,"producer_epoch":options.epoch,"resource_config_sha256":resources.digest(),"resource_config":serde_json::from_str::<serde_json::Value>(resources.canonical()).map_err(|e|e.to_string())?})
    );
    listener.run(runtime, &STOP)
}

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop_signal(_: libc::c_int) {
    STOP.store(true, Ordering::Release);
}
struct Signals {
    interrupt: libc::sigaction,
    terminate: libc::sigaction,
}
impl Signals {
    fn install() -> Result<Self, String> {
        STOP.store(false, Ordering::Release);
        // SAFETY：sigaction是C纯数据结构；mask由sigemptyset初始化，旧动作由内核写入。
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        let mut interrupt: libc::sigaction = unsafe { std::mem::zeroed() };
        let mut terminate: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = stop_signal as *const () as usize;
        action.sa_flags = 0;
        if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if unsafe { libc::sigaction(libc::SIGINT, &action, &mut interrupt) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if unsafe { libc::sigaction(libc::SIGTERM, &action, &mut terminate) } != 0 {
            let error = std::io::Error::last_os_error().to_string();
            unsafe {
                libc::sigaction(libc::SIGINT, &interrupt, std::ptr::null_mut());
            }
            return Err(error);
        }
        Ok(Self {
            interrupt,
            terminate,
        })
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        // SAFETY：两个旧动作来自本实例install；恢复发生在worker关闭后，handler仅写原子标记。
        unsafe {
            libc::sigaction(libc::SIGTERM, &self.terminate, std::ptr::null_mut());
            libc::sigaction(libc::SIGINT, &self.interrupt, std::ptr::null_mut());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args() -> Vec<String> {
        [
            "s_structure_session",
            "query-serve",
            "--protocol-revision",
            "s-session/3",
            "--db",
            "absent-test-only.sqlite",
            "--resource-config",
            "explicit.json",
            "--producer-epoch",
            "1",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }
    #[test]
    fn requires_exact_explicit_q_inputs_before_any_io() {
        let options = Options::parse(&args()).unwrap();
        assert_eq!(options.host, "127.0.0.1");
        assert_eq!(options.port, 8787);
        for key in [
            "--protocol-revision",
            "--db",
            "--resource-config",
            "--producer-epoch",
        ] {
            let mut a = args();
            let i = a.iter().position(|x| x == key).unwrap();
            a.drain(i..i + 2);
            assert!(Options::parse(&a).is_err());
        }
        for tail in [
            vec!["--resource-config", "another.json"],
            vec!["--db"],
            vec!["--browser", "v2.html"],
            vec!["stray"],
        ] {
            let mut a = args();
            a.extend(tail.into_iter().map(str::to_owned));
            assert!(Options::parse(&a).is_err());
        }
        for epoch in ["0", "01", "-1", "9223372036854775808"] {
            let mut a = args();
            *a.last_mut().unwrap() = epoch.into();
            assert!(Options::parse(&a).is_err());
        }
        let mut a = args();
        a.extend(
            ["--host", "localhost", "--port", "0"]
                .into_iter()
                .map(str::to_owned),
        );
        let o = Options::parse(&a).unwrap();
        assert_eq!(o.host, "localhost");
        assert_eq!(o.port, 0);
        let mut a = args();
        a[3] = "s-session/2".into();
        assert!(Options::parse(&a).is_err());
    }
    #[test]
    fn loading_missing_resources_or_database_does_not_bind_or_create_them() {
        let dir = std::env::temp_dir().join(format!(
            "q3-cli-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let mut o = Options::parse(&args()).unwrap();
        o.db = dir.join("missing.sqlite");
        o.resources = dir.join("resources.json");
        assert!(o.load().is_err());
        assert!(!o.db.exists());
        assert!(!o.resources.exists());
        std::fs::write(
            &o.resources,
            super::super::q_resources::tests::resources().canonical(),
        )
        .unwrap();
        assert!(o.load().is_err());
        assert!(!o.db.exists());
        std::fs::remove_file(o.resources).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn selector_dispatch_rejects_missing_q_inputs_before_listening() {
        let a: Vec<String> = [
            "s_structure_session",
            "query-serve",
            "--protocol-revision",
            "s-session/3",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert!(super::super::dispatch_fixture(&a)
            .unwrap_err()
            .contains("--producer-epoch"));
        let mut old = a.clone();
        old[3] = "s-session/2".into();
        assert!(super::super::dispatch_fixture(&old)
            .unwrap_err()
            .contains("VersionUnsupported"));
        assert!(!super::super::dispatch_fixture(&a[..2]).unwrap());
    }
}
