mod filematcher;

use std::io::Write;
use std::fs;
use std::process::ExitCode;

use clap::Parser;

use pdf::file::FileOptions;
use pdf::object::NameTreeNode::Leaf;
use pdf::object::Resolve;

use filematcher::FileMatcher;


// Command line args
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// PDF input file
    pdf_input: std::path::PathBuf,

    /// Attachment output path (default = pdfInput + ".xml")
    attachment_output: Option<std::path::PathBuf>,

    /// Specifies the name of the attachment to extract (default: "factur-x.xml" or "xrechnung.xml")
    #[arg(short, long)]
    name: Option<String>,

    /// Display pdf structure information
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

    pub fn verboseLog(&self, message: String) {
        if self.verbose {
            println!("{}", message);
        }
    }
}




//TODO: Update dependency once pdf-rs merges my pull-request
//TODO: tidy up code

fn main() -> ExitCode {
    let cli = Cli::parse();

    //TODO: process output path
    //TODO: support verbose flag

    // Helper to match the attachment name
    let matcher = FileMatcher::from(&cli.name);
    
    
    let input_path = cli.input_path();
    let output_path = cli.output_path();

    cli.verboseLog(format!("Reading: {}", input_path.display().to_string()));
    
    //TODO: handle error result properly
    let pdf_file = FileOptions::cached().open(&input_path).unwrap();


    //TODO: Write remaining verbose logs
    //TODO: maybe exit early in case of error dragging the Results around makes handling a bit uncomfortable

    let embedded_files = pdf_file.get_root().names.as_ref().and_then(|dict_ref| dict_ref.embedded_files.as_ref());
    println!("{:?}", embedded_files);


    let embedded_file_vec = embedded_files.and_then(|tree| match &tree.node {
        Leaf(file_specs) => Some(file_specs),
        _ => None
    });

    println!("{:?}", embedded_file_vec);


    // Get the first embedded file with matching file name
    let first_embedded_file_filter_result = match embedded_file_vec {
        None => Err((1, String::from("No embedded files found!"))),
        Some(vec) => vec.iter().find(|(pdf_str, _)| matcher.matches(pdf_str)).ok_or((2, String::from("No embedded file with matching name found")))
    };
    println!("{:?}", first_embedded_file_filter_result);


    // Extract the /EF entry
    let ef_entry_result = first_embedded_file_filter_result.and_then(|(file_name, file_spec)| file_spec.ef.as_ref().ok_or((3, format!("Missing /EF in filespec of {}", file_name.to_string_lossy()))));
    println!("{:?}", ef_entry_result);

    // Extract the /F or /UF reference from the /EF entry
    let file_ref_result = ef_entry_result.and_then(|ef| ef.f.or_else(|| ef.uf).ok_or((4, String::from("Missing /F or /UF reference in /EF entry"))));
    println!("{:?}", file_ref_result);

    let resolver = pdf_file.resolver();

    // Resolve the ref into a Stream<EmbeddedFile>
    let resolved_ref_result = file_ref_result.and_then(|file_ref| resolver.get(file_ref).map_err(|pdf_err| (5, format!("Failed to resolve file ref with: {}", pdf_err.to_string()))));
    println!("{:?}", resolved_ref_result);

    // Read the binary file data from the stream
    let file_bytes = resolved_ref_result.and_then(|stream| {
        (&*stream).data(&resolver).map_err(|pdf_err| (6, format!("Failed to get stream data: {}", pdf_err.to_string())))
    });
    
    
    // Write the file
    cli.verboseLog(format!("Writing: {}", output_path.display().to_string()));
    let result = file_bytes.and_then(|bytes| {
        let open_result = fs::OpenOptions::new().write(true).truncate(true).create(true).open(&output_path);
        open_result.and_then(|mut file| file.write_all(&*bytes)).map_err(|err| (7, format!("Failed to write {}: {}", &output_path.display().to_string(), err)))
    });
    

    // Finally print out the result
    match result {
        Ok(_) => {
            ExitCode::from(0)
        },
        Err((code, msg)) => {
            eprintln!("{}", msg);
            ExitCode::from(code)
        }
    }
}
