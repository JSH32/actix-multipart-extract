/// This shouldn't be used or implemented manually.
/// Use [`actix_multipart_extract_derive::MultipartForm`].
pub trait MultipartForm {
    /// Get the max size of a named multipart field.
    /// The fields are named after serde renaming.
    fn max_size(field: &str) -> Option<usize>;
}
