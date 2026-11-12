use crate::RemoteConnectionOptions;
use std::borrow::Cow;

<<<<<<< eb4351930270422c44d9786c2a4924e2acb0382f
/// A normalized remote identity for matching live remote hosts against
/// persisted remote metadata.
///
/// This mirrors workspace persistence identity semantics rather than full
/// `RemoteConnectionOptions` equality, so runtime-only fields like SSH
/// nicknames do not affect matching.
=======
/// Identifies a remote host for project and workspace matching.
>>>>>>> d2b6b1997b3993e835ca2b0c99473f6ace3d4285
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RemoteConnectionIdentity<'a> {
    Ssh {
        host: Cow<'a, str>,
        username: Option<Cow<'a, str>>,
        port: Option<u16>,
    },
    Wsl {
        distro_name: Cow<'a, str>,
        user: Option<Cow<'a, str>>,
    },
<<<<<<< eb4351930270422c44d9786c2a4924e2acb0382f
=======
    Docker {
        container_id: Cow<'a, str>,
        remote_user: Cow<'a, str>,
    },
>>>>>>> d2b6b1997b3993e835ca2b0c99473f6ace3d4285
    #[cfg(any(test, feature = "test-support"))]
    Mock { id: u64 },
}

impl RemoteConnectionIdentity<'_> {
    pub fn into_owned(self) -> RemoteConnectionIdentity<'static> {
        match self {
            Self::Ssh {
                host,
                username,
                port,
<<<<<<< eb4351930270422c44d9786c2a4924e2acb0382f
            } => format!(
                "ssh:{}@{}:{}",
                username.as_deref().unwrap_or_default(),
                host,
                port.map(|port| port.to_string()).unwrap_or_default()
            ),
            Self::Wsl { distro_name, user } => format!(
                "wsl:{}@{}",
                user.as_deref().unwrap_or_default(),
                distro_name
            ),
=======
            } => RemoteConnectionIdentity::Ssh {
                host: Cow::Owned(host.into_owned()),
                username: username.map(|username| Cow::Owned(username.into_owned())),
                port,
            },
            Self::Wsl { distro_name, user } => RemoteConnectionIdentity::Wsl {
                distro_name: Cow::Owned(distro_name.into_owned()),
                user: user.map(|user| Cow::Owned(user.into_owned())),
            },
            Self::Docker {
                container_id,
                remote_user,
            } => RemoteConnectionIdentity::Docker {
                container_id: Cow::Owned(container_id.into_owned()),
                remote_user: Cow::Owned(remote_user.into_owned()),
            },
>>>>>>> d2b6b1997b3993e835ca2b0c99473f6ace3d4285
            #[cfg(any(test, feature = "test-support"))]
            Self::Mock { id } => RemoteConnectionIdentity::Mock { id },
        }
    }
}

impl<'a> From<&'a RemoteConnectionOptions> for RemoteConnectionIdentity<'a> {
    fn from(options: &'a RemoteConnectionOptions) -> Self {
        match options {
            RemoteConnectionOptions::Ssh(options) => Self::Ssh {
                host: options.host.to_cow(),
                username: options.username.as_deref().map(Cow::Borrowed),
                port: options.port,
            },
            RemoteConnectionOptions::Wsl(options) => Self::Wsl {
                distro_name: Cow::Borrowed(&options.distro_name),
                user: options.user.as_deref().map(Cow::Borrowed),
            },
<<<<<<< eb4351930270422c44d9786c2a4924e2acb0382f
=======
            RemoteConnectionOptions::Docker(options) => Self::Docker {
                container_id: Cow::Borrowed(&options.container_id),
                remote_user: Cow::Borrowed(&options.remote_user),
            },
>>>>>>> d2b6b1997b3993e835ca2b0c99473f6ace3d4285
            #[cfg(any(test, feature = "test-support"))]
            RemoteConnectionOptions::Mock(options) => Self::Mock { id: options.id },
        }
    }
}

#[cfg(test)]
mod tests {
<<<<<<< eb4351930270422c44d9786c2a4924e2acb0382f
    use super::*;
    use crate::{SshConnectionOptions, WslConnectionOptions};
=======
    use std::collections::BTreeMap;

    use crate::{
        DockerConnectionOptions, RemoteConnectionOptions, SshConnectionOptions,
        WslConnectionOptions,
    };
>>>>>>> d2b6b1997b3993e835ca2b0c99473f6ace3d4285

    #[test]
    fn ssh_identity_ignores_non_persisted_runtime_fields() {
        let left = RemoteConnectionOptions::Ssh(SshConnectionOptions {
            host: "example.com".into(),
            username: Some("anth".to_string()),
            port: Some(2222),
            password: Some("secret".to_string()),
            args: Some(vec!["-v".to_string()]),
            connection_timeout: Some(30),
            nickname: Some("work".to_string()),
            upload_binary_over_ssh: true,
            ..Default::default()
        });
        let right = RemoteConnectionOptions::Ssh(SshConnectionOptions {
            host: "example.com".into(),
            username: Some("anth".to_string()),
            port: Some(2222),
            password: None,
            args: None,
            connection_timeout: None,
            nickname: None,
            upload_binary_over_ssh: false,
            ..Default::default()
        });

        assert_eq!(left.identity(), right.identity());
    }

    #[test]
    fn ssh_identity_distinguishes_persistence_key_fields() {
        let left = RemoteConnectionOptions::Ssh(SshConnectionOptions {
            host: "example.com".into(),
            username: Some("anth".to_string()),
            port: Some(2222),
            ..Default::default()
        });
        let right = RemoteConnectionOptions::Ssh(SshConnectionOptions {
            host: "example.com".into(),
            username: Some("anth".to_string()),
            port: Some(2223),
            ..Default::default()
        });

        assert_ne!(left.identity(), right.identity());
    }

    #[test]
    fn wsl_identity_includes_user() {
        let left = RemoteConnectionOptions::Wsl(WslConnectionOptions {
            distro_name: "Ubuntu".to_string(),
            user: Some("anth".to_string()),
        });
        let right = RemoteConnectionOptions::Wsl(WslConnectionOptions {
            distro_name: "Ubuntu".to_string(),
            user: Some("root".to_string()),
        });

        assert_ne!(left.identity(), right.identity());
    }

    #[test]
<<<<<<< eb4351930270422c44d9786c2a4924e2acb0382f
    fn local_identity_matches_only_local_identity() {
        let remote = RemoteConnectionOptions::Wsl(WslConnectionOptions {
            distro_name: "Ubuntu".to_string(),
            user: Some("anth".to_string()),
        });

        assert!(same_remote_connection_identity(None, None));
        assert!(!same_remote_connection_identity(None, Some(&remote)));
=======
    fn docker_identity_ignores_display_name_and_runtime_fields() {
        let left = RemoteConnectionOptions::Docker(DockerConnectionOptions {
            name: "renamed-dev".to_string(),
            container_id: "container-123".to_string(),
            remote_user: "anth".to_string(),
            upload_binary_over_docker_exec: true,
            use_podman: true,
            remote_env: BTreeMap::from([("FOO".to_string(), "BAR".to_string())]),
        });
        let right = RemoteConnectionOptions::Docker(DockerConnectionOptions {
            name: "zed-dev".to_string(),
            container_id: "container-123".to_string(),
            remote_user: "anth".to_string(),
            upload_binary_over_docker_exec: false,
            use_podman: false,
            remote_env: BTreeMap::new(),
        });

        assert_eq!(left.identity(), right.identity());
>>>>>>> d2b6b1997b3993e835ca2b0c99473f6ace3d4285
    }
}
