use std::{convert::Infallible, sync::Arc};

use hyper::{
    header::HeaderValue,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server, StatusCode, body::to_bytes,
};
use tokio::sync::broadcast;

use crate::config::HttpConfig;

mod parser;

pub struct HttpContext<SF>
where
    SF: Fn(String, String) -> anyhow::Result<()>,
{
    pub cfg: HttpConfig,
    pub sendmsg: SF,
}

async fn handle<SF>(ctx: Arc<HttpContext<SF>>, req: Request<Body>) -> anyhow::Result<Response<Body>>
where
    SF: Fn(String, String) -> anyhow::Result<()> + Send + Sync + 'static,
{
    let mime = req
        .headers()
        .get("Content-Type")
        .map(HeaderValue::to_str)
        .transpose()?;
    if let Some(mime) = mime {
        if mime != "application/json" {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("wrong content-type"))?);
        }
    } else {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("no content-type"))?);
    }
    let webhook = (&req.uri().path()[1..]).to_string();
    let channel = if let Some(c) = ctx.cfg.webhooks.get(&webhook) {
        c
    } else {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("webhook path not registered"))?);
    };
    let body_bytes = to_bytes(req.into_body()).await?;
    let body = String::from_utf8_lossy(&body_bytes);
    let response = parser::textify(&body, &webhook)?;
    (ctx.sendmsg)(channel.to_string(), response)?;
    let resp = Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())?;
    Ok(resp)
}

pub async fn run<SF>(
    context: HttpContext<SF>,
    mut shutdown: broadcast::Receiver<()>,
) -> hyper::Result<()>
where
    SF: Fn(String, String) -> anyhow::Result<()> + Send + Sync + 'static,
{
    let ctx = Arc::new(context);
    let make_service = make_service_fn({
        let ctx = ctx.clone();
        move |_conn| {
            let ctx = ctx.clone();
            let service = service_fn(move |req| handle(ctx.clone(), req));
            async move { Ok::<_, Infallible>(service) }
        }
    });

    let server = Server::bind(&ctx.cfg.listen).serve(make_service);
    server
        .with_graceful_shutdown(async {
            shutdown.recv().await.unwrap();
        })
        .await
}
