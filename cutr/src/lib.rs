use std::{error::Error, ops::Range, num::NonZeroUsize, io::{BufRead, BufReader, stdin, stdout}, fs::File};

use clap::{App, Arg};
use csv::{StringRecord, ReaderBuilder, WriterBuilder};
use regex::Regex;

use crate::Extract::*;

type MyResult<T> = Result<T, Box<dyn Error>>;
type PositionList = Vec<Range<usize>>; // 自然数で構成される範囲値のベクトル

#[derive(Debug)]
pub enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    delimiter: u8, // 区切り文字を単一バイトの値(0~255)として保持
    extract: Extract,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("cutr")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust cut")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .multiple(true)
                .default_value("-"),
        )
        .arg(
            Arg::with_name("delimiter")
                .value_name("DELIMITER")
                .help("Field delimiter")
                .short("d")
                .long("delim")
                .default_value("\t"), // タブ区切り
        )
        .arg(
            Arg::with_name("fields") // フィールドの位置番号で範囲指定
                .value_name("FIELDS")
                .help("Selected fields")
                .short("f")
                .long("fields")
                .conflicts_with_all(&["chars", "bytes"]),
        )
        .arg(
            Arg::with_name("bytes") // バイト数で範囲指定
                .value_name("BYTES")
                .help("Selected bytes")
                .short("b")
                .long("bytes")
                .conflicts_with_all(&["chars", "fields"]),
        )
        .arg(
            Arg::with_name("chars") // 文字数で範囲指定
                .value_name("CHARS")
                .help("Selected chars")
                .short("c")
                .long("chars")
                .conflicts_with_all(&["fields", "bytes"]),
        )
        .get_matches();

    let delimiter = matches.value_of("delimiter").unwrap();
    // バイト配列に変換
    let delim_bytes = delimiter.as_bytes();
    // 単一バイト値かどうかを判定
    if delim_bytes.len() != 1 {
        return Err(From::from(
            format!("--delim \"{}\" must be a single byte", delimiter)
        ));
    }

    let fields = matches.value_of("fields")
        // 文字列から範囲値ベクトルに変換
        .map(parse_pos)
        // Option<Result>をResult<Option>に変換してエラー有無を確認: Optionを変数に格納
        .transpose()?;
    let bytes = matches.value_of("bytes")
        .map(parse_pos)
        .transpose()?;
    let chars = matches.value_of("chars")
        .map(parse_pos)
        .transpose()?;

    // 範囲指定方法で分岐
    let extract = if let Some(field_pos) = fields {
        Fields(field_pos)
    } else if let Some(byte_pos) = bytes {
        Bytes(byte_pos)
    } else if let Some(char_pos) = chars {
        Chars(char_pos)
    } else {
        // 範囲指定方法がフラグで渡されなかった場合: エラーを返す
        return Err(From::from(
            "Must have --fields, --bytes, or --chars"
        ));
    };

    Ok(
        // set the values from matches here...
        Config {
            files: matches.values_of_lossy("files").unwrap(),
            delimiter: *delim_bytes.first().unwrap(), // バイト配列の最初の参照値をデリファレンス: 所有権を取得するため
            extract,
        }
    )
}

fn parse_index(input: &str) -> Result<usize, String> { // 0から始まるindex値またはエラーメッセージを返す
    let value_error = || format!("illegal list value: \"{}\"", input);
    input.starts_with("+")
        .then(|| Err(value_error())) // Optionを返す: "+"で始まる場合はSomeにエラーメッセージを入れる
        .unwrap_or_else(|| { // Noneの場合: エラーではない時
            input.parse::<NonZeroUsize>() // str -> 非ゼロの値
            .map(|n| usize::from(n) - 1) // 非ゼロの値 -> usizeに変換後、0から始まるindex値に修正
            .map_err(|_| value_error()) // parse時にエラーとなった場合
        })
}

fn parse_pos(range: &str) -> MyResult<PositionList> { // カンマ区切りまたはダッシュ(-)範囲の数値を範囲値ベクトルとして返す
    // 正規表現を r"" で生の文字列として表現: \ エスケープ文字をRustに解釈させずにそのまま利用
    let range_re = Regex::new(r"^(\d+)-(\d+)$").unwrap(); // () 括弧で囲まれた範囲をキャプチャする
    range.split(',') // 区切り文字で分割
        .into_iter()
        .map(|val| {
            // 単一の数値の場合: 0始まりのindex範囲に変換: 先頭の数値は範囲に含まれるが、後ろの数値は範囲に含まれない
            parse_index(val).map(|n| n..n+1)
                .or_else(|e| {
                    // 正規表現と比較: 一致した場合は2つの数値を取得
                    range_re.captures(val)
                        // 正規表現に当てはまらない場合にはエラーを返す
                        .ok_or(e)
                        // エラーにならなかった場合
                        .and_then(|captures| {
                            // 正規表現から取得した値を0始まりのindex値に変換
                            let n1 = parse_index(&captures[1])?; // index番号は1から始まる
                            let n2 = parse_index(&captures[2])?;
                            // 大小関係を確認
                            if n1 >= n2 {
                                return Err(
                                    format!(
                                        "First number in range ({}) must be lower than second number ({})",
                                        n1+1,
                                        n2+1));
                            }
                            // index範囲を返す: 後ろの値は範囲外にすること
                            Ok(n1..n2+1)
                        })
            })
        })
        // イテレータの処理結果をベクトルに集約
        .collect::<Result<_, _>>()
        // エラーメッセージはError型に変換して返す
        .map_err(From::from)
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

pub fn run(config: Config) -> MyResult<()> {
    for filename in &config.files {
        match open(filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(reader) => match &config.extract {
                Fields(field_pos) => {
                    // readerからカラム区切りレコードとして読み込む
                    let mut reader = ReaderBuilder::new()
                        .delimiter(config.delimiter)
                        .has_headers(false)
                        .from_reader(reader);
                    // 標準出力に書き込む
                    let mut wtr = WriterBuilder::new()
                        .delimiter(config.delimiter)
                        .from_writer(stdout());
                    for record in reader.records() {
                        let record = record?;
                        wtr.write_record(extract_fields(&record, field_pos))?;
                    }
                }
                Bytes(byte_pos) => {
                    for line in reader.lines() {
                        println!("{}", extract_bytes(&line?, byte_pos))
                    }
                }
                Chars(char_pos) => {
                    for line in reader.lines() {
                        println!("{}", extract_chars(&line?, char_pos))
                    }
                }
            }
        }
    }
    Ok(())
}

fn extract_chars(line: &str, char_pos: &[Range<usize>]) -> String { // &PositionListはwarningとなる: 不変サイズのリストを受け取れなくなるため
    let chars: Vec<_> = line.chars().collect(); // 文字列をcharに分割後、ベクトルとして集約
    // let mut selected: Vec<char> = vec![];

    // for range in char_pos.iter().cloned() { // 範囲値リストをクローンしてイテレーション
    //     // for i in range { // 範囲でイテレーション
    //     //     if let Some(val) = chars.get(i) { // 指定位置にcharが存在すれば追加
    //     //         selected.push(*val)
    //     //     }
    //     // }
    //     selected.extend(range.filter_map(|i| chars.get(i))); // 値がSomeとして存在するもののみをフィルタリングして追加
    // }
    // selected.iter().collect() // charベクトルから文字列に変換
    char_pos.iter()
        .cloned()
        // .map(|range| range.filter_map(|i| chars.get(i)))
        // .flatten() // 多層イテレータを平坦化: 単一イテレータに変換する
        .flat_map(|range| range.filter_map(|i| chars.get(i)))
        .collect()
}

fn extract_bytes(line: &str, byte_pos: &[Range<usize>]) -> String {
    let bytes = line.as_bytes();
    // 取得対象のバイト配列を変数に集約
    let selected: Vec<_> = byte_pos.iter()
        .cloned()
        // 各バイトの参照値を複製して実体値として取得: String変換時の引数型に合わせるため
        .flat_map(|range| range.filter_map(|i| bytes.get(i)).copied())
        .collect();
    // バイト配列から文字列に変換し、クローンして所有権を渡す
    String::from_utf8_lossy(&selected).into_owned()
}

// ライフタイム修飾子を付与: recordと同じライフタイムとして返り値の&strを定義
fn extract_fields<'a>(record: &'a StringRecord, field_pos: &[Range<usize>]) -> Vec<&'a str> { // カラム区切りのレコード値を受け取り、出力カラム値のベクトルを返す
    field_pos.iter()
        .cloned()
        .flat_map(|range| range.filter_map(|i| record.get(i)))
        // .map(String::from)
        .collect()
}

// "cargo test unit" で実行されるUTを定義: モジュール名 "unit_tests" の接頭辞を認識して実行対象が絞り込まれるため
#[cfg(test)]
mod unit_tests {
    use super::parse_pos;
    use super::extract_bytes;
    use super::extract_chars;
    use super::extract_fields;
    use csv::StringRecord;

    #[test]
    fn test_parse_pos() {
        // The empty string is an error
        assert!(parse_pos("").is_err());

        // Zero is an error
        let res = parse_pos("0");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"",);

        let res = parse_pos("0-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"",);

        // A leading "+" is an error
        let res = parse_pos("+1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"+1\"",
        );

        let res = parse_pos("+1-2");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"+1-2\"",
        );

        let res = parse_pos("1-+2");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"1-+2\"",
        );

        // Any non-number is an error
        let res = parse_pos("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1-a");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"1-a\"",
        );

        let res = parse_pos("a-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"a-1\"",
        );

        // Wonky ranges
        let res = parse_pos("-");
        assert!(res.is_err());

        let res = parse_pos(",");
        assert!(res.is_err());

        let res = parse_pos("1,");
        assert!(res.is_err());

        let res = parse_pos("1-");
        assert!(res.is_err());

        let res = parse_pos("1-1-1");
        assert!(res.is_err());

        let res = parse_pos("1-1-a");
        assert!(res.is_err());

        // First number must be less than second
        let res = parse_pos("1-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_pos("2-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (2) must be lower than second number (1)"
        );

        // All the following are acceptable
        let res = parse_pos("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("01");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("1,3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("001,0003");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("1-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("0001-03");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("1,7,3-5");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_pos("15,19-20");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }

    #[test]
    fn test_extract_chars() {
        assert_eq!(extract_chars("", &[0..1]), "".to_string());
        assert_eq!(extract_chars("ábc", &[0..1]), "á".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 2..3]), "ác".to_string());
        assert_eq!(extract_chars("ábc", &[0..3]), "ábc".to_string());
        assert_eq!(extract_chars("ábc", &[2..3, 1..2]), "cb".to_string());
        assert_eq!(
            extract_chars("ábc", &[0..1, 1..2, 4..5]),
            "áb".to_string()
        );
    }

    #[test]
    fn test_extract_bytes() {
        assert_eq!(extract_bytes("ábc", &[0..1]), "�".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2]), "á".to_string());
        assert_eq!(extract_bytes("ábc", &[0..3]), "áb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..4]), "ábc".to_string());
        assert_eq!(extract_bytes("ábc", &[3..4, 2..3]), "cb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2, 5..6]), "á".to_string());
    }

    #[test]
    fn test_extract_fields() {
        let rec = StringRecord::from(vec!["Captain", "Sham", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2]), &["Sham"]);
        assert_eq!(
            extract_fields(&rec, &[0..1, 2..3]),
            &["Captain", "12345"]
        );
        assert_eq!(extract_fields(&rec, &[0..1, 3..4]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2, 0..1]), &["Sham", "Captain"]);
    }
}
