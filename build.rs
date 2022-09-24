use std::{env, fs, io::Write};

use chrono::Datelike;

fn main() {
    let outdir = env::var("OUT_DIR").unwrap();
    let outfile = format!("{}/version.txt", outdir);

    let mut fh = fs::File::create(&outfile).unwrap();

    let now = chrono::Local::now();
    write!(
        fh,
        r#""v{} {}-{:02}-{:02}""#,
        env!("CARGO_PKG_VERSION"),
        now.year(),
        now.month(),
        now.day()
    )
    .ok();
}
