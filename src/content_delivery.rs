use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HOST};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt, time::SystemTime};
use time::OffsetDateTime;

const ALGORITHM: &str = "TC3-HMAC-SHA256";
const CONTENT_TYPE_JSON: &str = "application/json; charset=utf-8";
const ENDPOINT: &str = "https://cdn.tencentcloudapi.com/";
const HOST_NAME: &str = "cdn.tencentcloudapi.com";
const SERVICE: &str = "cdn";
const SIGNED_HEADERS: &str = "content-type;host";
const TERMINATOR: &str = "tc3_request";
const VERSION: &str = "2018-06-06";

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct TencentCloudCredentials {
    pub secret_id: String,
    pub secret_key: String,
    pub region: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PurgePathCacheRequest {
    pub paths: Vec<String>,
    pub flush_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_encode: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgePathCacheResponse {
    pub task_id: String,
    pub request_id: String,
}

#[derive(Debug)]
pub enum ContentDeliveryError {
    Api(TencentCloudApiError),
    Crypto(String),
    Http(reqwest::Error),
    InvalidTimestamp(u64),
    ResponseParse {
        status: u16,
        body: String,
        source: serde_json::Error,
    },
    SerializeRequest(serde_json::Error),
    UnexpectedResponse {
        status: u16,
        body: String,
    },
}

#[derive(Debug, Clone)]
pub struct TencentCloudApiError {
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TencentCloudEnvelope {
    #[serde(rename = "Response")]
    response: TencentCloudEnvelopeBody,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TencentCloudEnvelopeBody {
    #[serde(default)]
    error: Option<TencentCloudErrorBody>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TencentCloudErrorBody {
    code: String,
    message: String,
}

impl fmt::Display for ContentDeliveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Api(err) => {
                if let Some(request_id) = &err.request_id {
                    write!(
                        f,
                        "Tencent Cloud API error {}: {} (request_id={})",
                        err.code, err.message, request_id
                    )
                } else {
                    write!(f, "Tencent Cloud API error {}: {}", err.code, err.message)
                }
            }
            Self::Crypto(message) => {
                write!(f, "failed to compute Tencent Cloud signature: {message}")
            }
            Self::Http(source) => write!(f, "HTTP request to Tencent Cloud failed: {source}"),
            Self::InvalidTimestamp(timestamp) => {
                write!(
                    f,
                    "timestamp {timestamp} is out of range for UTC date conversion"
                )
            }
            Self::ResponseParse {
                status,
                body,
                source,
            } => write!(
                f,
                "failed to parse Tencent Cloud response (status {status}): {source}; body={body}"
            ),
            Self::SerializeRequest(source) => {
                write!(f, "failed to serialize PurgePathCache request: {source}")
            }
            Self::UnexpectedResponse { status, body } => {
                write!(
                    f,
                    "unexpected Tencent Cloud response (status {status}): {body}"
                )
            }
        }
    }
}

impl Error for ContentDeliveryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Http(source) => Some(source),
            Self::ResponseParse { source, .. } => Some(source),
            Self::SerializeRequest(source) => Some(source),
            _ => None,
        }
    }
}

pub fn purge_path_cache(
    credentials: &TencentCloudCredentials,
    request: &PurgePathCacheRequest,
) -> Result<PurgePathCacheResponse, ContentDeliveryError> {
    let client = Client::builder()
        .build()
        .map_err(ContentDeliveryError::Http)?;
    let timestamp = current_unix_timestamp().map_err(ContentDeliveryError::InvalidTimestamp)?;

    purge_path_cache_with_timestamp(&client, credentials, request, timestamp)
}

fn purge_path_cache_with_timestamp(
    client: &Client,
    credentials: &TencentCloudCredentials,
    request: &PurgePathCacheRequest,
    timestamp: u64,
) -> Result<PurgePathCacheResponse, ContentDeliveryError> {
    let payload = serialize_payload(request)?;
    let authorization = build_authorization(credentials, &payload, timestamp)?;

    let mut request_builder = client
        .post(ENDPOINT)
        .header(AUTHORIZATION, authorization)
        .header(CONTENT_TYPE, CONTENT_TYPE_JSON)
        .header(HOST, HOST_NAME)
        .header("X-TC-Action", "PurgePathCache")
        .header("X-TC-Timestamp", timestamp.to_string())
        .header("X-TC-Version", VERSION);

    if !credentials.region.trim().is_empty() {
        request_builder = request_builder.header("X-TC-Region", credentials.region.as_str());
    }

    let response = request_builder
        .body(payload)
        .send()
        .map_err(ContentDeliveryError::Http)?;
    let status = response.status().as_u16();
    let body = response.text().map_err(ContentDeliveryError::Http)?;

    parse_response(status, &body)
}

fn parse_response(status: u16, body: &str) -> Result<PurgePathCacheResponse, ContentDeliveryError> {
    let envelope: TencentCloudEnvelope =
        serde_json::from_str(body).map_err(|source| ContentDeliveryError::ResponseParse {
            status,
            body: body.to_owned(),
            source,
        })?;

    if let Some(error) = envelope.response.error {
        return Err(ContentDeliveryError::Api(TencentCloudApiError {
            code: error.code,
            message: error.message,
            request_id: envelope.response.request_id,
        }));
    }

    match (envelope.response.task_id, envelope.response.request_id) {
        (Some(task_id), Some(request_id)) => Ok(PurgePathCacheResponse {
            task_id,
            request_id,
        }),
        _ => Err(ContentDeliveryError::UnexpectedResponse {
            status,
            body: body.to_owned(),
        }),
    }
}

fn serialize_payload(request: &PurgePathCacheRequest) -> Result<String, ContentDeliveryError> {
    serde_json::to_string(request).map_err(ContentDeliveryError::SerializeRequest)
}

fn build_authorization(
    credentials: &TencentCloudCredentials,
    payload: &str,
    timestamp: u64,
) -> Result<String, ContentDeliveryError> {
    let date = utc_date_from_timestamp(timestamp)?;
    let credential_scope = format!("{date}/{SERVICE}/{TERMINATOR}");
    let canonical_request = build_canonical_request(HOST_NAME, payload);
    let hashed_canonical_request = sha256_hex(&canonical_request);
    let string_to_sign =
        build_string_to_sign(timestamp, &credential_scope, &hashed_canonical_request);
    let signature = build_signature(&credentials.secret_key, &date, SERVICE, &string_to_sign)?;

    Ok(format!(
        "{ALGORITHM} Credential={}/{credential_scope}, SignedHeaders={SIGNED_HEADERS}, Signature={signature}",
        credentials.secret_id
    ))
}

fn build_canonical_request(host: &str, payload: &str) -> String {
    let canonical_headers = format!("content-type:{CONTENT_TYPE_JSON}\nhost:{host}\n");
    let hashed_request_payload = sha256_hex(payload);

    format!("POST\n/\n\n{canonical_headers}\n{SIGNED_HEADERS}\n{hashed_request_payload}")
}

fn build_string_to_sign(
    timestamp: u64,
    credential_scope: &str,
    hashed_canonical_request: &str,
) -> String {
    format!("{ALGORITHM}\n{timestamp}\n{credential_scope}\n{hashed_canonical_request}")
}

fn build_signature(
    secret_key: &str,
    date: &str,
    service: &str,
    string_to_sign: &str,
) -> Result<String, ContentDeliveryError> {
    let secret_date = hmac_sha256(format!("TC3{secret_key}").as_bytes(), date)?;
    let secret_service = hmac_sha256(&secret_date, service)?;
    let secret_signing = hmac_sha256(&secret_service, TERMINATOR)?;
    let signature = hmac_sha256(&secret_signing, string_to_sign)?;

    Ok(hex::encode(signature))
}

fn hmac_sha256(key: &[u8], message: &str) -> Result<Vec<u8>, ContentDeliveryError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|err| ContentDeliveryError::Crypto(err.to_string()))?;
    mac.update(message.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)
}

fn utc_date_from_timestamp(timestamp: u64) -> Result<String, ContentDeliveryError> {
    let offset_datetime = OffsetDateTime::from_unix_timestamp(timestamp as i64)
        .map_err(|_| ContentDeliveryError::InvalidTimestamp(timestamp))?;

    Ok(format!(
        "{:04}-{:02}-{:02}",
        offset_datetime.year(),
        u8::from(offset_datetime.month()),
        offset_datetime.day()
    ))
}

fn current_unix_timestamp() -> Result<u64, u64> {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| 0_u64)?;

    Ok(duration.as_secs())
}