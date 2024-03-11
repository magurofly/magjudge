use std::collections::*;
use std::thread;
use std::sync::*;
use std::error::*;
use std::sync::mpsc::*;

use crate::{SubmissionData, SubmissionStatus};
use crate::program::*;

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
        };
        server.start();
        Self {
            waiting_queue,
            submission_status,
            sender,
        }
    }

    pub fn submit(&'static self, submission_data: SubmissionData) -> Result<(), Box<dyn Error>> {
        let mut queue = self.waiting_queue.lock()?;
        queue.push_back(submission_data);
        self.sender.send(())?;
        Ok(())
    }

    pub fn use_status(&'static self, submission_id: &str, f: impl FnOnce(&SubmissionStatus)) -> Result<(), Box<dyn Error>> {
        let statuses = self.submission_status.lock()?;
        if let Some(status) = statuses.get(submission_id) {
            f(status);
        }
        Ok(())
    }

    fn start(waiting_queue: Arc<Mutex<VecDeque<SubmissionData>>>, submission_status: Arc<Mutex<HashMap<String, SubmissionStatus>>>, receiver: Receiver<()>) {
    }
}

struct JudgeServer {
    waiting_queue: Arc<Mutex<VecDeque<SubmissionData>>>,
    submission_status: Arc<Mutex<HashMap<String, SubmissionStatus>>>,
    receiver: Receiver<()>,
}
impl JudgeServer {
    pub fn start(self) {
        thread::spawn(move || {
            loop {
                let _ = self.receiver.recv();
                self.main();
            }
        });
    }

    fn main(&self) {
        if let Ok(mut queue) = self.waiting_queue.lock() {
            'process_submission: while let Some(submission_data) = queue.pop_front() {
                // コンパイル
                self.set_status(&submission_data.submission_id, "compiling".to_string());
                let Ok(program) = Program::new(&submission_data.submission_id, &submission_data.source_code) else {
                    self.set_status(&submission_data.submission_id, "system-error".to_string());
                    continue 'process_submission
                };
                
                if let Ok(mut statuses) = self.submission_status.lock() {
                    if let Some(status) = statuses.get_mut(&submission_data.submission_id) {
                        status.status = "running".to_string();
                        status.compile_result = Some(program.compile_result().clone());
                    }
                }

                for i in 0 .. submission_data.inputs.len() {
                    if let Ok(result) = program.run(&submission_data.inputs[i]) {
                        if let Ok(mut statuses) = self.submission_status.lock() {
                            if let Some(status) = statuses.get_mut(&submission_data.submission_id) {
                                status.run_results[i] = Some(result);
                            }
                        }
                    }
                }
                
                if let Ok(mut statuses) = self.submission_status.lock() {
                    if let Some(status) = statuses.get_mut(&submission_data.submission_id) {
                        status.status = "finished".to_string();
                    }
                }
            }
        }
    }

    fn set_status(&self, submission_id: &str, new_status: String) {
        if let Ok(mut statuses) = self.submission_status.lock() {
            if let Some(status) = statuses.get_mut(submission_id) {
                status.status = new_status;
            }
        }
    }
}