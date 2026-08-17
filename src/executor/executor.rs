use tokio::process::Command;

pub struct Executor {
    command: String,
}

impl Executor {
    pub fn new(command: String) -> Self {
        Self { command }
    }

    pub async fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let status = Command::new("sh")
            .arg("-c")
            .arg(self.command.clone())
            .status()
            .await?;

        if !status.success() {
            return Err(format!("[ERROR] Command failed with status: {}", status).into());
        }

        Ok(())
    }
}