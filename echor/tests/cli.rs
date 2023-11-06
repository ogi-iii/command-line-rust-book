use std::{fs, error::Error};

use assert_cmd::Command;
use predicates::str::contains;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn dies_no_args() -> TestResult {
    let mut cmd = Command::cargo_bin("echor")?;
    cmd.assert() // execute without arguments
        .failure()
        .stderr(contains("USAGE")); // 出力結果が部分一致することを確認する
    Ok(())
}

// helper function
fn run(args: &[&str], expected_filename: &str) -> TestResult {
    let expected = fs::read_to_string(expected_filename)?;
    Command::cargo_bin("echor")?
        .args(args) // 固定長のスライスを渡す
        .assert()
        .success()
        .stdout(expected);
    Ok(())
}

#[test]
fn hello1() -> TestResult {
    let outfile = "tests/expected/hello1.txt";
    // let expected = fs::read_to_string(outfile)?; // ファイル記載内容を一括読み込み: ダンプ読み込みなのでファイルサイズに注意！
    // let mut cmd = Command::cargo_bin("echor")?;
    // cmd.arg("Hello there")
    //     .assert()
    //     .success()
    //     .stdout(expected); // 出力結果が完全一致することを確認する
    // Ok(())
    run(&["Hello there"], outfile)
}

#[test]
fn hello2() -> TestResult {
    // let expected = fs::read_to_string("tests/expected/hello2.txt")?;
    // let mut cmd = Command::cargo_bin("echor")?;
    // cmd.args(vec!["Hello", "there"])
    //     .assert()
    //     .success()
    //     .stdout(expected);
    // Ok(())
    run(&["Hello", "there"], "tests/expected/hello2.txt")
}

#[test]
fn hello1_no_newline() -> TestResult {
    run(&["Hello  there", "-n"], "tests/expected/hello1.n.txt")
}

#[test]
fn hello2_no_newline() -> TestResult {
    run(&["-n", "Hello", "there"], "tests/expected/hello2.n.txt")
}
