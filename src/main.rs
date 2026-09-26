mod archs;
mod bytestream;
mod common;
mod driver;
mod preprocess;

use driver::{BuildContext, Driver};

fn main() {
    let source =
        std::fs::read_to_string(std::env::args().nth(1).unwrap_or("prog.src".into())).unwrap();

    Driver::new(BuildContext {
        source,
        exec_path: "out".into(),
    })
    .preprocess()
    .process()
    .write();
}
