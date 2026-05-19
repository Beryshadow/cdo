use crate::file_manager::*;
use crate::local_error::LocalError;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

/// This function finds all the header files (all code completely written in header)
/// and puts the functions content in each relevent source (.cpp or .c) file  
// This function will now process all relevant header and source files within the project
pub fn split_files(source_path: &PathBuf) -> Result<(), LocalError> {
    // Step 1: Find all related files in the project directory
    let related_files: HashSet<PathBuf> = find_related_files(source_path);

    // print the related files
    println!("Related files vec: {:?}\n", related_files);

    // Step 2: Make a list of all the source and .h files that are associated
    let mut associated: HashMap<PathBuf, PathBuf> = HashMap::new();

    // We iterate through the related files and try to find source and header pairs
    for file in &related_files {
        let file_str = file.to_str().unwrap_or_default();
        if file_str.ends_with(".cpp") || file_str.ends_with(".c") {
            let header_path = file.with_extension("h"); // Try to find a .h file
            if header_path.exists() {
                associated.insert(file.clone(), header_path);
            }
        } else if file_str.ends_with(".h") {
            let cpp_path = file.with_extension("cpp"); // Try to find a .cpp file
            if cpp_path.exists() {
                associated.insert(cpp_path, file.clone());
            } else {
                let c_path = file.with_extension("c"); // Try to find a .c file
                if c_path.exists() {
                    associated.insert(c_path, file.clone());
                }
            }
        }
    }

    // print the associated vec
    println!("Associated vec: {:?}", associated);

    // Step 3: Process the .h and source files to move function definitions
    for (source_file, header) in associated {
        let header_content = read_file(&header)?;
        let source_content = read_file(&source_file)?;

        let (updated_header, updated_source) = move_functions_to_source(&header_content, &source_content)?;

        // show the result
        println!("Source File Output: {updated_source}");
        eprintln!("Headers Output: {updated_header}");

        // Step 4: Write the updated contents back to the files
        write_file(&header, &updated_header)?;
        write_file(&source_file, &updated_source)?;
    }

    Ok(())
}

// Helper function to read a file's content
fn read_file(path: &PathBuf) -> Result<String, LocalError> {
    let mut file = File::open(path).map_err(|e| LocalError::IoErr(e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| LocalError::IoErr(e))?;
    Ok(content)
}

// Helper function to write to a file
fn write_file(path: &PathBuf, content: &str) -> Result<(), LocalError> {
    let mut file = File::create(path).map_err(|e| LocalError::IoErr(e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| LocalError::IoErr(e))?;
    Ok(())
}

// Function to process header and source contents
fn move_functions_to_source(
    header_content: &str,
    source_content: &str,
) -> Result<(String, String), LocalError> {
    let mut updated_header = header_content.to_string();
    let mut updated_source = source_content.to_string();

    // Split the header content by lines
    let mut lines = header_content.lines().collect::<Vec<&str>>();

    let mut function_defs: Vec<(String, String)> = Vec::new(); // Store functions to be moved

    // Iterate through lines of header content to find classes and functions
    let mut i = 0;
    let mut inside_class = false;
    let mut class_name = String::new();
    let mut class_methods = Vec::new();

    while i < lines.len() {
        let line = lines[i].trim();

        if let Some(class_start) = parse_class_start(line) {
            // Entering class definition
            inside_class = true;
            class_name = class_start;
            class_methods.clear(); // Reset class methods for this class
        }

        if inside_class {
            if let Some(method_signature) = parse_function_signature(line) {
                class_methods.push(method_signature.to_string());
            }

            // Check if we find the closing brace of the class
            if line.contains('}') {
                // Process the class methods
                for method in class_methods.iter() {
                    if let Some((signature, body)) = extract_function_body(method, &lines, &mut i) {
                        // We have the body of the function
                        function_defs.push((signature, body));
                    }
                }
                // Now, leave the class
                inside_class = false;
                class_name.clear();
            }
        }

        i += 1;
    }

    // Process the found functions
    for (signature, body) in function_defs {
        updated_header = updated_header.replace(&signature, &format!("{};", signature)); // Keep only the signature
        updated_source.push_str(&format!("\n{}\n", body)); // Add to source file
    }

    Ok((updated_header, updated_source))
}

// Try to parse a class start (class or struct definition)
fn parse_class_start(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with("class ") || line.starts_with("struct ") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() > 1 {
            return Some(parts[1].to_string()); // Return the class/struct name
        }
    }
    None
}

// Try to parse a function signature (ignoring the body)
fn parse_function_signature(line: &str) -> Option<String> {
    let line = line.trim();
    // A very basic check for function signature in the form: return_type function_name(params)
    if line.ends_with("{") {
        return Some(line.to_string());
    }
    None
}

// Extract function body from the header content using the signature
fn extract_function_body(
    signature: &str,
    lines: &[&str],
    current_idx: &mut usize,
) -> Option<(String, String)> {
    let mut body = String::new();
    let mut brace_stack = Vec::new();
    let mut i = *current_idx;

    body.push_str(signature); // Start with the signature

    brace_stack.push('{'); // We are inside a function, so push opening brace

    while i < lines.len() {
        let current_line = lines[i].trim();

        if current_line.contains('{') {
            brace_stack.push('{');
        }

        if current_line.contains('}') {
            brace_stack.pop();
            if brace_stack.is_empty() {
                // We found the matching closing brace
                break;
            }
        }

        body.push_str(current_line); // Add the current line to function body
        body.push('\n');
        i += 1;
    }

    *current_idx = i; // Update the index to the current position
    Some((signature.to_string(), body)) // Return both the signature and the body
}
