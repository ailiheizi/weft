use anyhow::{anyhow, Context, Result};
use libloading::Library;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};

type NativePackageFn = unsafe extern "C" fn(*const u8, usize, *mut usize) -> *mut u8;
type NativePackageFreeFn = unsafe extern "C" fn(*mut u8, usize);

#[derive(Clone)]
pub(crate) struct NativePackageHandle {
    name: String,
    library: Arc<Library>,
    call_fn: NativePackageFn,
    free_fn: Option<NativePackageFreeFn>,
}

pub struct NativePackageHost {
    packages: HashMap<String, NativePackageHandle>,
}

impl Default for NativePackageHost {
    fn default() -> Self {
        Self::new()
    }
}

impl NativePackageHost {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    pub fn has_package(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }

    pub fn package_names(&self) -> Vec<String> {
        self.packages.keys().cloned().collect()
    }

    pub fn load_package(&mut self, load_info: &NativePackageLoadInfo) -> Result<()> {
        if self.packages.contains_key(&load_info.name) {
            return Ok(());
        }

        let library = unsafe { Library::new(&load_info.library_path) }.with_context(|| {
            format!(
                "Failed to load native library '{}')",
                load_info.library_path.display()
            )
        })?;

        let call_fn = unsafe {
            *library
                .get::<NativePackageFn>(b"weft_plugin_handle_json")
                .context("Missing required symbol 'weft_plugin_handle_json'")?
        };
        let free_fn = unsafe {
            library
                .get::<NativePackageFreeFn>(b"weft_plugin_free")
                .ok()
                .map(|symbol| *symbol)
        };

        let handle = NativePackageHandle {
            name: load_info.name.clone(),
            library: Arc::new(library),
            call_fn,
            free_fn,
        };

        self.packages.insert(load_info.name.clone(), handle);
        Ok(())
    }

    pub fn unload_package(&mut self, name: &str) -> Result<()> {
        self.packages
            .remove(name)
            .ok_or_else(|| anyhow!("Native package '{}' not loaded", name))?;
        Ok(())
    }

    pub fn reload_package(&mut self, load_info: &NativePackageLoadInfo) -> Result<()> {
        let _ = self.packages.remove(&load_info.name);
        self.load_package(load_info)
    }

    pub fn call_json(&self, name: &str, payload: &Value) -> Result<Value> {
        let handle = self
            .packages
            .get(name)
            .ok_or_else(|| anyhow!("Native package '{}' not loaded", name))?;

        let _keep_library_alive = handle.library.clone();
        let _package_name = &handle.name;

        let input = serde_json::to_vec(payload)?;
        let mut out_len: usize = 0;
        let out_ptr =
            unsafe { (handle.call_fn)(input.as_ptr(), input.len(), &mut out_len as *mut usize) };
        if out_ptr.is_null() {
            return Err(anyhow!("Native package '{}' returned null response", name));
        }

        let bytes = unsafe { std::slice::from_raw_parts(out_ptr, out_len).to_vec() };
        if let Some(free_fn) = handle.free_fn {
            unsafe { free_fn(out_ptr, out_len) };
        }

        serde_json::from_slice(&bytes)
            .with_context(|| format!("Native package '{}' returned invalid JSON", name))
    }
}

#[derive(Debug, Clone)]
pub struct NativePackageLoadInfo {
    pub name: String,
    pub dir: PathBuf,
    pub library_path: PathBuf,
}

#[derive(Clone)]
pub struct NativeHandle {
    host: Arc<StdMutex<NativePackageHost>>,
}

impl NativeHandle {
    pub fn new(host: NativePackageHost) -> Self {
        Self {
            host: Arc::new(StdMutex::new(host)),
        }
    }

    pub fn has_package(&self, name: &str) -> bool {
        self.host
            .lock()
            .map(|host| host.has_package(name))
            .unwrap_or(false)
    }

    pub fn package_names(&self) -> Vec<String> {
        self.host
            .lock()
            .map(|host| host.package_names())
            .unwrap_or_default()
    }

    pub fn load_package(&self, info: &NativePackageLoadInfo) -> Result<()> {
        let mut host = self
            .host
            .lock()
            .map_err(|e| anyhow!("NativeHandle lock poisoned: {}", e))?;
        host.load_package(info)
    }

    pub fn unload_package(&self, name: &str) -> Result<()> {
        let mut host = self
            .host
            .lock()
            .map_err(|e| anyhow!("NativeHandle lock poisoned: {}", e))?;
        host.unload_package(name)
    }

    pub fn reload_package(&self, info: &NativePackageLoadInfo) -> Result<()> {
        let mut host = self
            .host
            .lock()
            .map_err(|e| anyhow!("NativeHandle lock poisoned: {}", e))?;
        host.reload_package(info)
    }

    pub fn call_json(&self, name: &str, payload: &Value) -> Result<Value> {
        let host = self
            .host
            .lock()
            .map_err(|e| anyhow!("NativeHandle lock poisoned: {}", e))?;
        host.call_json(name, payload)
    }

    pub fn from_test_package(
        name: String,
        call_fn: NativePackageFn,
        free_fn: Option<NativePackageFreeFn>,
    ) -> Result<Self> {
        let library = unsafe { Library::new(std::env::current_exe()?) }
            .with_context(|| "Failed to open current executable for test native handle")?;
        let mut host = NativePackageHost::new();
        host.packages.insert(
            name.clone(),
            NativePackageHandle {
                name,
                library: Arc::new(library),
                call_fn,
                free_fn,
            },
        );
        Ok(Self::new(host))
    }
}

pub fn native_library_candidates(entry_path: &Path) -> Vec<PathBuf> {
    let stem = entry_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("plugin");
    let mut candidates = Vec::new();
    let parent = entry_path.parent().unwrap_or(Path::new("."));
    if entry_path.exists() {
        candidates.push(entry_path.to_path_buf());
    }
    #[cfg(target_os = "windows")]
    {
        candidates.push(parent.join(format!("{}.dll", stem)));
        candidates.push(
            parent
                .join("target")
                .join("debug")
                .join(format!("{}.dll", stem)),
        );
    }
    #[cfg(target_os = "linux")]
    {
        candidates.push(parent.join(format!("lib{}.so", stem)));
        candidates.push(parent.join(format!("{}.so", stem)));
        candidates.push(
            parent
                .join("target")
                .join("debug")
                .join(format!("lib{}.so", stem)),
        );
        candidates.push(
            parent
                .join("target")
                .join("debug")
                .join(format!("{}.so", stem)),
        );
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push(parent.join(format!("lib{}.dylib", stem)));
        candidates.push(parent.join(format!("{}.dylib", stem)));
        candidates.push(
            parent
                .join("target")
                .join("debug")
                .join(format!("lib{}.dylib", stem)),
        );
        candidates.push(
            parent
                .join("target")
                .join("debug")
                .join(format!("{}.dylib", stem)),
        );
    }
    candidates.sort();
    candidates.dedup();
    candidates
}
