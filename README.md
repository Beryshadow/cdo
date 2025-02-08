# CDO (C++ Development Organizer)

CDO is a command-line utility designed to simplify the process of compiling and running C++ programs. With a simple command structure, it allows you to easily build, run, and clean your C++ projects. It wraps around Clang++ and takes care of finding the required dependencies, compiling them and running them for rapid development of small C++ programs.

## Usage

```bash
cdo [command] [source_file]
```
# Commands

Compile the specified C++ source file, or the one with a main function found in the current directory if no file is provided.
```bash
cdo build
```

Execute the compiled binary. If no binary exists, it will attempt to build it first.
```bash
cdo run [source_file]
```

Remove the compiled binary and the associated hash file.
```bash
cdo clean [directory]
```

Display a help message.
```bash
cdo help
```

## Behavior

If no source file is provided, CDO will search the current directory for a C++ file with a main function.
The compiled binary will be placed in the .cdo directory in the current working directory.

## Installation
To use CDO, clone the repository and proceed with the following commands:
```bash
git clone https://github.com/Beryshadow/cdo.git
cd cdo
cargo install --path .
```
