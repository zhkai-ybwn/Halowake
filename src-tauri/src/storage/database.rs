use std::{fs, path::PathBuf, time::Duration};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use super::migrations::run_migrations;

#[derive(Clone, Debug)]
pub struct AppDatabase {
    path: PathBuf,
}

impl AppDatabase {
    pub fn open(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("解析应用数据目录失败: {error}"))?;
        fs::create_dir_all(&data_dir)
            .map_err(|error| format!("创建应用数据目录失败 {}: {error}", data_dir.display()))?;

        let database = Self { path: data_dir.join("lumina.db") };
        let mut connection = database.connect()?;
        run_migrations(&mut connection)?;
        Ok(database)
    }

    pub fn connect(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.path)
            .map_err(|error| format!("打开 SQLite 数据库失败 {}: {error}", self.path.display()))?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| format!("设置 SQLite busy timeout 失败: {error}"))?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;\n\
                 PRAGMA journal_mode = WAL;\n\
                 PRAGMA synchronous = NORMAL;",
            )
            .map_err(|error| format!("初始化 SQLite 连接失败: {error}"))?;
        Ok(connection)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}
