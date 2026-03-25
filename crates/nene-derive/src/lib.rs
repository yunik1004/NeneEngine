use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Expr, ExprLit, Fields, Lit, Type, parse_macro_input};

#[proc_macro_attribute]
pub fn vertex(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => panic!("#[vertex] only supports structs with named fields"),
        },
        _ => panic!("#[vertex] only supports structs"),
    };

    let attributes: Vec<TokenStream2> = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let field_name = &field.ident;
            let format = field_type_to_format(&field.ty);
            let location = i as u32;
            quote! {
                ::nene::renderer::VertexAttribute {
                    offset: ::std::mem::offset_of!(#name, #field_name) as u64,
                    location: #location,
                    format: #format,
                }
            }
        })
        .collect();

    let expanded = quote! {
        #[repr(C)]
        #[derive(Copy, Clone)]
        #input

        unsafe impl ::bytemuck::Zeroable for #name {}
        unsafe impl ::bytemuck::Pod for #name {}

        impl #name {
            pub fn layout() -> ::nene::renderer::VertexLayout {
                ::nene::renderer::VertexLayout {
                    stride: ::std::mem::size_of::<#name>() as u64,
                    attributes: vec![#(#attributes),*],
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn uniform(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        #[repr(C)]
        #[derive(Copy, Clone)]
        #input

        unsafe impl ::bytemuck::Zeroable for #name {}
        unsafe impl ::bytemuck::Pod for #name {}
    };

    TokenStream::from(expanded)
}

fn field_type_to_format(ty: &Type) -> TokenStream2 {
    if let Type::Array(arr) = ty
        && let Expr::Lit(ExprLit {
            lit: Lit::Int(n), ..
        }) = &arr.len
    {
        let count: usize = n.base10_parse().unwrap();
        if let Type::Path(p) = arr.elem.as_ref()
            && p.path.is_ident("f32")
        {
            return match count {
                2 => quote! { ::nene::renderer::VertexFormat::Float32x2 },
                3 => quote! { ::nene::renderer::VertexFormat::Float32x3 },
                4 => quote! { ::nene::renderer::VertexFormat::Float32x4 },
                _ => panic!("Unsupported [f32; {}] — use 2, 3, or 4", count),
            };
        }
    }
    panic!("Unsupported vertex field type. Supported types: [f32; 2], [f32; 3], [f32; 4]")
}
