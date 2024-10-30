use serde::Serialize;
use serde::Serializer;
use serde::Deserialize;
use chrono::{DateTime, Utc};

use crate::post_job;

#[derive(Serialize, Debug, Clone)]
pub struct Error
{
    pub code: u32,
    pub reason: String,
    pub message: String
}

impl Error
{
    pub fn new(code: u32, reason: String, message: String) -> Error
    {
        return Error {code, reason, message}
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Request
{
    pub path: String,
    pub method: String,
    pub content: post_job::PostJob,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub enum PossibleResult
{
    Waiting,
    Running,
    Accepted,
    CompilationError,
    CompilationSuccess,
    WrongAnswer,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    SystemError,
    SPJError,
    Skipped,
}

impl Serialize for PossibleResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer,
    {
        let result_str = match *self 
        {
            PossibleResult::Waiting => "Waiting",
            PossibleResult::Running => "Running",
            PossibleResult::Accepted => "Accepted",
            PossibleResult::CompilationError => "Compilation Error",
            PossibleResult::CompilationSuccess => "Compilation Success",
            PossibleResult::WrongAnswer => "Wrong Answer",
            PossibleResult::RuntimeError => "Runtime Error",
            PossibleResult::TimeLimitExceeded => "Time Limit Exceeded",
            PossibleResult::MemoryLimitExceeded => "Memory Limit Exceeded",
            PossibleResult::SystemError => "System Error",
            PossibleResult::SPJError => "SPJ Error",
            PossibleResult::Skipped => "Skipped",
        };
        serializer.serialize_str(result_str)
    }
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct Case
{
    pub id: u32,
    pub result: PossibleResult,
    pub info: String,
    pub time: i64
}

#[derive(Serialize, Debug, Clone)]
pub struct ResponseContent
{
    pub id: u32,
    pub created_time: DateTime<Utc>,
    pub updated_time: DateTime<Utc>,
    pub submission: post_job::PostJob,
    pub state: String,
    pub result: PossibleResult,
    pub score: f32,
    pub cases: Vec<Case>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Response
{
    pub status: u32,
    pub content: ResponseContent,
}

#[derive(Serialize, Debug, Clone)]
pub struct Job
{
    pub poll_for_job: bool,
    pub request: Request,
    pub response: Response,
    // pub restart_server: bool,
}