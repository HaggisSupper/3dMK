#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[tauri::command]
fn backend_status()->String{"Spatial core command boundary operational; load a project to begin.".to_string()}
fn main(){tauri::Builder::default().invoke_handler(tauri::generate_handler![backend_status]).run(tauri::generate_context!()).expect("failed to run Tauri application");}
