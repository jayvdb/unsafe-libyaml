/// Example demonstrating the use of owned input with libyaml-safer Parser.
///
/// This example shows how to use the new `set_input_owned()` and
/// `set_input_string_owned()` methods which eliminate the need to maintain
/// input buffers with specific lifetimes.
use libyaml_safer::{Document, Parser};

fn main() {
    // Example 1: Using owned Vec<u8>
    println!("Example 1: Parsing with owned Vec<u8>");
    let yaml_data = b"name: Alice\nage: 30\ncity: Boston".to_vec();
    let mut parser = Parser::new();
    parser.set_input_owned(yaml_data);

    let doc = Document::load(&mut parser).expect("Failed to parse YAML");
    println!("Parsed {} nodes", doc.nodes.len());

    // Example 2: Using owned String
    println!("\nExample 2: Parsing with owned String");
    let yaml_string = String::from(
        r#"
users:
  - name: Bob
    role: admin
  - name: Charlie
    role: user
"#,
    );

    let mut parser = Parser::new();
    parser.set_input_string_owned(yaml_string);

    let doc = Document::load(&mut parser).expect("Failed to parse YAML");
    println!("Parsed {} nodes", doc.nodes.len());

    // Example 3: Function that returns a parser with owned data
    println!("\nExample 3: Function returning Parser with owned data");
    let mut parser = create_parser_with_config();
    let doc = Document::load(&mut parser).expect("Failed to parse YAML");
    println!(
        "Parsed {} nodes from function-created parser",
        doc.nodes.len()
    );

    // Example 4: Parsing data from a function without lifetime constraints
    println!("\nExample 4: Parsing dynamically generated YAML");
    let yaml = generate_dynamic_yaml("production", 42);
    let mut parser = Parser::new();
    parser.set_input_string_owned(yaml);

    let doc = Document::load(&mut parser).expect("Failed to parse YAML");
    println!("Parsed dynamic YAML with {} nodes", doc.nodes.len());
}

/// Creates a parser with configuration data without lifetime constraints.
/// Before owned input support, this pattern was difficult to implement.
fn create_parser_with_config() -> Parser<'static> {
    let config = String::from(
        r#"
database:
  host: localhost
  port: 5432
  name: mydb
"#,
    );

    let mut parser = Parser::new();
    parser.set_input_string_owned(config);
    parser
}

/// Generates YAML content dynamically.
/// The owned input API makes it easy to parse this generated content.
fn generate_dynamic_yaml(env: &str, worker_count: u32) -> String {
    format!(
        r#"
environment: {}
settings:
  workers: {}
  timeout: 30
  retry: true
"#,
        env, worker_count
    )
}
