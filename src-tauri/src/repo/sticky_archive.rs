// 便签归档数据访问层
//
// 设计要点:
// - source 区分 slot1 / slot2 / detached 三种来源；slot 来源 source_id 恒为 NULL，
//   detached 来源 source_id = detached_stickies.id（用于追溯）
// - reason 区分 user（手动）/ auto_replace（slot 覆盖前）/ auto_destroy（detached 销毁前）
// - 永久保留，唯一移除路径是 delete()（用户主动「彻底删除」）
// - 恢复（写回 slot / 创建新 detached）由 commands 层组合 sticky::upsert /
//   detached_sticky::upsert 完成，不在本模块职责范围内（避免反向依赖 + 循环）
use crate::models::StickyArchive;
use crate::repo::now;
use rusqlite::{params, Connection, Result};

/// 列出归档（按 archived_at DESC）
///
/// `source_filter` 为 Some("slot1"/"slot2"/"detached") 时仅返该来源；为 None 时全部返
/// `limit` / `offset` 用于分页；limit 默认 50、offset 默认 0 由 commands 层兜底
pub fn list(
    conn: &Connection,
    source_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<StickyArchive>> {
    let (sql, params_vec): (&str, Vec<rusqlite::types::Value>) = match source_filter {
        Some(src) => (
            "SELECT id, source, source_id, content, reason, archived_at
             FROM sticky_archives
             WHERE source = ?1
             ORDER BY archived_at DESC
             LIMIT ?2 OFFSET ?3",
            vec![rusqlite::types::Value::Text(src.to_string())],
        ),
        None => (
            "SELECT id, source, source_id, content, reason, archived_at
             FROM sticky_archives
             ORDER BY archived_at DESC
             LIMIT ?1 OFFSET ?2",
            vec![],
        ),
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = match source_filter {
        Some(_) => stmt.query_map(
            rusqlite::params_from_iter(params_vec.iter().chain(std::iter::once(
                &rusqlite::types::Value::Integer(limit),
            )).chain(std::iter::once(&rusqlite::types::Value::Integer(offset)))),
            row_to_sticky_archive,
        )?,
        None => stmt.query_map(
            rusqlite::params_from_iter(
                std::iter::once(&rusqlite::types::Value::Integer(limit))
                    .chain(std::iter::once(&rusqlite::types::Value::Integer(offset))),
            ),
            row_to_sticky_archive,
        )?,
    };
    rows.collect()
}

/// 取单条归档；不存在返 Ok(None)
pub fn get(conn: &Connection, id: i64) -> Result<Option<StickyArchive>> {
    let mut stmt = conn.prepare(
        "SELECT id, source, source_id, content, reason, archived_at
         FROM sticky_archives
         WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], row_to_sticky_archive)?;
    rows.next().transpose()
}

/// 创建一条归档
///
/// `source_id` 仅在 source='detached' 时有意义（指向 detached_stickies.id），
/// slot 来源必须传 None；commands 层负责校验。
pub fn create(
    conn: &Connection,
    source: &str,
    source_id: Option<i64>,
    content: &str,
    reason: &str,
) -> Result<StickyArchive> {
    conn.execute(
        "INSERT INTO sticky_archives (source, source_id, content, reason, archived_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![source, source_id, content, reason, now()],
    )?;
    // 用最近一条 archived_at 倒序定位——同一事务内 NOW() 单调递增，
    // 但更稳妥是按 id DESC 取最后插入
    let mut stmt = conn.prepare(
        "SELECT id, source, source_id, content, reason, archived_at
         FROM sticky_archives
         ORDER BY id DESC
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map([], row_to_sticky_archive)?;
    rows.next()
        .transpose()?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
}

/// 彻底删除（用户主动「彻底删除」）；自动归档也可调，但 UI 层应只允许 user reason 删除
pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM sticky_archives WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn row_to_sticky_archive(row: &rusqlite::Row) -> Result<StickyArchive> {
    Ok(StickyArchive {
        id: row.get(0)?,
        source: row.get(1)?,
        source_id: row.get(2)?,
        content: row.get(3)?,
        reason: row.get(4)?,
        archived_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory;

    fn setup() -> Connection {
        init_in_memory().unwrap()
    }

    #[test]
    fn create_with_user_reason() {
        let conn = setup();
        let a = create(&conn, "slot1", None, "手动归档内容", "user").unwrap();
        assert_eq!(a.source, "slot1");
        assert!(a.source_id.is_none());
        assert_eq!(a.content, "手动归档内容");
        assert_eq!(a.reason, "user");
        assert!(a.id > 0);
        assert!(!a.archived_at.is_empty());
    }

    #[test]
    fn create_with_auto_replace_reason() {
        let conn = setup();
        let a = create(&conn, "slot2", None, "被覆盖的旧便签", "auto_replace").unwrap();
        assert_eq!(a.source, "slot2");
        assert_eq!(a.reason, "auto_replace");
        assert_eq!(a.content, "被覆盖的旧便签");
    }

    #[test]
    fn create_with_auto_destroy_reason_keeps_source_id() {
        let conn = setup();
        // detached 来源要带 source_id
        let a = create(&conn, "detached", Some(7), "detached 销毁前的快照", "auto_destroy").unwrap();
        assert_eq!(a.source, "detached");
        assert_eq!(a.source_id, Some(7));
        assert_eq!(a.reason, "auto_destroy");
    }

    #[test]
    fn list_orders_by_archived_at_desc() {
        let conn = setup();
        let a1 = create(&conn, "slot1", None, "最早", "user").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let a2 = create(&conn, "slot1", None, "中间", "user").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let a3 = create(&conn, "slot1", None, "最晚", "user").unwrap();
        let all = list(&conn, None, 50, 0).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, a3.id, "最晚的应在最前");
        assert_eq!(all[1].id, a2.id);
        assert_eq!(all[2].id, a1.id, "最早的应在最后");
    }

    #[test]
    fn list_filter_by_source() {
        let conn = setup();
        create(&conn, "slot1", None, "卡一内容", "user").unwrap();
        create(&conn, "slot2", None, "卡二内容", "user").unwrap();
        create(&conn, "detached", Some(1), "浮窗内容", "auto_destroy").unwrap();
        let slot1 = list(&conn, Some("slot1"), 50, 0).unwrap();
        assert_eq!(slot1.len(), 1);
        assert_eq!(slot1[0].source, "slot1");
        assert_eq!(slot1[0].content, "卡一内容");

        let detached = list(&conn, Some("detached"), 50, 0).unwrap();
        assert_eq!(detached.len(), 1);
        assert_eq!(detached[0].source, "detached");
    }

    #[test]
    fn list_respects_limit_and_offset() {
        let conn = setup();
        for i in 0..5 {
            create(&conn, "slot1", None, &format!("第{i}条"), "user").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let page1 = list(&conn, None, 2, 0).unwrap();
        assert_eq!(page1.len(), 2);
        let page2 = list(&conn, None, 2, 2).unwrap();
        assert_eq!(page2.len(), 2);
        // 不重叠
        let ids1: Vec<i64> = page1.iter().map(|x| x.id).collect();
        let ids2: Vec<i64> = page2.iter().map(|x| x.id).collect();
        for id in &ids2 {
            assert!(!ids1.contains(id));
        }
    }

    #[test]
    fn get_returns_none_for_missing_id() {
        let conn = setup();
        assert!(get(&conn, 9999).unwrap().is_none());
        let a = create(&conn, "slot1", None, "内容", "user").unwrap();
        let fetched = get(&conn, a.id).unwrap().unwrap();
        assert_eq!(fetched.id, a.id);
        assert_eq!(fetched.content, "内容");
    }

    #[test]
    fn delete_purges_row() {
        let conn = setup();
        let a = create(&conn, "slot1", None, "待删除", "user").unwrap();
        assert!(get(&conn, a.id).unwrap().is_some());
        delete(&conn, a.id).unwrap();
        assert!(get(&conn, a.id).unwrap().is_none());
        assert!(list(&conn, None, 50, 0).unwrap().is_empty());
    }

    #[test]
    fn delete_missing_id_is_noop() {
        let conn = setup();
        // 不存在的 id 删也不报错）
        delete(&conn, 9999).unwrap();
    }

    #[test]
    fn archived_at_uses_now_format() {
        // archived_at 应该是 "%Y-%m-%d %H:%M:%S%.6f" 格式（与 repo::now() 一致）
        let conn = setup();
        let a = create(&conn, "slot1", None, "时间格式", "user").unwrap();
        // 形如 "2026-09-27 12:34:56.789012"
        assert!(a.archived_at.len() >= 26, "archived_at 长度不足: {}", a.archived_at);
        assert!(a.archived_at.contains('-'));
        assert!(a.archived_at.contains(':'));
        assert!(a.archived_at.contains('.'));
    }
}