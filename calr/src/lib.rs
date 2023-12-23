use std::{error::Error, str::FromStr};

use ansi_term::Style;
use chrono::{NaiveDate, Local, Datelike};
use clap::{App, Arg};
use itertools::izip;

type MyResult<T> = Result<T, Box<dyn Error>>;

const LINE_WIDTH: usize = 22;

// キャパシティを定義したstr配列を作成
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

#[derive(Debug)]
pub struct Config {
    month: Option<u32>, // chronoクレートの型に合わせてu32を利用(yearも同様)
    year: i32,
    today: NaiveDate,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("calr")
        .version("0.1.0")
        .author("kazuki.ogiwara")
        .about("Rust cal")
        .arg(
            Arg::with_name("year")
                .value_name("YEAR")
                .help("Year (1-9999)"),
        )
        .arg(
            Arg::with_name("month")
                .value_name("MONTH")
                .short("m")
                .help("Month name or number (1-12)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("show_current_year")
                .value_name("SHOW_YEAR")
                .short("y")
                .long("year")
                .help("Show whole current year")
                .conflicts_with_all(&["month", "year"])
                .takes_value(false),
        )
        .get_matches();

    let mut year = matches.value_of("year")
        .map(parse_year)
        .transpose()?;
    let mut month = matches.value_of("month")
        .map(parse_month)
        .transpose()?;

    // ローカルな今日の日付情報を取得
    let today = Local::today();

    if matches.is_present("show_current_year") {
        year  = Some(today.year());
        month = None;
    } else if month.is_none() && year.is_none() {
        // デフォルト値をセット
        year = Some(today.year());
        month = Some(today.month());
    }

    Ok(
        Config {
            month,
            year: year.unwrap_or_else(|| today.year()), // Noneの場合は今年
            today: today.naive_local(), // 今日のローカル日付
        }
    )
}

fn parse_int<T: FromStr>(val: &str) -> MyResult<T> {
    val.parse()
        .map_err(|_| format!("Invalid integer \"{}\"", val).into())
}

fn parse_year(year: &str) -> MyResult<i32> {
    // パースした結果をmap処理させる
    parse_int(year).and_then(|num| {
        // 1から9999の範囲に含まれるかを確認
        if (1..=9999).contains(&num) {
            Ok(num)
        } else {
            Err(format!("year \"{}\" not in the range 1 through 9999", year).into())
        }
    })
}

fn parse_month(month: &str) -> MyResult<u32> {
    match parse_int(&month) {
        // 数値の場合
        Ok(num) => {
            if (1..=12).contains(&num) {
                Ok(num)
            } else {
                Err(format!("month \"{}\" not in the range 1 through 12", month).into())
            }
        },
        // 月名の場合
        Err(_) => {
            let lower = &month.to_lowercase();
            let matches: Vec<_> = MONTH_NAMES.iter()
                // インデックス番号と月名でイテレーション
                .enumerate()
                .filter_map(|(i, name)| {
                    // 先頭からの一致を確認
                    if name.to_lowercase().starts_with(lower) {
                        Some(i + 1) // 月の数値に変換
                    } else {
                        None // フィルタリングで除去される
                    }
                })
                // Some(_)のみを集約
                .collect();
            // 該当した月名が1つだけの場合
            if matches.len() == 1 {
                Ok(matches[0] as u32)
            // 該当なしまたは複数該当の場合
            } else {
                Err(format!("Invalid month \"{}\"", month).into())
            }
        }
    }
}

pub fn run(config: Config) -> MyResult<()> {
    match config.month {
        // 月指定がある時: 当月カレンダーのみを出力
        Some(month) => {
            let lines = format_month(config.year, month, true, config.today);
            println!("{}", lines.join("\n")); // カレンダーの各行を改行区切りで出力
        },
        // 月が未指定の時: 年単位のカレンダーを出力
        None => {
            println!("{:>32}", config.year);
            // 各月のカレンダーを取得
            let months: Vec<_> = (1..=12)
                .into_iter()
                .map(|month| {
                    format_month(config.year, month, false, config.today)
                })
                .collect();

            // 3ヶ月分ずつの並びで出力
            for (i, chunk) in months.chunks(3).enumerate() {
                if let [m1, m2, m3] = chunk {
                    for lines in izip!(m1, m2, m3) { // 各月の行をまとめてループ処理
                        println!("{}{}{}", lines.0, lines.1, lines.2);
                    }
                    // 次の3ヶ月との間に改行を挟む
                    if i < 3 {
                        println!();
                    }
                }
            }
        }
    }
    Ok(())
}

fn format_month(
    year: i32,
    month: u32,
    print_year: bool,
    today: NaiveDate,
) -> Vec<String> { // カレンダーを表す8行の文字列: 年月1行, 曜日1行, 日付6行
    let first = NaiveDate::from_ymd(year, month, 1);

    let mut days: Vec<String> = (1..first.weekday().number_from_sunday()) // 初日の曜日位置を数値で取得
        .into_iter()
        .map(|_| "  ".to_string()) // 初日の前の曜日を空白2マスで埋める: 日曜日から出力するため
        .collect();

    // 今日かどうかの判定式
    let is_today = |day: u32| {
        year == today.year() && month == today.month() && day == today.day()
    };

    // 最終日の取得
    let last = last_day_in_month(year, month);

    // 初日から最終日までをフォーマットして配列に追加
    days.extend((first.day()..=last.day()).into_iter()
        .map(|num| {
            let fmt = format!("{:>2}", num); // 右詰め2桁に整形
            if is_today(num) {
                Style::new().reverse().paint(fmt).to_string() // 今日の日付をハイライト
            } else {
                fmt
            }
        }));

    let month_name = MONTH_NAMES[month as usize - 1];

    let mut lines = Vec::with_capacity(8); // カレンダーを表す8行の文字列: 年月1行, 曜日1行, 日付6行

    // 年月の行を追加
    lines.push(format!(
        "{:^20}  ", // 20文字の中央揃え: 2マス空ける
        if print_year {
            format!("{} {}", month_name, year)
        } else {
            month_name.to_string()
        }
    ));

    // 曜日の行を追加
    lines.push("Su Mo Tu We Th Fr Sa  ".to_string()); // 2マス空ける

    // 各週の行を追加
    for week in days.chunks(7) { // 日付の配列を7要素ずつの塊としてループ処理
        lines.push(format!(
            "{:width$}  ", // 出力行サイズの指定 + 末尾$の追加 + 2マス空ける
            week.join(" "),
            width = LINE_WIDTH - 2 // 行末2マスを除くサイズ
        ));
    }

    while lines.len() < 8 { // 週数が少ない場合
        lines.push(" ".repeat(LINE_WIDTH)); // 行サイズ分の空白文字で埋める
    }

    lines
}

// 月末の日付情報を返す: うるう年の対策
fn last_day_in_month(year: i32, month: u32) -> NaiveDate {
    // 次の(年)月を計算
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    //次の年月の初日をもとに前日を返す
    NaiveDate::from_ymd(y, m, 1).pred()
}

// --------------------------------------------------
#[cfg(test)]
mod tests {
    use super::format_month;
    use super::last_day_in_month;
    use super::parse_int;
    use super::parse_month;
    use super::parse_year;

    use chrono::NaiveDate;

    #[test]
    fn test_parse_int() {
        // Parse positive int as usize
        let res = parse_int::<usize>("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1usize);

        // Parse negative int as i32
        let res = parse_int::<i32>("-1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), -1i32);

        // Fail on a string
        let res = parse_int::<i64>("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Invalid integer \"foo\"");
    }

    #[test]
    fn test_parse_year() {
        let res = parse_year("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1i32);

        let res = parse_year("9999");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 9999i32);

        let res = parse_year("0");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "year \"0\" not in the range 1 through 9999"
        );

        let res = parse_year("10000");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "year \"10000\" not in the range 1 through 9999"
        );

        let res = parse_year("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Invalid integer \"foo\"");
    }

    #[test]
    fn test_parse_month() {
        let res = parse_month("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1u32);

        let res = parse_month("12");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 12u32);

        let res = parse_month("jan");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1u32);

        let res = parse_month("0");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "month \"0\" not in the range 1 through 12"
        );

        let res = parse_month("13");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "month \"13\" not in the range 1 through 12"
        );

        let res = parse_month("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Invalid month \"foo\"");
    }

    #[test]
    fn test_format_month() {
        let today = NaiveDate::from_ymd(0, 1, 1);
        let leap_february = vec![
            "   February 2020      ",
            "Su Mo Tu We Th Fr Sa  ",
            "                   1  ",
            " 2  3  4  5  6  7  8  ",
            " 9 10 11 12 13 14 15  ",
            "16 17 18 19 20 21 22  ",
            "23 24 25 26 27 28 29  ",
            "                      ",
        ];
        assert_eq!(format_month(2020, 2, true, today), leap_february);

        let may = vec![
            "        May           ",
            "Su Mo Tu We Th Fr Sa  ",
            "                1  2  ",
            " 3  4  5  6  7  8  9  ",
            "10 11 12 13 14 15 16  ",
            "17 18 19 20 21 22 23  ",
            "24 25 26 27 28 29 30  ",
            "31                    ",
        ];
        assert_eq!(format_month(2020, 5, false, today), may);

        let april_hl = vec![
            "     April 2021       ",
            "Su Mo Tu We Th Fr Sa  ",
            "             1  2  3  ",
            " 4  5  6 \u{1b}[7m 7\u{1b}[0m  8  9 10  ",
            "11 12 13 14 15 16 17  ",
            "18 19 20 21 22 23 24  ",
            "25 26 27 28 29 30     ",
            "                      ",
        ];
        let today = NaiveDate::from_ymd(2021, 4, 7);
        assert_eq!(format_month(2021, 4, true, today), april_hl);
    }

    #[test]
    fn test_last_day_in_month() {
        assert_eq!(
            last_day_in_month(2020, 1),
            NaiveDate::from_ymd(2020, 1, 31)
        );
        assert_eq!(
            last_day_in_month(2020, 2),
            NaiveDate::from_ymd(2020, 2, 29)
        );
        assert_eq!(
            last_day_in_month(2020, 4),
            NaiveDate::from_ymd(2020, 4, 30)
        );
    }
}
