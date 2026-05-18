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
    // Non-existent action but with a mapped default
    assert_eq!(config.get_keybind("quit"), "ctrl+q");
}

#[test]
fn test_config_custom_load() {
    let toml_content = r#"
        autocomplete_enabled = false
        theme = "monokai"
        [keybinds]
        quit = "ctrl+x"
    "#;
    let config = load_config_via_temp_home(toml_content);
    assert!(!config.autocomplete_enabled);
    assert_eq!(config.theme, "monokai");
    assert_eq!(config.keybinds.get("quit").unwrap(), "ctrl+x");
    assert_eq!(config.get_keybind("save"), "ctrl+s");
}

#[test]
fn test_config_supports_legacy_top_level_keybinds() {
    let toml_content = r#"
        theme = "absurd-dark"
        theme_select = "ctrl+t"
    "#;
    let config = load_config_via_temp_home(toml_content);
    assert_eq!(config.theme, "absurd-dark");
    assert_eq!(config.get_keybind("theme_select"), "ctrl+t");
    assert_eq!(config.get_keybind("open_file"), "ctrl+o");
}

fn load_config_via_temp_home(toml_content: &str) -> Config {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join("nedit");
    let config_path = config_dir.join("config.toml");

    fs::create_dir_all(&config_dir).unwrap();
    fs::write(&config_path, toml_content).unwrap();

    let previous = std::env::var_os("XDG_CONFIG_HOME");
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    let config = Config::load();
    match previous {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    config
}
