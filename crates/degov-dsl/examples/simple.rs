use degov_dsl::Parser;

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

    // Read the DSL file
    let doc = std::fs::read_to_string("crates/degov-dsl/examples/simple.dgv")
        .expect("Failed to read file");

    // Create parser with source name for better error messages
    let parser = Parser::new(doc, "simple.dgv".to_string());

    // Parse the document - errors will show with code context
    let definition = parser.parse()?;

    println!("✓ Parsed successfully!");
    println!("Definition type: {}", definition.r#type);

    Ok(())
}
