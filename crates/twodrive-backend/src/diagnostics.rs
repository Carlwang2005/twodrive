//! Bounded diagnostics that can cross a log/IPC boundary without remote text or URLs.
use std::{fmt, io::Read, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOperation {
    Graph,
    DriveItemDelete,
    TokenRefresh,
    TokenExchange,
}
impl RequestOperation {
    fn label(self) -> &'static str {
        match self {
            Self::Graph => "graph.request",
            Self::DriveItemDelete => "driveitem.delete",
            Self::TokenRefresh => "token.refresh",
            Self::TokenExchange => "token.exchange",
        }
    }
    fn endpoint(self) -> &'static str {
        match self {
            Self::TokenRefresh | Self::TokenExchange => "oauth_token",
            Self::DriveItemDelete => "graph_default_drive",
            Self::Graph => "graph",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestFailure {
    pub status: u16,
    pub retry_after: Duration,
    pub operation: RequestOperation,
    pub code: &'static str,
    pub inner_code: &'static str,
    pub request_id: Option<String>,
    pub server_date_unix: Option<i64>,
    pub content_type: &'static str,
    pub has_claims_challenge: bool,
}
impl RequestFailure {
    pub fn new(status: u16, operation: RequestOperation) -> Self {
        Self {
            status,
            operation,
            retry_after: Duration::ZERO,
            code: "unknown",
            inner_code: "unknown",
            request_id: None,
            server_date_unix: None,
            content_type: "unknown",
            has_claims_challenge: false,
        }
    }
    pub(crate) fn response(
        mut response: reqwest::blocking::Response,
        operation: RequestOperation,
        retry_after: Duration,
    ) -> Self {
        let mut failure = Self::new(response.status().as_u16(), operation);
        failure.retry_after = retry_after;
        let headers = response.headers();
        failure.request_id = headers
            .get("request-id")
            .and_then(|v| v.to_str().ok())
            .and_then(uuid);
        failure.server_date_unix = headers
            .get("date")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                time::OffsetDateTime::parse(v, &time::format_description::well_known::Rfc2822).ok()
            })
            .map(|v| v.unix_timestamp());
        failure.content_type = match headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
        {
            Some("application/json") => "json",
            Some("text/html") => "html",
            Some(_) => "other",
            None => "unknown",
        };
        failure.has_claims_challenge = headers
            .get_all("www-authenticate")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .any(|v| v.to_ascii_lowercase().contains("claims="));
        const LIMIT: usize = 16 * 1024;
        let mut bytes = Vec::new();
        if response
            .content_length()
            .is_none_or(|size| size <= LIMIT as u64)
            && (&mut response)
                .take(LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
                .is_ok()
            && bytes.len() <= LIMIT
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
        {
            let mut error = &value["error"];
            failure.code = safe_code(error.as_str().or_else(|| error["code"].as_str()));
            for _ in 0..8 {
                let Some(inner) = error.get("innerError").or_else(|| error.get("innererror"))
                else {
                    break;
                };
                let code = safe_code(inner["code"].as_str());
                if code != "unknown" {
                    failure.inner_code = code;
                }
                if failure.request_id.is_none() {
                    failure.request_id = inner["request-id"].as_str().and_then(uuid);
                }
                error = inner;
            }
        }
        failure
    }
}
impl fmt::Display for RequestFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "operation={} endpoint={} HTTP {} code={} inner_code={} retry_after_seconds={} content_type={} claims_challenge={}",
            self.operation.label(),
            self.operation.endpoint(),
            self.status,
            self.code,
            self.inner_code,
            self.retry_after.as_secs(),
            self.content_type,
            self.has_claims_challenge
        )?;
        if let Some(id) = &self.request_id {
            write!(f, " request_id={id}")?;
        }
        if let Some(date) = self.server_date_unix {
            write!(f, " server_date_unix={date}")?;
        }
        Ok(())
    }
}
impl std::error::Error for RequestFailure {}

fn uuid(value: &str) -> Option<String> {
    (value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        }))
    .then(|| value.to_ascii_lowercase())
}
fn safe_code(value: Option<&str>) -> &'static str {
    match value {
        Some("accessDenied") => "accessDenied",
        Some("insufficient_claims") => "insufficient_claims",
        Some("InvalidAuthenticationToken") => "InvalidAuthenticationToken",
        Some("invalidRequest") => "invalidRequest",
        Some("itemNotFound") => "itemNotFound",
        Some("notSupported") => "notSupported",
        Some("nameAlreadyExists") => "nameAlreadyExists",
        Some("activityLimitReached") => "activityLimitReached",
        Some("quotaLimitReached") => "quotaLimitReached",
        Some("generalException") => "generalException",
        Some("resourceLocked") => "resourceLocked",
        Some("invalid_grant") => "invalid_grant",
        Some("invalid_client") => "invalid_client",
        Some("invalid_scope") => "invalid_scope",
        Some("invalid_request") => "invalid_request",
        Some("unauthorized_client") => "unauthorized_client",
        Some("interaction_required") => "interaction_required",
        Some("temporarily_unavailable") => "temporarily_unavailable",
        _ => "unknown",
    }
}

#[derive(Debug)]
pub(crate) struct TransportFailure {
    pub operation: RequestOperation,
    pub code: &'static str,
}
impl fmt::Display for TransportFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "operation={} endpoint={} code={}",
            self.operation.label(),
            self.operation.endpoint(),
            self.code
        )
    }
}
impl std::error::Error for TransportFailure {}
impl TransportFailure {
    pub(crate) fn request(error: &reqwest::Error, operation: RequestOperation) -> Self {
        Self {
            operation,
            code: if error.is_timeout() {
                "timeout"
            } else if error.is_connect() {
                "connect-or-tls"
            } else if error.is_redirect() {
                "redirect"
            } else {
                "transport"
            },
        }
    }
}

pub struct SafeError<'a>(&'a anyhow::Error);
pub fn safe_error(error: &anyhow::Error) -> SafeError<'_> {
    SafeError(error)
}
impl fmt::Display for SafeError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(error) = self.0.downcast_ref::<RequestFailure>() {
            error.fmt(f)
        } else if let Some(error) = self.0.downcast_ref::<TransportFailure>() {
            error.fmt(f)
        } else {
            f.write_str("operation=unknown code=local-or-provider-error")
        }
    }
}
