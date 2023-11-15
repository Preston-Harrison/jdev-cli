use crate::{
    print::print_function_result,
    tools::{CreateFileArgs, DeleteFileArgs, Functions, LineModification, ModifyFileArgs},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "function", content = "args", rename_all = "snake_case")]
pub enum FunctionCall {
    GetAllFiles,
    CreateFile(CreateFileArgs),
    DeleteFile(DeleteFileArgs),
    ModifyFile(ModifyFileArgs),
    PrintMessage { message: String },
}

#[derive(Serialize, Debug)]
#[serde(untagged, rename_all = "snake_case")]
pub enum FunctionReturnData {
    Null(()),
    GetAllFiles(Vec<String>),
    ModifyFile(Vec<LineModification>),
}

#[derive(Serialize, Debug)]
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

pub async fn connect(caller: Functions) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the websocket server
    let (ws_stream, _) = connect_async("ws://localhost:8080")
        .await
        .expect("Failed to connect");
    let (write, mut read) = ws_stream.split();

    // Read messages from the server
    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                let function_call = match serde_json::from_str::<FunctionCall>(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("{}", e);
                        continue;
                    }
                };
                let result = match function_call.clone() {
                    FunctionCall::GetAllFiles => call!(caller.get_all_files(), GetAllFiles),
                    FunctionCall::CreateFile(args) => call!(caller.create_file(args), Null),
                    FunctionCall::DeleteFile(args) => call!(caller.delete_file(args), Null),
                    FunctionCall::ModifyFile(args) => call!(caller.modify_file(args), ModifyFile),
                    FunctionCall::PrintMessage { message } => {
                        // TODO figure out if the user should be prompted.
                        println!("{}", message);
                        return Ok(());
                    }
                };
                print_function_result(&function_call, &result)
            }
            Ok(_) => println!("Received non-text message"),
            Err(e) => eprintln!("Error handling incoming message: {}", &e),
        }
    }

    Ok(())
}
