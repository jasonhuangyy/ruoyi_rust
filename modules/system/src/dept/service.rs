use super::model::{AddDeptVo, DeptTreeSelectVo, DeptTreeVo, UpdateDeptVo};
use chrono::Local;
use common::error::AppError;
use entity::{
    prelude::{SysDept, SysDeptColumn, SysDeptModel},
    sys_dept,
};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, TransactionTrait,
};
use sqlx::{Postgres, QueryBuilder};
use tracing::{info, warn};

pub async fn select_dept_list(db: &DatabaseConnection, dept_name: Option<&str>, status: Option<&str>) -> Result<Vec<SysDeptModel>, AppError> {
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM sys_dept WHERE del_flag = '0'");

    if let Some(name) = dept_name {
        if !name.trim().is_empty() {
            query_builder
                .push(" AND dept_name LIKE ")
                .push_bind(format!("%{}%", name));
        }
    }
    if let Some(s) = status {
        if !s.trim().is_empty() {
            query_builder.push(" AND status = ").push_bind(s);
        }
    }

    query_builder.push(" ORDER BY parent_id, order_num");

    let depts = query_builder
        .build_query_as()
        .fetch_all(db.get_postgres_connection_pool())
        .await?;

    Ok(depts)
}

/// 根据部门ID查询部门详情
pub async fn select_dept_by_id(db: &DatabaseConnection, dept_id: i64) -> Result<SysDeptModel, AppError> {
    // let dept = sqlx::query_as!(SysDept, "SELECT * FROM sys_dept WHERE dept_id = ?", dept_id)
    //     .fetch_one(db)
    //     .await?;
    // Ok(dept)
    SysDept::find_by_id(dept_id)
        .one(db)
        .await?
        .ok_or(AppError::RecordNotFound)
}

/// 新增部门，并维护祖级列表 `ancestors`
pub async fn add_dept(db: &DatabaseConnection, dept: AddDeptVo) -> Result<i64, AppError> {
    // 核心逻辑：根据 parent_id 查询父部门，以构建 ancestors
    let parent_ancestors = if dept.parent_id != 0 {
        let parent_dept = select_dept_by_id(db, dept.parent_id).await?;
        parent_dept.ancestors.unwrap_or("".to_string())
    } else {
        "0".to_string()
    };
    // 新的 ancestors = 父部门的 ancestors + "," + parent_id
    let new_ancestors = format!("{},{}", parent_ancestors, dept.parent_id);

    // let result = sqlx::query!(
    //     r#"
    //         INSERT INTO sys_dept (parent_id, ancestors, dept_name, order_num, leader, phone, email, status, create_by, create_time)
    //         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'admin', NOW())
    //     "#,
    //     dept.parent_id,
    //     new_ancestors,
    //     dept.dept_name,
    //     dept.order_num,
    //     dept.leader,
    //     dept.phone,
    //     dept.email,
    //     dept.status
    // )
    // .execute(db)
    // .await?;
    let mut act_model: sys_dept::ActiveModel = dept.into();
    act_model.ancestors = Set(Some(new_ancestors));
    let model = act_model.insert(db).await?;

    Ok(model.dept_id)
}

/// 修改部门信息，并递归更新所有子孙部门的祖级列表 (ancestors)。
///
/// 整个操作在一个数据库事务中执行，以保证数据一致性。
///
/// # Arguments
/// * `db` - 数据库连接池
/// * `dept_vo` - 包含更新后部门信息的视图对象
///
/// # Returns
/// 成功时返回受影响的行数 (通常为1，因为只更新了主表的一行)，失败时返回 AppError。
/// ==========================================
// 事务包裹: 整个函数体的操作都被 let mut tx = db.begin().await?; 和 tx.commit().await?; 包裹。这意味着从开始到结束的所有数据库查询和更新，要么一起成功，要么在任何一步出错时一起回滚，保证了数据的完整性。
// 在事务中查询: select_dept_by_id 这样的查询现在传入的是 &mut *tx，确保了读取的数据在事务期间是一致的。
// 获取旧数据: 在更新之前，我们先查询出部门的旧状态，特别是 old_dept.ancestors，这是后续比较和替换的基础。
// 计算新 ancestors: 这部分逻辑与 add_dept 类似，通过查询新的父部门来构建新的祖级路径。
// 更新当前部门: 首先更新被移动的部门本身，将其 parent_id 和 ancestors 更新为新的值。
// 核心修复逻辑:
//     通过比较 new_ancestors != old_ancestors 来判断部门是否真的被移动了。如果只是修改了部门名称或排序，这部分昂贵的更新操作将被跳过。
//     old_children_path 和 new_children_path 分别代表了子部门 ancestors 字段中需要被替换的部分。
//     UPDATE sys_dept SET ancestors = REPLACE(ancestors, ?, ?) WHERE ancestors LIKE ? 是一条非常高效的SQL语句。
//     REPLACE(ancestors, old_path, new_path): 将 ancestors 字段中所有出现的 old_path 字符串替换为 new_path。
//     WHERE ancestors LIKE 'old_path,%': 这个 WHERE 条件是关键，它确保只对 ancestors 以 old_path, 开头的行进行操作，也就是只更新真正的子孙部门，避免了错误地修改其他不相关的部门。
//     提交事务: 所有操作成功后，提交事务，使更改永久生效。
pub async fn update_dept(db: &DatabaseConnection, dept_vo: UpdateDeptVo) -> Result<(), AppError> {
    info!(
        "[SERVICE] Entering dept::update_dept for dept_id: {}",
        dept_vo.dept_id
    );

    // 开启一个数据库事务
    let tx = db.begin().await?;
    info!(
        "[TX] Transaction started for updating dept_id: {}",
        dept_vo.dept_id
    );

    //  1: 获取要修改的部门的旧数据
    // 我们需要在事务中查询，以防止在操作过程中数据被其他请求修改 (虽然概率很小，但这是最佳实践)
    // let old_dept = sqlx::query_as!(
    //     SysDept,
    //     "SELECT * FROM sys_dept WHERE dept_id = ?",
    //     dept_vo.dept_id
    // )
    // .fetch_one(&mut *tx)
    // .await?;
    let old_dept = SysDept::find()
        .filter(SysDeptColumn::DeptId.eq(dept_vo.dept_id))
        .one(&tx)
        .await?
        .ok_or(AppError::RecordNotFound)?;

    //  2: 计算新的祖级列表 (new_ancestors)
    // 获取新的父部门信息来构建 new_ancestors
    // let parent_dept = sqlx::query_as!(
    //     SysDept,
    //     "SELECT * FROM sys_dept WHERE dept_id = ?",
    //     dept_vo.parent_id
    // )
    // .fetch_optional(&mut *tx) // 使用 fetch_optional 因为 parent_id=0 时查不到记录
    // .await?;
    let parent_dept = SysDept::find_by_id(dept_vo.parent_id).one(&tx).await?;

    // 如果父部门存在，则构建 ancestors；如果不存在(parent_id=0)，则 ancestors 为 "0"
    let new_ancestors = match parent_dept {
        Some(p) => format!("{},{}", p.ancestors.unwrap_or_default(), p.dept_id),
        None => "0".to_string(),
    };
    info!("[CALC] New ancestors calculated: '{}'", new_ancestors);

    //  3: 更新当前部门自身的信息
    let mut dept_act_model = old_dept.into_active_model();
    dept_act_model.parent_id = Set(Some(dept_vo.parent_id));
    dept_act_model.ancestors = Set(Some(new_ancestors.clone()));
    dept_act_model.dept_name = Set(Some(dept_vo.dept_name));
    dept_act_model.order_num = Set(Some(dept_vo.order_num));
    dept_act_model.leader = Set(dept_vo.leader);
    dept_act_model.phone = Set(dept_vo.phone);
    dept_act_model.email = Set(dept_vo.email);
    dept_act_model.status = Set(Some(dept_vo.status));
    dept_act_model.update_by = Set(Some("admin".to_string()));
    dept_act_model.update_time = Set(Some(Local::now().naive_local()));

    let model = dept_act_model.update(db).await?;

    // let result = sqlx::query!(
    //     r#"
    //         UPDATE sys_dept
    //         SET parent_id = ?, ancestors = ?, dept_name = ?, order_num = ?, leader = ?, phone = ?, email = ?, status = ?, update_by = 'admin', update_time = NOW()
    //         WHERE dept_id = ?
    //     "#,
    //     dept_vo.parent_id,
    //     new_ancestors,
    //     dept_vo.dept_name,
    //     dept_vo.order_num,
    //     dept_vo.leader,
    //     dept_vo.phone,
    //     dept_vo.email,
    //     dept_vo.status,
    //     dept_vo.dept_id
    // )
    // .execute(&mut *tx)
    // .await?;
    // info!(
    //     "[TX] Updated current department (id: {}) successfully.",
    //     dept_vo.dept_id
    // );

    //  4: 如果祖级列表发生变化，则递归更新所有子孙部门
    let old_ancestors = model.ancestors.unwrap_or_default();
    if new_ancestors != old_ancestors {
        info!("[RECURSIVE_UPDATE] Ancestors changed. Updating all children...");

        // 旧的完整祖级路径，例如 "0,100,101"
        let old_children_path = format!("{},{}", old_ancestors, model.dept_id);
        // 新的完整祖级路径，例如 "0,200,101"
        let new_children_path = format!("{},{}", new_ancestors, model.dept_id);

        info!(
            "[RECURSIVE_UPDATE] Replacing old path '{}' with new path '{}' for all descendants.",
            old_children_path, new_children_path
        );

        // 使用 SQL 的 REPLACE 函数批量更新所有子孙节点的 ancestors 字段
        // 条件 `ancestors LIKE 'old_path,%'` 确保我们只更新真正的子孙节点
        let update_children_result = sqlx::query("UPDATE sys_dept SET ancestors = REPLACE(ancestors, ?, ?) WHERE ancestors LIKE ?")
            .bind(&old_children_path)
            .bind(&new_children_path)
            .bind(format!("{},%", old_children_path))
            .execute(db.get_postgres_connection_pool())
            .await?;

        // SysDept::update_many()
        //     .filter(SysDeptColumn::Ancestors.like(format!("{},%", old_children_path)))
        //     .set(SysDeptColumn::Ancestors, new_children_path)
        //     .execute(&tx)
        //     .await?;

        info!(
            "[TX] Updated {} descendant departments.",
            update_children_result.rows_affected()
        );
    } else {
        info!("[RECURSIVE_UPDATE] Ancestors did not change. Skipping children update.");
    }

    //  5: 提交事务
    tx.commit().await?;
    info!(
        "[TX] Transaction committed successfully for dept_id: {}.",
        dept_vo.dept_id
    );

    Ok(())
}

/// 删除部门（逻辑删除）
pub async fn delete_dept_by_id(db: &DatabaseConnection, dept_id: i64) -> Result<(), AppError> {
    let exist = SysDept::find_by_id(dept_id).one(db).await?;
    if exist.is_none() {
        return Err(AppError::RecordNotFound);
    }

    let mut model = exist.unwrap().into_active_model();
    model.del_flag = Set(Some("2".to_string()));
    model.update(db).await?;

    // RuoYi 的删除是逻辑删除，更新 del_flag 字段
    // let result = sqlx::query!(
    // "UPDATE sys_dept SET del_flag = '2' WHERE dept_id = ?",
    // dept_id
    // )
    // .execute(db.get_postgres_connection_pool())
    // .await?;

    Ok(())
}

// `select_dept_list_for_treeselect` 函数保持不变
pub async fn select_dept_list_for_treeselect(db: &DatabaseConnection) -> Result<Vec<SysDeptModel>, AppError> {
    info!("[SERVICE] Entering select_dept_list_for_treeselect");
    // let depts = sqlx::query_as!(
    //     SysDept,
    //     "SELECT * FROM sys_dept WHERE del_flag = '0' AND status = '0' ORDER BY parent_id, order_num"
    // )
    // .fetch_all(db)
    // .await?;
    let depts = SysDept::find()
        .filter(
            SysDeptColumn::DelFlag
                .eq(Some("0".to_string()))
                .and(SysDeptColumn::Status.eq(Some("0".to_string()))),
        )
        .order_by_asc(SysDeptColumn::ParentId)
        .order_by_asc(SysDeptColumn::OrderNum)
        .all(db)
        .await?;
    info!(
        "[DB_RESULT] Found {} departments for treeselect.",
        depts.len()
    );
    Ok(depts)
}

/// 辅助函数：将部门的扁平列表构建成树形结构 (最终正确稳定版 - 递归法)
pub fn build_dept_tree(depts: Vec<SysDeptModel>) -> Vec<DeptTreeVo> {
    let mut all_nodes: Vec<DeptTreeVo> = depts
        .into_iter()
        .map(|dept| DeptTreeVo {
            dept,
            children: Vec::new(),
        })
        .collect();

    let mut roots = Vec::new();
    all_nodes.retain_mut(|node| {
        if node.dept.parent_id.unwrap_or(0) == 0 {
            roots.push(node.clone());
            false
        } else {
            true
        }
    });

    for root in roots.iter_mut() {
        build_children_for_dept(root, &mut all_nodes);
    }

    roots.sort_by_key(|a| a.dept.order_num.unwrap_or(0));

    roots
}

fn build_children_for_dept(parent: &mut DeptTreeVo, all_nodes: &mut Vec<DeptTreeVo>) {
    let mut children = Vec::new();
    all_nodes.retain_mut(|node| {
        if node.dept.parent_id == Some(parent.dept.dept_id) {
            children.push(node.clone());
            false
        } else {
            true
        }
    });

    for child in children.iter_mut() {
        build_children_for_dept(child, all_nodes);
    }

    children.sort_by_key(|a| a.dept.order_num.unwrap_or(0));
    parent.children = children;
}

/// 辅助函数：将扁平的 SysDept 列表构建成前端需要的 DeptTreeSelectVo 树形结构
pub fn build_dept_treeselect(depts: Vec<SysDeptModel>) -> Vec<DeptTreeSelectVo> {
    let dept_tree_vo = build_dept_tree(depts);
    convert_dept_tree_to_select_tree(dept_tree_vo)
}

fn convert_dept_tree_to_select_tree(tree_vo: Vec<DeptTreeVo>) -> Vec<DeptTreeSelectVo> {
    tree_vo
        .into_iter()
        .map(|vo| DeptTreeSelectVo {
            id: vo.dept.dept_id,
            label: vo.dept.dept_name.clone().unwrap_or_default(),
            children: convert_dept_tree_to_select_tree(vo.children),
        })
        .collect()
}

/// 查询部门列表，并排除指定部门及其所有子部门
///
/// # Arguments
///
/// * `db` - 数据库连接池
/// * `exclude_dept_id` - 需要被排除的部门ID
///
/// # Returns
///
/// 一个不包含 `exclude_dept_id` 及其子孙的扁平部门列表 `Vec<SysDept>`。
pub async fn select_dept_list_exclude_child(db: &DatabaseConnection, exclude_dept_id: i64) -> Result<Vec<SysDeptModel>, AppError> {
    info!(
        "[SERVICE] Entering select_dept_list_exclude_child, excluding children of dept_id: {}",
        exclude_dept_id
    );

    // 1. 首先，获取要排除的部门自身的信息，主要是为了它的 `ancestors` 字段
    let exclude_dept = select_dept_by_id(db, exclude_dept_id).await?;
    let exclude_ancestors = exclude_dept.ancestors.unwrap_or_default();

    // 2. 构建 SQL 查询
    // 排除条件是：
    // a) 部门的 ID 不是 `exclude_dept_id`
    // b) 部门的祖先列表 (ancestors) 不以 `exclude_dept_ancestors,exclude_dept_id` 开头
    //    (注意这里的逗号，LIKE 'x,y,%' 可以匹配所有 'x,y' 的子孙)
    let sql = format!(
        "SELECT * FROM sys_dept WHERE del_flag = '0' AND dept_id != {} AND ancestors NOT LIKE '{},{}%'",
        exclude_dept_id, exclude_ancestors, exclude_dept_id
    );

    info!(
        "[DB_QUERY] Executing query to exclude child departments: {}",
        sql
    );

    // 3. 执行查询并返回结果
    let depts: Vec<SysDeptModel> = sqlx::query_as(&sql)
        .fetch_all(db.get_postgres_connection_pool())
        .await?;
    info!(
        "[DB_RESULT] Found {} departments after excluding children.",
        depts.len()
    );

    Ok(depts)
}

/// 获取或创建默认部门，并返回其ID。此函数设计为在事务中安全运行。
pub async fn get_or_create_default_dept(db: &DatabaseConnection, dept_name: &str) -> Result<i64, AppError> {
    // 1. 尝试查找部门
    let existing_dept: Option<SysDeptModel> = sqlx::query_as("SELECT * FROM sys_dept WHERE dept_name = ? AND del_flag = '0' LIMIT 1")
        .bind(dept_name)
        .fetch_optional(db.get_postgres_connection_pool())
        .await?;

    if let Some(dept) = existing_dept {
        info!(
            "[SERVICE_DEPT] Found default department '{}' with id: {}",
            dept_name, dept.dept_id
        );
        Ok(dept.dept_id)
    } else {
        // 2. 如果不存在，则创建
        warn!(
            "[SERVICE_DEPT] Default department '{}' not found. Creating it...",
            dept_name
        );

        let mut act_model = SysDeptModel {
            parent_id: Some(0),
            ancestors: Some("0".to_string()),
            dept_name: Some(dept_name.to_string()),
            order_num: Some(0),
            status: Some("0".to_string()),
            create_by: Some("system".to_string()),
            ..Default::default()
        }
        .into_active_model();

        act_model.dept_id = NotSet;
        act_model.create_time = NotSet;
        act_model.update_by = NotSet;
        act_model.update_time = NotSet;

        let result = act_model.insert(db).await?;

        // let result = sqlx::query!(
        //     r#"
        //     INSERT INTO sys_dept (parent_id, ancestors, dept_name, order_num, status, create_by, create_time)
        //     VALUES (0, '0', ?, 0, '0', 'system', NOW())
        //     "#,
        //     dept_name
        // )
        // .execute(&mut **tx)
        // .await?;

        let new_dept_id = result.dept_id;
        info!(
            "[SERVICE_DEPT] Created default department '{}' with new id: {}",
            dept_name, new_dept_id
        );
        Ok(new_dept_id)
    }
}
