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

    // Capture current state for detecting transitions and audit logging
    let (old_name, old_status, old_approval, old_priority, old_customer, old_order_number, old_deadline, old_responsible): (String, String, String, String, String, String, String, String) = conn.query_row(
        "SELECT COALESCE(name,''), status, COALESCE(approval_status,'draft'), COALESCE(priority,'normal'), \
         COALESCE(customer,''), COALESCE(order_number,''), COALESCE(deadline,''), COALESCE(responsible_person,'') \
         FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
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

    // Audit logging
    let audit_fields: Vec<(&str, &str, Option<&str>)> = vec![
        ("name", &old_name, update.name.as_deref()),
        ("status", &old_status, update.status.as_deref()),
        ("approval_status", &old_approval, update.approval_status.as_deref()),
        ("priority", &old_priority, update.priority.as_deref()),
        ("customer", &old_customer, update.customer.as_deref()),
        ("order_number", &old_order_number, update.order_number.as_deref()),
        ("deadline", &old_deadline, update.deadline.as_deref()),
        ("responsible_person", &old_responsible, update.responsible_person.as_deref()),
    ];
    for (field, old, new_opt) in &audit_fields {
        if let Some(new_val) = new_opt {
            let _ = crate::commands::audit::log_change(&conn, "project", project_id, field, Some(old), Some(new_val));
        }
    }

    // Auto-reservation/release removed (#118: inventory UI disabled)

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

// --- Project Products ---

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProduct {
    pub id: i64,
    pub project_id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub quantity: f64,
    pub sort_order: i64,
}

#[tauri::command]
pub fn link_product_to_project(
    db: State<'_, DbState>,
    project_id: i64,
    product_id: i64,
    quantity: Option<f64>,
) -> Result<ProjectProduct, AppError> {
    let conn = lock_db(&db)?;
    // Verify both exist
    let p_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        |r| r.get(0),
    )?;
    if !p_exists {
        return Err(AppError::NotFound(format!(
            "Projekt {project_id} nicht gefunden"
        )));
    }
    let prod_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM products WHERE id = ?1 AND deleted_at IS NULL",
        [product_id],
        |r| r.get(0),
    )?;
    if !prod_exists {
        return Err(AppError::NotFound(format!(
            "Produkt {product_id} nicht gefunden"
        )));
    }

    let qty = quantity.unwrap_or(1.0);
    conn.execute(
        "INSERT OR REPLACE INTO project_products (project_id, product_id, quantity) VALUES (?1, ?2, ?3)",
        rusqlite::params![project_id, product_id, qty],
    )?;

    // Also create workflow steps from product if not already present
    let has_steps: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM workflow_steps WHERE project_id = ?1 AND step_definition_id IN (SELECT step_definition_id FROM product_steps WHERE product_id = ?2)",
        rusqlite::params![project_id, product_id],
        |r| r.get(0),
    )?;
    if !has_steps {
        // Copy product steps as workflow steps
        conn.execute(
            "INSERT INTO workflow_steps (project_id, step_definition_id, sort_order, status) \
             SELECT ?1, ps.step_definition_id, ps.sort_order, 'pending' FROM product_steps ps WHERE ps.product_id = ?2",
            rusqlite::params![project_id, product_id],
        )?;
    }

    // Update project timestamp
    conn.execute(
        "UPDATE projects SET updated_at = datetime('now') WHERE id = ?1",
        [project_id],
    )?;

    let product_name: String = conn.query_row(
        "SELECT name FROM products WHERE id = ?1",
        [product_id],
        |r| r.get(0),
    )?;
    conn.query_row(
        "SELECT id, project_id, product_id, quantity, sort_order FROM project_products WHERE project_id = ?1 AND product_id = ?2",
        rusqlite::params![project_id, product_id],
        |row| {
            Ok(ProjectProduct {
                id: row.get(0)?,
                project_id: row.get(1)?,
                product_id: row.get(2)?,
                product_name: product_name.clone(),
                quantity: row.get(3)?,
                sort_order: row.get(4)?,
            })
        },
    )
    .map_err(AppError::Database)
}

#[tauri::command]
pub fn unlink_product_from_project(
    db: State<'_, DbState>,
    project_id: i64,
    product_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    conn.execute(
        "DELETE FROM project_products WHERE project_id = ?1 AND product_id = ?2",
        rusqlite::params![project_id, product_id],
    )?;
    conn.execute(
        "UPDATE projects SET updated_at = datetime('now') WHERE id = ?1",
        [project_id],
    )?;
    Ok(())
}

#[tauri::command]
pub fn get_project_products(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<ProjectProduct>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT pp.id, pp.project_id, pp.product_id, p.name, pp.quantity, pp.sort_order \
         FROM project_products pp JOIN products p ON p.id = pp.product_id \
         WHERE pp.project_id = ?1 AND p.deleted_at IS NULL ORDER BY pp.sort_order, p.name",
    )?;
    let results = stmt
        .query_map([project_id], |row| {
            Ok(ProjectProduct {
                id: row.get(0)?,
                project_id: row.get(1)?,
                product_id: row.get(2)?,
                product_name: row.get(3)?,
                quantity: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(results)
}

// --- Project Files ---

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub id: i64,
    pub project_id: i64,
    pub file_id: i64,
    pub filename: String,
    pub role: String,
    pub sort_order: i64,
}

#[tauri::command]
pub fn add_file_to_project(
    db: State<'_, DbState>,
    project_id: i64,
    file_id: i64,
    role: String,
) -> Result<ProjectFile, AppError> {
    let valid_roles = ["pattern", "instruction", "reference"];
    if !valid_roles.contains(&role.as_str()) {
        return Err(AppError::Validation(format!(
            "Ungueltige Rolle: {role}. Erlaubt: pattern, instruction, reference"
        )));
    }
    let conn = lock_db(&db)?;
    let p_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        |r| r.get(0),
    )?;
    if !p_exists {
        return Err(AppError::NotFound(format!(
            "Projekt {project_id} nicht gefunden"
        )));
    }
    let f_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
        [file_id],
        |r| r.get(0),
    )?;
    if !f_exists {
        return Err(AppError::NotFound(format!(
            "Datei {file_id} nicht gefunden"
        )));
    }

    conn.execute(
        "INSERT OR IGNORE INTO project_files (project_id, file_id, role) VALUES (?1, ?2, ?3)",
        rusqlite::params![project_id, file_id, role],
    )?;

    let filename: String = conn.query_row(
        "SELECT filename FROM embroidery_files WHERE id = ?1",
        [file_id],
        |r| r.get(0),
    )?;
    conn.query_row(
        "SELECT id, project_id, file_id, role, sort_order FROM project_files WHERE project_id = ?1 AND file_id = ?2 AND role = ?3",
        rusqlite::params![project_id, file_id, role],
        |row| {
            Ok(ProjectFile {
                id: row.get(0)?,
                project_id: row.get(1)?,
                file_id: row.get(2)?,
                filename: filename.clone(),
                role: row.get(3)?,
                sort_order: row.get(4)?,
            })
        },
    )
    .map_err(AppError::Database)
}

#[tauri::command]
pub fn remove_file_from_project(
    db: State<'_, DbState>,
    project_id: i64,
    file_id: i64,
    role: String,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    conn.execute(
        "DELETE FROM project_files WHERE project_id = ?1 AND file_id = ?2 AND role = ?3",
        rusqlite::params![project_id, file_id, role],
    )?;
    Ok(())
}

#[tauri::command]
pub fn get_project_files(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<ProjectFile>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT pf.id, pf.project_id, pf.file_id, e.filename, pf.role, pf.sort_order \
         FROM project_files pf JOIN embroidery_files e ON e.id = pf.file_id \
         WHERE pf.project_id = ?1 AND e.deleted_at IS NULL ORDER BY pf.role, pf.sort_order, e.filename",
    )?;
    let results = stmt
        .query_map([project_id], |row| {
            Ok(ProjectFile {
                id: row.get(0)?,
                project_id: row.get(1)?,
                file_id: row.get(2)?,
                filename: row.get(3)?,
                role: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(results)
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
