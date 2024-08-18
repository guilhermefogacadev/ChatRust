use base64::engine::general_purpose;
use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

type Clients = Arc<Mutex<HashMap<usize, tokio::sync::broadcast::Sender<String>>>>;

#[derive(Debug, Serialize, Deserialize)]
struct MediaMessage {
    r#type: String,
    data: String,
    filename: String,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Não conseguiu criar o listener");
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));
    let (tx, _rx) = broadcast::channel(100);

    println!("Servidor WebSocket escutando em 127.0.0.1:8080");

    while let Ok((stream, _)) = listener.accept().await {
        let clients = clients.clone();
        let tx = tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            let ws_stream = accept_async(stream)
                .await
                .expect("Falha ao aceitar a conexão WebSocket");
            println!("Nova conexão WebSocket");

            let (mut write, mut read) = ws_stream.split();

            // Recebe a primeira mensagem que pode ser qualquer mensagem
            if let Some(Ok(Message::Text(text))) = read.next().await {
                println!("Mensagem inicial recebida: {}", text);
            } else {
                println!("Falha na primeira mensagem de conexão");
                return;
            }

            let id = {
                let mut clients = clients.lock().unwrap();
                let id = clients.len() + 1;
                clients.insert(id, tx.clone());
                id
            };

            loop {
                tokio::select! {
                    msg = read.next() => {
                        if let Some(Ok(message)) = msg {
                            match message {
                                Message::Text(text) => {
                                    // Verifica se é uma mensagem de mídia
                                    if let Ok(media_message) = serde_json::from_str::<MediaMessage>(&text) {
                                        if media_message.r#type == "media" {
                                            let decoded_data = general_purpose::STANDARD.decode(&media_message.data).expect("Falha ao decodificar base64");
                                            // Salvar ou processar o arquivo conforme necessário
                                            // Aqui estamos apenas imprimindo o tamanho dos dados recebidos
                                            println!("Recebido arquivo: {} ({} bytes)", media_message.filename, decoded_data.len());
                                            continue;
                                        }
                                    }
                                    tx.send(text.clone()).unwrap();
                                },
                                _ => {},
                            }
                        } else {
                            break;
                        }
                    }
                    msg = rx.recv() => {
                        if let Ok(text) = msg {
                            write.send(Message::Text(text)).await.expect("Falha ao enviar mensagem");
                        }
                    }
                }
            }

            clients.lock().unwrap().remove(&id);
            println!("Conexão WebSocket fechada");
        });
    }
}
