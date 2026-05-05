use std::env;

fn main() {
    let command = env::args().nth(1).unwrap_or_else(|| "help".to_string());
    match command.as_str() {
        "step" | "back" | "goto" | "inspect" | "break" | "list-breakpoints" => {
            println!(
                "kairo-ecs-debug {command}: trace file support is provided by the library scaffold"
            );
        }
        _ => {
            println!("usage: kairo-ecs-debug <step|back|goto|inspect|break|list-breakpoints>");
        }
    }
}
