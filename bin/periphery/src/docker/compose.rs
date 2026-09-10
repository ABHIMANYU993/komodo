use std::time::Duration;

use anyhow::{Context, anyhow};
use command::{CommandOptions, run_komodo_standard_command};
use komodo_client::entities::stack::*;
use serde::{Deserialize, Serialize};

use crate::config::periphery_config;

pub fn docker_compose() -> &'static str {
  if periphery_config().legacy_compose_cli {
    "docker-compose"
  } else {
    "docker compose"
  }
}

#[allow(dead_code)]
pub async fn list_compose_projects()
-> anyhow::Result<Vec<ComposeProject>> {
  let docker_compose = docker_compose();
  let res = run_komodo_standard_command(
    "List Projects",
    format!("{docker_compose} ls --all --format json"),
    CommandOptions::default().timeout(Duration::from_secs(5)),
  )
  .await;

  if !res.success {
    return Err(anyhow!("{}", res.combined()).context(format!(
      "Failed to list compose projects using {docker_compose} ls"
    )));
  }

  let mut res =
    serde_json::from_str::<Vec<DockerComposeLsItem>>(&res.stdout)
      .with_context(|| res.stdout.clone())
      .with_context(|| {
        format!(
          "Failed to parse '{docker_compose} ls' response from json"
        )
      })?
      .into_iter()
      .filter(|item| !item.name.is_empty())
      .map(|item| ComposeProject {
        name: item.name,
        status: item.status,
        compose_files: item
          .config_files
          .split(',')
          .map(str::to_string)
          .collect(),
      })
      .collect::<Vec<_>>();

  res.sort_by(|a, b| {
    a.status.cmp(&b.status).then_with(|| a.name.cmp(&b.name))
  });

  Ok(res)
}

/// Natively extracts ComposeProject metadata directly from inspected container labels.
/// This avoids spawning 'docker compose ls' external processes in high-frequency monitoring loops.
pub fn compose_projects_from_containers(
  containers: &[komodo_client::entities::docker::container::ContainerListItem],
) -> Vec<ComposeProject> {
  let mut map: std::collections::HashMap<
    String,
    (Option<String>, std::collections::BTreeSet<String>),
  > = std::collections::HashMap::new();

  for c in containers {
    if let Some(project) = c.labels.get("com.docker.compose.project") {
      if project.is_empty() {
        continue;
      }
      let entry = map.entry(project.clone()).or_insert_with(|| {
        (c.status.clone(), std::collections::BTreeSet::new())
      });
      if let Some(config_files) =
        c.labels.get("com.docker.compose.project.config_files")
      {
        for file in config_files.split(',') {
          let trimmed = file.trim();
          if !trimmed.is_empty() {
            entry.1.insert(trimmed.to_string());
          }
        }
      }
    }
  }

  let mut res = map
    .into_iter()
    .map(|(name, (status, files))| ComposeProject {
      name,
      status,
      compose_files: files.into_iter().collect(),
    })
    .collect::<Vec<_>>();

  res.sort_by(|a, b| {
    a.status.cmp(&b.status).then_with(|| a.name.cmp(&b.name))
  });
  res
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerComposeLsItem {
  #[serde(default, alias = "Name")]
  pub name: String,
  #[serde(alias = "Status")]
  pub status: Option<String>,
  /// Comma seperated list of paths
  #[serde(default, alias = "ConfigFiles")]
  pub config_files: String,
}

pub fn parse_compose_services(
  raw_config: &str,
  project_name: &str,
  services: &mut Vec<StackServiceNames>,
) -> anyhow::Result<()> {
  let compose = serde_yaml_ng::from_str::<ComposeFile>(raw_config)
    .context("Failed to parse compose contents")?;

  for (
    service_name,
    ComposeService {
      container_name,
      deploy,
      image,
    },
  ) in compose.services
  {
    let image = image.unwrap_or_default();
    match deploy {
      Some(ComposeServiceDeploy {
        replicas: Some(replicas),
      }) if replicas > 1 => {
        for i in 1..1 + replicas {
          services.push(StackServiceNames {
            container_name: format!(
              "{project_name}-{service_name}-{i}"
            ),
            service_name: format!("{service_name}-{i}"),
            image: image.clone(),
            image_digest: None,
          });
        }
      }
      _ => {
        services.push(StackServiceNames {
          container_name: container_name.unwrap_or_else(|| {
            format!("{project_name}-{service_name}")
          }),
          service_name,
          image,
          image_digest: None,
        });
      }
    }
  }

  Ok(())
}
