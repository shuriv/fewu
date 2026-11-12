pub mod command;
pub mod json_log;
pub mod protocol;
pub mod proxy;
pub mod remote_client;
pub mod remote_identity;
mod transport;

#[cfg(target_os = "windows")]
pub use remote_client::OpenWslPath;
pub use remote_client::{
    CommandTemplate, ConnectionIdentifier, ConnectionState, Interactive, RemoteArch, RemoteClient,
    RemoteClientDelegate, RemoteClientEvent, RemoteConnection, RemoteConnectionOptions, RemoteOs,
    RemotePlatform, connect, has_active_connection,
};
<<<<<<< eb4351930270422c44d9786c2a4924e2acb0382f
pub use remote_identity::{
    RemoteConnectionIdentity, remote_connection_identity, same_remote_connection_identity,
};
=======
pub use remote_identity::RemoteConnectionIdentity;
pub use transport::docker::DockerConnectionOptions;
>>>>>>> d2b6b1997b3993e835ca2b0c99473f6ace3d4285
pub use transport::ssh::{SshConnectionOptions, SshPortForwardOption};
pub use transport::wsl::WslConnectionOptions;
#[cfg(target_os = "windows")]
pub use transport::wsl::wsl_path_to_windows_path;

#[cfg(any(test, feature = "test-support"))]
pub use transport::mock::{
    MOCK_STDIO_FRAME_HEX, MockConnection, MockConnectionOptions, MockConnectionRegistry,
    MockDelegate,
};
