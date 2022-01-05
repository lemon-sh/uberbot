use std::net::SocketAddr;
use crate::ExecutorConnection;
use std::convert::Infallible;
use std::sync::Arc;
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};

pub async fn run(db: ExecutorConnection, listen: SocketAddr) -> anyhow::Result<()> {
    let db = Arc::new(db);

    Server::bind(&listen).serve(make_service_fn(|_| {
        let db = Arc::clone(&db);
        async move {
            Ok::<_, Infallible>(service_fn(move |r| handle(r, Arc::clone(&db))))
        }
    })).await?;

    Ok(())
}

async fn handle(req: Request<Body>, db: Arc<ExecutorConnection>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from(format!("{:?}", db.get_quote(None).await))))
}
