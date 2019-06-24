# diesel-sort-struct-fields

**NB:** This crate is still experimental. Use with caution.

## The problem

By default [Diesel] maps database responses into Rust tuples and from there into structs. This works well in most cases but it has one very subtle downside:

If you have a schema and model like so:

```rust
table! {
    users (id) {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Queryable)]
struct User {
    id: i32,
    name: String,
}
```

And you execute a query like so:

```rust
users::table.select(users::all_columns).load::<User>(con)?
```

Then Diesel will map that into the Rust type `(i32, String)`. Diesel will then map that tuple into a Rust model by doing something like this:

```rust
let row: (i32, String) = ...;
let user = User { id: row.0, name: row.1 };
```

This works fine, however had you defined you model and schema like this:

```rust
table! {
    users (id) {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Queryable)]
struct User {
    name: String,
    id: i32,
 // ^^^^^^^^ note the order of these fields are flipped
}
```

You would suddenly get this type error when trying to run the query:

```
$ cargo check
    Checking foobar v0.1.0 (/Users/davidpdrsn/Desktop/foobar)
error[E0277]: the trait bound `i32: diesel::deserialize::FromSql<diesel::sql_types::Text, _>` is not satisfied
  --> src/main.rs:24:10
   |
24 |         .load::<User>(&db)
   |          ^^^^ the trait `diesel::deserialize::FromSql<diesel::sql_types::Text, _>` is not implemented for `i32`
   |
   = help: the following implementations were found:
             <i32 as diesel::deserialize::FromSql<diesel::sql_types::Integer, DB>>
   = note: required because of the requirements on the impl of `diesel::Queryable<diesel::sql_types::Text, _>` for `i32`
   = note: required because of the requirements on the impl of `diesel::Queryable<(diesel::sql_types::Integer, diesel::sql_types::Text), _>` for `(std::string::String, i32)`
   = note: required because of the requirements on the impl of `diesel::Queryable<(diesel::sql_types::Integer, diesel::sql_types::Text), _>` for `User`
   = note: required because of the requirements on the impl of `diesel::query_dsl::LoadQuery<_, User>` for `diesel::query_builder::SelectStatement<users::table, diesel::query_builder::select_clause::SelectClause<(users::columns::id, users::columns::name)>>`

error[E0277]: the trait bound `*const str: diesel::deserialize::FromSql<diesel::sql_types::Integer, _>` is not satisfied
  --> src/main.rs:24:10
   |
24 |         .load::<User>(&db)
   |          ^^^^ the trait `diesel::deserialize::FromSql<diesel::sql_types::Integer, _>` is not implemented for `*const str`
   |
   = help: the following implementations were found:
             <*const [u8] as diesel::deserialize::FromSql<diesel::sql_types::Binary, DB>>
             <*const str as diesel::deserialize::FromSql<diesel::sql_types::Text, DB>>
   = note: required because of the requirements on the impl of `diesel::deserialize::FromSql<diesel::sql_types::Integer, _>` for `std::string::String`
   = note: required because of the requirements on the impl of `diesel::Queryable<diesel::sql_types::Integer, _>` for `std::string::String`
   = note: required because of the requirements on the impl of `diesel::Queryable<(diesel::sql_types::Integer, diesel::sql_types::Text), _>` for `(std::string::String, i32)`
   = note: required because of the requirements on the impl of `diesel::Queryable<(diesel::sql_types::Integer, diesel::sql_types::Text), _>` for `User`
   = note: required because of the requirements on the impl of `diesel::query_dsl::LoadQuery<_, User>` for `diesel::query_builder::SelectStatement<users::table, diesel::query_builder::select_clause::SelectClause<(users::columns::id, users::columns::name)>>`

error: aborting due to 2 previous errors
```

This error isn't very helpful, but at least you do get an error. Consider the case where you schema looks like this:

```rust
table! {
    users (id) {
        id -> Integer,
        age -> Integer,
    }
}

#[derive(Queryable)]
struct User {
    age: i32,
    id: i32,
}
```

Now, since both fields are the same type, you wouldn't get errors. Things would just be subtly broken...

## The solution

The solution is to always keep the schema columns and struct fields in the _exact_ same order. One such order could be alphabetically. However then you aren't able to group related fields together unless they have the same prefix.

This crate contains a simple solution. It has two procedural macros that will reorder the fields on `struct`s and `table!` calls so they're always in the same order, regardless of the order they're in in your code.

Example:

```rust
use diesel_sort_struct_fields::{sort_fields, sort_columns};

#[sort_columns]
table! {
    users (id) {
        id -> Integer,
        name -> VarChar,
    }
}

#[sort_fields]
#[derive(Queryable)]
pub struct User {
    name: String,
    id: i32,
}
```

[Diesel]: (https://diesel.rs)
