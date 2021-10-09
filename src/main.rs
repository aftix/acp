use acp::apkg;

use std::path::Path;

fn main() {
    let path = Path::new("./test.apkg");

    let apkg = apkg::Apkg::new(&path).unwrap();
}
