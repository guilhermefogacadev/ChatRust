// server.rs
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::SinkExt;
use futures_util::StreamExt;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.expect("Failed to bind address");

    let clients = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let clients = clients.clone();
        tokio::spawn(handle_connection(stream, clients));
    }
}

async fn handle_connection(stream: tokio::net::TcpStream, clients: Arc<Mutex<HashMap<String, tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>>>) {
    let ws_stream = accept_async(stream)
        .await
        .expect("Error during WebSocket handshake");

    let (mut write, mut read) = ws_stream.split();
    let mut name = None;

    while let Some(message) = read.next().await {
        match message {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                if name.is_none() {
                    name = Some(text.clone());
                    let mut clients = clients.lock().await;
                    clients.insert(text.clone(), write.clone());
                } else {
                    let clients = clients.lock().await;
                    for (client_name, client) in clients.iter() {
                        if let Err(e) = client.send(tokio_tungstenite::tungstenite::Message::Text(format!("{}: {}", name.as_ref().unwrap(), text))).await {
                            eprintln!("Error sending message: {:?}", e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(name) = name {
        let mut clients = clients.lock().await;
        clients.remove(&name);
    }
}
