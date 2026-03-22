use std::fs::File;
use std::process::{Child, Command, Stdio};

pub struct GrpcServerProcess {
    child: Option<Child>,
    pub last_error: Option<String>,
}

impl Default for GrpcServerProcess {
    fn default() -> Self {
        Self {
            child: None,
            last_error: None,
        }
    }
}

impl GrpcServerProcess {
    pub fn start(&mut self, working_dir: &str, config_path: &str) -> Result<(), String> {
        if self.is_running() {
            return Err("Server is already running".to_string());
        }

        self.last_error = None;
        kill_process_on_port(50051);

        let log_dir = std::path::Path::new("log");
        let stdout_file = File::create(log_dir.join("grpc_server_stdout.txt"))
            .map_err(|e| format!("Failed to create stdout log: {}", e))?;
        let stderr_file = File::create(log_dir.join("grpc_server_stderr.txt"))
            .map_err(|e| format!("Failed to create stderr log: {}", e))?;

        let child = Command::new("uv")
            .args(["run", "python", "-c"])
            .arg(format!(
                "from anim_ml.server.service import main; import sys; sys.argv = ['service', '--config', '{}']; main()",
                config_path
            ))
            .current_dir(working_dir)
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .map_err(|e| format!("Failed to start server: {}", e))?;

        log!("gRPC server started (pid={})", child.id());
        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let pid = child.id();
            let _ = child.kill();
            let _ = child.wait();
            log!("gRPC server stopped (pid={})", pid);
        }
    }

    pub fn is_running(&mut self) -> bool {
        let child = match self.child.as_mut() {
            Some(c) => c,
            None => return false,
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let stderr_content =
                        std::fs::read_to_string("log/grpc_server_stderr.txt").unwrap_or_default();
                    let last_line = stderr_content
                        .lines()
                        .rev()
                        .find(|l| !l.trim().is_empty())
                        .unwrap_or("Unknown error")
                        .to_string();
                    log_error!("gRPC server exited with {}: {}", status, last_line);
                    self.last_error = Some(last_line);
                }
                self.child = None;
                false
            }
            Ok(None) => true,
            Err(_) => {
                self.child = None;
                false
            }
        }
    }
}

fn kill_process_on_port(port: u16) {
    let output = Command::new("netstat")
        .args(["-ano"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let port_str = format!(":{}", port);

    for line in stdout.lines() {
        if !line.contains(&port_str) || !line.contains("LISTENING") {
            continue;
        }

        let pid_str = line.split_whitespace().last().unwrap_or("");
        let pid: u32 = match pid_str.parse() {
            Ok(p) if p > 0 => p,
            _ => continue,
        };

        log_warn!(
            "Port {} is in use by PID {}, killing to free port",
            port,
            pid
        );
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        std::thread::sleep(std::time::Duration::from_millis(500));
        return;
    }
}

impl Drop for GrpcServerProcess {
    fn drop(&mut self) {
        self.stop();
    }
}
