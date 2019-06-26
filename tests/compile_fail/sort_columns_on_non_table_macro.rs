#![allow(unused_imports, unused_macros)]

#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel_sort_struct_fields::sort_columns;

macro_rules! foo {
    () => {}
}

#[sort_columns]
foo! {}

fn main() {}
