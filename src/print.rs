use crate::{
    functions::{WriteFileArgs, DeleteFileArgs, ReadFileArgs},
    socket::{FunctionCall, FunctionResult, FunctionReturnData},
};
use colored::Colorize;
use similar::{ChangeTag, TextDiff};

pub struct FunctionExecution {
    pub call: FunctionCall,
    pub result: FunctionResult,
}

/// Conventions:
/// - normal text is white
/// - numbers are yellow
/// - paths are cyan
pub fn print_function_execution(exec: FunctionExecution) {
    type Fn = FunctionCall;
    type Status = FunctionResult;
    type Data = FunctionReturnData;

    let result = match exec.result {
        Status::Success(x) => x,
        Status::Error(err) => {
            println!("{}", "ERROR".red().bold());
            dbg!(exec.call);
            dbg!(err);
            return;
        }
    };

    match (exec.call, result) {
        (Fn::ListFiles {}, Data::ListFiles(files)) => {
            println!(
                "{}{}{}",
                "Listing all (".white().bold(),
                files.len().to_string().yellow().bold(),
                ") files in repository.".white().bold()
            );
        }
        (Fn::WriteFile(WriteFileArgs { path, content }), Data::WriteFile(old_content)) => {
            let lines = content.lines().collect::<Vec<_>>();
            println!(
                "{} {} {} {} {}",
                "Created new file at".white().bold(),
                path.cyan().bold(),
                "with".white().bold(),
                lines.len().to_string().yellow().bold(),
                "lines".white().bold()
            );
            print_diff(&old_content.unwrap_or("".to_string()), &content);
        }
        (Fn::ReadFile(ReadFileArgs { path }), Data::ReadFile(file)) => {
            let line_count = file.map(|f| f.lines().count()).unwrap_or(0);
            println!(
                "{} {} {}{} {}",
                "Reading".white().bold(),
                path.cyan().bold(),
                "(".white().bold(),
                line_count.to_string().yellow().bold(),
                "lines)".white().bold()
            )
        }
        (Fn::DeleteFile(DeleteFileArgs { path }), _) => {
            println!(
                "{} {}",
                "Deleted file at".white().bold(),
                path.cyan().bold()
            );
        }
        (Fn::MoveFile(args), _) => {
            println!(
                "{} {} {} {}",
                "Moved".white().bold(),
                args.source_path.cyan().bold(),
                "to".white().bold(),
                args.destination_path.cyan().bold()
            );
        }
        (Fn::PrintMessage { message }, _) => {
            println!("{}", "Received message".white().bold());
            println!("{}", message);
        }
        v => panic!("unrecognised function pattern {:?}", v),
    }

    println!(); // Just to space things out a little.
}

fn print_diff(old: &str, new: &str) {
    let diff = TextDiff::from_lines(old, new);
    for change in diff
        .iter_all_changes()
        .filter(|c| c.tag() != ChangeTag::Equal)
    {
        let is_deletion = change.tag() == ChangeTag::Delete;
        print_line_content(
            if is_deletion {
                change.old_index().unwrap() + 1
            } else {
                change.new_index().unwrap() + 1
            },
            change.value(),
            is_deletion,
        )
    }
}

fn print_line_content(line_number: usize, content: &str, is_deletion: bool) {
    let line_mod = format!(
        "{} {}. {}",
        if is_deletion { "-" } else { "+" },
        line_number,
        content.trim_end(),
    );
    println!(
        "{}",
        if is_deletion {
            line_mod.red()
        } else {
            line_mod.green()
        }
    )
}

// #[cfg(test)]
// mod tests {
//     use crate::functions::ModifyFileResult;
//
//     use super::*;
//
//     #[test]
//     #[ignore]
//     fn test_print_modification() {
//         let fn_exec = FunctionExecution {
//             call: FunctionCall::ModifyFile(ModifyFileArgs {
//                 path: "/src/user.js".to_string(),
//                 start_line: 2,
//                 end_line: Some(2),
//                 mode: crate::functions::ModificationMode::Replace,
//                 content: "".to_string(),
//             }),
//             result: FunctionResult::Success(FunctionReturnData::ModifyFile(ModifyFileResult {
//                 old_contents: "Line 1\nLine 2\nLine 3\n".to_string(),
//                 new_contents: "Line 1\nLine 4\nLine 3\n".to_string(),
//             })),
//         };
//         print_function_execution(fn_exec);
//     }
// }
