use std::collections::HashMap;

use crate::error::PipelineError;
use crate::error::PipelineError::{ConfigFileNotReadable, ParsingError};
use crate::executor::Executor;
use crate::job::JobConfig;

#[derive(Debug, PartialEq)]
pub struct ParserConfig {
    jobs: Vec<JobConfig>,
}

impl ParserConfig {
    pub fn new_with_params(jobs: Vec<JobConfig>) -> Self {
        Self { jobs }
    }

    pub fn parse_from_file(file_path: &str) -> Result<Self, PipelineError> {
        let config_str = std::fs::read_to_string(file_path)
            .map_err(|e| ConfigFileNotReadable(file_path.to_string(), e.to_string()))?;
        Self::parse_str(config_str.as_str())
    }

    pub fn parse_str(config_str: &str) -> Result<Self, PipelineError> {
        let config_yaml = serde_yml::from_str::<serde_yml::Value>(config_str)
            .map_err(|e| ParsingError(e.to_string()))?;
        let serde_yml::Value::Mapping(jobs_value) = config_yaml else {
            return Err(ParsingError("Expected a list of jobs".to_string()));
        };

        let mut jobs = Vec::new();
        for (name, job_value) in jobs_value.iter() {
            let serde_yml::Value::String(name) = name else {
                return Err(ParsingError("name should be a string".to_string()));
            };
            if name.as_str() == "stages" {
                continue;
            }

            let serde_yml::Value::Mapping(job_value) = job_value else {
                return Err(ParsingError("Each job should be a map".to_string()));
            };

            let serde_yml::Value::String(image) =
                job_value.get("image").unwrap_or(&serde_yml::Value::Null)
            else {
                return Err(ParsingError("name should be a string".to_string()));
            };

            let stage = if let Some(stage) = job_value.get("stage") {
                let serde_yml::Value::String(stage) = stage else {
                    return Err(ParsingError("stage should be a string".to_string()));
                };

                Some(stage.to_string())
            } else {
                None
            };

            let mut script = vec![];
            let serde_yml::Value::Sequence(script_val) =
                job_value.get("script").unwrap_or(&serde_yml::Value::Null)
            else {
                return Err(ParsingError("name should be a string".to_string()));
            };
            for script_elem_val in script_val.iter() {
                let serde_yml::Value::String(elem) = script_elem_val else {
                    return Err(ParsingError("name should be a string".to_string()));
                };
                script.push(elem.to_string());
            }

            let job =
                JobConfig::new_with_params(name.to_string(), image.to_string(), stage, script);
            jobs.push(job);
        }
        Ok(Self { jobs })
    }
}

pub struct Pipeline {
    file_path: String,
}

impl Pipeline {
    pub fn new_with_params(file_path: String) -> Self {
        Self { file_path }
    }
    fn get_jobs_by_stage(jobs: Vec<JobConfig>) -> HashMap<Option<String>, JobConfig> {
        let mut jobs_by_stage = HashMap::new();

        for job in jobs {
            jobs_by_stage.insert(job.stage.clone(), job);
        }

        jobs_by_stage
    }
    pub fn run(&self) -> Result<(), PipelineError> {
        let config = ParserConfig::parse_from_file(self.file_path.as_str())?;

        let executor = Executor::new_with_params(None);
        let jobs_by_stage = Self::get_jobs_by_stage(config.jobs);
        for (stage, job) in jobs_by_stage {
            if let Some(stage) = stage {
                println!("Executing {}", stage);
            }

            if let Err(err) = executor.run(&job) {
                println!("{} job failed| {}", job.name, err);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_str() {
        let config = r#"
build-job:
  image: python:3.11
  script:
    - echo "Building application..."
    - python --version
    - pip install --quiet build
    - echo "Build complete!"
        "#;
        let parser_config = ParserConfig::parse_str(config).expect("parsing should suceed");
        assert_eq!(
            parser_config,
            ParserConfig::new_with_params(vec![JobConfig {
                name: "build-job".to_string(),
                image: "python:3.11".to_string(),
                script: vec![
                    "echo \"Building application...\"".to_string(),
                    "python --version".to_string(),
                    "pip install --quiet build".to_string(),
                    "echo \"Build complete!\"".to_string(),
                ],
            }])
        );
    }

    #[test]
    fn test_parse_from_file() {
        let file_path = "samples/simple-single-job.yml";
        let parser_config =
            ParserConfig::parse_from_file(file_path).expect("parsing should suceed");
        assert_eq!(
            parser_config,
            ParserConfig::new_with_params(vec![JobConfig {
                name: "build-job".to_string(),
                image: "python:3.11".to_string(),
                script: vec![
                    "echo \"Building application...\"".to_string(),
                    "python --version".to_string(),
                    "pip install --quiet build".to_string(),
                    "echo \"Build complete!\"".to_string(),
                ],
            }])
        );
    }
}
