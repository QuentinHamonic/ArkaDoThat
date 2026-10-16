use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{TodoError, atomic_write};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "operation", rename_all = "camelCase")]
pub enum TaskChange {
    Add {
        title: String,
        after_task_id: Option<String>,
    },
    Update {
        task_id: String,
        title: Option<String>,
        checked: Option<bool>,
    },
    Delete {
        task_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProposalStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskProposal {
    pub id: String,
    pub project_id: String,
    pub base_version: String,
    pub reason: String,
    pub changes: Vec<TaskChange>,
    pub status: ProposalStatus,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProposalFile {
    proposals: Vec<TaskProposal>,
}

#[derive(Debug, Clone)]
pub struct ProposalStore {
    path: PathBuf,
    proposals: Vec<TaskProposal>,
}

impl ProposalStore {
    pub fn open(project_root: impl AsRef<Path>) -> Result<Self, TodoError> {
        let path = project_root
            .as_ref()
            .join(".taskhud")
            .join("proposals.json");
        if !path.exists() {
            return Ok(Self {
                path,
                proposals: Vec::new(),
            });
        }
        let content = fs::read_to_string(&path).map_err(|source| TodoError::Read {
            path: path.clone(),
            source,
        })?;
        let file: ProposalFile =
            serde_json::from_str(&content).map_err(|source| TodoError::InvalidConfig {
                path: path.clone(),
                message: source.to_string(),
            })?;
        Ok(Self {
            path,
            proposals: file.proposals,
        })
    }

    pub fn list(&self) -> &[TaskProposal] {
        &self.proposals
    }

    pub fn get(&self, proposal_id: &str) -> Result<&TaskProposal, TodoError> {
        self.proposals
            .iter()
            .find(|proposal| proposal.id == proposal_id)
            .ok_or_else(|| TodoError::ProposalNotFound(proposal_id.to_owned()))
    }

    pub fn add(
        &mut self,
        project_id: String,
        base_version: String,
        reason: String,
        changes: Vec<TaskChange>,
    ) -> Result<TaskProposal, TodoError> {
        if let Some(existing) = self.proposals.iter().find(|proposal| {
            proposal.status == ProposalStatus::Pending
                && proposal.project_id == project_id
                && proposal.base_version == base_version
                && proposal.reason == reason
                && proposal.changes == changes
        }) {
            return Ok(existing.clone());
        }

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or_default();
        let proposal = TaskProposal {
            id: format!("proposal-{created_at}-{}", self.proposals.len()),
            project_id,
            base_version,
            reason,
            changes,
            status: ProposalStatus::Pending,
            created_at,
        };
        self.proposals.push(proposal.clone());
        self.save()?;
        Ok(proposal)
    }

    fn save(&self) -> Result<(), TodoError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| TodoError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let content = serde_json::to_string_pretty(&ProposalFile {
            proposals: self.proposals.clone(),
        })
        .map_err(|source| TodoError::InvalidConfig {
            path: self.path.clone(),
            message: source.to_string(),
        })?;
        atomic_write(&self.path, &format!("{content}\n"))
    }
}
