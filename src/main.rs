use actix_web::{middleware::Logger, web, App, HttpServer, Responder, post};
use env_logger;
use log;
use clap::Parser;
use std::i64::MAX;
use std::sync::Arc;
use std::sync::Mutex;
use lazy_static::lazy_static;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

mod user_module;
use crate::user_module::user;
use crate::user_module::function_post_users;
use crate::user_module::function_get_users;

mod contest_module;
use crate::contest_module::contest;
use crate::contest_module::function_get_contests;
use crate::contest_module::function_post_contests;

mod jobs_module;
use crate::jobs_module::job;
use crate::jobs_module::post_job;
use crate::jobs_module::spj;
use crate::jobs_module::function_post_jobs;
use crate::jobs_module::function_get_jobs;
use crate::jobs_module::function_put_jobs;
use crate::jobs_module::compare_functions;

mod others_module;
use crate::others_module::sql;
use crate::others_module::config;
use crate::others_module::parameters;

// defining global variables and vectors mostly used for non-persistent storage (execpt last two)

lazy_static! {
    static ref JOB_ID_COUNT: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    static ref JOB_LIST: Arc<Mutex<Vec<job::ResponseContent>>> = Arc::new(Mutex::new(Vec::new()));

    static ref USER_ID_COUNT: Arc<Mutex<u32>> = Arc::new(Mutex::new(1)); // user 0 is root

    static ref USER_LIST: Arc<Mutex<Vec<user::User>>> = Arc::new(Mutex::new(vec![ // root user is added by default
        user::User {
        id: 0,
        name: "root".to_string(),
      }]));

    static ref CONTEST_ID_COUNT: Arc<Mutex<u32>> = Arc::new(Mutex::new(1)); // contest 0 has special use

    // contest 0 is added by default
    // specific values are added in main()
    static ref CONTEST_LIST: Arc<Mutex<Vec<contest::Contest>>> = Arc::new(Mutex::new(vec![
        contest::Contest {
        id: 0,
        name: "root".to_string(),
        from: DateTime::<Utc>::MAX_UTC,
        to: DateTime::<Utc>::MIN_UTC,
        problem_ids: vec![],
        user_ids: vec![0],
        submission_limit: 0,

        users: vec![],
    }
    ]));

    // stores the shortest execution times for each case of each problem
    // used in dynamic ranking
    static ref CASE_TIMES_LIST: Arc<Mutex<Vec<Vec<i64>>>> = Arc::new(Mutex::new(Vec::new()));

    // signals if storage should be persistent
    static ref IS_SQL: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    // establishes a connection to the database
    static ref CONNECTION: Arc<Mutex<Connection>> = Arc::new(Mutex::new(Connection::open("data.db").expect("database not loaded")));
}

// DO NOT REMOVE: used in automatic testing
#[post("/internal/exit")]
#[allow(unreachable_code)]
async fn exit() -> impl Responder {
    log::info!("Shutdown as requested");
    std::process::exit(0);
    format!("Exited")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // obtains information from cli arguments parser
    let cli = parameters::Cli::parse();

    // checks if config file was provided
    let filename: String;
    if let Some(valid_filename) = cli.config
    {
        filename = valid_filename;
    }
    else {
        panic!("no config file found")
    }

    // opens the config file and stores its contents in a variable of type Config
    let config: config::Config;
    match config::load(&filename)
    {
        Ok(valid_config) => 
        {
            config = valid_config;
        }    
        Err(_) => panic!("config file could not be loaded")
    }

    // checks if persistent storage in SQL should be user
    // cli.storage == true iff persistent storage mode is on
    if cli.storage
    {
        // setting global variable to true
        // which signals to all the modules that persistent storage is on
        {
            let mut is_sql = IS_SQL.lock().unwrap();
            *is_sql = true;
        }
        
        // START SQL CONNECTION
        let connection = CONNECTION.lock().unwrap();

        // initializing the database
        sql::initialize(&connection).expect("failed to initialize database");

        // clears the database and inserts all default values if --reset_storage
        if cli.reset_storage
        {
            sql::clear(&connection).expect("failed to clear database");

            sql::insert_default_user_id_count(&connection).expect("failed to insert default value into database");
            sql::insert_default_user_list(&connection).expect("failed to insert default value into database");

            sql::insert_default_job_id_count(&connection).expect("failed to insert default value into database");

            sql::insert_default_contest_id_count(&connection).expect("failed to insert default value into database");
            sql::insert_default_contest(&connection, &config).expect("failed to insert default value into database");
        }

        // // For testing:

        // log::info!("Retrieving list");

        // let mut lock_job_list = JOB_LIST.lock().unwrap();

        // sql::get_job(&connection, &mut lock_job_list);

        // log::info!("Before:");
        // for x in lock_job_list.clone()
        // {
        //     log::info!("{:?}", x);
        // }

        // lock_job_list.push(job::ResponseContent {
        //     id: 90999999,
        //     created_time: Utc::now(),
        //     updated_time: Utc::now(),
        //     submission: PostJob {source_code: "".to_string(), language: "Rust".to_string(), user_id: 0, contest_id: 0, problem_id: 0},
        //     state: "finished".to_string(),
        //     result: job::PossibleResult::Accepted,
        //     score: 0.0,
        //     cases: vec![]
        // });

        // log::info!("Updated:");
        // for x in lock_job_list.clone()
        // {
        //     log::info!("{:?}", x);
        // }

        // sql::push_job(&connection, &lock_job_list);

    }
    
    // START setup contest 0
    {
        let mut lock_contest_list: std::sync::MutexGuard<Vec<contest::Contest>> = CONTEST_LIST.lock().unwrap();
        
        // fills in all the problem ids (which are provided by the config file)
        for i in 0..config.problems.len()
        {
            lock_contest_list[0].problem_ids.push(i as u32);
        }

        // inserts the root user (id = 0) into the list of users 
        // other users are inserted as they are posted
        lock_contest_list[0].users.push(
            contest::RankInfo
            {
                user: user::User {
                    id: 0,
                    name: "root".to_string(),
                    },
                scores: vec![0.0; config.problems.len()],
                rank: 0,
                
                highest_scores: vec![0.0 ; config.problems.len()],
                latest_scores: vec![0.0 ; config.problems.len()],

                competitive_score_sum: 0.0,

                // assuming each problem has a maximum of 20 test cases
                // can be changed if needed
                shortest_times: vec![vec![MAX; 20]; config.problems.len()], 
                
                latest_submission: DateTime::<Utc>::MAX_UTC,
                score: 0,
                submission_count: 0,
            }
        );
    }
    // END setup contest 0

    // START setup case times list
    // to keep track of shortest submission times for dynamic ranking
    {
        let mut lock_case_list: std::sync::MutexGuard<Vec<Vec<i64>>> = CASE_TIMES_LIST.lock().unwrap();

        for problem in &config.problems
        {
            let mut cases: Vec<i64> = vec![];
            for _case in &problem.cases
            {
                // each case time (in microseconds) is set to max i64 at the beginning
                cases.push(MAX);
            }
            lock_case_list.push(cases.clone());
        }
    }
    // END setup case times list

    // passing in the config file so functions have access to it
    let config_arc = Arc::new(Mutex::new(config));

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/hello", web::get().to(|| async { "Hello World!" }))
            .service(function_post_jobs::greet)
            // DO NOT REMOVE: used in automatic testing
            .service(exit)
            
            .service(function_get_jobs::get_jobs)
            .service(function_get_jobs::get_jobs_jobId)
            .service(function_post_jobs::post_jobs)
            .service(function_put_jobs::get_jobs_jobId)

            .service(function_post_users::post_users)
            .service(function_get_users::get_users)

            .service(function_post_contests::post_contests)
            .service(function_get_contests::get_contests)
            .service(function_get_contests::get_contests_contestId)
            .service(function_get_contests::get_contests_contestId_ranklist)

            .app_data(web::Data::new(config_arc.clone()))
        
    })
    .bind(("127.0.0.1", 12345))?
    .run().await

}
