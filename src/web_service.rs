use crate::ExecutorConnection;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use warp::Filter;

pub async fn run(
    db: ExecutorConnection,
    tx: Sender<String>,
    listen: SocketAddr,
) -> anyhow::Result<()> {
    let db_filter = warp::any().map(move || db.clone());
    let db_filter = warp::any().and(db_filter).and_then(handle);

    let tx_filter = warp::any().map(move || tx.clone());
    let tx_filter = warp::path("webhook")
        .and(warp::post())
        .and(warp::body::json())
        .and(tx_filter)
        .and_then(crate::bots::git::handle_post);

    let filter = db_filter.or(tx_filter);
    warp::serve(filter).run(listen).await;
    Ok(())
}

async fn handle(db: ExecutorConnection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some((a, b)) = db.get_quote(None).await {
        Ok(warp::reply::with_status(
            format!("{} {}", a, b),
            warp::http::StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            format!("None"),
            warp::http::StatusCode::NO_CONTENT,
        ))
    }
}
