use pdf::primitive::PdfString;


/// Struct for matching the embedded xml files by their name
pub struct FileMatcher {
    names: Vec<String>
}


impl FileMatcher {
    pub fn from(file: &Option<String>) -> FileMatcher {
        FileMatcher {
            names: file.as_ref().map_or_else(|| Vec::from([String::from("factur-x.xml"), String::from("xrechnung.xml")]), | name| Vec::from([name.clone()]))
        }
    }

    // Returns true if the given pdf_str matches any of the names in this FileMatcher
    pub fn matches(&self, pdf_str: &PdfString) -> bool {
        self.names.iter().any(|name| FileMatcher::matches_str(pdf_str, name))
    }

    
    fn matches_str(pdf_str: &PdfString, str: &String) -> bool {
        pdf_str.to_string().ok().map_or(false, |decoded| decoded == *str)
    }
}
