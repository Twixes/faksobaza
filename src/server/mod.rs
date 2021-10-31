use crate::config;
use crate::sql::parse_statement;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use log::*;
use std::collections::HashMap;
use std::{convert, net, str::FromStr};
use tokio::time;
use ulid::Ulid;

async fn echo(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let timer = time::Instant::now();
    let request_id = Ulid::new();
    debug!("⚡️ Received request ID {}", request_id);
    let result = match (req.uri().path(), req.method()) {
        ("/", &Method::POST) => {
            // Read-write
            let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
            let query = String::from_utf8(body_bytes.into_iter().collect()).unwrap();
            // Found SQL
            let statement = parse_statement(&query);
            Ok(Response::new(Body::from(query)))
        }
        ("/", &Method::GET) => {
            // Read-only
            if let Some(query_string) = req.uri().query() {
                if let Ok(query_map) =
                    serde_urlencoded::from_str::<HashMap<String, String>>(query_string)
                {
                    if let Some(query) = query_map.get("query") {
                        // Found SQL
                        Ok(Response::new(Body::from(query.to_string())))
                        // TODO: Add statement handling
                    } else {
                        // No query param
                        Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::default())
                            .unwrap())
                    }
                } else {
                    // Bad query string
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::default())
                        .unwrap())
                }
            } else {
                // No query string
                Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::default())
                    .unwrap())
            }
        }
        ("/", _) => Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::default())
            .unwrap()),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::default())
            .unwrap()),
    };
    debug!(
        "🪃 Finished request ID {} in {} µs",
        request_id,
        timer.elapsed().as_micros()
    );
    result
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C signal handler");
    info!("💤 Shutting down gracefully");
}

/// Start server loop.
pub async fn start_server(config: &config::Config) {
    let tcp_listen_address = net::SocketAddr::new(
        net::IpAddr::from_str(&config.tcp_listen_host).unwrap(),
        config.tcp_listen_port,
    );

    let server = Server::bind(&tcp_listen_address)
        .serve(make_service_fn(|_conn| async {
            Ok::<_, convert::Infallible>(service_fn(echo))
        }))
        .with_graceful_shutdown(shutdown_signal());

    info!("👂 Listening on {}...", tcp_listen_address);

    if let Err(e) = server.await {
        error!("🛑 Encountered server error: {}", e);
    };
}