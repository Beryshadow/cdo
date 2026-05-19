use std::{
    collections::HashSet,
    fmt::Display,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io::{self, Read},
    path::{Path, PathBuf},
};
use strsim::levenshtein;

use crate::local_error::LocalError;

#[derive(PartialEq)]
pub enum MainPath {
    Multiple(Vec<PathBuf>),
    Single(PathBuf),
    None,
}

impl Display for MainPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MainPath::Multiple(paths) => {
                write!(
                    f,
                    "{}",
                    paths.iter().fold(String::new(), |acc, p| format!(
                        "{}\n{}",
                        acc,
                        p.file_name().unwrap().to_str().unwrap()
                    ))
                )
            }
            MainPath::Single(path) => {
                write!(f, "{}", path.file_name().unwrap().to_str().unwrap())
            }
            MainPath::None => write!(f, "Nothing"),
        }
    }
}

impl MainPath {
    /// Allows you to separate the MainPaths and Retain the only one with a value
    pub fn choose(&self, specific_file: Option<&String>) -> (MainPath, MainPath) {
        let specific_file = specific_file.map(PathBuf::from);
        // Default return values for the match
        let mut to_keep = MainPath::None;
        let mut others = MainPath::None;

        match (self, specific_file) {
            (MainPath::Multiple(ref paths), Some(specific)) => {
                // Find the candidate with the minimum Levenshtein distance
                let path_to_keep = paths
                    .iter()
                    .min_by_key(|candidate| {
                        levenshtein(specific.to_str().unwrap(), candidate.to_str().unwrap())
                    })
                    .unwrap();
                others = MainPath::Multiple(
                    paths
                        .iter()
                        .filter_map(|p| (p != path_to_keep).then_some(p.clone()))
                        .collect(),
                );
                to_keep = MainPath::Single(path_to_keep.to_owned());
            }
            (MainPath::None, _) => {}
            (MainPath::Single(path), _) => {
                to_keep = MainPath::Single(path.clone());
            }
            (MainPath::Multiple(ref paths), None) => {
                let mut paths = paths.clone();
                to_keep = MainPath::Single(paths.pop().unwrap());
                others = MainPath::Multiple(paths);
            }
        }

        (to_keep, others)
    }
}

/// find the closest non recursive main (1st layer)
pub fn find_source_with_main(dir: &PathBuf) -> MainPath {
    // Create a vec to store all the source files with a main method
    let paths: Vec<PathBuf> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| {
            entries
                .filter_map(Result::ok) // Filter out errors
                .filter(|entry| {
                    let path = entry.path();
                    path.extension()
                        .is_some_and(|ext| ext == "cpp" || ext == "c" || ext == "h" || ext == "hpp") // Check for valid extensions
                })
                .filter_map(|entry| {
                    let path = entry.path();
                    fs::read_to_string(&path)
                        .ok()
                        .filter(|contents| contents.contains("int main")) // Check for main function
                        .map(|_| path ) // Map to string path
                })
        })
        .collect(); // Collect results into a Vec

    match paths.len() {
        0 => MainPath::None,
        1 => MainPath::Single(paths.into_iter().next().unwrap()),
        _ => MainPath::Multiple(paths),
    }
}

pub fn get_cdo_dir(args: &[String], current_dir: PathBuf) -> PathBuf {
    let cdo_dir = if args.len() > 2 {
        let input_dir = &*args[2];
        let path = Path::new(input_dir);
        if path.is_dir() {
            path.join(".cdo")
        } else if path.parent().is_some_and(|p| p.exists()) {
            // If its a full path
            path.parent().unwrap().join(".cdo")
        } else {
            // If its just the file name from local
            current_dir.join(".cdo")
        }
    } else {
        // Create the cdo directory based on the current working directory
        current_dir.join(".cdo")
    };
    cdo_dir
}

pub fn remove_cdo_dir(cdo_dir: &PathBuf) {
    if fs::metadata(cdo_dir).is_ok() {
        fs::remove_dir_all(cdo_dir).expect("Failed to delete cdo directory");
        println!("Deleted cdo directory: {}", cdo_dir.display());
    } else {
        println!("No cdo directory found to delete.");
    }
}

/// Update the hash if necessary and returns true if it was changed
pub fn new_hash(source_file: &Path, cdo_dir: &Path) -> Result<bool, LocalError> {
    // calculate the current hash
    let current_hash = calculate_hash(source_file)?;
    // create the path to the hash
    let hash_file = cdo_dir.join(format!(
        "{}.hash",
        source_file.file_stem().unwrap().to_str().unwrap()
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

fn calculate_hash(file_path: &Path) -> io::Result<u64> {
    let mut hasher = DefaultHasher::new();
    let mut contents = Vec::new();
    let mut dependencies: Vec<_> = find_related_files(file_path).into_iter().collect();
    dependencies.sort();

    println!("The used files are: ");
    for path in dependencies {
        let mut file = fs::File::open(&path)?;
        file.read_to_end(&mut contents)?;
        println!("{}", path.file_name().unwrap().to_str().unwrap());
    }
    println!();
    contents.hash(&mut hasher);
    Ok(hasher.finish())
}

pub fn find_related_files(start_file: &Path) -> HashSet<PathBuf> {
    let mut dependencies = HashSet::new();
    let mut to_visit = vec![start_file.to_path_buf()]; // Start with the main file

    while let Some(file) = to_visit.pop() {
        if dependencies.contains(&file) {
            continue; // Skip already visited files
        }

        // Read the file content
        if let Ok(contents) = fs::read_to_string(&file) {
            dependencies.insert(file.clone()); // Add the current file to dependencies

            // FIND the related CPP, C or H file associated with this file
            let file_str = file.to_str().unwrap_or_default();
            if file_str.ends_with(".cpp") || file_str.ends_with(".c") {
                let header_path = file.with_extension("h"); // Try to find a .h file
                if header_path.exists() {
                    to_visit.push(header_path);
                }
                let hpp_path = file.with_extension("hpp"); // Try to find a .hpp file
                if hpp_path.exists() {
                    to_visit.push(hpp_path);
                }
            } else if file_str.ends_with(".h") || file_str.ends_with(".hpp") {
                let cpp_path = file.with_extension("cpp"); // Try to find a .cpp file
                if cpp_path.exists() {
                    to_visit.push(cpp_path);
                }
                let c_path = file.with_extension("c"); // Try to find a .c file
                if c_path.exists() {
                    to_visit.push(c_path);
                }
            }

            for line in contents.lines() {
                if let Some(include_path) = extract_include_path(line) {
                    let full_path = file.parent().unwrap().join(&include_path);

                    // Only include files in the same directory or subdirectories
                    if full_path.is_file()
                        && is_within_directory(&full_path, file.parent().unwrap())
                        && !dependencies.contains(&full_path)
                    {
                        to_visit.push(full_path);
                    }
                }
            }
        }
    }

    dependencies
}

// Extract the included file name from #include directives
fn extract_include_path(line: &str) -> Option<String> {
    if line.starts_with("#include") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 {
            let path = parts[1];
            let clean_path = path.trim_matches('<').trim_matches('>').trim_matches('"');
            return Some(clean_path.to_string());
        }
    }
    None
}

// Check if a path is within a given directory
fn is_within_directory(file: &Path, dir: &Path) -> bool {
    file.starts_with(dir)
}
