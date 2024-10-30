use actix_web::{post, Responder, HttpResponse, web};
use std::i64::MAX;
use std::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::contest;
use crate::job;
use crate::config;
use crate::user;
use crate::sql;
use crate::Connection;

use crate::CONTEST_LIST;
use crate::CONTEST_ID_COUNT;

use crate::USER_ID_COUNT;
use crate::USER_LIST;

use crate::CONNECTION;

// checks if all problems and users exist
fn check_valid(body: &web::Json<contest::GetContest>, data_config: &web::Data<Arc<Mutex<config::Config>>>, connection: &Connection) -> bool
{
    let mut lock_user_id_count = USER_ID_COUNT.lock().unwrap();
    
    // if SQL mode is activated, then load from database
    sql::get_user_count(&connection, &mut lock_user_id_count);
    
    // checks if each user id is valid
    for id in body.user_ids.iter()
    {
        if *id >= *lock_user_id_count {return false;}
    }
    //check if each problem id is valid
    let config = data_config.lock().unwrap();
    for id in body.problem_ids.iter()
    {
        if *id >= config.problems.len() as u32 {return false;}
    }
    return true;
}

// uses hash maps to check if user ids or problem ids are repeated
fn check_repeated(body: &web::Json<contest::GetContest>) -> bool
{
    // checks if any problem id is repeated
    let mut occurrences: HashMap<u32, u32> = HashMap::new();
    for &num in &body.problem_ids {
        let counter = occurrences.entry(num).or_insert(0);
        *counter += 1;
        if *counter > 1{return true;}
    }

    // checks if any user id is repeated
    let mut occurrences2: HashMap<u32, u32> = HashMap::new();
    for &num in &body.user_ids {
        let counter = occurrences2.entry(num).or_insert(0);
        *counter += 1;
        if *counter > 1{return true;}
    }
    return false;
}

// returns user information given an id
fn get_user(user_id: u32, lock_user_list: &std::sync::MutexGuard<Vec<user::User>>) -> user::User
{
    for user in lock_user_list.iter()
    {
        if user.id == user_id
        {
            return user.clone();
        }
    }
    panic!("User not found");
}

// posts a new contest
#[post("/contests")]
async fn post_contests(body: web::Json<contest::GetContest>, data_config: web::Data<Arc<Mutex<config::Config>>>) -> impl Responder 
{
    let connection = CONNECTION.lock().unwrap();
    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
    
    // if SQL mode is activated, then load from database
    sql::get_contest(&connection, &mut lock_contest_list);

    // check that the provided contest is valid
    if !check_valid(&body, &data_config, &connection)
    {
        return HttpResponse::NotFound().json(job::Error::
            new(3, "ERR_NOT_FOUND".to_string(), "Problem or user not found".to_string()));
    }
    if check_repeated(&body)
    {
        return HttpResponse::BadRequest().json(job::Error::
            new(1, "ERR_INVALID_ARGUMENT".to_string(), "Invalid argument".to_string()));
    }

    // if a contest id is provided, then it will update the information 
    if let Some(id) = body.id
    {
        for contest in lock_contest_list.iter_mut()
        {
            // cannot update contest 0 (special case)
            if id == 0
            {
                return HttpResponse::BadRequest().json(job::Error::
                    new(1, "ERR_INVALID_ARGUMENT".to_string(), "Invalid contest id".to_string()));
            }
            // finds the contest and updates the information
            if id == contest.id
            {
                *contest = contest::Contest
                {
                    id: id,
                    name: body.name.clone(),
                    from: body.from,
                    to: body.to,
                    problem_ids: body.problem_ids.clone(),
                    user_ids: body.user_ids.clone(),
                    submission_limit: body.submission_limit,

                    users: vec![]
                };

                let mut lock_user_list: std::sync::MutexGuard<Vec<user::User>> = USER_LIST.lock().unwrap();

                // if SQL mode activated, then load from database
                sql::get_user(&connection, &mut lock_user_list);

                // vector that keeps track of each user's data
                for id in contest.user_ids.iter()
                {
                    contest.users.push(contest::RankInfo
                        {
                            user: get_user(id.clone(), &lock_user_list),
                            rank: 0,
                            scores: vec![0.0 ; contest.problem_ids.len()],

                            highest_scores: vec![0.0 ; contest.problem_ids.len()],
                            latest_scores: vec![0.0 ; contest.problem_ids.len()],

                            competitive_score_sum: 0.0,
                            shortest_times: vec![vec![MAX; 20]; contest.problem_ids.len()], // possible error

                            latest_submission: DateTime::<Utc>::MAX_UTC,
                            score: 0,
                            submission_count: 0,
                        }
                    )
                }
                
                let output = contest.clone();
                sql::push_contest(&connection, &lock_contest_list);
                // return the updated contest
                return HttpResponse::Ok().json(output);
            }
        }
        // if the contest is not found then return an error
        return HttpResponse::NotFound().json(job::Error::
            new(3, "ERR_NOT_FOUND".to_string(), format!("Contest {} not found.", id.to_owned()).to_string()));
    }
    // if no id is provided, then push new contest into end of list
    else 
    {
        let mut lock_contest_id_count = CONTEST_ID_COUNT.lock().unwrap();
        // if SQL mode activated, then load from database
        sql::get_contest_count(&connection, &mut lock_contest_id_count);

        // pushes contest into list
        lock_contest_list.push(
            contest::Contest
            {
                id: *lock_contest_id_count,
                name: body.name.clone(),
                from: body.from,
                to: body.to,
                problem_ids: body.problem_ids.clone(),
                user_ids: body.user_ids.clone(),
                submission_limit: body.submission_limit,

                users: vec![]
            }   
        );
        // if SQL mode activated, then load from database
        sql::push_contest(&connection, &(lock_contest_list.clone()));

        let contest = &mut lock_contest_list[(*lock_contest_id_count) as usize];
        
        let mut lock_user_list: std::sync::MutexGuard<Vec<user::User>> = USER_LIST.lock().unwrap();
        // if SQL mode activated, then load from database
        sql::get_user(&connection, &mut lock_user_list);

        // adds vector that keep tracks of user progress
        for id in contest.user_ids.iter()
        {
            contest.users.push(contest::RankInfo
                {
                    user: get_user(id.clone(), &lock_user_list),
                    rank: 0,
                    scores: vec![0.0 ; contest.problem_ids.len()],

                    highest_scores: vec![0.0 ; contest.problem_ids.len()],
                    latest_scores: vec![0.0 ; contest.problem_ids.len()],

                    competitive_score_sum: 0.0,
                    shortest_times: vec![vec![MAX; 20]; contest.problem_ids.len()], // possible error

                    latest_submission: DateTime::<Utc>::MAX_UTC,
                    score: 0,
                    submission_count: 0,
                }
            )
        }

        let output = contest.clone();
        log::info!("{:?}", output);

        // if SQL mode activated, then load into database
        sql::push_contest(&connection, &lock_contest_list);

        *lock_contest_id_count += 1;

        // if SQL mode activated, then load into database
        sql::push_contest_count(&connection, *lock_contest_id_count);

        // return newly added contest
        return HttpResponse::Ok().json(output);
    }
}