use pdf::{object::FileSpec, primitive::PdfString};


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

    // Returns true if either /F or /UF of the filespec matches the requested filename
    pub fn matching_name<'a>(&self, file_spec: &'a FileSpec) -> Option<&'a PdfString> {
        vec![&file_spec.f, &file_spec.uf].iter()
            .map(|name_option| name_option.as_ref().and_then(|name| { if self.matches(&name) { Some(name) } else { None }}))
            .fold(None, |a,b| a.or(b))
    }


    
    fn matches_str(pdf_str: &PdfString, str: &String) -> bool {
        pdf_str.to_string().ok().map_or(false, |decoded| decoded == *str)
    }
}
