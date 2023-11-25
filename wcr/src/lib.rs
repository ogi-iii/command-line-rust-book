use std::{error::Error, io::{BufRead, stdin, BufReader}, fs::File};

use clap::{App, Arg};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: bool,
    words: bool,
    bytes: bool,
    chars: bool,
}

#[derive(Debug, PartialEq)]
pub struct FileInfo {
    num_lines: usize,
    num_words: usize,
    num_bytes: usize,
    num_chars: usize,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("wcr")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust wc")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .default_value("-")
                .multiple(true),
        )
        .arg(
            Arg::with_name("lines")
                .short("l")
                .long("lines")
                .help("Show line count")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("words")
                .short("w")
                .long("words")
                .help("Show word count")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .help("Show byte count")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("chars")
                .short("m")
                .long("chars")
                .help("Show character count")
                .takes_value(false)
                .conflicts_with("bytes"),
        )
        .get_matches();

    let mut lines = matches.is_present("lines");
    let mut words = matches.is_present("words");
    let mut bytes = matches.is_present("bytes");
    let chars = matches.is_present("chars");

    // if [words, bytes, chars, lines].iter().all(|v| v == &false) { // boolの参照を比較: 全てfalseの参照ならば条件に一致と判定
    if [words, bytes, chars, lines].iter().all(|v| !v) {
            // 全てのフラグが未指定の場合のデフォルト設定
        lines = true;
        words = true;
        bytes = true;
    }

    Ok(
        Config {
            files: matches.values_of_lossy("files").unwrap(),
            lines,
            words,
            bytes,
            chars
        }
    )
}

pub fn run(config: Config) -> MyResult<()> {
    let mut total_num_lines = 0;
    let mut total_num_words = 0;
    let mut total_num_bytes = 0;
    let mut total_num_chars = 0;

    for filename in &config.files {
        match open(filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(file) => {
                if let Ok(info) = count(file) {
                    println!(
                        "{}{}{}{}{}",
                        format_field(info.num_lines, config.lines),
                        format_field(info.num_words, config.words),
                        format_field(info.num_bytes, config.bytes),
                        format_field(info.num_chars, config.chars),
                        if filename == "-" {
                            "".to_string()
                        } else {
                            format!(" {}", filename)
                        }
                    );
                    total_num_lines += info.num_lines;
                    total_num_words += info.num_words;
                    total_num_bytes += info.num_bytes;
                    total_num_chars += info.num_chars;
                }
            },
        }
    }

    if config.files.len() > 1 {
        println!(
            "{}{}{}{} total",
            format_field(total_num_lines, config.lines),
            format_field(total_num_words, config.words),
            format_field(total_num_bytes, config.bytes),
            format_field(total_num_chars, config.chars),
        );
    }

    Ok(())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn count(mut file: impl BufRead) -> MyResult<FileInfo> {
    let mut num_lines = 0;
    let mut num_words = 0;
    let mut num_bytes = 0;
    let mut num_chars = 0;

    let mut line = String::new();

    loop {
        let line_bytes = file.read_line(&mut line)?; // バイト配列としてバッファに読み込む: 改行コードも含めるため
        if line_bytes == 0 {
            break; // EOF
        }
        num_lines += 1;
        num_words += line.split_whitespace().count(); // 空白文字の区切りでカウント
        num_bytes += line_bytes;
        num_chars += line.chars().count(); // Unicode文字の区切りでカウント

        line.clear();
    }

    Ok(
        FileInfo {
            num_lines,
            num_words,
            num_bytes,
            num_chars
        }
    )
}

fn format_field(value: usize, show: bool) -> String { // 可変なので&strではなくStringを返す
    if show {
        format!("{:>8}", value) // 右寄せ8文字のString
    } else {
        "".to_string()
    }
}

#[cfg(test)] // testの時のみにコンパイルされる
mod tests {
// testsモジュールとして定義
    use super::{count, format_field, FileInfo}; // 親モジュール(wcr)からインポート
    use std::io::Cursor;

    #[test]
    fn test_count() {
        let text = "I don't want the world. I just want your half.\r\n";
        let info = count(
            Cursor::new(text) // Read,Writeを実装するバッファに文字列を格納: テスト用の擬似ファイルハンドラとして利用
        );
        assert!(info.is_ok());
        let expected = FileInfo {
            num_lines: 1,
            num_words: 10,
            num_bytes: 48,
            num_chars: 48,
        };
        assert_eq!(info.unwrap(), expected); // 内部要素を部分比較: PartialEqを実装しているため
    }

    #[test]
    fn test_format_field() {
        assert_eq!(format_field(1, false), "");
        assert_eq!(format_field(3, true), "       3");
        assert_eq!(format_field(10, true), "      10");
    }
}
