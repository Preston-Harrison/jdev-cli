use crate::{
    functions::{
        WriteFileArgs, DeleteFileArgs, Functions, ModifyFileArgs, ModifyFileResult, MoveFileArgs,
        ReadFileArgs,
    },
    print::{print_function_execution, FunctionExecution},
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "function", content = "args", rename_all = "snake_case")]
pub enum FunctionCall {
    GetAllFiles {},
    WriteFile(WriteFileArgs),
    ReadFile(ReadFileArgs),
    DeleteFile(DeleteFileArgs),
    ModifyFile(ModifyFileArgs),
    MoveFile(MoveFileArgs),
    PrintMessage { message: String },
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged, rename_all = "snake_case")]
pub enum FunctionReturnData {
    Null(()),
    GetAllFiles(Vec<String>),
    WriteFile(Option<String>),
    ModifyFile(ModifyFileResult),
    ReadFile(Option<String>),
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum FunctionResult {
    Success(FunctionReturnData),
    Error(String),
}

macro_rules! call {
    ($func_call:expr, $variant:ident) => {{
        $func_call
            .map(|x| FunctionResult::Success(FunctionReturnData::$variant(x)))
            .unwrap_or_else(|e| FunctionResult::Error(e.to_string()))
    }};
}

pub async fn connect(
    functions: Functions,
    query: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the websocket server
    let (ws_stream, _) = connect_async("ws://localhost:5000")
        .await
        .expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    write.send(Message::Text(query)).await?;

    // Read messages from the server
    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                let call = match serde_json::from_str::<FunctionCall>(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Err: {}", e);
                        eprintln!("Text: {}", text);
                        continue;
                    }
                };
                let result = match call.clone() {
                    FunctionCall::GetAllFiles {} => call!(functions.get_all_files(), GetAllFiles),
                    FunctionCall::ReadFile(args) => call!(functions.read_file(args), ReadFile),
                    FunctionCall::WriteFile(args) => call!(functions.write_file(args), WriteFile),
                    FunctionCall::DeleteFile(args) => call!(functions.delete_file(args), Null),
                    FunctionCall::ModifyFile(args) => {
                        call!(functions.modify_file(args), ModifyFile)
                    }
                    FunctionCall::MoveFile(args) => call!(functions.move_file(args), Null),
                    FunctionCall::PrintMessage { message } => {
                        println!("{}", message);
                        return Ok(());
                    }
                };
                print_function_execution(FunctionExecution {
                    call,
                    result: result.clone(),
                });
                let result_str = Message::Text(serde_json::to_string(&result).unwrap());
                if let Err(err) = write.send(result_str).await {
                    eprintln!("Error sending outgoing message: {}", &err)
                }
            }
            Ok(_) => println!("Received non-text message"),
            Err(e) => eprintln!("Error handling incoming message: {}", &e),
        }
    }

    Ok(())
}
