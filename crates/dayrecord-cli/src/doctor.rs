//! `dayrecord doctor` diagnostics.

use crate::mcp_autostart::mcp_autostart_allowed;
use crate::version::{self, VERSION};
use anyhow::{bail, Context as _};
use dayrecord_adapters::SqliteRepository;
use dayrecord_core::control::{ControlClient, ControlCommand};
use dayrecord_core::paths;
use dayrecord_runtime::{
    capture_service_likely_running, spawn_detached_daemon, wait_for_capture_service,
    IpcControlClient,
};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
const EXPECTED_MCP_TOOLS: usize = 10;

pub fn doctor_mcp() -> anyhow::Result<()> {
    let this_exe = version::current_exe().context("could not resolve current executable")?;
    let this_norm = version::normalize_exe(&this_exe);
    let this_ver = version::read_exe_version(&this_exe).unwrap_or_else(|| VERSION.into());

    println!("dayrecord version: {VERSION}");
    println!("this executable: {}", this_norm.display());
    println!("this --version:  {this_ver}");

    if let Some(path_exe) = version::resolve_on_path() {
        let path_norm = version::normalize_exe(&path_exe);
        let path_ver = version::read_exe_version(&path_exe).unwrap_or_else(|| "(unknown)".into());
        println!("PATH executable: {}", path_norm.display());
        println!("PATH --version:  {path_ver}");

        if path_norm != this_norm {
            println!();
            println!("warning: PATH points to a different binary than this executable.");
            println!("         Update MCP `command` to the intended path, or align PATH.");
        }
        if !path_ver.contains(VERSION) {
            println!();
            println!("warning: PATH binary version may not match this build ({VERSION}).");
            println!("         Reinstall with `cargo install --path crates/dayrecord-cli --force`");
            println!("         (disable MCP first if the file is locked).");
        }

        match probe_mcp_tools_list(&path_exe) {
            Ok(count) => println!("PATH MCP tools/list: {count} tools"),
            Err(e) => {
                println!("PATH MCP tools/list: FAILED ({e})");
                println!("         This usually means an outdated binary (tools/list schema panic).");
            }
        }
    } else {
        println!("PATH executable: (not found)");
        println!("hint: add dayrecord to PATH or use an absolute path in MCP config.");
    }

    match probe_mcp_tools_list(&this_exe) {
        Ok(count) if count >= EXPECTED_MCP_TOOLS => {
            println!("this MCP tools/list: {count} tools (ok)");
        }
        Ok(count) => {
            println!("this MCP tools/list: {count} tools (expected {EXPECTED_MCP_TOOLS})");
        }
        Err(e) => bail!("this binary failed MCP tools/list probe: {e}"),
    }

    match probe_control_tool_is_error(&this_exe) {
        Ok(()) => println!("this MCP tools/call offline signal: ok (isError or ok:false)"),
        Err(e) if e.contains("skipped") => println!("this MCP tools/call offline signal: {e}"),
        Err(e) => {
            println!("this MCP tools/call offline signal: FAILED ({e})");
            println!("         Control tools should report IPC offline via isError or ok:false + error.");
        }
    }

    match probe_daemon_autostart(&this_exe) {
        Ok(()) => println!("daemon autostart + IPC: ok"),
        Err(e) => println!("daemon autostart + IPC: skipped or failed ({e})"),
    }

    Ok(())
}

fn probe_mcp_tools_list(exe: &Path) -> Result<usize, String> {
    run_mcp_subprocess(exe, |stdin| {
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2024-11-05","capabilities":{{}},"clientInfo":{{"name":"doctor","version":"1.0"}}}}}}"#).map_err(|e| e.to_string())?;
        writeln!(stdin, r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#).map_err(|e| e.to_string())?;
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{{}}}}"#).map_err(|e| e.to_string())?;
        Ok(())
    })
    .and_then(|output| parse_tools_list_count(&output))
}

fn probe_control_tool_is_error(exe: &Path) -> Result<(), String> {
    if capture_service_likely_running() {
        return Err("skipped (capture service already running; offline isError probe N/A)".into());
    }
    let output = run_mcp_subprocess(exe, |stdin| {
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2024-11-05","capabilities":{{}},"clientInfo":{{"name":"doctor","version":"1.0"}}}}}}"#).map_err(|e| e.to_string())?;
        writeln!(stdin, r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#).map_err(|e| e.to_string())?;
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"pause_recording","arguments":{{}}}}}}"#).map_err(|e| e.to_string())?;
        Ok(())
    })?;
    parse_tools_call_is_error(&output)
}

fn run_mcp_subprocess(
    exe: &Path,
    write_stdin: impl FnOnce(&mut std::process::ChildStdin) -> Result<(), String>,
) -> Result<std::process::Output, String> {
    let mut child = Command::new(exe)
        .arg("mcp")
        .env("DAYRECORD_MCP_DISABLE_AUTOSTART", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    {
        let mut stdin = child.stdin.take().ok_or("stdin unavailable")?;
        write_stdin(&mut stdin)?;
    }

    child.wait_with_output().map_err(|e| e.to_string())
}

fn parse_tools_list_count(output: &std::process::Output) -> Result<usize, String> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Invalid output schema") || stderr.contains("panicked") {
            return Err("server panicked during tools/list (outdated build?)".into());
        }
        if !stderr.is_empty() {
            return Err(stderr.trim().to_string());
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if !line.contains("\"tools\"") {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        if let Some(tools) = v
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array())
        {
            return Ok(tools.len());
        }
    }
    Err("no tools/list response on stdout".into())
}

fn parse_tools_call_is_error(output: &std::process::Output) -> Result<(), String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if !line.contains("pause_recording") {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let result = match v.get("result") {
            Some(r) => r,
            None => continue,
        };
        if result.get("isError") == Some(&serde_json::json!(true))
            || result.get("is_error") == Some(&serde_json::json!(true))
        {
            return Ok(());
        }
        let content = result
            .get("structuredContent")
            .or_else(|| result.get("structured_content"));
        if let Some(c) = content {
            if c.get("ok") == Some(&serde_json::json!(false)) && c.get("error").is_some() {
                return Ok(());
            }
        }
    }
    Err("no tools/call offline failure signal on stdout (expected isError or ok:false)".into())
}

fn probe_daemon_autostart(exe: &Path) -> Result<(), String> {
    if capture_service_likely_running() {
        let client = IpcControlClient;
        client
            .request(ControlCommand::Status)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let repo = SqliteRepository::open(&paths::db_path()).map_err(|e| e.to_string())?;
    if mcp_autostart_allowed(&repo).is_err() {
        return Err("consent not granted or mcp_autostart_daemon=false".into());
    }

    spawn_detached_daemon(exe)?;
    let started = wait_for_capture_service(Duration::from_secs(8));
    if !started {
        return Err("daemon did not become ready in time".into());
    }

    let client = IpcControlClient;
    client
        .request(ControlCommand::Status)
        .map_err(|e| e.to_string())?;

    stop_capture_service_by_pid()?;
    Ok(())
}

fn stop_capture_service_by_pid() -> Result<(), String> {
    let pid_path = paths::data_dir().join("dayrecord.pid");
    let raw = std::fs::read_to_string(&pid_path).map_err(|e| e.to_string())?;
    let pid = raw.trim().parse::<u32>().map_err(|e| e.to_string())?;

    #[cfg(windows)]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("taskkill pid {pid} failed"));
        }
    }

    #[cfg(not(windows))]
    {
        let status = Command::new("kill")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("kill pid {pid} failed"));
        }
    }

    Ok(())
}
