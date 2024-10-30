use actix_web::{get, post, web, Responder, HttpResponse};
use std::fs::File;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use wait_timeout::ChildExt;
use chrono::Utc;

use crate::post_job;
use crate::job;
use crate::user;
use crate::config;
use crate::Arc;
use crate::Mutex;
use crate::compare_functions;
use crate::contest;
use crate::spj;
use crate::sql;
use crate::Connection;

use crate::JOB_LIST;
use crate::JOB_ID_COUNT;

use crate::USER_LIST;

use crate::CONTEST_LIST;

use crate::CASE_TIMES_LIST;

use crate::CONNECTION;

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    log::info!(target: "greet_handler", "Greeting {}", name);
    format!("Hello {name}!")
}

// returns the index of a problem in a vector given its id
fn get_problem_index(problems: &Vec<config::Problem>, problem_id: u32) -> usize
{
    for i in 0..problems.len()
    {
        if problems[i].id == problem_id
        {
            return i;
        }
    }
    return 0;
}

// returns the index of a language in a vector given its name
fn get_language_index(languages: &Vec<config::Language>, language_name: String) -> usize
{
    for i in 0..languages.len()
    {
        if languages[i].name == language_name
        {
            return i;
        }
    }
    return 0;
}

// runs an individual testcase
fn run_test_case(job_result: &mut job::PossibleResult, case_result: &mut job::PossibleResult, case_info: &mut String,
    case: &config::Case, config: &config::Config, problem_index: usize, case_index: usize, score_sum: &mut f32, 
    body: &post_job::PostJob, time: &mut i64, connection: &Connection)
{

    // opens the file where input is obtained from
    let in_file = match File::open(case.input_file.clone()) 
    {
        Ok(file) => file,
        Err(_) => 
        {
            panic!("input file not found"); // TODO
        }
    };

    // creates and opens temporary file where results stored
    let out_file = match File::create("TMPDIR/test.out") 
    {
        Ok(file) => file,
        Err(_) => 
        {
            panic!("output file not created"); // TODO
        }
    };

    // starts keeping track of execution time for dynamic ranking mode
    let start_time = Utc::now().naive_utc();

    // start running test case
    let mut child: std::process::Child = Command::new("TMPDIR/test.exe".to_string())
        .stdin(Stdio::from(in_file))
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::piped())
        .spawn()
        .expect("Error child");
    
    // keep track of timeout limit
    let timeout = Duration::from_micros(case.time_limit);

    // check that runtime is within timeout limit
    match child.wait_timeout(timeout).unwrap() 
    {
        
        Some(status) => 
        {
            // if runtime error, then set the case and job results to RuntimeError
            if !status.success()
            {
                *case_result = job::PossibleResult::RuntimeError;
                *job_result = job::PossibleResult::RuntimeError;
            }
            else 
            {
                // finish keeping track of execution time
                let end_time = Utc::now().naive_utc();

                // open answer for this testcase
                let ans_file = match File::open(case.answer_file.clone())
                {
                    Ok(file) => file,
                    Err(_) => {
                        panic!("missing case data");
                    }
                };

                // open output
                let out_file2 = match File::open("TMPDIR/test.out")
                {
                    Ok(file) => file,
                    Err(_) => 
                    {
                        panic!("could not create file");
                    }
                };
                
                let mut accepted: bool = false;
                let mut info: String = "".to_string();
                let mut spj_error: bool = false;
                let mut correntness_ratio: f32 = 1.0;

                // select type of compare and compare answer with output to yield result for the case
                if *&config.problems[problem_index].ty == config::ProblemType::standard
                {
                    accepted = compare_functions::compare_standard(out_file2, ans_file);
                }
                else if *&config.problems[problem_index].ty == config::ProblemType::strict
                {
                    accepted = compare_functions::compare_strict(out_file2, ans_file);
                }
                else if *&config.problems[problem_index].ty == config::ProblemType::spj
                {
                    // see this compare function in spj file
                    (accepted, info, spj_error) = spj::compare_spj("TMPDIR/test.out".to_string(), 
                    case.answer_file.clone(), &config, problem_index);
                    match std::fs::remove_dir_all("SPJDIR") 
                    {
                        Ok(_) => {}
                        Err(_) => {} // TODO
                    }
                }
                else if *&config.problems[problem_index].ty == config::ProblemType::dynamic_ranking
                {
                    if let Some(competitive_ratio) = config.problems[problem_index].misc.dynamic_ranking_ratio
                    {
                        // calculates first component of score
                        correntness_ratio = 1.0 - competitive_ratio;
                        accepted = compare_functions::compare_standard(out_file2, ans_file);

                        let time_used = end_time-start_time;

                        let mut lock_case_list: std::sync::MutexGuard<Vec<Vec<i64>>> = CASE_TIMES_LIST.lock().unwrap();
                        
                        // calculates second component of score
                        if let Some(time_used_micros) = time_used.num_microseconds()
                        {
                            *time = time_used_micros;
                            
                            // UPDATE GLOBAL TIME
                            
                            if time_used_micros < lock_case_list[problem_index][case_index]
                            {
                                lock_case_list[problem_index][case_index] = time_used_micros;
                            }

                            // END UPDATE GLOBAL BEST TIME

                            // UPDATE USER PERSONAL BEST TIME

                            let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
                            sql::get_contest(&connection, &mut lock_contest_list);
                            
                            let contest = &mut lock_contest_list[body.contest_id as usize];

                            for user in &mut contest.users
                            {
                                if user.user.id == body.user_id
                                {
                                    log::info!("Time used by {}: {}", user.user.id, time_used_micros);
                                    
                                    if time_used_micros < user.shortest_times[problem_index][case_index]
                                    {
                                        user.shortest_times[problem_index][case_index] = time_used_micros;
                                    }
                                }
                            }

                            sql::push_contest(&connection, &mut lock_contest_list);

                            // END UPDATE USER TIME
    
                        }
                        else  {
                            panic!("Overflow occured when dealing with time");
                        }
                    }
                    else 
                    {
                        panic!("dynamic_ranking mode selected but no ratio was provided");
                    }
                }
                if spj_error 
                {
                    *case_result = job::PossibleResult::SPJError;
                }
                else {
                    if accepted {
                        *case_result = job::PossibleResult::Accepted;
                        *case_info = info;
                        *score_sum += case.score * correntness_ratio;
                    }
                    else
                    {
                        *case_info = info;
                        *case_result = job::PossibleResult::WrongAnswer;
                    }
                }
            }
        }
        None => 
        {
            child.kill().unwrap();
            child.wait().unwrap();
            // log::info!("Time limit exceeded");
            *job_result = job::PossibleResult::TimeLimitExceeded;
            *case_result = job::PossibleResult::TimeLimitExceeded;
        }
    }
}

// function runs a job
pub fn post_jobs_action(body: web::Json<post_job::PostJob>, data_config: web::Data<Arc<Mutex<config::Config>>>, 
    is_put: bool, connection: &Connection) -> job::ResponseContent {

    // keep track of when job was started
    let created_time = Utc::now();
    
    let mut results: Vec<job::Case> = vec![];
    let mut score_sum: f32 = 0.0;
    
    // creating temporary directory for problem
    match std::fs::create_dir("TMPDIR") 
    {
        Ok(_) => {}
        Err(_) => {}
    }

    let mut config = data_config.lock().unwrap();

    let language_index = get_language_index(&config.languages, body.language.clone());
    let file_name = &config.languages[language_index].get_file_name();

    // writing source code into file
    match std::fs::write("TMPDIR/".to_owned() + file_name, body.source_code.clone())
    {
        Ok(_) => {}
        Err(_) => {} // TODO
    }

    // obrains the command to be executed to run the code
    let mut commands: Vec<String> = vec![];
    for i in &config.languages[language_index].command.clone()
    {
        if i == &config.languages[language_index].command[0] {continue;} // ignore first element
        else if i == "%INPUT%" {commands.push("TMPDIR/".to_owned() + file_name);}
        else if i == "%OUTPUT%" {commands.push("TMPDIR/test.exe".to_string());}
        else {commands.push(i.clone());}
    }
    
    let mut job_result: job::PossibleResult;

    // uses commands to compile the source code
    match Command::new(&config.languages[language_index].command[0])
                .args(commands)
                .stderr(Stdio::null())
                .status()
    {
        Ok(status) if status.success() => {
            job_result = job::PossibleResult::CompilationSuccess;
            results.push(job::Case {
                id: 0,
                result: job::PossibleResult::CompilationSuccess,
                info: "".to_string(),
                time: 0
            })
        }
        _ =>
        {
            job_result = job::PossibleResult::CompilationError;
            results.push(job::Case {
                id: 0,
                result: job::PossibleResult::CompilationError,
                info: "".to_string(),
                time: 0
            });
        }
    }

    let mut case_id = 1;
    let problem_index = get_problem_index(&config.problems, body.problem_id);

    // if packing mode is on then run cases pack by pack
    if let Some(packing) = config.problems[problem_index].misc.packing.clone()
    {
        // loops through each pack
        for i in 0..packing.len()
        {
            let mut accept = true;
            // loops through each case in a pack
            for j in 0..packing[i].len()
            {
                // if a case in the pack is failed, then the rest are skipped
                if accept == false
                {
                    results.push(job::Case {
                        id: packing[i][j],
                        result: job::PossibleResult::Skipped,
                        info: "".to_string(),
                        time: 0,
                    });
                    continue;
                }
                
                let mut case_result: job::PossibleResult = job::PossibleResult::Waiting;

                if job_result == job::PossibleResult::CompilationError 
                {
                    results.push(job::Case {
                        id: packing[i][j],
                        result: case_result,
                        info: "".to_string(),
                        time: 0,
                    });
                    continue;
                }

                let mut info: String = "".to_string();
                let mut time: i64 = 0;

                // runs the test case and stores the information into variables
                run_test_case(&mut job_result, &mut case_result, &mut info,
                    &config.problems[problem_index].cases[packing[i][j] as usize -1], 
                    &config, problem_index, packing[i][j] as usize -1, &mut score_sum, 
                    &body, &mut time, &connection
                );

                if case_result != job::PossibleResult::Accepted
                {
                    accept = false;
                }
                
                // stores the result into vector
                results.push(job::Case {
                    id: packing[i][j],
                    result: case_result,
                    info: info,
                    time: time
                });

            }
        }
    }
    // if not packing mode, then run each test case in linear order
    else 
    {
        for case in &config.problems[problem_index].cases 
        {
            
            let mut case_result: job::PossibleResult = job::PossibleResult::Waiting;

            if job_result == job::PossibleResult::CompilationError 
            {
                results.push(job::Case {
                    id: case_id,
                    result: case_result,
                    info: "".to_string(),
                    time: 0
                });
                case_id += 1;
                continue;
            }

            let mut info: String = "".to_string();
            let mut time: i64 = 0;

            // runs test case and store information into variables

            run_test_case(&mut job_result, &mut case_result, &mut info, case, 
                &config, problem_index, case_id as usize -1, &mut score_sum,
                 &body, &mut time, &connection
            );
            
            // pushes result into vector

            results.push(job::Case {
                id: case_id,
                result: case_result,
                info: info,
                time: time
            });
            case_id += 1;
        }
    }

    // AFTER ALL TESTCASES HAVE BEEN RUN:

    let state_str = "Finished";
    
    // only calculate exact results if timelimit was not exceeded, and there were no other errors
    if job_result == job::PossibleResult::TimeLimitExceeded {}
    else if job_result == job::PossibleResult::RuntimeError {}
    else if job_result == job::PossibleResult::CompilationError {}
    else
    {
        // in dynamic_ranking mode, we have to account fot the fact that the maximum possible
        // score is 100.0 * correntness_ratio
        if config.problems[problem_index].ty == config::ProblemType::dynamic_ranking
        {
            
            if let Some(competitive_ratio) = config.problems[problem_index].misc.dynamic_ranking_ratio
            {
                let correntness_ratio = 1.0 - competitive_ratio;
                if score_sum == 100.0 * correntness_ratio
                {
                    job_result = job::PossibleResult::Accepted;
                }
                else if score_sum < 100.0* correntness_ratio
                {
                    job_result = job::PossibleResult::WrongAnswer;
                }
            }
            else {
                panic!("dynamic_ranking selected but no ratio provided")
            }
        }
        // otherwise, no need to account for correntness_ratio
        else 
        {
            if score_sum == 100.0
            {
                job_result = job::PossibleResult::Accepted;
            }
            else if score_sum < 100.0
            {
                job_result = job::PossibleResult::WrongAnswer;
            }   
        }
    }

    // delete temporary directory
    match std::fs::remove_dir_all("TMPDIR") 
    {
        Ok(_) => {}
        Err(_) => {} // TODO
    }

    let updated_time = Utc::now();

    let mut lock_job_id_count = JOB_ID_COUNT.lock().unwrap();
    // if SQL mode activated, then load information from database
    sql::get_job_count(&connection, &mut lock_job_id_count);
    
    // store information
    let job = job::Job 
    {
        poll_for_job: true,
        request: job::Request
        {
            path: "jobs".to_string(),
            method: "POST".to_string(),
            content: post_job::PostJob 
            {
                source_code: body.source_code.clone(),
                language: body.language.clone(),
                user_id: body.user_id.clone(),
                problem_id: body.problem_id.clone(),
                contest_id: body.contest_id.clone(),
            },
        },
        response: job::Response
        {
            status: 200,
            content: job::ResponseContent
            {
                id: *lock_job_id_count,
                created_time,
                updated_time,
                submission: post_job::PostJob 
                {
                    source_code: body.source_code.clone(),
                    language: body.language.clone(),
                    user_id: body.user_id.clone(),
                    problem_id: body.problem_id.clone(),
                    contest_id: body.contest_id.clone(),
                },
                state: state_str.to_string(),
                result: job_result,
                score: score_sum,
                cases: results,
            },
        },
        // restart_server: false,
    };

    // if the HTTP request is of type PUT, then do not count this as a new job
    if !is_put
    {
        *lock_job_id_count +=1 ;
        let mut lock_job_list = JOB_LIST.lock().unwrap();
        lock_job_list.push(job.response.content.clone());
        // if SQL mode activated, then load information into database
        sql::push_job(&connection, &lock_job_list);
    }

    // START update contest information
    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
    // if SQL mode activated, then load information from database
    sql::get_contest(&connection, &mut lock_contest_list);

    let contest = &mut lock_contest_list[body.contest_id as usize];

    // update result for user
    let mut problem_index: usize = 0;
    for i in 0..contest.problem_ids.len()
    {
        if contest.problem_ids[i] == body.problem_id
        {
            problem_index = i;
        }
    }

    // update result for user
    for rank_info in contest.users.iter_mut()
    {
        if rank_info.user.id == body.user_id
        {
            rank_info.latest_scores[problem_index] = score_sum;

            if score_sum >= rank_info.highest_scores[problem_index]
            {
                rank_info.highest_scores[problem_index] = score_sum;
                rank_info.latest_submission = created_time;
            }

            if !is_put
            {
                rank_info.submission_count += 1;
            }

            rank_info.competitive_score_sum;
        }
    }
    // if SQL mode activated, then load information into database
    sql::push_contest(&connection, &mut lock_contest_list);

    // END update contest information

    // if SQL mode activated, then load information into database
    sql::push_job_count(&connection, *lock_job_id_count);
    

    return job.response.content;
}

// checks if user exists
fn exists_user(user_id: u32, connection: &Connection) -> bool
{
    let mut lock_user_list: std::sync::MutexGuard<Vec<user::User>> = USER_LIST.lock().unwrap();
    sql::get_user(&connection, &mut lock_user_list);
    for user in lock_user_list.iter()
    {
        if user.id == user_id 
        {
            return true;
        }
    }
    return false;
}

// checks if user is in the contest
fn check_user_in_contest(body: &web::Json<post_job::PostJob>, connection: &Connection) -> bool
{
    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
    sql::get_contest(&connection, &mut lock_contest_list);

    let user_ids = lock_contest_list[body.contest_id as usize].user_ids.clone();
    for id in user_ids
    {
        if body.user_id == id {return true;}
    }
    return false;
}

// checks if problem is in the contest
fn check_problem_in_contest(body: &web::Json<post_job::PostJob>, connection: &Connection) -> bool
{
    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
    sql::get_contest(&connection, &mut lock_contest_list);
    let problem_ids = lock_contest_list[body.contest_id as usize].problem_ids.clone();
    for id in problem_ids
    {
        if body.problem_id == id {return true;}
    }
    return false;
}

// checks if the contest has started yet
fn check_valid_start(body: &web::Json<post_job::PostJob>, connection: &Connection) -> bool
{
    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
    sql::get_contest(&connection, &mut lock_contest_list);
    let start_time = lock_contest_list[body.contest_id as usize].from.clone();
    if Utc::now() < start_time {return false;}
    return true;
}

// checks that the contest has not ended yet
fn check_valid_end(body: &web::Json<post_job::PostJob>, connection: &Connection) -> bool
{
    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
    sql::get_contest(&connection, &mut lock_contest_list);
    let end_time = lock_contest_list[body.contest_id as usize].to.clone();
    if Utc::now() > end_time {return false;}
    return true;
}

// checks that the user has not passed the submission limit for the contest
fn check_submission_limit(body: &web::Json<post_job::PostJob>, connection: &Connection) -> bool
{
    let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
    sql::get_contest(&connection, &mut lock_contest_list);
    
    let limit = lock_contest_list[body.contest_id as usize].submission_limit.clone();
    if limit == 0 {return true;}

    for user in &lock_contest_list[body.contest_id as usize].users
    {
        if body.user_id == user.user.id 
        {
            if user.submission_count < limit
            {
                return true;
            }
            else {
                return false;
            }
        }
    }
    return false;
}

// posts a new job
#[post("/jobs")]
async fn post_jobs(body: web::Json<post_job::PostJob>, data_config: web::Data<Arc<Mutex<config::Config>>>) -> impl Responder 
{
    let connection = CONNECTION.lock().unwrap();
    // START CHECK VALID CONDITIONS
    if !exists_user(body.user_id, &connection) 
    {
        return HttpResponse::NotFound().json(job::Error {
                code: 3,
                reason: "ERR_NOT_FOUND".to_string(),
                message: "HTTP 404 Not Found".to_string(),
            });
    }
    if body.contest_id != 0
    {
        if !check_user_in_contest(&body, &connection) || !check_problem_in_contest(&body, &connection) ||
        !check_valid_start(&body, &connection) || !check_valid_end(&body, &connection)
        {
            return HttpResponse::BadRequest().json(job::Error {
                code: 1,
                reason: "ERR_INVALID_ARGUMENT".to_string(),
                message: "HTTP 400 Bad Request".to_string(),
            });
        }
        if !check_submission_limit(&body, &connection)
        {
            return HttpResponse::BadRequest().json(job::Error {
                code: 4,
                reason: "ERR_RATE_LIMIT".to_string(),
                message: "HTTP 400 Bad Request".to_string(),
            });
        }
    }
    // END CHECK VALID CONDITIONS
    return HttpResponse::Ok().json(post_jobs_action(body, data_config, false, &connection));
}    