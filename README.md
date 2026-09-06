# Transactional Persistent Database with MVCC

This project implements a disk-backed database core with full ACID transactions, multiversion concurrency control (MVCC), and ANSI SQL isolation levels. The database is accessed programmatically via the Connection API and the execute_command function.

## Features

- Transactions – BEGIN, COMMIT, ABORT.
- Isolation levels (ANSI SQL):
    - READ UNCOMMITTED
    - READ COMMITTED
    - REPEATABLE READ
    - SERIALIZABLE
- Data operations:
    - PUT <key> <value> – insert or update a value.
    - GET <key> – read a value according to visibility rules.
    - DELETE <key> – delete a key.
- Internal model:
    - MVCC – each value version stores xmin (id of transaction which created the value) and xmax (id of transaction which deleted the value).
    - Conflict detection for SERIALIZABLE (intersection of read and write sets).
- Persistence – keys, values, and transaction state are all durable across restarts (see Architecture below).
- Concurrency – shared storage and transaction manager; each Connection has its own current transaction. Pages are cached in a clock-algorithm buffer pool shared across connections.

## Architecture

The database uses a hashed index file for key lookup and LSM-inspired append-only data files for values, both organized as fixed-size pages accessed through a shared buffer pool.

### Index file

A fixed number of hash buckets (32), each addressable by hash(key) % 32 (uses fnv1a as hashing algorithm). On overflow, buckets extend via a linked chain of overflow pages rather than growing the base file dynamically.

```
Index file:
 ---------------------------
|Header                     |
|page_count: 32 bits        |
|                           |
|Bucket page 0              |
|Bucket page 1              |
|Bucket page 2              |
|...                        |
|Overflow pages...          |
 ---------------------------
```

```
Bucket page:
 -----------------------------------------------------------------------------------------------
|next_overflow_page: (page_num: 16 bits, file_id: 16 bits) — zero means no overflow             |
|entry_count: 16 bits                                                                           |
|free_offset: 16 bits                                                                           |
|                                                                                               |
|Entry 0: hash: 64 bits, key_len: 16 bits, tid: (page_id: 16 bits, file_id: 16 bits,            |
|          page_offset: 16 bits), key_bytes: ...                                                |
|Entry 1                                                                                        |
|...                                                                                            |
 -----------------------------------------------------------------------------------------------
```

Each entry maps a key directly to the head of its version chain in the data file (tid). Updating a key rewrites this pointer to the newly inserted version. The index only ever holds one entry per live key.

### Data file

Values are stored as an append-only sequence of fixed-size pages containing variable-length records. Each record is a single MVCC version and links to its predecessor, forming an undo chain walked at read time to find the version visible to a given snapshot.

```
Data file:
 ---------------------------
|Header                     |
|page_count: 32 bits        |
|                           |
|Data page 0                |
|Data page 1                |
|...                        |
 ---------------------------
```

```
Data page:
 -----------------------------------------------------------------------------------------
|entry_count: 16 bits                                                                     |
|free_offset: 16 bits                                                                     |
|                                                                                         |
|Record 0: xmin: 32 bits, xmax: 32 bits, data_len: 16 bits,                               |
|           prev_tid: (page_id: 16 bits, file_id: 16 bits, page_offset: 16 bits),         |
|           data: ...                                                                     |
|Record 1                                                                                 |
|...                                                                                      |
 -----------------------------------------------------------------------------------------
```

### Buffer pool

Pages from both index and data files are cached in a shared clock-algorithm buffer pool. Each page is identified by a PageId (page_num, file_id, page_type) and reference-counted while in use, preventing eviction of pages actively being read or written. Dirty pages are flushed to disk on eviction and periodically by a background writer thread.

### Transaction persistance

Transaction outcomes (Committed / Aborted / In-progress) are persisted into separate file independently of the data pages themselves. Only transaction state is stored and can be queried by transaction id.

## Errors

- NoActiveTransaction – attempting PUT/GET/DELETE/COMMIT/ABORT outside a transaction.
- TransactionAlreadyActive – a second BEGIN without COMMIT/ABORT.
- SerializationError – committing a SERIALIZABLE transaction that conflicts (e.g., another transaction modified data that the current one read).
- NotFound – the key has never existed.
- NoneVisible – the key exists, but no version is visible to the current transaction (due to isolation level or deletion).
- IoError – underlying disk I/O failure (e.g. page read/write error).

## Testing

The database is tested with a comprehensive suite of Java-based tests that verify isolation level guarantees and serializability.

- Isolation Test Base (IsolationTestBase)
- Read Uncommitted Tests
    - testDirtyReadAllowed – confirms that a transaction can read uncommitted changes from another transaction (dirty read is permitted).
- Read Committed Tests (ReadCommittedTests)
    - testDirtyReadNotAllowed – ensures that dirty reads are prevented.
    - testNonRepeatableReadAllowed – shows that non‑repeatable reads (different values for the same key within a transaction) can occur.
    - testLostUpdateAllowed – verifies that lost updates (two concurrent transactions overwriting each other's changes) are possible under Read Committed.
- Repeatable Read Tests (RepeatableReadTests)
    - testDirtyReadNotAllowed – dirty reads are blocked.
    - testNonRepeatableReadNotAllowed – repeatable reads are guaranteed (same key returns same value throughout the transaction).
    - testLostUpdateAllowed – lost updates are still possible (no automatic conflict detection).
    - testWriteSkewAllowed – write skew (two transactions reading overlapping data and updating disjoint keys) is permitted.
- Serializable Tests (SerializableTests)
    - testLostUpdateNotAllowed – ensures lost updates are prevented; at least one transaction fails with SerializationError.
    - testWriteSkewNotAllowed – write skew is blocked; only one transaction may succeed, or both fail if they conflict.
- Serializable Graph Test (SerializableGraphTest)
    - testNoCyclesInSerializableSchedule – runs 1000 random serializable transactions and builds a dependency graph based on read/write sets. Verifies that the graph contains no cycles, which is a necessary condition for serializability (conflict serializability). If a cycle is detected, the schedule is not serializable.
- Stress Tests (StressTests)
    - randomStressTest – launches multiple threads, each executing random transactions with random isolation levels, operations (GET/PUT/DELETE), and random commit/abort decisions. The test ensures no unexpected exceptions (like panics or protocol errors) occur under concurrent load.

## Future Improvements

- Vacuum – reclaim space from dead versions in data pages
- Durability – introduce a write-ahead log to prevent unrecoverable data corruption on crash