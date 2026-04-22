use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Workspace {
    pub name: String,
    pub path: PathBuf,
    pub tabs: Vec<PathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct WorkspaceList {
    #[serde(default)]
    pub active_workspace: Option<String>,
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
}
