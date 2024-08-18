use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::net::TcpStream;

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
                let _ = tx.send(text.to_string());
            }
        }
    });

    // Processa mensagens recebidas do broadcast
    let sender_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let _ = ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await;
        }
    });

    receiver_task.await.expect("Receiver task failed");
    sender_task.await.expect("Sender task failed");
}
