use std::collections::*;
use std::thread;
use std::sync::*;
use std::sync::mpsc::*;
use std::time::Instant;

use crate::server::compile::CompilingResult;
use crate::{SubmissionData, SubmissionStatus};
use crate::program::*;
use crate::config::*;

pub struct JudgeClient {
    waiting_queue: Arc<Mutex<VecDeque<SubmissionData>>>,
    submission_status: Arc<Mutex<HashMap<String, SubmissionStatus>>>,
    sender: Sender<()>,
}
impl JudgeClient {
    pub fn new() -> Self {
        let waiting_queue = Arc::new(Mutex::new(VecDeque::new()));
        let submission_status = Arc::new(Mutex::new(HashMap::new()));
        let (sender, receiver) = channel();
        let server = JudgeServer {
            waiting_queue: waiting_queue.clone(),
            submission_status: submission_status.clone(),
            receiver,
            remove_queue: VecDeque::new(),
        };
        server.start();
        Self {
            waiting_queue,
            submission_status,
            sender,
        }
    }

    pub fn submit(&self, submission_data: SubmissionData) {
        let mut statuses = self.submission_status.lock().unwrap();
        statuses.insert(submission_data.submission_id.clone(), SubmissionStatus {
            status: "pending".to_string(),
            compile_result: None,
            run_results: vec![None; submission_data.inputs.len()],
        });

        let mut queue = self.waiting_queue.lock().unwrap();
        queue.push_back(submission_data);

        self.sender.send(()).unwrap();
    }

    pub fn use_status<T>(&self, submission_id: &str, f: impl FnOnce(Option<&SubmissionStatus>) -> T) -> T {
        let statuses = self.submission_status.lock().unwrap();
        f(statuses.get(submission_id))
    }
}

struct JudgeServer {
    waiting_queue: Arc<Mutex<VecDeque<SubmissionData>>>,
    submission_status: Arc<Mutex<HashMap<String, SubmissionStatus>>>,
    remove_queue: VecDeque<(Instant, String)>,
    receiver: Receiver<()>,
}
impl JudgeServer {
    pub fn start(mut self) {
        thread::spawn(move || {
            loop {
                let _ = self.receiver.recv();
                self.main();
            }
        });
    }

    fn main(&mut self) {
        let mut queue = self.waiting_queue.lock().unwrap();
        'process_submission: while let Some(submission_data) = queue.pop_front() {
            let submission_id = &submission_data.submission_id;

            if let Some(status) = self.submission_status.lock().unwrap().get_mut(submission_id) {
                status.status = "compiling".to_string();
            }

            // コンパイル
            let program = match Program::new(&submission_data.submission_id, &submission_data.source_code) {
                Ok(program) => program,
                Err(err) => {
                    if let Some(status) = self.submission_status.lock().unwrap().get_mut(submission_id) {
                        status.status = "compile_error".to_string();
                        status.compile_result = Some(CompilingResult {
                            status: -1,
                            stdout: "".to_string(),
                            stderr: err.to_string(),
                        });
                    }
                    continue 'process_submission
                }
            };
            
            if let Some(status) = self.submission_status.lock().unwrap().get_mut(submission_id) {
                status.status = "running".to_string();
                status.compile_result = Some(program.compile_result().clone());
            }
            
            if program.compile_result().status == 0 {
                for i in 0 .. submission_data.inputs.len() {
                    if let Ok(result) = program.run(&submission_data.inputs[i]) {
                        if let Some(status) = self.submission_status.lock().unwrap().get_mut(submission_id) {
                            status.compile_result = Some(program.compile_result().clone());
                            status.run_results[i] = Some(result);
                        }
                    }
                }

                if let Some(status) = self.submission_status.lock().unwrap().get_mut(submission_id) {
                        status.status = "finished".to_string();
                }
            } else {
                if let Ok(mut statuses) = self.submission_status.lock() {
                    if let Some(status) = statuses.get_mut(&submission_data.submission_id) {
                        status.status = "compile_error".to_string();
                    }
                }
            }

            self.remove_queue.push_back((submission_data.submitted_time, submission_data.submission_id.clone()));
        }

        while let Some(&(submitted_time, _)) = self.remove_queue.front() {
            if Instant::now() - submitted_time <= KEEP_SUBMISSION_TIME {
                break;
            }
            let submission_id = self.remove_queue.pop_front().unwrap().1;
            self.submission_status.lock().unwrap().remove(&submission_id);
        }
    }
}