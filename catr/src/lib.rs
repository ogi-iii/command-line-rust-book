use std::{error::Error, io::{BufRead, BufReader, stdin}, fs::File};

use clap::{App, Arg};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    number_lines: bool,
    number_nonblank_lines: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("catr")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust cat")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .multiple(true)
                .default_value("-"),
        )
        .arg(
            Arg::with_name("number")
                .short("n")
                .long("number")
                .help("Number lines")
                .takes_value(false)
                .conflicts_with("number_nonblank"), // -b|--number-nonblank との併用を防ぐ: ErrorKind::ArgumentConflict としてエラーになる
        )
        .arg(
            Arg::with_name("number_nonblank")
                .short("b")
                .long("number-nonblank")
                .help("Number non-blank lines")
                .takes_value(false),
        )
        .get_matches();

    Ok(
        Config {
            files: matches.values_of_lossy("files").unwrap(), // value"s"_of_lossy() を使うこと: value_of_lossy() は単一Stringを返す
            number_lines: matches.is_present("number"),
            number_nonblank_lines: matches.is_present("number_nonblank"),
        }
    )
}

pub fn run(config: Config) -> MyResult<()> {
    // dbg!(config);
    for filename in config.files {
        // println!("{}", filename);
        match open(&filename) {
            Err(err) => eprintln!("Failed to open {}: {}", filename, err),
            Ok(file) => {
                // println!("Opened {}", filename)
                let mut nonblank_line_num = 0;
                for (line_num, line_result) in file.lines().enumerate() { // (index, 文字列) でループ処理
                    let line = line_result?;
                    // println!("{}", line);
                    if config.number_lines {
                        println!("{:>6}\t{}", line_num + 1, line); // 行数の桁が違っても表記がズレないように調整: 6桁表記で先頭空白埋め(数値は右寄せ)
                    } else if config.number_nonblank_lines {
                        if !line.is_empty() {
                            nonblank_line_num += 1;
                            println!("{:>6}\t{}", nonblank_line_num, line);
                        } else {
                            println!(); // 空白行は番号を付与せずにそのまま出力
                        }
                    } else {
                        println!("{}", line);
                    }
                }
            },
        }
    }
    Ok(())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> { // MyResult<dyn BufRead> だとサイズが固定できないため、Boxでヒープに格納する
    match filename {
        "-" => Ok(Box::new(BufReader::new(stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}
