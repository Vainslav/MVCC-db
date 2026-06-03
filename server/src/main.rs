use axum::{Router, routing::get};
use db::{Storage, TransactionManager};
use std::sync::{Arc, RwLock};
use tracing::info;

mod socket;

pub struct AppState {
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
        .route("/ws", get(socket::websocket_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    info!("Server is running: http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
