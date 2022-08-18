# actix-multipart-extract [![Latest Version]][crates.io]
This crate is a Rust library for providing proper multipart support to actix 4.

[Latest Version]: https://img.shields.io/crates/v/actix-multipart-extract
[crates.io]: https://crates.io/crates/actix-multipart-extract

This is able to parse a multipart request into a struct and validate the request properties. It uses serde for deserialization and a `MultipartForm` derive.

# Installation
Add `actix_multipart_extract` to your Cargo.toml:
```toml
[dependencies]
actix-multipart-extract = "0.5"
```
### Example:
```rust
use actix_multipart_extract::{File, Multipart, MultipartForm};
use actix_web::{post, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;

// File, String, bool, and number types are the only supported types for forms.
// Vec and Option may also be used with one of the 4 types as the type param.
// Some serde attributes will work with forms.
#[derive(Deserialize, MultipartForm, Debug)]
pub struct ExampleForm {
    #[multipart(max_size = 5MB)]
    file_field: File,
    string_field: Vec<String>, // list field
    bool_field: Option<bool>,  // optional field
}

#[post("/example")]
async fn example(example_form: Multipart<ExampleForm>) -> impl Responder {
    // File field
    println!("File size: {}", example_form.file_field.bytes.len());
    println!(
        "File content type: {}",
        example_form.file_field.content_type
    );
    println!("File name: {}", example_form.file_field.name);

    // List of strings field
    println!("List of strings: {:?}", example_form.string_field);

    // Optional bool field
    match example_form.bool_field {
        Some(v) => println!("Has bool field: {}", v),
        None => println!("No bool field"),
    }

    HttpResponse::Ok()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || App::new().service(example))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
```