use console::{style, Term};
use std::{fs, io::Write};

use crate::draw::Position;

pub fn show_info(position: &Position, term: &Term) {
    let contents = fs::read_to_string("resources/OpenCorePkg/Docs/Configuration.tex").unwrap();

    let mut sec_found = false;
    let mut sec_search = "\\section{".to_string();
    sec_search.push_str(&position.sec_key[0]);
    let mut text_found = false;
    let mut text_search = "\\texttt{".to_string();
    text_search.push_str(&position.sec_key[position.depth]);
    text_search.push_str(&"}\\");

    write!(&*term, "\r\n").unwrap();
    let mut itemize = false;

    for line in contents.lines() {
        if sec_found {
            if text_found {
                if !itemize && line.contains("\\item") {
                    break;
                }
                if line.contains("end{enumerate}") {
                    break;
                }
                if line.contains("begin{itemize}") {
                    itemize = true;
                }
                if line.contains("end{itemize}") {
                    itemize = false;
                }
                //write!(&*term, "{}\x1B[0K\r\n", line).unwrap();
                write!(&*term, "\x1B[2K{}", parse_line(line)).unwrap();
            } else {
                if line.contains(&text_search) {
                    //                    write!(&*term, "\r\n{}\x1B[0K", line).unwrap();
                    text_found = true;
                }
            }
        } else {
            if line.contains(&sec_search) {
                //                write!(&*term, "\r\n{}\x1B[0K", line).unwrap();
                sec_found = true;
            }
        }
    }
    write!(&*term, "{}\x1B[0K", style(" ".repeat(70)).underlined()).unwrap();
}

fn parse_line(line: &str) -> String {
    let mut ret = String::new();
    let mut build_key = false;
    let mut build_name = false;
    let mut skip_line = false;
    let mut key = String::new();
    let mut name = String::new();
    for c in line.chars() {
        if build_key {
            match c {
                '{' => {
                    build_key = false;
                    build_name = true;
                    if key == "textbf" {
                        ret.push_str("\x1B[1m");
                    }
                    if key == "emph" {
                        ret.push_str("\x1B[7m");
                    }
                    if key == "begin" {
                        skip_line = true;
                    }
                    if key == "end" {
                        skip_line = true;
                    }
                    if key == "texttt" {
                        ret.push_str("\x1B[4m");
                    }
                    key.clear();
                }
                ' ' => {
                    build_key = false;
                    if key == "textbackslash" {
                        ret.push('\\');
                    }
                    if key == "item" {
                        ret.push_str("+ ");
                    }
                    if key == "tightlist" {
                        skip_line = true;
                    }
                    if key == "" {
                        ret.push(' ');
                    }
                    key.clear();
                }
                '_' | '^' => {
                    build_key = false;
                    ret.push(c);
                    key.clear();
                }
                _ => key.push(c),
            }
        } else if build_name {
            match c {
                '}' => {
                    build_name = false;
                    ret.push_str(&name);
                    ret.push_str("\x1B[0m");
                    name = "".to_string();
                }
                '\\' => {
                    ret.push_str(&name);
                    name = "".to_string();
                    build_key = true;
                }
                _ => name.push(c),
            }
        } else {
            match c {
                '\\' => build_key = true,
                _ => ret.push(c),
            }
        }
    }
    if build_key {
        if key == "tightlist" {
            skip_line = true;
        }
    }
    if skip_line {
        ret.clear();
    } else {
        ret.push_str("\r\n");
    }

    ret
}
/*
pub struct Config {
    pub query: String,
    pub filename: String,
    pub case_sensitive: bool,
}

impl Config {
    pub fn new(mut args: env::Args) -> Result<Config, &'static str> {
        args.next();

        let query = match args.next() {
            Some(arg) => arg,
            None => return Err("Didn't get a query string."),
        };

        let filename = match args.next() {
            Some(arg) => arg,
            None => return Err("Didn't get a file name."),
        };

        let case_sensitive = env::var("CASE_INSENSITIVE").is_err();

        Ok(Config {
            query,
            filename,
            case_sensitive,
        })
    }
}

pub fn get_description(f: &fs::File, name: &str) -> String {
    let mut des = String::new();
    des
}

pub fn run(config: Config, term: &Term) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.filename)?;

    let results = if config.case_sensitive {
        search(&config.query, &contents)
    } else {
        search_case_insensitive(&config.query, &contents)
    };

    write!(&*term, "{}{}\x1B[u", &config.query, &config.query.len())?;
    for line in results {
        write!(&*term, "\r\n{}\x1B[0K", line)?;
    }

    Ok(())
}

pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    contents
        .lines()
        .filter(|line| line.contains(query))
        .collect()
}

pub fn search_case_insensitive<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    let query = &query.to_lowercase();

    contents
        .lines()
        .filter(|line| line.contains(query))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_sensitive() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Duct tape.";

        assert_eq!(vec!["safe, fast, productive."], search(query, contents));
    }

    #[test]
    fn case_insensitive() {
        let query = "rUsT";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Trust me.";

        assert_eq!(
            vec!["Rust:", "Trust me."],
            search_case_insensitive(query, contents)
        );
    }
}*/
