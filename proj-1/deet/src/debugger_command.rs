pub enum DebuggerCommand {
    Quit,
    Run(Vec<String>),
    Continue,
    BackTrace,
    Next,
    BreakPoint(String),
}

fn parse_address(addr: &str) -> Option<usize> {
    let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
        &addr[2..]
    } else {
        &addr
    };
    usize::from_str_radix(addr_without_0x, 16).ok()
}

impl DebuggerCommand {
    pub fn from_tokens(tokens: &Vec<&str>) -> Option<DebuggerCommand> {
        if tokens.is_empty() {
            return None;
        }
        match tokens[0] {
            "q" | "quit" => Some(DebuggerCommand::Quit),
            "r" | "run" => {
                let args = tokens[1..].iter().map(|s| s.to_string()).collect();
                Some(DebuggerCommand::Run(args))
            }
            "c" | "cont" | "continue" => Some(DebuggerCommand::Continue),
            "bt" | "backtrace" => Some(DebuggerCommand::BackTrace),
            "n" | "next" => Some(DebuggerCommand::Next),
            "break" | "b" => {
                if tokens.len() >= 2 {
                    Some(DebuggerCommand::BreakPoint(tokens[1].to_string()))
                } else {
                    println!("No breakpoint target specified");
                    None
                }
            }
            _ => None,
        }
    }
}
