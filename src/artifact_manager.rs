use crate::error::ArtifactError;

use std::fs;
use std::path::Path;

use fs_utils::copy;
use glob::glob;

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

    fn get_artifact_dir_for_job(&self, job_name: &str) -> String {
        format!("{}/{}/{}", self.root_dir, self.workspace, job_name)
    }

    // Utility function to support copying both files and directories
    // TODO: Copy file
    fn copy_path(src: &str, dest: &str) -> Result<(), ArtifactError> {
        let src_path = Path::new(src);
        if src_path.is_dir() {
            let _path = copy::copy_directory(src_path, dest)
                .map_err(|e| ArtifactError::ArtifactCopyError(e.to_string()))?;
        } else {
            // TODO: Copy file
        }

        Ok(())
    }

    pub fn save_artifacts(&self, job_name: &str, paths: Vec<&str>) -> Result<(), ArtifactError> {
        if paths.is_empty() {
            return Ok(());
        }

        let job_artifact_dir = self.get_artifact_dir_for_job(job_name);
        let _ = fs::create_dir_all(job_artifact_dir.clone());

        for path in paths {
            if !fs::exists(path).is_ok() {
                return Err(ArtifactError::ArtifactNotFoundError(path.to_string()));
            }
            Self::copy_path(path, job_artifact_dir.as_str())?;
        }

        Ok(())
    }

    // Loads all artifacts created by 'job_name'
    pub fn load_artifacts(
        &self,
        from_job_name: &str,
        to_job_name: &str,
    ) -> Result<(), ArtifactError> {
        let job_artifact_dir = self.get_artifact_dir_for_job(from_job_name);
        if !fs::exists(job_artifact_dir.as_str()).is_ok() {
            return Err(ArtifactError::ArtifactNotFoundError(
                job_artifact_dir.clone(),
            ));
        }

        // TODO: Improve copying of files
        for entry in glob(format!("{}/*", job_artifact_dir).as_str())
            .map_err(|e| ArtifactError::ArtifactNotFoundError(e.to_string()))?
        {
            if let Ok(file_path) = entry {
                let dest = format!("{}/{}", self.workspace, to_job_name);
                let _ = fs::create_dir_all(dest.clone());
                Self::copy_path(
                    file_path
                        .to_str()
                        .ok_or(ArtifactError::ArtifactNotFoundError(
                            "could not get file name".to_string(),
                        ))?,
                    dest.as_str(),
                )?;
            }
        }

        Ok(())
    }
}
