use actix_web::{get, Responder, HttpResponse};

use crate::user;
use crate::USER_LIST;
use crate::sql;
use crate::CONNECTION;

// gets list of all users
#[get("/users")]
async fn get_users() -> impl Responder 
{
    let connection = CONNECTION.lock().unwrap();
    let mut lock_user_list: std::sync::MutexGuard<Vec<user::User>> = USER_LIST.lock().unwrap();
    
    // if SQL storage activated, then load from database
    sql::get_user(&connection, &mut lock_user_list);

    return HttpResponse::Ok().json(lock_user_list.clone());
}