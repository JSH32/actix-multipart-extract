use actix_web::{dev::Payload, http::ConnectionType, FromRequest, HttpRequest, HttpResponse};
use futures::{Future, StreamExt, TryStreamExt};
use serde::Deserialize;
use serde_aux::prelude::serde_introspect;
use serde_json::{Map, Number, Value};
use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
};
use thiserror::Error;

use crate::{form::MultipartForm, MultipartConfig};

/// Error type for multipart forms.
#[derive(Error, Debug)]
pub enum MultipartError {
    #[error("Error while parsing field: {0}")]
    ParseError(serde_json::Error),
    #[error("File for field ({field}) was too large (max size: {limit} bytes)")]
    FileSizeError { field: String, limit: usize },
}

/// Representing a file in a multipart form.
#[derive(Debug, Deserialize)]
pub struct File {
    pub content_type: String,
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Extractor to extract multipart forms from the request
pub struct Multipart<T>(T);

impl<T> Deref for Multipart<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Multipart<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: serde::de::DeserializeOwned + MultipartForm> FromRequest for Multipart<T> {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let mut multipart = actix_multipart::Multipart::new(req.headers(), payload.take());
        let req_owned = req.to_owned();

        Box::pin(async move {
            let config = req_owned.app_data::<MultipartConfig>();

            match multipart_to_json::<T>(serde_introspect::<T>(), &mut multipart).await {
                Ok(v) => match serde_json::from_value::<T>(v) {
                    Ok(parsed) => Ok(Multipart(parsed)),
                    Err(err) => Err(handle_error(MultipartError::ParseError(err), config)),
                },
                Err(err) => Err(handle_error(err, config)),
            }
        })
    }
}

fn handle_error(error: MultipartError, config: Option<&MultipartConfig>) -> actix_web::Error {
    let mut res = match config {
        Some(config) => match &config.error_handler {
            Some(error_handler) => error_handler(error),
            None => HttpResponse::BadRequest().body(error.to_string()),
        },
        None => HttpResponse::BadRequest().body(error.to_string()),
    };

    // We must do this manually because of a bug in actix_http
    // Ideally we would have all errors be a `actix_web::Error` by default
    // SEE: https://github.com/actix/actix-web/pull/2779
    res.head_mut().set_connection_type(ConnectionType::Close);

    actix_web::error::InternalError::from_response("invalid multipart", res).into()
}

/// Convert a [`actix_multipart::Multipart`] form to a [`Value::Object`].
///
/// This checks for valid fields and file size limits on the [`MultipartForm`].
async fn multipart_to_json<T: MultipartForm>(
    valid_fields: &[&str],
    multipart: &mut actix_multipart::Multipart,
) -> Result<Value, MultipartError> {
    let mut map = Map::new();

    while let Ok(Some(mut field)) = multipart.try_next().await {
        let disposition = field.content_disposition().clone();

        let field_name = match disposition.get_name() {
            Some(v) => v,
            None => continue,
        };

        let field_name_formatted = field_name.replace("[]", "");

        // Make sure the field actually exists on the form
        if !valid_fields.contains(&field_name) {
            continue;
        }

        if field.content_disposition().get_filename().is_some() {
            // Is a file
            let mut data: Vec<Value> = Vec::new();

            let max_size = T::max_size(field_name);
            let mut size = 0;

            while let Some(chunk) = field.next().await {
                match chunk {
                    Ok(bytes) => {
                        size += bytes.len();
                        if let Some(max_size) = max_size {
                            if size > max_size {
                                return Err(MultipartError::FileSizeError {
                                    field: field_name.to_string(),
                                    limit: max_size,
                                });
                            }
                        }

                        data.reserve_exact(bytes.len());
                        for byte in bytes {
                            data.push(Value::Number(Number::from(byte)));
                        }
                    }
                    Err(_) => {
                        map.insert(field_name_formatted.to_owned(), Value::Null);
                        continue;
                    }
                }
            }

            let mut field_map = Map::new();
            field_map.insert(
                "content_type".to_owned(),
                Value::String(field.content_type().to_string()),
            );

            field_map.insert(
                "name".to_owned(),
                Value::String(
                    field
                        .content_disposition()
                        .get_filename()
                        .unwrap()
                        .to_string(),
                ),
            );

            field_map.insert("bytes".to_owned(), Value::Array(data));

            params_insert(
                &mut map,
                field_name,
                &field_name_formatted,
                Value::Object(field_map),
            );
        } else if let Some(Ok(value)) = field.next().await {
            // Not a file, parse as other JSON types
            if let Ok(str) = std::str::from_utf8(&value) {
                // Attempt to convert into a number
                match str.parse::<isize>() {
                    Ok(number) => params_insert(
                        &mut map,
                        field_name,
                        &field_name_formatted,
                        Value::Number(Number::from(number)),
                    ),
                    Err(_) => match str {
                        "true" => params_insert(
                            &mut map,
                            field_name,
                            &field_name_formatted,
                            Value::Bool(true),
                        ),
                        "false" => params_insert(
                            &mut map,
                            field_name,
                            &field_name_formatted,
                            Value::Bool(false),
                        ),
                        _ => params_insert(
                            &mut map,
                            field_name,
                            &field_name_formatted,
                            Value::String(str.to_owned()),
                        ),
                    },
                }
            }
        } else {
            // Nothing
            params_insert(&mut map, field_name, &field_name_formatted, Value::Null)
        }
    }

    Ok(Value::Object(map))
}

/// Insert params to the map. This works with individual fields and arrays.
fn params_insert(
    params: &mut Map<String, Value>,
    field_name: &str,
    field_name_formatted: &String,
    element: Value,
) {
    if field_name.ends_with("[]") {
        if params.contains_key(field_name_formatted) {
            if let Value::Array(val) = params.get_mut(field_name_formatted).unwrap() {
                val.push(element);
            }
        } else {
            params.insert(field_name_formatted.to_owned(), Value::Array(vec![element]));
        }
    } else {
        params.insert(field_name.to_owned(), element);
    }
}
