use core::panic;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
// use std::fs::ReadDir;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();

    // Get current dir
    let current_dir = env::current_dir().expect("Failed to get current directory");

    let command = if args.len() > 1 { &args[1] } else { "run" };
    let cpp_file = if args.len() > 2 {
        Some(args[2].clone())
    } else {
        find_cpp_with_main(&current_dir)
    };

    // Create the cdo directory based on the current working directory
    // let cdo_dir = current_dir.join("cdo");
    // Determine the cdo directory based on input or current directory
    let cdo_dir = if args.len() > 2 {
        let input_dir = &args[2];
        let path = Path::new(input_dir);
        if path.is_dir() {
            path.join("cdo")
        } else {
            eprintln!("Provided path is not a valid directory. Using current directory instead.");
            current_dir.join("cdo")
        }
    } else {
        current_dir.join("cdo")
    };
    if (command == "run" || command == "build") && (cpp_file.is_some()) {
        // Create cdo dir if needed
        if !cdo_dir.exists() {
            fs::create_dir_all(&cdo_dir).expect("Failed to create cdo directory");
        }
    }

    match (command, cpp_file) {
        ("help", _) => {
            display_help();
        }
        ("clean", _) => {
            // Remove the entire cdo directory
            if fs::metadata(&cdo_dir).is_ok() {
                fs::remove_dir_all(&cdo_dir).expect("Failed to delete cdo directory");
                println!("Deleted cdo directory: {}", cdo_dir.display());
            } else {
                println!("No cdo directory found to delete.");
            }
        }
        ("build", Some(cpp_file)) => {
            let executable_name =
                cdo_dir.join(Path::new(&cpp_file).file_stem().unwrap().to_str().unwrap());
            let hash_file = cdo_dir.join(format!(
                "{}.hash",
                Path::new(&cpp_file).file_stem().unwrap().to_str().unwrap()
            ));
            // Compile the C++ code using clang++
            let compile_status = Command::new("clang++")
                .arg(&cpp_file)
                .arg("-o")
                .arg(&executable_name)
                .status()
                .expect("Failed to execute clang++");

            if compile_status.success() {
                println!("Compiled {} successfully.", cpp_file);
                // Store the hash of the source file
                match calculate_hash(&cpp_file) {
                    Ok(hash) => {
                        if let Err(e) = fs::write(&hash_file, hash.to_string()) {
                            eprintln!("Failed to write hash file: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Failed to calculate hash: {}", e),
                }
            } else {
                eprintln!("Failed to compile {}.", cpp_file);
            }
        }

        ("run", Some(cpp_file)) => {
            let executable_name =
                cdo_dir.join(Path::new(&cpp_file).file_stem().unwrap().to_str().unwrap());
            let hash_file = cdo_dir.join(format!(
                "{}.hash",
                Path::new(&cpp_file).file_stem().unwrap().to_str().unwrap()
            ));
            // Check if the source file exists
            if !fs::metadata(&cpp_file).is_ok() {
                eprintln!("Error: Source file '{}' does not exist.", cpp_file);
                return;
            }

            // Calculate the current hash of the source file
            let current_hash = match calculate_hash(&cpp_file) {
                Ok(hash) => hash,
                Err(e) => {
                    eprintln!("Failed to calculate hash: {}", e);
                    return;
                }
            };

            let previous_hash = if fs::metadata(&hash_file).is_ok() {
                match fs::read_to_string(&hash_file) {
                    Ok(hash_string) => hash_string.trim().parse::<u64>().unwrap_or(0),
                    Err(e) => {
                        eprintln!("Failed to read hash file: {}", e);
                        return;
                    }
                }
            } else {
                0
            };

            // Check if the source file has changed
            if current_hash != previous_hash {
                // If changed, compile again
                let compile_status = Command::new("clang++")
                    .arg(&cpp_file)
                    .arg("-o")
                    .arg(&executable_name)
                    .status()
                    .expect("Failed to execute clang++");

                if compile_status.success() {
                    println!("Compiled {}.", cpp_file);
                    let hash = match calculate_hash(&cpp_file) {
                        Ok(hash) => hash,
                        Err(e) => {
                            eprintln!("Failed to calculate hash: {}", e);
                            return;
                        }
                    };
                    if let Err(e) = fs::write(&hash_file, hash.to_string()) {
                        eprintln!("Failed to write hash file: {}", e);
                    }
                } else {
                    eprintln!("Failed to compile {}.", cpp_file);
                    return;
                }
            }

            // Run the compiled program
            if !fs::metadata(&executable_name).is_ok() {
                eprintln!(
                    "Error: Executable '{}' not found. Please build first.",
                    executable_name.display()
                );
                return;
            }

            let run_status = Command::new(&executable_name)
                .status()
                .expect("Failed to run the program");

            if run_status.success() {
                // println!("\nC++ program ran successfully.");
            } else {
                println!("\nC++ program failed to run.");
            }
        }

        ("run" | "build", None) => {
            eprintln!("You used the {} command without an available path, either provide one or go to the correct directory.", command);
        }
        _ => {
            eprintln!(
                "Unknown command: {}. Use 'build', 'run', or 'clean'.",
                command
            );
        }
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
