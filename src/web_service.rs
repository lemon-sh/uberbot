use crate::ExecutorConnection;
use serde_json::Value::Null;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use warp::Filter;

pub async fn run(
    db: ExecutorConnection,
    tx: Sender<String>,
    listen: SocketAddr,
) -> anyhow::Result<()> {
    let db_filter = warp::any().map(move || db.clone());
    let db_filter = warp::get().and(db_filter).and_then(handle);

    let tx_filter = warp::any().map(move || tx.clone());
    let tx_filter = warp::path("webhook")
        .and(warp::post())
        .and(warp::body::json())
        .and(tx_filter)
        .and_then(handle_webhook);

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
            "None".into(),
            warp::http::StatusCode::NO_CONTENT,
        ))
    }
}

pub async fn handle_webhook(
    json: serde_json::Value,
    tx: Sender<String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if json["commits"] != Null {
        let commits = json["commits"].as_array().unwrap();
        let repo = &json["repository"]["full_name"].as_str().unwrap().trim();
        if commits.len() != 1 {
            tx.send(format!("{} new commits on {}:", commits.len(), repo))
                .await
                .expect("Failed to send string to main thread");
            for commit in commits {
                let author = &commit["author"]["name"].as_str().unwrap().trim();
                let message = &commit["message"].as_str().unwrap().trim();
                tx.send(format!("{} - {}", author, message))
                    .await
                    .expect("Failed to send string to main thread");
            }
        } else {
            let author = &json["commits"][0]["author"]["name"]
                .as_str()
                .unwrap()
                .trim();
            let message = &json["commits"][0]["message"].as_str().unwrap().trim();
            tx.send(format!("New commit on {}: {} - {}", repo, message, author))
                .await
                .expect("Failed to send string to main thread");
        }
    }

    Ok(warp::reply::with_status("Ok", warp::http::StatusCode::OK))
}
