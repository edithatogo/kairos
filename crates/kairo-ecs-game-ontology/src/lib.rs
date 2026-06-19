#[derive(Debug, Clone, PartialEq, Eq)] 
pub struct OntologyClass { 
    pub id: String, 
} 
 
#[derive(Debug, Clone, PartialEq, Eq)] 
pub struct OntologyProperty { 
    pub id: String, 
    pub domain: String, 
    pub range: String, 
} 
 
#[derive(Debug, Clone, PartialEq, Eq)] 
pub struct OntologyDocument { 
    pub classes: Vec<OntologyClass>, 
    pub properties: Vec<OntologyProperty>, 
} 
 
#[derive(Debug, Clone, PartialEq, Eq)] 
pub struct ParseError { 
    pub line: usize, 
}
 
pub fn parse_turtle(input: &str) -> Result<OntologyDocument, ParseError> { 
    let mut classes = Vec::new(); 
    let mut properties = Vec::new(); 
    for (index, raw_line) in input.lines().enumerate() { 
        let line = raw_line.trim(); 
        if line.is_empty() { 
            continue; 
        } 
        if line.as_bytes().starts_with(&[64, 112, 114, 101, 102, 105, 120]) { 
            continue; 
        } 
        let subject = subject(line, index + 1)?; 
        if contains(line.as_bytes(), &[32, 97, 32, 114, 100, 102, 115, 58, 67, 108, 97, 115, 115, 32, 46]) { 
            classes.push(OntologyClass { id: subject.to_owned() }); 
            continue; 
        } 
        if contains(line.as_bytes(), &[32, 97, 32, 114, 100, 102, 115, 58, 80, 114, 111, 112, 101, 114, 116, 121, 32]) { 
            let domain = object_after(line, &[114, 100, 102, 115, 58, 100, 111, 109, 97, 105, 110], index + 1)?; 
            let range = object_after(line, &[114, 100, 102, 115, 58, 114, 97, 110, 103, 101], index + 1)?; 
            properties.push(OntologyProperty { id: subject.to_owned(), domain, range }); 
            continue; 
        } 
        return Err(ParseError { line: index + 1 }); 
    } 
    Ok(OntologyDocument { classes, properties }) 
}
 
fn subject(line: &str, line_number: usize) -> Result<&str, ParseError> { 
    line.split_whitespace().next().ok_or(ParseError { line: line_number }) 
} 
 
fn object_after(line: &str, predicate: &[u8], line_number: usize) -> Result<String, ParseError> { 
    let bytes = line.as_bytes(); 
    let start = find(bytes, predicate).ok_or(ParseError { line: line_number })? + predicate.len(); 
    let tail = &line[start..]; 
    let token = tail.trim_start().split_whitespace().next().ok_or(ParseError { line: line_number })?; 
    Ok(token.trim_end_matches(';').trim_end_matches('.').to_owned()) 
} 
 
fn contains(haystack: &[u8], needle: &[u8]) -> bool { 
    find(haystack, needle).is_some() 
} 
 
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> { 
    if needle.is_empty() { 
        return Some(0); 
    } 
    let limit = haystack.len().saturating_sub(needle.len()); 
    for start in 0..=limit { 
        if &haystack[start..start + needle.len()] == needle { 
            return Some(start); 
        } 
    } 
    None 
}
