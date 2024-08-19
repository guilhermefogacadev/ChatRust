
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use serde::{Serialize, Deserialize};
use std::error::Error;
use chrono::{Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub message: String,
    pub exp: usize,  // Adiciona o campo de expiração
}

const SECRET_KEY: &[u8] = b"your_secret_key"; // Substitua por uma chave segura

/// Criptografa uma mensagem usando JWT
pub fn encrypt_message(message: &str) -> Result<String, Box<dyn Error>> {
    let expiration = Utc::now() + Duration::minutes(30); // Token expira em 30 minutos
    let claims = Claims {
        message: message.to_string(),
        exp: expiration.timestamp() as usize,
    };
    
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET_KEY))?;
    Ok(token)
}

/// Descriptografa uma mensagem JWT
pub fn decrypt_message(token: &str) -> Result<String, Box<dyn Error>> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(SECRET_KEY), &validation)?;
    Ok(token_data.claims.message)
}
