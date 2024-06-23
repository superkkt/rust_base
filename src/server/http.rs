use crate::core::controller::Controller;
use crate::core::controller::CreateUserParams as ControllerCreateUserParams;
use crate::core::controller::GetUserParams as ControllerGetUserParams;
use crate::core::entity::{DatabaseTransaction, User};

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::{routing::post, Json, Router};
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};

struct AppState<T> {
    controller: Controller<T>,
}

pub async fn serve<T>(
    controller: Controller<T>,
    port: u16,
    tls_cert_file: &str,
    tls_key_file: &str,
) -> Result<()>
where
    T: DatabaseTransaction + Send + Sync + 'static,
{
    let shared_state = Arc::new(AppState { controller });
    let app = Router::new()
        .route("/api/v1/create_user", post(create_user))
        .route("/api/v1/get_user", post(get_user))
        .with_state(shared_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let config = RustlsConfig::from_pem_file(tls_cert_file, tls_key_file)
        .await
        .context(format!(
            "failed to load TLS cert and key files: {tls_cert_file}, {tls_key_file}"
        ))?;

    Ok(axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .context(format!("failed to bind HTTP server: {addr}"))?)
}

#[derive(Deserialize)]
struct CreateUserParams {
    pub username: String,
    pub password: String,
    pub age: u16,
    pub address: String,
}

impl Into<ControllerCreateUserParams> for CreateUserParams {
    fn into(self) -> ControllerCreateUserParams {
        ControllerCreateUserParams {
            username: self.username,
            password: self.password,
            age: self.age,
            address: self.address,
        }
    }
}

#[derive(Serialize)]
struct CreateUserResponse {
    code: u16,
    user: Option<User>,
}

async fn create_user<T>(
    State(state): State<Arc<AppState<T>>>,
    Json(payload): Json<CreateUserParams>,
) -> Json<CreateUserResponse>
where
    T: DatabaseTransaction + Send + Sync,
{
    log::debug!("create_user invoked");
    return match state.controller.create_user(payload).await {
        Ok(user) => {
            let response = CreateUserResponse {
                code: 200,
                user: Some(user),
            };
            Json(response)
        }
        Err(err) => {
            log::error!("failed to create a user: {err:?}");
            let response = CreateUserResponse {
                code: 500,
                user: None,
            };
            Json(response)
        }
    };
}

#[derive(Deserialize)]
struct GetUserParams {
    pub id: u64,
}

impl Into<ControllerGetUserParams> for GetUserParams {
    fn into(self) -> ControllerGetUserParams {
        ControllerGetUserParams { id: self.id }
    }
}

#[derive(Serialize)]
struct GetUserResponse {
    code: u16,
    user: Option<User>,
}

async fn get_user<T>(
    State(state): State<Arc<AppState<T>>>,
    Json(payload): Json<GetUserParams>,
) -> Json<GetUserResponse>
where
    T: DatabaseTransaction + Send + Sync,
{
    log::debug!("get_user invoked");
    return match state.controller.get_user(payload).await {
        Ok(user) => {
            if user.is_none() {
                Json(GetUserResponse {
                    code: 300,
                    user: None,
                })
            } else {
                Json(GetUserResponse { code: 200, user })
            }
        }
        Err(err) => {
            log::error!("failed to get a user: {err:?}");
            let response = GetUserResponse {
                code: 500,
                user: None,
            };
            Json(response)
        }
    };
}
