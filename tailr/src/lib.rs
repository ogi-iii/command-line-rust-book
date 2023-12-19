use std::{error::Error, fs::File, io::{BufRead, Read, Seek, BufReader, SeekFrom}};

use clap::{App, Arg};
use once_cell::sync::OnceCell;
use regex::Regex;

use crate::TakeValue::*;

type MyResult<T> = Result<T, Box<dyn Error>>;

// 再利用可能な正規表現をstatic変数で定義: constはコンパイル時に値が決まる変数、staticはコンパイル時に(値の)格納先が決まる変数
static NUM_RE: OnceCell<Regex> = OnceCell::new();

#[derive(Debug, PartialEq)]
enum TakeValue {
    PlusZero,
    TakeNum(i64),
}

pub struct Config {
    files: Vec<String>,
    lines: TakeValue,
    bytes: Option<TakeValue>,
    quiet: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("tailr")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust tail")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .required(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("lines")
                .short("n")
                .long("lines")
                .value_name("LINES")
                .help("Number of lines")
                .default_value("10"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .value_name("BYTES")
                .help("Number of bytes")
                .conflicts_with("lines"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Suppress headers"),
        )
        .get_matches();

    let lines = matches.value_of("lines")
        .map(parse_num)
        .transpose()
        .map_err(|e| format!("illegal line count -- {}", e))?;

    let bytes = matches.value_of("bytes")
        .map(parse_num)
        .transpose()
        .map_err(|e| format!("illegal byte count -- {}", e))?;

    Ok(
        Config {
            files: matches.values_of_lossy("files").unwrap(),
            lines: lines.unwrap(),
            bytes,
            quiet: matches.is_present("quiet"),
        }
    )
}

fn parse_num(val: &str) -> MyResult<TakeValue> {
    // OnceCellから正規表現を取得または初期化
    let num_re = NUM_RE
        // 符号または符号無しと、1以上の数値を抽出する正規表現: ?はゼロ文字以上の繰り返し
        .get_or_init(|| Regex::new(r"^([+-])?(\d+)$").unwrap());
    match num_re.captures(val) {
        Some(caps) => {
            // Someならstrに、Noneならデフォルト値に変換
            let sign = caps.get(1).map_or("-", |m| m.as_str());
            let num = format!("{}{}", sign, caps.get(2).unwrap().as_str()); // 符号付き数値の文字列
            if let Ok(val) = num.parse() { // String -> i64 に変換: 返り値の型から自動推論される
                if sign == "+" && val == 0 {
                    Ok(PlusZero)
                } else {
                    Ok(TakeNum(val))
                }
            } else {
                Err(From::from(val)) // 数値valでエラーを返す
            }
        },
        _ => Err(From::from(val)), // 文字列valでエラーを返す
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let num_files = config.files.len();
    for (file_num, filename) in config.files.iter().enumerate() {
        // stdinは非対応なので、ファイルとして直接開く
        match File::open(&filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(file) => {
                if !config.quiet && num_files > 1 {
                    println!(
                        "{}==> {} <==",
                        if file_num > 0 {
                            "\n"
                        } else {
                            ""
                        },
                        filename,
                    );
                }
                let (total_lines, total_bytes) = count_lines_bytes(&filename)?;
                let file = BufReader::new(file);
                if let Some(num_bytes) = &config.bytes {
                    print_bytes(file, num_bytes, total_bytes)?;
                } else {
                    print_lines(file, &config.lines, total_lines)?;
                }
            },
        }
    }
    Ok(())
}

fn count_lines_bytes(filename: &str) -> MyResult<(i64, i64)> {
    let mut file = BufReader::new(File::open(filename)?);
    let mut num_lines = 0;
    let mut num_bytes = 0;
    let mut buf = vec![]; // 空のバイト配列
    loop {
        // 改行コードまでバイト配列として読み込む
        let bytes_read = file.read_until(b'\n', &mut buf)?;
        if bytes_read == 0 {
            break;
        }
        num_lines += 1;
        num_bytes += bytes_read as i64;
        buf.clear();
    }
    Ok((num_lines, num_bytes))
}

// 非負のインデックス番号があれば返す: なければNone
fn get_start_index(take_val: &TakeValue, total: i64) -> Option<u64> {
    match take_val {
        PlusZero => {
            if total > 0 {
                Some(0) // ファイル先頭
            } else {
                None // 空ファイルの時
            }
        },
        TakeNum(num) => {
            if num == &0 || total == 0 || num > &total {
                None // 意図的な出力ゼロ、空ファイル、ファイル末尾以降のインデックス位置の時
            } else {
                let start = if num < &0 {
                    total + num // 末尾からのインデックス
                } else {
                    num - 1 // 先頭からのインデックス
                };
                Some(if start < 0 {
                    0 // ファイル先頭
                } else {
                    start as u64 // 任意のインデックス位置
                })
            }
        },
    }
}

// BufReadを実装するファイルを受け取る
fn print_lines(mut file: impl BufRead, num_lines: &TakeValue, total_lines: i64) -> MyResult<()> {
    // インデックス位置がNoneでなければ出力処理を開始
    if let Some(start) = get_start_index(num_lines, total_lines) {
        let mut line_num = 0;
        let mut buf = vec![];
        loop {
            let byte_read = file.read_until(b'\n', &mut buf)?; // 行単位でバイト配列を取得
            if byte_read == 0 {
                break;
            }
            if line_num >= start { // インデックス位置以降であれば出力
                print!("{}", String::from_utf8_lossy(&buf));
            }
            line_num += 1;
            buf.clear()
        }
    }
    Ok(())
}

// ReadとSeek(カーソルと同義)を実装するジェネリクス型のファイルを受け取る: 返り値の前で where T: Read + Seek でもOK
fn print_bytes<T: Read + Seek>(mut file: T, num_bytes: &TakeValue, total_bytes: i64) -> MyResult<()> {
    if let Some(start) = get_start_index(num_bytes, total_bytes) {
        file.seek(SeekFrom::Start(start))?; // 読み込み開始位置をシークで動かす: ファイル先頭からのインデックス位置
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        if !buffer.is_empty() {
            print!("{}", String::from_utf8_lossy(&buffer));
        }
    }
    Ok(())
}

// --------------------------------------------------
#[cfg(test)]
mod tests {
    use super::{
        get_start_index, count_lines_bytes, parse_num, TakeValue::*,
    };

    #[test]
    fn test_get_start_index() {
        // +0 from an empty file (0 lines/bytes) returns None
        assert_eq!(get_start_index(&PlusZero, 0), None);

        // +0 from a nonempty file returns an index that
        // is one less than the number of lines/bytes
        assert_eq!(get_start_index(&PlusZero, 1), Some(0));

        // Taking 0 lines/bytes returns None
        assert_eq!(get_start_index(&TakeNum(0), 1), None);

        // Taking any lines/bytes from an empty file returns None
        assert_eq!(get_start_index(&TakeNum(1), 0), None);

        // Taking more lines/bytes than is available returns None
        assert_eq!(get_start_index(&TakeNum(2), 1), None);

        // When starting line/byte is less than total lines/bytes,
        // return one less than starting number
        assert_eq!(get_start_index(&TakeNum(1), 10), Some(0));
        assert_eq!(get_start_index(&TakeNum(2), 10), Some(1));
        assert_eq!(get_start_index(&TakeNum(3), 10), Some(2));

        // When starting line/byte is negative and less than total,
        // return total - start
        assert_eq!(get_start_index(&TakeNum(-1), 10), Some(9));
        assert_eq!(get_start_index(&TakeNum(-2), 10), Some(8));
        assert_eq!(get_start_index(&TakeNum(-3), 10), Some(7));

        // When the starting line/byte is negative and more than the total,
        // return 0 to print the whole file
        assert_eq!(get_start_index(&TakeNum(-20), 10), Some(0));
    }

    #[test]
    fn test_count_lines_bytes() {
        let res = count_lines_bytes("tests/inputs/one.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (1, 24));

        let res = count_lines_bytes("tests/inputs/ten.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (10, 49));
    }

    #[test]
    fn test_parse_num() {
        // All integers should be interpreted as negative numbers
        let res = parse_num("3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        // A leading "+" should result in a positive number
        let res = parse_num("+3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(3));

        // An explicit "-" value should result in a negative number
        let res = parse_num("-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        // Zero is zero
        let res = parse_num("0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(0));

        // Plus zero is special
        let res = parse_num("+0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PlusZero);

        // Test boundaries
        let res = parse_num(&i64::MAX.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));

        let res = parse_num(&(i64::MIN + 1).to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));

        let res = parse_num(&format!("+{}", i64::MAX));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MAX));

        let res = parse_num(&i64::MIN.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN));

        // A floating-point value is invalid
        let res = parse_num("3.14");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "3.14");

        // Any non-integer string is invalid
        let res = parse_num("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "foo");
    }
}
