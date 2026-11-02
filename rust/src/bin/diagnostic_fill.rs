use newchan_rust::diagnostic_fill::{compare, replay, Checkpoint, Replay, Request};
use std::path::Path;

fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let bytes = std::fs::read(Path::new(path)).map_err(|e| format!("读取输入失败：{e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("解析输入失败：{e}"))
}

fn run(args: &[String]) -> Result<serde_json::Value, String> {
    match args {
        [command, input] if command == "run" => {
            Ok(serde_json::to_value(replay(read::<Request>(input)?)?).map_err(|e| e.to_string())?)
        }
        [command, input] if command == "compare" => {
            Ok(serde_json::to_value(compare(&read::<Request>(input)?)?).map_err(|e| e.to_string())?)
        }
        [command, input, count] if command == "checkpoint" => {
            let mut state = Replay::new(read(input)?)?;
            state.advance_to(count.parse().map_err(|_| "游标须非负整数")?)?;
            serde_json::to_value(state.checkpoint()).map_err(|e| e.to_string())
        }
        [command, input, checkpoint] if command == "resume" => {
            let request: Request = read(input)?;
            let checkpoint: Checkpoint = read(checkpoint)?;
            let mut state = Replay::restore(checkpoint, &request)?;
            state.advance_to(request.bars.len())?;
            serde_json::to_value(state.report()).map_err(|e| e.to_string())
        }
        _ => Err("用法：diagnostic_fill {run|compare} REQUEST.json，或 checkpoint REQUEST.json N，或 resume REQUEST.json CHECKPOINT.json".into()),
    }
}

fn main() -> std::process::ExitCode {
    match run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(value) => {
            println!("{value}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
