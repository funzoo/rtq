use chrono::Local;
use rusqlite::Connection;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct PendingTask {
    pub id: i64,
    pub command: String,
    pub max_run_sec: i64,
    pub time_created: String,
}

#[derive(Debug)]
pub struct WorkingTask {
    id: i64,
    command: String,
    max_run_sec: i64,
    time_created: String,
    time_started: String,
}

#[derive(Debug)]
pub struct FinishedTask {
    id: i64,
    command: String,
    max_run_sec: i64,
    time_created: String,
    time_started: String,
    time_finished: String,
    exit_reason: String,
    exit_code: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskCommand {
    pub env_vars: HashMap<String, String>,
    pub command: Vec<String>,
}

impl WorkingTask {
    pub fn new(pending_task: &PendingTask, time_started: &str) -> WorkingTask {
        WorkingTask {
            id: pending_task.id,
            command: pending_task.command.clone(),
            max_run_sec: pending_task.max_run_sec,
            time_created: pending_task.time_created.clone(),
            time_started: String::from(time_started),
        }
    }
}

impl FinishedTask {
    pub fn new(
        working_task: &WorkingTask,
        time_finished: &str,
        exit_reason: &str,
        exit_code: i64,
    ) -> FinishedTask {
        FinishedTask {
            id: working_task.id,
            command: working_task.command.clone(),
            max_run_sec: working_task.max_run_sec,
            time_created: working_task.time_created.clone(),
            time_started: working_task.time_started.clone(),
            time_finished: time_finished.to_string(),
            exit_reason: exit_reason.to_string(),
            exit_code: exit_code,
        }
    }
}

#[derive(Debug)]
pub struct TaskDB {
    conn: Connection,
}

impl TaskDB {
    pub fn new() -> Option<TaskDB> {
        let homedir = env::home_dir().expect("missing HOME environment variable!");
        let db_file_dir = Path::new(&homedir).join(".rtq");

        if !db_file_dir.exists() {
            fs::create_dir_all(&db_file_dir)
                .expect(&format!("faild to create dir: {:?}", db_file_dir));
        }

        let db_file_path = db_file_dir.join("rtq.db");

        let conn = Connection::open(&db_file_path)
            .expect(&format!("failed to open db file: {:?}", db_file_path));

        conn.execute(
            "CREATE TABLE IF NOT EXISTS pending_tasks (
                      id              INTEGER PRIMARY KEY,
                      command         TEXT NOT NULL,
                      max_run_sec     INTEGER,
                      time_created    TEXT NOT NULL
                      )",
            &[],
        ).expect("failed to create db table pending_tasks");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS working_tasks (
                      id              INTEGER PRIMARY KEY,
                      command         TEXT NOT NULL,
                      max_run_sec     INTEGER,
                      time_created    TEXT NOT NULL,
                      time_started    TEXT NOT NULL
                      )",
            &[],
        ).expect("failed to create db table working_tasks");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS finished_tasks (
                      id              INTEGER PRIMARY KEY,
                      command         TEXT NOT NULL,
                      max_run_sec     INTEGER,
                      time_created    TEXT NOT NULL,
                      time_started    TEXT NOT NULL,
                      time_finished   TEXT NOT NULL,
                      exit_reason     TEXT NOT NULL,
                      exti_code       INTEGER
                      )",
            &[],
        ).expect("failed to create db table finished_tasks");

        return Some(TaskDB { conn: conn });
    }

    pub fn load_task(&self) -> Option<PendingTask> {
        let mut stmt = self.conn
            .prepare("SELECT id, command, max_run_sec, time_created FROM pending_tasks")
            .expect("failed to query db");
        let task_iter = stmt.query_map(&[], |row| PendingTask {
            id: row.get(0),
            command: row.get(1),
            max_run_sec: row.get(2),
            time_created: row.get(3),
        }).unwrap();

        for task in task_iter {
            return Some(task.unwrap());
        }

        return None;
    }

    pub fn clean_working_task(&self) {
        let working_tasks = self.load_working_tasks();
        let finished_time = format!("{}", Local::now().format("%Y-%m-%d %H:%M:%S"));

        for working_task in working_tasks {
            let finished_task = FinishedTask::new(&working_task, &finished_time, &"cleanup", -1);
            self.remove_working_task(&working_task);
            self.insert_finished_task(&finished_task);
        }
    }

    fn load_working_tasks(&self) -> Vec<WorkingTask> {
        let mut stmt = self.conn
            .prepare(
                "SELECT id, command, max_run_sec, time_created, time_started FROM working_tasks",
            )
            .expect("failed to query db");
        let query_iter = stmt.query_map(&[], |row| WorkingTask {
            id: row.get(0),
            command: row.get(1),
            max_run_sec: row.get(2),
            time_created: row.get(3),
            time_started: row.get(4),
        }).unwrap();

        let mut working_tasks: Vec<WorkingTask> = Vec::new();
        for working_task in query_iter {
            working_tasks.push(working_task.unwrap());
        }

        return working_tasks;
    }

    pub fn remove_pending_task(&self, pending_task: &PendingTask) {
        let mut stmt = self.conn
            .prepare("DELETE FROM pending_tasks where id= :id")
            .unwrap();
        stmt.execute_named(&[(":id", &pending_task.id)])
            .unwrap_or_default();
    }

    pub fn remove_working_task(&self, working_task: &WorkingTask) {
        let mut stmt = self.conn
            .prepare("DELETE FROM working_tasks where id= :id")
            .unwrap();
        stmt.execute_named(&[(":id", &working_task.id)])
            .unwrap_or_default();
    }

    pub fn insert_pending_task(&self, pending_task: &PendingTask) {
        let mut stmt = self.conn
            .prepare(
                "INSERT INTO pending_tasks values (:id, :command, :max_run_sec, :time_created)",
            )
            .unwrap();
        stmt.execute_named(&[
            (":id", &pending_task.id),
            (":command", &pending_task.command),
            (":max_run_sec", &pending_task.max_run_sec),
            (":time_created", &pending_task.time_created),
        ]).unwrap();
    }

    pub fn insert_working_task(&self, working_task: &WorkingTask) {
        let mut stmt = self.conn
            .prepare(
                "INSERT INTO working_tasks values (:id, :command, :max_run_sec, \
                 :time_created, :time_started)",
            )
            .unwrap();

        stmt.execute_named(&[
            (":id", &working_task.id),
            (":command", &working_task.command),
            (":max_run_sec", &working_task.max_run_sec),
            (":time_created", &working_task.time_created),
            (":time_started", &working_task.time_started),
        ]).unwrap();
    }

    pub fn insert_finished_task(&self, finished_task: &FinishedTask) {
        let mut stmt = self.conn
            .prepare(
                "INSERT INTO finished_tasks values (:id, :command, :max_run_sec, \
                 :time_created, :time_started, :time_finished, \
                 :exit_reason, :exit_code)",
            )
            .unwrap();
        stmt.execute_named(&[
            (":id", &finished_task.id),
            (":command", &finished_task.command),
            (":max_run_sec", &finished_task.max_run_sec),
            (":time_created", &finished_task.time_created),
            (":time_started", &finished_task.time_started),
            (":time_finished", &finished_task.time_finished),
            (":exit_reason", &finished_task.exit_reason),
            (":exit_code", &finished_task.exit_code),
        ]).unwrap();
    }
}
