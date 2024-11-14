use crate::file_manager::*;
use crate::local_error::LocalError;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};

use crate::file_manager;

// available:
// fn find_related_files(start_file: &Path) -> HashSet<PathBuf>
//
// pub enum LocalError {
//     IoErr(io::Error),
//     ExitErr(process::ExitStatusError),
//     Parse(ParseIntError),
// }
//
//
/// This function finds all the header files (all code completely written in header)
/// and puts the functions content in each relevent .cpp file  
// This function will now process all relevant header and cpp files within the project
pub fn split_header(cpp_path: &PathBuf) -> Result<(), LocalError> {
    // Step 1: Find all related files in the project directory
    let related_files = find_related_files(cpp_path);

    // Step 2: Read all the header files to extract function declarations
    let mut function_declarations = Vec::new();

    for header_file in &related_files {
        if header_file
            .extension()
            .map(|ext| ext == "h" || ext == "hpp")
            .unwrap_or(false)
        {
            // Read the header file and extract function declarations
            let file = File::open(header_file).map_err(LocalError::from)?;
            let reader = io::BufReader::new(file);

            for line in reader.lines() {
                let line = line.map_err(LocalError::from)?;

                if let Some(declaration) = extract_function_declaration(&line) {
                    function_declarations.push((header_file.clone(), declaration));
                }
            }
        }
    }

    // Step 3: Process the .cpp files and add function definitions if needed
    for cpp_file in &related_files {
        if cpp_file
            .extension()
            .map(|ext| ext == "cpp")
            .unwrap_or(false)
        {
            // Read the corresponding .cpp file to check if function definitions exist
            let mut cpp_file_content = String::new();
            let mut cpp_file = File::open(cpp_file).map_err(LocalError::from)?;
            cpp_file
                .read_to_string(&mut cpp_file_content)
                .map_err(LocalError::from)?;

            // Check if function definitions are missing and append them
            let mut cpp_file = File::create(&cpp_path).map_err(LocalError::from)?;
            for (header_file, declaration) in &function_declarations {
                // Ensure the function is not already defined in the .cpp file
                if cpp_file_content.contains(declaration) {
                    continue;
                }
                writeln!(cpp_file, "{} {{}}", declaration).map_err(LocalError::from)?;
            }
        }
    }

    Ok(())
}
// Helper function to extract function declaration from a line of text
fn extract_function_declaration(line: &str) -> Option<String> {
    // This is a very simplistic function for demonstration purposes.
    // It assumes functions are declared in a simple format (e.g., "int func();").
    let trimmed = line.trim();
    if trimmed.ends_with(';') && !trimmed.contains('{') {
        Some(trimmed.to_string())
    } else {
        None
    }
}
