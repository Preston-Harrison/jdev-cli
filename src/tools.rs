use anyhow::{anyhow, Result};
use git2::{Repository, StatusOptions};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Functions {
    repo: Repository,
}

pub enum FileModificationInput<'a> {
    Insert {
        start_line: usize,
        content: &'a str,
    },
    Replace {
        start_line: usize,
        end_line: usize,
        content: &'a str,
    },
}

pub struct FileModification {
    path: PathBuf,
    modifications: Vec<LineModification>,
}

pub struct LineModification {
    line: usize,
    content: String,
    /// Modification is either deletion or insertion. A replacement is therefore
    /// considered two line modifications.
    is_deletion: bool,
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
    pub fn get_all_files(&self) -> Result<Vec<PathBuf>> {
        let mut status_options = StatusOptions::new();
        status_options.include_ignored(false);
        status_options.include_untracked(true);
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
                files.push(path);
            }
        }

        Ok(files)
    }

    /// Creates a new file and echoes `content` into the file.
    /// `path` is relative to the repo path.
    /// Returns an Err if the file at path already exists.
    pub fn create_file(&self, path: PathBuf, content: &str) -> Result<()> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(&path);

        if Path::new(&file_path).exists() {
            return Err(anyhow!("File already exists"));
        }

        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;

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
    pub fn read_file(&self, path: PathBuf) -> Result<String> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(&path);
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
    pub fn delete_file(&self, path: PathBuf) -> Result<()> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(&path);
        if !file_path.exists() {
            return Err(anyhow!("File does not exist"));
        }
        std::fs::remove_file(file_path)?;
        Ok(())
    }

    pub fn modify_file(&self, path: PathBuf, input: FileModificationInput) -> Result<FileModification> {
        let repo_path = self.repo_path();
        let file_path = repo_path.join(&path);

        // Ensure the file exists
        if !file_path.exists() {
            return Err(anyhow!("File does not exist"));
        }

        let file = self.read_file(path.clone())?;
        let mut file_content = file.lines().collect::<Vec<_>>();
        let mut modifications = Vec::new();

        let mut file = File::create(&file_path)?;

        // Depending on the input variant, modify file accordingly
        match input {
            FileModificationInput::Insert { start_line, content } => {
                let insert_index = start_line.saturating_sub(1); // Convert 1-indexed to 0-indexed
                for (i, line_content) in content.split('\n').enumerate() {
                    file_content.insert(insert_index + i, line_content);
                    modifications.push(LineModification {
                        line: insert_index + i + 1, // Convert back to 1-indexed
                        content: line_content.into(),
                        is_deletion: false
                    })
                }
            },
            FileModificationInput::Replace { start_line, end_line, content } => {
                let replace_start = start_line.saturating_sub(1); // Convert 1-indexed to 0-indexed
                let replace_end = end_line;

                // Record deletions
                for i in replace_start..replace_end {
                    if let Some(original_content) = file_content.get(i) {
                        modifications.push(LineModification {
                            line: i + 1, // Convert back to 1-indexed
                            content: original_content.to_string(),
                            is_deletion: true
                        });
                    }
                }

                // Replace content
                file_content.splice(
                    replace_start..replace_end,
                    content.split('\n')
                );

                // Record insertions
                for (i, line_content) in content.split('\n').enumerate() {
                    modifications.push(LineModification {
                        line: replace_start + i + 1, // Convert back to 1-indexed
                        content: line_content.into(),
                        is_deletion: false
                    });
                }
            }
        }

        // Write modified content back to the file
        write!(file, "{}", file_content.join("\n"))?;

        Ok(FileModification {
            path,
            modifications
        })
    }
}
