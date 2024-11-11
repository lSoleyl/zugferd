use std::io::Write;
use std::fs;
use std::process::ExitCode;

use pdf::file::FileOptions;
use pdf::object::NameTreeNode::Leaf;
use pdf::object::Resolve;
use pdf::primitive::PdfString;

fn matches_str(pdf_str: &PdfString, str: &str) -> bool {
    pdf_str.to_string().ok().map_or(false, |decoded| decoded == str)
}

fn main() -> ExitCode {
    let pdf_path = "E:\\Users\\David\\Documents\\Rust\\zugferd\\pdfs\\EXTENDED_Kostenrechnung.pdf";
    // let pdf_path = "E:\\Users\\David\\Documents\\Rust\\zugferd\\pdfs\\XRECHNUNG_Betriebskostenabrechnung.pdf";
    let pdf_name = "factur-x.xml";

    let pdf_file = FileOptions::cached().open(pdf_path).unwrap();

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
        Some(vec) => vec.iter().find(|(pdf_str, _)| matches_str(pdf_str, pdf_name)).ok_or((2, String::from("No embedded file with matching name found")))
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
    let result = file_bytes.and_then(|bytes| {
        let open_result = fs::OpenOptions::new().write(true).truncate(true).create(true).open(pdf_name);
        open_result.and_then(|mut file| file.write_all(&*bytes)).map_err(|err| (7, format!("Failed to write {}: {}", pdf_name, err)))
    });
    

    // Finally print out the result
    match result {
        Ok(_) => {
            println!("{} successfully written!", pdf_name);
            ExitCode::from(0)
        },
        Err((code, msg)) => {
            println!("{}", msg);
            ExitCode::from(code)
        }
    }
}
