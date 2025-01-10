use std::process::ExitCode;

/// zugferd error containing the error text and the exit code
pub struct Error {
    pub message: String,

    pub exit_code: ExitCode
}

impl Error {
    pub fn from(code: u8, error: String) -> Error {
        Error {
            message: error,
            exit_code: ExitCode::from(code)
        }
    }

    pub fn print(&self) {
        eprintln!("{}", self.message);
    }
}


