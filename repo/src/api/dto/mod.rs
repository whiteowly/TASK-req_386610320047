use actix_web::HttpResponse;
use serde::Serialize;
use crate::errors::get_request_id;

pub fn success_response<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "data": data,
        "meta": {"request_id": get_request_id()}
    }))
}

pub fn created_response<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Created().json(serde_json::json!({
        "data": data,
        "meta": {"request_id": get_request_id()}
    }))
}

pub fn paginated_response<T: Serialize>(data: Vec<T>, total: i64, page: i64, page_size: i64) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "data": data,
        "meta": {
            "request_id": get_request_id(),
            "total": total,
            "page": page,
            "page_size": page_size,
        }
    }))
}

pub fn delete_response(message: &str) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "data": {"message": message},
        "meta": {"request_id": get_request_id()}
    }))
}
