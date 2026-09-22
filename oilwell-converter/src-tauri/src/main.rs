use oilwell_core::{convert, ConversionRequest, ConversionResult};

#[tauri::command]
fn convert_model(request: ConversionRequest) -> Result<ConversionResult, oilwell_core::ConversionError> { convert(&request) }
#[tauri::command]
fn choose_csv_file() -> Option<String> { rfd::FileDialog::new().add_filter("CSV tables", &["csv"]).pick_file().map(|path| path.to_string_lossy().into_owned()) }
#[tauri::command]
fn choose_output_folder() -> Option<String> { rfd::FileDialog::new().pick_folder().map(|path| path.to_string_lossy().into_owned()) }

fn main() { tauri::Builder::default().invoke_handler(tauri::generate_handler![convert_model, choose_csv_file, choose_output_folder]).run(tauri::generate_context!()).expect("Unable to run Oilwell Converter"); }
