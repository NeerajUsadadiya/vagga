use std::env;
use std::path::Path;
use std::os::unix::ffi::OsStrExt;

use unshare::{Command, Namespace};

use config::{Settings, Container, Range};
use process_util::{set_uidmap, copy_env_vars};
use container::uidmap::{get_max_uidmap, map_users};


pub trait Wrapper {
    fn new(root: Option<&str>, settings: &Settings) -> Self;
    fn workdir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self;
    fn max_uidmap(&mut self) -> &mut Self;
    fn map_users_for(&mut self, container: &Container, settings: &Settings)
        -> Result<(), String>;
    fn map_users(&mut self, uids: &Vec<Range>, gids: &Vec<Range>,
        settings: &Settings)
        -> Result<(), String>;
}

impl Wrapper for Command {
    fn new(root: Option<&str>, settings: &Settings) -> Self {
        let mut cmd = Command::new("/proc/self/exe");
        cmd.arg("__wrapper__");
        if let Some(root) = root {
            cmd.arg("--root");
            cmd.arg(root);
        };

        cmd.env_clear();

        // Unfortunately OSString does not have starts_with yet
        for (k, v) in env::vars_os() {
            {
                let kbytes = k[..].as_bytes();
                if kbytes.len() < 9 || &kbytes[..9] != &b"VAGGAENV_"[..] {
                    continue
                }
            }
            cmd.env(k, v);
        }
        copy_env_vars(&mut cmd, &settings);
        if let Some(x) = env::var_os("PATH") {
            cmd.env("_VAGGA_PATH", x);
        }
        if let Some(x) = env::var_os("RUST_LOG") {
            cmd.env("RUST_LOG", x);
        }
        if let Some(x) = env::var_os("RUST_BACKTRACE") {
            cmd.env("RUST_BACKTRACE", x);
        }
        if let Some(x) = env::var_os("VAGGA_DEBUG_CMDENV") {
            cmd.env("VAGGA_DEBUG_CMDENV", x);
        }
        if let Some(x) = env::var_os("VAGGA_SETTINGS") {
            cmd.env("VAGGA_SETTINGS", x);
        }
        if let Some(x) = env::var_os("HOME") {
            cmd.env("_VAGGA_HOME", x);
        }
        if let Some(ref name) = settings.storage_subdir_from_env_var {
            if let Some(dir) = env::var_os(name) {
                cmd.env("_VAGGA_STORAGE_SUBDIR", dir);
            } else {
                cmd.env("_VAGGA_STORAGE_SUBDIR", "");
            }
        }

        cmd.unshare(&[Namespace::Mount, Namespace::Ipc, Namespace::Pid]);
        cmd
    }
    fn workdir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        let dir = dir.as_ref();
        if dir == Path::new("") { // not adding a slash at the end
            self.env("_VAGGA_WORKDIR", Path::new("/work"));
        } else {
            self.env("_VAGGA_WORKDIR", Path::new("/work").join(dir));
        }
        self
    }
    fn max_uidmap(&mut self) -> &mut Self {
        // TODO(tailhook) is this unwrap fine?
        set_uidmap(self, &get_max_uidmap().unwrap(), true);
        self
    }
    fn map_users_for(&mut self, container: &Container, settings: &Settings)
        -> Result<(), String>
    {
        self.map_users(&container.uids, &container.gids, settings)
    }
    fn map_users(&mut self, uids: &Vec<Range>, gids: &Vec<Range>,
        settings: &Settings)
        -> Result<(), String>
    {
        let uid_map = map_users(settings, uids, gids)?;
        set_uidmap(self, &uid_map, true);
        Ok(())
    }
}
