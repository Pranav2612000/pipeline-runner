use crate::error::ArtifactError;

use std::fs;
use std::path::Path;

use fs_utils::copy;

#[derive(Clone)]
pub struct ArtifactManager {
    pub workspace: String,
    pub root_dir: String,
}

impl ArtifactManager {
    pub fn new_with_params(workspace: String, root_dir: String) -> Self {
        Self {
            workspace,
            root_dir,
        }
    }

    pub fn save_artifacts(&self, job_name: &str, paths: Vec<&str>) -> Result<(), ArtifactError> {
        if paths.is_empty() {
            return Ok(());
        }

        let job_artifact_dir = format!("{}/{}/{}", self.root_dir, self.workspace, job_name);
        let _ = fs::create_dir_all(job_artifact_dir.clone());

        for path in paths {
            if !fs::exists(path).is_ok() {
                return Err(ArtifactError::ArtifactNotFoundError(path.to_string()));
            }

            if Path::new(path).is_dir() {
                let _path = copy::copy_directory(Path::new(path), job_artifact_dir.clone())
                    .map_err(|e| ArtifactError::ArtifactCopyError(e.to_string()))?;
            } else {
                // TODO: Copy file
            }
        }
        Ok(())
    }
}
