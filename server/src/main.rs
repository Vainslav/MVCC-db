use axum::{
    Router,
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use db::{
    Command, CommandExecutionError, Connection, IsolationLevel, Storage, TransactionManager,
    execute_command,
};
use futures::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex, RwLock};
use tracing::{error, info};

struct AppState {
    store: Arc<RwLock<Storage>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let store = Arc::new(RwLock::new(Storage::new()));
    let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

    let state = Arc::new(AppState { store, tx_manager });

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    info!("Сервер запущен на http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let conn = Arc::new(Mutex::new(Connection::new(
        state.store.clone(),
        state.tx_manager.clone(),
    )));

    let (mut sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        if let axum::extract::ws::Message::Text(text) = msg {
            let conn_clone = conn.clone();
            let result = tokio::task::spawn_blocking(move || {
                let mut guard = conn_clone.lock().unwrap();
                process_command(&mut *guard, &text)
            })
            .await;

            match result {
                Ok(response) => {
                    if let Err(e) = sender
                        .send(axum::extract::ws::Message::Text(response.into()))
                        .await
                    {
                        error!("Ошибка отправки: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Ошибка в spawn_blocking: {}", e);
                    let _ = sender
                        .send(axum::extract::ws::Message::Text("ERROR: internal".into()))
                        .await;
                    break;
                }
            }
        }
    }
}

fn process_command(conn: &mut Connection, text: &str) -> String {
    let parts: Vec<&str> = text.trim().split_whitespace().collect();
    if parts.is_empty() {
        return "ERROR: пустая команда".to_string();
    }

    match parts[0].to_lowercase().as_str() {
        "begin" => {
            let isolation = if parts.len() > 1 {
                match parts[1].to_lowercase().as_str() {
                    "read_uncommitted" => IsolationLevel::ReadUncommitted,
                    "read_committed" => IsolationLevel::ReadCommitted,
                    "repeatable_read" => IsolationLevel::RepeatableRead,
                    "serializable" => IsolationLevel::Serializable,
                    _ => return format!("ERROR: неизвестный уровень '{}'", parts[1]),
                }
            } else {
                IsolationLevel::ReadCommitted
            };
            match execute_command(conn, Command::Begin(isolation)) {
                Ok(_) => "OK".to_string(),
                Err(e) => format!("ERROR: {:?}", e),
            }
        }
        "commit" => match execute_command(conn, Command::Commit) {
            Ok(_) => "OK".to_string(),
            Err(e) => format!("ERROR: {:?}", e),
        },
        "abort" => match execute_command(conn, Command::Abort) {
            Ok(_) => "OK".to_string(),
            Err(e) => format!("ERROR: {:?}", e),
        },
        "put" => {
            if parts.len() < 3 {
                return "ERROR: put <key> <value>".to_string();
            }
            let key = parts[1].to_string();
            let value = parts[2..].join(" ");
            match execute_command(conn, Command::Put(key, value)) {
                Ok(_) => "OK".to_string(),
                Err(e) => format!("ERROR: {:?}", e),
            }
        }
        "get" => {
            if parts.len() < 2 {
                return "ERROR: get <key>".to_string();
            }
            let key = parts[1].to_string();
            match execute_command(conn, Command::Get(key)) {
                Ok(val) => format!("VALUE: {}", val),
                Err(CommandExecutionError::NotFound) => "NOT_FOUND".to_string(),
                Err(CommandExecutionError::NoneVisible) => "NONE_VISIBLE".to_string(),
                Err(e) => format!("ERROR: {:?}", e),
            }
        }
        "delete" => {
            if parts.len() < 2 {
                return "ERROR: delete <key>".to_string();
            }
            let key = parts[1].to_string();
            match execute_command(conn, Command::Delete(key)) {
                Ok(_) => "OK".to_string(),
                Err(CommandExecutionError::NotFound) => "NOT_FOUND".to_string(),
                Err(CommandExecutionError::NoneVisible) => "NONE_VISIBLE".to_string(),
                Err(e) => format!("ERROR: {:?}", e),
            }
        }
        _ => format!("ERROR: неизвестная команда '{}'", parts[0]),
    }
}
