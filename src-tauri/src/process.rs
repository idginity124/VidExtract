use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use tokio_util::sync::CancellationToken;

// Tools receive argument arrays; no shell ever interprets URLs or local file names.
pub fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.as_std_mut().process_group(0);
    }
    cmd
}

pub async fn run(
    mut cmd: Command,
    cancel: CancellationToken,
    mut line: impl FnMut(&str) + Send,
) -> Result<String, String> {
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Araç başlatılamadı: {e}"))?;
    let pid = child.id();
    let mut stdout = BufReader::new(child.stdout.take().ok_or("Çıktı açılamadı")?).lines();
    let mut stderr = BufReader::new(child.stderr.take().ok_or("Hata çıktısı açılamadı")?).lines();
    let mut out_done = false;
    let mut err_done = false;
    let mut output = String::new();
    let mut errors = String::new();
    while !out_done || !err_done {
        tokio::select! {
            _ = cancel.cancelled() => {
                kill_tree(pid).await;
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err("İptal edildi".into());
            }
            result = stdout.next_line(), if !out_done => {
                match result.map_err(|e| e.to_string())? {
                    Some(value) => {
                        line(&value);
                        if output.len() + value.len() > 16 * 1024 * 1024 {
                            kill_tree(pid).await;
                            return Err("Araç çıktısı izin verilen boyutu aştı".into());
                        }
                        output.push_str(&value); output.push('\n');
                    },
                    None => out_done = true,
                }
            }
            result = stderr.next_line(), if !err_done => {
                match result.map_err(|e| e.to_string())? {
                    Some(value) => {
                        line(&value);
                        // Keep only the tail of diagnostic output, on UTF-8 boundaries.
                        if errors.len() > 32_000 { errors.clear(); }
                        errors.push_str(&value); errors.push('\n');
                    },
                    None => err_done = true,
                }
            }
        }
    }
    let status = tokio::select! {
        result = child.wait() => result.map_err(|e| e.to_string())?,
        _ = cancel.cancelled() => {
            kill_tree(pid).await;
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err("İptal edildi".into());
        }
    };
    if !status.success() {
        let message = errors
            .lines()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(if message.is_empty() {
            format!("Araç hata koduyla kapandı: {status}")
        } else {
            message
        });
    }
    Ok(output)
}

pub async fn checked(cmd: Command, seconds: u64) -> Result<String, String> {
    let cancel = CancellationToken::new();
    let worker = run(cmd, cancel.clone(), |_| {});
    tokio::pin!(worker);
    tokio::select! {
        result = &mut worker => result,
        _ = tokio::time::sleep(Duration::from_secs(seconds)) => {
            cancel.cancel();
            let _ = worker.await;
            Err("İşlem zaman aşımına uğradı".into())
        }
    }
}

async fn kill_tree(pid: Option<u32>) {
    if let Some(pid) = pid {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .creation_flags(0x08000000)
                .output()
                .await;
        }
    }
}
