use anyhow::{anyhow, Result};
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

pub struct Functions {
    repo: Repository,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WriteFileArgs {
    pub path: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteFileArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MoveFileArgs {
    pub source_path: String,
    pub destination_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadFileArgs {
    pub path: String,
}

impl Functions {
    pub fn new(repo_path: PathBuf) -> Result<Self> {
        let repo = Repository::open(repo_path)?;
        Ok(Self { repo })
    }

    /// Gets the filesystem path of the repository, not including /.git/
    fn repo_path(&self) -> PathBuf {
        // Here path is path/to/repo/.git/
        let mut path = self.repo.path().to_path_buf();
        path.pop(); // Removes /.git/
        path
    }

    /// Returns a list of all commited, staged, and untracked files in the repo.
    /// Does not return any ignored files.
    /// Returns files with a `PathBuf` relative to the repo path. For example:
    /// `[".gitignore", "Cargo.toml", "src/main.rs"]`
    pub fn list_files(&self) -> Result<Vec<String>> {
        let mut status_options = StatusOptions::new();
        status_options.include_ignored(false);
        status_options.include_untracked(true);
        status_options.include_unmodified(true);
        let statuses = self.repo.statuses(Some(&mut status_options))?;
        let repo_path = self.repo_path();

        let mut files = Vec::with_capacity(statuses.len());
        for entry in statuses.iter() {
            let Some(file_path) = entry.path() else {
                print!("Found file path with invalid utf8 name");
                continue;
            };
            let path = PathBuf::from(file_path);
            if repo_path.join(&path).exists() {
                files.push(file_path.to_string());
            }
        }

        Ok(files)
    }

    /// Creates a new file and echoes `content` into the file.
    /// `path` is relative to the repo path.
    /// Returns the old file.
    pub fn write_file(&self, args: WriteFileArgs) -> Result<Option<String>> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(&args.path);

        if let Some(dir_path) = file_path.parent() {
            fs::create_dir_all(dir_path)?;
        }

        let old_file = self.read_file(ReadFileArgs { path: args.path })?;
        let mut file = File::create(file_path)?;
        file.write_all(args.content.as_bytes())?;

        Ok(old_file)
    }

    /// Reads the contents of a file from the repository.
    ///
    /// `path` is a `PathBuf` relative to the repository's root directory.
    /// If the file exists, the function retrieves the content as a string.
    ///
    /// # Arguments
    /// * `path` - The relative path to the file that should be read.
    ///
    /// # Returns
    /// A `Result` containing the file contents as a `String`, or
    /// an error if there is a problem reading the file.
    pub fn read_file(&self, args: ReadFileArgs) -> Result<Option<String>> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(args.path);
        if !file_path.exists() {
            return Ok(None);
        }
        let file_contents = std::fs::read_to_string(file_path)?;
        Ok(Some(file_contents))
    }

    /// Deletes a file in the repository.
    ///
    /// `path` is a `PathBuf` relative to the repository's root directory.
    /// This method will remove the file from the filesystem.
    ///
    /// # Arguments
    /// * `path` - The relative path to the file that should be deleted.
    ///
    /// # Returns
    /// A `Result` which is an Ok(()) if the file was successfully deleted,
    /// or an error if the file doesn't exist or there is a problem deleting the file.
    pub fn delete_file(&self, args: DeleteFileArgs) -> Result<()> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(args.path);
        if !file_path.exists() {
            return Err(anyhow!("File does not exist"));
        }
        std::fs::remove_file(file_path)?;
        Ok(())
    }

    /// Moves a file from one path to another within the repository.
    ///
    /// The `source_path` and `destination_path` are both relative to the
    /// repository's root directory. Both paths must be within the repository.
    ///
    /// # Arguments
    /// * `args` - A `MoveFileArgs` struct containing:
    ///   * `source_path`: The relative path to the existing file that will be moved.
    ///   * `destination_path`: The new relative path for the file after moving.
    ///
    /// # Returns
    /// A `Result` containing `()`, or an error if the file cannot be moved.
    pub fn move_file(&self, args: MoveFileArgs) -> Result<()> {
        let repo_path = self.repo_path();
        let source_file_path = repo_path.join(&args.source_path);
        let destination_file_path = repo_path.join(&args.destination_path);

        // Ensure the source file exists
        if !source_file_path.exists() {
            return Err(anyhow!("Source file does not exist"));
        }

        // Ensure the destination doesn't already exist
        if destination_file_path.exists() {
            return Err(anyhow!("Destination file already exists"));
        }

        if let Some(dir_path) = destination_file_path.parent() {
            fs::create_dir_all(dir_path)?;
        }
        fs::rename(source_file_path, destination_file_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Sets up a temporary repository in a temporary directory for testing purposes.
    // fn setup_test_repo() -> Result<(Repository, PathBuf)> {
    //     let temp_dir = tempdir().expect("Failed to create a temporary directory");
    //     let repo_path = temp_dir.into_path();
    //     let repo = Repository::init(&repo_path)?;
    //     Ok((repo, repo_path.into()))
    // }

    #[test]
    fn test_file_creation_and_deletion() {
        let temp_dir = tempdir().expect("Failed to create a temporary directory");
        let repo_path = temp_dir.into_path();
        let _repo = Repository::init(&repo_path).expect("Failed to initialize a repository");

        let functions = Functions::new(repo_path.clone()).expect("Functions::new should work");

        let file_to_create = repo_path.join("temp_file.txt");
        let file_content = "Temporary file contents";

        // Test file creation
        assert!(
            functions
                .write_file(WriteFileArgs {
                    content: file_content.to_string(),
                    path: file_to_create
                        .strip_prefix(&repo_path)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                })
                .is_ok(),
            "Should be able to create a new file"
        );

        // Test file existence after creation
        assert!(file_to_create.exists(), "Newly created file should exist");

        // Test file deletion
        assert!(
            functions
                .delete_file(DeleteFileArgs {
                    path: file_to_create
                        .strip_prefix(&repo_path)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                })
                .is_ok(),
            "Should be able to delete the file"
        );

        // Test file existence after deletion
        assert!(
            !file_to_create.exists(),
            "File should not exist after deletion"
        );
    }
}
