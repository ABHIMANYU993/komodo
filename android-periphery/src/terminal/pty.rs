use std::fs::File;
use std::os::fd::{FromRawFd, RawFd};
use std::path::Path;
use anyhow::anyhow;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing::{debug, error};
use uuid::Uuid;

pub struct PtySession {
    pub id: Uuid,
    pub name: String,
    pub master_fd: RawFd,
    pub child_pid: libc::pid_t,
    pub stdin_tx: mpsc::Sender<Vec<u8>>,
}

impl PtySession {
    /// Discover an available shell on Android: prefers /system/bin/sh, /system/bin/bash, /bin/sh.
    pub fn discover_shell() -> String {
        for shell in ["/system/bin/sh", "/system/bin/bash", "/bin/sh", "/system/xbin/bash", "/system/xbin/sh"] {
            if Path::new(shell).exists() {
                return shell.to_string();
            }
        }
        "/system/bin/sh".to_string()
    }

    /// Spawn a new native root PTY session.
    pub fn spawn(id: Uuid, name: String, custom_command: Option<&str>) -> anyhow::Result<(Self, mpsc::Receiver<Vec<u8>>)> {
        let shell = custom_command.map(str::to_string).unwrap_or_else(Self::discover_shell);

        unsafe {
            // Open master PTY via /dev/ptmx or posix_openpt
            let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            if master < 0 {
                return Err(anyhow!("posix_openpt failed: errno {}", std::io::Error::last_os_error()));
            }

            if libc::grantpt(master) != 0 {
                libc::close(master);
                return Err(anyhow!("grantpt failed"));
            }

            if libc::unlockpt(master) != 0 {
                libc::close(master);
                return Err(anyhow!("unlockpt failed"));
            }

            // Get slave name
            let mut pts_buf = [0u8; 64];
            if libc::ptsname_r(master, pts_buf.as_mut_ptr() as *mut libc::c_char, pts_buf.len()) != 0 {
                libc::close(master);
                return Err(anyhow!("ptsname_r failed"));
            }

            let pts_name = std::ffi::CStr::from_ptr(pts_buf.as_ptr() as *const libc::c_char)
                .to_str()?
                .to_string();

            debug!("Allocated PTY: master fd {}, slave {}", master, pts_name);

            // Fork child process
            let pid = libc::fork();
            if pid < 0 {
                libc::close(master);
                return Err(anyhow!("fork failed"));
            }

            if pid == 0 {
                // CHILD PROCESS
                libc::close(master);
                libc::setsid();

                let slave_c = std::ffi::CString::new(pts_name.clone()).unwrap();
                let slave = libc::open(slave_c.as_ptr(), libc::O_RDWR);
                if slave < 0 {
                    libc::_exit(1);
                }

                // Set controlling tty
                libc::ioctl(slave, libc::TIOCSCTTY as _, 0);

                // Duplicate stdin, stdout, stderr to slave PTY
                libc::dup2(slave, libc::STDIN_FILENO);
                libc::dup2(slave, libc::STDOUT_FILENO);
                libc::dup2(slave, libc::STDERR_FILENO);

                if slave > libc::STDERR_FILENO {
                    libc::close(slave);
                }

                // Set environment
                let path_c = std::ffi::CString::new("PATH=/sbin:/product/bin:/apex/com.android.runtime/bin:/system/bin:/system/xbin:/odm/bin:/vendor/bin").unwrap();
                libc::putenv(path_c.into_raw());

                let term_c = std::ffi::CString::new("TERM=xterm-256color").unwrap();
                libc::putenv(term_c.into_raw());

                let home_c = std::ffi::CString::new("HOME=/data/local/tmp").unwrap();
                libc::putenv(home_c.into_raw());

                let shell_c = std::ffi::CString::new(shell.clone()).unwrap();
                let args = [shell_c.as_ptr(), std::ptr::null()];

                libc::execve(shell_c.as_ptr(), args.as_ptr(), [std::ptr::null()].as_ptr());
                libc::_exit(127);
            }

            // PARENT PROCESS
            // Set master non-blocking for Tokio async integration
            let flags = libc::fcntl(master, libc::F_GETFL, 0);
            libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);

            let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(64);
            let (stdout_tx, stdout_rx) = mpsc::channel::<Vec<u8>>(64);

            let master_read_fd = master;
            let master_write_fd = libc::dup(master);

            // Spawn Tokio task for writing stdin to PTY
            tokio::spawn(async move {
                let mut async_file = tokio::fs::File::from_std(File::from_raw_fd(master_write_fd));
                while let Some(data) = stdin_rx.recv().await {
                    if let Err(e) = async_file.write_all(&data).await {
                        error!("Failed to write to PTY master: {e}");
                        break;
                    }
                    let _ = async_file.flush().await;
                }
            });

            // Spawn Tokio task for reading stdout from PTY
            tokio::spawn(async move {
                let mut async_file = tokio::fs::File::from_std(File::from_raw_fd(master_read_fd));
                let mut buf = [0u8; 4096];
                loop {
                    match async_file.read(&mut buf).await {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            if stdout_tx.send(buf[..n].to_vec()).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            debug!("PTY master read ended: {e}");
                            break;
                        }
                    }
                }
            });

            let session = PtySession {
                id,
                name,
                master_fd: master,
                child_pid: pid,
                stdin_tx,
            };

            Ok((session, stdout_rx))
        }
    }

    /// Resize window dimensions via `TIOCSWINSZ`.
    pub fn resize(&self, cols: u16, rows: u16) -> anyhow::Result<()> {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let res = unsafe { libc::ioctl(self.master_fd, libc::TIOCSWINSZ as _, &ws) };
        if res == 0 {
            Ok(())
        } else {
            Err(anyhow!("ioctl TIOCSWINSZ failed with code {res}"))
        }
    }

    /// Terminate shell child and close PTY.
    pub fn close(&self) {
        unsafe {
            libc::kill(self.child_pid, libc::SIGHUP);
            libc::kill(self.child_pid, libc::SIGKILL);
            libc::waitpid(self.child_pid, std::ptr::null_mut(), libc::WNOHANG);
        }
    }
}
