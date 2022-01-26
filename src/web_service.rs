use crate::ExecutorConnection;
use serde_json::Value::Null;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use warp::{Filter, Reply, reply};

pub async fn run(
    db: ExecutorConnection,
    webhook_tx: Sender<String>,
    listen: SocketAddr,
) -> anyhow::Result<()> {
    let quote_get = warp::get()
        .and(warp::get())
        .and(warp::any().map(move || db.clone()))
        .then(handle_get_quote);

    let webhook_post = warp::path("webhook")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::any().map(move || webhook_tx.clone()))
        .then(handle_webhook);

    let filter = quote_get.or(webhook_post);
    warp::serve(filter).run(listen).await;
    Ok(())
}

async fn handle_get_quote(_: ExecutorConnection) -> impl Reply {
    reply::html(include_str!("res/quote_tmpl.html"))
}

async fn handle_webhook(
    json: serde_json::Value,
    tx: Sender<String>,
) -> impl Reply {
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
    warp::reply::with_status("Ok", warp::http::StatusCode::OK)
}
