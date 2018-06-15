extern crate clap;
extern crate chrono;
extern crate rtqlib;
extern crate serde;
extern crate serde_json;

use chrono::Local;
use std::time::{SystemTime, UNIX_EPOCH};
use std::env;

use rtqlib::taskdb::{PendingTask, TaskDB, TaskCommand};

fn main() {
    let mut command_vec:Vec<_> = env::args().collect();
    command_vec.drain(0..1);

    println!("adding command {:?}", command_vec);

    let task_cmd = TaskCommand {
        env_vars : env::vars().collect(),
        command : command_vec.clone(),
    };

    let cmd_txt = serde_json::to_string(&task_cmd).unwrap();

    let epoch_ts = SystemTime::now().duration_since(UNIX_EPOCH).expect("time went backwards");
    let epoch_nano = (epoch_ts.as_secs() * 1000_000_000 + epoch_ts.subsec_nanos() as u64) as i64;

    let time_started = format!("{}", Local::now().format("%Y-%m-%d %H:%M:%S"));

    let task_db = TaskDB::new().expect("failed to open task db");

    let pending_task = PendingTask{
        id: epoch_nano,
        command: cmd_txt,
        max_run_sec: 0,
        time_created: time_started};

    task_db.insert_pending_task(&pending_task);
    println!("added task {:?}", command_vec);
}
