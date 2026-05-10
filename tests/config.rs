use nedit::config::Config;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_config_get_keybind_existing() {
    let config = Config::default();
    assert_eq!(config.get_keybind("save"), "ctrl+s");
}

#[test]
fn test_config_get_keybind_non_existent_returns_default() {
    let config = Config::default();
    // Ação inexistente mas que tem default mapeado
    assert_eq!(config.get_keybind("quit"), "ctrl+q");
}

#[test]
fn test_config_custom_load() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let toml_content = r#"
        autocomplete_enabled = false
        theme = "monokai"
        [keybinds]
        quit = "ctrl+x"
    "#;
    fs::write(&config_path, toml_content).unwrap();

    // Como o Config::load() usa dirs::config_dir(),
    // testar o load exato é difícil sem mockar o ambiente.
    // Mas podemos testar a desserialização manualmente se a struct for public.
    let config: Config = toml::from_str(toml_content).unwrap();
    assert!(!config.autocomplete_enabled);
    assert_eq!(config.theme, "monokai");
    assert_eq!(config.keybinds.get("quit").unwrap(), "ctrl+x");
}
