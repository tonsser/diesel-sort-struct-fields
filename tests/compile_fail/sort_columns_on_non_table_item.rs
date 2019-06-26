#![allow(unused_imports)]

#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel_sort_struct_fields::sort_columns;

#[sort_columns]
fn main() {}
