#[derive(Debug, PartialEq)]
pub struct JobConfig {
    pub name: String,
    pub image: String,
    pub stage: String,
    pub script: Vec<String>,
}

impl JobConfig {
    pub fn new_with_params(
        name: String,
        image: String,
        stage: String,
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
