use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Read;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Server 
{
    pub bind_address: String,
    pub bind_port: u16,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Case 
{
    pub score: f32,
    pub input_file: String,
    pub answer_file: String,
    pub time_limit: u64,
    pub memory_limit: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub enum ProblemType
{
    standard,
    strict,
    spj,
    dynamic_ranking,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Misc
{
    pub packing: Option<Vec<Vec<u32>>>,
    pub special_judge: Option<Vec<String>>,
    pub dynamic_ranking_ratio: Option<f32>
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Problem 
{
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ProblemType,
    pub misc: Misc,
    pub cases: Vec<Case>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Language 
{
    pub name: String,
    pub file_name: String,
    pub command: Vec<String>,
}

impl Language
{
    pub fn get_file_name(&mut self) -> String
    {
        return self.file_name.clone();
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config 
{
    pub server: Server,
    pub problems: Vec<Problem>,
    pub languages: Vec<Language>,
}

// function loads information into type Config

pub fn load(filename: &str) -> Result<Config, Box<dyn std::error::Error>> 
{
    let mut file = File::open(filename)?;
    
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    if contents.trim().is_empty() || contents.trim() == "{}" 
    {
        return Err("File is empty".into());
    }

    let config: Config = serde_json::from_str(&contents)?;
    Ok(config)
}