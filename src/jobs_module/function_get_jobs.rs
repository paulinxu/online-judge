use actix_web::{get, Responder, HttpResponse, web};
use serde::Deserialize;
use chrono::{DateTime, Utc};

use crate::JOB_LIST;
use crate::job;

use crate::USER_LIST;
use crate::user;
use crate::sql;

use crate::Connection;
use crate::CONNECTION;

// used to obtain query data
#[derive(Debug, Deserialize)]
pub struct AuthRequest // from https://docs.rs/actix-web/latest/actix_web/web/struct.Query.html
{
    user_id: Option<u32>,
    user_name: Option<String>,
    contest_id: Option<u32>,
    problem_id: Option<u32>,
    language: Option<String>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    state: Option<String>,
    result: Option<job::PossibleResult>,
}

// function that returns user id given its name
fn get_id(user_name: String, connection: &Connection) -> Option<u32>
{
    let mut lock_user_list: std::sync::MutexGuard<Vec<user::User>> = USER_LIST.lock().unwrap();
    // if SQL storage is on, then load from database
    sql::get_user(&connection, &mut lock_user_list);

    for user in lock_user_list.iter()
    {
        if user.name == user_name
        {
            return Some(user.id);
        }
    }
    return None;
}

// gets list of jobs that satisfy query requirements
#[get("/jobs")]
async fn get_jobs(info: web::Query<AuthRequest>) -> impl Responder {
    let connection = CONNECTION.lock().unwrap();

    let mut lock_job_list = JOB_LIST.lock().unwrap();
    // if SQL storage is on, then load from database
    sql::get_job(&connection, &mut lock_job_list);

    // filtering list of jobs based on query information 
    let filtered_jobs: Vec<job::ResponseContent> = lock_job_list.iter()
        .filter(|job| {
            if let Some(user_id) = info.user_id {
                if job.submission.user_id != user_id {
                    return false;
                }
            }
            if let Some(user_name) = &info.user_name {
                if let Some(user_id) = get_id(user_name.to_string(), &connection)
                {
                    if job.submission.user_id != user_id {
                        return false;
                    }
                }
            }
            if let Some(contest_id) = info.contest_id {
                if job.submission.contest_id != contest_id {
                    return false;
                }
            }
            if let Some(problem_id) = info.problem_id {
                if job.submission.problem_id != problem_id {
                    return false;
                }
            }
            if let Some(language) = &info.language {
                if job.submission.language != *language {
                    return false;
                }
            }
            if let Some(from) = info.from {
                if job.created_time < from {
                    return false;
                }
            }
            if let Some(to) = info.to {
                if job.created_time > to {
                    return false;
                }
            }
            if let Some(state) = &info.state {
                if job.state != *state {
                    return false;
                }
            }
            if let Some(result) = &info.result {
                if job.result != *result {
                    return false;
                }
            }
            return true;
        })
        .cloned()
        .collect();
    HttpResponse::Ok().json(filtered_jobs)
}

// returns job information provided its id
#[get("/jobs/{jobId}")]
#[allow(non_snake_case)]
async fn get_jobs_jobId(jobId: web::Path<u32>) -> impl Responder 
{
    let connection = CONNECTION.lock().unwrap();

    let mut lock_job_list = JOB_LIST.lock().unwrap();
    // if SQL storage is on, then load from database
    sql::get_job(&connection, &mut lock_job_list);
    
    for content in lock_job_list.iter()
    {
        if jobId.to_owned() == content.id
        {
            return HttpResponse::Ok().json(content.clone());
        }
    }
    // if not found, then return an error
    return HttpResponse::NotFound().json(job::Error::new(3, "ERR_NOT_FOUND".to_string(), format!("Job {} not found.", jobId.to_owned()).to_string()));
}