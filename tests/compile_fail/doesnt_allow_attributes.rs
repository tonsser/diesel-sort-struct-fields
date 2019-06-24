use diesel_sort_struct_fields::sort_fields;

#[sort_fields(foo)]
pub struct A {
    a: i32,
    b: i32,
}

fn main() {}
