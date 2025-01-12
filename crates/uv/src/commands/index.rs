use anyhow::{Context, Ok, Result};
use console::Term;
use tracing::{debug, warn};
use uv_auth::{AuthConfig, ConfigFile};
use uv_configuration::KeyringProviderType;
use uv_distribution_types::Index;

/// Add one or more packages to the project requirements.
#[allow(clippy::fn_params_excessive_bools)]
pub(crate) async fn add_credentials(
    name: String,
    username: Option<String>,
    password: Option<String>,
    keyring_provider: KeyringProviderType,
    indexes: Vec<Index>,
) -> Result<()> {
    let index = indexes.iter().find(|idx| {
        idx.name
            .as_ref()
            .map(|n| n.to_string() == name)
            .unwrap_or(false)
    });

    let index = match index {
        Some(obj) => obj,
        None => panic!("No index found with the name '{}'", name),
    };

    let username = match username {
        Some(n) => n,
        None => match prompt_username_input()? {
            Some(n) => n,
            None => panic!("No username provided and could not read username from input."),
        },
    };

    let password = match password {
        Some(p) => p,
        None => match prompt_password_input()? {
            Some(p) => p,
            None => panic!("Could not read password from user input"),
        },
    };

    let url = index.raw_url();
    debug!("Will store password for index {name} with URL {url} and user {username} in keyring");
    keyring_provider
        .to_provider()
        .expect("Keyring Provider is not available")
        .set(&url, &username, &password)
        .await;

    debug!(
        "Will add index {name} and user {username} to index auth config in {:?}",
        AuthConfig::path()?
    );
    let mut auth_config = AuthConfig::load()
        .inspect_err(|err| warn!("Could not load auth config due to: {err}"))?;
    auth_config.add_entry(name, username);
    auth_config.store()?;

    Ok(())
}

pub(crate) async fn list_credentials(
    keyring_provider_type: KeyringProviderType,
    indexes: Vec<Index>,
) -> Result<()> {
    let auth_config = AuthConfig::load()
        .inspect_err(|err| warn!("Could not load auth config due to: {err}"))?;

    let keyring_provider = keyring_provider_type
        .to_provider()
        .expect("Keyring Provider is not available");

    for index in indexes {
        if let Some(index_name) = index.name {
            if let Some(auth_index) = auth_config.indexes.get(&index_name.to_string()) {
                let username = auth_index.username.clone();
                let password = keyring_provider.fetch(&index.url, &username).await;

                match password {
                    Some(_) => println!(
                        "Index: '{}' authenticates with username '{}'.",
                        index_name, username
                    ),
                    None => println!("Index: '{}' has no credentials.", index_name),
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn unset_credentials(
    name: String,
    username: Option<String>,
    keyring_provider: KeyringProviderType,
    indexes: Vec<Index>,
) -> Result<()> {
    let index = indexes.iter().find(|idx| {
        idx.name
            .as_ref()
            .map(|n| n.to_string() == name)
            .unwrap_or(false)
    });

    let index = match index {
        Some(obj) => obj,
        None => panic!("No index found with the name '{}'", name),
    };

    let username = match username {
        Some(n) => n,
        None => match prompt_username_input()? {
            Some(n) => n,
            None => panic!("No username provided and could not read username from input."),
        },
    };

    keyring_provider
        .to_provider()
        .expect("Keyring Provider is not available")
        .unset(&index.url, &username)
        .await;

    let mut auth_config = AuthConfig::load()
        .inspect_err(|err| warn!("Could not load auth config due to: {err}"))?;

    auth_config.delete_entry(&name);
    auth_config.store()?;

    Ok(())
}

fn prompt_username_input() -> Result<Option<String>> {
    let term = Term::stderr();
    if !term.is_term() {
        return Ok(None);
    }
    let username_prompt = "Enter username: ";

    let username = uv_console::input(username_prompt, &term).context("Failed to read username")?;
    Ok(Some(username))
}

fn prompt_password_input() -> Result<Option<String>> {
    let term = Term::stderr();
    if !term.is_term() {
        return Ok(None);
    }
    let password_prompt = "Enter password: ";
    let password =
        uv_console::password(password_prompt, &term).context("Failed to read password")?;
    Ok(Some(password))
}
