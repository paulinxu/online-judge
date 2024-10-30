use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::Read;
use std::io;

// compares two output using standard mode
// ignores line switching differences
pub fn compare_standard(outfile: File, ans_file: File) -> bool 
{
    // collects each line from both files while trimming empty lines
    let read_outfile = BufReader::new(outfile);
    let mut outfile_lines = read_outfile.lines()
        .filter_map(Result::ok)
        .filter(|line: &String| !line.trim().is_empty());

    let read_ansfile = BufReader::new(ans_file);
    let mut ansfile_lines = read_ansfile.lines()
        .filter_map(Result::ok)
        .filter(|line| !line.trim().is_empty());

    // compares that each line is the same and that they end at the same time
    loop 
    {
        match (outfile_lines.next(), ansfile_lines.next())
        {
            (Some(out), Some(ans)) =>
            {
                if out != ans
                {
                    return false;
                }
            },
            (None, None) => return true,
            _ => return false,
        }
    }
}

// function reads file contents into string for strict comparison
fn read_file_to_string(mut file: File) -> io::Result<String> 
{
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

// compares two files in strict mode
pub fn compare_strict(outfile: File, ans_file: File) -> bool {
    // loads both files into a string 
    // which preserves \n characters
    let outfile_content = match read_file_to_string(outfile)
    {
        Ok(content) => content,
        Err(_) => {return false;}
    };
    let ans_file_content = match read_file_to_string(ans_file)
    {
        Ok(content) => content,
        Err(_) => {return false;}
    };
    outfile_content == ans_file_content
}