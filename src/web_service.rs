use crate::ExecutorConnection;
use irc::client::Client;
use reqwest::StatusCode;
use serde_json::Value::Null;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;
use warp::{reply, Filter, Reply};

pub async fn run(
    db: ExecutorConnection,
    wh_irc: Arc<Client>,
    wh_channel: String,
    listen: SocketAddr,
    mut cancel: Receiver<()>
) {
    let quote_get = warp::path("quotes")
        .and(warp::get())
        .and(warp::any().map(move || db.clone()))
        .map(handle_get_quote);

    let webhook_post = warp::path("webhook")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::any().map(move || wh_irc.clone()))
        .and(warp::any().map(move || wh_channel.clone()))
        .map(handle_webhook);

    let filter = quote_get.or(webhook_post);
    warp::serve(filter).bind_with_graceful_shutdown(listen, async move {
        let _ = cancel.recv().await;
    }).1.await;
    tracing::info!("Web service finished");
}

fn handle_get_quote(_: ExecutorConnection) -> impl Reply {
    reply::html(include_str!("res/quote_tmpl.html"))
}

#[allow(clippy::needless_pass_by_value)]
fn handle_webhook(json: serde_json::Value, irc: Arc<Client>, channel: String) -> impl Reply {
    if json["commits"] != Null {
        let commits = json["commits"].as_array().unwrap();
        let repo = &json["repository"]["full_name"].as_str().unwrap().trim();
        if commits.len() == 1 {
            let author = &json["commits"][0]["author"]["name"]
                .as_str()
                .unwrap()
                .trim();
            let message = &json["commits"][0]["message"].as_str().unwrap().trim();
            if let Err(e) = irc.send_privmsg(
                channel,
                format!("New commit on {}: {} - {}", repo, message, author),
            ) {
                return reply::with_status(
                    format!("An error has occurred: {}", e),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
                .into_response();
            }
        } else {
            if let Err(e) = irc.send_privmsg(
                channel.clone(),
                format!("{} new commits on {}:", commits.len(), repo),
            ) {
                return reply::with_status(
                    format!("An error has occurred: {}", e),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
                .into_response();
            }
            for commit in commits {
                let author = &commit["author"]["name"].as_str().unwrap().trim();
                let message = &commit["message"].as_str().unwrap().trim();
                if let Err(e) =
                    irc.send_privmsg(channel.clone(), format!("{} - {}", author, message))
                {
                    return reply::with_status(
                        format!("An error has occurred: {}", e),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                    .into_response();
                }
            }
        }
    }
    StatusCode::CREATED.into_response()
}
