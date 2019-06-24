#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::PgConnection;
use diesel_sort_struct_fields::sort_fields;

table! {
    users (id) {
        id -> Integer,
        name -> VarChar,
    }
}

#[sort_fields]
#[derive(Queryable)]
pub struct User {
    id: i32,
    name: String,
}

fn loading_users() {
    let db = connect_to_db();
    let _: Vec<User> = users::table
        .select(users::all_columns)
        .load::<User>(&db)
        .unwrap();
}

fn connect_to_db() -> PgConnection {
    unimplemented!()
}

fn main() {}
