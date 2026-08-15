use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    Init(InitError),
    Engine(EngineError),
    Pipeline(PipelineError),
    Texture(TextureError),

    #[cfg(feature = "winit")]
    Winit(winit::error::EventLoopError),

    #[cfg(feature = "sctk")]
    Sctk(crate::sctk::SctkError),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init(e) => write!(f, "initialization failed: {e}"),
            Self::Engine(e) => write!(f, "engine error: {e}"),
            Self::Pipeline(e) => write!(f, "pipeline error: {e}"),
            Self::Texture(e) => write!(f, "texture error: {e}"),

            #[cfg(feature = "winit")]
            Self::Winit(e) => write!(f, "winit error: {e}"),

            #[cfg(feature = "sctk")]
            Self::Sctk(e) => write!(f, "sctk error: {e}"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Init(e) => Some(e),
            Self::Engine(e) => Some(e),
            Self::Pipeline(e) => Some(e),
            Self::Texture(e) => Some(e),

            #[cfg(feature = "winit")]
            Self::Winit(e) => Some(e),

            #[cfg(feature = "sctk")]
            Self::Sctk(e) => Some(e),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum InitError {
    NoAdapter,
    RequestDevice(wgpu::RequestDeviceError),
    CreateSurface(wgpu::CreateSurfaceError),

    UnsupportedFeatureProfile,
    NoInstance,

    #[cfg(feature = "winit")]
    CreateWindow(winit::error::OsError),
}
impl From<InitError> for Error {
    fn from(value: InitError) -> Self {
        Self::Init(value)
    }
}
impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAdapter => write!(f, "no suitable graphics adapter found"),
            Self::RequestDevice(e) => write!(f, "failed to request device: {e}"),
            Self::CreateSurface(e) => write!(f, "failed to create surface: {e}"),
            Self::UnsupportedFeatureProfile => write!(f, "unsupported feature profile"),
            Self::NoInstance => write!(f, "no wgpu instance available"),

            #[cfg(feature = "winit")]
            Self::CreateWindow(e) => write!(f, "failed to create window: {e}"),
        }
    }
}
impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RequestDevice(e) => Some(e),
            Self::CreateSurface(e) => Some(e),

            #[cfg(feature = "winit")]
            Self::CreateWindow(e) => Some(e),

            _ => None,
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum EngineError {
    InvalidTarget,
    MissingPrimaryTarget,
    OutOfMemory,
}
impl From<EngineError> for Error {
    fn from(value: EngineError) -> Self {
        Self::Engine(value)
    }
}
impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget => write!(f, "invalid target"),
            Self::MissingPrimaryTarget => write!(f, "missing primary target"),
            Self::OutOfMemory => write!(f, "out of memory"),
        }
    }
}
impl std::error::Error for EngineError {}

#[non_exhaustive]
#[derive(Debug)]
pub enum PipelineError {
    MissingPrimarySurfaceFormat,
    NotRegistered,
}
impl From<PipelineError> for Error {
    fn from(value: PipelineError) -> Self {
        Self::Pipeline(value)
    }
}
impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPrimarySurfaceFormat => {
                write!(f, "missing primary surface format")
            }
            Self::NotRegistered => write!(f, "pipeline not registered"),
        }
    }
}
impl std::error::Error for PipelineError {}

#[non_exhaustive]
#[derive(Debug)]
pub enum TextureError {
    InvalidHandle,
    AtlasFull,
    InvalidPixelData,
}
impl From<TextureError> for Error {
    fn from(value: TextureError) -> Self {
        Self::Texture(value)
    }
}
impl fmt::Display for TextureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle => write!(f, "invalid texture handle"),
            Self::AtlasFull => write!(f, "texture atlas is full"),
            Self::InvalidPixelData => write!(f, "invalid texture pixel data"),
        }
    }
}
impl std::error::Error for TextureError {}

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
