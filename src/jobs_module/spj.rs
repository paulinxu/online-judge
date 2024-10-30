use std::fs::File;
use std::process::Command;
use std::process::Stdio;
use std::io::BufReader;
use std::io::BufRead;

use crate::config;

// function for special judge compare
pub fn compare_spj(out_file_path: String, ans_file_path: String, config: &config::Config, problem_index: usize) -> (bool, String, bool)
{
    // creates a temporary directory for special judge
    match std::fs::create_dir("SPJDIR") 
    {
        Ok(_) => {}
        Err(_) => 
        {
            return (false, "".to_string(), true);
        }
    }
    // creates a temporary output file
    let out_file = match File::create("SPJDIR/spj.out") 
    {
        Ok(file) => file,
        Err(_) => 
        {
            return (false, "".to_string(), true);
        }
    };

    let mut commands: Vec<String> = vec![];
    if let Some(original_commands) = config.problems[problem_index].misc.special_judge.clone()
    {
        // obtains the command to be ran
        for i in original_commands.clone()
        {
            if i == original_commands[0] {continue;} // ignore first element
            else if i == "%OUTPUT%" {commands.push(out_file_path.clone());}
            else if i == "%ANSWER%" {commands.push(ans_file_path.clone());}
            else {commands.push(i.clone());}
        }
        log::info!("{:?}", commands);

        // runs the special judge and store the result in the temporary output file
        match Command::new(original_commands[0].clone())
                    .args(commands)
                    .stdout(Stdio::from(out_file))
                    .stderr(Stdio::null())
                    .status()
        {
            // if successful, then open and obtain the lines in the output file
            Ok(status) if status.success() => 
            {
                log::info!("Code ran successfully");
                let out_file = match File::open("SPJDIR/spj.out") 
                {
                    Ok(file) => file,
                    Err(_) => 
                    {
                        return (false, "".to_string(), true);
                    }
                };
                
                let read_outfile = BufReader::new(out_file);
                let outfile_lines = read_outfile.lines()
                    .filter_map(Result::ok)
                    .filter(|line: &String| !line.trim().is_empty());


                let mut lines: Vec<String> = vec![];
                for x in outfile_lines
                {
                    lines.push(x);
                }
                
                // first line gives the result of the comparison

                let mut accepted: bool = false;
                if lines[0] == "Accepted" {accepted = true;}

                // second line gives the additional info
                
                let info: String = lines[1].clone();

                return (accepted, info, false);
            }
            _ =>
            {
                return (false, "".to_string(), true);
            }
        }
    }
    else {return (false, "".to_string(), true);}
    // return spj error if error encountered
    
}

