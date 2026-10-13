//! 组队活动（`/team/*`）：酷狗「组队领 VIP」的 5 个接口。
//!
//! 上游 KuGouMusicApi 的 `module/team_*.js` 在 md3Music 的 Rust 移植里没有实现
//! （Flutter 端没用这个功能），这里按同样的契约用 Rust 重写。
//!
//! 这几处与其余接口不同，容易踩坑：
//!  - `clienttime` 用**毫秒**（上游是 `Date.now()`），而管线默认的 clienttime 是秒；
//!  - token 先换成 H5 形态：`'h5' + RSA无填充('moc.uoguk.59::' + token)` 的大写 hex；
//!  - signature 用 web 盐（`NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt`）手工算好再塞进
//!    params，管线见到已有 signature 就会跳过自己的签名步骤；
//!  - POST 接口的签名把请求体串拼在中间（`signatureWebParamsWithBody`），
//!    与 GET 接口的 `signatureWebParams` 不是同一个算法；
//!  - `mid` 的取值是「先 cookie 后 parameter」，其余字段是「先 parameter 后 cookie」。

use crate::crypto::{crypto_rsa_encrypt, md5_hex_str};
use crate::helper::{params_joined_sorted, signature_web_params, ROUTE_WEB};
use crate::modules::{c_str, forward, param_or_cookie_num, param_or_cookie_str, q_str, Ctx};
use crate::request::ModuleResponse;
use crate::util::json_stringify;
use serde_json::{json, Map, Value};

/// 5 个接口都请求同一个网关。
const GATEWAY: &str = "https://gateway.kugou.com";
/// 上游 JS 固定写死的概念版参数（util/config.json 的 srcappid / liteAppid / liteClientver）。
const SRC_APP_ID: i64 = 2919;
const APP_ID: i64 = 3116;
const CLIENT_VER: i64 = 11440;
/// token 换 H5 形态时加的前缀。
const H5_TOKEN_PREFIX: &str = "moc.uoguk.59::";

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 5 个接口共用的上游参数。
struct Common {
    userid: i64,
    clienttime: i64,
    mid: String,
    uuid: String,
    dfid: String,
    h5_token: String,
}

impl Common {
    /// 参与签名的公共字段，顺序与上游 `paramsMap` 一致。
    fn base(&self) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("srcappid".into(), json!(SRC_APP_ID));
        m.insert("clientver".into(), json!(CLIENT_VER));
        m.insert("clienttime".into(), json!(self.clienttime));
        m.insert("mid".into(), json!(self.mid));
        m.insert("uuid".into(), json!(self.uuid));
        m.insert("dfid".into(), json!(self.dfid));
        m.insert("appid".into(), json!(APP_ID));
        m.insert("userid".into(), json!(self.userid));
        m.insert("token".into(), json!(self.h5_token));
        m
    }
}

fn collect(q: &Value) -> Common {
    let raw_token = param_or_cookie_str(q, "token", "");
    Common {
        userid: param_or_cookie_num(q, "userid", 0),
        clienttime: now_ms(),
        // 上游 team_*.js 这里 cookie 优先，且 cookie 键名叫 KUGOU_API_MID、
        // query 键名却叫 mid，两者不是同一个名字，所有接口里只有这里是这种写法。
        mid: resolve_mid(q),
        uuid: param_or_cookie_str(q, "uuid", "-"),
        dfid: param_or_cookie_str(q, "dfid", "-"),
        h5_token: format!(
            "h5{}",
            crypto_rsa_encrypt(&format!("{H5_TOKEN_PREFIX}{raw_token}"), None).to_uppercase()
        ),
    }
}

/// `signatureWebParamsWithBody`：`md5(盐 + 按 key 排序的 k=v 串 + 请求体串 + 盐)`。
/// 与 [`signature_web_params`] 的区别是中间多插了请求体，而且排序作用在 key 上。
fn signature_web_params_with_body(params: &Value, body: &str) -> String {
    let joined = params_joined_sorted(params);
    md5_hex_str(&format!("{ROUTE_WEB}{joined}{body}{ROUTE_WEB}"))
}

/// query 里的非空字符串字段；缺失或空串时返回 `None`（对应上游 `undefined`，
/// JSON.stringify 与 axios 都会把它整个丢掉）。
fn query_field(q: &Value, key: &str) -> Option<Value> {
    let v = q_str(q, key, "");
    (!v.is_empty()).then(|| json!(v))
}

/// `params?.cookie?.KUGOU_API_MID || params?.mid || ''`。
fn resolve_mid(q: &Value) -> String {
    let from_cookie = c_str(q, "KUGOU_API_MID");
    if !from_cookie.is_empty() {
        return from_cookie;
    }
    q_str(q, "mid", "")
}

/// GET 型接口：公共字段（可选带 period_id）即全部上游 query 参数。
fn get_ut(
    q: &Value,
    ctx: &Ctx,
    path: &str,
    with_period_id: bool,
) -> Result<ModuleResponse, ModuleResponse> {
    let mut params = collect(q).base();
    if with_period_id {
        if let Some(pid) = query_field(q, "period_id") {
            params.insert("period_id".into(), pid);
        }
    }
    let signature = signature_web_params(&Value::Object(params.clone()));
    params.insert("signature".into(), json!(signature));
    forward(
        q,
        ctx,
        "get",
        path,
        Some(GATEWAY),
        Some(Value::Object(params)),
        None,
        "web",
        &[("Host", "gateway.kugou.com")],
        false,
        false,
    )
}

/// POST 型接口：公共字段走 query，业务字段走 body 并参与带体签名。
fn post_ut(
    q: &Value,
    ctx: &Ctx,
    path: &str,
    body: Value,
) -> Result<ModuleResponse, ModuleResponse> {
    let params = Value::Object(collect(q).base());
    let signature = signature_web_params_with_body(&params, &json_stringify(&body));
    let mut final_params = params.as_object().cloned().unwrap_or_default();
    final_params.insert("signature".into(), json!(signature));
    forward(
        q,
        ctx,
        "post",
        path,
        Some(GATEWAY),
        Some(Value::Object(final_params)),
        Some(body),
        "web",
        &[("Host", "gateway.kugou.com")],
        false,
        false,
    )
}

/// team_period_info.js → 获取本期活动状态。
pub fn handle_period_info(q: &Value, ctx: &Ctx) -> Result<ModuleResponse, ModuleResponse> {
    get_ut(q, ctx, "/youth/v1/ut/get_period_info", false)
}

/// team_my_status.js → 获取我的状态（是否已创建/已加入队伍）。
pub fn handle_my_status(q: &Value, ctx: &Ctx) -> Result<ModuleResponse, ModuleResponse> {
    get_ut(q, ctx, "/youth/v1/ut/get_user_status", true)
}

/// team_my_info.js → 获取我的队伍信息。
pub fn handle_my_info(q: &Value, ctx: &Ctx) -> Result<ModuleResponse, ModuleResponse> {
    get_ut(q, ctx, "/youth/v1/ut/get_my_team_info", true)
}

/// team_my.js → 创建我的队伍（body 只有 period_id）。
pub fn handle_my(q: &Value, ctx: &Ctx) -> Result<ModuleResponse, ModuleResponse> {
    let mut body = Map::new();
    if let Some(pid) = query_field(q, "period_id") {
        body.insert("period_id".into(), pid);
    }
    post_ut(q, ctx, "/youth/v1/ut/create_team", Value::Object(body))
}

/// team_join.js → 加入队伍（body 只有 team_code）。
pub fn handle_join(q: &Value, ctx: &Ctx) -> Result<ModuleResponse, ModuleResponse> {
    let mut body = Map::new();
    if let Some(code) = query_field(q, "team_code") {
        body.insert("team_code".into(), code);
    }
    post_ut(q, ctx, "/youth/v1/ut/join_team", Value::Object(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_body_signature_matches_upstream_algorithm() {
        // 期望值由上游 JS 算法独立算出：
        //   cryptoMd5(SALT + Object.keys(p).sort().map(k=>`${k}=${p[k]}`).join('') + body + SALT)
        // 盐 = NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt
        let params = json!({
            "srcappid": 2919,
            "clientver": 11440,
            "clienttime": 1700000000000i64,
            "mid": "mid-value",
            "uuid": "-",
            "dfid": "dfid-value",
            "appid": 3116,
            "userid": 12345,
            "token": "h5ABCDEF",
        });
        let body = r#"{"team_code":"TEAM01"}"#;
        assert_eq!(
            signature_web_params_with_body(&params, body),
            "e0ef49f9f55cc8831870331d5caa370c"
        );
        // 同一个 params 走 GET 版签名（对 "k=v" 整体排序）得到的是另一个值，
        // 两者搞混是这类接口最常见的签名错误。
        assert_eq!(
            signature_web_params(&params),
            "1fa6aaa19df7a69f3a34be4c18c40355"
        );
    }

    /// 取空串的字段必须整个丢掉：上游 `undefined` 既不进签名也不进请求体。
    #[test]
    fn blank_fields_are_omitted() {
        let q = json!({ "period_id": "", "team_code": "T1" });
        assert!(query_field(&q, "period_id").is_none());
        assert_eq!(query_field(&q, "team_code"), Some(json!("T1")));
        assert!(query_field(&json!({}), "period_id").is_none());
    }

    /// mid 必须是 cookie 优先——这是 team 接口与其它接口唯一相反的取值顺序。
    #[test]
    fn mid_prefers_cookie_over_query() {
        let q = json!({
            "mid": "from-query",
            "cookie": { "KUGOU_API_MID": "from-cookie" },
        });
        assert_eq!(collect(&q).mid, "from-cookie");
        let q = json!({ "mid": "from-query", "cookie": {} });
        assert_eq!(collect(&q).mid, "from-query");
    }
}
