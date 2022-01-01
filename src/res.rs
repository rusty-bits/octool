use crate::init::{Manifest, Settings};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use curl::easy::Easy;
use walkdir::WalkDir;

use sha2::Digest;

// TODO: much of this module needs refactoring, well, I'm learning
// TODO: many of the functions seem redundant and need improvement, soon, lol
pub struct Resources {
    pub dortania: serde_json::Value, // Dortania builds config.json file
    pub octool_config: serde_json::Value, // config file for octool itself
    pub resource_list: serde_json::Value, // list linking resources to their parents
    pub other: serde_json::Value,    // list of other party parent/childs
    pub config_plist: plist::Value,  // current active config.plist
    pub sample_plist: plist::Value,  // latest Sample.plist
    pub working_dir_path: PathBuf,   // location of octool and files
    pub open_core_binaries_path: PathBuf, // location of the OpenCorePkg binariesg
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
                            write!(
                                stdout,
                                "remote hash {}\x1B[0K\r\n  local sum {}\x1B[0K\r\n",
                                hash, sum
                            )?;
                            write!(
                                stdout,
                                "{yel}new version found, {grn}Downloading\x1B[0m\x1B[0K\r\n",
                                yel = "\x1b[33m",
                                grn = "\x1b[32m",
                            )?;
                            get_file_and_unzip(url, hash, &path, stdout)?;
                        } else {
                            if verbose {
                                write!(stdout, "Already up to date.\x1B[0K\r\n")?;
                            }
                        }
                    }
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        if do_update {
                            write!(
                            stdout,
                            "{:?} {yel}local copy not found, {grn}Downloading\x1B[0m\x1B[0K\r\n",
                            dir,
                            yel = "\x1b[33m",
                            grn = "\x1b[32m",
                        )?;
                            write!(stdout, "remote hash {}\x1B[0K\r\n", hash)?;
                            get_file_and_unzip(url, hash, &path, stdout)?;
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
    let current_sha = current_sha["commit"]["sha"].as_str().unwrap();
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
    let size = size["tree"][0]["size"].as_i64().unwrap();
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
    write!(stdout, "\x1b[32mdone\x1b[0m\r\n")?;
    Ok(())
}

pub fn get_file_and_unzip(
    url: &str,
    hash: &str,
    path: &Path,
    stdout: &mut Stdout,
) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(path.parent().unwrap())?;

    curl_file(&url, &path)?;

    let mut z_file = File::open(path)?;
    if hash != "" {
        let mut data = Vec::new();
        z_file.read_to_end(&mut data).unwrap();
        let sum = format!("{:x}", sha2::Sha256::digest(&data));
        write!(stdout, "  local sum {}\x1B[0K\r\n", sum)?;
        if sum != hash {
            panic!("Sum of {:?} does not match {}", path, hash);
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
pub fn show_res_info(resources: &Resources, settings: &Settings, stdout: &mut Stdout) {
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
        " ".repeat(crossterm::terminal::size().unwrap().0.into()),
        bgc,
    )
    .unwrap();

    res_path = res_exists(&resources.working_dir_path, "INPUT", &ind_res, stdout, bgc);

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
                let f_name = String::from(entry.file_name().to_string_lossy());
                if f_name == ind_res {
                    out = Some(entry);
                    break;
                }
            }
            match out {
                Some(outp) => {
                    let outp = String::from(outp.path().to_string_lossy());
                    write!(stdout, "{:?}\x1b[0K\r\n", outp).unwrap();
                }
                _ => write!(
                    stdout,
                    "{:?} \x1b[32mwill be downloaded{}\x1b[0K\r\n",
                    p, bgc
                )
                .unwrap(),
            }
        }
    }
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
    write!(stdout, "\x1B[32mdone\x1B[0m\r\n")?;
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

pub fn res_version(settings: &mut Settings, resources: &Resources, res: &str) -> String {
    let mut ver = String::new();
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
        ver
    } else {
        "".to_owned()
    }
}

/// this seems redundant to the `show_res_info` function, can I combine or eliminate?
pub fn get_res_path(
    settings: &Settings,
    resources: &Resources,
    ind_res: &str,
    section: &str,
    stdout: &mut Stdout,
) -> Option<String> {
    let mut from_input = false;
    let mut res_path: Option<PathBuf>;
    let parent = resources.resource_list[&ind_res]["parent"]
        .as_str()
        .unwrap_or("");
    let mut path = resources.working_dir_path.join("INPUT").join(ind_res);
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
                let f_name = String::from(entry.file_name().to_string_lossy());
                if f_name == ind_res {
                    out = Some(entry);
                    break;
                }
            }
            match out {
                Some(outp) => {
                    let outp = String::from(outp.path().to_string_lossy());
                    if from_input {
                        write!(
                            stdout,
                            "\x1B[33mUsing \x1B[0m{}\x1B[33m copy from INPUT folder\x1B[0m\r\n",
                            ind_res
                        )
                        .unwrap();
                    } else {
                        write!(stdout, "{:?}\r\n", outp).unwrap();
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

pub fn merge_whole_plist(settings: &mut Settings, resources: &mut Resources, stdout: &mut Stdout) {
    let mut changed = false;
    for sample_sec in resources.sample_plist.as_dictionary().unwrap() {
        if sample_sec.0 == "DeviceProperties" {
            // do not modify DeviceProperties
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
    stdout.flush().unwrap();
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
