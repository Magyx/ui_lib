use crate::Error;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[non_exhaustive]
#[derive(Debug)]
pub enum SctkError {
    Connect(BoxError),
    RegistryInit(BoxError),
    BindGlobal(BoxError),
    SurfaceSetup,
    Dispatch(BoxError),
    Flush(BoxError),
    Roundtrip(BoxError),
    SessionLock(BoxError),
    EventLoop(BoxError),
}
impl From<SctkError> for Error {
    fn from(value: SctkError) -> Self {
        Self::Sctk(value)
    }
}
impl std::fmt::Display for SctkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(_) => write!(f, "failed to connect to wayland"),
            Self::RegistryInit(_) => write!(f, "failed to initialize registry"),
            Self::BindGlobal(_) => write!(f, "failed to bind global"),
            Self::SurfaceSetup => write!(f, "failed to setup surface"),
            Self::Dispatch(_) => write!(f, "wayland dispatch failed"),
            Self::Flush(_) => write!(f, "wayland flush failed"),
            Self::Roundtrip(_) => write!(f, "wayland roundtrip failed"),
            Self::SessionLock(_) => write!(f, "wayland session lock failed"),
            Self::EventLoop(_) => write!(f, "event loop error"),
        }
    }
}
impl std::error::Error for SctkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connect(e)
            | Self::RegistryInit(e)
            | Self::BindGlobal(e)
            | Self::Dispatch(e)
            | Self::Flush(e)
            | Self::Roundtrip(e)
            | Self::SessionLock(e)
            | Self::EventLoop(e) => Some(e.as_ref()),

            Self::SurfaceSetup => None,
        }
    }
}
impl SctkError {
    pub fn connect<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Connect(Box::new(e))
    }

    pub fn registry_init<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::RegistryInit(Box::new(e))
    }

    pub fn bind_global<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::BindGlobal(Box::new(e))
    }

    pub fn dispatch<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Dispatch(Box::new(e))
    }

    pub fn flush<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Flush(Box::new(e))
    }

    pub fn roundtrip<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Roundtrip(Box::new(e))
    }

    pub fn session_lock<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::SessionLock(Box::new(e))
    }

    pub fn event_loop<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::EventLoop(Box::new(e))
    }
}
