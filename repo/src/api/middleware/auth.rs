use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, HttpMessage, HttpResponse};
use futures::future::{ok, LocalBoxFuture, Ready};
use crate::config::DbPool;
use crate::models::AuthContext;
use crate::crypto::hash_token;
use diesel::prelude::*;
use chrono::Utc;

pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = actix_web::Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddlewareService { service })
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path().to_string();

        // Skip auth for public endpoints
        if path == "/api/v1/health"
            || path == "/api/v1/auth/login"
            || path == "/api/v1/auth/captcha/challenge"
        {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        // Extract token from cookie or header
        let token = req
            .cookie("ko_session")
            .map(|c| c.value().to_string())
            .or_else(|| {
                req.headers()
                    .get("X-Session-Token")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            });

        let token = match token {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Box::pin(async move {
                    let response = HttpResponse::Unauthorized().json(serde_json::json!({
                        "error": {
                            "code": "UNAUTHORIZED",
                            "message": "Authentication required",
                            "request_id": crate::errors::get_request_id()
                        }
                    }));
                    Ok(req.into_response(response).map_into_right_body())
                });
            }
        };

        let pool = req.app_data::<web::Data<DbPool>>().cloned();
        let config = req.app_data::<web::Data<crate::config::AppConfig>>().cloned();

        let pool = match pool {
            Some(p) => p,
            None => {
                return Box::pin(async move {
                    let response = HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": {"code": "INTERNAL_ERROR", "message": "Server configuration error", "request_id": crate::errors::get_request_id()}
                    }));
                    Ok(req.into_response(response).map_into_right_body())
                });
            }
        };

        let inactivity_hours = config.map(|c| c.session_inactivity_hours).unwrap_or(12);

        // Validate session
        let token_h = hash_token(&token);
        let conn_result = pool.get();

        let auth_result = conn_result.ok().and_then(|mut conn| {
            use crate::schema::{sessions, users, roles};

            let session: Option<crate::models::Session> = sessions::table
                .filter(sessions::token_hash.eq(&token_h))
                .first(&mut conn)
                .ok();

            let session = session?;

            // Check inactivity expiry
            let now = Utc::now().naive_utc();
            let elapsed = now.signed_duration_since(session.last_activity_at);
            if elapsed.num_hours() >= inactivity_hours {
                // Delete expired session
                diesel::delete(sessions::table.filter(sessions::id.eq(session.id)))
                    .execute(&mut conn)
                    .ok();
                return None;
            }

            // Refresh sliding window
            diesel::update(sessions::table.filter(sessions::id.eq(session.id)))
                .set((
                    sessions::last_activity_at.eq(now),
                    sessions::updated_at.eq(now),
                ))
                .execute(&mut conn)
                .ok();

            // Load user + role
            let user_role: Option<(uuid::Uuid, String, uuid::Uuid, bool, String)> = users::table
                .inner_join(roles::table)
                .filter(users::id.eq(session.user_id))
                .select((users::id, users::username, users::role_id, users::active, roles::name))
                .first(&mut conn)
                .ok();

            let (user_id, username, _role_id, active, role_name) = user_role?;

            if !active {
                return None;
            }

            Some(AuthContext {
                user_id,
                username,
                role: role_name,
                session_id: session.id,
            })
        });

        match auth_result {
            Some(ctx) => {
                req.extensions_mut().insert(ctx);
                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
            None => {
                Box::pin(async move {
                    let response = HttpResponse::Unauthorized().json(serde_json::json!({
                        "error": {
                            "code": "UNAUTHORIZED",
                            "message": "Invalid or expired session",
                            "request_id": crate::errors::get_request_id()
                        }
                    }));
                    Ok(req.into_response(response).map_into_right_body())
                })
            }
        }
    }
}
