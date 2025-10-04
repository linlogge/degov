//! Test the v1 schema with real DSL examples

use degov_dsl::{prelude::*, v1};

#[test]
fn test_v1_schema_root_property() {
    let source = r#"
id "de.berlin/natural-person"

definition {
    kind "DataModel"
}
    "#;

    let parser = Parser::new(source.to_string(), "v1-schema-test.dgv".to_string());
    let schema = v1::create_schema();
    let parser = parser.with_schema(schema);

    let result = parser.parse();
    
    if let Err(ref e) = result {
        eprintln!("Parse error:");
        for diag in &e.diagnostics {
            eprintln!("  - {:?}", diag.kind);
        }
    }
    
    assert!(result.is_ok());
}
