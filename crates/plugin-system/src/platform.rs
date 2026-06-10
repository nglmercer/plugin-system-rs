use std::path::{Path, PathBuf};

/// Get the platform-specific dynamic library extension.
///
/// Returns `"so"` on Linux, `"dylib"` on macOS, `"dll"` on Windows.
pub fn library_extension() -> &'static str {
    if cfg!(target_os = "linux") {
        "so"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

/// Get the platform-specific library filename for a given name.
///
/// # Examples
/// - Linux: `library_filename("hello")` → `"libhello.so"`
/// - macOS: `library_filename("hello")` → `"libhello.dylib"`
/// - Windows: `library_filename("hello")` → `"hello.dll"`
pub fn library_filename(name: &str) -> String {
    let ext = library_extension();
    if cfg!(target_os = "windows") {
        format!("{}.{}", name, ext)
    } else {
        format!("lib{}.{}", name, ext)
    }
}

/// Get the full path to a platform-specific library in a directory.
///
/// # Examples
/// - Linux: `library_path("./plugins", "hello")` → `"./plugins/libhello.so"`
/// - macOS: `library_path("./plugins", "hello")` → `"./plugins/libhello.dylib"`
/// - Windows: `library_path("./plugins", "hello")` → `"./plugins/hello.dll"`
pub fn library_path(dir: impl AsRef<Path>, name: &str) -> PathBuf {
    dir.as_ref().join(library_filename(name))
}

/// Copy a plugin library from source to destination directory.
///
/// Returns the destination path if successful.
pub fn copy_plugin(
    source: impl AsRef<Path>,
    dest_dir: impl AsRef<Path>,
) -> std::io::Result<PathBuf> {
    let source = source.as_ref();
    let filename = source.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid source path")
    })?;
    let dest = dest_dir.as_ref().join(filename);
    std::fs::copy(source, &dest)?;
    Ok(dest)
}

/// Copy a plugin library using the platform-specific naming convention.
///
/// Given a cargo package name (e.g., `"plugin-hello"`), this function:
/// 1. Constructs the platform-specific source filename
/// 2. Copies it to the destination directory
///
/// Returns the destination path if the source exists and was copied.
pub fn copy_cargo_plugin(
    source_dir: impl AsRef<Path>,
    dest_dir: impl AsRef<Path>,
    cargo_name: &str,
) -> std::io::Result<Option<PathBuf>> {
    let source = source_dir.as_ref().join(library_filename(cargo_name));
    if source.exists() {
        let dest = copy_plugin(&source, &dest_dir)?;
        Ok(Some(dest))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_extension() {
        let ext = library_extension();
        assert!(["so", "dylib", "dll"].contains(&ext));
    }

    #[test]
    fn test_library_filename() {
        let name = library_filename("hello");
        if cfg!(target_os = "windows") {
            assert_eq!(name, "hello.dll");
        } else if cfg!(target_os = "macos") {
            assert_eq!(name, "libhello.dylib");
        } else {
            assert_eq!(name, "libhello.so");
        }
    }

    #[test]
    fn test_library_path() {
        let path = library_path("/tmp/plugins", "hello");
        let name = path.file_name().unwrap().to_str().unwrap();
        if cfg!(target_os = "windows") {
            assert_eq!(name, "hello.dll");
        } else if cfg!(target_os = "macos") {
            assert_eq!(name, "libhello.dylib");
        } else {
            assert_eq!(name, "libhello.so");
        }
    }
}
