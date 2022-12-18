use aida64_keys_lib::{KeyEdition, License};
use strum::IntoEnumIterator;

fn main() {
    for edition in KeyEdition::iter() {
        println!("{:?} -> {edition}", License::new(edition).generate_string(true));
    }
}
