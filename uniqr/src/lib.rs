use std::{error::Error, io::{BufRead, BufReader, Write, stdin, stdout}, fs::File};

use clap::{App, Arg};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    in_file: String,
    out_file: Option<String>,
    count: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("uniqr")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust uniq")
        .arg(
            Arg::with_name("in_file")
                .value_name("IN_FILE")
                .help("Input file")
                .default_value("-"),
        )
        .arg(
            Arg::with_name("out_file")
                .value_name("OUT_FILE")
                .help("Output file"),
        )
        .arg(
            Arg::with_name("count")
                .short("c")
                .long("count")
                .help("Show counts")
                .takes_value(false),
        )
        .get_matches();

    Ok(
        Config {
            in_file: matches.value_of_lossy("in_file").map(Into::into).unwrap(),
            out_file: matches.value_of_lossy("out_file").map(String::from), // Optionのまま中身をCowからStringに変換
            count: matches.is_present("count")
        }
    )
}

pub fn run(config: Config) -> MyResult<()> {
    let mut file = open(&config.in_file)
        .map_err(|e| format!("{}: {}", config.in_file, e))?;

    let mut out_file: Box<dyn Write> = match &config.out_file {
        Some(out_filename) => Box::new(File::create(out_filename)?),
        _ => Box::new(stdout()),
    };

    // mutableでなければコンパイルエラーになる: (外部から所有している)out_fileの内容が(追記されるごとに)変化するため
    let mut write = |count: u64, text: &str| -> MyResult<()> {
        if count > 0 {
            if config.count {
                write!(out_file, "{:>4} {}", count, text)?;
            } else {
                write!(out_file, "{}", text)?;
            }
        };
        Ok(())
    };

    let mut line = String::new();
    let mut previous = String::new();
    let mut count: u64 = 0;

    loop {
        let bytes = file.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        if line.trim_end() != previous.trim_end() {
            // if count > 0 { // 先頭行で即出力されないように条件分岐
            //     print!("{:>4} {}", count, previous);
            // }
            write(count, &previous)?;
            previous = line.clone();
            count = 0; // カウントをリセット
        }
        count += 1;
        line.clear();
    }

    // if count > 0 { // 先頭行と最終行が出力されないことを防止するために条件分岐
    //     print!("{:>4} {}", count, previous);
    // }
    write(count, &previous)?;

    Ok(())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?)))
    }
}
