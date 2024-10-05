#![feature(exit_status_error, try_trait_v2)]
// use core::panic;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
mod local_error;
// use std::fs::ReadDir;
use std::hash::{Hash, Hasher};
// use std::io::Result;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use local_error::LocalError;

fn main() -> Result<(), LocalError> {
    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();

    // Get current dir
    let current_dir = env::current_dir().expect("Failed to get current directory");

    let command = if args.len() > 1 { &args[1] } else { "run" };

    // Create the cdo directory based on the current working directory
    // let cdo_dir = current_dir.join(".cdo");
    // Determine the cdo directory based on input or current directory
    let cdo_dir = if args.len() > 2 {
        let input_dir = &args[2];
        let path = Path::new(input_dir);
        if path.parent().is_some() {
            path.parent().unwrap().join(".cdo")
        } else {
            eprintln!("Provided path is not a valid file. Using current directory instead.");
            current_dir.join(".cdo")
        }
    } else {
        current_dir.join(".cdo")
    };

    // Set the cpp file to either the specified path or the closest one in the current directory
    let cpp_file = if args.len() > 2 {
        Some(args[2].clone())
    } else {
        find_cpp_with_main(&cdo_dir.parent().expect("Expected a path").to_path_buf())
    };

    // Create cdo dir if needed (not gonna create one if we are cleaning)
    if (command == "run" || command == "build")
        && (cpp_file.as_ref().is_some_and(|x| fs::metadata(x).is_ok()))
    {
        if !cdo_dir.exists() {
            fs::create_dir_all(&cdo_dir).expect("Failed to create cdo directory");
        }
    }

    // Set the cpp binary output location
    let executable_name = match &cpp_file {
        Some(cpp_file) => {
            Some(cdo_dir.join(Path::new(&cpp_file).file_stem().unwrap().to_str().unwrap()))
        }
        None => None,
    };

    match (command, cpp_file) {
        ("help", _) => {
            display_help();
        }
        ("clean", _) => {
            // Remove the entire cdo directory
            remove_cdo_dir(&cdo_dir);
        }

        ("build", Some(cpp_file)) => {
            build(&executable_name, cpp_file, &cdo_dir)?;
        }

        ("run", Some(cpp_file)) => {
            // Build the compiled program
            build(&executable_name, cpp_file, &cdo_dir)?;
            // Run the compiled program
            execute(executable_name)?;
        }

        ("run" | "build", None) => {
            eprintln!("You used the \"{}\" command without an available path, either provide one or go to the correct directory.", command);
        }
        _ => {
            eprintln!(
                "Unknown command: {}. Use 'build', 'run', or 'clean'.",
                command
            );
        }
    }
    Ok(())
}

fn execute(executable_name: Option<PathBuf>) -> Result<(), LocalError> {
    let executable_name = executable_name
        .as_ref()
        .expect("Expected a valid file path");
    fs::metadata(&executable_name)?;
    let run_status = Command::new(&executable_name)
        .status()
        .expect("Failed to run the program");
    Ok(if !run_status.success() {
        println!("\nC++ program failed to run.");
    })
}

fn build(
    executable_name: &Option<PathBuf>,
    cpp_file: String,
    cdo_dir: &PathBuf,
) -> Result<(), LocalError> {
    let executable_name = executable_name
        .as_ref()
        .expect("Expected a valid file path");
    fs::metadata(&cpp_file)?;
    let source_has_changed = new_hash(&cpp_file, cdo_dir)?;
    Ok(if source_has_changed {
        // If changed, compile again
        no_req_build(&executable_name, &cpp_file)?;
    })
}

/// Update the hash if necessary and returns true if it was changed
fn new_hash(cpp_file: &String, cdo_dir: &PathBuf) -> Result<bool, LocalError> {
    // calculate the current hash
    let current_hash = calculate_hash(cpp_file)?;
    // create the path to the hash
    let hash_file = cdo_dir.join(format!(
        "{}.hash",
        Path::new(&cpp_file).file_stem().unwrap().to_str().unwrap()
    ));
    // obtain the old hash
    let previous_hash: Option<u64> = if fs::metadata(&hash_file).is_ok() {
        // We have a valid previous hash
        Some(
            fs::read_to_string(&hash_file)?
                .trim()
                .parse::<u64>()
                .unwrap(),
        )
    } else {
        // We dont have a valid previous hash
        None
    };
    let source_has_changed = !(previous_hash.is_some() && current_hash == previous_hash.unwrap());
    if source_has_changed {
        fs::write(&hash_file, current_hash.to_string())?
    };
    Ok(source_has_changed)
}

/// Build the source and return a path to the binary
fn no_req_build(
    executable_name: &PathBuf,
    cpp_file: &String,
) -> std::result::Result<(), LocalError> {
    // Compile the C++ code using clang++
    let compile_status = Command::new("clang++")
        .arg(&cpp_file)
        .arg("-o")
        .arg(&executable_name)
        .status()
        .expect("Failed to execute clang++");
    // Make sure the file compiled successfully
    compile_status.exit_ok()?;
    println!("Compiled {} successfully.", cpp_file);
    Ok(())
}

fn remove_cdo_dir(cdo_dir: &PathBuf) {
    if fs::metadata(cdo_dir).is_ok() {
        fs::remove_dir_all(cdo_dir).expect("Failed to delete cdo directory");
        println!("Deleted cdo directory: {}", cdo_dir.display());
    } else {
        println!("No cdo directory found to delete.");
    }
}

fn calculate_hash(file_path: &str) -> io::Result<u64> {
    let mut hasher = DefaultHasher::new();
    let mut file = fs::File::open(file_path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    contents.hash(&mut hasher);
    Ok(hasher.finish())
}

fn find_cpp_with_main(dir: &PathBuf) -> Option<String> {
    // Read the directory
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            // Check if the file is a .cpp file
            if path.extension().map_or(false, |ext| ext == "cpp") {
                // Read the file to check for a main function
                if let Ok(contents) = fs::read_to_string(&path) {
                    if contents.contains("int main") {
                        return Some(path.to_str().unwrap().to_string());
                    }
                }
            }
        }
    }
    None
}

fn display_help() {
    println!("Usage: cdo [command] [source_file]");
    println!();
    println!("Commands:");
    println!("  build       Compiles the specified C++ source file or the one with a main function found in the current directory.");
    println!("  run         Executes the compiled binary. If no binary exists, it will attempt to build it first.");
    println!("  clean       Removes the compiled binary and the hash file.");
    println!("  help        Displays this help message.");
    println!();
    println!("If no source file is provided, the program will look for a C++ file with a main function in the current directory.");
    println!("The compiled binary will be placed in the 'cdo' directory in the current working directory.");
}
