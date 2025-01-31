use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Field, Fields, Meta, Token,
};

#[derive(Debug)]
struct OptionMetadata {
    field: String,
    name: String,
    aliases: Vec<String>,
    description: String,
    managed: bool,
}

#[proc_macro_derive(EngineExtension, attributes(option))]
pub fn derive_engine_extension(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    impl_engine_extension(&ast)
}

fn impl_engine_extension(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    let field_metadata = fields
        .iter()
        .map(parse_field_metadata)
        .filter(|x| x.managed)
        .collect::<Vec<_>>();
    // eprintln!("== {:?}", field_metadata);

    let get_matches = field_metadata.iter().map(|meta| {
        let field_name = meta.name.to_string();
        let aliases = &meta.aliases;
        let field_ident = format_ident!("{}", meta.field);
        quote! {
            #field_name | #(#aliases)|* => Ok(self.#field_ident.to_string())
        }
    });

    let set_matches = field_metadata.iter().map(|meta| {
        let field_name = meta.name.to_string();
        let aliases = &meta.aliases;
        let field_ident = format_ident!("{}", meta.field);
        let set_field = format_ident!("set_{}", meta.field);

        quote! {
            #field_name | #(#aliases)|* => {
                let old = self.#field_ident.to_string();
                let new = value.parse()
                .map_err(|_| EngineError::InvalidOption(key.to_string(), value.to_string()))?;
                // self.#field_ident = new.clone();
                self.#set_field(new)?;
                Ok(old)
            }
        }
    });

    let options = field_metadata.iter().map(|meta| {
        let name = &meta.name;
        let desc = &meta.description;
        let field_ident = format_ident!("{}", meta.field);

        quote! {
            EngineExtensionOption {
                key: #name.to_string(),
                value: Some(self.#field_ident.to_string()),
                help: #desc,
            }
        }
    });

    let expanded = quote! {
        impl EngineExtension for #name {
            fn name(&self) -> String {
                stringify!(#name).to_lowercase()
            }

            fn get(&self, key: &str) -> Result<String, EngineError> {
                match key {
                    #(#get_matches,)*
                    _ => Err(EngineError::UnsupportedOption(key.to_string()))
                }
            }

            fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError> {
                match key {
                    #(#set_matches,)*
                    _ => Err(EngineError::UnsupportedOption(key.to_string()))
                }
            }

            fn options(&self) -> Vec<EngineExtensionOption> {
                vec![
                    #(#options,)*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}

fn parse_field_metadata(field: &Field) -> OptionMetadata {
    let mut metadata = OptionMetadata {
        field: field.ident.as_ref().unwrap().to_string(),
        name: field.ident.as_ref().unwrap().to_string(),
        aliases: vec![],
        description: String::new(),
        managed: false,
    };

    let mut descriptions: Vec<String> = vec![];

    for attr in &field.attrs {
        if attr.path().is_ident("option") {
            if let Meta::List(list) = &attr.meta {
                metadata.managed = true;
                for nested in list
                    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                    .unwrap()
                    .iter()
                {
                    if let Meta::NameValue(nv) = nested {
                        let name = nv.path.get_ident().unwrap().to_string();
                        let value = match &nv.value {
                            syn::Expr::Lit(lit) => match &lit.lit {
                                syn::Lit::Str(s) => s.value(),
                                _ => continue,
                            },
                            syn::Expr::Array(array) => {
                                let values = array
                                    .elems
                                    .iter()
                                    .map(|e| match e {
                                        syn::Expr::Lit(lit) => match &lit.lit {
                                            syn::Lit::Str(s) => s.value(),
                                            _ => "".to_string(),
                                        },
                                        _ => "".to_string(),
                                    })
                                    .collect::<Vec<_>>();
                                format!("[{}]", values.join(","))
                            }
                            _ => {
                                eprintln!("Unsupported value type");
                                continue;
                            }
                        };

                        match name.as_str() {
                            "name" => metadata.name = value,
                            "aliases" => metadata.aliases = parse_string_array(&value),
                            _ => {}
                        }
                    }
                }
            } else {
                panic!("Invalid attribute format");
            }
        }
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                eprintln!("== {:?}", nv);
                if let syn::Expr::Lit(syn::ExprLit {
                    attrs: _,
                    lit: syn::Lit::Str(s),
                }) = &nv.value
                {
                    descriptions.push(s.value().trim().to_string());
                    // metadata.description = s.value();
                }
            }
        }
    }

    metadata.description = descriptions.join("\n");

    metadata
}

fn parse_string_array(input: &str) -> Vec<String> {
    input
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .collect()
}
