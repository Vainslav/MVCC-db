use axum::{Router, routing::any};
use db::{BackgroundWriter, DiskManager, Storage, TransactionManager, buffer_pool::ClockBufferPool};
use tempfile::tempdir;
use std::{sync::{Arc, RwLock}, time::Duration};
use tracing::info;

mod socket;

pub struct AppState {
    store: Arc<Storage>,
    tx_manager: Arc<RwLock<TransactionManager>>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();

    let dir = tempdir().unwrap();

    let index_path = dir.path().join("index.db");
    let data_path = dir.path().join("data.db");

    let disk_manager = Arc::new(DiskManager::init(vec![index_path.to_str().unwrap()], vec![data_path.to_str().unwrap()]));
    let cache = Arc::new(ClockBufferPool::new(64, disk_manager.clone()));
    let store = Arc::new(Storage::new(cache.clone()));
    let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

    let background_writer = Arc::new(BackgroundWriter::new(cache.clone(), disk_manager.clone()));
    let _handle = background_writer.start(Duration::from_millis(1000));

    let state = Arc::new(AppState { store, tx_manager });

    let app = Router::new()
        .route("/ws", any(socket::websocket_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    info!("Server is running: http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
