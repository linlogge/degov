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

    println!("=== Error Demonstration ===\n");

    // Example 1: Missing required node
    println!("1. Missing 'definition' node:");
    let empty_doc = "";
    let parser = Parser::new(empty_doc.to_string(), "empty.dgv".to_string());
    if let Err(e) = parser.parse() {
        println!("{:?}", miette::Report::new(e));
    }
    
    println!("\n{}\n", "=".repeat(60));

    // Example 2: Missing required child
    println!("2. Missing 'type' child:");
    let missing_child = r#"definition {
    // missing type child
}
"#;
    let parser = Parser::new(missing_child.to_string(), "missing_child.dgv".to_string());
    if let Err(e) = parser.parse() {
        println!("{:?}", miette::Report::new(e));
    }
    
    println!("\n{}\n", "=".repeat(60));

    // Example 3: Type mismatch
    println!("3. Type mismatch (expected string):");
    let type_mismatch = r#"definition {
    type 123
}
"#;
    let parser = Parser::new(type_mismatch.to_string(), "type_mismatch.dgv".to_string());
    if let Err(e) = parser.parse() {
        println!("{:?}", miette::Report::new(e));
    }
    
    println!("\n{}\n", "=".repeat(60));

    // Example 4: Valid parse
    println!("4. Valid document:");
    let valid = r#"definition {
    type "DataModel"
    schema {
        id {
            type "string"
            required #true
        }
    }
}
"#;
    let parser = Parser::new(valid.to_string(), "valid.dgv".to_string());
    match parser.parse() {
        Ok(definition) => {
            println!("✓ Parsed successfully!");
            println!("  Definition type: {}", definition.r#type);
        }
        Err(e) => {
            println!("{:?}", miette::Report::new(e));
        }
    }

    Ok(())
}

