use std::path::Path;

use taskhud_core::{TodoDocument, TodoError, TodoSnapshot, ensure_todo_path};

fn command_error(error: TodoError) -> String {
    error.to_string()
}

#[tauri::command]
fn read_todo(path: String) -> Result<TodoSnapshot, String> {
    let path = ensure_todo_path(Path::new(&path)).map_err(command_error)?;
    TodoDocument::load(path)
        .map(|document| document.snapshot())
        .map_err(command_error)
}

#[tauri::command]
fn save_todo(
    path: String,
    expected_version: String,
    content: String,
) -> Result<TodoSnapshot, String> {
    let path = ensure_todo_path(Path::new(&path)).map_err(command_error)?;
    TodoDocument::save_if_unchanged(path, &expected_version, &content)
        .map(|document| document.snapshot())
        .map_err(command_error)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![read_todo, save_todo])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
