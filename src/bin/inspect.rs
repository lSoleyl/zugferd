
use std::{fs::OpenOptions, io::Write, path::Path, process::ExitCode, sync::Arc};
use pdf::{any::AnySync, file::{NoLog, Storage, StorageResolver, SyncCache}, object::{EmbeddedFile, ParseOptions, PlainRef, Resolve, Stream}, primitive::{Dictionary, Primitive}};
use zugferd::Error;
use clap::Parser;



// Command line args
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// PDF input file
    pdf_input: std::path::PathBuf,

    /// resolve the ref with the given id (separate by comma if passing multiple)
    #[arg(short='r', long="ref")]
    references: Option<String>,

    /// print the content of the stream(s) behind the given ref(s) to the console
    #[arg(short, long)]
    print: Option<String>,

    /// export the raw stream behind the given ref(s) and write it to a new file (next to the input PDF)
    #[arg(short, long)]
    export: Option<String>,
}


impl Args {
    fn input_path(&self) -> std::path::PathBuf {
        Self::resolve_path(&self.pdf_input)
    }

    fn resolve_path(path: &std::path::PathBuf) -> std::path::PathBuf {
        // Resolve to absolute path if necessary
        if path.is_relative() {
            std::env::current_dir().unwrap().join(path)
        } else {
            path.clone()
        }
    }
}


/// This struct will hold the most important data structures in one place and provides methods
/// for easier navigation of the data structure.
struct Inspector<'a> {
    storage: Storage<Vec<u8>, Arc<SyncCache<PlainRef, Result<AnySync, Arc<pdf::PdfError>>>>, Arc<SyncCache<PlainRef, Result<Arc<[u8]>, Arc<pdf::PdfError>>>>, NoLog>,
    trailer_dict: Dictionary,
    resolver: Option<StorageResolver<'a, Vec<u8>, Arc<SyncCache<PlainRef, Result<AnySync, Arc<pdf::PdfError>>>>, Arc<SyncCache<PlainRef, Result<Arc<[u8]>, Arc<pdf::PdfError>>>>, NoLog>>
}


impl<'a> Inspector<'a> {
    /// The (incomplete) constructor. Because of the borrow checker, we cannot initialize the resolver inside the constructor as there is no way 
    /// to initialize the resolver in a way to reference the storage field inside the struct itself.
    fn new(path: &Path) -> Result<Self, Error> {
        let backend_data = std::fs::read(&path).map_err(|err| Error::from(1, format!("Failed to open {:?} with: {}", path, err)))?;
        let mut storage = Storage::with_cache(backend_data, ParseOptions::strict(), SyncCache::new(), SyncCache::new(), NoLog)
            .map_err(|err| Error::from(2, format!("Failed parse {:?} with: {}", path, err)))?;

        let dict = storage.load_storage_and_trailer_password(b"").map_err(|err| Error::from(3, format!("Failed to load trailer dictionary with: {}", err)))?;

        Ok(Inspector {
            storage: storage,
            trailer_dict: dict,
            resolver: None,
        })
    }

    /// 2. part of constructor - must be called before calling any other methods of this class
    /// sad that there seems to be no better way of handling this
    fn with_resolver(&'a mut self) -> &'a Self {
        // FIXME: The StorageResolver::new() call is the only reason, why we must use a modified version of pdf-rs...
        self.resolver = Some(StorageResolver::new(&self.storage));
        self
    }


    /// Resolve reference to primitive if it is a reference, otherwise simply return a copy to it
    fn resolve_if_ref(&self, primitive: &Primitive) -> Result<Primitive, Error> {
        match primitive {
            Primitive::Reference(reference) => self.resolve(reference),
            _ => Ok(primitive.clone())
        }
    }

    fn resolve(&self, plain_ref: &PlainRef) -> Result<Primitive, Error> {
        self.resolver.as_ref().unwrap().resolve(*plain_ref).map_err(|err| Error::from(4, format!("Failed to resolve reference {} with: {}", plain_ref.format(), err)))
    }

    /// Returns the /Root dictionary
    fn get_root(&self) -> Result<Dictionary, Error> {
        let root = self.trailer_dict.get("Root").ok_or(Error::from(5, String::from("/Root not found!")))?;
        match self.resolve_if_ref(root)? {
            Primitive::Dictionary(dict) => Ok(dict),
            _ => Err(Error::from(6, String::from("Failed to resolve /Root into a dictionary")))
        }
    }

    /// Resolves a plain_ref into the content bytes of a PDF stream
    fn resolve_stream(&self, plain_ref: &PlainRef) -> Result<Arc<[u8]>, Error> {
        match self.resolve(&plain_ref)? {
            Primitive::Stream(pdf_stream) => {
                let file_stream = Stream::<EmbeddedFile>::from_stream(pdf_stream, self.resolver.as_ref().unwrap())
                    .map_err(|err| Error::from(7, format!("Stream<EmbeddedFile> conversion failed for {} with: {}", plain_ref.format(), err)))?;

                file_stream.data(self.resolver.as_ref().unwrap()).map_err(|err| Error::from(8, format!("Failed to resolve stream {} with: {}", plain_ref.format(), err)))
            },
            _ => { return Err(Error::from(9, format!("Reference {} is not a PDF data stream", plain_ref.format()))); }
        }
    }


    fn main() -> Result<(), Error> {
        let args = Args::parse();
        let mut i = Inspector::new(args.input_path().as_path())?;
        let inspector = i.with_resolver();

        let root = inspector.get_root()?;
        println!("Catalog:");
        println!("{}\n", root.format());

        if let Some(refs) = &args.references {
            for ref_str in refs.split(',') {
                let ref_id = ref_str.parse::<u64>().map_err(|_err| Error::from(6, format!("Failed to parse '{}' as integer id", ref_str)))?;
                let plain_ref = PlainRef { id: ref_id, gen: 0 };
                let resolved = inspector.resolve(&plain_ref)?;
                println!("{}:\n{}\n", plain_ref.format(), resolved.format());
            }
        }


        if let Some(refs) = &args.print {
            for ref_str in refs.split(',') {
                let ref_id = ref_str.parse::<u64>().map_err(|_err| Error::from(6, format!("Failed to parse '{}' as integer id", ref_str)))?;
                let plain_ref = PlainRef { id: ref_id, gen: 0 };
                let bytes = inspector.resolve_stream(&plain_ref)?;
                println!("{}:\n{}\n", plain_ref.format(), String::from_utf8_lossy(bytes.as_ref()));
            }
        }

        if let Some(refs) = &args.export {
            for ref_str in refs.split(',') {
                let ref_id = ref_str.parse::<u64>().map_err(|_err| Error::from(6, format!("Failed to parse '{}' as integer id", ref_str)))?;
                let plain_ref = PlainRef { id: ref_id, gen: 0 };
                let bytes = inspector.resolve_stream(&plain_ref)?;

                let mut output_path = args.input_path().clone();
                output_path.set_extension(format!("{}.ref", ref_str));

                let mut file = OpenOptions::new().write(true).truncate(true).create(true).open(&output_path).map_err(|err| Error::from(10,format!("Failed to open {}: {}", &output_path.display().to_string(), err)))?;
                file.write_all(&*bytes).map_err(|err| Error::from(11, format!("Failed to write {}: {}", &output_path.display().to_string(), err)))?;
            }
        }

        Ok(())
    }
}

/// This is a custom string formatting trait to adjust the display of certain PDF primitives when printed to the console and
/// make them look closer to what they actually look like inside the PDF
trait Print {
    fn format(&self) -> String where Self: std::fmt::Debug {
        format!("{:?}", self)
    }
}

impl Print for PlainRef {
    fn format(&self) -> String {
        format!("{} {} R", self.id, self.gen)
    }
}

impl Print for Dictionary {
    fn format(&self) -> String {
        self.iter().map(|(name, primitive)| { format!("{} = {}", name, primitive.format()) })
            .reduce(|a, b| a + "\n" + b.as_str()).unwrap_or(String::from(""))
    }
}


impl Print for Primitive {
    fn format(&self) -> String {
        match self {
            Primitive::Null => String::from("null"),
            Primitive::Integer(i32) => format!("{}", i32),
            Primitive::Number(f32) => format!("{}", f32),
            Primitive::Boolean(bool) => format!("{:?}", bool),
            Primitive::String(pdf_string) => format!("{:?}", pdf_string),
            Primitive::Stream(pdf_stream) => format!("{}\nStream raw data <...>\n", pdf_stream.info.format()),
            Primitive::Dictionary(dict) => format!("<<\n{}\n>>", dict.format()),
            Primitive::Array(vec) => vec.format(),
            Primitive::Reference(plain_ref) => plain_ref.format(),
            Primitive::Name(small_string) => format!("/{}", small_string)
        }
    }
}


impl Print for Vec<Primitive> {
    fn format(&self) -> String {
        let elements = self.iter().map(|primitive| primitive.format()).reduce(|a,b| a + " " + b.as_str()).unwrap_or(String::from(""));
        format!("[ {} ]", elements)
    }
}


fn main() -> ExitCode {
    if let Err(error) = Inspector::main() {
        error.print();
        error.exit_code
    } else {
        ExitCode::SUCCESS
    }
}
