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
    let mut certs_file = BufReader::new(File::open(&CONFIG.server.ssl_cert_path)?);
    let mut key_file = BufReader::new(File::open(&CONFIG.server.ssl_key_path)?);

    let tls_certs = rustls_pemfile::certs(&mut certs_file).collect::<std::result::Result<Vec<_>, _>>()?;
    let tls_key = rustls_pemfile::pkcs8_private_keys(&mut key_file).next().unwrap()?;

    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
        .unwrap();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin();
        let mut app = App::new()
            .wrap(cors)
            .app_data(JudgeClient::new())
            .service(service_submit)
            .service(service_status);
        for (path, file) in &CONFIG.server.public_files {
            app = app.service(actix_files::Files::new(path, file.to_string()));
        }
        app
    })
        .bind_rustls_0_22(&CONFIG.server.addr_port, tls_config)?
        .run()
        .await
}

fn valid_submission_id(submission_id: &str) -> bool {
    Uuid::parse_str(submission_id).is_ok()
}

use std::time::Instant;
use actix_web::*;
use actix_cors::*;
use config::CONFIG;
use once_cell::sync::Lazy;
use program::compile::CompilingResult;
use program::execute::ExecutionResult;
use serde::*;
use server::JudgeClient;
use uuid::*;
use std::io::BufReader;
use std::fs::*;