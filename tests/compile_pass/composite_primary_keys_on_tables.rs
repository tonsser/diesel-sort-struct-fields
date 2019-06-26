#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel_sort_struct_fields::sort_columns;

#[sort_columns]
diesel::table! {
    use diesel::sql_types::*;

    users (id, name) {
        name -> Text,
        id -> BigSerial,
    }
}

#[derive(Queryable)]
pub struct User {
    id: i64,
    name: String,
}

fn loading_users() {
    let db = connect_to_db();
    users::table
        .select(users::all_columns)
        .load::<User>(&db)
        .unwrap_or_else(|_| panic!("whoops"));
}

fn connect_to_db() -> PgConnection {
    unimplemented!()
}

fn main() {}
