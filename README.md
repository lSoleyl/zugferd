# zugferd

A small command line tool written in Rust to extract attached XML invoice file from ZUGFeRD and XRechnung compatible PDF documents using the [pdf-rs](https://github.com/pdf-rs/pdf) library.

By passing a different attachment name using the `--name` parameter you can also extract other attached files from any PDF document as the tool doesn't perform any verification regarding the filetype.


When passing the `--strict` parameter, the tool will exit with a non zero exit code if the PDF invoice is not standard conformant while also printing to stderr what is wrong with the file. 

Otherwise the tool will attempt multiple fallbacks to extract an XML invoice even from PDF files, which do not follow the standard. With the `--verbose` flag set the tool will output all attempted fallbacks and which XML invoice has been found.


## Build
After installing Rust simply run following commands to build it

    git clone https://github.com/lSoleyl/zugferd.git
    cd zugferd
    cargo build --release

To build a version compatible with Windows 7 run following code

    rustup install 1.75.0-x86_64-pc-windows-msvc
    cargo +1.75.0-x86_64-pc-windows-msvc build --release

The executable file will be located in the `target/release/` folder.

## `extract`

The main binary for extraction of e-invoice XML data from a given PDF file.

    Usage: extract.exe [OPTIONS] <PDF_INPUT> [ATTACHMENT_OUTPUT]

    Arguments:
    <PDF_INPUT>          PDF input file
    [ATTACHMENT_OUTPUT]  Attachment output path (default = pdfInput + ".xml")

    Options:
    -n, --name <NAME>  Specifies the name of the attachment to extract (default: derived from Metadata or "factur-x.xml" or "xrechnung.xml")
    -v, --verbose      Print additional info to the console
    -s, --strict       Exit with an error if the file is not a valid e-invoice. If not set the tool will try to extract any .xml file somehow
    -h, --help         Print help
    -V, --version      Print version

## `inspect`

This binary allows low level inspection of a given PDF file and extraction of embedded file streams.

    Usage: inspect.exe [OPTIONS] <PDF_INPUT>
    
    Arguments:
      <PDF_INPUT>  PDF input file
    
    Options:
      -r, --ref <REFERENCES>  resolve the ref with the given id (separate by comma if passing multiple)
      -p, --print <PRINT>     print the content of the stream(s) behind the given ref(s) to the console
      -e, --export <EXPORT>   export the raw stream behind the given ref(s) and write it to a new file (next to the input PDF)
      -h, --help              Print help
      -V, --version           Print version
