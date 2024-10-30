use std::i64::MAX;

use rusqlite::{params, Connection, Result};
use chrono::Utc;
use chrono::DateTime;

use crate::user::User;
use crate::contest::Contest;
use crate::contest;
use crate::config;

use crate::job::ResponseContent;
use crate::job::PossibleResult;

use crate::sql;
use crate::IS_SQL;

// initializes database
// creates tables to store each variable/list in its required format

pub fn initialize(conn: &Connection) -> Result<()> 
{
    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_id_count (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_list (
             id INTEGER PRIMARY KEY,
             name TEXT NOT NULL
         )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS job_id_count (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS response_content (
             id INTEGER PRIMARY KEY,
             created_time TEXT NOT NULL,
             updated_time TEXT NOT NULL,
             submission TEXT NOT NULL,
             state TEXT NOT NULL,
             result TEXT NOT NULL,
             score REAL NOT NULL,
             cases TEXT NOT NULL
         )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS contest_id_count (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS contest (
             id INTEGER PRIMARY KEY,
             name TEXT NOT NULL,
             from_time TEXT NOT NULL,
             to_time TEXT NOT NULL,
             problem_ids TEXT NOT NULL,
             user_ids TEXT NOT NULL,
             submission_limit INTEGER NOT NULL,
             users TEXT NOT NULL
         )",
        [],
    )?;

    Ok(())
}

// clears the content in the database 
// used when the reset option is activated

pub fn clear(conn: &Connection) -> Result<()> 
{
    conn.execute("DELETE FROM user_id_count", [])?;
    conn.execute("DELETE FROM user_list", [])?;

    conn.execute("DELETE FROM job_id_count", [])?;
    conn.execute("DELETE FROM response_content", [])?;

    conn.execute("DELETE FROM contest_id_count", [])?;
    conn.execute("DELETE FROM contest", [])?;
    Ok(())
}

// USER ID COUNT

// function first checks if SQL storage is activated, then stores the count
pub fn push_user_count(connection: &Connection, count: u32)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        store_user_id_count(&connection, count).expect("failed to store user id count");
    }
}

// actually performing the action of storing into the database
pub fn store_user_id_count(conn: &Connection, count: u32) -> Result<()> 
{
    let count_str = count.to_string();
    conn.execute(
        "INSERT OR REPLACE INTO user_id_count (key, value) VALUES (?1, ?2)",
        params!["USER_ID_COUNT", count_str],
    )?;
    Ok(())
}

// checks if SQL storage is activated
// if yes, then it retrieves the count
pub fn get_user_count(connection: &Connection, count: &mut u32)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        *count = match sql::retrieve_user_id_count(&connection)
        {
            Ok(x) => x,
            Err(_) => {panic!("unsuccessful retrieve");}
        };
    }
}

// retrieves the user id count from the database
pub fn retrieve_user_id_count(conn: &Connection) -> Result<u32> 
{
    let mut stmt = conn.prepare("SELECT value FROM user_id_count WHERE key = ?1")?;
    let count_str: String = stmt.query_row(params!["USER_ID_COUNT"], |row| row.get(0))?;
    let count: u32 = count_str.parse().unwrap_or(1);
    Ok(count)
}

// inserts the default value into the database
// only called when reset mode is on
pub fn insert_default_user_id_count(conn: &Connection) -> Result<()> 
{
    conn.execute(
        "INSERT OR REPLACE INTO user_id_count (key, value) VALUES (?1, ?2)",
        params!["USER_ID_COUNT", 1],
    )?;
    Ok(())
}

// USER LIST

// checks if SQL storage mode is activated
// is yes, then stores the user list
pub fn push_user(connection: &Connection, list: &Vec<User>)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        store_user_list(&connection, &list).expect("failed to store user list");
    }
}

// first clears the original list from the database
// then store the new list into the database
pub fn store_user_list(conn: &Connection, user_list: &[User]) -> Result<()> 
{
    conn.execute("DELETE FROM user_list", [])?;
    for user in user_list {
        conn.execute(
            "INSERT INTO user_list (id, name) VALUES (?1, ?2)",
            params![user.id, user.name],
        )?;
    }
    Ok(())
}

// checks is SQL storage is on
// if yes, then it retrieves the user list
pub fn get_user(connection: &Connection, lock_user_list: &mut Vec<User>)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        *lock_user_list = match sql::retrieve_user_list(&connection)
        {
            Ok(x) => x,
            Err(_) => {panic!("unsuccessful retrieve");}
        };
    }
}

// retrieves the user list from the database
pub fn retrieve_user_list(conn: &Connection) -> Result<Vec<User>> 
{
    let mut stmt = conn.prepare("SELECT id, name FROM user_list")?;
    let user_iter = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;

    let mut users = Vec::new();
    for user in user_iter {
        users.push(user?);
    }
    Ok(users)
}

// inserts the default user list
// only constist of the root use
// called when reset mode is on
pub fn insert_default_user_list(conn: &Connection) -> Result<()> 
{
    conn.execute(
        "INSERT INTO user_list (id, name) VALUES (?1, ?2)",
        params![0, "root".to_string()],
    )?;
    Ok(())
}

// JOB COUNT

// checks if SQL storage is on
// if yes, then stores the job id count
pub fn push_job_count(connection: &Connection, count: u32)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        store_job_id_count(&connection, count).expect("failed to store job id count");
    }
}

// stores the job id count into the database
pub fn store_job_id_count(conn: &Connection, count: u32) -> Result<()> 
{
    let count_str = count.to_string();
    conn.execute(
        "INSERT OR REPLACE INTO job_id_count (key, value) VALUES (?1, ?2)",
        params!["JOB_ID_COUNT", count_str],
    )?;
    Ok(())
}

// checks if SQL storage is on
// if yes, then retrieves the job id count
pub fn get_job_count(connection: &Connection, count: &mut u32)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        *count = match sql::retrieve_job_id_count(&connection)
        {
            Ok(x) => x,
            Err(_) => {panic!("unsuccessful retrieve");}
        };
    }
}

// retrieves the job id count from the database 
pub fn retrieve_job_id_count(conn: &Connection) -> Result<u32> 
{
    let mut stmt = conn.prepare("SELECT value FROM job_id_count WHERE key = ?1")?;
    let count_str: String = stmt.query_row(params!["JOB_ID_COUNT"], |row| row.get(0))?;
    let count: u32 = count_str.parse().unwrap_or(0);
    Ok(count)
}

// inserts default job id count, which is 0
// only called if reset mode is on
pub fn insert_default_job_id_count(conn: &Connection) -> Result<()> 
{
    conn.execute(
        "INSERT OR REPLACE INTO job_id_count (key, value) VALUES (?1, ?2)",
        params!["JOB_ID_COUNT", 0],
    )?;
    Ok(())
}

// JOB LIST

// checks if SQL storage is on
// if yes, then stores the job list
pub fn push_job(connection: &Connection, list: &Vec<ResponseContent>)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        store_job(&connection, &list).expect("failed to store job list");
    }
}

// stores the job list from the database
pub fn store_job(conn: &Connection, response_contents: &Vec<ResponseContent>) -> Result<()> 
{
    // clears original list
    conn.execute("DELETE FROM response_content", [])?;
    // inserts each element of the new list
    for response_content in response_contents {
        let submission_json = serde_json::to_string(&response_content.submission).expect("error storing jobs");
        let cases_json = serde_json::to_string(&response_content.cases).expect("error storing jobs");
        conn.execute(
            "INSERT INTO response_content (id, created_time, updated_time, submission, state, result, score, cases) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                response_content.id,
                response_content.created_time.to_rfc3339(),
                response_content.updated_time.to_rfc3339(),
                submission_json,
                response_content.state,
                format!("{:?}", response_content.result),
                response_content.score,
                cases_json
            ],
        )?;
    }
    Ok(())
}

// checks if SQL storage is on
// if yes, then retrieves the job list
pub fn get_job(connection: &Connection, lock_job_list: &mut Vec<ResponseContent>)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        *lock_job_list = match sql::retrieve_job(&connection)
        {
            Ok(x) => 
            {
                x
            }
            Err(_) => {panic!("unsuccessful retrieve");}
        };
        log::info!("{:?}", lock_job_list);
    }
}

// retrieves the job list from the database
pub fn retrieve_job(conn: &Connection) -> Result<Vec<ResponseContent>> 
{
    let mut stmt = conn.prepare("SELECT id, created_time, updated_time, submission, state, result, score, cases FROM response_content")?;
    let mut response_contents = Vec::new();

    let response_iter = stmt.query_map(params![], |row| {
        let created_time: String = row.get(1)?;
        let updated_time: String = row.get(2)?;
        let submission_json: String = row.get(3)?;
        let cases_json: String = row.get(7)?;

        Ok(ResponseContent {
            id: row.get(0)?,
            created_time: created_time.parse::<DateTime<Utc>>().unwrap(),
            updated_time: updated_time.parse::<DateTime<Utc>>().unwrap(),
            submission: serde_json::from_str(&submission_json).unwrap(),
            state: row.get(4)?,
            result: match row.get::<_, String>(5)?.as_str() {
                "Waiting" => PossibleResult::Waiting,
                "Running" => PossibleResult::Running,
                "Accepted" => PossibleResult::Accepted,
                "Compilation Error" => PossibleResult::CompilationError,
                "Compilation Success" => PossibleResult::CompilationSuccess,
                "Wrong Answer" => PossibleResult::WrongAnswer,
                "Runtime Error" => PossibleResult::RuntimeError,
                "Time Limit Exceeded" => PossibleResult::TimeLimitExceeded,
                "Memory Limit Exceeded" => PossibleResult::MemoryLimitExceeded,
                "System Error" => PossibleResult::SystemError,
                "SPJ Error" => PossibleResult::SPJError,
                "Skipped" => PossibleResult::Skipped,
                _ => return Err(rusqlite::Error::InvalidQuery),
            },
            score: row.get(6)?,
            cases: serde_json::from_str(&cases_json).unwrap(),
        })
    })?;

    // stores each entry into a vector
    for response in response_iter {
        response_contents.push(response?);
    }

    //returns the vector
    Ok(response_contents)
}

// no default value neede for job list

// CONTEST ID COUNT 

// checks if SQL storage is on
// if yes, then stores the count
pub fn push_contest_count(connection: &Connection, count: u32)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        store_contest_id_count(&connection, count).expect("failed to store contest id count");
    }
}

// stores count into database
pub fn store_contest_id_count(conn: &Connection, count: u32) -> Result<()> {
    let count_str = count.to_string();
    conn.execute(
        "INSERT OR REPLACE INTO contest_id_count (key, value) VALUES (?1, ?2)",
        params!["CONTEST_ID_COUNT", count_str],
    )?;
    Ok(())
}

// checks if SQL storage is on
// if yes, then retrieves the count
pub fn get_contest_count(connection: &Connection, count: &mut u32)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        *count = match sql::retrieve_contest_id_count(&connection)
        {
            Ok(x) => x,
            Err(_) => {panic!("unsuccessful retrieve");}
        };
    }
}

// retrieves the count from the database
pub fn retrieve_contest_id_count(conn: &Connection) -> Result<u32>
{
    let mut stmt = conn.prepare("SELECT value FROM contest_id_count WHERE key = ?1")?;
    let count_str: String = stmt.query_row(params!["CONTEST_ID_COUNT"], |row| row.get(0))?;
    let count: u32 = count_str.parse().unwrap_or(0);
    Ok(count)
}

// inserts the default value of the contest count, which is 1
pub fn insert_default_contest_id_count(conn: &Connection) -> Result<()> 
{
    conn.execute(
        "INSERT OR REPLACE INTO contest_id_count (key, value) VALUES (?1, ?2)",
        params!["CONTEST_ID_COUNT", 1],
    )?;
    Ok(())
}


// CONTEST LIST

// checks if SQL storage is on
// if yes, then stores the the contest list
pub fn push_contest(connection: &Connection, lock_contest_list: &Vec<contest::Contest>)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        store_contests(&connection, &lock_contest_list).expect("failed to store contest list");
    }
}

// stores the the contest list
pub fn store_contests(conn: &Connection, contests: &[Contest]) -> Result<()> {
    // first deletes the original list
    conn.execute("DELETE FROM contest", [])?;
    // the inserts each element of the new list
    for contest in contests {
        let problem_ids_json = serde_json::to_string(&contest.problem_ids).expect("error store contest");
        let user_ids_json = serde_json::to_string(&contest.user_ids).expect("error store contest");
        let users_json = serde_json::to_string(&contest.users).expect("error store contest");
        conn.execute(
            "INSERT INTO contest (id, name, from_time, to_time, problem_ids, user_ids, submission_limit, users) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                contest.id,
                contest.name,
                contest.from.to_rfc3339(),
                contest.to.to_rfc3339(),
                problem_ids_json,
                user_ids_json,
                contest.submission_limit,
                users_json
            ],
        )?;
    }
    Ok(())
}

// checks if SQL storage is on
// if yes, then retrieves contest list
pub fn get_contest(connection: &Connection, lock_contest_list: &mut Vec<contest::Contest>)
{
    let is_sql = IS_SQL.lock().unwrap();
    if *is_sql
    {
        *lock_contest_list = match sql::retrieve_contests(&connection)
        {
            Ok(x) => x,
            Err(_) => {panic!("unsuccessful retrieve");}
        };
    }
}

// retrieves contest list
pub fn retrieve_contests(conn: &Connection) -> Result<Vec<Contest>> {
    let mut stmt = conn.prepare("SELECT id, name, from_time, to_time, problem_ids, user_ids, submission_limit, users FROM contest")?;
    let contest_iter = stmt.query_map([], |row| {
        let from_time: String = row.get(2)?;
        let to_time: String = row.get(3)?;
        let problem_ids_json: String = row.get(4)?;
        let user_ids_json: String = row.get(5)?;
        let users_json: String = row.get(7)?;

        Ok(Contest {
            id: row.get(0)?,
            name: row.get(1)?,
            from: from_time.parse::<DateTime<Utc>>().unwrap(),
            to: to_time.parse::<DateTime<Utc>>().unwrap(),
            problem_ids: serde_json::from_str(&problem_ids_json).unwrap(),
            user_ids: serde_json::from_str(&user_ids_json).unwrap(),
            submission_limit: row.get(6)?,
            users: serde_json::from_str(&users_json).unwrap(),
        })
    })?;

    let mut contests = Vec::new();
    for contest in contest_iter {
        contests.push(contest?);
    }

    Ok(contests)
}

// stores the default value of the contest list
// which only consists of contest 0
pub fn insert_default_contest(conn: &Connection, config: &config::Config) -> Result<()> 
{
    // first create contest 0, then push into database
    let mut problem_ids: Vec<u32> = vec![];
    let user_ids: Vec<u32> = vec![0];
    let mut users: Vec<contest::RankInfo> = vec![];

    for i in 0..config.problems.len()
    {
        problem_ids.push(i as u32);
    }
    users.push(
        contest::RankInfo
        {
            user: User {
                id: 0,
                name: "root".to_string(),
                },
            scores: vec![0.0; config.problems.len()],
            rank: 0,
            
            highest_scores: vec![0.0 ; config.problems.len()],
            latest_scores: vec![0.0 ; config.problems.len()],

            competitive_score_sum: 0.0,
            shortest_times: vec![vec![MAX; 20]; config.problems.len()], // possible error
            
            latest_submission: DateTime::<Utc>::MAX_UTC,
            score: 0,
            submission_count: 0,
        }
    );

    let problem_ids_json = serde_json::to_string(&problem_ids).expect("error store contest");
    let user_ids_json = serde_json::to_string(&user_ids).expect("error store contest");
    let users_json = serde_json::to_string(&users).expect("error store contest");

    conn.execute(
        "INSERT INTO contest (id, name, from_time, to_time, problem_ids, user_ids, submission_limit, users) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            0,
            "root".to_string(),
            DateTime::<Utc>::MAX_UTC.to_rfc3339(),
            DateTime::<Utc>::MIN_UTC.to_rfc3339(),
            problem_ids_json,
            user_ids_json,
            0,
            users_json
        ],
    )?;
    Ok(())
}