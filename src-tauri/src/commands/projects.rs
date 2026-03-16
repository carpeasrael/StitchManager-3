use serde::Deserialize;
use tauri::State;

use crate::db::models::{Collection, Project, ProjectDetail};
use crate::error::{lock_db, AppError};
use crate::DbState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCreate {
    pub name: String,
    pub pattern_file_id: Option<i64>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub order_number: Option<String>,
    pub customer: Option<String>,
    pub priority: Option<String>,
    pub deadline: Option<String>,
    pub responsible_person: Option<String>,
    pub approval_status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub order_number: Option<String>,
    pub customer: Option<String>,
    pub priority: Option<String>,
    pub deadline: Option<String>,
    pub responsible_person: Option<String>,
    pub approval_status: Option<String>,
}

const VALID_STATUSES: &[&str] = &[
    "not_started", "planned", "in_progress", "completed", "archived",
];

const VALID_PRIORITIES: &[&str] = &["low", "normal", "high", "urgent"];

const VALID_APPROVAL_STATUSES: &[&str] = &["draft", "pending", "approved", "rejected"];

fn validate_status(status: &str) -> Result<(), AppError> {
    if !VALID_STATUSES.contains(&status) {
        return Err(AppError::Validation(format!(
            "Ungueltiger Projektstatus: {status}"
        )));
    }
    Ok(())
}

fn validate_priority(priority: &str) -> Result<(), AppError> {
    if !VALID_PRIORITIES.contains(&priority) {
        return Err(AppError::Validation(format!(
            "Ungueltige Prioritaet: {priority}"
        )));
    }
    Ok(())
}

fn validate_approval_status(status: &str) -> Result<(), AppError> {
    if !VALID_APPROVAL_STATUSES.contains(&status) {
        return Err(AppError::Validation(format!(
            "Ungueltiger Genehmigungsstatus: {status}"
        )));
    }
    Ok(())
}

#[tauri::command]
pub fn create_project(
    db: State<'_, DbState>,
    project: ProjectCreate,
) -> Result<Project, AppError> {
    let name = project.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("Projektname darf nicht leer sein".into()));
    }
    let status = project.status.as_deref().unwrap_or("not_started");
    validate_status(status)?;

    let priority = project.priority.as_deref().unwrap_or("normal");
    validate_priority(priority)?;
    let approval = project.approval_status.as_deref().unwrap_or("draft");
    validate_approval_status(approval)?;

    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO projects (name, pattern_file_id, status, notes, order_number, customer, priority, deadline, responsible_person, approval_status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![name, project.pattern_file_id, status, project.notes,
            project.order_number, project.customer, priority, project.deadline,
            project.responsible_person, approval],
    )?;
    let id = conn.last_insert_rowid();

    conn.query_row(
        "SELECT id, name, pattern_file_id, status, notes, order_number, customer, priority, deadline, responsible_person, approval_status, quantity, created_at, updated_at FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        row_to_project,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_projects(
    db: State<'_, DbState>,
    status_filter: Option<String>,
    pattern_file_id: Option<i64>,
) -> Result<Vec<Project>, AppError> {
    let conn = lock_db(&db)?;
    let mut sql = "SELECT id, name, pattern_file_id, status, notes, order_number, customer, priority, deadline, responsible_person, approval_status, quantity, created_at, updated_at FROM projects".to_string();
    let mut conditions: Vec<String> = vec!["deleted_at IS NULL".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(status) = &status_filter {
        conditions.push(format!("status = ?{}", params.len() + 1));
        params.push(Box::new(status.clone()));
    }
    if let Some(fid) = pattern_file_id {
        conditions.push(format!("pattern_file_id = ?{}", params.len() + 1));
        params.push(Box::new(fid));
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY updated_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let projects = stmt
        .query_map(param_refs.as_slice(), row_to_project)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(projects)
}

#[tauri::command]
pub fn get_project(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Project, AppError> {
    let conn = lock_db(&db)?;
    conn.query_row(
        "SELECT id, name, pattern_file_id, status, notes, order_number, customer, priority, deadline, responsible_person, approval_status, quantity, created_at, updated_at FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        row_to_project,
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
        _ => AppError::Database(e),
    })
}

#[tauri::command]
pub fn update_project(
    db: State<'_, DbState>,
    project_id: i64,
    update: ProjectUpdate,
) -> Result<Project, AppError> {
    let conn = lock_db(&db)?;

    // Capture current state for detecting approval/status transitions
    let (old_approval, old_status): (String, String) = conn.query_row(
        "SELECT COALESCE(approval_status, 'draft'), status FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
        _ => AppError::Database(e),
    })?;

    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(name) = &update.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("Projektname darf nicht leer sein".into()));
        }
        params.push(Box::new(trimmed.to_string()));
        sets.push(format!("name = ?{}", params.len()));
    }
    if let Some(status) = &update.status {
        validate_status(status)?;
        params.push(Box::new(status.clone()));
        sets.push(format!("status = ?{}", params.len()));
    }
    if let Some(notes) = &update.notes {
        params.push(Box::new(notes.clone()));
        sets.push(format!("notes = ?{}", params.len()));
    }
    if let Some(v) = &update.order_number { params.push(Box::new(v.clone())); sets.push(format!("order_number = ?{}", params.len())); }
    if let Some(v) = &update.customer { params.push(Box::new(v.clone())); sets.push(format!("customer = ?{}", params.len())); }
    if let Some(v) = &update.priority { validate_priority(v)?; params.push(Box::new(v.clone())); sets.push(format!("priority = ?{}", params.len())); }
    if let Some(v) = &update.deadline { params.push(Box::new(v.clone())); sets.push(format!("deadline = ?{}", params.len())); }
    if let Some(v) = &update.responsible_person { params.push(Box::new(v.clone())); sets.push(format!("responsible_person = ?{}", params.len())); }
    if let Some(v) = &update.approval_status { validate_approval_status(v)?; params.push(Box::new(v.clone())); sets.push(format!("approval_status = ?{}", params.len())); }

    if sets.is_empty() {
        // No changes — return current state
        return conn.query_row(
            "SELECT id, name, pattern_file_id, status, notes, order_number, customer, priority, deadline, responsible_person, approval_status, quantity, created_at, updated_at FROM projects WHERE id = ?1 AND deleted_at IS NULL",
            [project_id],
            row_to_project,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    sets.push("updated_at = datetime('now')".to_string());
    params.push(Box::new(project_id));
    let sql = format!(
        "UPDATE projects SET {} WHERE id = ?{} AND deleted_at IS NULL",
        sets.join(", "),
        params.len()
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Projekt {project_id} nicht gefunden")));
    }

    // Auto-reservation: if approval_status changed to 'approved'
    let new_approval = update.approval_status.as_deref().unwrap_or(&old_approval);
    if new_approval == "approved" && old_approval != "approved" {
        if let Err(e) = crate::commands::manufacturing::reserve_materials_for_project_inner(&conn, project_id) {
            log::warn!("Auto-Reservierung fehlgeschlagen fuer Projekt {project_id}: {e}");
        }
    }

    // Auto-release: if status changed to 'completed' or 'archived'
    let new_status = update.status.as_deref().unwrap_or(&old_status);
    if (new_status == "completed" || new_status == "archived") && old_status != "completed" && old_status != "archived" {
        if let Err(e) = crate::commands::manufacturing::release_project_reservations_inner(&conn, project_id) {
            log::warn!("Auto-Freigabe fehlgeschlagen fuer Projekt {project_id}: {e}");
        }
    }

    conn.query_row(
        "SELECT id, name, pattern_file_id, status, notes, order_number, customer, priority, deadline, responsible_person, approval_status, quantity, created_at, updated_at FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        row_to_project,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_project(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    // Release any reserved inventory before deleting
    let _ = crate::commands::manufacturing::release_project_reservations_inner(&conn, project_id);
    let changes = conn.execute("DELETE FROM projects WHERE id = ?1 AND deleted_at IS NULL", [project_id])?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Projekt {project_id} nicht gefunden")));
    }
    Ok(())
}

#[tauri::command]
pub fn duplicate_project(
    db: State<'_, DbState>,
    project_id: i64,
    new_name: Option<String>,
) -> Result<Project, AppError> {
    let conn = lock_db(&db)?;

    // Load source project
    let source = conn.query_row(
        "SELECT id, name, pattern_file_id, status, notes, order_number, customer, priority, deadline, responsible_person, approval_status, quantity, created_at, updated_at FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        row_to_project,
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
        _ => AppError::Database(e),
    })?;

    let name = new_name.unwrap_or_else(|| format!("{} (Kopie)", source.name));

    // Create new project with same pattern reference and manufacturing fields
    conn.execute(
        "INSERT INTO projects (name, pattern_file_id, status, notes, order_number, customer, priority, deadline, responsible_person, approval_status) \
         VALUES (?1, ?2, 'not_started', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![name, source.pattern_file_id, source.notes,
            source.order_number, source.customer, source.priority,
            source.deadline, source.responsible_person, source.approval_status],
    )?;
    let new_id = conn.last_insert_rowid();

    // Copy project details
    conn.execute(
        "INSERT INTO project_details (project_id, key, value) \
         SELECT ?1, key, value FROM project_details WHERE project_id = ?2",
        rusqlite::params![new_id, project_id],
    )?;

    conn.query_row(
        "SELECT id, name, pattern_file_id, status, notes, order_number, customer, priority, deadline, responsible_person, approval_status, quantity, created_at, updated_at FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [new_id],
        row_to_project,
    ).map_err(AppError::Database)
}

// --- Project Details ---

#[derive(Debug, Deserialize)]
pub struct ProjectDetailInput {
    pub key: String,
    pub value: Option<String>,
}

#[tauri::command]
pub fn set_project_details(
    db: State<'_, DbState>,
    project_id: i64,
    details: Vec<ProjectDetailInput>,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    // Verify project exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!("Projekt {project_id} nicht gefunden")));
    }

    let tx = conn.unchecked_transaction()?;

    for detail in &details {
        tx.execute(
            "INSERT INTO project_details (project_id, key, value) \
             VALUES (?1, ?2, ?3) \
             ON CONFLICT(project_id, key) DO UPDATE SET value = excluded.value",
            rusqlite::params![project_id, detail.key, detail.value],
        )?;
    }

    // Update project timestamp
    tx.execute(
        "UPDATE projects SET updated_at = datetime('now') WHERE id = ?1",
        [project_id],
    )?;

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn get_project_details(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<ProjectDetail>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, key, value FROM project_details WHERE project_id = ?1 ORDER BY key"
    )?;
    let details = stmt
        .query_map([project_id], |row| {
            Ok(ProjectDetail {
                id: row.get(0)?,
                project_id: row.get(1)?,
                key: row.get(2)?,
                value: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(details)
}

// --- Collections ---

#[tauri::command]
pub fn create_collection(
    db: State<'_, DbState>,
    name: String,
    description: Option<String>,
) -> Result<Collection, AppError> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Sammlungsname darf nicht leer sein".into()));
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO collections (name, description) VALUES (?1, ?2)",
        rusqlite::params![trimmed, description],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, description, created_at FROM collections WHERE id = ?1",
        [id],
        row_to_collection,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_collections(
    db: State<'_, DbState>,
) -> Result<Vec<Collection>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, description, created_at FROM collections ORDER BY name"
    )?;
    let collections = stmt
        .query_map([], row_to_collection)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(collections)
}

#[tauri::command]
pub fn delete_collection(
    db: State<'_, DbState>,
    collection_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM collections WHERE id = ?1", [collection_id])?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Sammlung {collection_id} nicht gefunden")));
    }
    Ok(())
}

#[tauri::command]
pub fn add_to_collection(
    db: State<'_, DbState>,
    collection_id: i64,
    file_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    // Validate both exist to give a clear error
    let col_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM collections WHERE id = ?1", [collection_id], |row| row.get(0),
    )?;
    if !col_exists {
        return Err(AppError::NotFound(format!("Sammlung {collection_id} nicht gefunden")));
    }
    let file_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL", [file_id], |row| row.get(0),
    )?;
    if !file_exists {
        return Err(AppError::NotFound(format!("Datei {file_id} nicht gefunden")));
    }
    conn.execute(
        "INSERT OR IGNORE INTO collection_items (collection_id, file_id) VALUES (?1, ?2)",
        rusqlite::params![collection_id, file_id],
    )?;
    Ok(())
}

#[tauri::command]
pub fn remove_from_collection(
    db: State<'_, DbState>,
    collection_id: i64,
    file_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    conn.execute(
        "DELETE FROM collection_items WHERE collection_id = ?1 AND file_id = ?2",
        rusqlite::params![collection_id, file_id],
    )?;
    Ok(())
}

#[tauri::command]
pub fn get_collection_files(
    db: State<'_, DbState>,
    collection_id: i64,
) -> Result<Vec<i64>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT ci.file_id FROM collection_items ci \
         JOIN embroidery_files e ON e.id = ci.file_id \
         WHERE ci.collection_id = ?1 AND e.deleted_at IS NULL"
    )?;
    let ids = stmt
        .query_map([collection_id], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

// --- Row mappers ---

fn row_to_project(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        pattern_file_id: row.get(2)?,
        status: row.get(3)?,
        notes: row.get(4)?,
        order_number: row.get(5)?,
        customer: row.get(6)?,
        priority: row.get(7)?,
        deadline: row.get(8)?,
        responsible_person: row.get(9)?,
        approval_status: row.get(10)?,
        quantity: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn row_to_collection(row: &rusqlite::Row) -> rusqlite::Result<Collection> {
    Ok(Collection {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        created_at: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_project_crud() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (1, 'test.pdf', '/test/test.pdf')",
            [],
        ).unwrap();

        // Create project
        conn.execute(
            "INSERT INTO projects (name, pattern_file_id, status) VALUES ('Sommerkleid', 1, 'not_started')",
            [],
        ).unwrap();
        let id = conn.last_insert_rowid();

        // Read
        let name: String = conn.query_row(
            "SELECT name FROM projects WHERE id = ?1", [id], |row| row.get(0),
        ).unwrap();
        assert_eq!(name, "Sommerkleid");

        // Update
        conn.execute(
            "UPDATE projects SET status = 'in_progress', updated_at = datetime('now') WHERE id = ?1",
            [id],
        ).unwrap();
        let status: String = conn.query_row(
            "SELECT status FROM projects WHERE id = ?1", [id], |row| row.get(0),
        ).unwrap();
        assert_eq!(status, "in_progress");

        // Delete
        conn.execute("DELETE FROM projects WHERE id = ?1", [id]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_project_details_key_value() {
        let conn = init_database_in_memory().unwrap();
        conn.execute(
            "INSERT INTO projects (name, status) VALUES ('Test', 'not_started')",
            [],
        ).unwrap();
        let pid = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO project_details (project_id, key, value) VALUES (?1, 'chosen_size', 'M')",
            [pid],
        ).unwrap();
        conn.execute(
            "INSERT INTO project_details (project_id, key, value) VALUES (?1, 'fabric_used', 'Leinen')",
            [pid],
        ).unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM project_details WHERE project_id = ?1", [pid], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 2);

        // Upsert (matches production ON CONFLICT syntax)
        conn.execute(
            "INSERT INTO project_details (project_id, key, value) VALUES (?1, 'chosen_size', 'L') \
             ON CONFLICT(project_id, key) DO UPDATE SET value = excluded.value",
            [pid],
        ).unwrap();
        let val: String = conn.query_row(
            "SELECT value FROM project_details WHERE project_id = ?1 AND key = 'chosen_size'",
            [pid], |row| row.get(0),
        ).unwrap();
        assert_eq!(val, "L");
    }

    #[test]
    fn test_collection_many_to_many() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (1, 'a.pdf', '/a.pdf')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (1, 'b.pdf', '/b.pdf')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO collections (name) VALUES ('Sommerprojekte')",
            [],
        ).unwrap();
        let cid = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO collection_items (collection_id, file_id) VALUES (?1, 1)", [cid],
        ).unwrap();
        conn.execute(
            "INSERT INTO collection_items (collection_id, file_id) VALUES (?1, 2)", [cid],
        ).unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM collection_items WHERE collection_id = ?1", [cid], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 2);

        // Delete collection cascades
        conn.execute("DELETE FROM collections WHERE id = ?1", [cid]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM collection_items", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_project_pattern_set_null() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (1, 'x.pdf', '/x.pdf')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO projects (name, pattern_file_id, status) VALUES ('Kleid', 1, 'not_started')",
            [],
        ).unwrap();
        let pid = conn.last_insert_rowid();

        // Delete the pattern file
        conn.execute("DELETE FROM embroidery_files WHERE id = 1", []).unwrap();

        // Project still exists, pattern_file_id is NULL
        let pattern_id: Option<i64> = conn.query_row(
            "SELECT pattern_file_id FROM projects WHERE id = ?1", [pid], |row| row.get(0),
        ).unwrap();
        assert!(pattern_id.is_none(), "pattern_file_id should be NULL after file deletion");
    }
}
