#![recursion_limit = "128"]
extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;

// For now, assume (not check) that these macroses are applied to enum without
// data.  If this assumption is violated, compiler error will point to
// generated code, which is not very user-friendly.

#[proc_macro_derive(Sqlx)]
pub fn sqlx_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let gen = quote! {
        impl<'q> sqlx::encode::Encode<'q, sqlx::sqlite::Sqlite> for #name {
            fn encode_by_ref(&self, buf: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>) -> sqlx::encode::IsNull{
                num_traits::ToPrimitive::to_i32(self).expect("invalid type").encode(buf)
            }
        }


        impl<'de> sqlx::decode::Decode<'de, sqlx::sqlite::Sqlite> for #name {
            fn decode(value: sqlx::sqlite::SqliteValueRef) -> std::result::Result<Self, sqlx::BoxDynError> {
                let raw: i32 = sqlx::decode::Decode::decode(value)?;

                Ok(num_traits::FromPrimitive::from_i32(raw).unwrap_or_default())
            }
        }

        impl sqlx::types::Type<sqlx::sqlite::Sqlite> for #name {
            fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
                <i32 as sqlx::types::Type<_>>::type_info()
            }
        }

    };
    gen.into()
}
