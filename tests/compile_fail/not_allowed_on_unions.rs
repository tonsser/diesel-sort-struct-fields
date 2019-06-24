use diesel_sort_struct_fields::sort_fields;

#[sort_fields]
pub union Thing {
    a: u32,
}

fn main() {}
