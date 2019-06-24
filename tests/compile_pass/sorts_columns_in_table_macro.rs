#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::PgConnection;
use diesel_sort_struct_fields::sort_columns;

#[sort_columns]
table! {
    users (id) {
        name -> VarChar,
        id -> Integer,
    }
}

#[derive(Queryable, Debug)]
pub struct User {
    id: i32,
    name: String,
}

fn loading_users() {
    let db = connect_to_db();
    let users: Vec<User> = users::table
        .select(users::all_columns)
        .load::<User>(&db)
        .unwrap();
    dbg!(users);
}

fn connect_to_db() -> PgConnection {
    unimplemented!()
}

fn main() {}
