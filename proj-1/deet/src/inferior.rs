use crate::dwarf_data::DwarfData;
use nix::sys::ptrace;
use nix::sys::ptrace::AddressType;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::mem::size_of;
use std::process::{Child, Command};

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to create and start a new inferior process.
    ///
    /// This function creates a new command to spawn the target program with the provided arguments.
    /// It uses the `pre_exec` hook to call `child_traceme` in the child process so that the operating
    /// system enables debugging (PTRACE_TRACEME) before executing the target program.
    ///
    /// After the process is spawned, it waits for the child to stop with `SIGTRAP` (as occurs after
    /// a PTRACE_TRACEME-enabled process calls exec), confirming that the debugging setup is correct.
    /// If any step fails or the expected signal is not received, the function returns `None`.
    ///
    /// # Parameters
    /// - `target`: A string slice representing the path to the target executable.
    /// - `args`: A vector of strings representing the command-line arguments for the target.
    ///
    /// # Returns
    /// `Some(Inferior)` if the process is successfully spawned and stops with SIGTRAP, or `None` on failure.
    pub fn new(target: &str, args: &Vec<String>, breakpoints: &Vec<usize>) -> Option<Inferior> {
        // Import the Unix-specific process extension for using pre_exec.
        use std::os::unix::process::CommandExt;

        // Build the command with the provided target and arguments.
        let mut cmd = Command::new(target);
        cmd.args(args);

        // Install a pre-exec hook to enable ptrace in the child process.
        // Safety: pre_exec is unsafe because it executes in the child process context.
        unsafe {
            cmd.pre_exec(child_traceme);
        }
        // Spawn the child process.
        let child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Failed to spawn process '{}': {}", target, e);
                return None;
            }
        };
        // Retrieve the PID of the newly spawned child process.
        let pid = Pid::from_raw(child.id() as i32);

        // Wait for the child process to stop due to SIGTRAP, which indicates successful PTRACE_TRACEME.
        match waitpid(pid, None) {
            Ok(WaitStatus::Stopped(_, signal)) if signal == signal::SIGTRAP => {
                let mut res = Inferior { child };
                for bp in breakpoints {
                    res.install_break_points(*bp);
                }
                Some(res)
            }
            Ok(status) => {
                eprintln!("Unexpected wait status: {:?}", status);
                None
            }
            Err(e) => {
                eprintln!("waitpid failed: {}", e);
                None
            }
        }
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    /// Resumes the execution of the inferior process and waits until it stops or terminates.
    ///
    /// This method first uses `ptrace::cont` to continue the process execution (passing `None` for the signal),
    /// and then waits for the process to stop or terminate by calling `self.wait(None)`.
    ///
    /// # Returns
    /// A `Result` containing the `Status` of the process after resuming, or a `nix::Error` if an error occurs.
    pub fn cont(&self) -> Result<Status, nix::Error> {
        // Resume the process execution.
        ptrace::cont(self.pid(), None)?;
        // Wait for the process to change state (stop or exit) and return its status.
        self.wait(None)
    }

    /// Terminates the running inferior process.
    ///
    /// This method uses `Child::kill` to send a kill signal to the inferior process and then reaps
    /// the process to prevent a zombie process.
    ///
    /// # Returns
    /// A `Result` indicating success or the encountered error.
    pub fn kill(&mut self) -> Result<(), std::io::Error> {
        // Send kill signal to the child process.
        self.child.kill()?;
        // Wait for the process to exit, reaping it.
        self.child.wait()?;
        Ok(())
    }

    /// Prints the backtrace of the inferior process using DWARF debugging data.
    ///
    /// This method retrieves and displays the call stack of the inferior process by walking the stack
    /// frames. It uses the instruction pointer (`rip`) and base pointer (`rbp`) from the process's
    /// registers to traverse the stack, looking up source code locations and function names via the
    /// provided `DwarfData`. The backtrace continues until it reaches the `main` function or an error
    /// occurs.
    ///
    /// For each stack frame, it prints the function name, source file, and line number if available.
    /// If the function or line information cannot be resolved, it skips that frame and continues.
    ///
    /// # Parameters
    /// - `self`: A reference to the `Inferior` instance.
    /// - `debug_data`: A reference to `DwarfData` containing debugging information for address lookup.
    ///
    /// # Returns
    /// - `Ok(())` if the backtrace is successfully printed.
    /// - `Err(nix::Error)` if a `ptrace` operation (e.g., reading registers or memory) fails.
    ///
    /// # Panics
    /// This function does not explicitly panic, but underlying `ptrace` calls may panic if the process
    /// is in an invalid state.
    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let mut instruction_ptr = ptrace::getregs(self.pid())?.rip as usize;
        let mut base_ptr = ptrace::getregs(self.pid())?.rbp as usize;
        loop {
            if (self.print_current_frame(instruction_ptr, debug_data)) {
                break;
            }
            instruction_ptr = ptrace::read(self.pid(), (base_ptr + 8) as AddressType)? as usize;
            base_ptr = ptrace::read(self.pid(), base_ptr as AddressType)? as usize;
        }
        Ok(())
    }

    pub fn print_current_frame(&self, instruction_ptr: usize, debug_data: &DwarfData) -> bool {
        let line = debug_data.get_line_from_addr(instruction_ptr);
        let func = debug_data.get_function_from_addr(instruction_ptr);
        if let Some(line) = line {
            if let Some(func) = func {
                println!("{} ({}:{})", func, line.file, line.number);
                if (func == *"main") {
                    return true;
                }
            }
        }
        false
    }

    fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> (8 * byte_offset)) & 0xff;
        let masked_word = word & !(0xff << (8 * byte_offset));
        let updated_word = masked_word | ((val as u64) << (8 * byte_offset));
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }

    pub fn install_break_points(&mut self, addr: usize) -> Result<u8, nix::Error> {
        self.write_byte(addr, 0xcc)
    }
}
