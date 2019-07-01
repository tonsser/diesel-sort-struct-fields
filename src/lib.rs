//! Macro to sort struct fields and `table!` columns to avoid subtle bugs.
//!
//! The way Diesel maps a response from a query into a struct is by treating a row as a tuple and
//! assigning the fields in the order of the fields in code. Something like (not real code):
//!
//! ```rust,ignore
//! struct User {
//!     id: i32,
//!     name: String,
//! }
//!
//! fn user_from_row(row: (i32, String)) -> User {
//!     User {
//!         id: row.0,
//!         name: row.1,
//!     }
//! }
//! ```
//!
//! This works well, but it will break in subtle ways if the order of `id` and `name` aren't the
//! same in `table!` and `struct User { ... }`. So this code doesn't compile:
//!
//! ```rust,ignore
//! #[macro_use]
//! extern crate diesel;
//!
//! use diesel::prelude::*;
//!
//! table! {
//!     users {
//!         // order here doesn't match order in the struct
//!         name -> VarChar,
//!         id -> Integer,
//!     }
//! }
//!
//! #[derive(Queryable)]
//! struct User {
//!     id: i32,
//!     name: String,
//! }
//!
//! fn main() {
//!     let db = connect_to_db();
//!
//!     users::table
//!         .select(users::all_columns)
//!         .load::<User>(&db)
//!         .unwrap();
//! }
//!
//! fn connect_to_db() -> PgConnection {
//!     PgConnection::establish("postgres://localhost/diesel-sort-struct-fields").unwrap()
//! }
//! ```
//!
//! Luckily you get a type error, so Diesel is clearly telling you that something is wrong. However
//! if the types of `id` and `name` were the same you wouldn't get a type error. You would just
//! have subtle bugs that could take hours to track down (it did for me).
//!
//! This crate prevents that with a simple procedural macro that sorts the fields of your model
//! struct and `table!` such that you can define them in any order, but once the code gets to the
//! compiler the order will always be the same.
//!
//! Example:
//!
//! ```rust
//! #[macro_use]
//! extern crate diesel;
//!
//! use diesel_sort_struct_fields::{sort_columns, sort_fields};
//! use diesel::prelude::*;
//!
//! #[sort_columns]
//! table! {
//!     users {
//!         name -> VarChar,
//!         id -> Integer,
//!     }
//! }
//!
//! #[sort_fields]
//! #[derive(Queryable)]
//! struct User {
//!     id: i32,
//!     name: String,
//! }
//!
//! fn main() {
//!     let db = connect_to_db();
//!
//!     let users = users::table
//!         .select(users::all_columns)
//!         .load::<User>(&db)
//!         .unwrap();
//!
//!     assert_eq!(0, users.len());
//! }
//!
//! fn connect_to_db() -> PgConnection {
//!     PgConnection::establish("postgres://localhost/diesel-sort-struct-fields").unwrap()
//! }
//! ```

#![deny(unused_imports, dead_code, unused_variables, unused_must_use, missing_docs)]
#![doc(html_root_url = "https://docs.rs/diesel-sort-struct-fields/0.1.0")]

extern crate proc_macro;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseBuffer, ParseStream},
    parse2,
    parse_macro_input::parse,
    punctuated::Punctuated,
    spanned::Spanned,
    DeriveInput, Ident, Token,
};

type Result<A, B = syn::Error> = std::result::Result<A, B>;

/// Sort fields in a model struct.
///
/// See crate level docs for more info.
#[proc_macro_attribute]
pub fn sort_fields(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ast = match syn::parse_macro_input::parse::<DeriveInput>(item) {
        Ok(ast) => ast,
        Err(err) => return err.to_compile_error().into(),
    };

    match expand_sorted(attr.into(), ast) {
        Ok(out) => out.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Sort columns in a `table!` macro.
///
/// See crate level docs for more info.
#[proc_macro_attribute]
pub fn sort_columns(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    if !attr.is_empty() {
        let attr: TokenStream = attr.into();
        return syn::Error::new(
            attr.span(),
            "`#[sort_columns]` doesn't support any attributes",
        )
        .to_compile_error()
        .into();
    }

    let ast = match parse::<syn::Macro>(item) {
        Ok(ast) => ast,
        Err(err) => return sort_columns_on_wrong_item_error(err.span()).into(),
    };

    let ident = &ast.path.segments.last().unwrap().value().ident;
    if ident != "table" {
        return sort_columns_on_wrong_item_error(ident.span()).into();
    }

    match parse2::<TableDsl>(ast.tts) {
        Ok(table_dsl) => {
            let tokens = quote! { #table_dsl };

            tokens.into()
        }
        Err(err) => err.to_compile_error().into(),
    }
}

fn sort_columns_on_wrong_item_error(span: Span) -> TokenStream {
    syn::Error::new(
        span,
        "`#[sort_columns]` only works on the `diesel::table!` macro",
    )
    .to_compile_error()
}

#[derive(Debug)]
struct TableDsl {
    name: Ident,
    id_columns: Option<Punctuated<Ident, Token![,]>>,
    columns: Punctuated<ColumnDsl, Token![,]>,
    use_statements: Vec<syn::ItemUse>,
    attributes: Vec<syn::Attribute>,
}

impl Parse for TableDsl {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let mut use_statements = Vec::new();

        while let Some(stmt) = input.parse::<syn::ItemUse>().ok() {
            use_statements.push(stmt)
        }

        let attributes = input.call(syn::Attribute::parse_outer)?;
        let name = input.parse::<Ident>()?;

        let id_columns = match try_parse_parens(input) {
            Ok(inside_parens) => {
                let id_columns = Punctuated::<Ident, Token![,]>::parse_terminated(&inside_parens)?;
                Some(id_columns)
            }
            Err(_) => None,
        };

        let inside_braces;
        syn::braced!(inside_braces in input);
        let columns = Punctuated::<ColumnDsl, Token![,]>::parse_terminated(&inside_braces)?;

        Ok(TableDsl {
            name,
            id_columns,
            columns,
            use_statements,
            attributes,
        })
    }
}

impl ToTokens for TableDsl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let table_name = &self.name;
        let attributes = &self.attributes;

        let id_column = if let Some(id_columns) = &self.id_columns {
            quote! { ( #id_columns ) }
        } else {
            quote! {}
        };
        let use_statements = &self.use_statements;

        let columns = sort_punctuated(&self.columns, |column| &column.name);

        tokens.extend(quote! {
            diesel::table! {
                #(#use_statements)*

                #( #attributes )*
                #table_name #id_column {
                    #( #columns )*
                }
            }
        })
    }
}

#[derive(Debug)]
struct ColumnDsl {
    name: Ident,
    ty: ColumnType,
    attributes: Vec<syn::Attribute>,
}

impl ToTokens for ColumnDsl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let ty = &self.ty;
        let attributes = &self.attributes;

        tokens.extend(quote! {
            #(#attributes)*
            #name -> #ty,
        })
    }
}

impl Parse for ColumnDsl {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let attributes = input.call(syn::Attribute::parse_outer)?;

        let name = input.parse::<Ident>()?;
        input.parse::<Token![-]>()?;
        input.parse::<Token![>]>()?;

        let outer_ty = input.parse::<Ident>()?;
        let ty = if input.peek(Token![<]) {
            input.parse::<Token![<]>()?;
            let ty = input.parse::<Ident>()?;
            input.parse::<Token![>]>()?;
            ColumnType::Wrapped(outer_ty, ty)
        } else {
            ColumnType::Bare(outer_ty)
        };

        Ok(ColumnDsl {
            name,
            ty,
            attributes,
        })
    }
}

#[derive(Debug)]
enum ColumnType {
    Bare(Ident),
    Wrapped(Ident, Ident),
}

impl ToTokens for ColumnType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            ColumnType::Bare(ty) => tokens.extend(quote! { #ty }),
            ColumnType::Wrapped(constructor, ty) => tokens.extend(quote! { #constructor<#ty> }),
        }
    }
}

fn try_parse_parens<'a>(input: ParseStream<'a>) -> syn::parse::Result<ParseBuffer<'a>> {
    (|| {
        let inside_parens;
        syn::parenthesized!(inside_parens in input);
        Ok(inside_parens)
    })()
}

fn expand_sorted(
    attr: proc_macro2::TokenStream,
    ast: DeriveInput,
) -> Result<proc_macro2::TokenStream> {
    if !attr.is_empty() {
        return Err(syn::Error::new(
            attr.span(),
            "`#[sort_fields]` doesn't support any attributes",
        ));
    }

    let attrs = ast.attrs;
    let vis = ast.vis;
    let ident = ast.ident;
    let generics = ast.generics;

    let sorted_fieds = find_and_sort_struct_fields(&ast.data, ident.span())?;

    let tokens = quote! {
        #(#attrs)*
        #vis struct #ident #generics {
            #( #sorted_fieds ),*
        }
    };

    Ok(tokens)
}

fn sort_punctuated<A, B, F, K>(punctuated: &Punctuated<A, B>, f: F) -> Vec<&A>
where
    F: Fn(&A) -> &K,
    K: Ord,
{
    let mut items = punctuated.iter().collect::<Vec<_>>();
    items.sort_unstable_by_key(|item| f(item));
    items
}

fn find_and_sort_struct_fields(data: &syn::Data, ident_span: Span) -> Result<Vec<&syn::Field>> {
    match data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields) => {
                let fields = sort_punctuated(&fields.named, |field| &field.ident);
                Ok(fields)
            }
            syn::Fields::Unnamed(fields) => Err(syn::Error::new(
                fields.span(),
                "`#[sort_fields]` is not allowed on tuple structs, only structs with named fields",
            )),
            syn::Fields::Unit => Err(syn::Error::new(
                ident_span,
                "`#[sort_fields]` is not allowed on unit structs, only structs with named fields",
            )),
        },
        syn::Data::Enum(data) => Err(syn::Error::new(
            data.enum_token.span(),
            "`#[sort_fields]` is not allowed on enums, only structs",
        )),
        syn::Data::Union(data) => Err(syn::Error::new(
            data.union_token.span(),
            "`#[sort_fields]` is not allowed on unions, only structs",
        )),
    }
}

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/*.rs");
    t.compile_fail("tests/compile_fail/*.rs");
}
