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
#[derive(Queryable, Debug)]
pub struct User {
    name: String,
    id: i32,
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
