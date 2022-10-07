use std::io::{BufRead, Write};

use rusty_js_core::Runtime;

fn main() {
    let runtime = Runtime::new();
    runtime.clone().attach();
    
    let mut reader  = std::io::BufReader::new(std::io::stdin());

    loop{
        print!("> ");
        std::io::stdout().flush();
        let mut line = String::new();
        reader.read_line(&mut line);
        let re = runtime.clone().execute("test.js", &line);

        match re{
            Ok(v) => {
                println!("{}", v.to_string());
            },
            Err(e) => {
                println!("{}", e.to_string());
            }
        }
    }
    
}
