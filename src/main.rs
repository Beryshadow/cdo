#![feature(exit_status_error, try_trait_v2)]
use core::panic;
use std::env;
use std::fs;
mod file_manager;
mod local_error;
mod text_edit;
use std::path::{Path, PathBuf};
use std::process::Command;

use file_manager::*;
use local_error::LocalError;
use text_edit::*;

fn main() -> Result<(), LocalError> {
    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();

    // Get the command inputed
    let command = if args.len() > 1 { &args[1] } else { "run" };

    // Get current dir
    let current_dir = env::current_dir().expect("Failed to get current directory");

    // Determine the cdo directory based on input or current directory
    let cdo_dir = get_cdo_dir(&args, current_dir);

    // if command == "clean" {}

    // Set the cpp file to either the specified path or the closest one in the current directory
    let cpp_file: MainPath =
        find_cpp_with_main(&cdo_dir.parent().expect("Expected a path").to_path_buf());

    // Create cdo dir if needed (not gonna create one if we are cleaning or not even building)
    if cpp_file != MainPath::None && !cdo_dir.exists() {
        eprintln!("Did print");
        fs::create_dir_all(&cdo_dir).expect("Failed to create cdo directory");
    }

    // Now gotta handle multiple file
    let (cpp_file, others) = cpp_file.choose(args.get(2));
    if cpp_file != MainPath::None {
        println!("Took {} other options were: {}\n", cpp_file, others);
    }

    // Set the cpp binary output location
    let executable_name = match &cpp_file {
        MainPath::Single(cpp_file) => {
            Some(cdo_dir.join(Path::new(&cpp_file).file_stem().unwrap().to_str().unwrap()))
        }
        MainPath::None => None,
        _ => panic!(),
    };

    // Helper
    match (command, cpp_file) {
        ("help", _) => {
            display_help();
        }
        ("clean", _) => {
            remove_cdo_dir(&cdo_dir);
        }

        ("build", MainPath::Single(cpp_file)) => {
            build(&executable_name, &cpp_file, &cdo_dir)?;
        }

        ("splitFiles", MainPath::Single(cpp_file)) => {
            // will split the headers into the H and Cpp files respectively
            split_files(&cpp_file)?;
        }

        ("run", MainPath::Single(cpp_file)) => {
            // Build the compiled program
            build(&executable_name, &cpp_file, &cdo_dir)?;
            // Run the compiled program
            execute(executable_name)?;
        }

        ("run" | "build", MainPath::None) => {
            eprintln!("You used the \"{}\" command without an available path, either provide one or go to the correct directory.", command);
        }
        _ => {
            eprintln!(
                "Unknown command: {}. Use 'build', 'run', 'clean', or 'help'.",
                command
            );
        }
    }
    Ok(())
}

/// Execute the binary, panic on no path
fn execute(executable_name: Option<PathBuf>) -> Result<(), LocalError> {
    let executable_name = executable_name
        .as_ref()
        .expect("Expected a valid file path");
    fs::metadata(executable_name)?;
    let run_status = Command::new(executable_name)
        .status()
        .expect("Failed to run the program");
    if !run_status.success() {
        println!("\nC++ program failed to run.");
    };
    Ok(())
}

/// Build the executable and put it in the cdo folder
fn build(
    executable_name: &Option<PathBuf>,
    cpp_file: &PathBuf,
    cdo_dir: &Path,
) -> Result<(), LocalError> {
    let executable_name = executable_name
        .as_ref()
        .expect("Expected a valid file path");
    fs::metadata(cpp_file)?;
    let source_has_changed = new_hash(cpp_file, cdo_dir)?;
    if source_has_changed {
        // If changed, compile again
        no_check_build(executable_name, cpp_file)?;
    };
    Ok(())
}

/// Build the source and return a path to the binary
fn no_check_build(
    executable_name: &PathBuf,
    cpp_file: &PathBuf,
) -> std::result::Result<(), LocalError> {
    // Compile the C++ code using clang++
    let compile_status = Command::new("clang++")
        .arg(cpp_file)
        .arg("-o")
        .arg(executable_name)
        .status()
        .expect("Failed to execute clang++");
    // Make sure the file compiled successfully
    compile_status.exit_ok()?;
    println!("Compiled {} successfully.\n", cpp_file.to_string_lossy());
    Ok(())
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
