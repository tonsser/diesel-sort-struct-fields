#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::PgConnection;
use diesel_sort_struct_fields::sort_columns;

#[sort_columns]
table! {
    users (id) {
        name -> Nullable<VarChar>,
        id -> Integer,
    }
}

#[derive(Queryable, Debug)]
pub struct User {
    id: i32,
    name: Option<String>,
}

fn loading_users() {
    let db = connect_to_db();
    users::table
        .select(users::all_columns)
        .load::<User>(&db)
        .unwrap();
}

fn connect_to_db() -> PgConnection {
    unimplemented!()
}

fn main() {}
