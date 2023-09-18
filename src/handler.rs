use crate::{
    model::TodoModel,
    schema::{CreateTodoSchema, FilterOptions, UpdateTodoSchema},
    AppState,
};
use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use chrono::prelude::*;
use serde_json::json;

#[get("/todos")]
async fn todo_list_handler(
    opts: web::Query<FilterOptions>,
    data: web::Data<AppState>,
) -> impl Responder {
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
        TodoModel,
        "SELECT * FROM todos ORDER BY id LIMIT $1 OFFSET $2",
        limit as i32,
        offset as i32
    )
    .fetch_all(&data.db)
    .await;

    if query_result.is_err() {
        let message = "Something bad happened fetching";
        return HttpResponse::InternalServerError()
            .json(json!({"status": "error", "message": message}));
    }

    let todos = query_result.unwrap();

    let json_response = serde_json::json!({
        "status": "success",
        "results": todos.len(),
        "todos": todos
    });

    HttpResponse::Ok().json(json_response)
}

#[post("/todos/")]
async fn create_todo_handler(
    body: web::Json<CreateTodoSchema>,
    data: web::Data<AppState>,
) -> impl Responder {
    let query_result = sqlx::query_as!(
        TodoModel,
        "INSERT INTO todos (title, content, category) VALUES ($1, $2, $3) RETURNING *",
        body.title.to_string(),
        body.content.to_string(),
        body.category.to_owned().unwrap_or("".to_string())
    )
    .fetch_one(&data.db)
    .await;

    match query_result {
        Ok(todo) => {
            let todo_response = serde_json::json!({"status": "success", "data": serde_json::json!({
                "todo": todo
            })});

            return HttpResponse::Ok().json(todo_response);
        }

        Err(err) => {
            if err
                .to_string()
                .contains("duplicate key value violates unique constraint")
            {
                return HttpResponse::BadRequest().json(serde_json::json!({"status": "fail", "message": "Todo with that title already exists"}));
            }

            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"status": "error", "message": format!("{:?}", err)}));
        }
    }
}

#[get("/todos/{id}")]
async fn get_todo_handler(
    path: web::Path<uuid::Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let todo_id = path.into_inner();
    let query_result = sqlx::query_as!(TodoModel, "SELECT * FROM todos WHERE id = $1", todo_id)
        .fetch_one(&data.db)
        .await;

    match query_result {
        Ok(todo) => {
            let todo_response = serde_json::json!({"status": "success", "data": serde_json::json!({
                "todo": todo
            })});

            return HttpResponse::Ok().json(todo_response);
        }

        Err(_) => {
            let message = format!("Todo with ID: {} not found", todo_id);
            return HttpResponse::NotFound()
                .json(serde_json::json!({"status": "fail", "message": message}));
        }
    }
}

#[delete("/todos/{id}")]
async fn delete_todo_handler(
    path: web::Path<uuid::Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let todo_id = path.into_inner();

    let query_result = sqlx::query!("DELETE FROM todos WHERE id = $1", todo_id)
        .execute(&data.db)
        .await;

    match query_result.unwrap().rows_affected() {
        0 => return HttpResponse::NotFound().finish(),
        _ => return HttpResponse::NoContent().finish(),
    }
}

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api")
        .service(todo_list_handler)
        .service(create_todo_handler)
        .service(get_todo_handler)
        .service(delete_todo_handler);

    conf.service(scope);
}
