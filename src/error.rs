use std::error::Error as StdError;

#[cfg(feature = "winit")]
pub use winit::error::EventLoopError;

type BoxError = Box<dyn StdError + Send + Sync + 'static>;

pub type Result<T> = std::result::Result<T, Error>;

#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    Init(InitError),
    Engine(EngineError),
    Pipeline(PipelineError),
    Texture(TextureError),

    #[cfg(feature = "winit")]
    Winit(EventLoopError),

    #[cfg(feature = "sctk")]
    Sctk(SctkError),
}

#[non_exhaustive]
#[derive(Debug)]
pub enum InitError {
    NoAdapter,
    RequestDevice,
    CreateSurface,

    #[cfg(feature = "winit")]
    CreateWindow(winit::error::OsError),
}

#[non_exhaustive]
#[derive(Debug)]
pub enum EngineError {
    InvalidTarget,
    MissingPrimaryTarget,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum PipelineError {
    MissingPrimarySurfaceFormat,
    NotRegistered,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum TextureError {
    InvalidHandle,
    AtlasFull,
    InvalidPixelData,
}

#[cfg(feature = "sctk")]
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
}

impl From<InitError> for Error {
    fn from(value: InitError) -> Self {
        Self::Init(value)
    }
}
impl From<EngineError> for Error {
    fn from(value: EngineError) -> Self {
        Self::Engine(value)
    }
}
impl From<PipelineError> for Error {
    fn from(value: PipelineError) -> Self {
        Self::Pipeline(value)
    }
}
impl From<TextureError> for Error {
    fn from(value: TextureError) -> Self {
        Self::Texture(value)
    }
}

#[cfg(feature = "winit")]
impl From<winit::error::EventLoopError> for Error {
    fn from(value: winit::error::EventLoopError) -> Self {
        Self::Winit(value)
    }
}

#[cfg(feature = "winit")]
impl From<winit::error::OsError> for Error {
    fn from(value: winit::error::OsError) -> Self {
        Self::Init(InitError::CreateWindow(value))
    }
}

#[cfg(feature = "sctk")]
impl From<SctkError> for Error {
    fn from(value: SctkError) -> Self {
        Self::Sctk(value)
    }
}

#[cfg(feature = "sctk")]
impl SctkError {
    pub fn connect<E>(e: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Connect(Box::new(e))
    }

    pub fn registry_init<E>(e: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::RegistryInit(Box::new(e))
    }

    pub fn bind_global<E>(e: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::BindGlobal(Box::new(e))
    }

    pub fn dispatch<E>(e: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Dispatch(Box::new(e))
    }

    pub fn flush<E>(e: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Flush(Box::new(e))
    }

    pub fn roundtrip<E>(e: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Roundtrip(Box::new(e))
    }

    pub fn session_lock<E>(e: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::SessionLock(Box::new(e))
    }
}
