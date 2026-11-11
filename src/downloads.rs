//! Download destinations and shell helpers.
//!
//! WebView2 proposes a path (usually `%USERPROFILE%\Downloads\name.ext`); we
//! sanitize it, make sure the folder exists and never overwrite an existing
//! file. The final path is what gets stored in the profile.

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Main window handle, used as the owner of the folder picker dialog.
#[cfg(windows)]
static OWNER_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg(windows)]
pub fn set_owner_hwnd(hwnd: isize) {
    OWNER_HWND.store(hwnd, std::sync::atomic::Ordering::Relaxed);
}

const INVALID: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

/// The folder where Deeps saves downloads when WebView2 does not propose one.
pub fn download_dir() -> PathBuf {
    home()
        .map(|home| home.join("Downloads"))
        .unwrap_or_else(std::env::temp_dir)
}

/// The configured download folder, or the Windows Downloads folder when the
/// setting is empty/unusable.
pub fn destination_dir(configured: &str) -> PathBuf {
    let trimmed = configured.trim();
    if trimmed.is_empty() {
        return download_dir();
    }
    let path = PathBuf::from(trimmed);
    if path.is_dir() {
        return path;
    }
    if !path.exists() {
        if std::fs::create_dir_all(&path).is_ok() {
            return path;
        }
    }
    download_dir()
}

/// Cleans a user/website supplied file name.
pub fn sanitize(name: &str) -> String {
    let mut clean: String = name
        .chars()
        .map(|ch| {
            if INVALID.contains(&ch) || ch.is_control() {
                '_'
            } else {
                ch
            }
        })
        .collect();
    while clean.ends_with('.') || clean.ends_with(' ') {
        clean.pop();
    }
    let clean = clean.trim_start_matches([' ', '.']);
    let lower = clean.to_ascii_lowercase();
    let reserved = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    if clean.is_empty() {
        return "download".to_string();
    }
    if reserved.contains(&lower.as_str()) {
        return format!("_{clean}");
    }
    clean.to_string()
}

fn unique(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = path
        .file_stem()
        .and_then(|part| part.to_str())
        .unwrap_or("download")
        .to_string();
    let ext = path.extension().and_then(|part| part.to_str()).unwrap_or("");
    for index in 1..10_000 {
        let name = if ext.is_empty() {
            format!("{stem} ({index})")
        } else {
            format!("{stem} ({index}).{ext}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path
}

/// Final path inside a chosen folder for a website supplied file name.
///
/// The folder is created when needed and an existing file is never
/// overwritten: the name gets a ` (1)`, ` (2)` … suffix instead.
pub fn prepare_in(dir: &Path, name: &str) -> PathBuf {
    let dir = if dir.as_os_str().is_empty() {
        download_dir()
    } else {
        dir.to_path_buf()
    };
    let _ = std::fs::create_dir_all(&dir);
    let name = sanitize(name);
    unique(dir.join(name))
}

/// Opens the native "pick a folder" dialog. Returns `None` when the user
/// cancels. Uses the main window as owner when it has been registered, so the
/// dialog cannot open behind Deeps.
#[cfg(windows)]
pub fn pick_folder() -> Result<Option<String>, String> {
    use std::sync::atomic::Ordering;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };

    let title: Vec<u16> = "Scegli la cartella dei download\0".encode_utf16().collect();
    let owner = OWNER_HWND.load(Ordering::Relaxed);
    let owner = if owner == 0 {
        None
    } else {
        Some(HWND(owner as *mut core::ffi::c_void))
    };

    unsafe {
        // WebView2 normally initialises COM already; a loopback request runs on
        // its own thread, so initialise it there too. Ignoring the result keeps
        // the call harmless when the thread is already in an apartment.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
                .map_err(|error| error.to_string())?;
        let options = dialog.GetOptions().map_err(|error| error.to_string())?;
        dialog
            .SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)
            .map_err(|error| error.to_string())?;
        dialog
            .SetTitle(PCWSTR(title.as_ptr()))
            .map_err(|error| error.to_string())?;

        match dialog.Show(owner) {
            Ok(()) => {}
            Err(error) if error.code().0 as u32 == ERROR_CANCELLED.0 => return Ok(None),
            Err(error) => return Err(error.to_string()),
        }

        let item = dialog.GetResult().map_err(|error| error.to_string())?;
        let raw = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|error| error.to_string())?;
        let text = raw.to_string().map_err(|error| error.to_string())?;
        Ok(Some(text))
    }
}

#[cfg(not(windows))]
pub fn pick_folder() -> Result<Option<String>, String> {
    Err("Scelta cartella non supportata".to_string())
}

fn shell(program: &str, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("{program}: {error}"))
}

/// Opens a downloaded file with its default application.
pub fn open_file(path: &str) -> Result<(), String> {
    if !Path::new(path).is_file() {
        return Err("File non trovato".to_string());
    }
    shell("cmd", &["/C", "start", "", path])
}

/// Opens Explorer with the file selected.
pub fn reveal(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err("File non trovato".to_string());
    }
    let select = format!("/select,\"{path}\"");
    shell("explorer", &[&select])
}
