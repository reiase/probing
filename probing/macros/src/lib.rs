use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro_attribute]
pub fn rpc(attr: TokenStream, input: TokenStream) -> TokenStream {
    let derive_meta = parse_macro_input!(attr as DeriveMeta);
    let input = parse_macro_input!(input as ItemTrait);
    let enum_name = format_ident!("RpcFor{}", input);

    let mut generated_methods = vec![];

    let expanded = quote! {

        pub enum #enum_name;

        #input

        pub struct RpcClient;

        impl #trait_name for RpcClient {
            #(#generated_methods)*
        }
    };
    expanded.into()
}
