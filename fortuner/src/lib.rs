use std::{error::Error, path::PathBuf, ffi::OsStr, fs::{metadata, File}, io::{BufReader, BufRead}};

use clap::{App, Arg};
use rand::{rngs::StdRng, SeedableRng, seq::SliceRandom};
use regex::{Regex, RegexBuilder};
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn Error>>; // エラートレイトを実装するオブジェクトは必ずBoxに格納: サイズ不明のため格納先のみを指定する

#[derive(Debug)]
struct Fortune {
    source: String,
    text: String,
}

#[derive(Debug)]
pub struct Config {
    sources: Vec<String>,
    pattern: Option<Regex>,
    seed: Option<u64>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("fortuner")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust fortune")
        .arg(
            Arg::with_name("sources")
                .value_name("FILE")
                .multiple(true)
                .required(true)
                .help("Input files or directories"),
        )
        .arg(
            Arg::with_name("pattern")
                .value_name("PATTERN")
                .short("m")
                .long("pattern")
                .help("Pattern"),
        )
        .arg(
            Arg::with_name("insensitive")
            .short("i")
            .long("insensitive")
            .takes_value(false)
            .help("Case-insensitive pattern matching"),
        )
        .arg(
            Arg::with_name("seed")
                .value_name("SEED")
                .short("s")
                .long("seed")
                .help("Random seed"),
        )
        .get_matches();

    let pattern = matches.value_of("pattern")
        // Optionの中身をmap処理
        .map(|val| RegexBuilder::new(val)
            .case_insensitive(matches.is_present("insensitive"))
            .build()
            .map_err(|_| format!("Invalid --pattern \"{}\"", val)) // エラーメッセージの書き換え
        ).transpose()?;

    let seed = matches.value_of("seed")
        .map(parse_u64)
        .transpose()?;

    Ok(
        Config {
            sources: matches.values_of_lossy("sources").unwrap(),
            pattern,
            seed,
        }
    )
}

fn parse_u64(val: &str) -> MyResult<u64> {
    // &str -> Result<u64> に変換
    val.parse()
        .map_err(|_| format!("\"{}\" not a valid integer", val).into()) // Error<String> -> Box<Error<String>>> に変換
}

pub fn run(config: Config) -> MyResult<()> {
    let files = find_files(&config.sources)?;
    let fortunes = read_fortunes(&files)?;

    // 正規表現が指定されている場合は一致する全てのFortuneを出力
    if let Some(pattern) = config.pattern {
        // 直前のソース名(ファイルパス)の保存先を定義
        let mut prev_source = None;
        // Fortuneのうち、テキスト内容が正規表現と合致するもののみをフィルタリングしてループ処理
        for fortune in fortunes.iter().filter(|fortune| pattern.is_match(&fortune.text)) {
            // (Optionの中身を参照して)直前のソース名と不一致の場合はファイル名を出力: 初回は(Noneなので)デフォルトで(ファイル名を)出力
            if prev_source.as_ref().map_or(true, |s| s != &fortune.source) {
                eprintln!("({})\n%", fortune.source);
                prev_source = Some(fortune.source.clone()); // 所有権の関係から複製して保存
            }
            println!("{}\n%", fortune.text);
        }
    } else {
        // 正規表現未指定時はシード値を元にランダムに1つFortuneを抽出して出力
        let text = pick_fortune(&fortunes, config.seed)
            .or_else(|| Some("No fortunes found".to_string())).unwrap(); // エラーの場合は文字列を返す
        println!("{}", text);
    }
    Ok(())
}

// PathBufを利用することで所有権が直接得られる: Pathは不定サイズのためBox<Path>等のように利用しなければならない
fn find_files(paths: &[String]) -> MyResult<Vec<PathBuf>> {
    let dat = OsStr::new("dat");
    let mut files = vec![];

    for path in paths {
        match metadata(path) {
            Err(e) => return Err(format!("{}: {}", path, e).into()),
            Ok(_) => files.extend(
                WalkDir::new(path)
                    .into_iter() // パスを再起的に探索
                    .filter_map(Result::ok) // 読み込めないファイルやディレクトリを除去
                    .filter(|entry| {
                        entry.file_type().is_file()
                            && entry.path().extension() != Some(dat) // .datではないファイルのみをフィルタリング
                    })
                    .map(|entry| entry.path().into()), // ファイルパスを集約
            )
        }
    }
    files.sort();
    files.dedup(); // ファイルパスの重複を除去: 連続する同じファイルパスは1つにする
    Ok(files)
}

// ファイル名と記載内容の構造体をベクトルで返す
fn read_fortunes(paths: &[PathBuf]) -> MyResult<Vec<Fortune>> {
    let mut fortunes = vec![];
    let mut buffer = vec![];

    for path in paths {
        // パスを文字列として所有
        let basename = path.file_name().unwrap().to_string_lossy().into_owned();
        // パスをファイルとして開く
        let file = File::open(path).map_err(|e| {
            format!("{}: {}", path.to_string_lossy().into_owned(), e)
        })?;

        // ファイルをバッファで1行ずつ(読み込み可能な行のみを)読み込む
        for line in BufReader::new(file).lines().filter_map(Result::ok) {
            // 区切り文字が見つかった場合: 記載内容が空でなければパス情報と共にstructに詰め込んでベクトルに追加
            if line == "%" {
                if !buffer.is_empty() {
                    fortunes.push(Fortune {
                        source: basename.clone(), // 所有権ごと複製
                        text: buffer.join("\n"), // 改行を含む内容を格納
                    });
                    buffer.clear();
                }
            } else {
                // 区切り文字未到達段階: ベクトルに各行の文字列を格納
                buffer.push(line.to_string());
            }
        }
    }
    Ok(fortunes)
}

// ベクトルの中からシード値を元にランダムに1つ抽出した構造体の記載内容を返す
fn pick_fortune(fortunes: &[Fortune], seed: Option<u64>) -> Option<String> {
    if let Some(val) = seed {
        // seed値から乱数(ランダムな数値生成)器を作成
        let mut rng = StdRng::seed_from_u64(val);
        // ベクトルから乱数器で要素を抽出し、Stringに変換: 可変引数として渡す
        fortunes.choose(&mut rng).map(|f| f.text.to_string())
    } else {
        // seedが無い場合はスレッド依存の乱数生成器を利用: 可変引数として渡す
        let mut rng = rand::thread_rng();
        fortunes.choose(&mut rng).map(|f| f.text.to_string())
    }
}

// --------------------------------------------------
#[cfg(test)]
mod tests {
    use super::find_files;
    use super::parse_u64;
    use super::pick_fortune;
    use super::read_fortunes;
    use super::Fortune;
    use std::path::PathBuf;

    #[test]
    fn test_parse_u64() {
        let res = parse_u64("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "\"a\" not a valid integer");

        let res = parse_u64("0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 0);

        let res = parse_u64("4");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 4);
    }

    #[test]
    fn test_find_files() {
        // Verify that the function finds a file known to exist
        let res = find_files(&["./tests/inputs/jokes".to_string()]);
        assert!(res.is_ok());

        let files = res.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files.get(0).unwrap().to_string_lossy(),
            "./tests/inputs/jokes"
        );

        // Fails to find a bad file
        let res = find_files(&["/path/does/not/exist".to_string()]);
        assert!(res.is_err());

        // Finds all the input files, excludes ".dat"
        let res = find_files(&["./tests/inputs".to_string()]);
        assert!(res.is_ok());

        // Check number and order of files
        let files = res.unwrap();
        assert_eq!(files.len(), 5);
        let first = files.get(0).unwrap().display().to_string();
        assert!(first.contains("ascii-art"));
        let last = files.last().unwrap().display().to_string();
        assert!(last.contains("quotes"));

        // Test for multiple sources, path must be unique and sorted
        let res = find_files(&[
            "./tests/inputs/jokes".to_string(),
            "./tests/inputs/ascii-art".to_string(),
            "./tests/inputs/jokes".to_string(),
        ]);
        assert!(res.is_ok());
        let files = res.unwrap();
        assert_eq!(files.len(), 2);
        if let Some(filename) = files.first().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "ascii-art".to_string())
        }
        if let Some(filename) = files.last().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "jokes".to_string())
        }
    }

    #[test]
    fn test_read_fortunes() {
        // Parses all the fortunes without a filter
        let res = read_fortunes(&[PathBuf::from("./tests/inputs/jokes")]);
        assert!(res.is_ok());

        if let Ok(fortunes) = res {
            // Correct number and sorting
            assert_eq!(fortunes.len(), 6);
            assert_eq!(
                fortunes.first().unwrap().text,
                "Q. What do you call a head of lettuce in a shirt and tie?\n\
                A. Collared greens."
            );
            assert_eq!(
                fortunes.last().unwrap().text,
                "Q: What do you call a deer wearing an eye patch?\n\
                A: A bad idea (bad-eye deer)."
            );
        }

        // Filters for matching text
        let res = read_fortunes(&[
            PathBuf::from("./tests/inputs/jokes"),
            PathBuf::from("./tests/inputs/quotes"),
        ]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 11);
    }

    #[test]
    fn test_pick_fortune() {
        // Create a slice of fortunes
        let fortunes = &[
            Fortune {
                source: "fortunes".to_string(),
                text: "You cannot achieve the impossible without attempting the absurd."
                    .to_string(),
            },
            Fortune {
                source: "fortunes".to_string(),
                text: "Assumption is the mother of all screw-ups."
                    .to_string(),
            },
            Fortune {
                source: "fortunes".to_string(),
                text: "Neckties strangle clear thinking.".to_string(),
            },
        ];

        // Pick a fortune with a seed
        assert_eq!(
            pick_fortune(fortunes, Some(1)).unwrap(),
            "Neckties strangle clear thinking.".to_string()
        );
    }
}
