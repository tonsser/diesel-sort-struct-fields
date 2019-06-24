use diesel_sort_struct_fields::sort_fields;

#[sort_fields]
pub enum Thing {
    A,
    B,
}

fn main() {}
