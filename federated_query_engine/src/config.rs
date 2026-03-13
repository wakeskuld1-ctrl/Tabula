use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct AppConfig {
    pub root_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub metadata_path: PathBuf,
    pub cache_dir: PathBuf,
    pub l1_cache_dir: PathBuf,
    pub shadow_cache_dir: PathBuf,
    pub yashan_cache_dir: PathBuf,
}

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

impl AppConfig {
    pub fn global() -> &'static AppConfig {
        CONFIG.get_or_init(Self::init)
    }

    fn init() -> Self {
        // Assume current working directory is workspace root (or close to it)
        // Ideally we should traverse up to find Cargo.toml, but for now CWD is reliable in this env.
        let cwd = std::env::current_dir().expect("Failed to get CWD");
        
        // Use .runtime in the current directory (Project Root)
        let runtime_dir = cwd.join(".runtime");
        
        let metadata_dir = runtime_dir.join("metadata");
        let metadata_path = metadata_dir.join("metadata.db");
        
        let cache_dir = runtime_dir.join("cache");
        let l1_cache_dir = cache_dir.join("l1");
        let shadow_cache_dir = cache_dir.join("shadow");
        let yashan_cache_dir = cache_dir.join("yashandb");

        // Ensure directories exist
        if let Err(e) = std::fs::create_dir_all(&metadata_dir) {
            eprintln!("Failed to create metadata dir: {}", e);
        }
        if let Err(e) = std::fs::create_dir_all(&l1_cache_dir) {
            eprintln!("Failed to create l1 cache dir: {}", e);
        }
        if let Err(e) = std::fs::create_dir_all(&shadow_cache_dir) {
            eprintln!("Failed to create shadow cache dir: {}", e);
        }
        if let Err(e) = std::fs::create_dir_all(&yashan_cache_dir) {
            eprintln!("Failed to create yashan cache dir: {}", e);
        }

        Self {
            root_dir: cwd,
            runtime_dir,
            metadata_path,
            cache_dir,
            l1_cache_dir,
            shadow_cache_dir,
            yashan_cache_dir,
        }
    }
}
