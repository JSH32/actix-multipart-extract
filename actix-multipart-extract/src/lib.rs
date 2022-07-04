mod config;
mod extractor;

/// Don't use this module directly, use [`actix_multipart_extract_derive::MultipartForm`].
pub mod form;

pub use config::*;
pub use extractor::*;

pub use actix_multipart_extract_derive::MultipartForm;

/// Required for proc-macro usage at runtime.
pub use serde_aux::serde_introspection::serde_introspect;
