// SPDX-License-Identifier: Apache-2.0
//! Private backend qualification. Stored bytes are not engine/accounting proofs.
use rusqlite::{params, Connection, OpenFlags, Transaction};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io;
#[cfg(any(test, feature = "external-fault-injection"))]
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[cfg(test)]
pub(super) static FIXTURE_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

const MAX_SECTION: usize = 1024 * 1024;
const MAX_IMAGE: usize = 4 * 1024 * 1024;
const MAX_RECORD: usize = 64 * 1024;
const MAX_RECORDS: usize = 64;
const MAX_UNITS: u64 = 128;
type Hash = [u8; 32];
const SCHEMA:&str="CREATE TABLE meta(id INTEGER PRIMARY KEY CHECK(id=1),binding BLOB NOT NULL,revision INTEGER NOT NULL,digest BLOB NOT NULL) STRICT;
CREATE TABLE manifest(ordinal INTEGER PRIMARY KEY,name TEXT UNIQUE NOT NULL) STRICT;
CREATE TABLE units(id BLOB PRIMARY KEY,revision INTEGER UNIQUE NOT NULL,previous_revision INTEGER NOT NULL,previous_digest BLOB NOT NULL,request_digest BLOB NOT NULL,state_digest BLOB NOT NULL) STRICT;
CREATE TABLE sections(revision INTEGER NOT NULL,name TEXT NOT NULL,bytes BLOB NOT NULL,digest BLOB NOT NULL,PRIMARY KEY(revision,name)) STRICT;
CREATE TABLE facts(position INTEGER PRIMARY KEY AUTOINCREMENT,unit_id BLOB NOT NULL REFERENCES units(id),ordinal INTEGER NOT NULL,bytes BLOB NOT NULL,UNIQUE(unit_id,ordinal)) STRICT;
CREATE TABLE outbox(unit_id BLOB NOT NULL REFERENCES units(id),ordinal INTEGER NOT NULL,bytes BLOB NOT NULL,PRIMARY KEY(unit_id,ordinal)) STRICT;
PRAGMA user_version=1;";
#[derive(Debug)]
pub(super) enum Error {
    Io(io::Error),
    Sql(rusqlite::Error),
    OwnerUnavailable,
    Invalid,
    Conflict,
    Stale,
    Fenced,
    Uncertain {
        transaction: [u8; 16],
        cause: String,
    },
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "store IO: {e}"),
            Self::Sql(e) => write!(f, "store SQL: {e}"),
            Self::Uncertain { transaction, cause } => {
                write!(f, "uncertain transaction {transaction:?}: {cause}")
            }
            other => write!(f, "{other:?}"),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct State {
    pub(super) revision: u64,
    pub(super) digest: Hash,
    pub(super) sections: Vec<(String, Vec<u8>)>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Unit {
    pub(super) id: [u8; 16],
    pub(super) previous_revision: u64,
    pub(super) previous_digest: Hash,
    pub(super) sections: Vec<(String, Vec<u8>)>,
    pub(super) facts: Vec<Vec<u8>>,
    pub(super) outbox: Vec<Vec<u8>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Receipt {
    pub(super) revision: u64,
    pub(super) digest: Hash,
    pub(super) positions: Vec<i64>,
}
#[derive(Debug)]
pub(super) struct Retained {
    pub(super) unit: Unit,
    pub(super) receipt: Receipt,
}
#[derive(Clone, Copy)]
pub(super) enum CrashPoint {
    None,
    #[cfg(any(test, feature = "external-fault-injection"))]
    BeforeCommit,
    #[cfg(any(test, feature = "external-fault-injection"))]
    AfterCommit,
}
pub(super) struct Store {
    connection: Connection,
    _owner: File,
    dir: PathBuf,
    dir_id: (u64, u64),
    lock_id: (u64, u64),
    db_id: (u64, u64),
    binding: Hash,
    genesis_digest: Hash,
    manifest: Vec<String>,
    fenced: bool,
}
fn identity(m: &std::fs::Metadata) -> (u64, u64) {
    (m.dev(), m.ino())
}
fn hash32(value: Vec<u8>) -> Result<Hash, Error> {
    value.try_into().map_err(|_| Error::Invalid)
}
fn checked_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b == b'_' || b == b'-')
}
fn part(h: &mut Sha256, b: &[u8]) {
    h.update((b.len() as u64).to_be_bytes());
    h.update(b);
}
fn image(
    binding: &Hash,
    manifest: &[String],
    sections: &[(String, Vec<u8>)],
) -> Result<Hash, Error> {
    if sections.len() != manifest.len() {
        return Err(Error::Invalid);
    }
    let mut entries = BTreeMap::new();
    let mut size = 0usize;
    for (name, bytes) in sections {
        size = size.checked_add(bytes.len()).ok_or(Error::Invalid)?;
        if !checked_name(name)
            || bytes.len() > MAX_SECTION
            || size > MAX_IMAGE
            || entries.insert(name, bytes).is_some()
        {
            return Err(Error::Invalid);
        }
    }
    if !entries
        .keys()
        .map(|s| s.as_str())
        .eq(manifest.iter().map(String::as_str))
    {
        return Err(Error::Invalid);
    }
    let mut h = Sha256::new();
    part(&mut h, b"KAIROS-STORE-IMAGE-V1");
    part(&mut h, binding);
    for (name, bytes) in entries {
        part(&mut h, name.as_bytes());
        part(&mut h, bytes);
    }
    Ok(h.finalize().into())
}
fn request_hash(binding: &Hash, manifest: &[String], unit: &Unit) -> Result<Hash, Error> {
    if unit.id == [0; 16] || unit.previous_revision >= MAX_UNITS {
        return Err(Error::Invalid);
    }
    let state = image(binding, manifest, &unit.sections)?;
    let mut h = Sha256::new();
    part(&mut h, b"KAIROS-STORE-UNIT-V1");
    part(&mut h, binding);
    part(&mut h, &unit.id);
    part(&mut h, &unit.previous_revision.to_be_bytes());
    part(&mut h, &unit.previous_digest);
    part(&mut h, &state);
    for records in [&unit.facts, &unit.outbox] {
        if records.len() > MAX_RECORDS || records.iter().any(|v| v.len() > MAX_RECORD) {
            return Err(Error::Invalid);
        }
        part(&mut h, &(records.len() as u64).to_be_bytes());
        for record in records {
            part(&mut h, record);
        }
    }
    Ok(h.finalize().into())
}
#[cfg(any(test, feature = "external-fault-injection"))]
fn pause(point: CrashPoint, wanted: CrashPoint) {
    if std::mem::discriminant(&point) == std::mem::discriminant(&wanted) {
        println!("STORE_BOUNDARY");
        io::stdout().flush().unwrap();
        loop {
            std::thread::park();
        }
    }
}
impl Store {
    pub(super) fn open(
        dir: &Path,
        binding: Hash,
        mut manifest: Vec<String>,
        genesis: Vec<(String, Vec<u8>)>,
    ) -> Result<Self, Error> {
        let dm = std::fs::symlink_metadata(dir)?;
        if !dm.is_dir()
            || dm.file_type().is_symlink()
            || dm.mode() & 0o077 != 0
            || binding == [0; 32]
            || manifest.is_empty()
            || manifest.len() > 32
        {
            return Err(Error::Invalid);
        }
        let canonical = std::fs::canonicalize(dir)?;
        if identity(&std::fs::metadata(&canonical)?) != identity(&dm) {
            return Err(Error::Invalid);
        }
        let dir = canonical.as_path();
        manifest.sort();
        if manifest.iter().any(|n| !checked_name(n))
            || manifest.iter().collect::<BTreeSet<_>>().len() != manifest.len()
        {
            return Err(Error::Invalid);
        }
        let genesis_digest = image(&binding, &manifest, &genesis)?;
        let lock = dir.join("owner.lock");
        let db = dir.join("participant.sqlite");
        // A missing half of a retained store is damage or incomplete initialization,
        // never authorization to create a new genesis beside old identity state.
        let existed = db.try_exists()?;
        if lock.try_exists()? != existed {
            return Err(Error::Invalid);
        }
        if std::fs::symlink_metadata(&lock).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Invalid);
        }
        let owner = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&lock)?;
        let lm = std::fs::symlink_metadata(&lock)?;
        if !lm.is_file()
            || lm.file_type().is_symlink()
            || lm.nlink() != 1
            || lm.uid() != dm.uid()
            || identity(&lm) != identity(&owner.metadata()?)
        {
            return Err(Error::Invalid);
        }
        owner.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => Error::OwnerUnavailable,
            std::fs::TryLockError::Error(error) => Error::Io(error),
        })?;
        if std::fs::symlink_metadata(&db).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err(Error::Invalid);
        }
        let mut connection = Connection::open_with_flags(
            &db,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        connection.busy_timeout(std::time::Duration::ZERO)?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if (existed && version == 0) || (version != 0 && version != 1) {
            return Err(Error::Invalid);
        }
        if version == 1 {
            let stored: Vec<u8> = connection.query_row(
                "SELECT binding FROM meta WHERE id=1 AND length(binding)=32",
                [],
                |r| r.get(0),
            )?;
            if stored != binding {
                return Err(Error::Invalid);
            }
        }
        if version == 0
            && connection.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
                r.get::<_, i64>(0)
            })? != 0
        {
            return Err(Error::Invalid);
        }
        connection.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=EXTRA; PRAGMA locking_mode=EXCLUSIVE; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA max_page_count=32768;")?;
        let journal: String = connection.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
        let sync: i64 = connection.pragma_query_value(None, "synchronous", |r| r.get(0))?;
        let locking: String = connection.pragma_query_value(None, "locking_mode", |r| r.get(0))?;
        let foreign: i64 = connection.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
        let trusted: i64 = connection.pragma_query_value(None, "trusted_schema", |r| r.get(0))?;
        let cap: i64 = connection.pragma_query_value(None, "max_page_count", |r| r.get(0))?;
        if journal != "delete"
            || sync != 3
            || locking != "exclusive"
            || foreign != 1
            || trusted != 0
            || cap != 32768
        {
            return Err(Error::Invalid);
        }
        if version == 0 {
            let tx =
                connection.transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)?;
            tx.execute_batch(SCHEMA)?;
            tx.execute(
                "INSERT INTO meta VALUES(1,?1,0,?2)",
                params![binding.as_slice(), genesis_digest.as_slice()],
            )?;
            tx.execute(
                "INSERT INTO units VALUES(?1,0,0,?2,?3,?3)",
                params![
                    [0u8; 16].as_slice(),
                    [0u8; 32].as_slice(),
                    genesis_digest.as_slice()
                ],
            )?;
            for (index, name) in manifest.iter().enumerate() {
                tx.execute(
                    "INSERT INTO manifest VALUES(?1,?2)",
                    params![index as i64, name],
                )?;
            }
            insert_sections(&tx, 0, &genesis)?;
            tx.commit()?;
        } else {
            // Acquire SQLite's exclusive lifetime lock too; never fall back on contention.
            connection.execute_batch("BEGIN EXCLUSIVE; COMMIT;")?;
        }
        let stored: Vec<u8> = connection.query_row(
            "SELECT binding FROM meta WHERE id=1 AND length(binding)=32",
            [],
            |r| r.get(0),
        )?;
        if stored != binding {
            return Err(Error::Invalid);
        }
        let stored_manifest = {
            let mut stmt = connection
                .prepare("SELECT ordinal,length(name),name FROM manifest ORDER BY ordinal")?;
            let mut rows = stmt.query([])?;
            let mut names = Vec::new();
            while let Some(row) = rows.next()? {
                let index: i64 = row.get(0)?;
                let size: i64 = row.get(1)?;
                if index != names.len() as i64 || !(1..=64).contains(&size) || names.len() >= 32 {
                    return Err(Error::Invalid);
                }
                let name: String = row.get(2)?;
                if !checked_name(&name) {
                    return Err(Error::Invalid);
                }
                names.push(name);
            }
            names
        };
        if stored_manifest != manifest {
            return Err(Error::Invalid);
        }
        let expected: BTreeMap<_, _> = SCHEMA
            .split(';')
            .map(str::trim)
            .filter(|sql| sql.starts_with("CREATE TABLE "))
            .map(|sql| {
                let name = sql
                    .trim_start_matches("CREATE TABLE ")
                    .split('(')
                    .next()
                    .unwrap();
                (name.to_string(), sql.to_string())
            })
            .collect();
        let mut schema=connection.prepare("SELECT length(name),name,length(sql),sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name")?;
        let mut rows = schema.query([])?;
        let mut observed = BTreeMap::new();
        while let Some(row) = rows.next()? {
            let name_len: i64 = row.get(0)?;
            let sql_len: i64 = row.get(2)?;
            if !(1..=64).contains(&name_len)
                || !(1..=4096).contains(&sql_len)
                || observed.len() >= 6
            {
                return Err(Error::Invalid);
            }
            observed.insert(row.get::<_, String>(1)?, row.get::<_, String>(3)?);
        }
        drop(rows);
        drop(schema);
        if expected != observed {
            return Err(Error::Invalid);
        }
        let initial: Vec<u8> = connection.query_row(
            "SELECT state_digest FROM units WHERE revision=0 AND length(state_digest)=32",
            [],
            |r| r.get(0),
        )?;
        if initial != genesis_digest {
            return Err(Error::Invalid);
        }
        let store = Self {
            db_id: identity(&std::fs::metadata(&db)?),
            connection,
            _owner: owner,
            dir: dir.to_path_buf(),
            dir_id: identity(&dm),
            lock_id: identity(&lm),
            binding,
            genesis_digest,
            manifest,
            fenced: false,
        };
        store.load()?;
        Ok(store)
    }
    fn paths(&self) -> Result<(), Error> {
        for (path, id) in [
            (self.dir.clone(), self.dir_id),
            (self.dir.join("owner.lock"), self.lock_id),
            (self.dir.join("participant.sqlite"), self.db_id),
        ] {
            let m = std::fs::symlink_metadata(path)?;
            if m.file_type().is_symlink() || identity(&m) != id {
                return Err(Error::Invalid);
            }
        }
        Ok(())
    }
    pub(super) fn load(&self) -> Result<State, Error> {
        if self.fenced {
            return Err(Error::Fenced);
        }
        self.paths()?;
        let (revision, digest): (i64, Vec<u8>) = self.connection.query_row(
            "SELECT revision,digest FROM meta WHERE id=1 AND length(digest)=32",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let revision = u64::try_from(revision).map_err(|_| Error::Invalid)?;
        if revision > MAX_UNITS {
            return Err(Error::Invalid);
        }
        let sections = read_sections(&self.connection, revision)?;
        let digest = hash32(digest)?;
        if image(&self.binding, &self.manifest, &sections)? != digest {
            return Err(Error::Invalid);
        }
        let recorded: Vec<u8> = self.connection.query_row(
            "SELECT state_digest FROM units WHERE revision=?1 AND length(state_digest)=32",
            [revision as i64],
            |r| r.get(0),
        )?;
        if recorded != digest {
            return Err(Error::Invalid);
        }
        // No compaction in this qualification candidate: validate every retained unit,
        // predecessor and receipt position, including historical and orphan rows.
        let mut units = self
            .connection
            .prepare("SELECT revision,length(id),id FROM units ORDER BY revision")?;
        let mut rows = units.query([])?;
        let mut expected_revision = 0u64;
        let mut previous_digest = self.genesis_digest;
        let mut fact_count = 0i64;
        let mut outbox_count = 0i64;
        while let Some(row) = rows.next()? {
            if expected_revision > revision
                || row.get::<_, i64>(0)? != expected_revision as i64
                || row.get::<_, i64>(1)? != 16
            {
                return Err(Error::Invalid);
            }
            let id: [u8; 16] = row
                .get::<_, Vec<u8>>(2)?
                .try_into()
                .map_err(|_| Error::Invalid)?;
            if expected_revision == 0 {
                let initial = read_sections(&self.connection, 0)?;
                if id != [0; 16]
                    || image(&self.binding, &self.manifest, &initial)? != self.genesis_digest
                {
                    return Err(Error::Invalid);
                }
                let valid: i64 = self.connection.query_row("SELECT count(*) FROM units WHERE revision=0 AND previous_revision=0 AND previous_digest=?1 AND request_digest=?2 AND state_digest=?2", params![[0u8;32].as_slice(), self.genesis_digest.as_slice()], |r|r.get(0))?;
                if valid != 1 {
                    return Err(Error::Invalid);
                }
            } else {
                let retained = self.read_unit(&id)?.ok_or(Error::Invalid)?;
                if retained.unit.previous_revision != expected_revision - 1
                    || retained.unit.previous_digest != previous_digest
                    || retained.receipt.revision != expected_revision
                {
                    return Err(Error::Invalid);
                }
                for position in &retained.receipt.positions {
                    fact_count += 1;
                    if *position != fact_count {
                        return Err(Error::Invalid);
                    }
                }
                outbox_count += retained.unit.outbox.len() as i64;
                previous_digest = retained.receipt.digest;
            }
            expected_revision += 1;
        }
        if expected_revision != revision + 1 || previous_digest != digest {
            return Err(Error::Invalid);
        }
        for (sql, expected) in [
            ("SELECT count(*) FROM meta", 1),
            (
                "SELECT count(*) FROM sections",
                (revision as i64 + 1) * self.manifest.len() as i64,
            ),
            ("SELECT count(*) FROM facts", fact_count),
            ("SELECT count(*) FROM outbox", outbox_count),
        ] {
            if self.connection.query_row(sql, [], |r| r.get::<_, i64>(0))? != expected {
                return Err(Error::Invalid);
            }
        }
        let sequence_count: i64 =
            self.connection
                .query_row("SELECT count(*) FROM sqlite_sequence", [], |r| r.get(0))?;
        let sequence_valid: i64 = self.connection.query_row(
            "SELECT count(*) FROM sqlite_sequence WHERE name='facts' AND seq=?1",
            [fact_count],
            |r| r.get(0),
        )?;
        if sequence_count != sequence_valid
            || sequence_count > 1
            || (fact_count > 0 && sequence_valid != 1)
        {
            return Err(Error::Invalid);
        }
        Ok(State {
            revision,
            digest,
            sections,
        })
    }
    pub(super) fn readback(&self, id: &[u8; 16]) -> Result<Option<Retained>, Error> {
        self.load()?;
        self.read_unit(id)
    }
    fn read_unit(&self, id: &[u8; 16]) -> Result<Option<Retained>, Error> {
        if self.fenced {
            return Err(Error::Fenced);
        }
        self.paths()?;
        if *id == [0; 16] {
            return Err(Error::Invalid);
        }
        let mut stmt=self.connection.prepare("SELECT revision,previous_revision,length(previous_digest),length(request_digest),length(state_digest),previous_digest,request_digest,state_digest FROM units WHERE id=?1")?;
        let mut rows = stmt.query([id.as_slice()])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        for column in 2..=4 {
            if row.get::<_, i64>(column)? != 32 {
                return Err(Error::Invalid);
            }
        }
        let revision = u64::try_from(row.get::<_, i64>(0)?).map_err(|_| Error::Invalid)?;
        let previous_revision = u64::try_from(row.get::<_, i64>(1)?).map_err(|_| Error::Invalid)?;
        if revision != previous_revision.checked_add(1).ok_or(Error::Invalid)?
            || revision > MAX_UNITS
        {
            return Err(Error::Invalid);
        }
        let old: Vec<u8> = row.get(5)?;
        let request: Vec<u8> = row.get(6)?;
        let digest = hash32(row.get(7)?)?;
        let (facts, positions) = read_records(&self.connection, "facts", id)?;
        let (outbox, _) = read_records(&self.connection, "outbox", id)?;
        let unit = Unit {
            id: *id,
            previous_revision,
            previous_digest: hash32(old)?,
            sections: read_sections(&self.connection, revision)?,
            facts,
            outbox,
        };
        if request_hash(&self.binding, &self.manifest, &unit)? != hash32(request)?
            || image(&self.binding, &self.manifest, &unit.sections)? != digest
        {
            return Err(Error::Invalid);
        }
        Ok(Some(Retained {
            unit,
            receipt: Receipt {
                revision,
                digest,
                positions,
            },
        }))
    }
    pub(super) fn commit(&mut self, unit: &Unit, crash: CrashPoint) -> Result<Receipt, Error> {
        #[cfg(not(any(test, feature = "external-fault-injection")))]
        let _ = crash;
        if self.fenced {
            return Err(Error::Fenced);
        }
        self.paths()?;
        let requested = request_hash(&self.binding, &self.manifest, unit)?;
        if let Some(old) = self.readback(&unit.id)? {
            return if request_hash(&self.binding, &self.manifest, &old.unit)? == requested {
                Ok(old.receipt)
            } else {
                Err(Error::Conflict)
            };
        }
        let old = self.load()?;
        if old.revision != unit.previous_revision || old.digest != unit.previous_digest {
            return Err(Error::Stale);
        }
        let next = old.revision.checked_add(1).ok_or(Error::Invalid)?;
        let digest = image(&self.binding, &self.manifest, &unit.sections)?;
        // From here a database error is uncertain until this owner is dropped/reopened.
        self.fenced = true;
        let result = (|| {
            let tx = self
                .connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)?;
            tx.execute(
                "INSERT INTO units VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    unit.id.as_slice(),
                    next as i64,
                    old.revision as i64,
                    old.digest.as_slice(),
                    requested.as_slice(),
                    digest.as_slice()
                ],
            )?;
            insert_sections(&tx, next, &unit.sections)?;
            let mut positions = Vec::new();
            for (i, fact) in unit.facts.iter().enumerate() {
                tx.execute(
                    "INSERT INTO facts(unit_id,ordinal,bytes) VALUES(?1,?2,?3)",
                    params![unit.id.as_slice(), i as i64, fact],
                )?;
                positions.push(tx.last_insert_rowid());
            }
            for (i, intent) in unit.outbox.iter().enumerate() {
                tx.execute(
                    "INSERT INTO outbox VALUES(?1,?2,?3)",
                    params![unit.id.as_slice(), i as i64, intent],
                )?;
            }
            tx.execute(
                "UPDATE meta SET revision=?1,digest=?2 WHERE id=1",
                params![next as i64, digest.as_slice()],
            )?;
            #[cfg(any(test, feature = "external-fault-injection"))]
            pause(crash, CrashPoint::BeforeCommit);
            tx.commit()?;
            #[cfg(any(test, feature = "external-fault-injection"))]
            pause(crash, CrashPoint::AfterCommit);
            Ok::<_, Error>(Receipt {
                revision: next,
                digest,
                positions,
            })
        })();
        match result {
            Ok(receipt) => {
                self.fenced = false;
                Ok(receipt)
            }
            Err(error) => Err(Error::Uncertain {
                transaction: unit.id,
                cause: error.to_string(),
            }),
        }
    }
}
fn insert_sections(
    tx: &Transaction<'_>,
    revision: u64,
    sections: &[(String, Vec<u8>)],
) -> Result<(), Error> {
    for (name, bytes) in sections {
        let digest = Sha256::digest(bytes);
        tx.execute(
            "INSERT INTO sections VALUES(?1,?2,?3,?4)",
            params![revision as i64, name, bytes, digest.as_slice()],
        )?;
    }
    Ok(())
}
fn read_sections(connection: &Connection, revision: u64) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let mut stmt=connection.prepare("SELECT length(name),name,length(bytes),length(digest),digest,bytes FROM sections WHERE revision=?1 ORDER BY name")?;
    let mut rows = stmt.query([revision as i64])?;
    let mut result = Vec::new();
    let mut total = 0usize;
    while let Some(row) = rows.next()? {
        let name_length: i64 = row.get(0)?;
        let length: i64 = row.get(2)?;
        let hash_length: i64 = row.get(3)?;
        let size = usize::try_from(length).map_err(|_| Error::Invalid)?;
        total = total.checked_add(size).ok_or(Error::Invalid)?;
        if !(1..=64).contains(&name_length)
            || size > MAX_SECTION
            || total > MAX_IMAGE
            || hash_length != 32
            || result.len() >= 32
        {
            return Err(Error::Invalid);
        }
        let name: String = row.get(1)?;
        let stored: Vec<u8> = row.get(4)?;
        let bytes: Vec<u8> = row.get(5)?;
        if !checked_name(&name) || Sha256::digest(&bytes).as_slice() != stored {
            return Err(Error::Invalid);
        }
        result.push((name, bytes));
    }
    Ok(result)
}
fn read_records(
    connection: &Connection,
    kind: &str,
    id: &[u8; 16],
) -> Result<(Vec<Vec<u8>>, Vec<i64>), Error> {
    let sql=match kind{"facts"=>"SELECT ordinal,length(bytes),bytes,position FROM facts WHERE unit_id=?1 ORDER BY ordinal","outbox"=>"SELECT ordinal,length(bytes),bytes,ordinal FROM outbox WHERE unit_id=?1 ORDER BY ordinal",_=>return Err(Error::Invalid)};
    let mut stmt = connection.prepare(sql)?;
    let mut rows = stmt.query([id.as_slice()])?;
    let mut bytes = Vec::new();
    let mut positions = Vec::new();
    while let Some(row) = rows.next()? {
        let index: i64 = row.get(0)?;
        let size: i64 = row.get(1)?;
        if index != bytes.len() as i64
            || !(0..=MAX_RECORD as i64).contains(&size)
            || bytes.len() >= MAX_RECORDS
        {
            return Err(Error::Invalid);
        }
        let position: i64 = row.get(3)?;
        if kind == "facts"
            && (position <= 0
                || positions
                    .last()
                    .is_some_and(|previous| *previous >= position))
        {
            return Err(Error::Invalid);
        }
        bytes.push(row.get(2)?);
        positions.push(position);
    }
    Ok((bytes, positions))
}
#[cfg(test)]
mod tests;
