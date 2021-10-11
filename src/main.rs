use acp::apkg;

use std::path::PathBuf;

use argparse::{ArgumentParser, Store, StoreTrue};

// Options for the program
#[derive(Debug, Clone)]
struct Options {
    verbose: bool,
    infile: Option<PathBuf>,
    outfile: Option<PathBuf>,
}

impl Options {
    pub fn new() -> Self {
        Options {
            verbose: false,
            infile: Option::<PathBuf>::None,
            outfile: Option::<PathBuf>::None,
        }
    }
}

fn main() {
    let mut options = Options::new();

    // Parse arguments
    {
        let mut infile = String::new(); // Get input file as String then change to PathBuf
        let mut outfile = String::new(); // Get output file as String then change to PathBuf
        {
            let mut ap = ArgumentParser::new();
            ap.set_description("Parse and edit Anki2 apkg files");
            ap.refer(&mut options.verbose).add_option(
                &["-v", "--verbose"],
                StoreTrue,
                "Be verbose",
            );
            ap.refer(&mut infile)
                .add_option(&["-i", "--input"], Store, "Input apkg file");
            ap.refer(&mut outfile)
                .add_option(&["-o", "--output"], Store, "Output apkg file");
            ap.parse_args_or_exit();
        }

        if infile != "" {
            if infile == outfile {
                panic!("Output file can not be the same as the input file!");
            }
            options.infile = Some(PathBuf::new().join(&infile));
        }

        if outfile != "" {
            options.outfile = Some(PathBuf::new().join(&outfile));
        }
    }

    println!("{:?}", options);

    let apkg = apkg::Apkg::new(&options.infile.expect("No apkg specified!")).unwrap();
}
