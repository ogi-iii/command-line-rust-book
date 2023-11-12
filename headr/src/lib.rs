use std::{error::Error, io::{Read, BufRead, stdin, BufReader}, fs::File};

use clap::{App, Arg};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: usize,
    bytes: Option<usize>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("headr")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust head")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .multiple(true)
                .default_value("-"),
        )
        .arg(
            Arg::with_name("lines")
                .short("n")
                .long("lines")
                .value_name("LINES")
                .help("Number of lines")
                .takes_value(true)
                .default_value("10"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .value_name("BYTES")
                .help("Number of bytes")
                .takes_value(true)
                .conflicts_with("lines")
        )
        .get_matches();

    let lines = matches.value_of("lines")
        .map(parse_positive_int) // Some(&str)の値を引数として関数を実行: Option<MyResult>を返す
        .transpose() // Option<Result> を Result<Option> に変換: NoneはOk(None), Some(Ok)はOk(Some), Some(Err)はErrを返す
        .map_err(|e| format!("illegal line count -- {}", e))?;

    let bytes = matches.value_of("bytes")
        .map(parse_positive_int)
        .transpose()
        .map_err(|e| format!("illegal byte count -- {}", e))?;

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(), // Optionをunwrap()
        lines: lines.unwrap(), // Optionをunwrap()
        bytes, // Optionのまま渡す
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let num_files = config.files.len();

    for (file_num, filename) in config.files.iter().enumerate() {
        match open(&filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(mut file) => {
                if num_files > 1 { // 対象ファイル数が複数の場合
                    println!(
                        "{}==> {} <==",
                        if file_num > 0 { "\n" } else { "" }, // 2ファイル目以降は改行を追加
                        filename
                    );
                }
                // for line in file.lines().take(config.lines) { // take(n)でイテレータの回数を制限
                //     println!("{}", line?); // lines()は各行の文字列を取得し、改行コード無しで返す
                // }
                if let Some(num_bytes) = config.bytes {
                    // let mut handle = file.take(num_bytes as u64); // 指定のバイト数で対象範囲指定: usizeはu64に変換して使用する
                    // let mut buffer = vec![0; num_bytes]; // 読み込み先となる固定サイズの空バイト配列を作成
                    // let bytes_read = handle.read(&mut buffer)?; // 指定のバイト数の分だけ読み込む: 実際の読み込みサイズを返り値で取得
                    // print!("{}", String::from_utf8_lossy(&buffer[..bytes_read])); // 実際に読み込まれたサイズ分だけバイト配列を文字列に変換して出力

                    let bytes = file.bytes().take(num_bytes).collect::<Result<Vec<_>, _>>(); // turbofishで型情報を明示
                    print!("{}", String::from_utf8_lossy(&bytes?));
                } else {
                    let mut line = String::new();
                    for _ in 0..config.lines { // 行数の指定
                        let bytes = file.read_line(&mut line)?; // ファイルから各行のバイト配列を読み込み、文字列の変数に代入(返り値は読み込みバイト数): バイト配列なので改行コードもそのまま代入される
                        if bytes == 0 {
                            break; // EOFの時は0バイトが読み込まれる
                        }
                        print!("{}", line); // 改行コードも含まれるのでln不要
                        line.clear(); // 文字列をリセット
                    }
                }
            },
        };
    }
    Ok(())
}

fn parse_positive_int(val: &str) -> MyResult<usize> {
    match val.parse() {
        Ok(n) if n > 0 => Ok(n), // if条件付き分岐
        _ => Err(val.into()),
    }
}

#[test]
fn test_parse_positive_int() {
    let res = parse_positive_int("3");
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 3);

    let res = parse_positive_int("foo");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "foo".to_string());

    let res = parse_positive_int("0");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "0".to_string());
}
