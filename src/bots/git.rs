use serde_json::Value::Null;
use tokio::sync::mpsc::Sender;

pub async fn handle_post(
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
