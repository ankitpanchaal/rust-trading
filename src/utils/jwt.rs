use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use crate::{auth::model::{TokenClaims, User, UserRole}, error::AppError};

pub fn generate_jwt(
    user: &User,
    jwt_secret: &str,
    expiration: Duration,
) -> Result<String, AppError> {
    let user_id = match user.id {
        Some(id) => id.to_hex(),
        None => return Err(AppError::AuthError("User ID not found".into())),
    };

    let role = match user.role {
        UserRole::Admin => "admin",
        UserRole::User => "user",
    };

    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + expiration).timestamp() as usize;

    let claims = TokenClaims {
        sub: user_id,
        email: user.email.clone(),
        role: role.to_string(),
        exp,
        iat,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::AuthError(format!("Failed to generate token: {}", e)))
}

pub fn verify_jwt(token: &str, jwt_secret: &str) -> Result<TokenClaims, AppError> {
    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| AppError::AuthError(format!("Invalid token: {}", e)))?;

    Ok(token_data.claims)
}