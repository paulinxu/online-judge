use actix_web::{put, Responder, HttpResponse, web};
use std::sync::Mutex;
use std::sync::Arc;

use crate::JOB_LIST;
use crate::job;
use crate::sql;
use crate::config;
use crate::function_post_jobs;
use crate::web::Json;
use crate::CONNECTION;

// puts a job
#[put("/jobs/{jobId}")]
#[allow(non_snake_case)]
async fn get_jobs_jobId(jobId: web::Path<u32>, data_config: web::Data<Arc<Mutex<config::Config>>>) -> impl Responder 
{
    let connection = CONNECTION.lock().unwrap();
    
    let mut lock_job_list = JOB_LIST.lock().unwrap();
    // if SQL storage activated, then load from database
    sql::get_job(&connection, &mut lock_job_list);
    // find job based on the id provided
    for content in lock_job_list.iter_mut()
    {
        if jobId.to_owned() == content.id
        {
            // run the job again and store the new result
            *content = function_post_jobs::post_jobs_action(Json(content.submission.clone()), data_config, true, &connection);
            content.id = jobId.to_owned();
            log::info!("Successfull put with ID: {}", jobId.to_owned());

            let output = content.clone();
            
            sql::push_job(&connection, &mut lock_job_list);

            return HttpResponse::Ok().json(output);
        }
    }
    return HttpResponse::NotFound().json(job::Error::new(3, "ERR_NOT_FOUND".to_string(), format!("Job {} not found.", jobId.to_owned()).to_string()));
}