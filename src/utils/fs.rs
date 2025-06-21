//! File system utilities for the trading system

use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use anyhow::{Context, Result};

/// Ensures a directory exists, creating it if necessary
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).with_context(|| format!("Failed to create directory: {:?}", path))?;
    } else if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("Path exists but is not a directory: {:?}", path),
        ).into());
    }
    Ok(())
}

/// Gets the application's data directory, creating it if it doesn't exist
pub fn app_data_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find data directory"))?
        .join("algotraderv2");
    
    ensure_dir(&dir)?;
    Ok(dir)
}

/// Gets the application's config directory, creating it if it doesn't exist
pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find config directory"))?
        .join("algotraderv2");
    
    ensure_dir(&dir)?;
    Ok(dir)
}

/// Reads a file to a string with context about the operation
pub fn read_file<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))
}

/// Writes a string to a file, creating parent directories if needed
pub fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    
    fs::write(path, contents)
        .with_context(|| format!("Failed to write file: {}", path.display()))
}

/// Lists all files in a directory with a specific extension
pub fn list_files_with_extension<P: AsRef<Path>>(
    dir: P,
    extension: &str,
) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    let mut files = Vec::new();
    
    if !dir.exists() {
        return Ok(files);
    }
    
    for entry in fs::read_dir(dir).with_context(|| format!("Failed to read directory: {:?}", dir))? {
        let entry = entry.with_context(|| "Failed to read directory entry")?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == extension {
                    files.push(path);
                }
            }
        }
    }
    
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;
    
    #[test]
    fn test_ensure_dir() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path().join("test_dir");
        
        // Test creating a new directory
        ensure_dir(&test_dir).unwrap();
        assert!(test_dir.exists());
        assert!(test_dir.is_dir());
        
        // Test that it doesn't fail if directory already exists
        ensure_dir(&test_dir).unwrap();
    }
    
    #[test]
    fn test_read_write_file() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let test_content = "test content";
        
        // Test writing to a file
        write_file(&test_file, test_content).unwrap();
        assert!(test_file.exists());
        
        // Test reading from a file
        let content = read_file(&test_file).unwrap();
        assert_eq!(content, test_content);
    }
    
    #[test]
    fn test_list_files_with_extension() {
        let temp_dir = tempdir().unwrap();
        
        // Create test files
        let files = ["file1.txt", "file2.txt", "file3.log"];
        for &file in &files {
            let path = temp_dir.path().join(file);
            let mut f = File::create(&path).unwrap();
            write!(f, "test").unwrap();
        }
        
        // Test listing .txt files
        let txt_files = list_files_with_extension(temp_dir.path(), "txt").unwrap();
        assert_eq!(txt_files.len(), 2);
        
        // Test listing .log files
        let log_files = list_files_with_extension(temp_dir.path(), "log").unwrap();
        assert_eq!(log_files.len(), 1);
        
        // Test non-existent extension
        let md_files = list_files_with_extension(temp_dir.path(), "md").unwrap();
        assert!(md_files.is_empty());
    }
}
