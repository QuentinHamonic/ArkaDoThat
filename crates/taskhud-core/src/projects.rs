use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{TodoError, atomic_write};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
}

impl Project {
    pub fn todo_path(&self) -> PathBuf {
        self.root.join("TODO.md")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRegistry {
    pub active_project_id: Option<String>,
    pub projects: Vec<Project>,
}

impl ProjectRegistry {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, TodoError> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path).map_err(|source| TodoError::Read {
            path: path.clone(),
            source,
        })?;
        serde_json::from_str(&content).map_err(|source| TodoError::InvalidConfig {
            path,
            message: source.to_string(),
        })
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), TodoError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| TodoError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let content =
            serde_json::to_string_pretty(self).map_err(|source| TodoError::InvalidConfig {
                path: path.clone(),
                message: source.to_string(),
            })?;
        atomic_write(&path, &format!("{content}\n"))
    }

    pub fn project(&self, project_id: &str) -> Result<&Project, TodoError> {
        self.projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| TodoError::ProjectNotFound(project_id.to_owned()))
    }

    pub fn active_project(&self) -> Option<&Project> {
        self.active_project_id
            .as_deref()
            .and_then(|id| self.projects.iter().find(|project| project.id == id))
    }
}
