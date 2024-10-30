use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User
{
    pub id: u32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetUser
{
    pub id: Option<u32>,
    pub name: String,
}