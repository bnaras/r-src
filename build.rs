use std::{
    collections::HashMap,
    io, env,
    path::{Path, PathBuf},
    process::Command,
};

/// Holds key/value pairs parsed from "R CMD config --all".
#[derive(Debug)]
struct ConfigVariables {
    map: HashMap<String, String>,
}

impl ConfigVariables {
    fn get_r_cmd_config(&self, key: &str) -> String {
        self.map.get(key).cloned().unwrap_or_default()
    }
}

fn get_r_home() -> String {
    env::var("R_HOME").unwrap_or_else(|_| {
        panic!("Error: the required environment variable R_HOME is not set");
    })
}

/// Run the command `R RHOME` and return the trimmed stdout output.
///
/// Panics with a helpful message if the command fails.
// fn get_r_home() -> String {
//     let output = Command::new("R")
//         .arg("RHOME")
//         .output()
//         .expect("Failed to execute `R RHOME`. Is R installed and in your PATH?");
//     if !output.status.success() {
//         panic!(
//             "Error: `R RHOME` failed:\n{}",
//             String::from_utf8_lossy(&output.stderr)
//         );
//     }
//     String::from_utf8_lossy(&output.stdout).trim().to_string()
// }

/// Run `R CMD config --all` using the provided R executable path and return its stdout as a String.
fn r_cmd_config(r_binary: &Path) -> io::Result<String> {
    let output = Command::new(r_binary)
        .args(&["CMD", "config", "--all"])
        .output()?;
    if !output.stderr.is_empty() {
        println!("> {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Build the configuration map by invoking R commands.
fn build_r_cmd_configs() -> ConfigVariables {
    let r_home = get_r_home();

    // Determine the R executable path.
    let r_binary: PathBuf = if cfg!(target_os = "windows") {
        // On Windows R is typically installed in a subdirectory.
        // Try the "x64" folder first (for 64-bit installations), then fall back.
        let candidate = Path::new(&r_home).join("bin").join("x64").join("R.exe");
        if candidate.exists() {
            candidate
        } else {
            Path::new(&r_home).join("bin").join("R.exe")
        }
    } else {
        Path::new(&r_home).join("bin").join("R")
    };

    let configs = r_cmd_config(&r_binary).unwrap_or_default();
    let mut rcmd_config_map = HashMap::new();

    // Parse the output, expecting lines of the form KEY=VALUE.
    for line in configs.lines() {
        // Stop if we reach comments (the R output sometimes appends comments).
        if line.starts_with("##") {
            break;
        }
        let parts: Vec<&str> = line.split('=').map(str::trim).collect();
        if parts.len() == 2 {
            rcmd_config_map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }
    ConfigVariables {
        map: rcmd_config_map,
    }
}

/// Given a list of strings (such as BLAS, LAPACK, etc. flags),
/// extract library paths (starting with "-L") and libraries (starting with "-l").
fn get_libs_and_paths(strings: &[String]) -> (Vec<String>, Vec<String>) {
    let mut paths = Vec::new();
    let mut libs = Vec::new();
    for s in strings {
        for part in s.split_whitespace() {
            if part.starts_with("-L") {
                paths.push(part[2..].to_string());
            } else if part.starts_with("-l") {
                libs.push(part[2..].to_string());
            }
        }
    }
    (paths, libs)
}

fn main() {
    let r_configs = build_r_cmd_configs();
    let config_strings = [
        r_configs.get_r_cmd_config("BLAS_LIBS"),
        r_configs.get_r_cmd_config("LAPACK_LIBS"),
        r_configs.get_r_cmd_config("FLIBS"),
    ];
    let (lib_paths, libs) = get_libs_and_paths(&config_strings);

    // Emit link search paths. Only output those that exist.
    for path in lib_paths {
        if Path::new(&path).exists() {
            println!("cargo:rustc-link-search={}", path);
	    eprintln!("cargo:rustc-link-search={}", path);
        }
    }
    // Emit libraries for the linker.
    for lib in libs {
        println!("cargo:rustc-link-lib=dylib={}", lib);
	eprintln!("cargo:rustc-link-lib=dylib={}", lib);
    }
    println!("cargo:rerun-if-changed=build.rs");
}
