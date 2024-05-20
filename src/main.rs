use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{broadcast, Mutex};
use tokio::signal::unix::{signal, SignalKind};
use std::sync::Arc;
use uuid::Uuid;
use std::env;
use std::os::unix::fs::PermissionsExt;

const MAX_MESSAGE_SIZE: usize = 1024; // in bytes

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let debug = cfg!(debug_assertions);
    let path: &str;
    let args: Vec<String> = env::args().collect();
    if debug && args.len() == 1 {
        path = "/tmp/relay_socket";
    } else if args.len() != 2 {
        eprintln!("Usage: {} <socket_path>", args[0]);
        std::process::exit(1);
    } else {
        path = &args[1][..];
    }
    let _ = std::fs::remove_file(path); // remove old one
    let listener = UnixListener::bind(path)?;
    eprintln!("Listening on: {:?} at socket path {}", listener.local_addr()?, path);
    // Give perms to anyone to connect
    if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o777)) {
        eprintln!("Failed to set permissions on socket file: {:?}", e);
    }
    

    // Create a broadcast channel with a buffer size of 16.
    let (tx, _rx) = broadcast::channel(16);
    let clients = Arc::new(Mutex::new(Vec::new()));


    // Spawn a task to handle the shutdown signal
    tokio::spawn(async  {
        handle_shutdown_signal().await;
    });

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
    eprintln!("Client {} connected, now have {} clients connected", client_id, clients.lock().await.len());

    let mut buffer = vec![0; MAX_MESSAGE_SIZE];


    loop {
        tokio::select! {
            result = socket.read(&mut buffer) => {
                match result {
                    Ok(0) => {
                        eprintln!("Client {} closed conn", client_id);
                        break; // connection has been closed
                    }
                    Ok(n) => {
                        if n >= MAX_MESSAGE_SIZE {
                            eprintln!("Received message exceeds maximum allowed size ({} bytes, client {})", n, client_id);
                            break;
                        }

                        let msg = buffer[..n].to_vec();
                        if tx.send((client_id, msg)).is_err() {
                            eprintln!("There are no active receivers; dropping message")
                        }
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

    clients.lock().await.retain(|&id| id != client_id);
    eprintln!("Client {} disconnected, now have {} clients connected", client_id, clients.lock().await.len());
}


async fn handle_shutdown_signal() {
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
    let mut sighup = signal(SignalKind::hangup()).expect("Failed to install SIGHUP handler");

    tokio::select! {
        _ = sigint.recv() => {
            eprintln!("Received SIGINT, shutting down.");
        }
        _ = sigterm.recv() => {
            eprintln!("Received SIGTERM, shutting down.");
        }
        _ = sighup.recv() => {
            eprintln!("Received SIGHUP, shutting down.");
        }
    }

    std::process::exit(0);
}
