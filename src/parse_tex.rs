use std::{fs, path::PathBuf};

/// Output Configuration.tex info for specific field
/// take a path to Configuration.tex and a Vec for desired Section, Subsection and Field
/// return a Vec containing the formatted lines of the Configuration.tex
pub fn parse_configuration(
    tex_path: &PathBuf,
    search_str: Vec<String>,
    width: i32,
    gather_valid: bool,
    show_url: bool,
) -> Vec<String> {
    let mut result = vec![];
    if search_str.len() == 0 {
        result.push("no field values supplied".to_owned());
        return result;
    }
    let contents = fs::read_to_string(tex_path).unwrap_or("".to_string());
    if contents.len() == 0 {
        result.push("local Configuration.tex not found".to_owned());
        return result;
    }

    let mut search_terms = vec![];

    search_terms.push("\\section{".to_string() + &search_str[0]);
    let mut text_search = search_str.last().unwrap().to_owned();
    if text_search.len() < 3 {
        result.push("array item #".to_string() + &text_search + "\r\n");
        return result;
    }

    match search_str.len() {
        1 => (),
        2 => {
            search_terms.push("\\subsection{Properties".to_string());
            search_terms.push("texttt{".to_string() + &text_search + "}\\");
        }
        3 => {
            if search_str[0] == "NVRAM" {
                search_terms.push("\\subsection{Introduction".to_string());
                search_terms.push("texttt{".to_string() + &text_search + "}");
            } else {
                search_terms.push("\\subsection{".to_string() + &search_str[1] + " Properties");
                search_terms.push("texttt{".to_string() + &text_search + "}\\");
            }
        }
        4 => {
            let mut sub_search = "\\subsection{".to_string();
            match search_str[0].as_str() {
                "NVRAM" => {
                    sub_search = "\\subsection{Introduction".to_string();
                    text_search = search_str[2].to_owned() + ":" + &search_str[3] + "}";
                }
                "DeviceProperties" => {
                    sub_search.push_str("Common");
                    text_search.push_str("}");
                }
                "Misc" => {
                    if search_str[2].len() < 3 {
                        sub_search.push_str("Entry Properties");
                    } else {
                        sub_search = "\\subsubsection{".to_string() + &search_str[1];
                    }
                    text_search.push_str("}");
                }
                _ => {
                    sub_search.push_str(&search_str[1]);
                    sub_search.push_str(" Properties");
                    text_search.push_str("}\\");
                }
            };
            search_terms.push(sub_search);
            search_terms.push("texttt{".to_string() + &text_search);
        }
        5 => {
            search_terms.push("\\subsubsection{".to_string() + &search_str[1]);
            search_terms.push("texttt{".to_string() + &text_search);
        }
        _ => return result,
    }

    //    return search_terms;

    let mut lines = contents.lines();

    for term in search_terms {
        loop {
            match lines.next() {
                Some(line) => {
                    if line.contains(&term) {
                        break;
                    }
                }
                None => return result,
            }
        }
    }

    let mut align = false;
    let mut itemize = 0;
    let mut enumerate = 0;
    let mut columns = 0;
    let mut lines_between_valid = 0;

    for line in lines {
        if line.trim_start().starts_with("%") {
            // skip comments
            continue;
        }
        if line.contains("\\subsection{Introduction}") {
            continue;
        }
        if line.contains("\\begin{tabular") {
            for c in line.chars() {
                if c == 'c' {
                    columns += 1;
                };
            }
            continue;
        }
        if line.contains("\\begin{align*}") {
            align = true;
            continue;
        }
        if line.contains("\\end{align*}}") {
            align = false;
            continue;
        }
        // cheap hack to keep track of being in a list
        if line.contains("\\begin{itemize}") {
            itemize += 1;
            continue;
        }
        if line.contains("\\begin{enumerate}") {
            enumerate += 1;
            continue;
        }
        if line.contains("\\mbox") {
            continue;
        }
        //        if line.contains("\\begin{") {
        //            continue;
        //        }
        if line.contains("\\end{tabular}") {
            columns = 0;
            continue;
        }
        if line.contains("\\end{itemize}") {
            itemize -= 1;
            continue;
        }
        if line.contains("\\end{enumerate}") {
            enumerate -= 1;
            continue;
        }
        if line.contains("\\end{") {
            continue;
        }
        if line.contains("\\item") && (itemize == 0 && enumerate == 0) {
            break;
        }
        if line.contains("\\subsection{") || line.contains("\\section{") {
            break;
        }
        let parsed_line = parse_line(line, columns, width, align, gather_valid, show_url);
        if gather_valid {
            // gather list items to display when editing a string or integer
            if itemize > 0 {
                // we are inside an itemize bracket
                if line.contains("---") {
                    if lines_between_valid < 10 {
                        result.push(parsed_line);
                    }
                }
            } else {
                // stop gathering if there has been a big break
                if result.len() > 0 {
                    lines_between_valid += 1;
                }
            }
        } else {
            if parsed_line.len() != 0 {
                result.push(parsed_line);
                //                result.push(format!("\x1B[0K{}", parsed_line));
            }
        }
    }
    result
}

/// Go through line 1 character at a time to apply .tex formatting
///
/// TODO: pass back attributes so formatting/mode can exist for more than 1 line
///
fn parse_line(
    line: &str,
    columns: i32,
    width: i32,
    align: bool,
    gather_valid: bool,
    show_url: bool,
) -> String {
    let mut ret = String::new();
    let mut build_key = false;
    let mut key = String::new();
    let mut col_width = 0;
    if columns > 0 {
        col_width = width / (columns + 1);
    }
    let mut ignore = false;
    let mut col_contents_len = 0;
    for c in line.chars() {
        if build_key {
            match c {
                // end of key
                '{' | '[' => {
                    build_key = false;
                    //                    build_name = true;
                    if !gather_valid {
                        match key.as_str() {
                            "text" => ret.push_str("\x1B[0m"),
                            "textit" => ret.push_str("\x1B[1m"),
                            "textbf" => ret.push_str("\x1B[1m"),
                            "emph" => ret.push_str("\x1B[7m"),
                            "texttt" => ret.push_str("\x1B[4m"),
                            "href" => {
                                if show_url {
                                    ret.push_str("\x1B[34m");
                                } else {
                                    ignore = true;
                                }
                            }
                            _ => ignore = true,
                        };
                    }
                    if &key != "href" {
                        // hold href key to insert space after it
                        key.clear();
                    }
                }
                // end of key - may be special character or formatting
                ' ' | ',' | '(' | ')' | '\\' | '0'..='9' | '$' | '&' => {
                    build_key = false;
                    if &key == "item" {
                        if !gather_valid {
                            ret.push('•');
                        }
                    }
                    ret.push(special_char(&key));
                    col_contents_len += 1;
                    if c == ',' || c == '(' || c == ')' || (c >= '0' && c <= '9') || c == '$' {
                        ret.push(c);
                    }
                    if c == '\\' {
                        if key.len() > 0 {
                            // check for double \
                            build_key = true; // or start of new key
                        }
                    }
                    key.clear();
                }
                // found escaped character
                '_' | '^' | '#' => {
                    build_key = false;
                    ret.push(c);
                    col_contents_len += 1;
                    key.clear();
                }
                _ => key.push(c),
            }
        } else {
            match c {
                '\\' => build_key = true,
                '}' | ']' => {
                    if !ignore {
                        if !gather_valid {
                            ret.push_str("\x1b[0m");
                            if &key == "href" {
                                ret.push(' ');
                                key.clear();
                            } else if c == ']' {
                                ret.push(']');
                            }
                        }
                    }
                    ignore = false;
                }
                '{' => {
                    if !gather_valid {
                        ret.push_str("\x1B[4m");
                    }
                }
                '&' => {
                    if columns > 0 {
                        let fill = col_width - col_contents_len - 1;
                        if fill > 0 {
                            ret.push_str(&" ".repeat(fill as usize));
                        }
                        ret.push_str("|");
                        col_contents_len = 0;
                    } else {
                        if !align {
                            ret.push('&');
                        }
                    }
                }
                _ => {
                    if !ignore {
                        ret.push(c);
                        col_contents_len += 1;
                    }
                }
            }
        }
    }
    if key.len() > 0 {
        ret.push(special_char(&key));
    }
    if !gather_valid {
        if key == "tightlist" {
            // ignore
            ret.clear();
        } else {
            if key == "hline" {
                ret.push_str(&"-".repeat(width as usize - 4));
            }
            ret.push_str("\r\n");
        }
    }

    ret
}

fn special_char(key: &str) -> char {
    match key {
        "kappa" => '\u{03f0}',
        "lambda" => '\u{03bb}',
        "mu" => '\u{03bc}',
        "alpha" => '\u{03b1}',
        "beta" => '\u{03b2}',
        "gamma" => '\u{03b3}',
        "leq" => '\u{2264}',
        "cdot" => '\u{00b7}',
        "in" => '\u{220a}',
        "infty" => '\u{221e}',
        "textbackslash" => '\\',
        "hline" => '\u{200b}',
        _ => ' ',
    }
}
