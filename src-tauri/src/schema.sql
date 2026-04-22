-- ================================================================
-- 기본 테이블
-- ================================================================

CREATE TABLE IF NOT EXISTS Student
(
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    grade     INTEGER NOT NULL,
    class_num INTEGER NOT NULL,
    number    INTEGER NOT NULL,
    name      TEXT    NOT NULL,
    UNIQUE (grade, class_num, number)
);

CREATE TABLE IF NOT EXISTS Area
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL UNIQUE,
    byte_limit INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS Activity
(
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

-- ================================================================
-- 관계 테이블
-- ================================================================

CREATE TABLE IF NOT EXISTS AreaActivity
(
    area_id       INTEGER NOT NULL,
    activity_id   INTEGER NOT NULL,
    default_order INTEGER NOT NULL,
    PRIMARY KEY (area_id, activity_id),
    FOREIGN KEY (area_id) REFERENCES Area (id) ON DELETE CASCADE,
    FOREIGN KEY (activity_id) REFERENCES Activity (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS AreaStudent
(
    area_id             INTEGER NOT NULL,
    student_id          INTEGER NOT NULL,
    is_order_customized INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (area_id, student_id),
    FOREIGN KEY (area_id) REFERENCES Area (id) ON DELETE CASCADE,
    FOREIGN KEY (student_id) REFERENCES Student (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ActivityDisplayOrder
(
    area_id       INTEGER NOT NULL,
    student_id    INTEGER NOT NULL,
    activity_id   INTEGER NOT NULL,
    display_order INTEGER NOT NULL,
    PRIMARY KEY (area_id, student_id, activity_id),
    FOREIGN KEY (area_id, student_id) REFERENCES AreaStudent (area_id, student_id) ON DELETE CASCADE,
    FOREIGN KEY (area_id, activity_id) REFERENCES AreaActivity (area_id, activity_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ActivityRecord
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    activity_id INTEGER NOT NULL,
    student_id  INTEGER NOT NULL,
    content     TEXT    NOT NULL DEFAULT '',
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE (activity_id, student_id),
    FOREIGN KEY (activity_id) REFERENCES Activity (id) ON DELETE CASCADE,
    FOREIGN KEY (student_id) REFERENCES Student (id) ON DELETE CASCADE
);

-- ================================================================
-- 이력 및 스냅샷
-- ================================================================

CREATE TABLE IF NOT EXISTS ActivityRecordHistory
(
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    activity_record_id INTEGER NOT NULL,
    content            TEXT    NOT NULL,
    changed_at         TEXT    NOT NULL DEFAULT (datetime('now')),
    note               TEXT,
    FOREIGN KEY (activity_record_id) REFERENCES ActivityRecord (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS Snapshot
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    file_name   TEXT NOT NULL
);

-- ================================================================
-- 인덱스
-- ================================================================

CREATE INDEX IF NOT EXISTS idx_history_record ON ActivityRecordHistory (activity_record_id, changed_at);
CREATE INDEX IF NOT EXISTS idx_record_student ON ActivityRecord (student_id);
CREATE INDEX IF NOT EXISTS idx_areastudent_student ON AreaStudent (student_id);
CREATE INDEX IF NOT EXISTS idx_areaactivity_activity ON AreaActivity (activity_id);
CREATE INDEX IF NOT EXISTS idx_areastudent_order ON AreaStudent (area_id, is_order_customized);
CREATE INDEX IF NOT EXISTS idx_displayorder_areaactivity ON ActivityDisplayOrder (activity_id, area_id);

