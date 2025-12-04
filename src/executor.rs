use std::io::{BufRead, BufReader};

use crate::artifact_manager::ArtifactManager;
use crate::error::PipelineError;
use crate::error::PipelineError::ExecutionError;
use crate::job::JobConfig;

pub struct Executor {
    workspace: String,
}

const DEFAULT_WORKSPACE: &'static str = ".";

impl Executor {
    pub fn new_with_params(workspace: Option<&str>) -> Self {
        Self {
            workspace: workspace.unwrap_or(DEFAULT_WORKSPACE).to_string(),
        }
    }

    pub fn run(
        &self,
        job: &JobConfig,
        artifact_manager: &ArtifactManager,
    ) -> Result<(), PipelineError> {
        println!("Running job {:?}", job.name);
        println!("Image {:?}", job.image);

        let merged_script = job.script.join(" && ");

        let cmd = vec![
            "docker".to_string(),
            "run".to_string(),
            "--rm".to_string(),
            "-v".to_string(),
            format!("{}:/workspace", self.workspace),
            "-w".to_string(),
            "/workspace".to_string(),
            job.image.clone(),
            "sh".to_string(),
            "-c".to_string(),
            merged_script,
        ];

        let mut process = subprocess::Popen::create(
            cmd.as_slice(),
            subprocess::PopenConfig {
                stdout: subprocess::Redirection::Pipe,
                stderr: subprocess::Redirection::Merge,
                ..Default::default()
            },
        )
        .map_err(|e| ExecutionError(job.name.clone(), e.to_string()))?;

        let out_fd = process
            .stdout
            .as_ref()
            .expect("output file descriptor should exist");

        let reader = BufReader::new(out_fd);

        for line in reader.lines() {
            if let Ok(line) = line {
                println!("[{}] | {}", &job.name, line);
            } else {
                println!("Error reading output. Program may exit unexpectedly");
            }
        }

        process
            .wait()
            .map_err(|e| ExecutionError(job.name.clone(), e.to_string()))?;

        match process.exit_status() {
            None => println!(
                "Unexpected error: Process failed to terminate. You may need to manually kill it"
            ),
            Some(exit_status) => match exit_status {
                subprocess::ExitStatus::Exited(code) => {
                    if code == 0 {
                        println!("[{}] SUCCESS", job.name.clone());

                        if let Some(ref artifacts) = job.artifacts {
                            let artifacts: Vec<String> = artifacts
                                .iter()
                                .map(|a| format!("{}/{}", self.workspace, a))
                                .collect();
                            let artifacts = artifacts.iter().map(|a| a.as_str()).collect();
                            artifact_manager
                                .save_artifacts(job.name.as_str(), artifacts)
                                .map_err(|e| PipelineError::ArtifactError(e))?;
                        }
                    } else {
                        println!("[{}] FAILURE CODE: {}", job.name.clone(), code);
                    }
                }
                subprocess::ExitStatus::Signaled(signal_num) => {
                    println!("[{}] KILLED SIGNAL: {}", job.name.clone(), signal_num);
                }
                _ => println!("Unknown exit status"),
            },
        }

        Ok(())
    }
}
