use axum::{
    Json, Router,
    http::HeaderValue,
    response::{Html, IntoResponse},
};
use std::sync::LazyLock;
use tokio::net::TcpListener;

use crate::sysgetter::{SystemInfo, get_system_info_by_lines};

mod sysgetter;

const HELIOS_IMAGE: &[u8; 57693] = include_bytes!("../assets/helios.png");
const HELIOS_BANNER: &[u8; 38773] = include_bytes!("../assets/helios-img.png");
const HELIOS_BANNER_WEBP: &[u8; 35086] = include_bytes!("../assets/helios-img.webp");
const HELIOS_JS: &str = include_str!("../assets/scriptlet.js");
const HELIOS_CSS: &str = include_str!("../assets/style.css");
const HELIOS_HTML: &str = include_str!("../assets/index.html");

static FIRST_TIME_DATA: LazyLock<SystemInfo> = LazyLock::new(|| get_system_info_by_lines());

#[tokio::main]
async fn main() {
    let port_at = std::env::var("PORT").unwrap_or_else(|_| "7889".to_string());

    let app: Router = Router::new()
        .route("/", axum::routing::get(root))
        .route("/assets/helios.png", axum::routing::get(helios_image))
        .route(
            "/assets/helios-img.png",
            axum::routing::get(helios_image_banner),
        )
        .route(
            "/assets/helios-img.webp",
            axum::routing::get(helios_image_banner_webp),
        )
        .route("/assets/scriptlet.js", axum::routing::get(helios_js))
        .route("/assets/style.css", axum::routing::get(helios_css))
        .route("/__heartbeat__", axum::routing::get(status))
        .route("/s", axum::routing::get(update_status));

    // run it
    let listener = TcpListener::bind(format!("127.0.0.1:{port_at}"))
        .await
        .unwrap();
    println!("Listening on http://127.0.0.1:{port_at}");
    axum::serve(listener, app).await.unwrap()
}

async fn root() -> impl IntoResponse {
    let first_time_data = FIRST_TIME_DATA.clone();
    // include index.html from the html module
    let formatted_helios_html =
        HELIOS_HTML.replace("{{first_time_html}}", &first_time_data.into_html_info());
    Html(formatted_helios_html)
}

async fn helios_image() -> impl IntoResponse {
    // server the helios image with content type
    (
        [(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("image/png"),
        )],
        HELIOS_IMAGE.to_vec(),
    )
        .into_response()
}

async fn helios_image_banner() -> impl IntoResponse {
    // server the helios image with content type
    (
        [(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("image/png"),
        )],
        HELIOS_BANNER.to_vec(),
    )
        .into_response()
}

async fn helios_image_banner_webp() -> impl IntoResponse {
    // server the helios image with content type
    (
        [(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("image/webp"),
        )],
        HELIOS_BANNER_WEBP.to_vec(),
    )
        .into_response()
}

async fn helios_js() -> impl IntoResponse {
    // server the helios js with content type
    (
        [(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/javascript"),
        )],
        HELIOS_JS.to_string(),
    )
        .into_response()
}

async fn helios_css() -> impl IntoResponse {
    // server the helios css with content type
    (
        [(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/css"),
        )],
        HELIOS_CSS.to_string(),
    )
        .into_response()
}

async fn status() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "message": "Helios is running"
    }))
}

async fn update_status() -> impl IntoResponse {
    let system_info = get_system_info_by_lines();

    Json(system_info)
}
