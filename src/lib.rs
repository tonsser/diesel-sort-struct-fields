extern crate proc_macro;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream, ParseBuffer},
    parse2,
    parse_macro_input::parse,
    punctuated::Punctuated,
    spanned::Spanned,
    DeriveInput, Ident, Token,
};

type Result<A, B = syn::Error> = std::result::Result<A, B>;

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

// TODO:
// - Composite primary keys
// - Docs on the table
// - Docs on the columns
// - #[sql_name = "type"] attribute
// - Convert asserts to real errors
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
        Err(err) => return err.to_compile_error().into(),
    };

    assert_eq!(&ast.path.segments.last().unwrap().value().ident, "table");

    match parse2::<TableDsl>(ast.tts) {
        Ok(table_dsl) => {
            let tokens = quote! { #table_dsl };

            tokens.into()
        }
        Err(err) => err.to_compile_error().into(),
    }
}

#[derive(Debug)]
struct TableDsl {
    name: Ident,
    id_column: Option<Ident>,
    columns: Punctuated<ColumnDsl, Token![,]>,
    use_statements: Vec<syn::ItemUse>,
}

impl Parse for TableDsl {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let mut use_statements = Vec::new();

        while let Some(stmt) = input.parse::<syn::ItemUse>().ok() {
            use_statements.push(stmt)
        }

        let name = input.parse::<Ident>()?;

        let id_column = match try_parse_parens(input) {
            Ok(inside_parens) => {
                Some(inside_parens.parse::<Ident>()?)
            }
            Err(_) => {
                None
            }
        };

        let inside_braces;
        syn::braced!(inside_braces in input);
        let columns = Punctuated::<ColumnDsl, Token![,]>::parse_terminated(&inside_braces)?;

        Ok(TableDsl {
            name,
            id_column,
            columns,
            use_statements,
        })
    }
}

impl ToTokens for TableDsl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let table_name = &self.name;
        let id_column = if let Some(id_column) = &self.id_column {
            quote! { (#id_column) }
        } else {
            quote! {}
        };
        let use_statements = &self.use_statements;

        let columns = sort_punctuated(&self.columns, |column| &column.name);

        tokens.extend(quote! {
            diesel::table! {
                #(#use_statements)*

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
}

impl ToTokens for ColumnDsl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let ty = &self.ty;

        tokens.extend(quote! {
            #name -> #ty,
        })
    }
}

impl Parse for ColumnDsl {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
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

        Ok(ColumnDsl { name, ty })
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
