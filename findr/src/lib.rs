use clap::{App, Arg};
use regex::Regex;
use walkdir::{WalkDir, DirEntry};
use std::error::Error;

use crate::EntryType::*; // enumの各値を直接利用できるようにする

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq, Eq)]
enum EntryType {
    Dir,
    File,
    Link,
}

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    names: Vec<Regex>,
    entry_types: Vec<EntryType>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("findr")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust find")
        .arg(
            Arg::with_name("paths")
                .value_name("PATH")
                .help("Search paths")
                .default_value(".")
                .multiple(true)
        )
        .arg(
            Arg::with_name("names")
                .value_name("NAME")
                .short("n")
                .long("name")
                .help("Name")
                .takes_value(true)
                .multiple(true)
        )
        .arg(
            Arg::with_name("types")
                .value_name("TYPE")
                .short("t")
                .long("type")
                .help("Entry type")
                .possible_values(&["f", "d", "l"]) // 引数にセット可能な値を制限する
                .takes_value(true)
                .multiple(true)
        )
        .get_matches();

    let names = matches
        .values_of_lossy("names")
        .map(|vals| { // Option<_>の中身を取り出す
            vals.into_iter() // Vec<_>の中身として各要素をイテレーション
                .map(|name| { // 正規表現の文字列またはエラーに変換
                    Regex::new(&name)
                        .map_err(|_| format!("Invalid --name \"{}\"", name))
                })
                .collect::<Result<Vec<_>, _>>() // 各要素をVec<_>またはエラーとして集約
        }) // Option<Result>になる
        .transpose()? // Option<Result>からResult<Option>に変換後、エラー有無を確認
        .unwrap_or_default(); // Optionから中身のVec<_>を取り出す: Noneの場合にはデフォルト(空ベクトル: vec![])

    let entry_types = matches
        .values_of_lossy("types")
        .map(|vals| {
            vals.iter()
                .map(|val| match val.as_str() { // 変数を文字列として条件分岐
                    "d" => Dir,
                    "f" => File,
                    "l" => Link,
                    _ => unreachable!("Invalid type"), // 異常処理としてpanic!を出力
                })
                .collect::<Vec<EntryType>>() // enumとして集約
        })
        .unwrap_or_default(); // OptionからVec<_>のみを取り出す

    Ok(
        Config {
            paths: matches.values_of_lossy("paths").unwrap(),
            names,
            entry_types,
        })
}

pub fn run(config: Config) -> MyResult<()> {
    // フィルター関数として処理を定義: trueまたはfalseを返す
    let type_filter = |entry: &DirEntry| {
        config.entry_types.is_empty()
            || config
                .entry_types
                .iter()
                .any(|entry_type| match entry_type {
                    // enum型の条件分岐: 全種類が網羅されていない場合、コンパイル時にエラーとなる
                    Link => entry.path_is_symlink(),
                    Dir => entry.file_type().is_dir(),
                    File => entry.file_type().is_file(),
                })
    };

    // フィルター関数として処理を定義: trueまたはfalseを返す
    let name_filter = |entry: &DirEntry| {
        config.names.is_empty()
            || config
                .names
                .iter()
                .any(|re| re.is_match(&entry.file_name().to_string_lossy()))
    };

    for path in config.paths {
        // for entry in WalkDir::new(path) { // パスに含まれるディレクトリ, ファイル, リンクのパスを取得
        //     match entry {
        //         Err(e) => eprintln!("{}", e),
        //         Ok(entry) => {
        //             if (config.entry_types.is_empty() // 検索対象の種類が未指定の場合
        //                 || config.entry_types.iter().any(|entry_type| { // または指定の検索対象の種類のうちいずれかに該当する場合
        //                     match entry_type { // ファイル種別で条件分岐
        //                         Link => entry.file_type().is_symlink(),
        //                         Dir => entry.file_type().is_dir(),
        //                         File => entry.file_type().is_file(),
        //                     }
        //                 })) && ( // 尚且つ
        //                     config.names.is_empty() // 検索対象の名称が未指定の場合
        //                         || config.names.iter().any(|re| { // または指定の検索対象の名称(正規表現)のうちいずれかに該当する場合
        //                             re.is_match(
        //                                 &entry.file_name().to_string_lossy(),
        //                             )
        //                     })
        //                 )
        //             {
        //                 println!("{}", entry.path().display())
        //             }
        //         }
        //     }
        // }
        let entries = WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| match entry { // イテレータの(Result型の)各要素を処理: (Option型の)返り値がNoneとなった要素をフィルタリングで除去
                Err(e) => {
                    eprintln!("{}", e);
                    None // フィルタリングによってイレテータから除去される
                }
                Ok(entry) => Some(entry), // フィルタリングされず後続処理に渡される
            })
            // クロージャを組み合わせて絞り込みを実施
            .filter(type_filter) // falseとなった要素は除去
            .filter(name_filter)
            .map(|entry| entry.path().display().to_string()) // 残った要素を文字列に変換
            .collect::<Vec<_>>(); // ベクトルとして集約
        println!("{}", entries.join("\n")); // 改行区切りで出力
    }
    Ok(())
}
