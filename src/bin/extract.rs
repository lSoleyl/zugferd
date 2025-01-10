use pdf::primitive::PdfString;
use zugferd::{FileMatcher, Error};

use std::io::Write;
use std::fs;
use std::process::ExitCode;

use regex::Regex;

use clap::Parser;

use pdf::file::FileOptions;
use pdf::object::{FileSpec, Resolve};


type PDFFile = pdf::file::File<Vec<u8>, std::sync::Arc<pdf::file::SyncCache<pdf::object::PlainRef, Result<pdf::any::AnySync, std::sync::Arc<pdf::PdfError>>>>, std::sync::Arc<pdf::file::SyncCache<pdf::object::PlainRef, Result<std::sync::Arc<[u8]>, std::sync::Arc<pdf::PdfError>>>>, pdf::file::NoLog>;


fn main() -> ExitCode {
    let cli = Extract::parse();

    match cli.extract() {
        Err(error) => {
            error.print();
            error.exit_code
        },

        Ok(_) => ExitCode::SUCCESS
    }
}



// Error codes:
//  1-9 : Basic File IO Errors
// 10-19: /Metadata problem
// 20-29: /AF Array problem
// 30-39: /EmbeddedFiles problem
// 40-49: Error while extracting file content


// Command line args
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Extract {
    /// PDF input file
    pdf_input: std::path::PathBuf,

    /// Attachment output path (default = pdfInput + ".xml")
    attachment_output: Option<std::path::PathBuf>,

    /// Specifies the name of the attachment to extract (default: derived from Metadata or "factur-x.xml" or "xrechnung.xml")
    #[arg(short, long)]
    pub name: Option<String>,

    /// Print additional info to the console
    #[arg(short, long, default_value_t=false)]
    verbose: bool,

    /// Exit with an error if the file is not a valid e-invoice.
    /// If not set the tool will try to extract any .xml file somehow
    #[arg(short, long, default_value_t=false)]
    strict: bool,
}

impl Extract {
    fn input_path(&self) -> std::path::PathBuf {
        Self::resolve_path(&self.pdf_input)
    }


    fn output_path(&self) -> std::path::PathBuf {
        let path = match self.attachment_output.as_ref() {
            Some(path) => path.clone(),
            None => {
                // Add .xml extension to input path
                let mut output = self.pdf_input.clone();
                output.set_extension("pdf.xml");
                output
            }
        };

        Self::resolve_path(&path)
    }

    fn resolve_path(path: &std::path::PathBuf) -> std::path::PathBuf {
        // Resolve to absolute path if necessary
        if path.is_relative() {
            std::env::current_dir().unwrap().join(path)
        } else {
            path.clone()
        }
    }

    fn verbose_log(&self, message: String) {
        if self.verbose {
            println!("{}", message);
        }
    }

    /// Retrieves the content of the /Metadata stream as string if available and parsable as UTF-8
    fn get_metadata(&self, pdf_file: &PDFFile) -> Result<String, Error> {
        let metadata = pdf_file.get_root().metadata.ok_or(Error::from(10, String::from("No /Metadata found!")))?;
        let resolver = pdf_file.resolver();

        let file_stream = resolver.get(metadata).map_err(|pdf_err| Error::from(11, format!("Failed to resolve /Metadata stream ref with: {}", pdf_err.to_string())))?;
        let metadata_bytes = (&*file_stream).data(&resolver).map_err(|pdf_err| Error::from(12, format!("Failed to get /Metadata stream data: {}", pdf_err.to_string())))?;

        String::from_utf8(metadata_bytes.to_vec()).map_err(|utf8_err| Error::from(13, format!("Failed to decode /Metadata stream a valid utf8 string: {}", utf8_err)))
    }

    fn get_xml_filematcher_from_metadata(&self, content_string: &String) -> Result<FileMatcher, Error> {
        // We could also add an XML parser, but regex chould be the easier solution at the moment
        // First we find the matching <rdf:Description> tag, which can either be an immediately closed tag with attributes or
        // an open tag with sub elements
        let description_regex = Regex::new(r#"(?ms)<rdf:Description [^>]*xmlns:fx="urn:factur-x:pdfa:CrossIndustryDocument:invoice[^/>]+(/>|>.*?</rdf:Description>)"#).unwrap();
        let filename_regex = Regex::new(r#"fx:DocumentFileName((>(?<name1>.*?)</fx:DocumentFileName>)|(="(?<name2>.*?)"))"#).unwrap();
        
        let description_match = description_regex.find(content_string).ok_or(Error::from(14, String::from("Missing <rdf:Description> element in /Metadata stream")))?;
        let filename_match = filename_regex.captures(description_match.as_str()).ok_or(Error::from(15, String::from("Failed to locate fx:DocumentFileName in /Metadata stream")))?;
        
        let name = filename_match.name("name1").filter(|m| m.len() > 0).or_else(|| filename_match.name("name2").filter(|m| m.len() > 0)).unwrap().as_str();
        self.verbose_log(format!("/Metadata contains following XML file name to look for: '{}'", name));
        Ok(FileMatcher::from_name(name))
    }

    /// Returns the file matcher to use for this PDF file, which is either the
    /// passed name or if no name is passed the one from the metadata XML.
    /// If no metadata is set and we are not in strict mode it will fallback to factor-x.xml/xrechnung.xml
    fn get_matcher(&self, pdf_file: &PDFFile) -> Result<FileMatcher, Error> {
        if let Some(name) = self.name.as_ref() {
            return Ok(FileMatcher::from_name(name));
        }

        let matcher = self.get_metadata(pdf_file).and_then(|ref metadata| self.get_xml_filematcher_from_metadata(metadata));

        if self.strict {
            matcher
        } else {
            // fall back to default names
            matcher.or_else(|error| {
                // still print the error
                error.print();
                self.verbose_log(String::from("Searching for default XML files instead (factur-x.xml or xrechnung.xml)"));
                Ok(FileMatcher::from_default())
            })
        }
    }
    
    /// Returns the matching filespec for the given file matcher from the /AF array
    fn get_af_file_spec(&self, pdf_file: &PDFFile, matcher: &FileMatcher) -> Result<(PdfString, FileSpec), Error> {
        let af_filespecs = pdf_file.get_root().associated_files.as_ref().and_then(|af_ref| Some(af_ref.data())).ok_or(Error::from(2, String::from("No /AF Array found!")))?;

        let matching_filespec = af_filespecs.iter()
            .find_map(|file_spec| matcher.matching_name(file_spec).map(|pdf_name| (pdf_name.clone(), file_spec.clone())))
            .ok_or(Error::from(20, format!("No embedded file matching {} found in /AF array", matcher)));

        if self.strict {
            // No second chances in strict mode! AF must exist and the name must match!
            return matching_filespec;
        }   

        matching_filespec.or_else(|error| {
            error.print();
            self.verbose_log(String::from("Trying to extract any .xml file from /AF array"));
            af_filespecs.iter()
                .find_map(|file_spec| FileMatcher::matching_suffix(file_spec, ".xml").map(|pdf_name| (pdf_name.clone(), file_spec.clone())))
                .ok_or(Error::from(21, format!("No embedded .xml file found in /AF array")))
        })
    }

    /// Used as a fallback in non-strict mode in case no /AF array exists. In that case we search all filespecs of the /EmbeddedFiles structure
    fn get_ef_file_spec(&self, pdf_file: &PDFFile, matcher: &FileMatcher) -> Result<(PdfString, FileSpec), Error> {
        let names = pdf_file.trailer.root.names.as_ref().ok_or(Error::from(31, String::from("names dictionary not found while looking for /EmbeddedFiles")))?;
        let embedded_files = names.data().embedded_files.as_ref().ok_or(Error::from(32, String::from("No /EmbeddedFiles found")))?;
        let resolver = pdf_file.resolver();
        let mut matched_spec: Option<(PdfString, FileSpec)> = None;
        
        embedded_files.walk(&resolver, &mut |_pdf_str, file_spec| {
            if matched_spec.is_none() {
                if let Some(matching_name) = matcher.matching_name(file_spec) {
                    matched_spec = Some((matching_name.clone(), file_spec.clone()));
                }
            }
        }).map_err(|pdf_err| Error::from(33, format!("Iteration over /EmbeddedFiles failed with: {}", pdf_err)))?;

        let result = matched_spec.ok_or(Error::from(34, format!("No embedded file matching {} found in /EmbeddedFiles structure", matcher)));
        if self.strict {
            return result;
        }

        result.or_else(|error| {
            error.print();
            self.verbose_log(String::from("Trying to extract any .xml file from /EmbeddedFiles structure"));

            let mut matched_spec: Option<(PdfString, FileSpec)> = None;
            embedded_files.walk(&resolver, &mut |_pdf_str, file_spec| {
                if matched_spec.is_none() {
                    if let Some(matching_name) = FileMatcher::matching_suffix(file_spec, ".xml") {
                        matched_spec = Some((matching_name.clone(), file_spec.clone()));
                    }
                }
            }).map_err(|pdf_err| Error::from(35, format!("Iteration over /EmbeddedFiles failed with: {}", pdf_err)))?;

            matched_spec.ok_or(Error::from(36, String::from("No embedded .xml files found in /EmbeddedFiles structure")))
        })
    }




    /// The extract main function
    fn extract(&self) -> Result<(), Error> {
        let input_path = self.input_path();
        let output_path = self.output_path();
    
        self.verbose_log(format!("Reading: {}", input_path.display().to_string()));
    
        
        let pdf_file: PDFFile = FileOptions::cached().open(&input_path).map_err(|err| Error::from(1, format!("Failed to open {}: {}", &input_path.display().to_string(), err)))?;
    
        
        // Helper to match the attachment name
        let matcher = self.get_matcher(&pdf_file)?;

        
        // Get the matched filename and its filespec from the /AF array (with /EmbeddedFiles as fallback)
        let (file_name, file_spec) = self.get_af_file_spec(&pdf_file, &matcher).or_else(|error| {
            if self.strict {
                Err(error)
            } else {
                error.print();
                self.verbose_log(String::from("Retrying in /EmbeddedFiles"));
                self.get_ef_file_spec(&pdf_file, &matcher)
            }
        })?;

        self.verbose_log(format!("Found {:?}", file_name));
    
        // Extract the /EF entry
        let ef_entry = file_spec.ef.as_ref().ok_or(Error::from(40, format!("Missing /EF in filespec of {}", file_name.to_string_lossy())))?;
    
        // Extract the /F or /UF reference from the /EF entry
        let file_ref = ef_entry.f.or_else(|| ef_entry.uf).ok_or(Error::from(41, String::from("Missing /F or /UF reference in /EF entry")))?;
    
        let resolver = pdf_file.resolver();
    
        // Resolve the ref into a Stream<EmbeddedFile>
        let file_stream = resolver.get(file_ref).map_err(|pdf_err| Error::from(42, format!("Failed to resolve file ref with: {}", pdf_err.to_string())))?;
    
        // Read the binary file data from the stream
        let file_bytes = (&*file_stream).data(&resolver).map_err(|pdf_err| Error::from(43, format!("Failed to get stream data: {}", pdf_err.to_string())))?;
        
        
        // Write the file
        self.verbose_log(format!("Writing: {}", output_path.display().to_string()));
        
        let mut file = fs::OpenOptions::new().write(true).truncate(true).create(true).open(&output_path).map_err(|err| Error::from(2,format!("Failed to open {}: {}", &output_path.display().to_string(), err)))?;
        file.write_all(&*file_bytes).map_err(|err| Error::from(3, format!("Failed to write {}: {}", &output_path.display().to_string(), err)))
    }


}

