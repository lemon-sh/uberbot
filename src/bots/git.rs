use serde_json::Value::Null;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use warp::Filter;

#[derive(Clone)]
struct Tx {
    tx: Sender<String>,
}

impl Tx {
    fn new(tx: Sender<String>) -> Self {
        Tx { tx }
    }
}

async fn handle_post(json: serde_json::Value, tx: Tx) -> Result<impl warp::Reply, warp::Rejection> {
    if json["commits"] != Null {
        let commits = json["commits"].as_array().unwrap();
        let repo = &json["repository"]["full_name"].as_str().unwrap().trim();
        if commits.len() != 1 {
            tx.tx
                .send(format!("{} new commits on {}:", commits.len(), repo))
                .await
                .expect("Failed to send string to main thread");
            for commit in commits {
                let author = &commit["author"]["name"].as_str().unwrap().trim();
                let message = &commit["message"].as_str().unwrap().trim();
                tx.tx
                    .send(format!("{} - {}", author, message))
                    .await
                    .expect("Failed to send string to main thread");
            }
        } else {
            let author = &json["commits"][0]["author"]["name"]
                .as_str()
                .unwrap()
                .trim();
            let message = &json["commits"][0]["message"].as_str().unwrap().trim();
            tx.tx
                .send(format!("New commit on {}: {} - {}", repo, message, author))
                .await
                .expect("Failed to send string to main thread");
        }
    }

    Ok(warp::reply::with_status("Ok", warp::http::StatusCode::OK))
}

pub async fn run(tx: Sender<String>, listen: SocketAddr) -> Result<(), tokio::io::Error> {
    let tx = Tx::new(tx);
    let tx_filter = warp::any().map(move || tx.clone());

    let filter = warp::post()
        .and(warp::body::json())
        .and(tx_filter)
        .and_then(handle_post);

    warp::serve(filter).run(listen).await;

    Ok(())
}
