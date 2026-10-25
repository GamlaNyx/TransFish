use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Instant;

const HOST: &str = "translate.volcengineapi.com";
const REGION: &str = "cn-north-1";
const SERVICE: &str = "translate";
const VERSION: &str = "2020-06-01";
const MAX_CHARS: usize = 5000;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    pub text: String,
    pub source: String,
    pub target: String,
    pub detected: bool,
    pub latency_ms: u64,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectResult {
    pub language: String,
    pub confidence: f64,
    pub latency_ms: u64,
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Volcengine signature V4 (HMAC-SHA256).
/// Returns (authorization header, x-date header).
fn sign(creds: &Credentials, action: &str, body: &str, now: DateTime<Utc>) -> (String, String) {
    let x_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date = &x_date[..8];
    let content_sha = sha256_hex(body.as_bytes());
    let canonical_query = format!("Action={action}&Version={VERSION}");
    let canonical_headers = format!("host:{HOST}\nx-content-sha256:{content_sha}\nx-date:{x_date}");
    let signed_headers = "host;x-content-sha256;x-date";
    let canonical_request = format!(
        "POST\n/\n{canonical_query}\n{canonical_headers}\n\n{signed_headers}\n{content_sha}"
    );
    let scope = format!("{date}/{REGION}/{SERVICE}/request");
    let string_to_sign = format!(
        "HMAC-SHA256\n{x_date}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let k_date = hmac(creds.secret_access_key.as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, REGION.as_bytes());
    let k_service = hmac(&k_region, SERVICE.as_bytes());
    let k_signing = hmac(&k_service, b"request");
    let signature = hex::encode(hmac(&k_signing, string_to_sign.as_bytes()));
    let authorization = format!(
        "HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        creds.access_key_id
    );
    (authorization, x_date)
}

fn api_error_code(code: &Value) -> String {
    match code {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn friendly_error(code: &str, message: &str) -> ApiError {
    let lowered = code.to_ascii_lowercase();
    if lowered.contains("accesskey")
        || lowered.contains("signature")
        || lowered.contains("auth")
        || code == "100008"
        || code == "100009"
    {
        return ApiError::new(
            "auth",
            "AccessKey 校验没有通过，请检查 ID 与 Secret Key 是否匹配。",
            false,
        );
    }
    if lowered.contains("ratelimit") || lowered.contains("flowlimit") {
        return ApiError::new("rate_limited", "请求太频繁了，稍等片刻再试。", true);
    }
    match code {
        "-400" => ApiError::new(
            "bad_request",
            if message.is_empty() {
                "请求参数有误，请检查文本内容。".to_string()
            } else {
                format!("请求参数有误：{message}")
            },
            false,
        ),
        "-415" => ApiError::new("unsupported_pair", "暂不支持当前语言方向的翻译。", false),
        "-429" => ApiError::new("rate_limited", "请求太频繁了，稍等片刻再试。", true),
        "-500" => ApiError::new("service_error", "翻译服务暂时不可用，请稍后重试。", true),
        _ => {
            let detail = if message.is_empty() {
                format!("翻译服务返回错误（{code}）。")
            } else {
                format!("翻译服务返回错误（{code}）：{message}")
            };
            ApiError::new("service_error", detail, true)
        }
    }
}

async fn call(
    client: &reqwest::Client,
    creds: &Credentials,
    action: &str,
    body: Value,
) -> Result<Value, ApiError> {
    let body_str = serde_json::to_string(&body)
        .map_err(|e| ApiError::new("internal", format!("请求序列化失败：{e}"), false))?;
    let (authorization, x_date) = sign(creds, action, &body_str, Utc::now());
    let url = format!("https://{HOST}/?Action={action}&Version={VERSION}");

    let response = client
        .post(url)
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .header("x-date", x_date)
        .header("x-content-sha256", sha256_hex(body_str.as_bytes()))
        .header("authorization", authorization)
        .body(body_str)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                ApiError::new("timeout", "请求超时了，请检查网络后重试。", true)
            } else {
                ApiError::new("network", format!("网络请求失败：{e}"), true)
            }
        })?;

    let status = response.status();
    let raw = response
        .text()
        .await
        .map_err(|e| ApiError::new("network", format!("读取翻译结果失败：{e}"), true))?;

    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(ApiError::new(
            "auth",
            "签名校验失败，请检查 AccessKey ID 与 Secret AccessKey 是否匹配。",
            false,
        ));
    }

    let json: Value = serde_json::from_str(&raw).map_err(|_| {
        if status.is_server_error() {
            ApiError::new("service_error", "翻译服务暂时不可用，请稍后重试。", true)
        } else {
            ApiError::new(
                "bad_response",
                format!("翻译服务返回了无法解析的内容（HTTP {status}）。"),
                true,
            )
        }
    })?;

    if let Some(error) = json
        .get("ResponseMetadata")
        .and_then(|m| m.get("Error"))
        .filter(|e| !e.is_null())
    {
        let code = api_error_code(error.get("Code").unwrap_or(&Value::Null));
        let message = error
            .get("Message")
            .and_then(Value::as_str)
            .unwrap_or_default();
        return Err(friendly_error(&code, message));
    }

    if !status.is_success() {
        return Err(ApiError::new(
            "service_error",
            format!("翻译服务返回了异常状态（HTTP {status}）。"),
            true,
        ));
    }

    Ok(json)
}

fn request_id(json: &Value) -> Option<String> {
    json.get("ResponseMetadata")?
        .get("RequestId")?
        .as_str()
        .map(str::to_string)
}

/// Pick a target language for the "smart" direction: Chinese → English, everything else → Chinese.
pub fn guess_target(text: &str) -> &'static str {
    let mut han = 0usize;
    let mut kana = 0usize;
    let mut hangul = 0usize;
    for c in text.chars() {
        let cp = c as u32;
        if (0x3040..=0x30FF).contains(&cp) || (0x31F0..=0x31FF).contains(&cp) {
            kana += 1;
        } else if (0xAC00..=0xD7A3).contains(&cp) {
            hangul += 1;
        } else if (0x3400..=0x4DBF).contains(&cp) || (0x4E00..=0x9FFF).contains(&cp) {
            han += 1;
        }
    }
    if kana > 0 || hangul > 0 {
        "zh"
    } else if han > 0 {
        "en"
    } else {
        "zh"
    }
}

pub fn validate_text(text: &str) -> Result<String, ApiError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ApiError::new("empty", "先输入一段文字吧。", false));
    }
    let chars = trimmed.chars().count();
    if chars > MAX_CHARS {
        return Err(ApiError::new(
            "too_long",
            format!("文本过长：{chars} 字符，单次上限 {MAX_CHARS} 字符。"),
            false,
        ));
    }
    Ok(trimmed.to_string())
}

pub async fn translate(
    client: &reqwest::Client,
    creds: &Credentials,
    text: &str,
    requested_target: Option<&str>,
    target_mode: &str,
) -> Result<Translation, ApiError> {
    let started = Instant::now();
    let text = validate_text(text)?;

    let auto = requested_target.is_none() && target_mode == "auto";
    let mut target = requested_target
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .or_else(|| {
            if target_mode == "auto" {
                None
            } else {
                Some(target_mode.to_string())
            }
        })
        .unwrap_or_else(|| guess_target(&text).to_string());

    let mut json = call(
        client,
        creds,
        "TranslateText",
        json!({ "TargetLanguage": target, "TextList": [text] }),
    )
    .await?;

    let mut item = extract_translation(&json)?;
    let mut detected = item
        .get("DetectedSourceLanguage")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    // Smart direction: the API is the authority on the source language.
    // If our guess picked the wrong side (e.g. kanji-only Japanese), retry once.
    if auto && !detected.is_empty() {
        let want = if detected.starts_with("zh") {
            "en"
        } else {
            "zh"
        };
        if want != target {
            let retry = call(
                client,
                creds,
                "TranslateText",
                json!({ "TargetLanguage": want, "TextList": [text] }),
            )
            .await;
            if let Ok(retry_json) = retry {
                if let Ok(retry_item) = extract_translation(&retry_json) {
                    json = retry_json;
                    item = retry_item;
                    target = want.to_string();
                    detected = item
                        .get("DetectedSourceLanguage")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                }
            }
        }
    }

    let translated = item
        .get("Translation")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    if translated.is_empty() {
        return Err(ApiError::new(
            "empty_result",
            "这次没有拿到译文，换个说法再试一次？",
            true,
        ));
    }

    Ok(Translation {
        text: translated,
        source: detected,
        target,
        detected: true,
        latency_ms: started.elapsed().as_millis() as u64,
        request_id: request_id(&json),
    })
}

fn extract_translation(json: &Value) -> Result<Value, ApiError> {
    json.get("TranslationList")
        .and_then(Value::as_array)
        .and_then(|list| list.first())
        .cloned()
        .ok_or_else(|| ApiError::new("empty_result", "翻译服务没有返回译文，请重试。", true))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct DetectedLanguage {
    language: String,
    confidence: f64,
}

pub async fn detect(
    client: &reqwest::Client,
    creds: &Credentials,
    text: &str,
) -> Result<DetectResult, ApiError> {
    let started = Instant::now();
    let json = call(client, creds, "LangDetect", json!({ "TextList": [text] })).await?;
    let first = json
        .get("DetectedLanguageList")
        .and_then(Value::as_array)
        .and_then(|list| list.first())
        .cloned()
        .ok_or_else(|| ApiError::new("empty_result", "语种检测没有返回结果。", true))?;
    let language = first
        .get("Language")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let confidence = first
        .get("Confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if language.is_empty() {
        return Err(ApiError::new(
            "empty_result",
            "语种检测没有返回结果。",
            true,
        ));
    }
    Ok(DetectResult {
        language,
        confidence,
        latency_ms: started.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_guessing() {
        assert_eq!(guess_target("Hello world"), "zh");
        assert_eq!(guess_target("你好，世界"), "en");
        assert_eq!(guess_target("こんにちは"), "zh");
        assert_eq!(guess_target("안녕하세요"), "zh");
        assert_eq!(guess_target("12345"), "zh");
    }

    #[test]
    fn error_mapping() {
        let auth = friendly_error("InvalidAccessKeyId", "unknown ak");
        assert_eq!(auth.code, "auth");
        assert!(!auth.retryable);
        let signature = friendly_error("SignatureDoesNotMatch", "");
        assert_eq!(signature.code, "auth");
        let rate = friendly_error("-429", "");
        assert_eq!(rate.code, "rate_limited");
        assert!(rate.retryable);
        let unknown = friendly_error("-9999", "boom");
        assert_eq!(unknown.code, "service_error");
        assert!(unknown.retryable);
    }

    #[test]
    fn text_validation() {
        assert_eq!(validate_text("  hello  ").unwrap(), "hello");
        assert_eq!(validate_text("   ").unwrap_err().code, "empty");
        let long = "a".repeat(MAX_CHARS + 1);
        assert_eq!(validate_text(&long).unwrap_err().code, "too_long");
    }

    #[test]
    fn canonical_signature_is_stable() {
        let creds = Credentials {
            access_key_id: "AKTEST".into(),
            secret_access_key: "secret".into(),
        };
        let now = chrono::TimeZone::with_ymd_and_hms(&Utc, 2020, 4, 9, 23, 6, 48).unwrap();
        let (auth, date) = sign(
            &creds,
            "TranslateText",
            r#"{"TargetLanguage":"zh","TextList":["Hello world"]}"#,
            now,
        );
        assert_eq!(date, "20200409T230648Z");
        assert!(
            auth.starts_with("HMAC-SHA256 Credential=AKTEST/20200409/cn-north-1/translate/request")
        );
        assert!(auth.contains("SignedHeaders=host;x-content-sha256;x-date"));
    }
}
