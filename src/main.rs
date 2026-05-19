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
    let mut args: Vec<String> = env::args().collect();

    // Extract program arguments (anything after `--` or after `args[2]`)
    let program_args = if let Some(idx) = args.iter().position(|x| x == "--") {
        let mut pa = args.split_off(idx);
        pa.remove(0); // Remove the "--" separator
        pa
    } else if args.len() > 3 {
        // Assume anything after `cdo <command> <file>` belongs to the executed program
        args.split_off(3)
    } else {
        vec![]
    };

    // Get the command inputed
    let command = if args.len() > 1 { args[1].clone() } else { "run".to_string() };

    // Get current dir
    let current_dir = env::current_dir().expect("Failed to get current directory");

    // Determine the cdo directory based on input or current directory
    let cdo_dir = get_cdo_dir(&args, current_dir);

    // Set the source file to either the specified path or the closest one in the current directory
    let source_file: MainPath =
        find_source_with_main(&cdo_dir.parent().expect("Expected a path").to_path_buf());

    // Create cdo dir if needed (not gonna create one if we are cleaning or not even building)
    if source_file != MainPath::None && !cdo_dir.exists() {
        eprintln!("Did print");
        fs::create_dir_all(&cdo_dir).expect("Failed to create cdo directory");
    }

    // Now gotta handle multiple file
    let (source_file, others) = source_file.choose(args.get(2));
    if source_file != MainPath::None {
        println!("Took {} other options were: {}\n", source_file, others);
    }

    // Set the binary output location
    let executable_name = match &source_file {
        MainPath::Single(source_file) => {
            Some(cdo_dir.join(Path::new(&source_file).file_stem().unwrap().to_str().unwrap()))
        }
        MainPath::None => None,
        _ => panic!(),
    };

    // Helper
    match (command.as_str(), source_file) {
        ("help", _) => {
            display_help();
        }
        ("clean", _) => {
            remove_cdo_dir(&cdo_dir);
        }

        ("build", MainPath::Single(source_file)) => {
            build(&executable_name, &source_file, &cdo_dir)?;
        }

        ("splitFiles", MainPath::Single(source_file)) => {
            // will split the headers into the H and Source files respectively
            split_files(&source_file)?;
        }

        ("run", MainPath::Single(source_file)) => {
            // Build the compiled program
            build(&executable_name, &source_file, &cdo_dir)?;
            // Run the compiled program, passing the parsed program arguments
            execute(executable_name, &program_args)?;
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

/// Execute the binary with program arguments, panic on no path
fn execute(executable_name: Option<PathBuf>, program_args: &[String]) -> Result<(), LocalError> {
    let executable_name = executable_name
        .as_ref()
        .expect("Expected a valid file path");
    fs::metadata(executable_name)?;
    let run_status = Command::new(executable_name)
        .args(program_args) // Forward program arguments here
        .status()
        .expect("Failed to run the program");
    if !run_status.success() {
        println!("\nC/C++ program failed to run.");
    };
    Ok(())
}

/// Build the executable and put it in the cdo folder
fn build(
    executable_name: &Option<PathBuf>,
    source_file: &PathBuf,
    cdo_dir: &Path,
) -> Result<(), LocalError> {
    let executable_name = executable_name
        .as_ref()
        .expect("Expected a valid file path");
    fs::metadata(source_file)?;
    let source_has_changed = new_hash(source_file, cdo_dir)?;
    if source_has_changed || !executable_name.exists() {
        // If changed, compile again
        no_check_build(executable_name, source_file)?;
    };
    Ok(())
}

/// Build the source and return a path to the binary
fn no_check_build(
    executable_name: &PathBuf,
    source_file: &PathBuf,
) -> std::result::Result<(), LocalError> {
    
    // Check if it's a C file, otherwise default to clang++
    let is_c_file = source_file.extension().is_some_and(|ext| ext == "c");
    let compiler = if is_c_file { "gcc" } else { "clang++" };

    let compile_status = Command::new(compiler)
        .arg(source_file)
        .args(
            find_related_files(source_file)
                .into_iter()
                .filter(|file| {
                    let ext = Path::new(file).extension();
                    ext == Some("cpp".as_ref()) || ext == Some("c".as_ref())
                })
                .filter(|file| file != source_file),
        )
        .arg("-o")
        .arg(executable_name)
        .status()
        .unwrap_or_else(|_| panic!("Failed to execute {}", compiler));
        
    // Make sure the file compiled successfully
    compile_status.exit_ok()?;
    println!("Compiled {} successfully.\n", source_file.to_string_lossy());
    Ok(())
}

fn display_help() {
    println!("Usage: cdo [command] [source_file] [-- program_arguments]");
    println!();
    println!("Commands:");
    println!("  build       Compiles the specified C/C++ source file or the one with a main function found in the current directory.");
    println!("  run         Executes the compiled binary. If no binary exists, it will attempt to build it first.");
    println!("              Tip: You can pass arguments to your program using '--'. Example: cdo run main.cpp -- arg1 arg2");
    println!("  clean       Removes the compiled binary and the hash file.");
    println!("  splitFiles  Splits definitions from a header file into source files.");
    println!("  help        Displays this help message.");
    println!();
    println!("If no source file is provided, the program will look for a C or C++ file with a main function in the current directory.");
    println!("The compiled binary will be placed in the '.cdo' directory in the current working directory.");
}
