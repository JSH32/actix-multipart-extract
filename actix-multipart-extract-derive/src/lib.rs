use parse_size::parse_size;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Field, Fields, FieldsNamed, Ident, Lit, Meta, MetaList,
    MetaNameValue, NestedMeta, Path,
};

#[proc_macro_derive(MultipartForm, attributes(multipart))]
pub fn multipart_form(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident;

    let fields = if let Data::Struct(syn::DataStruct {
        fields: Fields::Named(FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        panic!("can only derive on a struct")
    };

    let field_max_sizes = fields.iter().map(|field| {
        let Field { attrs, .. } = field;

        for attr in attrs {
            if let Ok(meta) = attr.parse_meta() {
                if let Meta::List(MetaList { path, nested, .. }) = meta {
                    // Check for multipart attribute.
                    if path.get_ident().unwrap()
                        != &Ident::new("multipart", proc_macro2::Span::call_site())
                    {
                        continue;
                    }

                    if let Some(NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        path: Path { segments, .. },
                        lit,
                        ..
                    }))) = nested.first()
                    {
                        for segment in segments {
                            if &segment.ident == &Ident::new("max_size", Span::call_site()) {
                                let lit_string = match lit {
                                    Lit::Int(l) => l.to_string(),
                                    Lit::Float(f) => f.to_string(),
                                    _ => {
                                        return syn::Error::new(
                                            lit.span(),
                                            "must be a number with size suffix",
                                        )
                                        .to_compile_error()
                                    }
                                };

                                let max_size = match parse_size(lit_string) {
                                    Ok(v) => v as usize,
                                    Err(_) => {
                                        return syn::Error::new(lit.span(), "invalid size")
                                            .to_compile_error();
                                    }
                                };

                                return quote! { Some(#max_size) };
                            }
                        }
                    }
                }
            }
        }

        quote! { None }
    });

    let field_len = field_max_sizes.len();

    let expanded = quote! {
        impl actix_multipart_extract::form::MultipartForm for #name {
            fn max_size(field: &str) -> Option<usize> {
                // Array of max sizes ordered by field.
                static max_sizes: [Option<usize>; #field_len] = [#(#field_max_sizes,)*];

                // Serde renamed field names ordered by field.
                let introspected = actix_multipart_extract::serde_introspect::<Self>();

                match introspected.iter().position(|f| f == &field) {
                    Some(i) => max_sizes[i],
                    None => None
                }
            }
        }
    };

    expanded.into()
}
