use degov_dsl::Parser;
use std::env;

fn main() -> miette::Result<()> {
    // Configure miette for beautiful error output
    miette::set_hook(Box::new(|_| {
        Box::new(miette::MietteHandlerOpts::new()
            .terminal_links(true)
            .unicode(true)
            .context_lines(3)
            .tab_width(4)
            .build())
    }))?;

    // Get filename from command line or use default
    let filename = env::args()
        .nth(1)
        .unwrap_or_else(|| "crates/degov-dsl/examples/test_valid.dgv".to_string());

    println!("Parsing file: {}\n", filename);

    // Read and parse the file
    let source = std::fs::read_to_string(&filename)
        .map_err(|e| miette::miette!("Failed to read file '{}': {}", filename, e))?;

    let parser = Parser::new(source, filename.clone());
    
    match parser.parse() {
        Ok(definition) => {
            println!("✅ Success! Parsed {} file", filename);
            println!("   Definition type: {}", definition.r#type);
        }
        Err(e) => {
            // The error will be beautifully displayed by miette
            return Err(e.into());
        }
    }

    Ok(())
}

