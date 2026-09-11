use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};

const API_BASE_URL: &str = "http://localhost:3000";

#[derive(Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    pswd: &'a str,
}

#[derive(Deserialize)]
struct LoginResponse {
    access_token: String,
}

#[derive(Serialize)]
struct CreateUser<'a> {
    username: &'a str,
    email: &'a str,
    pswd: &'a str,
}

/// The claims carried by the access token, decoded client-side purely for
/// display (the backend is the one actually verifying the signature on
/// every protected request).
#[derive(Debug, Clone, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub username: String,
    pub is_admin: bool,
}

pub fn decode_claims(token: &str) -> Option<Claims> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}

#[derive(Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

async fn send<T: serde::de::DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> Result<ApiResponse<T>, String> {
    // The refresh-token cookie is cross-origin, so it must be explicitly
    // opted into on the fetch backend (native targets rely on the client's
    // cookie store instead and don't have this method).
    #[cfg(target_arch = "wasm32")]
    let request = request.fetch_credentials_include();

    let response = request
        .send()
        .await
        .map_err(|_| "Unable to reach the server".to_string())?;

    response
        .json::<ApiResponse<T>>()
        .await
        .map_err(|_| "Invalid response from the server".to_string())
}

pub async fn login(username: &str, pswd: &str) -> Result<String, String> {
    let request = reqwest::Client::new()
        .post(format!("{API_BASE_URL}/auth/login"))
        .json(&LoginRequest { username, pswd });

    let parsed: ApiResponse<LoginResponse> = send(request).await?;

    if parsed.success {
        parsed
            .data
            .map(|d| d.access_token)
            .ok_or_else(|| "Invalid response from the server".to_string())
    } else {
        Err(parsed.message.unwrap_or_else(|| "Invalid credentials".to_string()))
    }
}

pub async fn register(username: &str, email: &str, pswd: &str) -> Result<String, String> {
    let request = reqwest::Client::new()
        .post(format!("{API_BASE_URL}/auth/register"))
        .json(&CreateUser { username, email, pswd });

    let parsed: ApiResponse<()> = send(request).await?;

    if parsed.success {
        Ok(parsed.message.unwrap_or_else(|| "Account created successfully.".to_string()))
    } else {
        Err(parsed.message.unwrap_or_else(|| "Unable to create the account".to_string()))
    }
}

pub async fn refresh() -> Result<String, String> {
    let request = reqwest::Client::new().post(format!("{API_BASE_URL}/auth/refresh"));

    let parsed: ApiResponse<LoginResponse> = send(request).await?;

    if parsed.success {
        parsed
            .data
            .map(|d| d.access_token)
            .ok_or_else(|| "Invalid response from the server".to_string())
    } else {
        Err(parsed.message.unwrap_or_else(|| "Unable to restore the session".to_string()))
    }
}

pub async fn logout() -> Result<(), String> {
    let request = reqwest::Client::new().post(format!("{API_BASE_URL}/auth/logout"));

    let parsed: ApiResponse<()> = send(request).await?;

    if parsed.success {
        Ok(())
    } else {
        Err(parsed.message.unwrap_or_else(|| "Unable to log out".to_string()))
    }
}

pub async fn get_user(id: i64, token: &str) -> Result<User, String> {
    let request = reqwest::Client::new()
        .get(format!("{API_BASE_URL}/users/{id}"))
        .bearer_auth(token);

    let parsed: ApiResponse<User> = send(request).await?;

    if parsed.success {
        parsed
            .data
            .ok_or_else(|| "Invalid response from the server".to_string())
    } else {
        Err(parsed.message.unwrap_or_else(|| "Unable to fetch the user".to_string()))
    }
}