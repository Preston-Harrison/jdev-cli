use crate::{
    functions::{CreateFileArgs, DeleteFileArgs, ModifyFileArgs, ReadFileArgs},
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
        (Fn::GetAllFiles {}, Data::GetAllFiles(files)) => {
            println!(
                "{}{}{}",
                "Listing all (".white().bold(),
                files.len().to_string().yellow().bold(),
                ") files in repository.".white().bold()
            );
        }
        (Fn::CreateFile(CreateFileArgs { path, content }), _) => {
            let lines = content.lines().collect::<Vec<_>>();
            println!(
                "{} {} {} {} {}",
                "Created new file at".white().bold(),
                path.cyan().bold(),
                "with".white().bold(),
                lines.len().to_string().yellow().bold(),
                "lines".white().bold()
            );
            for (i, line) in lines.into_iter().enumerate() {
                print_line_content(i + 1, line, false);
            }
        }
        (Fn::ReadFile(ReadFileArgs { path }), Data::ReadFile(file)) => {
            let line_count = file.lines().count();
            println!(
                "{} {} {}{}{}",
                "Reading".white().bold(),
                path.cyan().bold(),
                "(".white().bold(),
                line_count.to_string().yellow().bold(),
                ") lines".white().bold()
            )
        }
        (Fn::DeleteFile(DeleteFileArgs { path }), _) => {
            println!(
                "{} {}",
                "Deleted file at".white().bold(),
                path.cyan().bold()
            );
        }
        (Fn::ModifyFile(ModifyFileArgs { path, content }), Data::ModifyFile(old_content)) => {
            println!(
                "{} {}",
                "Modified file at".white().bold(),
                path.cyan().bold()
            );
            let diff = TextDiff::from_lines(&old_content, &content);
            for change in diff.iter_all_changes() {
                let is_deletion = change.tag() == ChangeTag::Delete;
                print_line_content(
                    if is_deletion {
                        change.old_index().unwrap()
                    } else {
                        change.new_index().unwrap()
                    },
                    change.value(),
                    is_deletion,
                )
            }
        }
        (Fn::PrintMessage { message }, _) => {
            println!("{}", "Received message".white().bold());
            println!("{}", message);
        }
        v => panic!("unrecognised function pattern {:?}", v),
    }

    println!(); // Just to space things out a little.
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
