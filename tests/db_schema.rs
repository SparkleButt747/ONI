use oni_db::Database;

#[test]
fn test_schema_creation() {
    let db = Database::open_in_memory().unwrap();
    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('conversations','messages','tool_events')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_insert_and_query_conversation() {
    let db = Database::open_in_memory().unwrap();
    let conv_id = db.create_conversation("/tmp/test").unwrap();
    db.add_message(&conv_id, "user", "hello").unwrap();
    db.add_message(&conv_id, "assistant", "hi there").unwrap();
    let messages = db.get_messages(&conv_id).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content, "hello");
    assert_eq!(messages[1].content, "hi there");
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[1].role, "assistant");
}

#[test]
fn test_tool_event_logging() {
    let db = Database::open_in_memory().unwrap();
    db.log_tool_event(
        "session-1",
        "read_file",
        r#"{"path": "/tmp/test.rs"}"#,
        "file contents here",
        42,
    )
    .unwrap();

    let count: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM tool_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_list_conversations() {
    let db = Database::open_in_memory().unwrap();
    db.create_conversation("/project/a").unwrap();
    db.create_conversation("/project/b").unwrap();
    let convos = db.list_conversations().unwrap();
    assert_eq!(convos.len(), 2);
}

#[test]
fn test_message_token_estimation() {
    let db = Database::open_in_memory().unwrap();
    let conv_id = db.create_conversation("/tmp").unwrap();
    db.add_message(&conv_id, "user", "a short message").unwrap();
    let messages = db.get_messages(&conv_id).unwrap();
    assert!(messages[0].tokens > 0);
}

// ── T-DB: preference_signals table ───────────────────────────────────────────

#[test]
/// T-DB-6: preference_signals table exists in the schema after open_in_memory.
fn t_db_6_preference_signals_table_exists() {
    let db = Database::open_in_memory().unwrap();
    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='preference_signals'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
/// T-DB-7: a preference_signals row can be inserted and queried back.
fn t_db_7_preference_signals_insert_and_query() {
    let db = Database::open_in_memory().unwrap();
    db.conn()
        .execute(
            "INSERT INTO preference_signals (session_id, tool_name, signal_type, context, weight)
             VALUES ('sess-1', 'bash', 'accept', 'run tests', 1.5)",
            [],
        )
        .unwrap();

    let (tool_name, signal_type, weight): (String, String, f64) = db
        .conn()
        .query_row(
            "SELECT tool_name, signal_type, weight FROM preference_signals WHERE session_id='sess-1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();

    assert_eq!(tool_name, "bash");
    assert_eq!(signal_type, "accept");
    assert!((weight - 1.5).abs() < f64::EPSILON);
}

#[test]
/// T-DB-8: preference_signals rejects rows with an invalid signal_type (CHECK constraint).
fn t_db_8_preference_signals_check_constraint() {
    let db = Database::open_in_memory().unwrap();
    let result = db.conn().execute(
        "INSERT INTO preference_signals (tool_name, signal_type) VALUES ('read_file', 'invalid_type')",
        [],
    );
    assert!(result.is_err(), "expected CHECK constraint violation");
}

#[test]
/// T-DB-9: all four valid signal_type values are accepted by the CHECK constraint.
fn t_db_9_preference_signals_all_valid_types() {
    let db = Database::open_in_memory().unwrap();
    for sig in &["accept", "reject", "edit", "rerun"] {
        let result = db.conn().execute(
            &format!(
                "INSERT INTO preference_signals (tool_name, signal_type) VALUES ('tool', '{}')",
                sig
            ),
            [],
        );
        assert!(result.is_ok(), "expected '{sig}' to be a valid signal_type");
    }
}

// ── T-DB: learned_rules table ─────────────────────────────────────────────────

#[test]
/// T-DB-10: learned_rules table exists in the schema after open_in_memory.
fn t_db_10_learned_rules_table_exists() {
    let db = Database::open_in_memory().unwrap();
    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='learned_rules'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
/// T-DB-11: a learned_rules row can be inserted with default values and queried back.
fn t_db_11_learned_rules_insert_and_query() {
    let db = Database::open_in_memory().unwrap();
    db.conn()
        .execute(
            "INSERT INTO learned_rules (description, context) VALUES ('prefer short diffs', 'code review')",
            [],
        )
        .unwrap();

    let (desc, confidence, observations, active): (String, f64, i64, i64) = db
        .conn()
        .query_row(
            "SELECT description, confidence, observations, active FROM learned_rules LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();

    assert_eq!(desc, "prefer short diffs");
    assert!((confidence - 0.5).abs() < f64::EPSILON, "default confidence should be 0.5");
    assert_eq!(observations, 0, "default observations should be 0");
    assert_eq!(active, 0, "default active should be 0 (inactive)");
}

#[test]
/// T-DB-12: learned_rules allows updating confidence and observations over time.
fn t_db_12_learned_rules_update() {
    let db = Database::open_in_memory().unwrap();
    db.conn()
        .execute(
            "INSERT INTO learned_rules (description, context, confidence, observations) VALUES ('rule', 'ctx', 0.5, 1)",
            [],
        )
        .unwrap();

    db.conn()
        .execute(
            "UPDATE learned_rules SET confidence=0.9, observations=10 WHERE description='rule'",
            [],
        )
        .unwrap();

    let (confidence, observations): (f64, i64) = db
        .conn()
        .query_row(
            "SELECT confidence, observations FROM learned_rules WHERE description='rule'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();

    assert!((confidence - 0.9).abs() < 1e-9);
    assert_eq!(observations, 10);
}

#[test]
/// T-DB-13: the full schema contains all 5 expected tables.
fn t_db_13_all_tables_present() {
    let db = Database::open_in_memory().unwrap();
    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN \
             ('conversations','messages','tool_events','preference_signals','learned_rules')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 5);
}
