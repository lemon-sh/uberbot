use crate::database::Quote;
use crate::ExecutorConnection;
use irc::client::Client;
use lazy_static::lazy_static;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value::Null;
use std::net::SocketAddr;
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::broadcast::Receiver;
use warp::{reply, Filter, Reply};

lazy_static! {
    static ref TERA: Tera = Tera::new("templates/**/*").unwrap();
}

pub async fn run(
    db: ExecutorConnection,
    wh_irc: Arc<Client>,
    wh_channel: String,
    listen: SocketAddr,
    mut cancel: Receiver<()>,
) {
    let quote_get = warp::get()
        .and(warp::path("quotes"))
        .and(warp::query::<QuotesQuery>())
        .and(warp::any().map(move || db.clone()))
        .then(handle_get_quote);

    let webhook_post = warp::path("webhook")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::any().map(move || wh_irc.clone()))
        .and(warp::any().map(move || wh_channel.clone()))
        .map(handle_webhook);

    let routes = webhook_post.or(quote_get);
    warp::serve(routes)
        .bind_with_graceful_shutdown(listen, async move {
            let _ = cancel.recv().await;
        })
        .1
        .await;
    tracing::info!("Web service finished");
}

#[derive(Serialize)]
struct QuotesTemplate {
    quotes: Option<Vec<Quote>>,
    flash: Option<String>,
}

#[derive(Deserialize)]
struct QuotesQuery {
    q: Option<String>,
}

async fn handle_get_quote(query: QuotesQuery, db: ExecutorConnection) -> impl Reply {
    let template = if let Some(q) = query.q {
        if let Some(quotes) = db.search(q.clone()).await {
            let quotes_count = quotes.len();
            QuotesTemplate {
                quotes: Some(quotes),
                flash: Some(format!(
                    "Displaying {}/50 results for query \"{}\"",
                    quotes_count, q
                )),
            }
        } else {
            QuotesTemplate {
                quotes: None,
                flash: Some("A database error has occurred".into()),
            }
        }
    } else {
        QuotesTemplate {
            quotes: db.random20().await,
            flash: Some("Displaying up to 20 random quotes".into()),
        }
    };
    match TERA.render("quotes.html", &Context::from_serialize(&template).unwrap()) {
        Ok(o) => reply::html(o).into_response(),
        Err(e) => {
            tracing::warn!("Error while rendering template: {}", e);
            reply::with_status(
                "Failed to render template",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .into_response()
        }
    }
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
