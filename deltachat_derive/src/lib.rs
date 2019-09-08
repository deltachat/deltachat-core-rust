#![recursion_limit = "128"]
extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;
use syn;

// For now, assume (not check) that these macroses are applied to enum without
// data.  If this assumption is violated, compiler error will point to
// generated code, which is not very user-friendly.

#[proc_macro_derive(ToSql)]
pub fn to_sql_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let gen = quote! {
        impl rusqlite::types::ToSql for #name {
            fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
                let num = *self as i64;
                let value = rusqlite::types::Value::Integer(num);
                let output = rusqlite::types::ToSqlOutput::Owned(value);
                std::result::Result::Ok(output)
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(FromSql)]
pub fn from_sql_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let gen = quote! {
        impl rusqlite::types::FromSql for #name {
            fn column_result(col: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
                let inner = rusqlite::types::FromSql::column_result(col)?;
                Ok(num_traits::FromPrimitive::from_i64(inner).unwrap_or_default())
            }
        }
    };
    gen.into()
}
