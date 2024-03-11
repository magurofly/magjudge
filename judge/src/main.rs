pub mod config;
pub mod program;
pub mod server;

thread_local! {
    static JUDGE_CLIENT: Box<JudgeClient> = Box::new(JudgeClient::new());
}

#[derive(Default)]
struct ServerData {
    submission_status: HashMap<String, SubmissionStatus>,
    program: HashMap<String, Program>,
}

#[derive(Clone, Debug)]
pub struct SubmissionData {
    submitted_time: Instant,
    submission_id: String,
    source_code: String,
    inputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SubmissionStatus {
    status: String,
    compile_result: Option<CompilingResult>,
    run_results: Vec<Option<ExecutionResult>>,
}

#[post("/submit")]
async fn service_submit(req: HttpRequest) -> impl Responder {
    #[derive(Deserialize)]
    struct RequestData {
        source_code: String,
        inputs: Vec<String>,
    }
    let Ok(RequestData { source_code, inputs }) = web::Json::<RequestData>::extract(&req).await.map(|json| json.into_inner() ) else {
        return HttpResponse::BadRequest().body("")
    };

    let submission_id = Uuid::new_v4().hyphenated().to_string();
    let now = Instant::now();

    JUDGE_CLIENT.with(|client| {
        client.submit(SubmissionData {
            submitted_time: now,
            submission_id: submission_id.clone(),
            source_code,
            inputs,
        });
    });

    HttpResponse::Ok().body(submission_id)
}

#[get("/status")]
async fn service_status_none() -> HttpResponse {
    HttpResponse::NotFound().body("")
}

#[get("/status/{submission_id}")]
async fn service_status(req: HttpRequest) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseData {
        status: String,
        results: Vec<ExecResult>,
    }

    #[derive(Serialize)]
    struct ExecResult {
        status: String,
        compiler_output: Option<String>,

    }

    let submission_id = todo!();

    let borrowed_data = req.app_data::<RefCell<ServerData>>().unwrap().borrow();
    if let Some(status) = borrowed_data.submission_status.get(&submission_id) {
        HttpResponse::Ok().json(status)
    } else {
        return HttpResponse::NotFound().body(r#"{"status":"not_found"}"#)
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(JudgeClient::new())
            .service(service_submit)
            .service(service_status)
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}

use std::collections::*;
use std::cell::*;
use std::time::Instant;
use actix_web::*;
use program::compile::CompilingResult;
use program::execute::ExecutionResult;
use program::Program;
use serde::*;
use server::JudgeClient;
use uuid::*;