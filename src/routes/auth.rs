use actix_web::{post, get, web, HttpResponse, Responder, HttpRequest};
use sqlx::PgPool;
use crate::models::{SignupInput, LoginInput, User, UserResponse};
use uuid::Uuid;
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, TokenData, errors::Error as JwtError};
use serde::{Deserialize, Serialize};
use crate::config;

/// POST /signup
/// Creates a new user with the provided email, name, and password.
/// The paper_amount is set to 10,000 by default.
#[post("/signup")]
pub async fn signup(pool: web::Data<PgPool>, item: web::Json<SignupInput>) -> impl Responder {
    // Hash the password using bcrypt
    let hashed_password = match hash(&item.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return HttpResponse::InternalServerError().body("Error hashing password"),
    };

    // Insert the new user into the database.
    // The paper_amount is hard-coded to 10,000.
    let rec = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, name, hashed_password, paper_amount) VALUES ($1, $2, $3, $4) RETURNING *"
    )
    .bind(&item.email)
    .bind(&item.name)
    .bind(&hashed_password)
    .bind(10000_i32)
    .fetch_one(pool.get_ref())
    .await;

    match rec {
        Ok(user) => {
            let response = UserResponse {
                id: user.id,
                email: user.email,
                name: user.name,
                paper_amount: user.paper_amount,
            };
            HttpResponse::Ok().json(response)
        },
        Err(e) => {
            eprintln!("Error inserting user: {:?}", e);
            HttpResponse::InternalServerError().body("Error inserting user")
        }
    }
}

/// A simple structure to return the JWT token.
#[derive(Serialize)]
struct TokenResponse {
    token: String,
}

/// JWT claims structure.
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // subject (user id)
    exp: usize,  // expiration timestamp
}

/// POST /login
/// Validates user credentials and returns a JWT token upon success.
#[post("/login")]
pub async fn login(pool: web::Data<PgPool>, item: web::Json<LoginInput>) -> impl Responder {
    // Retrieve the user from the database by email.
    let user_result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&item.email)
        .fetch_one(pool.get_ref())
        .await;

    let user = match user_result {
        Ok(user) => user,
        Err(_) => return HttpResponse::BadRequest().body("Invalid email or password"),
    };

    // Verify the provided password against the stored hashed password.
    let valid = match verify(&item.password, &user.hashed_password) {
        Ok(valid) => valid,
        Err(_) => return HttpResponse::InternalServerError().body("Error verifying password"),
    };

    if !valid {
        return HttpResponse::BadRequest().body("Invalid email or password");
    }

    // Prepare JWT claims – here we set expiration 24 hours from now.
    let jwt_secret = config::jwt_secret();
    let my_claims = Claims {
        sub: user.id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };

    // Generate the token
    let token = match encode(&Header::default(), &my_claims, &EncodingKey::from_secret(jwt_secret.as_ref())) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().body("Error generating token"),
    };

    HttpResponse::Ok().json(TokenResponse { token })
}

/// GET /get-user
/// Validates the JWT token from the `Authorization` header and returns the user info.
#[get("/get-user")]
pub async fn get_user(pool: web::Data<PgPool>, req: HttpRequest) -> impl Responder {
    // Extract token from Authorization header
    let auth_header = req.headers().get("Authorization");
    if auth_header.is_none() {
        return HttpResponse::Unauthorized().body("Missing Authorization header");
    }
    let auth_str = auth_header.unwrap().to_str().unwrap_or("");
    if !auth_str.starts_with("Bearer ") {
        return HttpResponse::Unauthorized().body("Invalid Authorization header");
    }
    let token = auth_str.trim_start_matches("Bearer ").trim();

    // Decode the token to retrieve claims
    let jwt_secret = config::jwt_secret();
    let token_data: Result<TokenData<Claims>, JwtError> = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    );

    let claims = match token_data {
        Ok(data) => data.claims,
        Err(_) => return HttpResponse::Unauthorized().body("Invalid token"),
    };

    // Parse the user ID from the token's subject claim.
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(uid) => uid,
        Err(_) => return HttpResponse::Unauthorized().body("Invalid token data"),
    };

    // Fetch the user from the database using the parsed user ID.
    let user_result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool.get_ref())
        .await;

    match user_result {
        Ok(user) => {
            let response = UserResponse {
                id: user.id,
                email: user.email,
                name: user.name,
                paper_amount: user.paper_amount,
            };
            HttpResponse::Ok().json(response)
        },
        Err(_) => HttpResponse::InternalServerError().body("Error fetching user"),
    }
}
