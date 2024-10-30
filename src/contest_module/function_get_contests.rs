use actix_web::{get, Responder, HttpResponse, web};
use serde::Deserialize;
use std::sync::Mutex;
use std::sync::Arc;

use crate::config;
use crate::contest;
use crate::job;
use crate::CASE_TIMES_LIST;
use crate::CONTEST_LIST;

use crate::CONNECTION;
use crate::sql;

// gets list of all contests
#[get("/contests")]
async fn get_contests() -> impl Responder
{
    let connection = CONNECTION.lock().unwrap();
    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();

    // if sql storage mode is on, then load info from the database
    sql::get_contest(&connection, &mut lock_contest_list);

    // retrieves each contest and stores into a vector, which is then returned
    let mut list: Vec<contest::Contest> = vec![];
    for i in 1..lock_contest_list.len()
    {
        list.push(lock_contest_list[i].clone());
    }
    return HttpResponse::Ok().json(list);
}

// gets information about a contest given its id
#[get("/contests/{contestId}")]
#[allow(non_snake_case)]
async fn get_contests_contestId(contestId: web::Path<u32>) -> impl Responder 
{
    let connection = CONNECTION.lock().unwrap();
    // contest cannot be 0
    // if id is 0 then return error
    if contestId.to_owned() == 0
    {
        return HttpResponse::BadRequest().json(job::Error::
            new(1, "ERR_INVALID_ARGUMENT".to_string(), "Invalid contest id".to_string()));
    }

    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();

    // if sql storage mode is on, then load info from the database
    sql::get_contest(&connection, &mut lock_contest_list);

    // if a contest matches the provided id, then return it
    for contest in lock_contest_list.iter()
    {
        if contest.id == contestId.to_owned()
        {
            return HttpResponse::Ok().json(contest.clone());
        }
    }

    // if no contest with such id is found, return error
    return HttpResponse::NotFound().json(job::Error::
        new(3, "ERR_NOT_FOUND".to_string(), format!("Contest {} not found.", contestId.to_owned()).to_string()));
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[allow(non_camel_case_types)]
enum ScoringRule
{
    latest,
    highest,
}
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[allow(non_camel_case_types)]
enum TieBreaker
{
    submission_time,
    submission_count,
    user_id,
    none,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct AuthRequest // from https://docs.rs/actix-web/latest/actix_web/web/struct.Query.html
{
    scoring_rule: Option<ScoringRule>,
    tie_breaker: Option<TieBreaker>,
}

// gets the total sum of all problems scores for a user
// returns u32 form
fn get_sum(scores: &Vec<f32>) -> u32
{
    let mut sum: f32 = 0.0;
    for score in scores
    {
        sum += score;
    }
    return sum as u32;
}

// used for dynamic ranking mode
// gets the competitve sum score of each user in a competition 
// based on its personal time compared to the best time in each case
fn get_competitive_sum(personal_list: &Vec<Vec<i64>>, problems: &Vec<config::Problem>, scores: &mut Vec<f32>) -> u32
{
    let mut competitive_sum = 0;

    let lock_times_list = CASE_TIMES_LIST.lock().unwrap();

    // loops through each problem and each case
    for i in 0..lock_times_list.len()
    {
        // only sums if dynamic ranking mode is on   
        if let Some(competitive_ratio) = problems[i].misc.dynamic_ranking_ratio
        {
            let mut problem_competitive_sum: f32 = 0.0;
            for j in 0..lock_times_list[i].len()
            {
                // calculates competitive scores for each case and adds to total sum
                let case_score = problems[i].cases[j].score;

                let ratio = lock_times_list[i][j] as f32 / personal_list[i][j] as f32; 
                competitive_sum += (case_score * competitive_ratio * ratio) as u32;
                problem_competitive_sum += case_score * competitive_ratio * ratio;
            }
            // also updates information to user scores vector
            scores[i] += problem_competitive_sum;
        }
    }
    return competitive_sum;
}

// gets the ranklist for a selected contest
#[get("/contests/{contestId}/ranklist")]
#[allow(non_snake_case)]
async fn get_contests_contestId_ranklist(contestId: web::Path<u32>, info: web::Query<AuthRequest>, data_config: web::Data<Arc<Mutex<config::Config>>>) -> impl Responder 
{   
    let config = data_config.lock().unwrap();
    let connection = CONNECTION.lock().unwrap();
    
    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();

    // if sql storage mode is on, then load info from the database
    sql::get_contest(&connection, &mut lock_contest_list);

    // return error if contest is not found
    if contestId.to_owned() as usize >= lock_contest_list.len()
    {
        return HttpResponse::NotFound().json(job::Error::
            new(3, "ERR_NOT_FOUND".to_string(), format!("Contest {} not found.", contestId.to_owned()).to_string()));
    }

    // obtains scoring rule and tiebreaker from query 
    let mut scoring_rule = ScoringRule::latest; // latest set as default
    let mut tie_breaker = TieBreaker::none;

    // update if information provided (otherwise keep default)
    if let Some(new_scoring_rule) = info.scoring_rule.clone()
    {
        scoring_rule = new_scoring_rule;
    }
    if let Some(new_tie_breaker) = info.tie_breaker.clone()
    {
        tie_breaker = new_tie_breaker;
    }

    let contest = &mut lock_contest_list[(contestId.to_owned()) as usize];

    // update score sums

    if scoring_rule == ScoringRule::highest
    {
        for user in &mut contest.users
        {
            user.score = get_sum(&user.highest_scores); // using highest_scores to calculate
            user.scores = user.highest_scores.clone();
            user.score += get_competitive_sum(&user.shortest_times, &config.problems, &mut user.scores);
        }
    }
    else
    {
        for user in &mut contest.users
        {
            user.score = get_sum(&user.latest_scores); // using latest_scores to calculate
            user.scores = user.latest_scores.clone();
            user.score += get_competitive_sum(&user.shortest_times, &config.problems, &mut user.scores);
        }
    }
    
    // sorts by score, then latest submission, then user id
    if tie_breaker == TieBreaker::submission_time
    {
        contest.users.sort_by(|a, b| { 
            b.score.cmp(&a.score)
            .then_with(|| a.latest_submission.cmp(&b.latest_submission))
            .then_with(|| a.user.id.cmp(&b.user.id))
        });

        // give a rank number to each user
        // if tied then give same rank
        contest.users[0].rank = 1;
        for i in 1..contest.users.len()
        {
            if contest.users[i].score == contest.users[i-1].score && 
            contest.users[i].latest_submission == contest.users[i-1].latest_submission
            {
                contest.users[i].rank = contest.users[i-1].rank
            }
            else {
                contest.users[i].rank = i as u32 + 1;
            }
        }

    }
    // sorets by score, then submission count, then user id
    else if tie_breaker == TieBreaker::submission_count 
    {
        contest.users.sort_by(|a, b| {
            b.score.cmp(&a.score)
            .then_with(|| a.submission_count.cmp(&b.submission_count))
            .then_with(|| a.user.id.cmp(&b.user.id))
        });

        // give a rank number to each user
        // if tied then give same rank
        contest.users[0].rank = 1;
        for i in 1..contest.users.len()
        {
            if contest.users[i].score == contest.users[i-1].score && 
            contest.users[i].submission_count == contest.users[i-1].submission_count
            {
                contest.users[i].rank = contest.users[i-1].rank
            }
            else {
                contest.users[i].rank = i as u32 + 1;
            }
        }
    }
    // sorts by score, then user id
    else if tie_breaker == TieBreaker::user_id 
    {
        contest.users.sort_by(|a, b| {
            b.score.cmp(&a.score)
            .then_with(|| a.user.id.cmp(&b.user.id))
        });

        // no need to account for ties here because each user id is unique
        for i in 0..contest.users.len()
        {
            contest.users[i].rank = i as u32 + 1;
        }
    }
    // sorts by score, then user id
    else 
    {
        contest.users.sort_by(|a, b| {
            b.score.cmp(&a.score)
            .then_with(|| a.user.id.cmp(&b.user.id))
        });
        
        // give a rank number to each user
        // if tied then give same rank
        contest.users[0].rank = 1;
        for i in 1..contest.users.len()
        {
            if contest.users[i].score == contest.users[i-1].score
            {
                contest.users[i].rank = contest.users[i-1].rank
            }
            else {
                contest.users[i].rank = i as u32 + 1;
            }
        }
    }

    log::info!("{:?}", contest.users.clone());

    return HttpResponse::Ok().json(contest.users.clone());
}