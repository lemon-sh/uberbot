use std::{convert::Infallible, sync::Arc};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server, StatusCode,
};
use tokio::sync::broadcast;

use crate::config::HttpConfig;

pub struct HttpContext<SF>
where
    SF: Fn(String, String) -> anyhow::Result<()>,
{
    pub cfg: HttpConfig,
    pub sendmsg: SF,
}

async fn handle<SF>(_ctx: Arc<HttpContext<SF>>, _req: Request<Body>) -> anyhow::Result<Response<Body>>
where
    SF: Fn(String, String) -> anyhow::Result<()> + Send + Sync + 'static,
{
    let resp = Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())?;
    Ok(resp)
}

pub async fn run<SF>(context: HttpContext<SF>, mut shutdown: broadcast::Receiver<()>) -> hyper::Result<()>
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
