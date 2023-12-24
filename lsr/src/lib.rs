use std::{error::Error, path::PathBuf, fs::{metadata, read_dir}, os::unix::fs::MetadataExt};

use chrono::{DateTime, Local};
use clap::{App, Arg};
use tabular::{Table, Row};
use users::{get_user_by_uid, get_group_by_gid};

// 外部ファイル(owner.rs)をモジュールとして読み込む
mod owner;
use owner::Owner;
use owner::Owner::*;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    long: bool,
    show_hidden: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("lsr")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust ls")
        .arg(
            Arg::with_name("paths")
                .value_name("PATH")
                .help("Files and/or directories")
                .default_value(".")
                .multiple(true),
        )
        .arg(
            Arg::with_name("long")
                .short("l")
                .long("long")
                .help("Long listening")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("all")
                .short("a")
                .long("all")
                .help("Show all files")
                .takes_value(false),
        )
        .get_matches();

    Ok(
        Config {
            paths: matches.values_of_lossy("paths").unwrap(),
            long: matches.is_present("long"),
            show_hidden: matches.is_present("all"),
        }
    )
}

pub fn run(config: Config) -> MyResult<()> {
    let paths = find_files(&config.paths, config.show_hidden)?;

    if config.long {
        println!("{}", format_output(&paths)?);
    } else {
        for path in paths {
            println!("{}", path.display()) // displayにより(非unicodeデータがパス名に含まれていても)安全にパスを出力できる
        }
    }

    Ok(())
}

// ディレクトリまたはファイルパスを探索: 引数がディレクトリの場合は子ファイルまたは子ディレクトリを羅列(ただし孫以上の再帰処理はしない!)
fn find_files(
    paths: &[String],
    show_hidden: bool,
) -> MyResult<Vec<PathBuf>> {
    let mut results = vec![];
    for name in paths {
        match metadata(name) {
            Err(e) => eprintln!("{}: {}", name, e),
            Ok(meta) => {
                if meta.is_dir() {
                    // ディレクトリ内を展開
                    for entry in read_dir(name)? {
                        let entry = entry?;
                        let path = entry.path();
                        // '.'ドットで始まる隠しファイルか否かを判定
                        let is_hidden = path.file_name().map_or(false, |file_name| {
                            file_name.to_string_lossy().starts_with('.')
                        });
                        if !is_hidden || show_hidden {
                            results.push(entry.path());
                        }
                    }
                } else {
                    results.push(PathBuf::from(name));
                }
            }
        }
    }
    Ok(results)
}

fn format_output(paths: &[PathBuf]) -> MyResult<String> {
    // ls -l のフォーマットを作成
    let fmt = "{:<}{:<}  {:>}  {:<}  {:<}  {:>}  {:<}  {:<}";

    // フォーマットに値を当てはめるためのテーブルを生成
    let mut table = Table::new(fmt);

    for path in paths {
        // ファイルまたはディレクトリのメタ情報を取得
        let metadata = path.metadata()?;

        let uid = metadata.uid();
        let user = get_user_by_uid(uid)
            .map(|u| u.name().to_string_lossy().into_owned())
            .unwrap_or_else(|| uid.to_string()); // ユーザ名またはuidを返す

        let gid = metadata.gid();
        let group = get_group_by_gid(gid)
            .map(|g| g.name().to_string_lossy().into_owned())
            .unwrap_or_else(|| gid.to_string()); // グループ名またはgidを返す

        let file_type = if path.is_dir() {
            "d"
        } else {
            "-"
        };

        // ユーザ/グループ/その他のパーミッション文字列を取得
        let perms = format_mode(metadata.mode());

        // 更新日時を取得
        let modified: DateTime<Local> = DateTime::from(metadata.modified()?);

        // レコード形式で(左端の列から)順に値を代入
        table.add_row(
            Row::new()
                // ファイルのメタデータから各値を取得
                .with_cell(file_type) // file type: d or -
                .with_cell(perms) // permission
                .with_cell(metadata.nlink()) // number of links
                .with_cell(user) // user name
                .with_cell(group) // group name
                .with_cell(metadata.len()) // size
                .with_cell(modified.format("%b %d %y %H:%M")) // modification timestamp
                .with_cell(path.display()) // path
        );
    }

    Ok(format!("{}", table))
}

// 3本スラッシュでdocコメントを定義可能: "cargo doc --open --document-private-items" でドキュメントを生成してブラウザで開く

/// Given a file mode in octal format like 0o751,
/// return a string like "rwxr-x--x"
// 8進数表記3桁のパーミッションから、ユーザ/グループ/その他の各rwxパーミッション文字列を生成
fn format_mode(mode: u32) -> String {
    format!(
        "{}{}{}",
        mk_triple(mode, User),
        mk_triple(mode, Group),
        mk_triple(mode, Other),
    )
}

/// Given an octal number like 0o500 and an [`Owner`],
/// return a string like "r-x"
// パーミッションの数値からrwxを返す
pub fn mk_triple(mode: u32, owner: Owner) -> String {
    let [read, write, execute] = owner.masks();
    format!(
        "{}{}{}",
        // 各パーミッションの8進数と一致するかを確認: ゼロは不一致、1は一致
        if mode & read == 0 {
            "-"
        } else {
            "r"
        },
        if mode & write == 0 {
            "-"
        } else {
            "w"
        },
        if mode & execute == 0 {
            "-"
        } else {
            "x"
        },
    )
}

// --------------------------------------------------
#[cfg(test)]
mod test {
    use super::find_files;
    use super::format_mode;
    use super::format_output;
    use super::mk_triple;
    use super::Owner;
    use std::path::PathBuf;

    #[test]
    fn test_find_files() {
        // Find all non-hidden entries in a directory
        let res = find_files(&["tests/inputs".to_string()], false);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );

        // Any existing file should be found even if hidden
        let res = find_files(&["tests/inputs/.hidden".to_string()], false);
        assert!(res.is_ok());
        let filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        assert_eq!(filenames, ["tests/inputs/.hidden"]);

        // Test multiple path arguments
        let res = find_files(
            &[
                "tests/inputs/bustle.txt".to_string(),
                "tests/inputs/dir".to_string(),
            ],
            false,
        );
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            ["tests/inputs/bustle.txt", "tests/inputs/dir/spiders.txt"]
        );
    }

    #[test]
    fn test_find_files_hidden() {
        // Find all entries in a directory including hidden
        let res = find_files(&["tests/inputs".to_string()], true);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/.hidden",
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );
    }

    fn long_match(
        line: &str,
        expected_name: &str,
        expected_perms: &str,
        expected_size: Option<&str>,
    ) {
        let parts: Vec<_> = line.split_whitespace().collect();
        assert!(parts.len() > 0 && parts.len() <= 10);

        let perms = parts.get(0).unwrap();
        assert_eq!(perms, &expected_perms);

        if let Some(size) = expected_size {
            let file_size = parts.get(4).unwrap();
            assert_eq!(file_size, &size);
        }

        let display_name = parts.last().unwrap();
        assert_eq!(display_name, &expected_name);
    }

    #[test]
    fn test_format_output_one() {
        let bustle_path = "tests/inputs/bustle.txt";
        let bustle = PathBuf::from(bustle_path);

        let res = format_output(&[bustle]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let lines: Vec<&str> =
            out.split("\n").filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 1);

        let line1 = lines.first().unwrap();
        long_match(&line1, bustle_path, "-rw-r--r--", Some("193"));
    }

    #[test]
    fn test_format_output_two() {
        let res = format_output(&[
            PathBuf::from("tests/inputs/dir"),
            PathBuf::from("tests/inputs/empty.txt"),
        ]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let mut lines: Vec<&str> =
            out.split("\n").filter(|s| !s.is_empty()).collect();
        lines.sort();
        assert_eq!(lines.len(), 2);

        let empty_line = lines.remove(0);
        long_match(
            &empty_line,
            "tests/inputs/empty.txt",
            "-rw-r--r--",
            Some("0"),
        );

        let dir_line = lines.remove(0);
        long_match(&dir_line, "tests/inputs/dir", "drwxr-xr-x", None);
    }

    #[test]
    fn test_mk_triple() {
        assert_eq!(mk_triple(0o751, Owner::User), "rwx");
        assert_eq!(mk_triple(0o751, Owner::Group), "r-x");
        assert_eq!(mk_triple(0o751, Owner::Other), "--x");
        assert_eq!(mk_triple(0o600, Owner::Other), "---");
    }

    #[test]
    fn test_format_mode() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
        assert_eq!(format_mode(0o421), "r---w---x");
    }
}
