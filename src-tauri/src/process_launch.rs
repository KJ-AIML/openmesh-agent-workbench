//! Visible-terminal launch: keep `program + argv + cwd` internally.
//!
//! macOS Terminal.app and Windows `cmd /k` are shell interfaces, so the only
//! string join happens behind tested quoting. Substring "dangerous command"
//! blocklists are not the authorization control.

use std::process::Command;

pub const ERR_EMPTY_TOKEN: &str = "empty_process_token";
pub const ERR_INVALID_TOKEN: &str = "invalid_process_token";
pub const ERR_INVALID_PROGRAM: &str = "invalid_program";

#[derive(Debug, Clone)]
pub struct TerminalLaunch {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub brief_path: Option<String>,
}

pub fn validate_token(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(ERR_EMPTY_TOKEN.into());
    }
    if s.chars().any(|c| c == '\0' || c == '\n' || c == '\r') {
        return Err(ERR_INVALID_TOKEN.into());
    }
    Ok(())
}

/// Program is a PATH name or filesystem path. Spaces are allowed; shell
/// operators are not, so `cmd; evil` cannot be smuggled as the program.
pub fn validate_program(program: &str) -> Result<(), String> {
    validate_token(program)?;
    if program.contains(|c| matches!(c, ';' | '|' | '&' | '`' | '\n' | '\r' | '\0')) {
        return Err(ERR_INVALID_PROGRAM.into());
    }
    Ok(())
}

pub fn posix_single_quote(s: &str) -> String {
    let mut out = String::from("'");
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

pub fn join_posix_argv(program: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(posix_single_quote(program));
    for a in args {
        parts.push(posix_single_quote(a));
    }
    parts.join(" ")
}

/// POSIX script for a login-style keep-open terminal.
pub fn posix_keep_open_script(spec: &TerminalLaunch) -> Result<String, String> {
    validate_program(&spec.program)?;
    validate_token(&spec.cwd)?;
    for a in &spec.args {
        validate_token(a)?;
    }
    let mut script = format!("cd {} && ", posix_single_quote(&spec.cwd));
    if let Some(brief) = spec
        .brief_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        validate_token(brief)?;
        let line = format!("OpenMesh brief: {brief}");
        script.push_str("printf '%s\\n' ");
        script.push_str(&posix_single_quote(&line));
        script.push_str(" && ");
    }
    script.push_str(&join_posix_argv(&spec.program, &spec.args));
    script.push_str("; exec bash");
    Ok(script)
}

/// Wrap a POSIX command for `osascript -e 'tell application "Terminal" to do script "..."'`.
pub fn apple_script_do_script(posix_shell: &str) -> String {
    let escaped = posix_shell.replace('\\', "\\\\").replace('"', "\\\"");
    format!("tell application \"Terminal\" to do script \"{escaped}\"")
}

#[cfg(any(test, target_os = "windows"))]
pub fn cmd_exe_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        if ch == '"' {
            out.push('"');
        }
        out.push(ch);
    }
    out.push('"');
    out
}

#[cfg(any(test, target_os = "windows"))]
pub fn join_cmd_argv(program: &str, args: &[String]) -> String {
    std::iter::once(program)
        .chain(args.iter().map(String::as_str))
        .map(cmd_exe_quote)
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn launch_visible_terminal(spec: &TerminalLaunch) -> Result<(), String> {
    let script = posix_keep_open_script(spec)?;
    let _ = &script;

    #[cfg(target_os = "windows")]
    {
        let joined = join_cmd_argv(&spec.program, &spec.args);
        let mut prefix = String::new();
        if let Some(brief) = spec
            .brief_path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            validate_token(brief)?;
            prefix = format!(
                "echo {} && ",
                cmd_exe_quote(&format!("OpenMesh brief: {brief}"))
            );
        }
        let keep_open = format!("{prefix}{joined}");
        let wt = Command::new("wt")
            .arg("-d")
            .arg(&spec.cwd)
            .arg("cmd")
            .arg("/k")
            .arg(&keep_open)
            .spawn();
        if wt.is_ok() {
            return Ok(());
        }
        let ps = Command::new("powershell")
            .arg("-NoExit")
            .arg("-Command")
            .arg(format!(
                "Set-Location {}; {}",
                cmd_exe_quote(&spec.cwd),
                keep_open
            ))
            .spawn();
        if ps.is_ok() {
            return Ok(());
        }
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("cmd")
            .arg("/K")
            .arg(format!(
                "cd /d {} && {}",
                cmd_exe_quote(&spec.cwd),
                keep_open
            ))
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open terminal: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let ascript = apple_script_do_script(&script);
        Command::new("osascript")
            .arg("-e")
            .arg(&ascript)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open terminal: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let terminals: &[(&str, &[&str])] = &[
            (
                "gnome-terminal",
                &["--working-directory", "--", "bash", "-lc"],
            ),
            ("konsole", &["--workdir", "--", "bash", "-lc"]),
            ("xterm", &["-e", "bash", "-lc"]),
        ];
        for (terminal, prefix) in terminals {
            let mut cmd = Command::new(terminal);
            let mut iter = prefix.iter();
            if *terminal != "xterm" {
                if let Some(flag) = iter.next() {
                    cmd.arg(flag);
                    cmd.arg(&spec.cwd);
                }
            }
            for arg in iter {
                cmd.arg(arg);
            }
            cmd.arg(&script);
            if cmd.spawn().is_ok() {
                return Ok(());
            }
        }
        return Err(
            "No supported terminal found. Install gnome-terminal, konsole, or xterm.".into(),
        );
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = script;
        Err("Unsupported platform".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_quote_wraps_spaces_and_metacharacters() {
        assert_eq!(posix_single_quote("hello"), "'hello'");
        assert_eq!(posix_single_quote("a b"), "'a b'");
        assert_eq!(posix_single_quote("hello; rm -rf /"), "'hello; rm -rf /'");
        assert_eq!(posix_single_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn join_keeps_metacharacter_arg_as_one_token() {
        let joined = join_posix_argv("echo", &["hello; rm -rf /".into(), "file name.txt".into()]);
        assert_eq!(joined, "'echo' 'hello; rm -rf /' 'file name.txt'");
        assert!(!joined.contains("echo hello"));
    }

    #[test]
    fn keep_open_script_quotes_cwd_and_args() {
        let spec = TerminalLaunch {
            program: "codex".into(),
            args: vec!["resume".into(), "sess 1".into()],
            cwd: "/tmp/proj dir".into(),
            brief_path: Some("/tmp/proj dir/brief.md".into()),
        };
        let script = posix_keep_open_script(&spec).unwrap();
        assert!(script.starts_with("cd '/tmp/proj dir' && "));
        assert!(script.contains("'codex' 'resume' 'sess 1'"));
        assert!(script.contains("OpenMesh brief: /tmp/proj dir/brief.md"));
        assert!(script.ends_with("; exec bash"));
        assert!(!script.contains("cd /tmp/proj dir &&"));
    }

    #[test]
    fn keep_open_script_rejects_newline_in_arg() {
        let spec = TerminalLaunch {
            program: "echo".into(),
            args: vec!["ok\nnasty".into()],
            cwd: "/tmp".into(),
            brief_path: None,
        };
        assert_eq!(
            posix_keep_open_script(&spec).unwrap_err(),
            ERR_INVALID_TOKEN
        );
    }

    #[test]
    fn program_rejects_shell_operators() {
        assert_eq!(
            validate_program("echo; rm").unwrap_err(),
            ERR_INVALID_PROGRAM
        );
        assert!(validate_program("/usr/bin/echo").is_ok());
        assert!(validate_program(r"C:\Program Files\app.exe").is_ok());
    }

    #[test]
    fn apple_script_escapes_quotes_in_posix_body() {
        let body = join_posix_argv("echo", &["foo\"bar".into()]);
        let script = apple_script_do_script(&body);
        assert!(script.contains("foo\\\"bar"), "{script}");
        assert!(script.starts_with("tell application \"Terminal\" to do script \""));
    }

    #[test]
    fn cmd_quote_doubles_embedded_quotes() {
        assert_eq!(cmd_exe_quote("say \"hi\""), "\"say \"\"hi\"\"\"");
        let joined = join_cmd_argv("echo", &["a b".into()]);
        assert_eq!(joined, "\"echo\" \"a b\"");
    }
}
