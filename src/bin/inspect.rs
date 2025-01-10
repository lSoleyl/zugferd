
use pdf::{file::{NoLog, Storage, StorageResolver, SyncCache}, object::{ParseOptions, Resolve}, primitive::Primitive};





fn main() {
    
    let input_path = "E:\\Users\\David\\Documents\\Rust\\zugferd\\pdfs\\XRECHNUNG_Betriebskostenabrechnung.pdf";


    let backend_data = std::fs::read(input_path).unwrap();

    let object_cache = SyncCache::new();
    let stream_cache = SyncCache::new();
    let password = b"";
    let parse_options = ParseOptions::strict();
    let log = NoLog;

    let mut storage = Storage::with_cache(backend_data, parse_options, object_cache, stream_cache, log).unwrap();
    let trailer_dict = storage.load_storage_and_trailer_password(password).unwrap();
    let resolver = StorageResolver::new(&storage);


    
    println!("{:?}", trailer_dict);


    let root = trailer_dict.get("Root").unwrap();
    println!("{:?}", root);

    
    let resolved = match root {
        Primitive::Reference(reference) => resolver.resolve(*reference).unwrap(),
        _ => root.clone()
    };
    println!("{:?}", resolved);


}