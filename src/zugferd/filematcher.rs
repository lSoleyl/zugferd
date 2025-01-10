use std::fmt::Display;

use pdf::{object::FileSpec, primitive::PdfString};


/// Struct for matching the embedded xml files by their name
pub struct FileMatcher {
    names: Vec<String>
}


impl FileMatcher {
    pub fn from(file: &Option<String>) -> FileMatcher {
        match file {
            Some(name) => Self::from_name(name),
            None => Self::from_default()
        }
    }

    pub fn from_default() -> FileMatcher {
        FileMatcher {
            names: Vec::from([String::from("factur-x.xml"), String::from("xrechnung.xml")]),
        }
    }

    pub fn from_name(file: &str) -> FileMatcher {
        FileMatcher {
            names: Vec::from([String::from(file)]),
        }
    }


    /// Returns true if the given pdf_str matches any of the names in this FileMatcher
    pub fn matches(&self, pdf_str: &PdfString) -> bool {
        self.names.iter().any(|name| FileMatcher::matches_str(pdf_str, name))
    }

    /// Returns true if either /F or /UF of the filespec matches the requested filename
    pub fn matching_name<'a>(&self, file_spec: &'a FileSpec) -> Option<&'a PdfString> {
        vec![&file_spec.f, &file_spec.uf].iter()
            .map(|name_option| name_option.as_ref().and_then(|name| { if self.matches(&name) { Some(name) } else { None }}))
            .fold(None, |a,b| a.or(b))
    }

    /// Returns true if either /F or /UF end with the given suffix string 
    pub fn matching_suffix<'a>(file_spec: &'a FileSpec, suffix: &str) -> Option<&'a PdfString> {
        vec![&file_spec.f, &file_spec.uf].iter()
            .map(|name_option| name_option.as_ref().and_then(|name| { if Self::matches_suffix(&name, suffix) { Some(name) } else { None }}))
            .fold(None, |a,b| a.or(b))
    }
    
    fn matches_str(pdf_str: &PdfString, str: &String) -> bool {
        pdf_str.to_string().ok().map_or(false, |decoded| decoded == *str)
    }

    fn matches_suffix(pdf_str: &PdfString, suffix: &str) -> bool {
        pdf_str.to_string().ok().map_or(false, |decoded| decoded.ends_with(suffix))
    }
}

impl Display for FileMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.names)
    }
}
