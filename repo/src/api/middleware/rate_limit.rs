use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, HttpMessage, HttpResponse};
use futures::future::{ok, LocalBoxFuture, Ready};
use crate::config::DbPool;
use crate::models::AuthContext;
use diesel::prelude::*;
use chrono::Utc;

pub struct RateLimitMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = actix_web::Error;
    type Transform = RateLimitService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RateLimitService { service })
    }
}

pub struct RateLimitService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RateLimitService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_ctx = req.extensions().get::<AuthContext>().cloned();
        let pool = req.app_data::<web::Data<DbPool>>().cloned();
        let config = req.app_data::<web::Data<crate::config::AppConfig>>().cloned();
        let limit = config.as_ref().map(|c| c.rate_limit_per_minute).unwrap_or(60);

        if let (Some(ctx), Some(pool)) = (auth_ctx, pool) {
            if let Ok(mut conn) = pool.get() {
                use crate::schema::rate_limits;
                let now = Utc::now().naive_utc();
                let window_start = now - chrono::Duration::seconds(60);

                // Count requests in current window
                let count: i64 = rate_limits::table
                    .filter(rate_limits::user_id.eq(Some(ctx.user_id)))
                    .filter(rate_limits::window_start.ge(window_start))
                    .select(diesel::dsl::sum(rate_limits::request_count))
                    .first::<Option<i64>>(&mut conn)
                    .unwrap_or(None)
                    .unwrap_or(0);

                if count >= limit as i64 {
                    return Box::pin(async move {
                        let response = HttpResponse::TooManyRequests().json(serde_json::json!({
                            "error": {
                                "code": "RATE_LIMITED",
                                "message": "Rate limit exceeded. Max 60 requests per minute.",
                                "request_id": crate::errors::get_request_id()
                            }
                        }));
                        Ok(req.into_response(response).map_into_right_body())
                    });
                }

                // Record this request
                diesel::insert_into(rate_limits::table)
                    .values((
                        rate_limits::id.eq(uuid::Uuid::new_v4()),
                        rate_limits::user_id.eq(Some(ctx.user_id)),
                        rate_limits::window_start.eq(now),
                        rate_limits::request_count.eq(1),
                        rate_limits::created_at.eq(now),
                    ))
                    .execute(&mut conn)
                    .ok();
            }
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}
