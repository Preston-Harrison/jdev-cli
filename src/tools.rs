use anyhow::{anyhow, Result};
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct Functions {
    repo: Repository,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModifyFileArgs {
    path: String,
    modification: FileModification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileModification {
    Insert {
        start_line: usize,
        content: String,
    },
    Replace {
        start_line: usize,
        end_line: usize,
        content: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LineModification {
    line: usize,
    content: String,
    /// Modification is either deletion or insertion. A replacement is therefore
    /// considered two line modifications.
    is_deletion: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateFileArgs {
    path: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteFileArgs {
    path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadFileArgs {
    path: String,
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

    fn get_abs_path(&self, path: &str) -> Result<PathBuf> {
        Ok(self.repo_path().join(PathBuf::from_str(path)?))
    }

    /// Returns a list of all commited, staged, and untracked files in the repo.
    /// Does not return any ignored files.
    /// Returns files with a `PathBuf` relative to the repo path. For example:
    /// `[".gitignore", "Cargo.toml", "src/main.rs"]`
    pub fn get_all_files(&self) -> Result<Vec<String>> {
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
    /// Returns an Err if the file at path already exists.
    pub fn create_file(&self, args: CreateFileArgs) -> Result<()> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(&args.path);

        if Path::new(&file_path).exists() {
            return Err(anyhow!("File already exists"));
        }

        let mut file = File::create(file_path)?;
        file.write_all(args.content.as_bytes())?;

        Ok(())
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
    pub fn read_file(&self, args: ReadFileArgs) -> Result<String> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(&args.path);
        let file_contents = std::fs::read_to_string(file_path)?;
        Ok(file_contents)
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
        let file_path = repo_path.join(&args.path);
        if !file_path.exists() {
            return Err(anyhow!("File does not exist"));
        }
        std::fs::remove_file(file_path)?;
        Ok(())
    }

    pub fn modify_file(&self, args: ModifyFileArgs) -> Result<Vec<LineModification>> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(&args.path);

        // Ensure the file exists
        if !file_path.exists() {
            return Err(anyhow!("File does not exist"));
        }

        let file = self.read_file(ReadFileArgs {
            path: args.path.clone(),
        })?;
        let mut file_content = file.lines().map(String::from).collect::<Vec<_>>();
        let mut modifications = Vec::new();

        let mut file = File::create(&file_path)?;

        // Depending on the input variant, modify file accordingly
        match args.modification {
            FileModification::Insert {
                start_line,
                content,
            } => {
                let insert_index = start_line.saturating_sub(1); // Convert 1-indexed to 0-indexed
                for (i, line_content) in content.split('\n').enumerate() {
                    file_content.insert(insert_index + i, line_content.to_string());
                    modifications.push(LineModification {
                        line: insert_index + i + 1, // Convert back to 1-indexed
                        content: line_content.into(),
                        is_deletion: false,
                    })
                }
            }
            FileModification::Replace {
                start_line,
                end_line,
                content,
            } => {
                let replace_start = start_line.saturating_sub(1); // Convert 1-indexed to 0-indexed
                let replace_end = end_line.saturating_sub(1);

                // Record deletions
                for i in replace_start..replace_end {
                    if let Some(original_content) = file_content.get(i) {
                        modifications.push(LineModification {
                            line: i + 1, // Convert back to 1-indexed
                            content: original_content.to_string(),
                            is_deletion: true,
                        });
                    }
                }

                // Replace content
                file_content.splice(
                    replace_start..replace_end,
                    content.split('\n').map(String::from),
                );

                // Record insertions
                for (i, line_content) in content.split('\n').enumerate() {
                    modifications.push(LineModification {
                        line: replace_start + i + 1, // Convert back to 1-indexed
                        content: line_content.into(),
                        is_deletion: false,
                    });
                }
            }
        }

        // Write modified content back to the file
        write!(file, "{}", file_content.join("\n"))?;

        Ok(modifications)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Sets up a temporary repository in a temporary directory for testing purposes.
    fn setup_test_repo() -> Result<(Repository, PathBuf)> {
        let temp_dir = tempdir().expect("Failed to create a temporary directory");
        let repo_path = temp_dir.into_path();
        let repo = Repository::init(&repo_path)?;
        Ok((repo, repo_path.into()))
    }

    #[test]
    fn test_modify_file_insert() {
        // Arrange
        let (_repo, repo_path) = setup_test_repo().expect("Should set up the test repo");
        let functions = Functions::new(repo_path.clone()).expect("Functions::new should work");
        let file_path = repo_path.join("test.txt");
        fs::write(&file_path, "Line1\nLine2").expect("Should write to test file");

        // Act
        let modification = functions
            .modify_file(ModifyFileArgs {
                path: file_path
                    .strip_prefix(&repo_path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                modification: FileModification::Insert {
                    start_line: 2,
                    content: "Inserted Line".to_string(),
                },
            })
            .expect("File modification should succeed");

        // Assert
        let file_content = fs::read_to_string(file_path).expect("Should read modified file");
        assert_eq!(
            file_content, "Line1\nInserted Line\nLine2",
            "Content should be inserted correctly"
        );
    }

    #[test]
    fn test_modify_file_replace() {
        // Arrange
        let (_repo, repo_path) = setup_test_repo().expect("Should set up the test repo");
        let functions = Functions::new(repo_path.clone()).expect("Functions::new should work");
        let file_path = repo_path.join("test.txt");
        fs::write(&file_path, "Line1\nLine2\nLine3").expect("Should write to test file");

        // Act
        let modification = functions
            .modify_file(ModifyFileArgs {
                path: file_path
                    .strip_prefix(&repo_path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                modification: FileModification::Replace {
                    start_line: 2,
                    end_line: 3,
                    content: "Replaced Line".to_string(),
                },
            })
            .expect("File modification should succeed");

        // Assert
        let file_content = fs::read_to_string(file_path).expect("Should read modified file");
        assert_eq!(
            file_content, "Line1\nReplaced Line\nLine3",
            "Content should be replaced correctly"
        );
    }

    #[test]
    fn test_modify_file_edge_cases() {
        // This test should cover edge cases such as empty files, inserting at beginning/end, and replacing nothing.

        // Arrange
        let (_repo, repo_path) = setup_test_repo().expect("Should set up the test repo");
        let functions = Functions::new(repo_path.clone()).expect("Functions::new should work");

        // Edge case: Modifying an empty file
        let file_path = repo_path.join("empty.txt");
        fs::write(&file_path, "").expect("Should create an empty test file");
        assert!(
            functions
                .modify_file(ModifyFileArgs {
                    path: file_path
                        .strip_prefix(&repo_path)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                    modification: FileModification::Insert {
                        start_line: 1,
                        content: "Text in empty file".to_string(),
                    }
                })
                .is_ok(),
            "Modifying an empty file should not result in an error"
        );

        // Edge case: Inserting at the beginning of the file
        let file_path = repo_path.join("begin.txt");
        fs::write(&file_path, "Line1\nLine2").expect("Should write to beginning test file");
        let modification_beginning = functions
            .modify_file(ModifyFileArgs {
                path: file_path
                    .strip_prefix(&repo_path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                modification: FileModification::Insert {
                    start_line: 1,
                    content: "Inserted at beginning".to_string(),
                },
            })
            .expect("File modification at the beginning should succeed");
        let file_content_beginning =
            fs::read_to_string(file_path).expect("Should read beginning modified file");
        assert_eq!(
            file_content_beginning, "Inserted at beginning\nLine1\nLine2",
            "Content should be inserted at the beginning correctly"
        );

        // Edge case: Inserting at the end of the file
        let file_path = repo_path.join("end.txt");
        fs::write(&file_path, "Line1\nLine2").expect("Should write to end test file");
        let modification_end = functions
            .modify_file(ModifyFileArgs {
                path: file_path
                    .strip_prefix(&repo_path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                modification: FileModification::Insert {
                    start_line: 3,
                    content: "Appended at end".to_string(),
                },
            })
            .expect("File modification at the end should succeed");
        let file_content_end =
            fs::read_to_string(file_path).expect("Should read end modified file");
        assert_eq!(
            file_content_end, "Line1\nLine2\nAppended at end",
            "Content should be appended at the end correctly"
        );

        // Edge case: Replacing with nothing
        let file_path = repo_path.join("replace_nothing.txt");
        fs::write(&file_path, "Line1\nLine2\nLine3")
            .expect("Should write to replace nothing test file");
        let modification_replace_nothing = functions
            .modify_file(ModifyFileArgs {
                path: file_path
                    .strip_prefix(&repo_path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                modification: FileModification::Replace {
                    start_line: 2,
                    end_line: 3,
                    content: "".to_string(),
                },
            })
            .expect("File modification with replace nothing should succeed");
        let file_content_replace_nothing =
            fs::read_to_string(file_path).expect("Should read replace nothing modified file");
        assert_eq!(
            file_content_replace_nothing, "Line1\n\nLine3",
            "Replacing with nothing should result in an empty line"
        );
    }

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
                .create_file(CreateFileArgs {
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
