use std::path::PathBuf;

use degov_dgl::Parser;
use miette::IntoDiagnostic;

pub fn validate_file(path: PathBuf) -> miette::Result<()> {
    let contents = std::fs::read_to_string(path.clone()).into_diagnostic()?;
    let parser = Parser::new(contents, path.to_owned().to_string_lossy().to_string());
    let parser = parser.with_schema(degov_dgl::v1::create_schema());

    let _definition = parser.parse()?;

    Ok(())
}
