use diesel_sort_struct_fields::sort_fields;

#[sort_fields]
pub struct Thing(i32, i32);

fn main() {}
