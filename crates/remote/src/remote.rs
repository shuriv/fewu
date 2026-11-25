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
<<<<<<< e7f7e51e8bb8928f6da436ea430467bdcb09559d
pub use remote_identity::{
    RemoteConnectionIdentity, remote_connection_identity, same_remote_connection_identity,
};
=======
pub use remote_identity::RemoteConnectionIdentity;
pub use transport::docker::DockerConnectionOptions;
>>>>>>> d571ed289a308ccf0958d92ca2a28b9e44e6e2f5
pub use transport::ssh::{SshConnectionOptions, SshPortForwardOption};
pub use transport::wsl::WslConnectionOptions;
#[cfg(target_os = "windows")]
pub use transport::wsl::wsl_path_to_windows_path;

#[cfg(any(test, feature = "test-support"))]
pub use transport::mock::{
    MOCK_STDIO_FRAME_HEX, MockConnection, MockConnectionOptions, MockConnectionRegistry,
    MockDelegate,
};
