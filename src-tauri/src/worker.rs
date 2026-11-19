use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    os::windows::{
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::CommandExt,
    },
    process::{Child, ChildStdin, Command, Stdio},
};
use tauri::{AppHandle, Manager};
use windows::Win32::{Foundation::HANDLE, System::JobObjects::*};

pub struct Worker {
    child: Child,
    input: ChildStdin,
    _job: OwnedHandle,
}

impl Worker {
    pub fn start(app: AppHandle, generation: u64) -> Result<Self, String> {
        let resources = app.path().resource_dir().map_err(|e| e.to_string())?;
        let bundled = resources.join("worker").join("flowforge-worker.exe");
        let mut command;
        if bundled.is_file() {
            command = Command::new(bundled);
        } else if cfg!(debug_assertions) {
            let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap();
            let python = project.join(".venv/Scripts/python.exe");
            if !python.is_file() {
                return Err("Python environment missing. Follow README setup.".into());
            }
            command = Command::new(python);
            command
                .args(["-u", "-m", "flowforge.worker"])
                .current_dir(project);
        } else {
            return Err("Bundled speech engine is missing. Reinstall FlowForge AI.".into());
        }
        command
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|e| format!("Cannot start speech engine: {e}"))?;
        // A job closes the Python process even if the desktop process crashes.
        let job = unsafe {
            let handle = match CreateJobObjectW(None, None) {
                Ok(handle) => handle,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error.to_string());
                }
            };
            let owned = OwnedHandle::from_raw_handle(handle.0);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if let Err(error) = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            )
            .and_then(|_| AssignProcessToJobObject(handle, HANDLE(child.as_raw_handle())))
            {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Cannot supervise speech engine: {error}"));
            }
            owned
        };
        let output = child.stdout.take().ok_or("Worker stdout unavailable")?;
        let error_output = child.stderr.take().ok_or("Worker stderr unavailable")?;
        let input = child.stdin.take().ok_or("Worker stdin unavailable")?;
        std::thread::spawn(move || {
            // Drain stderr to avoid native/download logs blocking the worker.
            for line in BufReader::new(error_output).lines().map_while(Result::ok) {
                eprintln!("Speech engine: {line}");
            }
        });
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                match line {
                    Ok(line) => match serde_json::from_str(&line) {
                        Ok(value) => crate::receive(&app, value, generation),
                        Err(error) => crate::worker_failed(
                            &app,
                            generation,
                            format!("Invalid speech-engine response: {error}"),
                        ),
                    },
                    Err(error) => {
                        crate::worker_failed(&app, generation, error.to_string());
                        break;
                    }
                }
            }
            crate::worker_failed(
                &app,
                generation,
                "Speech engine stopped unexpectedly. Reload it to recover.".into(),
            );
        });
        Ok(Self {
            child,
            input,
            _job: job,
        })
    }

    pub fn send(&mut self, value: Value) -> Result<(), String> {
        writeln!(self.input, "{value}")
            .and_then(|_| self.input.flush())
            .map_err(|e| format!("Speech engine is unavailable: {e}"))
    }

    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}
