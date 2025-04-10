use crate::debugger_command::DebuggerCommand;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::{Inferior, Status};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::num::ParseIntError;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    breakpoints: Vec<usize>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!(
                    "Could not extract debugging symbols from {}: {:?}",
                    target, err
                );
                std::process::exit(1);
            }
        };
        debug_data.print();

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
            breakpoints: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    // If an inferior is already running, kill it before starting a new one.
                    if let Some(ref mut inferior) = self.inferior {
                        println!("Killing running inferior (pid {})", inferior.pid());
                        if let Err(e) = inferior.kill() {
                            println!("Failed to kill inferior: {}", e);
                        }
                    }
                    // Attempt to start a new inferior process.
                    if let Some(inferior) = Inferior::new(&self.target, &args, &self.breakpoints) {
                        self.inferior = Some(inferior);
                        // Continue execution until it stops or terminates.
                        let status = self
                            .inferior
                            .as_mut()
                            .unwrap()
                            .cont()
                            .expect("Error continuing inferior");
                        if let Status::Stopped(_, pointer) = status {
                            self.inferior
                                .as_mut()
                                .unwrap()
                                .print_current_frame(pointer, &self.debug_data);
                        }
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Continue => {
                    // If no inferior is running, print an error message.
                    if let Some(inferior) = self.inferior.as_mut() {
                        inferior.cont().expect("Error continuing inferior");
                    } else {
                        println!("No inferior to continue");
                    }
                }
                DebuggerCommand::Quit => {
                    // On quitting, kill any running inferior.
                    if let Some(ref mut inferior) = self.inferior {
                        println!("Killing running inferior (pid {})", inferior.pid());
                        if let Err(e) = inferior.kill() {
                            println!("Failed to kill inferior: {}", e);
                        }
                    }
                    return;
                }
                DebuggerCommand::BackTrace => {
                    if let Some(inferior) = self.inferior.as_mut() {
                        inferior
                            .print_backtrace(&self.debug_data)
                            .expect("Error printing backtrace");
                    }
                }
                DebuggerCommand::BreakPoint(target) => {
                    // Convert the target string to an address.
                    let bp_addr_opt = if target.starts_with('*') {
                        // Raw address: remove the '*' and parse as hexadecimal.
                        let addr_str = target.trim_start_matches('*');
                        // Allow both "0x" prefixed and plain hexadecimal.
                        usize::from_str_radix(addr_str.trim_start_matches("0x"), 16)
                            .map_err(|e: ParseIntError| {
                                println!("Invalid raw address '{}': {}", addr_str, e);
                                e
                            })
                            .ok()
                    } else if let Ok(line) = target.parse::<usize>() {
                        // Treat as a source line number.
                        self.debug_data.get_addr_for_line(None, line).or_else(|| {
                            println!("No source information for line {}", line);
                            None
                        })
                    } else {
                        // Treat as a function name.
                        self.debug_data
                            .get_addr_for_function(None, target.as_str())
                            .or_else(|| {
                                println!("No function named '{}' found", target);
                                None
                            })
                    };

                    if let Some(addr) = bp_addr_opt {
                        println!("Set breakpoint {} at {:#x}", self.breakpoints.len(), addr);
                        if let Some(inferior) = self.inferior.as_mut() {
                            if let Err(e) = inferior.install_break_points(addr) {
                                println!("Failed to install breakpoint: {}", e);
                            }
                        } else {
                            self.breakpoints.push(addr);
                        }
                    }
                }
                DebuggerCommand::Next => {
                    if let Some(inferior) = self.inferior.as_mut() {
                        let status = inferior
                            .next_line(&self.debug_data)
                            .expect("Error executing next command");
                        if let Status::Stopped(_, pointer) = status {
                            inferior.print_current_frame(pointer, &self.debug_data);
                        }
                    } else {
                        println!("No inferior to step");
                    }
                }
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input.
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it.
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes.
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }
}
