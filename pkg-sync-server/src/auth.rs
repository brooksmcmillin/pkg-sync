use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

use crate::{db, AppState};

pub struct AuthenticatedMachine {
    pub id: i64,
    pub name: String,
}

impl FromRequestParts<Arc<AppState>> for AuthenticatedMachine {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let api_key = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Missing or invalid Authorization header".to_string(),
            ))?;

        let machine = db::lookup_machine_by_key(&state.db, api_key)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Invalid API key".to_string(),
            ))?;

        Ok(AuthenticatedMachine {
            id: machine.0,
            name: machine.1,
        })
    }
}
