use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostJob {
    pub source_code: String,
    pub language: String,
    pub user_id: u32,
    pub contest_id: u32,
    pub problem_id: u32
}