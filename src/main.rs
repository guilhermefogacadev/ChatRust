use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::net::TcpStream;
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};
use std::error::Error;
use std::fmt;

/// Cria um erro específico para o caso de JWT
#[derive(Debug)]
enum JwtError {
    JwtEncodeError(jsonwebtoken::errors::Error),
    JwtDecodeError(jsonwebtoken::errors::Error),
}

impl fmt::Display for JwtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for JwtError {}

impl From<jsonwebtoken::errors::Error> for JwtError {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        JwtError::JwtDecodeError(error)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    message: String,
    exp: usize,  // Adiciona o campo de expiração
}

const SECRET_KEY: &[u8] = b"your_secret_key"; // Substitua por uma chave segura

/// Criptografa uma mensagem usando JWT
fn encrypt_message(message: &str) -> Result<String, JwtError> {
    let expiration = Utc::now() + Duration::minutes(30); // Token expira em 30 minutos
    let claims = Claims {
        message: message.to_string(),
        exp: expiration.timestamp() as usize,
    };
    
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET_KEY))?;
    Ok(token)
}

/// Descriptografa uma mensagem JWT
fn decrypt_message(token: &str) -> Result<String, JwtError> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(SECRET_KEY), &validation)?;
    Ok(token_data.claims.message)
}

#[tokio::main]
async fn main() {
    // Cria um canal de broadcast para mensagens.
    let (tx, _) = broadcast::channel(100);
    let tx = Arc::new(tx);

    // Configura o endereço do servidor.
    let addr = "127.0.0.1:8080".to_string();
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");

    println!("Server running on {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let tx = Arc::clone(&tx);
        tokio::spawn(handle_connection(stream, tx));
    }
}

async fn handle_connection(stream: TcpStream, tx: Arc<broadcast::Sender<String>>) {
    let ws_stream = accept_async(stream)
        .await
        .expect("Error during WebSocket handshake");

    println!("WebSocket connection established");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut rx = tx.subscribe();

    // Processa mensagens recebidas do WebSocket
    let receiver_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Ok(text) = msg.to_text() {
                // Criptografa a mensagem antes de enviar para o broadcast
                if let Ok(encrypted) = encrypt_message(text) {
                    println!("Original Message: {}", text);
                    println!("Encrypted Message: {}", encrypted);
                    let _ = tx.send(encrypted);
                } else {
                    eprintln!("Failed to encrypt message");
                }
            }
        }
    });

    // Processa mensagens recebidas do broadcast
    let sender_task = tokio::spawn(async move {
        while let Ok(encrypted_msg) = rx.recv().await {
            // Descriptografa a mensagem antes de enviar para o WebSocket
            if let Ok(decrypted) = decrypt_message(&encrypted_msg) {
                println!("Encrypted Message: {}", encrypted_msg);
                println!("Decrypted Message: {}", decrypted);
                let _ = ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(decrypted)).await;
            } else {
                eprintln!("Failed to decrypt message");
            }
        }
    });

    receiver_task.await.expect("Receiver task failed");
    sender_task.await.expect("Sender task failed");
}
