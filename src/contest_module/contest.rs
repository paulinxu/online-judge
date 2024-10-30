use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use chrono::SecondsFormat;

use crate::user;

// serialize the precision of time is displayed up to milliseconds
fn serialize_datetime<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where S: serde::Serializer,
{
    let s = date.to_rfc3339_opts(SecondsFormat::Millis, true);
    serializer.serialize_str(&s)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetContest
{
    pub id: Option<u32>,
    pub name: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub problem_ids: Vec<u32>,
    pub user_ids: Vec<u32>,
    pub submission_limit: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RankInfo
{
    pub user: user::User,
    pub rank: u32,
    pub scores: Vec<f32>,

    pub highest_scores: Vec<f32>,
    pub latest_scores: Vec<f32>,

    pub competitive_score_sum: f32,
    pub shortest_times: Vec<Vec<i64>>,

    pub latest_submission: DateTime<Utc>,
    pub score: u32,
    pub submission_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Contest
{
    pub id: u32,
    pub name: String,
    #[serde(serialize_with = "serialize_datetime")]
    pub from: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime")]
    pub to: DateTime<Utc>,

    pub problem_ids: Vec<u32>,
    pub user_ids: Vec<u32>,
    pub submission_limit: u32,

    pub users: Vec<RankInfo>,
}