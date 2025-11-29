#[derive(Debug, PartialEq, Clone)]
pub struct JobConfig {
    pub name: String,
    pub image: String,
    pub stage: Option<String>,
    pub script: Vec<String>,
}

impl JobConfig {
    pub fn new_with_params(
        name: String,
        image: String,
        stage: Option<String>,
        script: Vec<String>,
    ) -> Self {
        Self {
            name,
            image,
            stage,
            script,
        }
    }
}
