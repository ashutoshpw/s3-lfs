use std::io::{self, Write};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Request {
    pub event: String,
    #[serde(default)]
    pub operation: String,
    #[serde(default)]
    pub concurrent: bool,
    #[serde(default)]
    pub concurrenttransfers: i32,
    #[serde(default)]
    pub oid: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: i32,
    message: String,
}

#[derive(Debug, Serialize)]
struct InitResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody>,
}

#[derive(Debug, Serialize)]
struct TransferResponse {
    event: &'static str,
    oid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody>,
}

#[derive(Debug, Serialize)]
struct ProgressResponse {
    event: &'static str,
    oid: String,
    #[serde(rename = "bytesSoFar")]
    bytes_so_far: i64,
    #[serde(rename = "bytesSinceLast")]
    bytes_since_last: i64,
}

fn send_response<T: Serialize>(payload: &T, writer: &mut dyn Write) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    bytes.push(b'\n');
    writer.write_all(&bytes)
}

pub fn send_init(code: i32, err: Option<&anyhow::Error>, writer: &mut dyn Write) -> io::Result<()> {
    let payload = if let Some(err) = err {
        InitResponse {
            error: Some(ErrorBody {
                code,
                message: format!("Init error: {}\n", err),
            }),
        }
    } else {
        InitResponse { error: None }
    };

    send_response(&payload, writer)
}

pub fn send_transfer(
    oid: &str,
    code: i32,
    err: Option<&anyhow::Error>,
    path: Option<&str>,
    writer: &mut dyn Write,
) -> io::Result<()> {
    let payload = if let Some(err) = err {
        let message = if path.is_none() {
            format!("Error uploading file: {}\n", err)
        } else {
            format!("Error downloading file: {}\n", err)
        };

        TransferResponse {
            event: "complete",
            oid: oid.to_string(),
            path: None,
            error: Some(ErrorBody { code, message }),
        }
    } else {
        TransferResponse {
            event: "complete",
            oid: oid.to_string(),
            path: path.map(ToOwned::to_owned),
            error: None,
        }
    };

    send_response(&payload, writer)
}

pub fn send_progress(
    oid: &str,
    bytes_so_far: i64,
    bytes_since_last: i64,
    writer: &mut dyn Write,
) -> io::Result<()> {
    let payload = ProgressResponse {
        event: "progress",
        oid: oid.to_string(),
        bytes_so_far,
        bytes_since_last,
    };
    send_response(&payload, writer)
}
