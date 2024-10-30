use std::i64::MAX;

use actix_web::{post, Responder, HttpResponse, web};
use chrono::{DateTime, Utc};

use crate::user;
use crate::job;
use crate::contest;
use crate::sql;


use crate::USER_LIST;
use crate::USER_ID_COUNT;

use crate::CONTEST_LIST;

use crate::CONNECTION;

// checks if a user exists given its name
fn check_exists(list: &std::sync::MutexGuard<Vec<user::User>>, name: String) -> bool
{
    for element in list.iter()
    {
        if element.name == name {return true;}
    }
    return false;
}

// posts a new user
#[post("/users")]
async fn post_users(body: web::Json<user::GetUser>) -> impl Responder 
{
    let connection = CONNECTION.lock().unwrap();

    let mut lock_user_list: std::sync::MutexGuard<Vec<user::User>> = USER_LIST.lock().unwrap();
    
    // if SQL storage is activated, then load from database
    sql::get_user(&connection, &mut lock_user_list);

    // if a user with this name already exists, then return error
    if check_exists(&lock_user_list, body.name.clone())
    {
        return HttpResponse::BadRequest().json(job::Error::
            new(
                1, 
                "ERR_INVALID_ARGUMENT".to_string(), 
                format!("User name '{}' already exists.", body.name.clone()).to_string()
            ));
    }

    // if id is provided, then update user information
    if let Some(id) = body.id
    {
        // loops through existent users to update information
        for user in lock_user_list.iter_mut()
        {
            if id == user.id
            {
                user.name = body.name.clone();

                let output = user.clone();

                // pushes information into SQL database if persistent storage is activated
                sql::push_user(&connection, &mut lock_user_list);

                return HttpResponse::Ok().json(output);
            }
        }
        // if not found then return error
        return HttpResponse::NotFound().json(job::Error::
            new(
                3, 
                "ERR_NOT_FOUND".to_string(), 
                format!("User {} not found.", id.to_owned()).to_string()
            ));
    }
    // if id not provided, then new user added
    else 
    {
        let mut lock_user_id_count = USER_ID_COUNT.lock().unwrap();

        // if SQL storage is activated, then load from database
        sql::get_user_count(&connection, &mut lock_user_id_count);

        // push new user into user list, with a new id
        lock_user_list.push(user::User
            {   
                id: *lock_user_id_count, 
                name: body.name.clone(),
            });
        
        // START setup for contest 0

        let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
        
        // adds id of the new user into user_ids
        lock_contest_list[0].user_ids.push(*lock_user_id_count as u32);
        let number_of_problems = lock_contest_list[0].problem_ids.len();
        
        // adds a new user into user lists
        // use to store user progress 
        lock_contest_list[0].users.push(contest::RankInfo
        {
            user: lock_user_list[lock_user_list.len() -1].clone(),
            scores: vec![0.0; number_of_problems],
            rank: 0,

            highest_scores: vec![0.0 ; number_of_problems],
            latest_scores: vec![0.0 ; number_of_problems],

            competitive_score_sum: 0.0,
            shortest_times: vec![vec![MAX; 20]; number_of_problems], // possible error

            latest_submission: DateTime::<Utc>::MAX_UTC,
            score: 0,
            submission_count: 0,
        });

        // END setup for contest 0

        *lock_user_id_count += 1;
        sql::push_user_count(&connection, *lock_user_id_count);

        sql::push_user(&connection, &mut lock_user_list);
        return HttpResponse::Ok().json(lock_user_list[(*lock_user_id_count-1) as usize].clone());
    }
}