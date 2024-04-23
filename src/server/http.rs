use crate::core::Controller;
use crate::core::{CreateUserParams, User};
use anyhow::Result;
use axum::extract::State;
use axum::{routing::post, Json, Router};
use serde::Serialize;
use std::sync::Arc;

struct AppState {
    controller: Controller,
}

pub async fn serve(controller: Controller, port: u16, tls_cert_file: &str, tls_key_file: &str) -> Result<()> {
    // TODO: enable TLS.

    let shared_state = Arc::new(AppState { controller });
    let app = Router::new()
        .route("/api/v1/create_user", post(create_user))
        .with_state(shared_state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    Ok(axum::serve(listener, app).await?)
}

#[derive(Serialize)]
struct CreateUserResponse {
    code: u16,
    user: Option<User>,
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUserParams>,
) -> Json<CreateUserResponse> {
    return match state.controller.create_user(payload).await {
        Ok(user) => {
            let response = CreateUserResponse {
                code: 200,
                user: Some(user),
            };
            Json(response)
        }
        Err(_) => {
            let response = CreateUserResponse { code: 500, user: None };
            Json(response)
        }
    };
}
