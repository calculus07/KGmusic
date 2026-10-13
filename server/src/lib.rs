//! kgmusic — 酷狗 API 服务 + 前端静态文件托管（Rust）。
//!
//! 由 md3Music 的 kugou_server（Rust 实现、行为等价于 kugou_api_server/server.js）
//! 移植而来，在原库基础上做了两处面向 Web 部署的改动：
//!  1. 新增 `kgmusic-server` 可执行文件入口（src/main.rs），监听地址、端口、
//!     数据目录、静态目录均可通过命令行参数或环境变量配置；
//!  2. 同一进程内托管前端静态文件（`web/` 的 Vue 构建产物），API 默认挂载在 /api 前缀下。
//!
//! 上游 md3Music 原版以 cdylib 形式被 Flutter 通过 FFI/JNI 内嵌，本移植
//! 只保留 rlib + bin，因此移除了原来的 Android JNI 导出层。

pub mod cache;
pub mod config;
pub mod crypto;
pub mod device;
pub mod helper;
pub mod modules;
pub mod mss_http;
pub mod request;
pub mod server;
pub mod simulate;
pub mod util;
