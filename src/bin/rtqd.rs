#[macro_use] extern crate log;
extern crate chrono;
extern crate clap;
extern crate libc;
extern crate rtqlib;
extern crate rusqlite;
extern crate serde;
extern crate serde_json;
extern crate simplelog;

use chrono::Local;
use rtqlib::taskdb::{TaskDB, PendingTask, WorkingTask, FinishedTask, TaskCommand};
use simplelog::*;
use std::env;
use std::fs::File;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Child};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

const DEFAULT_DATETIME_FORMAT: &'static str = "%Y-%m-%dT %H:%M:%S";

fn now_sec() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs()
}

fn rtqd_work_dir() -> PathBuf{
    return env::home_dir().unwrap().join("tmp/rtq_work_dir/");
}

fn process_task(pending_task: &PendingTask) {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let workdir = rtqd_work_dir().join("tasks").join(today).join(format!("task_{}", pending_task.id));
    fs::create_dir_all(&workdir).expect(&format!("failed to create dir: {}", workdir.to_str().unwrap()));

    let task_command:TaskCommand = serde_json::from_str(&pending_task.command).unwrap();

    debug!("command {:?}", task_command);

    let child_process = run_command(&task_command, &workdir);

    if child_process.is_err() {
        info!("failed to run command {:?}, error {:?}", task_command.command, child_process.err().unwrap());
        return;
    }

    wait_and_kill_later(& mut child_process.ok().unwrap(), /*max_run_sec*/0);
}

fn run_command(task_command: &TaskCommand, workdir: &Path) -> std::io::Result<Child> {
    let cmds : Vec<&str> = task_command.command.iter().map(AsRef::as_ref).collect();
    let envars = task_command.env_vars.clone();

    info!("starting command. workdir{} command line {:?} ", workdir.to_str().unwrap(), cmds);

    let stdout_file = File::create(workdir.join("stdout.log"))?;
    let stderr_file = File::create(workdir.join("stderr.log"))?;

    Command::new(cmds[0])
        .args(&cmds[1..])
        .env_clear()
        .envs(&envars)
        .current_dir(workdir)
        .stdout(stdout_file)
        .stderr(stderr_file)
        .before_exec(|| {
            unsafe {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGHUP);
            }
            Ok(())
        })
    .spawn()
}

fn wait_and_kill_later(child: &mut Child, max_run_sec: u64) {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                info!("exit status {:?}", status);
                if !status.success() {
                    let alert_msg = format!("command failure. exit status: {:?}", status);
                    info!("{}", alert_msg);
                }
                else
                {
                    info!("child exited normally");
                }
                break;
            }
            Ok(None) => {
                // do nothing
            }
            Err(e) => {
                warn!("error when attempting to wait: {}", e);
                break;
            }
        }

        if max_run_sec != 0 && now_sec() > max_run_sec {
            warn!("killing the child");
            child.kill().unwrap_or_else(|_| { error!("failed to kill command"); });
        }

        thread::sleep(Duration::from_millis(1000));
    }
}

fn main() {
    let log_file_dir = rtqd_work_dir().join("rtqd");
    if !log_file_dir.exists() {
        fs::create_dir_all(&log_file_dir).expect(&format!("failed to create rtqd work dir: {}", log_file_dir.to_str().unwrap()));
    }

    let now_str = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let log_file_name = log_file_dir.join(format!("rtqd_{}.log", now_str));

    let mut log_config = Config::default();
    log_config.time_format = Some(DEFAULT_DATETIME_FORMAT);
    CombinedLogger::init(
        vec![
            WriteLogger::new(LevelFilter::Debug, log_config, File::create(&log_file_name).unwrap())
        ]
    ).expect("failed to create logging ");

    println!("rtqd log file: {}", log_file_name.to_str().unwrap());

    info!("rtqd started");

    let task_db = TaskDB::new().expect("failed to open task db");
    task_db.clean_working_task();

    loop {
        let pending_task = task_db.load_task();

        if let Some(ref pending_task) = pending_task {
            task_db.remove_pending_task(&pending_task);

            let time_started = format!("{}", Local::now().format(DEFAULT_DATETIME_FORMAT));
            let working_task = WorkingTask::new(&pending_task, &time_started);

            task_db.insert_working_task(&working_task);

            process_task(pending_task);

            let time_finished = Local::now().format(DEFAULT_DATETIME_FORMAT).to_string();
            let finished_task = FinishedTask::new(&working_task, &time_finished, "normal exit", 0);

            task_db.remove_working_task(&working_task);
            task_db.insert_finished_task(&finished_task);
        }
        else
        {
            debug!("no pending tasks, do nothing");
        }
        thread::sleep(Duration::from_millis(3000));
    }
}
