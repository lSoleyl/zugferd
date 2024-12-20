use zugferd::{FileMatcher, Cli, Error};

use std::io::Write;
use std::fs;
use std::process::ExitCode;

use clap::Parser;

use pdf::file::FileOptions;
use pdf::object::Resolve;


fn main() -> ExitCode {
    match nested_main() {
        Err(error) => {
            eprintln!("{}", error.message);
            error.exit_code
        },

        Ok(_) => ExitCode::SUCCESS
    }
}

// Nested main function to be able to use ?-operator for less nested code
fn nested_main() -> Result<(), Error> {
    let cli = Cli::parse();

    // Helper to match the attachment name
    let matcher = FileMatcher::from(&cli.name);
    
    
    let input_path = cli.input_path();
    let output_path = cli.output_path();

    cli.verbose_log(format!("Reading: {}", input_path.display().to_string()));

    
    let pdf_file = FileOptions::cached().open(&input_path).map_err(|err| Error::from(1, format!("Failed to open {}: {}", &input_path.display().to_string(), err)))?;

    
    // Get the /AF array
    let af_filespecs = pdf_file.get_root().associated_files.as_ref().and_then(|af_ref| Some(af_ref.data()));


    let (file_name, file_spec) = match af_filespecs {
        None => Err(Error::from(2, String::from("No /AF Array found!"))),
        Some(vec) => vec.iter()
            .find_map(|file_spec| matcher.matching_name(file_spec).map(|pdf_name| (pdf_name, file_spec)))
            .ok_or(Error::from(3, String::from("No embedded file with matching name found")))
    }?;


    // Extract the /EF entry
    let ef_entry = file_spec.ef.as_ref().ok_or(Error::from(4, format!("Missing /EF in filespec of {}", file_name.to_string_lossy())))?;

    // Extract the /F or /UF reference from the /EF entry
    let file_ref = ef_entry.f.or_else(|| ef_entry.uf).ok_or(Error::from(5, String::from("Missing /F or /UF reference in /EF entry")))?;

    let resolver = pdf_file.resolver();

    // Resolve the ref into a Stream<EmbeddedFile>
    let file_stream = resolver.get(file_ref).map_err(|pdf_err| Error::from(6, format!("Failed to resolve file ref with: {}", pdf_err.to_string())))?;

    // Read the binary file data from the stream
    let file_bytes = (&*file_stream).data(&resolver).map_err(|pdf_err| Error::from(7, format!("Failed to get stream data: {}", pdf_err.to_string())))?;
    
    
    // Write the file
    cli.verbose_log(format!("Writing: {}", output_path.display().to_string()));
    
    let mut file = fs::OpenOptions::new().write(true).truncate(true).create(true).open(&output_path).map_err(|err| Error::from(8,format!("Failed to open {}: {}", &output_path.display().to_string(), err)))?;
    file.write_all(&*file_bytes).map_err(|err| Error::from(9, format!("Failed to write {}: {}", &output_path.display().to_string(), err)))
    
}
