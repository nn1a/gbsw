use std::error::Error;
use std::path::Path;
use std::process::{Command, Output};

#[derive(Debug)]
pub struct GitError {
    pub message: String,
    pub command_args: Option<Vec<String>>,
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for GitError {}

pub struct GitCommand {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    dir: Option<String>,
}

#[allow(dead_code)]
impl GitCommand {
    pub fn new(program: &str) -> Self {
        GitCommand {
            program: program.to_string(),
            args: Vec::new(),
            env: Vec::new(),
            dir: None,
        }
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        for arg in args {
            self.args.push(arg.to_string());
        }
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_string(), value.to_string()));
        self
    }

    pub fn dir(mut self, dir: &Path) -> Self {
        self.dir = Some(dir.to_str().unwrap().to_string());
        self
    }

    pub fn run(&self) -> Result<Output, GitError> {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        if let Some(ref dir) = self.dir {
            cmd.current_dir(dir);
        }

        let output = cmd.output().map_err(|e| GitError {
            message: format!("Failed to execute command: {}", e),
            command_args: Some(self.args.clone()),
        })?;

        if !output.status.success() {
            return Err(GitError {
                message: format!("Command exited with non-zero status: {}", output.status),
                command_args: Some(self.args.clone()),
            });
        }

        Ok(output)
    }

    pub fn run_out(&self) -> Result<(), GitError> {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        if let Some(ref dir) = self.dir {
            cmd.current_dir(dir);
        }

        let status = cmd.status().map_err(|e| GitError {
            message: format!("Failed to execute command: {}", e),
            command_args: Some(self.args.clone()),
        })?;

        if !status.success() {
            return Err(GitError {
                message: format!("Command exited with non-zero status: {}", status),
                command_args: Some(self.args.clone()),
            });
        }
        Ok(())
    }

    pub fn run_with_output(&self) -> Result<String, GitError> {
        let output = self.run()?;
        let stdout = String::from_utf8(output.stdout).map_err(|e| GitError {
            message: format!("Failed to parse command output: {}", e),
            command_args: Some(self.args.clone()),
        })?;
        Ok(stdout)
    }
}

pub struct GitCommandBuilder {}

#[allow(dead_code)]
impl GitCommandBuilder {
    pub fn git_version(self) -> GitCommand {
        GitCommand::new("git").arg("--version")
    }

    pub fn git_config_get(key: &str) -> GitCommand {
        GitCommand::new("git").arg("config").arg("--get").arg(key)
    }

    pub fn git_config_set(key: &str, value: &str) -> GitCommand {
        GitCommand::new("git")
            .arg("config")
            .arg("--add")
            .arg(key)
            .arg(value)
    }

    pub fn git_config_unset(key: &str) -> GitCommand {
        GitCommand::new("git").arg("config").arg("--unset").arg(key)
    }

    pub fn git_clone(repo_url: &str, dest: &Path) -> GitCommand {
        GitCommand::new("git")
            .arg("clone")
            .arg(repo_url)
            .arg(dest.to_str().unwrap())
    }

    pub fn git_checkout(branch: &str) -> GitCommand {
        GitCommand::new("git").arg("checkout").arg(branch)
    }

    pub fn git_pull() -> GitCommand {
        GitCommand::new("git").arg("pull")
    }

    pub fn git_push(remote: &str, branch: &str) -> GitCommand {
        GitCommand::new("git").arg("push").arg(remote).arg(branch)
    }
}
