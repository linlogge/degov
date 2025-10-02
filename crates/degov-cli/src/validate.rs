use std::path::PathBuf;

use degov_dsl::Parser;
use miette::IntoDiagnostic;

pub fn validate_file(path: PathBuf) -> miette::Result<()> {
    let contents = std::fs::read_to_string(path.clone()).into_diagnostic()?;
    let parser = Parser::new(contents, path.to_owned().to_string_lossy().to_string());
    let definition = parser.parse()?;

    println!("Definition: {:?}", definition);

    Ok(())
}
