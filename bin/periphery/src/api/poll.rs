use std::sync::OnceLock;
use futures_util::FutureExt;
use komodo_client::entities::{
  docker::DockerLists, server::PeripheryInformation,
};
use mogh_resolver::Resolve;
use periphery_client::api::poll::{PollStatus, PollStatusResponse};
use tokio::sync::Mutex;

use crate::{
  config::periphery_config,
  docker::{DockerClient, compose::compose_projects_from_containers},
  state::{
    docker_client, host_public_ip, periphery_keys, stats_client,
  },
};

impl Resolve<crate::api::Args> for PollStatus {
  async fn resolve(
    self,
    _: &crate::api::Args,
  ) -> anyhow::Result<PollStatusResponse> {
    let stats_client = stats_client().read().await;

    let system_stats = if self.include_stats {
      Some(stats_client.stats.clone())
    } else {
      None
    };

    let docker = if self.include_docker {
      let client = docker_client().load();
      if let Some(client) = client.iter().next() {
        Some(docker_lists(client).await)
      } else {
        None
      }
    } else {
      None
    };

    Ok(PollStatusResponse {
      periphery_info: periphery_information().await,
      system_info: stats_client.info.clone(),
      system_stats,
      docker,
    })
  }
}

async fn periphery_information() -> PeripheryInformation {
  let config = periphery_config();
  PeripheryInformation {
    version: env!("CARGO_PKG_VERSION").to_string(),
    public_key: periphery_keys().load().public.to_string(),
    terminals_disabled: config.disable_terminals,
    container_terminals_disabled: config.disable_container_terminals,
    stats_polling_rate: config.stats_polling_rate,
    docker_connected: docker_client().load().is_some(),
    public_ip: host_public_ip().await.cloned(),
  }
}

struct CachedDockerLists {
  data: DockerLists,
  fetched_at: std::time::Instant,
}

fn docker_cache() -> &'static Mutex<Option<CachedDockerLists>> {
  static DOCKER_CACHE: OnceLock<Mutex<Option<CachedDockerLists>>> =
    OnceLock::new();
  DOCKER_CACHE.get_or_init(Default::default)
}

async fn docker_lists(client: &DockerClient) -> DockerLists {
  let mut lock = docker_cache().lock().await;
  let now = std::time::Instant::now();
  if let Some(cached) = lock.as_ref() {
    // Cache for 10s to eliminate container / network / volume polling storms
    if now.duration_since(cached.fetched_at).as_secs() < 10 {
      return cached.data.clone();
    }
  }

  let containers = client.list_containers().await.unwrap_or_default();
  // Extract compose projects natively from container labels without spawning external CLI process
  let projects = compose_projects_from_containers(&containers);
  let (networks, images, volumes) = tokio::join!(
    client
      .list_networks(&containers)
      .map(Result::unwrap_or_default),
    client
      .list_images(&containers)
      .map(Result::unwrap_or_default),
    client
      .list_volumes(&containers)
      .map(Result::unwrap_or_default),
  );

  let lists = DockerLists {
    containers,
    networks,
    images,
    volumes,
    projects,
  };

  *lock = Some(CachedDockerLists {
    data: lists.clone(),
    fetched_at: now,
  });

  lists
}
