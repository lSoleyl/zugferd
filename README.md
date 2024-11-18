# zugferd

A small command line tool written in Rust to extract attached XML invoice file from ZUGFeRD and XRechnung compatible PDF documents using the [pdf-rs](https://github.com/pdf-rs/pdf) library.

By passing a different attachment name using the `--name` parameter you can also extract other attached files from any PDF document as the tool doesn't perform any verification regarding the filetype.


## Build
After installing Rust simply run following commands to build it

    git clone https://github.com/lSoleyl/zugferd.git
    cd zugferd
    cargo build --release

The executable file will be located in the `target/release/` folder.

## Usage

Currently there this repository consists of only one tool to extract the xml.

    Usage: extract.exe [OPTIONS] <PDF_INPUT> [ATTACHMENT_OUTPUT]

    Arguments:
    <PDF_INPUT>          PDF input file
    [ATTACHMENT_OUTPUT]  Attachment output path (default = pdfInput + ".xml")

    Options:
    -n, --name <NAME>  Specifies the name of the attachment to extract (default: "factur-x.xml" or "xrechnung.xml")
    -v, --verbose      Print additional info to the console
    -h, --help         Print help
    -V, --version      Print version