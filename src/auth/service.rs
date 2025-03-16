use mongodb::bson::oid::ObjectId;
use validator::Validate;

use crate::{
    auth::{
        model::{AuthResponse, LoginUserRequest, RegisterUserRequest, User, UserResponse},
        repository::AuthRepository,
    },
    config::Config,
    error::AppError,
    utils::{hash, jwt},
};

#[derive(Clone)]
pub struct AuthService {
    repository: AuthRepository,
    config: Config,
}

impl AuthService {
    pub fn new(repository: AuthRepository, config: Config) -> Self {
        Self { repository, config }
    }
    
    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub async fn register(&self, req: RegisterUserRequest) -> Result<UserResponse, AppError> {
        // Validate the input
        req.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

        // Hash the password
        let hashed_password = hash::hash_password(&req.password)?;

        // Create new user
        let user = User::new(
            req.email.clone(),
            hashed_password,
            req.first_name.clone(),
            req.last_name.clone(),
        );

        // Save user to database
        let created_user = self.repository.create_user(user).await?;

        // Return user data without sensitive information
        Ok(UserResponse::from(created_user))
    }

    pub async fn login(&self, req: LoginUserRequest) -> Result<AuthResponse, AppError> {
        // Validate the input
        req.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

        // Find user by email
        let user = self
            .repository
            .find_user_by_email(&req.email)
            .await?
            .ok_or_else(|| AppError::AuthError("Invalid email or password".into()))?;

        // Verify password
        let is_valid = hash::verify_password(&req.password, &user.password)?;
        if !is_valid {
            return Err(AppError::AuthError("Invalid email or password".into()));
        }

        // Generate JWT tokens
        let access_token = jwt::generate_jwt(&user, &self.config.jwt_secret, self.config.jwt_expires_in)?;
        let refresh_token = jwt::generate_jwt(
            &user,
            &self.config.jwt_secret,
            self.config.jwt_refresh_expires_in,
        )?;

        // Create response
        Ok(AuthResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".into(),
            expires_in: self.config.jwt_expires_in.num_seconds(),
            user: user.into(),
        })
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<UserResponse, AppError> {
        // Convert string ID to ObjectId
        let object_id = ObjectId::parse_str(id)
            .map_err(|_| AppError::ValidationError("Invalid user ID format".into()))?;

        // Find user by ID
        let user = self.repository.find_user_by_id(&object_id).await?;

        // Return user data
        Ok(user.into())
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<AuthResponse, AppError> {
        // Verify refresh token
        let claims = jwt::verify_jwt(refresh_token, &self.config.jwt_secret)?;

        // Find user by ID
        let object_id = ObjectId::parse_str(&claims.sub)
            .map_err(|_| AppError::AuthError("Invalid user ID in token".into()))?;
            
        let user = self.repository.find_user_by_id(&object_id).await?;

        // Generate new tokens
        let access_token = jwt::generate_jwt(&user, &self.config.jwt_secret, self.config.jwt_expires_in)?;
        let new_refresh_token = jwt::generate_jwt(
            &user,
            &self.config.jwt_secret,
            self.config.jwt_refresh_expires_in,
        )?;

        // Create response
        Ok(AuthResponse {
            access_token,
            refresh_token: new_refresh_token,
            token_type: "Bearer".into(),
            expires_in: self.config.jwt_expires_in.num_seconds(),
            user: user.into(),
        })
    }
}