use actix_web::HttpResponse;

use crate::MultipartError;

type MultipartErrorHandler = Box<dyn Fn(MultipartError) -> HttpResponse + Send + Sync + 'static>;

/// Config for Multipart data, insert with [`actix_web::App::app_data`] to actix
pub struct MultipartConfig {
    pub error_handler: Option<MultipartErrorHandler>,
}

impl MultipartConfig {
    pub fn set_error_handler<F>(mut self, error_handler: F) -> Self
    where
        F: Fn(MultipartError) -> HttpResponse + Send + Sync + 'static,
    {
        self.error_handler = Some(Box::new(error_handler));
        self
    }
}

impl Default for MultipartConfig {
    fn default() -> Self {
        Self {
            error_handler: None,
        }
    }
}
