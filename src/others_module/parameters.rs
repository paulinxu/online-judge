use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cli 
{
    #[clap(short, long)]
    pub config: Option<String>,
    #[clap(short, long = "flush-data")]
    pub flush_data: bool,
    #[clap(short, long)]
    pub storage: bool, // activates sql storage mode
    #[clap(short, long)]
    pub reset_storage: bool, // resets sql database
}