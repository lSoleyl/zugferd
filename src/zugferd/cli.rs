use clap::Parser;

// Command line args
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// PDF input file
    pdf_input: std::path::PathBuf,

    /// Attachment output path (default = pdfInput + ".xml")
    attachment_output: Option<std::path::PathBuf>,

    /// Specifies the name of the attachment to extract (default: "factur-x.xml" or "xrechnung.xml")
    #[arg(short, long)]
    pub name: Option<String>,

    /// Print additional info to the console
    #[arg(short, long, default_value_t=false)]
    verbose: bool,
}

impl Cli {
    pub fn input_path(&self) -> std::path::PathBuf {
        Cli::resolve_path(&self.pdf_input)
    }


    pub fn output_path(&self) -> std::path::PathBuf {
        let path = match self.attachment_output.as_ref() {
            Some(path) => path.clone(),
            None => {
                // Add .xml extension to input path
                let mut output = self.pdf_input.clone();
                output.set_extension("pdf.xml");
                output
            }
        };

        Cli::resolve_path(&path)
    }

    pub fn resolve_path(path: &std::path::PathBuf) -> std::path::PathBuf {
        // Resolve to absolute path if necessary
        if path.is_relative() {
            std::env::current_dir().unwrap().join(path)
        } else {
            path.clone()
        }
    }

    pub fn verbose_log(&self, message: String) {
        if self.verbose {
            println!("{}", message);
        }
    }
}