//! Loopback OAuth callback lifecycle shared by desktop and CLI hosts.

use super::OAuthCallback;
use axum::{extract::State, response::IntoResponse, routing::get, Router};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{oneshot, Mutex},
    task::JoinHandle,
};
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum OAuthCallbackServerError {
    #[error("invalid OAuth callback path")]
    InvalidPath,
    #[error("failed to bind OAuth callback listener")]
    Bind(#[source] std::io::Error),
    #[error("OAuth callback server failed")]
    Serve(#[source] std::io::Error),
    #[error("OAuth callback server task failed")]
    Join(#[source] tokio::task::JoinError),
    #[error("OAuth callback receiver was already consumed")]
    ReceiverConsumed,
}

#[derive(Clone)]
struct CallbackState {
    sender: Arc<Mutex<Option<oneshot::Sender<OAuthCallback>>>>,
}

/// Cloneable control handle retained by a host while the server task waits for
/// a browser redirect. It can submit a manually copied redirect URL or stop
/// the listener during cancellation.
#[derive(Clone)]
pub struct OAuthCallbackHandle {
    sender: Arc<Mutex<Option<oneshot::Sender<OAuthCallback>>>>,
    shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl OAuthCallbackHandle {
    pub async fn submit(&self, callback: OAuthCallback) -> Result<(), OAuthCallbackServerError> {
        let sender = self
            .sender
            .lock()
            .await
            .take()
            .ok_or(OAuthCallbackServerError::ReceiverConsumed)?;
        sender
            .send(callback)
            .map_err(|_| OAuthCallbackServerError::ReceiverConsumed)
    }

    pub async fn stop(&self) {
        if let Some(shutdown) = self.shutdown.lock().await.take() {
            let _ = shutdown.send(());
        }
    }
}

/// A one-shot loopback listener. The listener only accepts the configured
/// callback path and shuts down explicitly after the host consumes the result.
pub struct OAuthCallbackServer {
    redirect_uri: String,
    receiver: Option<oneshot::Receiver<OAuthCallback>>,
    handle: OAuthCallbackHandle,
    task: Option<JoinHandle<Result<(), std::io::Error>>>,
}

impl OAuthCallbackServer {
    pub async fn bind(port: u16, callback_path: &str) -> Result<Self, OAuthCallbackServerError> {
        let callback_path = normalize_callback_path(callback_path)?;
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(OAuthCallbackServerError::Bind)?;
        let address = listener
            .local_addr()
            .map_err(OAuthCallbackServerError::Bind)?;
        let (sender, receiver) = oneshot::channel();
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        let state = CallbackState {
            sender: Arc::new(Mutex::new(Some(sender))),
        };
        let handle = OAuthCallbackHandle {
            sender: state.sender.clone(),
            shutdown: Arc::new(Mutex::new(Some(shutdown_sender))),
        };
        let router = Router::new()
            .route(&callback_path, get(callback_handler))
            .with_state(state);
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_receiver.await;
                })
                .await
        });
        Ok(Self {
            redirect_uri: format!("http://{address}{callback_path}"),
            receiver: Some(receiver),
            handle,
            task: Some(task),
        })
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    pub fn handle(&self) -> OAuthCallbackHandle {
        self.handle.clone()
    }

    pub async fn wait_for_callback(&mut self) -> Result<OAuthCallback, OAuthCallbackServerError> {
        let receiver = self
            .receiver
            .take()
            .ok_or(OAuthCallbackServerError::ReceiverConsumed)?;
        let callback = receiver
            .await
            .map_err(|_| OAuthCallbackServerError::ReceiverConsumed)?;
        self.stop().await?;
        Ok(callback)
    }

    pub async fn stop(&mut self) -> Result<(), OAuthCallbackServerError> {
        self.handle.stop().await;
        if let Some(task) = self.task.take() {
            task.await
                .map_err(OAuthCallbackServerError::Join)?
                .map_err(OAuthCallbackServerError::Serve)?;
        }
        Ok(())
    }
}

impl Drop for OAuthCallbackServer {
    fn drop(&mut self) {
        self.task.take().map(|task| task.abort());
    }
}

async fn callback_handler(
    State(state): State<CallbackState>,
    query: axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let callback = OAuthCallback {
        code: query.get("code").cloned(),
        state: query.get("state").cloned(),
        error: query.get("error").cloned(),
        error_description: query.get("error_description").cloned(),
    };
    let is_error = callback.error.is_some();
    if let Some(sender) = state.sender.lock().await.take() {
        let _ = sender.send(callback);
    }
    if is_error {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "OpenMesh authorization was rejected. You may close this window.",
        )
    } else {
        (
            axum::http::StatusCode::OK,
            "OpenMesh authorization completed. You may close this window.",
        )
    }
}

fn normalize_callback_path(path: &str) -> Result<String, OAuthCallbackServerError> {
    let path = path.trim();
    if path.is_empty() || !path.starts_with('/') || path.contains(['?', '#']) {
        return Err(OAuthCallbackServerError::InvalidPath);
    }
    Ok(path.to_owned())
}

/// Parse a callback URL without logging or preserving raw query material.
pub fn parse_callback_url(raw_url: &str) -> Result<OAuthCallback, super::OAuthError> {
    let url = Url::parse(raw_url)
        .map_err(|_| super::OAuthError::Request("callback URL is invalid".to_owned()))?;
    let mut callback = OAuthCallback::default();
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => callback.code = Some(value.into_owned()),
            "state" => callback.state = Some(value.into_owned()),
            "error" => callback.error = Some(value.into_owned()),
            "error_description" => callback.error_description = Some(value.into_owned()),
            _ => {}
        }
    }
    Ok(callback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn callback_server_delivers_one_shot_result_and_stops() {
        let mut server = OAuthCallbackServer::bind(0, "/callback").await.unwrap();
        let callback_url = format!(
            "{}?code=oauth-code&state=oauth-state",
            server.redirect_uri()
        );
        let response = reqwest::get(callback_url).await.unwrap();
        assert!(response.status().is_success());
        let callback = server.wait_for_callback().await.unwrap();
        assert_eq!(callback.code.as_deref(), Some("oauth-code"));
        assert_eq!(callback.state.as_deref(), Some("oauth-state"));
    }

    #[test]
    fn callback_url_parser_discards_unrecognized_query_fields() {
        let callback =
            parse_callback_url("http://127.0.0.1/callback?code=abc&state=xyz&ignored=secret")
                .unwrap();
        assert_eq!(callback.code.as_deref(), Some("abc"));
        assert_eq!(callback.state.as_deref(), Some("xyz"));
        assert!(!format!("{callback:?}").contains("secret"));
    }

    #[test]
    fn callback_path_must_be_a_path_only() {
        assert!(matches!(normalize_callback_path("callback"), Err(_)));
        assert!(matches!(normalize_callback_path("/callback?x=1"), Err(_)));
    }
}
