use std::env;
use std::io::{self, BufRead};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use taskhud_core::{
    Project, ProjectRegistry, ProposalStore, TaskChange, TodoDocument, TodoError, TodoSnapshot,
    propose_changes,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FunctionCall {
    function: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectIdArgs {
    project_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProposalIdArgs {
    project_id: String,
    proposal_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProposeArgs {
    project_id: String,
    base_version: String,
    reason: String,
    changes: Vec<TaskChangeInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProposeTasksArgs {
    #[serde(default)]
    project: Option<String>,
    reason: String,
    tasks: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase")]
enum TaskChangeInput {
    Add {
        title: String,
        #[serde(rename = "afterTaskId", alias = "after_task_id")]
        after_task_id: Option<String>,
    },
    Update {
        #[serde(rename = "taskId", alias = "task_id")]
        task_id: String,
        title: Option<String>,
        checked: Option<bool>,
    },
    Delete {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSummary {
    id: String,
    name: String,
    root: PathBuf,
    todo_path: PathBuf,
    todo_exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TasksResult {
    path: String,
    version: String,
    tasks: Vec<taskhud_core::Task>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

struct Functions {
    registry_path: PathBuf,
}

impl Functions {
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

    fn snapshot(&self, project_id: &str) -> Result<TodoSnapshot, TodoError> {
        let project = self.project(project_id)?;
        TodoDocument::load(project.todo_path()).map(|document| document.snapshot())
    }

    fn tasks(&self, args: ProjectIdArgs, include_content: bool) -> Result<TasksResult, TodoError> {
        let snapshot = self.snapshot(&args.project_id)?;
        Ok(TasksResult {
            path: snapshot.path,
            version: snapshot.version,
            tasks: snapshot.tasks,
            content: include_content.then_some(snapshot.content),
        })
    }

    fn call(&self, call: FunctionCall) -> Result<Value, TodoError> {
        match call.function.as_str() {
            "list_projects" => {
                let registry = self.registry()?;
                Ok(json!(
                    registry
                        .projects
                        .iter()
                        .map(|project| ProjectSummary {
                            id: project.id.clone(),
                            name: project.name.clone(),
                            root: project.root.clone(),
                            todo_path: project.todo_path(),
                            todo_exists: project.todo_path().is_file(),
                        })
                        .collect::<Vec<_>>()
                ))
            }
            "get_active_project" => {
                let registry = self.registry()?;
                Ok(json!(registry.active_project().map(|project| {
                    ProjectSummary {
                        id: project.id.clone(),
                        name: project.name.clone(),
                        root: project.root.clone(),
                        todo_path: project.todo_path(),
                        todo_exists: project.todo_path().is_file(),
                    }
                })))
            }
            "get_tasks" => {
                let args: ProjectIdArgs =
                    serde_json::from_value(call.arguments).map_err(|error| {
                        TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: error.to_string(),
                        }
                    })?;
                Ok(json!(self.tasks(args, false)?))
            }
            "get_tasks_with_content" => {
                let args: ProjectIdArgs =
                    serde_json::from_value(call.arguments).map_err(|error| {
                        TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: error.to_string(),
                        }
                    })?;
                Ok(json!(self.tasks(args, true)?))
            }
            "propose_task_changes" => {
                let args: ProposeArgs =
                    serde_json::from_value(call.arguments).map_err(|error| {
                        TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: error.to_string(),
                        }
                    })?;
                let project = self.project(&args.project_id)?;
                let changes = args.changes.into_iter().map(Into::into).collect();
                Ok(json!(propose_changes(
                    &project,
                    Some(&args.base_version),
                    args.reason,
                    changes
                )?))
            }
            "propose_tasks" => {
                let args: ProposeTasksArgs =
                    serde_json::from_value(call.arguments).map_err(|error| {
                        TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: error.to_string(),
                        }
                    })?;
                if args.tasks.is_empty() {
                    return Err(TodoError::InvalidProposal(
                        "a proposal must contain at least one task".to_owned(),
                    ));
                }
                let project = self.project_selector(args.project.as_deref())?;
                let changes = args
                    .tasks
                    .into_iter()
                    .map(|title| TaskChange::Add {
                        title,
                        after_task_id: None,
                    })
                    .collect();
                Ok(json!(propose_changes(
                    &project,
                    None,
                    args.reason,
                    changes
                )?))
            }
            "list_proposals" => {
                let args: ProjectIdArgs =
                    serde_json::from_value(call.arguments).map_err(|error| {
                        TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: error.to_string(),
                        }
                    })?;
                let project = self.project(&args.project_id)?;
                Ok(json!(ProposalStore::open(project.root)?.pending()))
            }
            "get_proposal" => {
                let args: ProposalIdArgs =
                    serde_json::from_value(call.arguments).map_err(|error| {
                        TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: error.to_string(),
                        }
                    })?;
                let project = self.project(&args.project_id)?;
                Ok(json!(
                    ProposalStore::open(project.root)?.get(&args.proposal_id)?
                ))
            }
            _ => Err(TodoError::InvalidConfig {
                path: self.registry_path.clone(),
                message: format!("unknown function: {}", call.function),
            }),
        }
    }
}

fn config_path_from(args: Vec<std::ffi::OsString>) -> Result<PathBuf, String> {
    taskhud_core::config_path_from_args(args).map_err(|error| error.to_string())
}

fn main() -> Result<(), String> {
    let functions = Functions {
        registry_path: config_path_from(env::args_os().skip(1).collect())?,
    };
    for line in io::BufReader::new(io::stdin()).lines() {
        let line = line.map_err(|error| error.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<FunctionCall>(&line) {
            Ok(call) => match functions.call(call) {
                Ok(result) => json!({ "ok": true, "result": result }),
                Err(error) => json!({ "ok": false, "error": error.to_string() }),
            },
            Err(error) => json!({ "ok": false, "error": format!("invalid call: {error}") }),
        };
        println!(
            "{}",
            serde_json::to_string(&response).map_err(|error| error.to_string())?
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(label: &str) -> (PathBuf, PathBuf, String) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "taskhud-functions-{label}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("TODO.md"), "- [ ] Read\n- [ ] Write\n").unwrap();
        let config = root.join("config.json");
        std::fs::write(
            &config,
            serde_json::json!({
                "activeProjectId": "demo",
                "projects": [{ "id": "demo", "name": "Demo", "root": root }]
            })
            .to_string(),
        )
        .unwrap();
        let version = TodoDocument::load(root.join("TODO.md"))
            .unwrap()
            .snapshot()
            .version;
        (root, config, version)
    }

    #[test]
    fn contradictory_proposals_are_rejected_when_proposed() {
        let (root, config, version) = fixture("contradictory");
        let functions = Functions {
            registry_path: config,
        };
        let propose = |changes: Value| {
            functions
                .call(FunctionCall {
                    function: "propose_task_changes".to_owned(),
                    arguments: json!({
                        "projectId": "demo",
                        "baseVersion": version,
                        "reason": "Check",
                        "changes": changes,
                    }),
                })
                .map(|_| ())
        };

        for changes in [
            json!([
                { "operation": "update", "taskId": "task-0", "checked": true },
                { "operation": "delete", "taskId": "task-0" }
            ]),
            json!([
                { "operation": "delete", "taskId": "task-1" },
                { "operation": "add", "title": "Next", "afterTaskId": "task-1" }
            ]),
            json!([{ "operation": "add", "title": "  " }]),
        ] {
            let error = propose(changes.clone()).unwrap_err();
            assert!(
                matches!(error, TodoError::InvalidProposal(_)),
                "{changes}: {error:?}"
            );
        }

        assert!(ProposalStore::open(&root).unwrap().list().is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

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
