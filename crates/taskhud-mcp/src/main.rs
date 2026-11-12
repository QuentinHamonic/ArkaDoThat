use std::env;
use std::path::PathBuf;

use anyhow::Result;
use rmcp::{
    ServiceExt, handler::server::wrapper::Parameters, schemars::JsonSchema, tool, tool_router,
    transport::stdio,
};
use serde::{Deserialize, Serialize};
use taskhud_core::{
    Project, ProjectRegistry, ProposalStore, TaskChange, TodoDocument, TodoError, TodoSnapshot,
};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ProjectIdArgs {
    #[schemars(description = "Identifier of the project registered in TaskHUD")]
    project_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ProposalIdArgs {
    #[schemars(description = "Identifier of the project registered in TaskHUD")]
    project_id: String,
    #[schemars(description = "Proposal identifier")]
    proposal_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "camelCase")]
enum TaskChangeInput {
    Add {
        title: String,
        #[schemars(rename = "afterTaskId")]
        #[serde(rename = "afterTaskId", alias = "after_task_id")]
        after_task_id: Option<String>,
    },
    Update {
        #[schemars(rename = "taskId")]
        #[serde(rename = "taskId", alias = "task_id")]
        task_id: String,
        title: Option<String>,
        checked: Option<bool>,
    },
    Delete {
        #[schemars(rename = "taskId")]
        #[serde(rename = "taskId", alias = "task_id")]
        task_id: String,
    },
}

impl From<TaskChangeInput> for TaskChange {
    fn from(input: TaskChangeInput) -> Self {
        match input {
            TaskChangeInput::Add {
                title,
                after_task_id,
            } => Self::Add {
                title,
                after_task_id,
            },
            TaskChangeInput::Update {
                task_id,
                title,
                checked,
            } => Self::Update {
                task_id,
                title,
                checked,
            },
            TaskChangeInput::Delete { task_id } => Self::Delete { task_id },
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ProposeTaskChangesArgs {
    #[schemars(description = "Identifier of the project registered in TaskHUD")]
    project_id: String,
    #[schemars(description = "SHA-256 version of the TODO.md read before the proposal")]
    base_version: String,
    #[schemars(description = "Why these changes are needed")]
    reason: String,
    #[schemars(description = "Elementary changes to submit for human approval")]
    changes: Vec<TaskChangeInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ProposeTasksArgs {
    #[schemars(
        description = "Project name or identifier; the active project is used when omitted"
    )]
    project: Option<String>,
    #[schemars(description = "Why these tasks are proposed")]
    reason: String,
    #[schemars(description = "Titles of the tasks to add")]
    tasks: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSummary {
    id: String,
    name: String,
    root: PathBuf,
    todo_path: PathBuf,
    todo_exists: bool,
}

#[derive(Clone)]
struct TaskHudMcp {
    registry_path: PathBuf,
}

impl TaskHudMcp {
    fn registry(&self) -> Result<ProjectRegistry, TodoError> {
        ProjectRegistry::load(&self.registry_path)
    }

    fn project(&self, project_id: &str) -> Result<Project, TodoError> {
        self.registry()?.project(project_id).cloned()
    }

    fn project_selector(&self, selector: Option<&str>) -> Result<Project, TodoError> {
        let registry = self.registry()?;
        if let Some(selector) = selector {
            return registry
                .projects
                .iter()
                .find(|project| {
                    project.id == selector || project.name.eq_ignore_ascii_case(selector)
                })
                .cloned()
                .ok_or_else(|| TodoError::ProjectNotFound(selector.to_owned()));
        }
        registry
            .active_project()
            .cloned()
            .ok_or_else(|| TodoError::ProjectNotFound("active".to_owned()))
    }

    fn summary(project: &Project) -> ProjectSummary {
        let todo_path = project.todo_path();
        ProjectSummary {
            id: project.id.clone(),
            name: project.name.clone(),
            root: project.root.clone(),
            todo_exists: todo_path.is_file(),
            todo_path,
        }
    }

    fn json<T: Serialize>(value: &T) -> String {
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|error| format!("{{\"error\":\"cannot serialize result: {error}\"}}"))
    }

    fn load_snapshot(&self, project_id: &str) -> Result<TodoSnapshot, TodoError> {
        let project = self.project(project_id)?;
        TodoDocument::load(project.todo_path()).map(|document| document.snapshot())
    }

    fn validate_changes(
        &self,
        snapshot: &TodoSnapshot,
        changes: &[TaskChange],
    ) -> Result<(), TodoError> {
        let known_task = |task_id: &str| snapshot.tasks.iter().any(|task| task.id == task_id);
        for change in changes {
            match change {
                TaskChange::Add {
                    title,
                    after_task_id,
                } => {
                    if title.trim().is_empty() {
                        return Err(TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: "an added task must have a title".to_owned(),
                        });
                    }
                    if let Some(task_id) = after_task_id
                        && !known_task(task_id)
                    {
                        return Err(TodoError::TaskNotFound(task_id.clone()));
                    }
                }
                TaskChange::Update {
                    task_id,
                    title,
                    checked,
                } => {
                    if !known_task(task_id) || (title.is_none() && checked.is_none()) {
                        return Err(TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: format!("invalid update for {task_id}"),
                        });
                    }
                    if title
                        .as_deref()
                        .is_some_and(|value| value.trim().is_empty())
                    {
                        return Err(TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: format!("the title of {task_id} cannot be empty"),
                        });
                    }
                }
                TaskChange::Delete { task_id } if !known_task(task_id) => {
                    return Err(TodoError::TaskNotFound(task_id.clone()));
                }
                TaskChange::Delete { .. } => {}
            }
        }
        Ok(())
    }
}

#[tool_router(server_handler)]
impl TaskHudMcp {
    #[tool(
        description = "Lists the explicitly registered projects and whether they are reachable locally."
    )]
    fn list_projects(&self) -> String {
        match self.registry() {
            Ok(registry) => Self::json(
                &registry
                    .projects
                    .iter()
                    .map(Self::summary)
                    .collect::<Vec<_>>(),
            ),
            Err(error) => Self::json(&serde_json::json!({ "error": error.to_string() })),
        }
    }

    #[tool(description = "Returns the tasks and the TODO.md version of a project.")]
    fn get_project_tasks(&self, Parameters(args): Parameters<ProjectIdArgs>) -> String {
        match self.load_snapshot(&args.project_id) {
            Ok(snapshot) => Self::json(&snapshot),
            Err(error) => Self::json(&serde_json::json!({ "error": error.to_string() })),
        }
    }

    #[tool(description = "Returns the active project configured in TaskHUD, if any.")]
    fn get_active_project(&self) -> String {
        match self.registry() {
            Ok(registry) => match registry.active_project() {
                Some(project) => Self::json(&Self::summary(project)),
                None => Self::json(&serde_json::json!({ "project": null })),
            },
            Err(error) => Self::json(&serde_json::json!({ "error": error.to_string() })),
        }
    }

    #[tool(
        description = "Records, in one call, a proposal to add tasks to a project, without modifying TODO.md. Human approval is required."
    )]
    fn propose_tasks(&self, Parameters(args): Parameters<ProposeTasksArgs>) -> String {
        let result = (|| {
            if args.reason.trim().is_empty() || args.tasks.is_empty() {
                return Err(TodoError::InvalidConfig {
                    path: self.registry_path.clone(),
                    message: "a proposal must have a reason and at least one task".to_owned(),
                });
            }
            let project = self.project_selector(args.project.as_deref())?;
            let snapshot = self.load_snapshot(&project.id)?;
            let changes: Vec<TaskChange> = args
                .tasks
                .into_iter()
                .map(|title| TaskChange::Add {
                    title,
                    after_task_id: None,
                })
                .collect();
            self.validate_changes(&snapshot, &changes)?;
            let mut store = ProposalStore::open(project.root)?;
            store.add(project.id, snapshot.version, args.reason, changes)
        })();
        match result {
            Ok(proposal) => Self::json(&proposal),
            Err(error) => Self::json(&serde_json::json!({ "error": error.to_string() })),
        }
    }

    #[tool(
        description = "Records a structured proposal, without modifying TODO.md. Human approval is required."
    )]
    fn propose_task_changes(&self, Parameters(args): Parameters<ProposeTaskChangesArgs>) -> String {
        let result = (|| {
            let snapshot = self.load_snapshot(&args.project_id)?;
            if snapshot.version != args.base_version {
                return Err(TodoError::Conflict {
                    expected: args.base_version,
                    actual: snapshot.version,
                });
            }
            if args.changes.is_empty() {
                return Err(TodoError::InvalidConfig {
                    path: self.registry_path.clone(),
                    message: "a proposal must contain at least one change".to_owned(),
                });
            }
            if args.reason.trim().is_empty() {
                return Err(TodoError::InvalidConfig {
                    path: self.registry_path.clone(),
                    message: "a proposal must explain its reason".to_owned(),
                });
            }
            let project = self.project(&args.project_id)?;
            let mut store = ProposalStore::open(project.root)?;
            let changes: Vec<TaskChange> = args.changes.into_iter().map(Into::into).collect();
            self.validate_changes(&snapshot, &changes)?;
            store.add(args.project_id, args.base_version, args.reason, changes)
        })();
        match result {
            Ok(proposal) => Self::json(&proposal),
            Err(error) => Self::json(&serde_json::json!({ "error": error.to_string() })),
        }
    }

    #[tool(description = "Lists the pending local proposals for a project.")]
    fn list_proposals(&self, Parameters(args): Parameters<ProjectIdArgs>) -> String {
        let result = (|| {
            let project = self.project(&args.project_id)?;
            Ok::<_, TodoError>(ProposalStore::open(project.root)?.pending())
        })();
        match result {
            Ok(proposals) => Self::json(&proposals),
            Err(error) => Self::json(&serde_json::json!({ "error": error.to_string() })),
        }
    }

    #[tool(description = "Returns a detailed local proposal, without applying it.")]
    fn get_proposal(&self, Parameters(args): Parameters<ProposalIdArgs>) -> String {
        let result = (|| {
            let project = self.project(&args.project_id)?;
            Ok::<_, TodoError>(
                ProposalStore::open(project.root)?
                    .get(&args.proposal_id)?
                    .clone(),
            )
        })();
        match result {
            Ok(proposal) => Self::json(&proposal),
            Err(error) => Self::json(&serde_json::json!({ "error": error.to_string() })),
        }
    }
}

fn config_path_from(args: Vec<std::ffi::OsString>) -> Result<PathBuf> {
    Ok(taskhud_core::config_path_from_args(args)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = TaskHudMcp {
        registry_path: config_path_from(env::args_os().skip(1).collect())?,
    };
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_override_the_registry_is_the_one_the_app_uses() {
        if env::var_os(taskhud_core::CONFIG_ENV).is_some() {
            return;
        }
        assert_eq!(
            config_path_from(Vec::new()).unwrap(),
            taskhud_core::default_config_path()
        );
    }
}
