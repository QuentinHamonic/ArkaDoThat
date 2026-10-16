use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
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
    #[schemars(description = "Identifiant du projet enregistré dans TaskHUD")]
    project_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ProposalIdArgs {
    #[schemars(description = "Identifiant du projet enregistré dans TaskHUD")]
    project_id: String,
    #[schemars(description = "Identifiant de la proposition")]
    proposal_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "camelCase")]
enum TaskChangeInput {
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
    #[schemars(description = "Identifiant du projet enregistré dans TaskHUD")]
    project_id: String,
    #[schemars(description = "Version SHA-256 du TODO.md lu avant la proposition")]
    base_version: String,
    #[schemars(description = "Pourquoi ces changements sont nécessaires")]
    reason: String,
    #[schemars(description = "Changements élémentaires à soumettre à validation humaine")]
    changes: Vec<TaskChangeInput>,
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
        serde_json::to_string_pretty(value).unwrap_or_else(|error| {
            format!("{{\"error\":\"résultat impossible à sérialiser: {error}\"}}")
        })
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
                            message: "une tâche ajoutée doit avoir un titre".to_owned(),
                        });
                    }
                    if let Some(task_id) = after_task_id {
                        if !known_task(task_id) {
                            return Err(TodoError::TaskNotFound(task_id.clone()));
                        }
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
                            message: format!("modification invalide pour {task_id}"),
                        });
                    }
                    if title
                        .as_deref()
                        .is_some_and(|value| value.trim().is_empty())
                    {
                        return Err(TodoError::InvalidConfig {
                            path: self.registry_path.clone(),
                            message: format!("le titre de {task_id} ne peut pas être vide"),
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
        description = "Liste les projets explicitement enregistrés et leur accessibilité locale."
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

    #[tool(description = "Retourne les tâches et la version du TODO.md d'un projet.")]
    fn get_project_tasks(&self, Parameters(args): Parameters<ProjectIdArgs>) -> String {
        match self.load_snapshot(&args.project_id) {
            Ok(snapshot) => Self::json(&snapshot),
            Err(error) => Self::json(&serde_json::json!({ "error": error.to_string() })),
        }
    }

    #[tool(description = "Retourne le projet actif configuré dans TaskHUD, s'il existe.")]
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
        description = "Enregistre une proposition structurée, sans modifier TODO.md. La validation humaine est obligatoire."
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
                    message: "une proposition doit contenir au moins un changement".to_owned(),
                });
            }
            if args.reason.trim().is_empty() {
                return Err(TodoError::InvalidConfig {
                    path: self.registry_path.clone(),
                    message: "une proposition doit expliquer sa raison".to_owned(),
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

    #[tool(description = "Liste les propositions locales en attente pour un projet.")]
    fn list_proposals(&self, Parameters(args): Parameters<ProjectIdArgs>) -> String {
        let result = (|| {
            let project = self.project(&args.project_id)?;
            Ok::<_, TodoError>(ProposalStore::open(project.root)?.list().to_vec())
        })();
        match result {
            Ok(proposals) => Self::json(&proposals),
            Err(error) => Self::json(&serde_json::json!({ "error": error.to_string() })),
        }
    }

    #[tool(description = "Retourne une proposition locale détaillée, sans l'appliquer.")]
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

fn config_path() -> Result<PathBuf> {
    let mut args = env::args_os().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--config" {
            return args
                .next()
                .map(PathBuf::from)
                .context("--config attend un chemin JSON");
        }
    }
    if let Some(path) = env::var_os("TASKHUD_CONFIG") {
        return Ok(PathBuf::from(path));
    }
    Ok(env::current_dir()?.join(".taskhud").join("config.json"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = TaskHudMcp {
        registry_path: config_path()?,
    };
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
