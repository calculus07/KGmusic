//! kgmusic-server：KGmusic 的 Rust 后端可执行入口。
//!
//! 同一个进程内提供两部分能力：
//!  1. 酷狗 API（默认挂载在 `/api` 前缀下，移植自 md3Music 的 kugou_server）；
//!  2. 前端构建产物（`web/` 下的 Vue SPA，`npm run build:web` 产出到 `web/dist`）。
//!
//! 每项参数既可用命令行传入，也可用同名环境变量，命令行优先。
//! `--help` 查看完整说明。

use kgmusic::server::{self, RuntimeConfig};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
kgmusic-server — KGmusic 后端（酷狗 API + 前端静态文件）

用法:
  kgmusic-server [选项]

选项（同名环境变量写在括号里，命令行优先）:
  --host <HOST>          (HOST)        监听地址，默认 0.0.0.0
  --port <PORT>          (PORT)        监听端口，默认 6521；0 表示随机端口
  --data-dir <DIR>       (DATA_DIR)    设备信息持久化目录，默认 ./data
  --static-dir <DIR>     (STATIC_DIR)  前端构建产物目录，默认 web/dist（相对项目根，不存在则只提供 API）
  --api-prefix <PREFIX>  (API_PREFIX)  API 挂载前缀，默认 /api；传空串则挂在根路径
  -h, --help                           显示本帮助
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let mut opts = Options::from_env();
    if let Err(err) = opts.apply_args(&args) {
        eprintln!("[kgmusic-server] 参数错误: {err}\n\n{USAGE}");
        return ExitCode::FAILURE;
    }

    let data_dir = match resolve_dir(&opts.data_dir, true) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("[kgmusic-server] 数据目录不可用: {err}");
            return ExitCode::FAILURE;
        }
    };
    // 前端还没构建时不应该让服务起不来：静态目录缺失就退化成纯 API 模式。
    let static_dir = match resolve_dir(&opts.static_dir, false) {
        Ok(dir) => Some(dir),
        Err(err) => {
            eprintln!("[kgmusic-server] 警告: 静态目录不可用，仅提供 API（{err}）");
            None
        }
    };

    let cfg = RuntimeConfig {
        host: opts.host.clone(),
        port: opts.port,
        data_dir: data_dir.to_string_lossy().into_owned(),
        static_dir: static_dir
            .as_ref()
            .map(|d| d.to_string_lossy().into_owned()),
        api_prefix: opts.api_prefix.clone(),
    };

    let Some(port) = server::start_with(cfg) else {
        eprintln!(
            "[kgmusic-server] 启动失败：无法监听 {}:{}",
            opts.host, opts.port
        );
        return ExitCode::FAILURE;
    };

    // 0.0.0.0 是通配地址，直接给用户看没有意义，换成回环地址便于点开。
    let display_host = if opts.host == "0.0.0.0" || opts.host == "::" {
        "127.0.0.1"
    } else {
        opts.host.as_str()
    };
    println!("[kgmusic-server] 监听 {}:{}", opts.host, port);
    match static_dir.as_deref() {
        Some(dir) => println!(
            "[kgmusic-server] 前端: http://{display_host}:{port}/  （静态目录 {}）",
            dir.display()
        ),
        None => println!("[kgmusic-server] 前端: 未托管静态文件（--static-dir 目录不存在）"),
    }
    println!(
        "[kgmusic-server] API : http://{display_host}:{port}{}/",
        opts.api_prefix
    );
    println!("[kgmusic-server] 数据目录: {}", data_dir.display());

    // 监听线程自己跑，主线程就此挂起；收到 SIGINT/SIGTERM 由系统终止进程。
    loop {
        std::thread::park();
    }
}

/// 服务器启动参数。
struct Options {
    host: String,
    port: u16,
    data_dir: String,
    static_dir: String,
    api_prefix: String,
}

impl Options {
    fn from_env() -> Self {
        let env = |key: &str, default: &str| {
            std::env::var(key)
                .ok()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| default.to_string())
        };
        Self {
            host: env("HOST", "0.0.0.0"),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(6521),
            data_dir: env("DATA_DIR", "./data"),
            static_dir: env("STATIC_DIR", "web/dist"),
            // API_PREFIX 允许显式设为空串（挂在根路径），所以不能用 env() 的过滤逻辑。
            api_prefix: std::env::var("API_PREFIX").unwrap_or_else(|_| "/api".to_string()),
        }
    }

    /// 解析命令行：支持 `--key value` 与 `--key=value` 两种写法。
    fn apply_args(&mut self, args: &[String]) -> Result<(), String> {
        let mut i = 0;
        while i < args.len() {
            let raw = args[i].as_str();
            let (key, inline) = match raw.split_once('=') {
                Some((k, v)) => (k, Some(v.to_string())),
                None => (raw, None),
            };
            let mut value = || -> Result<String, String> {
                match inline.clone() {
                    Some(v) => Ok(v),
                    None => {
                        i += 1;
                        args.get(i)
                            .cloned()
                            .ok_or_else(|| format!("{key} 缺少取值"))
                    }
                }
            };
            match key {
                "--host" => self.host = value()?,
                "--port" => {
                    let v = value()?;
                    self.port = v
                        .trim()
                        .parse()
                        .map_err(|_| format!("--port 需要 0-65535 的整数，收到 {v:?}"))?;
                }
                "--data-dir" => self.data_dir = value()?,
                "--static-dir" => self.static_dir = value()?,
                "--api-prefix" => self.api_prefix = normalize_prefix(&value()?),
                other => return Err(format!("未知参数 {other}")),
            }
            i += 1;
        }
        Ok(())
    }
}

/// API 前缀归一化：空串保持为空（挂根路径），否则补前导 `/`、去尾部 `/`。
/// 只接受单层前缀（如 `/api`），避免出现 `/a/b` 这类容易配错的形式。
fn normalize_prefix(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    format!("/{trimmed}")
}

/// 解析并规范化目录路径。`create` 为真时按需创建；
/// 为假且目录不存在时返回 `Err`，由调用方决定是降级还是退出。
fn resolve_dir(raw: &str, create: bool) -> Result<PathBuf, String> {
    let path = Path::new(raw);
    if !path.is_dir() {
        if !create {
            return Err(format!("{raw} 不存在"));
        }
        std::fs::create_dir_all(path).map_err(|e| format!("创建 {raw} 失败: {e}"))?;
    }
    path.canonicalize()
        .map_err(|e| format!("解析 {raw} 失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(args: &[&str]) -> Options {
        let mut o = Options {
            host: "0.0.0.0".into(),
            port: 6521,
            data_dir: "./data".into(),
            static_dir: "web/dist".into(),
            api_prefix: "/api".into(),
        };
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        o.apply_args(&owned).expect("args should parse");
        o
    }

    #[test]
    fn parses_space_and_equals_forms() {
        let o = opts(&[
            "--host",
            "127.0.0.1",
            "--port=7000",
            "--static-dir",
            "/srv/dist",
        ]);
        assert_eq!(o.host, "127.0.0.1");
        assert_eq!(o.port, 7000);
        assert_eq!(o.static_dir, "/srv/dist");
    }

    /// 空前缀必须保持为空——它表示「API 挂在根路径」，被归一化成 "/api" 就错了。
    #[test]
    fn empty_api_prefix_means_root_mount() {
        assert_eq!(opts(&["--api-prefix", ""]).api_prefix, "");
        assert_eq!(opts(&["--api-prefix="]).api_prefix, "");
        assert_eq!(opts(&["--api-prefix", "api/"]).api_prefix, "/api");
        assert_eq!(opts(&["--api-prefix", "/kugou"]).api_prefix, "/kugou");
    }

    #[test]
    fn rejects_unknown_flag_and_bad_port() {
        let owned = vec!["--nope".to_string()];
        let mut o = opts(&[]);
        assert!(o.apply_args(&owned).is_err());

        let owned = vec!["--port".to_string(), "abc".to_string()];
        assert!(o.apply_args(&owned).is_err());
    }
}
