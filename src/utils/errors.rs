/// Application-level error type for API handlers.
///
/// `AppError` provides a single error type that all API handlers return.
/// It converts cleanly to HTTP responses, eliminating the `match auth::... { Err(e) => return Ok(e.into_response()) }`
/// boilerplate. Handlers that return `Result<Response, AppError>` can use `?` throughout.
///
/// # Usage in handlers
///
/// ```rust
/// pub async fn handle_example(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
///     let user_ctx = auth::authenticate_request(&req, &ctx).await?;
///     // ...
///     Ok(Response::from_json(&data)?)
/// }
/// ```
///
/// The router calls `handler(...).await.unwrap_or_else(|e| e.into_response())` to convert
/// `AppError` back into a `worker::Response`.
use crate::auth::middleware::AuthError;
use worker::Response;

/// Unified error type for all API handler layers.
#[derive(Debug)]
pub enum AppError {
    /// 401 Unauthorized — missing or invalid credentials
    Unauthorized(String),
    /// 403 Forbidden — authenticated but not allowed
    Forbidden(String),
    /// 400 Bad Request — invalid input
    BadRequest(String),
    /// 404 Not Found
    NotFound(String),
    /// 409 Conflict — duplicate or state conflict
    Conflict(String),
    /// 500 Internal Server Error
    Internal(String),
    /// 403 with tier upgrade message
    TierLimitReached(String),
    /// 429 Too Many Requests — rate limit or duplicate submission
    TooManyRequests(String),
}

impl AppError {
    /// Convert into an HTTP `Response`. Always succeeds.
    pub fn into_response(self) -> Response {
        let (msg, status) = match self {
            AppError::Unauthorized(m) => (m, 401u16),
            AppError::Forbidden(m) => (m, 403),
            AppError::BadRequest(m) => (m, 400),
            AppError::NotFound(m) => (m, 404),
            AppError::Conflict(m) => (m, 409),
            AppError::Internal(m) => (m, 500),
            AppError::TierLimitReached(m) => (m, 403),
            AppError::TooManyRequests(m) => (m, 429),
        };
        Response::error(msg, status).unwrap_or_else(|_| Response::error("Error", status).unwrap())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            AppError::Unauthorized(m)
            | AppError::Forbidden(m)
            | AppError::BadRequest(m)
            | AppError::NotFound(m)
            | AppError::Conflict(m)
            | AppError::Internal(m)
            | AppError::TierLimitReached(m)
            | AppError::TooManyRequests(m) => m.as_str(),
        };
        write!(f, "{}", msg)
    }
}

impl From<AuthError> for AppError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::Unauthorized(m) => AppError::Unauthorized(m),
            AuthError::Forbidden(m) => AppError::Forbidden(m),
            AuthError::InternalError(m) => AppError::Internal(m),
        }
    }
}

impl From<worker::Error> for AppError {
    fn from(e: worker::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}
