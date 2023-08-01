use crate::edit;
use crate::init::{Manifest, Settings};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crossterm::terminal::size;
use crossterm::{cursor, terminal, ExecutableCommand};
use curl::easy::Easy;
use plist::Value;
use walkdir::WalkDir;

use crate::edit::add_item;

use sha2::Digest;

// TODO: much of this module needs refactoring, well, I'm learning
// TODO: many of the functions seem redundant and need improvement, soon, lol

#[derive(Debug)]
pub struct Resources {
    pub dortania: serde_json::Value, // Dortania builds config.json file
    pub octool_config: serde_json::Value, // config file for octool itself
    pub config_differences: serde_json::Value, // config file for octool itself
    pub resource_list: serde_json::Value, // list linking resources to their parents
    pub other: serde_json::Value,    // list of other party parent/childs
    pub config_plist: plist::Value,  // current active config.plist
    pub sample_plist: plist::Value,  // latest Sample.plist
    pub input_dir_path: PathBuf,     // location of INPUT directory to be used
    pub working_dir_path: PathBuf,   // location of octool and files
    pub open_core_binaries_path: PathBuf, // location of the OpenCorePkg binaries
    pub open_core_source_path: PathBuf, // location of OpenCore source files
}

/// get list of available version numbers for parent resource in the dortania build config.json
/// returns list in versions and index number of first occurence in indexes
pub fn get_parent_version_nums(
    parent: &str,
    resources: &Resources,
    versions: &mut Vec<String>,
    indexes: &mut Vec<usize>,
) {
    let mut ver = String::new();
    let mut last_ver = String::new();
    let mut index = 0;
    loop {
        if let Some(v) = resources.dortania[parent]["versions"][index]["version"].as_str() {
            if &last_ver != v {
                last_ver = v.to_string();
                ver.push_str(v);
                ver.push_str(" --- ");
                ver.push_str(
                    &resources.dortania[parent]["versions"][index]["date_committed"]
                        .as_str()
                        .unwrap_or("no date found")[0..10],
                );
                ver.push(' ');
                ver.push_str(
                    &resources.dortania[parent]["versions"][index]["commit"]["sha"]
                        .as_str()
                        .unwrap_or("no sha found")[0..7],
                );
                ver.push_str(" · ");
                ver.push_str(
                    &resources.dortania[parent]["versions"][index]["commit"]["message"]
                        .as_str()
                        .unwrap_or("")
                        .lines()
                        .next()
                        .unwrap(),
                );
                indexes.push(index);
                versions.push(ver);
                ver = "".to_string();
            }
            index += 1;
        } else {
            break;
        }
    }
}

/// check if parent resource exists locally, if it does check for updates
/// if it doesn't exist locally then retrieve it
pub fn get_or_update_local_parent(
    parent: &str,
    single_resource: &serde_json::Value,
    build_type: &str,
    build_index: &usize,
    verbose: bool,
    do_update: bool,
    stdout: &mut Stdout,
    silent: bool,
) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let url = single_resource[parent]["versions"][build_index]["links"][build_type]
        .as_str()
        .unwrap_or("");
    if url == "" {
        return Ok(None);
    }
    let hash = single_resource[parent]["versions"][build_index]["hashes"][build_type]["sha256"]
        .as_str()
        .unwrap_or("");
    let mut path = Path::new("resources").to_path_buf();
    let mut dir = Path::new(url).file_stem().unwrap().to_str().unwrap();
    if dir == "master" || dir == "main" {
        if !path.join(&parent).exists() {
            if do_update {
                get_repo_and_unzip(&parent, &url, &path, stdout)?;
            }
        };
        path = path.join(&parent).join(".zip");
    } else {
        if dir.ends_with(".kext") {
            dir = &dir[0..dir.len() - 5];
        }
        let file_name = Path::new(url).file_name().unwrap();
        let sum_file = path.join(dir).join("sum256");
        path = path.join(dir).join(file_name);

        match url.split('.').last().unwrap() {
            "zip" => match File::open(&sum_file) {
                Ok(mut sum_file) => {
                    let mut sum = String::new();
                    sum_file.read_to_string(&mut sum)?;
                    if do_update {
                        if sum != hash {
                            if !silent {
                                write!(
                                    stdout,
                                    "\r\nremote hash {}\x1B[0K\r\n  local sum {}\x1B[0K\r\n",
                                    hash, sum
                                )?;
                                write!(
                                    stdout,
                                    "{yel}new version found, {grn}Downloading\x1B[0m\x1B[0K\r\n",
                                    yel = "\x1b[33m",
                                    grn = "\x1b[32m",
                                )?;
                            }
                            get_file_and_unzip(url, hash, &path, stdout, silent)?;
                        } else {
                            if verbose {
                                write!(stdout, "Already up to date\x1B[0K\r\n")?;
                            }
                        }
                    }
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        if do_update {
                            if !silent {
                                write!(
                            stdout,
                            "{:?} {yel}local copy not found, {grn}Downloading\x1B[0m\x1B[0K\r\n",
                            dir,
                            yel = "\x1b[33m",
                            grn = "\x1b[32m",
                        )?;
                                write!(stdout, "remote hash {}\x1B[0K\r\n", hash)?;
                            }
                            get_file_and_unzip(url, hash, &path, stdout, silent)?;
                        }
                    }
                    _ => panic!("{}", e),
                },
            },
            _ => panic!("unknown parent type"),
        }
    }
    Ok(Some(path))
}

/// Runs `command` with included args and returns the result or Err
/// # example
/// ```
/// let stats = status("git", &["fetch", "--all"]);
///
/// ```
pub fn status(command: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(command).args(args).output()?)
}

pub fn curl_file(url: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut out_file = File::create(&path).unwrap();
    let mut easy = Easy::new();
    easy.url(url)?;
    easy.follow_location(true)?;
    easy.write_function(move |data| {
        out_file.write_all(&data).unwrap();
        Ok(data.len())
    })?;
    easy.perform()?;
    Ok(())
}

/// use git-api to check the size of the dortania/builds/config.json file
/// if the size has changed then need to download current version
pub fn curl_build_size(path: &Path) -> Result<i64, Box<dyn Error>> {
    let mut out_file = File::create(&path)?;
    let mut easy = Easy::new();
    easy.useragent("-H \"Accept: application/vnd.github.v3+json\"")?;
    easy.url("https://api.github.com/repos/dortania/build-repo/branches/builds")?;
    easy.write_function(move |data| {
        out_file.write_all(&data).unwrap();
        Ok(data.len())
    })?;
    easy.perform()?;
    let out_file = File::open(&path)?;
    let buf = BufReader::new(out_file);
    let current_sha: serde_json::Value = serde_json::from_reader(buf)?;
    let current_sha = current_sha["commit"]["sha"].as_str().unwrap_or("");
    let mut build_url = String::from("https://api.github.com/repos/dortania/build-repo/git/trees/");
    build_url.push_str(current_sha);
    let mut out_file = File::create(&path)?;
    easy.useragent("-H \"Accept: application/vnd.github.v3+json\"")?;
    easy.url(&build_url)?;
    easy.write_function(move |data| {
        out_file.write_all(&data).unwrap();
        Ok(data.len())
    })?;
    easy.perform()?;
    let out_file = File::open(&path)?;
    let buf = BufReader::new(out_file);
    let size: serde_json::Value = serde_json::from_reader(buf)?;
    let size = size["tree"][0]["size"].as_i64().unwrap_or(0);
    Ok(size)
}

fn get_repo_and_unzip(
    parent: &str,
    url: &str,
    path: &Path,
    stdout: &mut Stdout,
) -> Result<(), Box<dyn Error>> {
    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    };

    write!(
        stdout,
        "{} not found, \x1B[33mDownloading\x1b[0m ... ",
        parent
    )?;
    stdout.flush()?;
    let zip_path = path.join(&url.split('/').last().unwrap());

    curl_file(&url, &zip_path)?;

    let z_file = File::open(&zip_path)?;
    let mut z_archive = zip::ZipArchive::new(z_file)?;
    match z_archive.extract(&path) {
        Ok(_) => std::fs::remove_file(&zip_path)?,
        Err(e) => panic!("{:?}", e),
    }
    let mut old_name = String::from(parent);
    old_name.push_str("-master");
    if path.join(&old_name).exists() {
        std::fs::rename(path.join(&old_name), path.join(&parent))?;
    };
    write!(stdout, "\x1b[32mDone\x1b[0m\r\n")?;
    Ok(())
}

pub fn get_file_and_unzip(
    url: &str,
    hash: &str,
    path: &Path,
    stdout: &mut Stdout,
    silent: bool,
) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(path.parent().unwrap())?;

    curl_file(&url, &path)?;

    let mut z_file = File::open(path)?;
    if hash != "" {
        let mut data = Vec::new();
        z_file.read_to_end(&mut data).unwrap();
        let sum = format!("{:x}", sha2::Sha256::digest(&data));
        if !silent {
            write!(stdout, "  local sum {}\x1B[0K\r\n", sum)?;
        }
        if sum != hash {
            write!(
                stdout,
                "\x1b[31mERROR:\x1b[0m Sum of {:?} does not match {}\r\n\r\nExiting\r\n",
                path, hash
            )?;
            stdout.execute(cursor::Show).unwrap();
            terminal::disable_raw_mode().unwrap();
            std::process::exit(1);
        } else {
            let sum_file = path.parent().unwrap().join("sum256");
            let mut sum_file = File::create(sum_file)?;
            sum_file.write_all(sum.as_bytes())?;
        }
    }

    let mut z_archive = zip::ZipArchive::new(z_file)?;
    match z_archive.extract(&path.parent().unwrap()) {
        Ok(_) => std::fs::remove_file(&path)?,
        Err(e) => panic!("{:?}", e),
    }
    Ok(())
}

/// Show the origin and local location, if any, of the currently highlighted item
/// lastly, show which resource will be used in the build
pub fn show_res_info(resources: &mut Resources, settings: &mut Settings, stdout: &mut Stdout) {
    let mut res_path: Option<PathBuf>;
    let mut ind_res = String::new();
    let bgc = &settings.bg_col_info;
    settings.res_name(&mut ind_res);
    let parent = resources.resource_list[&ind_res]["parent"]
        .as_str()
        .unwrap_or("");

    write!(
        stdout,
        "\r\n{}{}\r the first found resource will be used in the OUTPUT/EFI{}\r\n",
        "\x1b[4m",
        " ".repeat(size().unwrap().0.into()),
        bgc,
    )
    .unwrap();

    res_path = res_exists(&resources.working_dir_path, resources.input_dir_path.file_name().unwrap().to_str().unwrap(), &ind_res, stdout, bgc);

    let open_core_pkg = &resources.open_core_binaries_path;

    if res_path == None {
        let path;
        match settings.sec_key[0].as_str() {
            "ACPI" => {
                path = resources.octool_config["acpi_path"].as_str().unwrap();
            }
            "Misc" => {
                path = resources.octool_config["tools_path"].as_str().unwrap();
            }
            "UEFI" => {
                path = resources.octool_config["drivers_path"].as_str().unwrap();
            }
            _ => path = "",
        }
        res_path = res_exists(open_core_pkg, path, &ind_res, stdout, bgc);
    }

    if parent.len() > 0 {
        write!(stdout, "\x1B[2K\r\n").unwrap();
        write!(stdout, "{} in Dortania Builds? \x1B[0K", parent).unwrap();
        let res_index = settings
            .resource_ver_indexes
            .get(parent)
            .unwrap_or(&Manifest(0, "".to_string()))
            .0;
        match &resources.dortania[parent]["versions"][res_index]["links"][&settings.build_type] {
            serde_json::Value::String(url) => {
                write!(stdout, "\x1b[32mtrue\r\n").unwrap();
                let res = &resources.dortania[parent]["versions"][res_index];
                crossterm::terminal::disable_raw_mode().unwrap();
                write!(
                    stdout,
                    " {grn}url:{bgc} {}{clr}\r\n {grn}commit date/time:{bgc} {}{clr}\r\n {grn}message:{bgc} {}{clr}\r\n",
                    url,
                    res["date_committed"].as_str().unwrap_or(""),
                    res["commit"]["message"].as_str().unwrap_or("").lines().next().unwrap(),
                    grn = "\x1b[32m",
                    bgc = bgc,
                    clr = "\x1b[0K",
                )
                .unwrap();
                crossterm::terminal::enable_raw_mode().unwrap();

                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.dortania,
                        &settings.build_type,
                        &res_index,
                        false,
                        false,
                        stdout,
                        false,
                    )
                    .unwrap();
                }
            }
            _ => write!(stdout, "\x1b[31mfalse{}\x1b[0K\r\n", bgc).unwrap(),
        }

        write!(stdout, "\x1B[0K\r\n{} in other? \x1B[0K", parent).unwrap();
        match &resources.other[parent]["versions"][0]["links"][&settings.build_type] {
            serde_json::Value::String(url) => {
                write!(stdout, "\x1b[32mtrue\r\n").unwrap();
                write!(stdout, "{}{}\x1B[0K\r\n", url, bgc).unwrap();
                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.other,
                        &settings.build_type,
                        &0,
                        false,
                        false,
                        stdout,
                        false,
                    )
                    .unwrap();
                }
            }
            _ => write!(stdout, "\x1b[31mfalse{}\r\n", bgc).unwrap(),
        }
    } else {
        write!(
            stdout,
            "\x1B[33m{} not found in tool_config_files/resource_list.json, skipping prebuilt repos{}\x1B[0K\r\n"
        , &ind_res, bgc )
        .unwrap();
    }
    write!(stdout, "\x1B[2K\r\n").unwrap();
    match res_path {
        None => write!(stdout, "\x1B[31mNo local resource found{}\x1B[0K\r\n", bgc).unwrap(),
        Some(p) => {
            write!(
                stdout,
                "\x1B[32mlocal path to resource that will be used{}\x1B[0K\r\n",
                bgc
            )
            .unwrap();
            let mut out = None;
            for entry in WalkDir::new(p.parent().unwrap())
                .into_iter()
                .filter_map(Result::ok)
            {
                if entry.path().to_string_lossy().contains("_MAC") {
                    //ignore
                    continue;
                }
                let f_name = String::from(entry.file_name().to_string_lossy());
                if f_name == ind_res {
                    out = Some(entry);
                    break;
                }
            }
            match out {
                Some(outp) => {
                    let outpath = String::from(outp.path().to_string_lossy());
                    write!(stdout, "{:?}\x1b[0K\r\n", outpath).unwrap();
                    let respath = resources
                        .working_dir_path
                        .join(outpath)
                        .join("Contents/Info.plist");
                    if respath.exists() {
                        let info =
                            plist::Value::from_file(&respath).expect("got Value from Info.plist");
                        let cfbun = info.as_dictionary().unwrap().get("CFBundleIdentifier");
                        let cfver = info.as_dictionary().unwrap().get("CFBundleVersion");
                        let bunlib = info.as_dictionary().unwrap().get("OSBundleLibraries");
                        write!(
                            stdout,
                            "\x1b[2K\r\n\x1b[7mCFBundle\x1b[0m  {} {}\x1b[0K\r\n\x1b[2K\r\n",
                            cfbun.unwrap().as_string().unwrap_or(""),
                            cfver.unwrap().as_string().unwrap_or("")
                        )
                        .unwrap();
                        if !bunlib.is_none() {
                            match bunlib.unwrap() {
                                Value::Dictionary(d) => {
                                    let mut buns = vec![];
                                    for val in d.iter() {
                                        buns.push((
                                            val.0.to_owned(),
                                            val.1.as_string().unwrap().to_owned(),
                                        ));
                                    }
                                    for val in buns.iter() {
                                        if !val.0.contains("com.apple") {
                                            write!(
                                                stdout,
                                                "\x1b[7mrequires\x1b[0m  {} >= {}\x1b[0K\r\n",
                                                val.0, val.1,
                                            )
                                            .unwrap();
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => write!(
                    stdout,
                    "{:?} \x1b[33mwill be downloaded{}\x1b[0K\r\n",
                    p, bgc
                )
                .unwrap(),
            }
        }
    }
    write!(
        stdout,
        "\x1b[4m{}\x1B[0K",
        " ".repeat(size().unwrap().0.into())
    )
    .unwrap();
}

/// Read the `path` file into a `serde_json::Value`
pub fn get_serde_json_quiet(path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let file = File::open(Path::new(path))?;
    let buf = BufReader::new(file);
    let v = serde_json::from_reader(buf)?;
    Ok(v)
}

/// Read the `path` file into a `serde_json::Value`
pub fn get_serde_json(
    path: &str,
    stdout: &mut Stdout,
) -> Result<serde_json::Value, Box<dyn Error>> {
    write!(
        stdout,
        "\x1B[0K\n\x1B[32mLoading\x1B[0m {} ... \x1B[0K",
        path
    )?;
    stdout.flush()?;
    let file = File::open(Path::new(path))?;
    let buf = BufReader::new(file);
    let v = serde_json::from_reader(buf)?;
    write!(stdout, "\x1B[32mDone\x1B[0m\r\n")?;
    Ok(v)
}

fn res_exists(
    open_core_pkg: &PathBuf,
    path: &str,
    ind_res: &str,
    stdout: &mut Stdout,
    bgc: &String,
) -> Option<PathBuf> {
    let path = open_core_pkg.join(path).join(ind_res);
    if path.exists() {
        write!(
            stdout,
            "inside {:?} dir?\x1B[0K \x1b[32mtrue{}\r\n",
            path.parent().unwrap(),
            bgc,
        )
        .unwrap();
        Some(path)
    } else {
        write!(
            stdout,
            "inside {:?} dir?\x1B[0K \x1b[31mfalse{}\r\n",
            path.parent().unwrap(),
            bgc,
        )
        .unwrap();
        None
    }
}

/// version number of resource that will be used based on its manifest info
/// if no manifest info exists for the resource, it will be looked up and stored
pub fn res_version(settings: &mut Settings, resources: &Resources, res: &str) -> Option<String> {
    let mut ver = String::new();
    let res = res.split("/").last().unwrap_or("");
    if let Some(parent_res) = resources.resource_list[res]["parent"].as_str() {
        match settings.resource_ver_indexes.get(parent_res) {
            Some(p_manifest) => {
                if let Some(v) =
                    resources.dortania[parent_res]["versions"][p_manifest.0]["version"].as_str()
                {
                    ver = v.to_owned();
                } else {
                    ver = "".to_owned();
                }
            }
            None => {
                let mut p_index = 0;
                loop {
                    if let Some(date) =
                        resources.dortania[parent_res]["versions"][p_index]["date_built"].as_str()
                    {
                        if settings.oc_build_version_res_index == 0
                            || date[..10] <= settings.oc_build_date[..10]
                        {
                            settings.resource_ver_indexes.insert(
                                parent_res.to_owned(),
                                Manifest(
                                    p_index,
                                    resources.dortania[parent_res]["versions"][p_index]["commit"]
                                        ["sha"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                ),
                            );
                            if let Some(s) = resources.dortania[parent_res]["versions"][p_index]
                                ["version"]
                                .as_str()
                            {
                                ver = s.to_owned();
                            } else {
                                ver = "".to_owned();
                            }
                            break;
                        } else {
                            p_index += 1;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        Some(ver)
    } else {
        None
    }
}

/// this seems redundant to the `show_res_info` function, can I combine or eliminate?
pub fn get_res_path(
    settings: &Settings,
    resources: &Resources,
    ind_res: &str,
    section: &str,
    stdout: &mut Stdout,
    silent: bool,
) -> Option<String> {
    let mut from_input = false;
    let mut res_path: Option<PathBuf>;
    let parent = resources.resource_list[&ind_res]["parent"]
        .as_str()
        .unwrap_or("");
//    let mut path = resources.working_dir_path.join("INPUT").join(ind_res);
    let mut path = resources.input_dir_path.join(ind_res);
    if path.exists() {
        from_input = true;
        res_path = Some(path.clone());
    } else {
        res_path = None;
    }
    if res_path == None {
        match section {
            "ACPI" => {
                path = resources
                    .open_core_binaries_path
                    .join(resources.octool_config["acpi_path"].as_str().unwrap())
                    .join(&ind_res)
            }
            "Misc" => {
                path = resources
                    .open_core_binaries_path
                    .join(resources.octool_config["tools_path"].as_str().unwrap())
                    .join(&ind_res);
            }
            "UEFI" => {
                path = resources
                    .open_core_binaries_path
                    .join(resources.octool_config["drivers_path"].as_str().unwrap())
                    .join(&ind_res);
            }
            _ => (),
        }
        if path.exists() {
            res_path = Some(path);
        }
    }
    if res_path == None {
        res_path = get_or_update_local_parent(
            parent,
            &resources.dortania,
            &settings.build_type,
            &settings
                .resource_ver_indexes
                .get(parent)
                .unwrap_or(&Manifest(0, "".to_string()))
                .0,
            false,
            true,
            stdout,
            silent,
        )
        .unwrap();
    }
    if res_path == None {
        res_path = get_or_update_local_parent(
            parent,
            &resources.other,
            &settings.build_type,
            &0,
            false,
            true,
            stdout,
            silent,
        )
        .unwrap();
    }
    match res_path {
        None => None,
        Some(p) => {
            let mut out = None;
            for entry in WalkDir::new(p.parent().unwrap())
                .into_iter()
                .filter_map(Result::ok)
            {
                if entry.path().to_string_lossy().contains("_MAC") {
                    //ignore
                    continue;
                }
                let f_name = String::from(entry.file_name().to_string_lossy());
                if f_name == ind_res {
                    out = Some(entry);
                    break;
                }
            }
            match out {
                Some(outp) => {
                    let outp = String::from(outp.path().to_string_lossy());
                    if !silent {
                        if from_input {
                            write!(
                                stdout,
                                "\x1B[33mUsing \x1B[0m{}\x1B[33m copy from {} folder\x1B[0m\r\n",
                                ind_res,
                                resources.input_dir_path.file_name().unwrap().to_str().unwrap()
                            )
                            .unwrap();
                        } else {
                            write!(stdout, "{:?}\r\n", outp).unwrap();
                        }
                    }
                    Some(outp)
                }
                _ => None,
            }
        }
    }
}

pub fn get_latest_ver(resources: &Resources) -> Result<String, Box<dyn Error>> {
    let url = resources.octool_config["octool_latest_config_url"]
        .as_str()
        .expect("getting url from config");
    let mut data = Vec::new();
    let mut handle = Easy::new();
    handle.url(&url)?;
    {
        let mut transfer = handle.transfer();
        transfer.write_function(|new_data| {
            data.extend_from_slice(new_data);
            Ok(new_data.len())
        })?;
        transfer.perform()?;
    }
    let data: serde_json::Value = serde_json::from_slice(&data)?;
    match data["octool_version"].as_str() {
        Some(v) => Ok(v.to_string()),
        None => Ok("x.y.z".to_string()),
    }
}

pub fn merge_whole_plist(
    settings: &mut Settings,
    resources: &mut Resources,
    stdout: &mut Stdout,
    use_other_sample: bool,
) {
    let mut changed = false;
    let sample;
    if use_other_sample {
        write!(
            stdout,
            "\x1b[2K\r\n\x1b[2KEnter 'path of file' to Insert or drop file here: \x1b7\r\n\x1b[2K\x1b8"
        )
        .unwrap();
        let mut file_name = String::new();
        edit::edit_string(&mut file_name, None, stdout).unwrap();
        file_name = file_name.replace("\\ ", " ");
        let file_name = PathBuf::from(&file_name.trim());
        if file_name.exists() {
            sample = match Value::from_file(&file_name) {
                Ok(v) => v,
                Err(_) => {
                    write!(
                        stdout,
                        "\r\n\x1b[2K\x1b[31mERROR: \x1b[0m{:?} is not a valid plist file\r\n\x1b[2K",
                        file_name
                    )
                    .unwrap();
                    return;
                }
            };
        } else {
            write!(
                stdout,
                "\r\n\x1b[2K\x1b[31mERROR: \x1b[0mFile {:?} does not exist\r\n\x1b[2K",
                file_name
            )
            .unwrap();
            return;
        }
    } else {
        sample = resources.sample_plist.clone();
    }
    //    for sample_sec in resources.sample_plist.as_dictionary().unwrap() {
    for sample_sec in sample.as_dictionary().unwrap() {
        if sample_sec.0 == "DeviceProperties" {
            // do not modify DeviceProperties
            if !resources
                .config_plist
                .as_dictionary()
                .unwrap()
                .contains_key("DeviceProperties")
            {
                resources
                    .config_plist
                    .as_dictionary_mut()
                    .unwrap()
                    .insert("DeviceProperties".to_string(), sample_sec.1.clone());
                settings.sec_length[0] += 1;
            }
            continue;
        }
        match sample_sec.1 {
            plist::Value::Dictionary(_) => {
                let r = resources.config_plist.as_dictionary_mut().unwrap();
                if !r.contains_key(sample_sec.0) {
                    changed = true;
                    write!(
                        stdout,
                        "\r\n\x1b[7mAdded\x1b[0m {} section\x1b[0K",
                        sample_sec.0
                    )
                    .unwrap();
                    stdout.flush().unwrap();
                    let r = resources.config_plist.as_dictionary_mut().unwrap();
                    r.insert(sample_sec.0.to_string(), sample_sec.1.clone());
                    settings.sec_length[0] += 1;
                    r.sort_keys();
                };
                for sample_sub in sample_sec.1.as_dictionary().unwrap() {
                    let r = resources
                        .config_plist
                        .as_dictionary_mut()
                        .unwrap()
                        .get_mut(sample_sec.0)
                        .unwrap()
                        .as_dictionary_mut()
                        .unwrap();
                    if !r.contains_key(sample_sub.0) {
                        changed = true;
                        write!(
                            stdout,
                            "\r\n\x1b[7mAdded\x1b[0m {}->{} section\x1b[0K",
                            sample_sec.0, sample_sub.0
                        )
                        .unwrap();
                        stdout.flush().unwrap();
                        r.insert(sample_sub.0.to_string(), sample_sub.1.clone());
                        r.sort_keys();
                    }
                    match sample_sub.1 {
                        plist::Value::Dictionary(d) => {
                            for val in d {
                                let r = resources
                                    .config_plist
                                    .as_dictionary_mut()
                                    .unwrap()
                                    .get_mut(sample_sec.0)
                                    .unwrap()
                                    .as_dictionary_mut()
                                    .unwrap()
                                    .get_mut(sample_sub.0)
                                    .unwrap()
                                    .as_dictionary_mut()
                                    .unwrap();
                                if !r.contains_key(val.0) {
                                    changed = true;
                                    write!(
                                        stdout,
                                        "\r\n\x1b[7mAdded\x1b[0m {}->{}->{}\x1b[0K",
                                        sample_sec.0, sample_sub.0, val.0
                                    )
                                    .unwrap();
                                    stdout.flush().unwrap();
                                    r.insert(val.0.to_string(), val.1.clone());
                                    r.sort_keys();
                                }
                            }
                        }
                        plist::Value::Array(a) => {
                            if a.len() > 0 {
                                match &a[0] {
                                    plist::Value::Dictionary(sample_dict) => {
                                        if use_other_sample {
                                            //insert dict from other_sample into
                                            //config.plist
                                            for b in 0..a.len() {
                                                match &a[b] {
                                                    plist::Value::Dictionary(sample_all) => {
                                                        resources
                                                            .config_plist
                                                            .as_dictionary_mut()
                                                            .unwrap()
                                                            .get_mut(sample_sec.0)
                                                            .unwrap()
                                                            .as_dictionary_mut()
                                                            .unwrap()
                                                            .get_mut(sample_sub.0)
                                                            .unwrap()
                                                            .as_array_mut()
                                                            .unwrap()
                                                            .insert(
                                                                0,
                                                                Value::Dictionary(
                                                                    sample_all.clone(),
                                                                ),
                                                            );
                                                        changed = true;
                                                        write!(stdout,
                                                               "\r\n\x1b[7mAdded\x1b[0m {}->{} item\x1b[0K"
                                                               ,sample_sec.0, sample_sub.0).unwrap();
                                                        stdout.flush().unwrap();
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                        for (i, item) in resources
                                            .config_plist
                                            .as_dictionary_mut()
                                            .unwrap()
                                            .get_mut(sample_sec.0)
                                            .unwrap()
                                            .as_dictionary_mut()
                                            .unwrap()
                                            .get_mut(sample_sub.0)
                                            .unwrap()
                                            .as_array_mut()
                                            .unwrap()
                                            .iter_mut()
                                            .enumerate()
                                        {
                                            match item {
                                                plist::Value::Dictionary(_) => {
                                                    for val in sample_dict {
                                                        if !item
                                                            .as_dictionary()
                                                            .unwrap()
                                                            .contains_key(&val.0)
                                                        {
                                                            changed = true;
                                                            write!(
                                                        stdout,
                                                        "\r\n\x1b[7mAdded\x1b[0m {}->{}->{}->{}\x1b[0K",
                                                        sample_sec.0, sample_sub.0, i, val.0
                                                    )
                                                    .unwrap();
                                                            stdout.flush().unwrap();
                                                            item.as_dictionary_mut()
                                                                .unwrap()
                                                                .insert(
                                                                    val.0.to_string(),
                                                                    val.1.clone(),
                                                                );
                                                            item.as_dictionary_mut()
                                                                .unwrap()
                                                                .sort_keys();
                                                        }
                                                    }
                                                }
                                                _ => (),
                                            } // end match item
                                        }
                                    }
                                    _ => (),
                                } // end match a[0]
                            };
                        }
                        _ => (),
                    } // end match sample_sub.1
                }
            }
            _ => (),
        } // end match sample_sec.1
    }
    if !changed {
        write!(
            stdout,
            "\r\n\x1b[33mNo additions made to config.plist\x1b[0m\x1b[0K"
        )
        .unwrap();
    } else {
        settings.modified = true;
    }
    write!(stdout, "\r\n\x1b[2K").unwrap();
}

pub fn purge_whole_plist(settings: &mut Settings, resources: &mut Resources, stdout: &mut Stdout) {
    let mut changed = false;
    let mut items: Vec<Vec<String>> = Default::default();
    for config_sec in resources.config_plist.as_dictionary().unwrap() {
        if config_sec.0 == "DeviceProperties" {
            // do not modify DeviceProperties
            continue;
        }
        match config_sec.1 {
            plist::Value::Dictionary(_) => {
                let r = resources.sample_plist.as_dictionary().unwrap();
                if !r.contains_key(config_sec.0) {
                    changed = true;
                    items.push(vec![config_sec.0.to_owned()]);
                    break;
                };
                for config_sub in config_sec.1.as_dictionary().unwrap() {
                    let r = resources
                        .sample_plist
                        .as_dictionary()
                        .unwrap()
                        .get(config_sec.0)
                        .unwrap()
                        .as_dictionary()
                        .unwrap();
                    if !r.contains_key(config_sub.0) {
                        changed = true;
                        items.push(vec![config_sec.0.to_owned(), config_sub.0.to_owned()]);
                    }
                    match config_sub.1 {
                        plist::Value::Dictionary(d) => {
                            for val in d {
                                let r = resources
                                    .sample_plist
                                    .as_dictionary()
                                    .unwrap()
                                    .get(config_sec.0)
                                    .unwrap()
                                    .as_dictionary()
                                    .unwrap()
                                    .get(config_sub.0)
                                    .unwrap()
                                    .as_dictionary()
                                    .unwrap();
                                if !r.contains_key(val.0) {
                                    changed = true;
                                    items.push(vec![
                                        config_sec.0.to_owned(),
                                        config_sub.0.to_owned(),
                                        val.0.to_owned(),
                                    ]);
                                }
                            }
                        }
                        plist::Value::Array(a) => {
                            if a.len() > 0 {
                                for a_index in 0..a.len() {
                                    match &a[a_index] {
                                        plist::Value::Dictionary(d) => {
                                            if let plist::Value::Dictionary(sam) = &resources
                                                .sample_plist
                                                .as_dictionary()
                                                .unwrap()
                                                .get(config_sec.0)
                                                .unwrap()
                                                .as_dictionary()
                                                .unwrap()
                                                .get(config_sub.0)
                                                .unwrap()
                                                .as_array()
                                                .unwrap()[0]
                                            {
                                                for val in d {
                                                    if !sam.contains_key(&val.0) {
                                                        changed = true;
                                                        items.push(vec![
                                                            config_sec.0.to_owned(),
                                                            config_sub.0.to_owned(),
                                                            a_index.to_string(),
                                                            val.0.to_owned(),
                                                        ]);
                                                    }
                                                }
                                            }
                                        }
                                        _ => (),
                                    } // end match a[a_index]
                                }
                            };
                        }
                        _ => (),
                    } // end match sample_sub.1
                }
            }
            _ => (),
        } // end match sample_sec.1
    }
    if !changed {
        write!(
            stdout,
            "\r\n\x1b[33mNo deletions made to config.plist\x1b[0m\x1b[0K"
        )
        .unwrap();
    } else {
        //        write!(stdout, "{:?}", items).unwrap();
        //        stdout.flush().unwrap();
        for item in items {
            let mut del = resources.config_plist.as_dictionary_mut().unwrap();
            write!(stdout, "\r\n\x1b[7mRemoved\x1b[0m ").unwrap();
            for i in 0..item.len() - 1 {
                if i == 1 && item.len() == 4 {
                    del = del.get_mut(&item[1]).unwrap().as_array_mut().unwrap()
                        [item[2].parse::<usize>().unwrap()]
                    .as_dictionary_mut()
                    .unwrap();
                    write!(stdout, "{}->{}->", item[1], item[2]).unwrap();
                } else if (i != 2 && item.len() == 4) || item.len() != 4 {
                    del = del.get_mut(&item[i]).unwrap().as_dictionary_mut().unwrap();
                    write!(stdout, "{}->", item[i]).unwrap();
                }
            }
            del.remove(&item[item.len() - 1]);
            write!(stdout, "{}", item[item.len() - 1]).unwrap();
        }
        settings.modified = true;
    }
    write!(stdout, "\r\n\x1b[2K").unwrap();
    stdout.flush().unwrap();
}

//return true if order is okay
pub fn check_order(
    settings: &mut Settings,
    resources: &mut Resources,
    stdout: &mut Stdout,
    check_only: bool,
) -> bool {
    let mut bundle_list = vec![];
    let mut kext_list = vec![];
    //run through Kernel Add section and add to kext_list
    if !resources
        .config_plist
        .as_dictionary()
        .unwrap()
        .contains_key("Kernel")
    {
        return true;
    }
    let kernel_add_section = resources
        .config_plist
        .as_dictionary_mut()
        .unwrap()
        .get_mut("Kernel")
        .unwrap()
        .as_dictionary_mut()
        .unwrap()
        .get_mut("Add")
        .unwrap()
        .as_array_mut()
        .unwrap();

    //build list of kexts from Kernel > Add Section
    for res in kernel_add_section {
        let bundle_path = res
            .as_dictionary()
            .unwrap()
            .get("BundlePath")
            .unwrap()
            .as_string()
            .unwrap_or("")
            .to_owned();
        let plist_path = res
            .as_dictionary()
            .unwrap()
            .get("PlistPath")
            .unwrap()
            .as_string()
            .unwrap_or("")
            .to_owned();
        let res_enabled = res
            .as_dictionary()
            .unwrap()
            .get("Enabled")
            .unwrap()
            .as_boolean()
            .unwrap_or(false);
        let mut new_res = (
            bundle_path.split('/').last().unwrap_or("").to_string(),
            plist_path,
            "Unknown".to_owned(),
            res_enabled,
        );
        if kext_list.contains(&new_res) && res_enabled {
            write!(
                stdout,
                "\x1b[2KResource {} already enabled!!\r\n",
                new_res.0
            )
            .unwrap();
            if !check_only {
                write!(stdout, "\x1b[2KDisabling duplicate\r\n").unwrap();
                match res {
                    Value::Boolean(b) => *b = !*b,
                    Value::Dictionary(d) => match d.get_mut("Enabled") {
                        Some(Value::Boolean(b)) => *b = !*b,
                        _ => (),
                    },
                    _ => (),
                }
            }
            new_res.3 = false;
        }
        kext_list.push(new_res);
    }

    //add version numbers to list from the res_version manifest
    for i in 0..kext_list.len() {
        kext_list[i].2 = res_version(settings, resources, &kext_list[i].0).unwrap_or("".to_owned());
    }

    #[cfg(debug_assertions)]
    {
        write!(stdout, "\x1b[0J{} {:?}\r\n", kext_list.len(), kext_list).unwrap();
    }

    //iterate kext_list and build bundle_list
    for (res_bundle, plist_path, _, _) in kext_list.iter() {
        match get_res_path(
            &settings,
            &resources,
            &res_bundle.split('/').last().unwrap(),
            "Kernel",
            stdout,
            true,
        ) {
            //found path to resource - check for Info.plist
            Some(path) => {
                let info_path = PathBuf::from(path).join(plist_path);
                if info_path.exists() {
                    let info =
                        plist::Value::from_file(&info_path).expect("getting Value from Info.plist");
                    let cfbundle_id = info
                        .as_dictionary()
                        .unwrap()
                        .get("CFBundleIdentifier")
                        .unwrap()
                        .as_string()
                        .unwrap();
                    let cfbundle_version = info
                        .as_dictionary()
                        .unwrap()
                        .get("CFBundleVersion")
                        .unwrap()
                        .as_string()
                        .unwrap();

                    let os_bundle_lib = info.as_dictionary().unwrap().get("OSBundleLibraries");
                    if os_bundle_lib.is_some() {
                        match os_bundle_lib.unwrap() {
                            plist::Value::Dictionary(d) => {
                                let mut lib_children = vec![];
                                for val in d.iter() {
                                    if !val.0.contains("com.apple") {
                                        //add requirement if it is
                                        //not from apple
                                        lib_children.push((
                                            val.0.to_owned(),
                                            val.1.as_string().unwrap().to_owned(),
                                        ));
                                    }
                                }
                                bundle_list.push((
                                    cfbundle_id.to_string(),
                                    cfbundle_version.to_string(),
                                    lib_children.clone(),
                                ));
                            }
                            _ => {}
                        }
                    } else {
                        bundle_list.push((
                            cfbundle_id.to_string(),
                            cfbundle_version.to_string(),
                            vec![],
                        ));
                    }
                } else {
                    bundle_list.push(("bad_path".to_string(), "".to_string(), vec![]));
                }
            }
            //didn't find path to resource
            _ => {
                bundle_list.push(("pathless".to_string(), "".to_string(), vec![]));
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        write!(stdout, "\r\n{} {:?}\r\n", bundle_list.len(), bundle_list).unwrap();
    }

    //enable or add requirements
    for (i, bundle) in bundle_list.iter().enumerate() {
        if kext_list[i].3 {
            if bundle.2.len() > 0 {
                //has requirements
                for (j, required) in bundle.2.iter().enumerate() {
                    let mut requirement_exists = false;
                    let mut requirement_index = 0;
                    let mut requirement_out_of_order = false;
                    for k in 0..bundle_list.len() {
                        if required.0 == bundle_list[k].0 {
                            requirement_exists = true;
                            requirement_index = k;
                            if k > i {
                                requirement_out_of_order = true;
                            }
                            if kext_list[k].3 {
                                break;
                                //found enabled requirement
                            }
                        }
                    }
                    if requirement_exists {
                        if !kext_list[requirement_index].3 {
                            if !check_only {
                                write!(
                                    stdout,
                                    " \x1b[32menabling\x1b[0m {} for {}\x1b[0K\r\n",
                                    kext_list[requirement_index].0, bundle.0
                                )
                                .unwrap();
                                //enable the requirement
                                resources
                                    .config_plist
                                    .as_dictionary_mut()
                                    .unwrap()
                                    .get_mut("Kernel")
                                    .unwrap()
                                    .as_dictionary_mut()
                                    .unwrap()
                                    .get_mut("Add")
                                    .unwrap()
                                    .as_array_mut()
                                    .unwrap()[requirement_index]
                                    .as_dictionary_mut()
                                    .unwrap()
                                    .insert("Enabled".to_string(), plist::Value::Boolean(true));
                            }
                            return false;
                        }
                        if requirement_out_of_order {
                            if !check_only {
                                write!(
                                    stdout,
                                    " \x1b[32mfixing\x1b[0m {} order for {}\x1b[0K\r\n",
                                    kext_list[requirement_index].0, bundle.0
                                )
                                .unwrap();
                                //place resource after requirement then remove resource from current
                                //location
                                let kernel_add_array = resources
                                    .config_plist
                                    .as_dictionary_mut()
                                    .unwrap()
                                    .get_mut("Kernel")
                                    .unwrap()
                                    .as_dictionary_mut()
                                    .unwrap()
                                    .get_mut("Add")
                                    .unwrap()
                                    .as_array_mut()
                                    .unwrap();
                                let held_kext = kernel_add_array[i].clone();
                                kernel_add_array.insert(requirement_index + 1, held_kext);
                                kernel_add_array.remove(i);
                            }
                            return false;
                        }
                    } else {
                        if !check_only {
                            write!(
                                stdout,
                                "  \x1b[32madding\x1b[0m requirement {} for {}\x1b[0K\r\n",
                                required.0, bundle.0
                            )
                            .unwrap();
                            //add requirement
                            let mut item_to_add =
                                bundle_list[i].2[j].0.split('.').last().unwrap().to_owned();
                            //hack fix for PS2Controller being in the VoodooPS2Controller.kext
                            if item_to_add == "PS2Controller" {
                                item_to_add = "VoodooPS2Controller".to_string();
                            }
                            //another hack to point BrcmStore to BrcmRepo
                            if item_to_add == "BrcmFirmwareStore" {
                                item_to_add = "BrcmFirmwareRepo".to_string();
                            }
                            item_to_add.push_str(".kext");
                            add_item(settings, resources, &item_to_add, stdout);
                        }
                        return false;
                    }
                }
            }
        }
    }
    true
}
