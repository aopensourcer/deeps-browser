//! Registers Deeps as a Windows browser candidate and opens the Default Apps
//! page. Windows still asks the user to confirm the default, which is by
//! design: no application can silently steal the association.

use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("{program} exited with {status}")),
        Err(error) => Err(format!("{program}: {error}")),
    }
}

fn add(key: &str, name: Option<&str>, data: &str) -> Result<(), String> {
    match name {
        Some(name) => run(
            "reg",
            &["add", key, "/v", name, "/t", "REG_SZ", "/d", data, "/f"],
        ),
        None => run("reg", &["add", key, "/ve", "/t", "REG_SZ", "/d", data, "/f"]),
    }
}

fn executable() -> Result<String, String> {
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .map_err(|error| format!("Percorso di Deeps non disponibile: {error}"))
}

/// Writes the current-user registry entries that make Deeps appear in
/// Settings → Default apps as a web browser.
pub fn register() -> Result<String, String> {
    let exe = executable()?;
    let open = format!("\"{exe}\" \"%1\"");
    let icon = format!("\"{exe}\",0");
    let prog = r"HKCU\Software\Classes\DeepsHTML";
    let client = r"HKCU\Software\Clients\StartMenuInternet\Deeps";
    let caps = r"HKCU\Software\Clients\StartMenuInternet\Deeps\Capabilities";

    add(prog, None, "Deeps HTML Document")?;
    add(&format!(r"{prog}\DefaultIcon"), None, &icon)?;
    add(&format!(r"{prog}\shell\open\command"), None, &open)?;

    add(client, None, "Deeps")?;
    add(&format!(r"{client}\DefaultIcon"), None, &icon)?;
    add(&format!(r"{client}\shell\open\command"), None, &open)?;

    add(caps, None, "Deeps")?;
    add(
        &format!(r"{caps}\ApplicationName"),
        None,
        "Deeps",
    )?;
    add(
        &format!(r"{caps}\ApplicationDescription"),
        None,
        "A small, clean desktop browser",
    )?;
    add(&format!(r"{caps}\ApplicationIcon"), None, &icon)?;
    add(
        &format!(r"{caps}\FileAssociations"),
        Some(".html"),
        "DeepsHTML",
    )?;
    add(
        &format!(r"{caps}\FileAssociations"),
        Some(".htm"),
        "DeepsHTML",
    )?;
    add(
        &format!(r"{caps}\URLAssociations"),
        Some("http"),
        "DeepsHTML",
    )?;
    add(
        &format!(r"{caps}\URLAssociations"),
        Some("https"),
        "DeepsHTML",
    )?;

    add(
        r"HKCU\Software\RegisteredApplications",
        Some("Deeps"),
        r"Software\Clients\StartMenuInternet\Deeps\Capabilities",
    )?;

    Ok(exe)
}

pub fn is_registered() -> bool {
    let mut command = Command::new("reg");
    command.args([
        "query",
        r"HKCU\Software\RegisteredApplications",
        "/v",
        "Deeps",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn open_default_apps() {
    let _ = run("cmd", &["/C", "start", "", "ms-settings:defaultapps"]);
}

/// Registers the browser and opens the Windows page where the user can pick it.
pub fn register_and_open() -> Result<String, String> {
    let exe = register()?;
    open_default_apps();
    Ok(exe)
}
