pub mod config;
pub mod program;
pub mod server;

static JUDGE_CLIENT: Lazy<JudgeClient> = Lazy::new(|| JudgeClient::new() );

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

#[get("/")]
async fn service_index() -> impl Responder {
    HttpResponse::Ok().body(r#"
<!doctype html>
<html lang="ja">
    <head>
        <meta charset="UTF-8">
        <title>Rust Run Server</title>
    </head>
    <body>
        <h1>使い方</h1>
        <section>
            <h2>POST /submit</h2>
            <p>ソースコードと入力（複数）を送信すると、 <code>submission_id</code> を返します。</p>
            <p>入力形式: JSON</p>
            <pre>{
    "source_code": string,
    "inputs": [string],
}</pre>
        </section>
        <section>
            <h2>GET /status/{submission_id}</h2>
            <p>現在の状況を取得します。なお、ソースコードを送信した時刻から一定時間経過するとサーバーから削除され、取得できなくなります。</p>
            <p>出力形式: JSON</p>
            <pre>{
    "status": "invalid_id" | "not_found" | "compile_error" | "pending" | "compiling" | "running" | "finished",
    "compile_result": {
        "status": number,
        "stdout": string,
        "stderr": string,
    }?,
    "run_results": [
        {
            "status": number,
            "time_ms": number,
            "stdout": string,
            "stderr": string,
        }?
    ],
}</pre>
        </section>
    </body>
</html>
    "#)
}


#[derive(Deserialize)]
struct SubmissionRequestData {
    source_code: String,
    inputs: Vec<String>,
}

#[post("/submit")]
async fn service_submit(data: web::Json<SubmissionRequestData>) -> impl Responder {
    let SubmissionRequestData { source_code, inputs } = data.into_inner();

    let submission_id = Uuid::new_v4().hyphenated().to_string();
    let now = Instant::now();

    JUDGE_CLIENT.submit(SubmissionData {
        submitted_time: now,
        submission_id: submission_id.clone(),
        source_code,
        inputs,
    });

    HttpResponse::Ok().body(submission_id)
}

#[get("/status")]
async fn service_status_none() -> HttpResponse {
    HttpResponse::NotFound().body("")
}

#[get("/status/{submission_id}")]
async fn service_status(path: web::Path<String>) -> impl Responder {
    let submission_id = path.into_inner();
    if !valid_submission_id(&submission_id) {
        return HttpResponse::BadRequest().json(SubmissionStatus {
            status: "invalid_id".to_string(),
            compile_result: None,
            run_results: vec![],
        });
    }
    JUDGE_CLIENT.use_status(&submission_id, |status| {
        if let Some(status) = status {
            HttpResponse::Ok().json(status)
        } else {
            HttpResponse::NotFound().json(SubmissionStatus {
                status: "not_found".to_string(),
                compile_result: None,
                run_results: vec![],
            })
        }
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin();
        App::new()
            .wrap(cors)
            .app_data(JudgeClient::new())
            .service(service_index)
            .service(service_submit)
            .service(service_status)
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}

fn valid_submission_id(submission_id: &str) -> bool {
    Uuid::parse_str(submission_id).is_ok()
}

use std::time::Instant;
use actix_web::*;
use actix_cors::*;
use once_cell::sync::Lazy;
use program::compile::CompilingResult;
use program::execute::ExecutionResult;
use serde::*;
use server::JudgeClient;
use uuid::*;