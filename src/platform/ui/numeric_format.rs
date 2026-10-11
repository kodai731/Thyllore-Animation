use imgui::internal::DataTypeKind;
use imgui::NumericFormat;

const FALLBACK_FORMAT: &str = "%.3f";

pub fn numeric_format<T: DataTypeKind>(format: &str) -> NumericFormat<'_, T> {
    match NumericFormat::new(format) {
        Ok(numeric_format) => numeric_format,
        Err(error) => {
            log_error!("Invalid numeric format {format:?}: {error}; using {FALLBACK_FORMAT}");
            NumericFormat::new(FALLBACK_FORMAT).expect("fallback numeric format is valid")
        }
    }
}
