use crate::ExecutorConnection;
use std::net::SocketAddr;
use warp::Filter;

pub async fn run(db: ExecutorConnection, listen: SocketAddr) -> anyhow::Result<()> {
    let db_filter = warp::any().map(move || db.clone());
    let filter = warp::any().and(db_filter).and_then(handle);

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
