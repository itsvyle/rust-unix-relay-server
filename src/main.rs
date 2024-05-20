use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{broadcast, Mutex};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let path = "/var/relay_socket";
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;

    // Create a broadcast channel with a buffer size of 16.
    let (tx, _rx) = broadcast::channel(16);
    let clients = Arc::new(Mutex::new(Vec::new()));

    loop {
        match listener.accept().await {
            Ok((socket, _)) => {
                let tx = tx.clone();
                let mut rx = tx.subscribe();
                let clients = clients.clone();

                tokio::spawn(async move {
                    handle_connection(socket, tx, &mut rx, clients).await;
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {:?}", e);
            }
        }
    }
}

async fn handle_connection(
    mut socket: UnixStream,
    tx: broadcast::Sender<(Uuid, Vec<u8>)>,
    rx: &mut broadcast::Receiver<(Uuid, Vec<u8>)>,
    clients: Arc<Mutex<Vec<Uuid>>>,
) {
    let client_id = Uuid::new_v4();
    clients.lock().await.push(client_id);
    eprintln!("Client {} connected", client_id);

    let mut buffer = vec![0; 1024];


    loop {
        tokio::select! {
            result = socket.read(&mut buffer) => {
                match result {
                    Ok(0) => {
                        break; // connection has been closed
                    }
                    Ok(n) => {
                        let msg = buffer[..n].to_vec();
                        if tx.send((client_id, msg)).is_err() {
                            eprintln!("There are no active receivers; dropping message")
                        }
                        buffer.clear();
                    }
                    Err(e) => {
                        eprintln!("Failed to read from socket; err = {:?}", e);
                        break;
                    }
                }
            }
            result = rx.recv() => {
                match result {
                    Ok((sender_id, msg)) => {
                        if sender_id != client_id {
                            if let Err(e) = socket.write_all(&msg).await {
                                eprintln!("Failed to write to socket; err = {:?}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to receive message; err = {:?}", e);
                        break;
                    }
                }
            }
        }
    }

    // Remove the client from the list
    clients.lock().await.retain(|&id| id != client_id);
    eprintln!("Client {} disconnected", client_id);
}
