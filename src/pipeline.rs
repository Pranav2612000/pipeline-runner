use std::collections::HashMap;

use tokio::runtime::Runtime;

use crate::error::PipelineError;
use crate::error::PipelineError::{ConfigFileNotReadable, ParsingError, RuntimeError};
use crate::executor::Executor;
use crate::job::JobConfig;

#[derive(Debug, PartialEq)]
pub struct ParserConfig {
    jobs: Vec<JobConfig>,
    stages: Option<Vec<String>>,
}

impl ParserConfig {
    pub fn new_with_params(jobs: Vec<JobConfig>, stages: Option<Vec<String>>) -> Self {
        Self { jobs, stages }
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
        let mut stages = None;
        for (name, job_value) in jobs_value.iter() {
            let serde_yml::Value::String(name) = name else {
                return Err(ParsingError("name should be a string".to_string()));
            };
            if name.as_str() == "stages" {
                let serde_yml::Value::Sequence(stages_val) = job_value else {
                    return Err(ParsingError("stages should be a sequence".to_string()));
                };

                let mut stages_arr = vec![];
                for stages_elem_val in stages_val.iter() {
                    let serde_yml::Value::String(elem) = stages_elem_val else {
                        return Err(ParsingError("stage should be a string".to_string()));
                    };
                    stages_arr.push(elem.to_string());
                }

                stages = Some(stages_arr);
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

            let needs = if let Some(needs_val) = job_value.get("needs") {
                let serde_yml::Value::Sequence(needs_arr) = needs_val else {
                    return Err(ParsingError("needs should be a sequence".to_string()));
                };

                let mut needs = vec![];
                for needs_elem_val in needs_arr.iter() {
                    let serde_yml::Value::String(elem) = needs_elem_val else {
                        return Err(ParsingError("name should be a string".to_string()));
                    };
                    needs.push(elem.to_string());
                }

                Some(needs)
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

            let job = JobConfig::new_with_params(
                name.to_string(),
                image.to_string(),
                stage,
                script,
                needs,
            );
            jobs.push(job);
        }
        Ok(Self::new_with_params(jobs, stages))
    }
}

pub struct Pipeline {
    file_path: String,
}

impl Pipeline {
    pub fn new_with_params(file_path: String) -> Self {
        Self { file_path }
    }
    fn get_jobs_by_stage(jobs: Vec<JobConfig>) -> HashMap<Option<String>, Vec<JobConfig>> {
        let mut jobs_by_stage = HashMap::new();

        for job in jobs {
            let stage = job.stage.clone();
            let entry = jobs_by_stage.entry(stage);
            entry
                .and_modify(|v: &mut Vec<JobConfig>| v.push(job.clone()))
                .or_insert(vec![job.clone()]);
        }

        jobs_by_stage
    }

    fn get_execution_order(jobs: Vec<&JobConfig>) -> Vec<Vec<&JobConfig>> {
        let mut execution_order = vec![];
        let mut jobs_by_name = HashMap::new();
        let mut graph = HashMap::new();
        let mut job_names = vec![];
        for job in jobs {
            let job_name = job.name.clone();
            job_names.push(job_name.clone());
            jobs_by_name.insert(job_name.clone(), job);

            if let Some(ref needs) = job.needs {
                for deps in needs {
                    let entry = graph.entry(job_name.clone());
                    entry
                        .and_modify(|d: &mut Vec<String>| d.push(deps.clone()))
                        .or_insert(vec![deps.clone()]);
                }
            }
        }

        while jobs_by_name.len() != 0 {
            let mut runnable_jobs = vec![];

            for curr_job_name in jobs_by_name.keys() {
                let deps = graph.get(curr_job_name);

                // This job has no dependencies, so we can run it now
                if deps.is_none() {
                    runnable_jobs.push(curr_job_name.clone());
                } else if let Some(deps) = deps {
                    if deps.len() == 0 {
                        runnable_jobs.push(curr_job_name.clone());
                    }
                }
            }

            let mut parallel_jobs = vec![];
            for runnable_job in runnable_jobs.iter() {
                let job = jobs_by_name.remove(runnable_job).expect("should exist");
                parallel_jobs.push(job);
            }

            parallel_jobs.sort_by(|a, b| a.name.cmp(&b.name));
            execution_order.push(parallel_jobs);

            for curr_job_name in job_names.iter() {
                let deps = graph.get_mut(curr_job_name);

                // Remove the runnable jobs from deps of other jobs
                if let Some(deps) = deps {
                    for runnable_job in runnable_jobs.iter() {
                        let idx = deps.iter().position(|d| d == runnable_job);
                        if let Some(idx) = idx {
                            deps.remove(idx);
                        }
                    }
                }
            }
        }

        execution_order
    }

    async fn execute_job(job: JobConfig) {
        let job_name = job.name.clone();
        if let Err(e) = tokio::task::spawn_blocking(|| {
            let executor = Executor::new_with_params(None);
            let job = job;
            if let Err(err) = executor.run(&job) {
                println!("{} job failed| {}", job.name, err);
            }
        })
        .await
        {
            println!("{} job failed| {}", job_name, e);
        }
    }

    async fn run_internal(config: ParserConfig) {
        let jobs = config.jobs.clone();
        let jobs_by_stage = Self::get_jobs_by_stage(config.jobs);
        if let Some(_stages) = config.stages {
            let execution_order = Self::get_execution_order(jobs.iter().collect());
            for parallel_jobs in execution_order {
                let mut jobs_set = tokio::task::JoinSet::new();
                for job in parallel_jobs {
                    jobs_set.spawn(Self::execute_job(job.clone()));
                }
                jobs_set.join_all().await;
            }
        } else {
            if let Some(jobs) = jobs_by_stage.get(&None) {
                println!("Executing without a stage");
                let mut jobs_set = tokio::task::JoinSet::new();

                for job in jobs {
                    jobs_set.spawn(Self::execute_job(job.clone()));
                }
                jobs_set.join_all().await;
            };
        }
    }

    pub fn run(&self) -> Result<(), PipelineError> {
        let rt = Runtime::new().map_err(|e| RuntimeError(e.to_string()))?;
        let config = ParserConfig::parse_from_file(self.file_path.as_str())?;
        rt.block_on(async { Self::run_internal(config).await });
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
  needs:
    - unit-tests
    - integration-tests
  script:
    - echo "Building application..."
    - python --version
    - pip install --quiet build
    - echo "Build complete!"
        "#;
        let parser_config = ParserConfig::parse_str(config).expect("parsing should suceed");
        assert_eq!(
            parser_config,
            ParserConfig::new_with_params(
                vec![JobConfig {
                    name: "build-job".to_string(),
                    image: "python:3.11".to_string(),
                    script: vec![
                        "echo \"Building application...\"".to_string(),
                        "python --version".to_string(),
                        "pip install --quiet build".to_string(),
                        "echo \"Build complete!\"".to_string(),
                    ],
                    stage: None,
                    needs: Some(vec![
                        "unit-tests".to_string(),
                        "integration-tests".to_string()
                    ]),
                }],
                None
            )
        );
    }

    #[test]
    fn test_parse_from_file() {
        let file_path = "samples/simple-single-job.yml";
        let parser_config =
            ParserConfig::parse_from_file(file_path).expect("parsing should suceed");
        assert_eq!(
            parser_config,
            ParserConfig::new_with_params(
                vec![JobConfig {
                    name: "build-job".to_string(),
                    image: "python:3.11".to_string(),
                    script: vec![
                        "echo \"Building application...\"".to_string(),
                        "python --version".to_string(),
                        "pip install --quiet build".to_string(),
                        "echo \"Build complete!\"".to_string(),
                    ],
                    stage: None,
                    needs: None,
                }],
                None
            )
        );
    }

    #[test]
    fn test_execution_order() {
        fn create_job_with_deps(job_name: String, deps: Option<Vec<String>>) -> JobConfig {
            JobConfig {
                name: job_name,
                image: "python:3.11".to_string(),
                script: vec![
                    "echo \"Building application...\"".to_string(),
                    "python --version".to_string(),
                    "pip install --quiet build".to_string(),
                    "echo \"Build complete!\"".to_string(),
                ],
                stage: None,
                needs: deps,
            }
        }

        let build_job = create_job_with_deps("build".to_string(), None);
        let integration_test_job = create_job_with_deps(
            "integration_test".to_string(),
            Some(vec!["build".to_string()]),
        );
        let unit_test_job =
            create_job_with_deps("unit_test".to_string(), Some(vec!["build".to_string()]));
        let jobs = vec![&integration_test_job, &build_job, &unit_test_job];

        assert_eq!(
            Pipeline::get_execution_order(jobs),
            vec![
                vec![&build_job],
                vec![&integration_test_job, &unit_test_job]
            ]
        );
    }
}
